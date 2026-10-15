//! Ponte Luau: mlua + scheduler do Lune + tipos e Instance do `lune-roblox` (MPL-2.0, como dependência).
//!
//! Decisão (ver docs/decisions.md): reaproveitar `lune-roblox` para Instance e tipos de dados.
//! O DataModel do jogo vive no registro interno dele; a engine lê pela API pública de `Instance`.

use std::sync::{Arc, Mutex};

mod scripts;
mod view;

pub use view::{PartShape, PartView};

use lune_roblox::instance::Instance;
use mlua::prelude::*;
use mlua_luau_scheduler::{Functions, Scheduler};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("erro Lua: {0}")]
    Lua(#[from] LuaError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Linha do console (print, warn ou erro de script).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLine {
    Print(String),
    Warn(String),
    Error(String),
}

pub type Log = Arc<Mutex<Vec<LogLine>>>;

/// Relógio virtual em segundos: o teste avança o tempo sem esperar (passo fixo de 1/60 s).
pub type Clock = Arc<Mutex<ClockState>>;

#[derive(Default)]
pub struct ClockState {
    now: f64,
    wakers: Vec<std::task::Waker>,
}

pub const STEP: f64 = 1.0 / 60.0;

pub struct Runtime {
    lua: Lua,
    // Rc: o Drop do Scheduler remove metadados do Lua, então ele não pode ser clonado de verdade.
    scheduler: std::rc::Rc<Scheduler>,
    game: Instance,
    log: Log,
    clock: Clock,
    runner: std::cell::RefCell<Option<Runner>>,
}

type Runner = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

impl Runtime {
    pub fn new() -> Result<Self> {
        let lua = Lua::new();
        let log: Log = Arc::default();

        // Tipos de dados e Instance do Roblox como globais.
        for pair in lune_roblox::module(lua.clone())?.pairs::<LuaValue, LuaValue>() {
            let (key, value) = pair?;
            lua.globals().set(key, value)?;
        }

        let game = Instance::new_orphaned("DataModel");
        lua.globals()
            .set("game", lune_roblox::instance::instance_to_lua(&lua, game)?)?;
        let workspace: LuaValue = lua.load("return game.Workspace").eval()?;
        lua.globals().set("workspace", workspace)?;

        install_output(&lua, &log)?;
        scripts::install_require(&lua)?;
        let scheduler = std::rc::Rc::new(Scheduler::new(lua.clone()));
        let clock: Clock = Arc::default();
        install_task(&lua, &clock)?;
        let error_log = log.clone();
        scheduler.set_error_callback(move |err| {
            // Erro de um script não derruba o resto (docs/bug-prevention.md, item 1).
            if let Ok(mut lines) = error_log.lock() {
                lines.push(LogLine::Error(err.to_string()));
            }
        });
        Ok(Self {
            lua,
            scheduler,
            game,
            log,
            clock,
            runner: std::cell::RefCell::new(None),
        })
    }

    pub(crate) fn lua(&self) -> &Lua {
        &self.lua
    }

    pub fn game(&self) -> Instance {
        self.game
    }

    pub fn log(&self) -> Vec<LogLine> {
        self.log
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }

    /// Avança o jogo `dt` segundos: move o relógio, acorda os `task.wait` vencidos e roda o scheduler
    /// até não haver mais nada pronto. Chamar uma vez por quadro (o futuro do scheduler é mantido
    /// entre chamadas, senão as threads em espera seriam descartadas).
    pub fn tick(&self, dt: f64) {
        let wakers = match self.clock.lock() {
            Ok(mut clock) => {
                clock.now += dt;
                std::mem::take(&mut clock.wakers)
            }
            Err(_) => return,
        };
        for waker in wakers {
            waker.wake();
        }
        let Ok(mut slot) = self.runner.try_borrow_mut() else {
            return;
        };
        let mut future = slot.take().unwrap_or_else(|| {
            let scheduler = std::rc::Rc::clone(&self.scheduler);
            Box::pin(async move { scheduler.run().await })
        });
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        if future.as_mut().poll(&mut context).is_pending() {
            *slot = Some(future);
        }
    }

    /// Tempo virtual atual, em segundos.
    pub fn now(&self) -> f64 {
        self.clock.lock().map(|c| c.now).unwrap_or(0.0)
    }

    /// Roda um script (como `Script`) e avança o relógio virtual em passos de 1/60 s até todas
    /// as threads terminarem ou passarem `max_seconds` virtuais (guarda contra loop infinito).
    pub fn run_script(&self, name: &str, source: &str) -> Result<()> {
        self.run_script_for(name, source, 3600.0)
    }

