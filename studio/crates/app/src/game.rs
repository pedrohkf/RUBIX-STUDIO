//! Liga o jogo (Rojo → Luau) ao render: o DataModel do Luau é a fonte de verdade e cada Part do
//! Workspace vira uma entidade. Hot reload: um patch do Rojo recria o Runtime e relê a árvore.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lua::{LogLine, PartShape, PartView, Runtime};
use rbx_types::Ref;
use rojo_client::{Poll, RojoClient, Session};

/// Quantas linhas do console aparecem na tela.
const CONSOLE_LINES: usize = 8;

struct Live {
    view: PartView,
    entity: Entity,
}

pub struct LuaWorld {
    runtime: Runtime,
    session: Session,
    parts: HashMap<Ref, Live>,
    console: Vec<String>,
    /// Quantas linhas do log do Runtime atual já foram para o console.
    log_seen: usize,
    reloads: u32,
}

#[derive(Component)]
pub struct LuaPart;

#[derive(Component)]
pub struct ConsoleText;

impl LuaWorld {
    pub fn connect(url: &str) -> Result<Self, String> {
        let session = Session::connect(&RojoClient::new(url)).map_err(|err| err.to_string())?;
        let runtime = boot(&session)?;
        Ok(Self {
            runtime,
            session,
            parts: HashMap::new(),
            console: Vec::new(),
            log_seen: 0,
            reloads: 0,
        })
    }
}

fn boot(session: &Session) -> Result<Runtime, String> {
    let runtime = Runtime::new().map_err(|err| err.to_string())?;
    runtime
        .load_tree(&session.dom)
        .map_err(|err| err.to_string())?;
    runtime.start_all_scripts().map_err(|err| err.to_string())?;
    Ok(runtime)
}

fn line_text(line: &LogLine) -> String {
    match line {
        LogLine::Print(text) => text.clone(),
        LogLine::Warn(text) => format!("[aviso] {text}"),
        LogLine::Error(text) => format!("[erro] {}", text.lines().next().unwrap_or_default()),
    }
}

pub fn setup_console(mut commands: Commands) {
    commands.spawn((
        ConsoleText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(15.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Um quadro do jogo: patches do Rojo, scripts, console e sincronização das Parts.
pub fn step(
    mut commands: Commands,
    time: Res<Time>,
    world: Option<NonSendMut<LuaWorld>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut transforms: Query<&mut Transform, With<LuaPart>>,
    mut console: Query<&mut Text, With<ConsoleText>>,
) {
    let Some(mut world) = world else { return };
    let world = &mut *world;

    match world.session.poll() {
        Ok(Poll::Patched { .. }) => match boot(&world.session) {
            Ok(runtime) => {
                // Hot reload: descarta o mundo antigo e recomeça com o código novo.
                world.runtime = runtime;
                world.log_seen = 0;
                world.reloads += 1;
                world
                    .console
                    .push(format!("-- hot reload #{} --", world.reloads));
                for (_, live) in world.parts.drain() {
                    commands.entity(live.entity).despawn();
                }
            }
            Err(err) => world.console.push(format!("[erro] hot reload: {err}")),
        },
        Ok(Poll::Idle) => {}
        Err(err) => world.console.push(format!("[erro] Rojo: {err}")),
    }

    world.runtime.tick(f64::from(time.delta_secs()));

    let log = world.runtime.log();
    for line in log.iter().skip(world.log_seen) {
        let text = line_text(line);
        println!("{text}");
        world.console.push(text);
    }
    world.log_seen = log.len();
    if let Ok(mut text) = console.single_mut() {
        let start = world.console.len().saturating_sub(CONSOLE_LINES);
        text.0 = world.console.get(start..).unwrap_or_default().join("\n");
    }

    let mut seen: HashSet<Ref> = HashSet::new();
    for view in world.runtime.workspace_parts() {
        seen.insert(view.id);
        match world.parts.get_mut(&view.id) {
            Some(live) if live.view == view => {}
            Some(live) => {
                if let Ok(mut current) = transforms.get_mut(live.entity) {
                    *current = part_transform(&view);
                }
                if live.view.color != view.color || live.view.transparency != view.transparency {
                    commands
                        .entity(live.entity)
                        .insert(MeshMaterial3d(materials.add(part_material(&view))));
                }
                live.view = view;
            }
            None => {
                let entity = commands
                    .spawn((
                        LuaPart,
                        Mesh3d(meshes.add(part_mesh(view.shape))),
                        MeshMaterial3d(materials.add(part_material(&view))),
                        part_transform(&view),
                    ))
                    .id();
                world.parts.insert(view.id, Live { view, entity });
            }
        }
    }
    world.parts.retain(|id, live| {
        let keep = seen.contains(id);
        if !keep {
            commands.entity(live.entity).despawn();
        }
        keep
    });
}

fn part_mesh(shape: PartShape) -> Mesh {
    match shape {
        PartShape::Block => Cuboid::new(1.0, 1.0, 1.0).into(),
        PartShape::Ball => Sphere::new(0.5).into(),
        PartShape::Cylinder => Cylinder::new(0.5, 1.0).into(),
    }
}

fn part_material(view: &PartView) -> StandardMaterial {
    let [r, g, b] = view.color;
    let alpha = 1.0 - view.transparency.clamp(0.0, 1.0);
    StandardMaterial {
        base_color: Color::srgba(r, g, b, alpha),
        alpha_mode: if alpha < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        perceptual_roughness: 0.9,
        ..default()
    }
}

/// CFrame do Roblox → Transform do Bevy (ambos de mão direita, Y para cima, 1 stud = 1 unidade).
fn part_transform(view: &PartView) -> Transform {
    let [x, y, z] = view.rotation;
    let rotation = Quat::from_mat3(&Mat3::from_cols(
        Vec3::from(x),
        Vec3::from(y),
        Vec3::from(z),
    ));
    let size = Vec3::from(view.size);
    match view.shape {
        PartShape::Cylinder => Transform {
            translation: Vec3::from(view.position),
            // O cilindro do Roblox tem o eixo no X; o do Bevy, no Y.
            rotation: rotation * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(size.y, size.x, size.z),
        },
        _ => Transform {
            translation: Vec3::from(view.position),
            rotation,
            scale: size,
        },
    }
}
