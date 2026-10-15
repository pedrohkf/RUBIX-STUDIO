//! Carrega a árvore do Rojo no DataModel do Luau e executa Script/ModuleScript.
//!
//! `script` é uma global por script (a própria Instance); `require(module)` roda o
//! ModuleScript uma vez e guarda o resultado (cache por Instance), como no Roblox.

use std::collections::HashMap;

use lune_roblox::instance::{Instance, instance_to_lua};
use mlua::prelude::*;
use rbx_dom_weak::WeakDom;
use rbx_types::{Ref, Variant};

use crate::{Error, Result, Runtime};

const SERVICES: &[&str] = &[
    "ReplicatedStorage",
    "ServerScriptService",
    "ServerStorage",
    "StarterPlayer",
    "StarterGui",
    "StarterPack",
    "Workspace",
    "Lighting",
    "Players",
    "ReplicatedFirst",
];

/// Cria o `require` global. O cache fica numa tabela no registro, chaveada pelo Ref da Instance.
pub(crate) fn install_require(lua: &Lua) -> LuaResult<()> {
    let cache = lua.create_table()?;
    let require = lua.create_function(move |lua, module: LuaAnyUserData| {
        let instance = *module
            .borrow::<Instance>()
            .map_err(|_| LuaError::runtime("require: esperava um ModuleScript"))?;
        if !instance.is_a("ModuleScript") {
            return Err(LuaError::runtime(format!(
                "require: {} não é um ModuleScript",
                instance.get_full_name()
            )));
        }
        let key = format!("{:?}", instance.dom_ref);
        let cached: LuaValue = cache.get(key.as_str())?;
        match cached {
            LuaValue::Nil => {}
            LuaValue::Table(marker)
                if marker.raw_len() == 0 && marker.contains_key("loading")? =>
            {
                return Err(LuaError::runtime(format!(
                    "require: dependência circular em {}",
                    instance.get_full_name()
                )));
            }
            value => return Ok(value),
        }
        let loading = lua.create_table()?;
        loading.set("loading", true)?;
        cache.set(key.as_str(), loading)?;
        let result = run_source(lua, instance, &module);
        match result {
            Ok(value) => {
                // Módulo que devolve nil vira `true`-like no Roblox é erro; aqui devolvemos nil.
                cache.set(key.as_str(), value.clone())?;
                Ok(value)
            }
            Err(err) => {
                cache.set(key.as_str(), LuaValue::Nil)?;
                Err(err)
            }
        }
    })?;
    lua.globals().set("require", require)
}

fn source_of(instance: &Instance) -> LuaResult<String> {
    match instance.get_property("Source") {
        Some(Variant::String(source)) => Ok(source),
        _ => Err(LuaError::runtime(format!(
            "{} não tem Source",
            instance.get_full_name()
        ))),
    }
}

/// Compila o Source da Instance numa função Lua com `script` apontando para ela.
fn compile(lua: &Lua, instance: Instance, script: &LuaAnyUserData) -> LuaResult<LuaFunction> {
    let source = source_of(&instance)?;
    let env = lua.create_table()?;
    env.set("script", script.clone())?;
    let meta = lua.create_table()?;
    meta.set("__index", lua.globals())?;
    env.set_metatable(Some(meta))?;
    let name = format!("={}", instance.get_full_name());
    lua.load(source)
        .set_name(name)
        .set_environment(env)
        .into_function()
}

/// Executa o Source na thread atual (usado por `require`; o módulo não pode ceder ao carregar).
fn run_source(lua: &Lua, instance: Instance, script: &LuaAnyUserData) -> LuaResult<LuaValue> {
    compile(lua, instance, script)?.call::<LuaValue>(())
}

impl Runtime {
    /// Copia a árvore do Rojo para o DataModel do Luau. Serviços reaproveitam `GetService`.
    /// Devolve quantas Instances foram criadas.
    pub fn load_tree(&self, dom: &WeakDom) -> Result<usize> {
        let mut created = 0usize;
        let mut map: HashMap<Ref, Instance> = HashMap::new();
        let game = self.game();
        map.insert(dom.root_ref(), game);
        let mut queue: std::collections::VecDeque<Ref> = std::collections::VecDeque::new();
        queue.push_back(dom.root_ref());
        while let Some(id) = queue.pop_front() {
            let Some(parent_inst) = dom.get_by_ref(id) else {
                continue;
            };
            for child_id in parent_inst.children() {
                let Some(child) = dom.get_by_ref(*child_id) else {
                    continue;
                };
                let Some(parent) = map.get(&id).copied() else {
                    continue;
                };
                let instance = if id == dom.root_ref() && SERVICES.contains(&child.class.as_str()) {
                    self.service(&child.class)?
                } else {
                    let instance = Instance::new_in_dom(game.dom_id, child.class.as_str());
                    instance.set_parent(Some(parent));
                    created += 1;
                    instance
                };
                instance.set_name(child.name.as_str());
                for (name, value) in &child.properties {
                    if name.as_str() == "Name" {
                        continue;
                    }
                    instance.set_property(name.as_str(), value.clone());
                }
                map.insert(*child_id, instance);
                queue.push_back(*child_id);
            }
        }
        Ok(created)
    }

    fn service(&self, name: &str) -> Result<Instance> {
        let value: LuaValue = self
            .lua()
            .load("local game, name = ... return game:GetService(name)")
            .call((self.game_value()?, name.to_owned()))?;
        match value {
            LuaValue::UserData(data) => data
                .borrow::<Instance>()
                .map(|inst| *inst)
                .map_err(Error::Lua),
            _ => Err(Error::Lua(LuaError::runtime(format!(
                "serviço {name} inválido"
            )))),
        }
    }

    fn game_value(&self) -> Result<LuaValue> {
        Ok(instance_to_lua(self.lua(), self.game())?)
    }

    /// Acha uma Instance por caminho, por exemplo `ServerScriptService.Server.Main`.
    pub fn find_path(&self, path: &str) -> Option<Instance> {
        let mut current = self.game();
        for part in path.split('.') {
            current = current.find_child(|child| child.name == part)?;
        }
        Some(current)
    }

    /// Agenda um Script (ou LocalScript) da árvore carregada, sem avançar o relógio.
    /// O código roda no próximo `tick` (ou em `run_instance`).
    pub fn start_instance(&self, instance: Instance) -> Result<()> {
        source_of(&instance).map_err(Error::Lua)?;
        let value = instance_to_lua(self.lua(), instance)?;
        let LuaValue::UserData(script) = value else {
            return Ok(());
        };
        // A função do chunk vira a própria thread: sem frame Rust no meio, `task.wait` pode ceder.
        let function = compile(self.lua(), instance, &script).map_err(Error::Lua)?;
        self.spawn_function(function)
    }

    /// Agenda todos os Script e LocalScript da árvore (ordem da árvore). Devolve quantos.
    pub fn start_all_scripts(&self) -> Result<usize> {
        let mut started = 0;
        for instance in self.game().get_descendants_preorder() {
            if instance.is_a("Script") || instance.is_a("LocalScript") {
                self.start_instance(instance)?;
                started += 1;
            }
        }
        Ok(started)
    }

    /// Roda um Script (ou LocalScript) até acabar (relógio virtual acelerado, para testes).
    pub fn run_instance(&self, instance: Instance) -> Result<()> {
        self.start_instance(instance)?;
        self.drive(3600.0);
        Ok(())
    }
}