    pub fn run_script_for(&self, name: &str, source: &str, max_seconds: f64) -> Result<()> {
        let chunk = self.lua.load(source).set_name(name);
        self.scheduler.push_thread_front(chunk, ())?;
        self.drive(max_seconds);
        Ok(())
    }

    /// Agenda uma função Lua como thread do scheduler (roda no próximo `tick`).
    pub(crate) fn spawn_function(&self, function: LuaFunction) -> Result<()> {
        self.scheduler.push_thread_back(function, ())?;
        Ok(())
    }

    /// Avança o scheduler e o relógio virtual até acabar o trabalho ou estourar `max_seconds`.
    fn drive(&self, max_seconds: f64) {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        let mut run = std::pin::pin!(self.scheduler.run());
        let limit = self.now() + max_seconds;
        loop {
            if run.as_mut().poll(&mut context).is_ready() {
                return;
            }
            let wakers = match self.clock.lock() {
                Ok(mut c) => {
                    c.now += STEP;
                    if c.now > limit {
                        return;
                    }
                    std::mem::take(&mut c.wakers)
                }
                Err(_) => return,
            };
            for waker in wakers {
                waker.wake();
            }
        }
    }
}

fn install_output(lua: &Lua, log: &Log) -> LuaResult<()> {
    let join = |lua: &Lua, args: LuaMultiValue| -> LuaResult<String> {
        let tostring: LuaFunction = lua.globals().get("tostring")?;
        let mut parts = Vec::new();
        for value in args {
            parts.push(tostring.call::<String>(value)?);
        }
        Ok(parts.join("\t"))
    };
    let print_log = log.clone();
    lua.globals().set(
        "print",
        lua.create_function(move |lua, args: LuaMultiValue| {
            let line = join(lua, args)?;
            if let Ok(mut lines) = print_log.lock() {
                lines.push(LogLine::Print(line));
            }
            Ok(())
        })?,
    )?;
    let warn_log = log.clone();
    lua.globals().set(
        "warn",
        lua.create_function(move |lua, args: LuaMultiValue| {
            let line = join(lua, args)?;
            if let Ok(mut lines) = warn_log.lock() {
                lines.push(LogLine::Warn(line));
            }
            Ok(())
        })?,
    )?;
    Ok(())
}

fn install_task(lua: &Lua, clock: &Clock) -> LuaResult<()> {
    let fns = Functions::new(lua.clone())?;
    let wait_clock = clock.clone();
    let wait = lua.create_async_function(move |_, seconds: Option<f64>| {
        let clock = wait_clock.clone();
        async move {
            let now = |c: &Clock| c.lock().map(|s| s.now).unwrap_or(0.0);
            let start = now(&clock);
            let target = start + seconds.unwrap_or(0.0).max(0.0);
            // Cede a vez ao menos uma vez, como o task.wait do Roblox.
            yield_once().await;
            std::future::poll_fn(|context| {
                if now(&clock) >= target {
                    return std::task::Poll::Ready(());
                }
                if let Ok(mut state) = clock.lock() {
                    state.wakers.push(context.waker().clone());
                }
                std::task::Poll::Pending
            })
            .await;
            Ok(now(&clock) - start)
        }
    })?;
    let task = lua.create_table()?;
    task.set("spawn", fns.spawn)?;
    task.set("defer", fns.defer)?;
    task.set("cancel", fns.cancel)?;
    task.set("wait", wait)?;
    lua.globals().set("task", task)?;
    Ok(())
}

async fn yield_once() {
    let mut yielded = false;
    std::future::poll_fn(|context| {
        if yielded {
            std::task::Poll::Ready(())
        } else {
            yielded = true;
            context.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .await;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn m4_instance_new_part_under_workspace() {
        let rt = Runtime::new().unwrap();
        rt.run_script(
            "m4",
            r#"
            local p = Instance.new("Part")
            p.Name = "Cubo"
            p.Size = Vector3.new(4, 1, 2)
            p.Parent = workspace
            print("filhos:", #workspace:GetChildren(), p.Size)
            "#,
        )
        .unwrap();
        let log = rt.log();
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(
            matches!(&log[0], LogLine::Print(l) if l.starts_with("filhos:\t1")),
            "{log:?}"
        );
        let workspace = rt.game().find_child(|c| c.class == "Workspace").unwrap();
        let part = workspace.get_children();
        assert_eq!(part.len(), 1);
        assert_eq!(part[0].get_name(), "Cubo");
    }

    #[test]
    fn script_error_is_isolated_and_logged() {
        let rt = Runtime::new().unwrap();
        rt.run_script("boom", "error('boom')").unwrap();
        rt.run_script("ok", "print('depois')").unwrap();
        let log = rt.log();
        assert!(
            matches!(&log[0], LogLine::Error(m) if m.contains("boom")),
            "{log:?}"
        );
        assert_eq!(log[1], LogLine::Print("depois".into()));
    }

    #[test]
    fn task_wait_and_spawn_order() {
        let rt = Runtime::new().unwrap();
        rt.run_script(
            "task",
            r#"
            task.spawn(function() task.wait(0.02); print("b") end)
            print("a")
            task.wait(0.05)
            print("c")
            "#,
        )
        .unwrap();
        let lines: Vec<_> = rt.log().into_iter().collect();
        assert_eq!(
            lines,
            vec![
                LogLine::Print("a".into()),
                LogLine::Print("b".into()),
                LogLine::Print("c".into())
            ]
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tree_tests {
    use super::*;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use rbx_types::Variant;

    fn script(class: &str, name: &str, source: &str) -> InstanceBuilder {
        InstanceBuilder::new(class)
            .with_name(name)
            .with_property("Source", Variant::String(source.into()))
    }

    #[test]
    fn require_cache_script_parent_and_services() {
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel"));
        let root = dom.root_ref();
        let rs = dom.insert(root, InstanceBuilder::new("ReplicatedStorage"));
        let shared = dom.insert(rs, InstanceBuilder::new("Folder").with_name("Shared"));
        dom.insert(
            shared,
            script(
                "ModuleScript",
                "Config",
                "print('config carregado') return {Nome = 'Exemplo'}",
            ),
        );
        let sss = dom.insert(root, InstanceBuilder::new("ServerScriptService"));
        dom.insert(
            sss,
            script(
                "Script",
                "Main",
                r#"
                local RS = game:GetService("ReplicatedStorage")
                local a = require(RS.Shared.Config)
                local b = require(script.Parent.Parent.ReplicatedStorage.Shared.Config)
                print(a.Nome, a == b)
                "#,
            ),
        );
        let rt = Runtime::new().unwrap();
        rt.load_tree(&dom).unwrap();
        let main = rt.find_path("ServerScriptService.Main").unwrap();
        rt.run_instance(main).unwrap();
        assert_eq!(
            rt.log(),
            vec![
                LogLine::Print("config carregado".into()),
                LogLine::Print("Exemplo\ttrue".into())
            ]
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod view_tests {
    use super::*;

    #[test]
    fn tick_runs_wait_loops_across_frames_and_parts_are_readable() {
        let rt = Runtime::new().unwrap();
        let ws = rt.find_path("Workspace").unwrap();
        let script = Instance::new_in_dom(rt.game().dom_id, "Script");
        script.set_parent(Some(ws));
        script.set_property(
            "Source",
            rbx_types::Variant::String(
                r#"
                local p = Instance.new("Part")
                p.Name = "Girando"
                p.Size = Vector3.new(2, 2, 2)
                p.Color = Color3.fromRGB(0, 255, 0)
                p.Parent = workspace
                local a = 0
                while true do
                    a += 1
                    p.CFrame = CFrame.new(a, 5, 0) * CFrame.Angles(0, math.rad(90), 0)
                    task.wait(0.1)
                end
                "#
                .into(),
            ),
        );
        rt.start_all_scripts().unwrap();
        for _ in 0..60 {
            rt.tick(1.0 / 60.0); // 1 s de jogo em 60 quadros
        }
        let parts = rt.workspace_parts();
        assert_eq!(parts.len(), 1, "{parts:?}");
        let part = &parts[0];
        assert_eq!(part.name, "Girando");
        assert_eq!(part.size, [2.0, 2.0, 2.0]);
        assert_eq!(part.color, [0.0, 1.0, 0.0]);
        // ~10 passos de 0,1 s em 1 s: a posição X andou (11 ± 1), Y fixo.
        assert!(
            (10.0..=12.0).contains(&part.position[0]),
            "{:?}",
            part.position
        );
        assert_eq!(part.position[1], 5.0);
        // Giro de 90° em Y: eixo X da Part aponta para -Z.
        assert!(part.rotation[0][2].abs() > 0.99, "{:?}", part.rotation);
    }
}
