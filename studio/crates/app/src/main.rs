//! Janela do Studio-like. Etapa atual: Nível A (chão, personagem, câmera isométrica, clique para andar).
//! Escala: 1 stud = 1 unidade do mundo (eixo Y para cima, como no Roblox).
//!
//! Modo de teste (QA), por variáveis de ambiente:
//! - `STUDIO_CLICKS="t:x,z;t:x,z"`: injeta cliques direitos no ponto do mundo (x, z) no tempo t (s),
//!   passando pelo mesmo caminho do mouse (mundo → tela → raio → chão).
//! - `STUDIO_SHOTS="t:arquivo.png;..."`: print só da janela no tempo t.
//! - `STUDIO_ROJO=http://127.0.0.1:34872`: carrega o jogo de um `rojo serve` (Luau → render, com hot reload).
//!   Sem isso, abre a cena provisória embutida.
//! - `STUDIO_EXIT_AFTER=s`: fecha depois de s segundos e imprime FPS médio e posição final.

mod camera;
mod game;
mod movement;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::PrimaryWindow;

use movement::{Click, Mover};

/// Metade da altura do personagem provisório (caixa 2×5×1): centro fica 2,5 acima dos pés.
const HALF_HEIGHT: f32 = 2.5;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct GameCamera {
    distance: f32,
}

#[derive(Component)]
struct Marker {
    born: f32,
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct PlayerMover(Mover);

/// Clique direito em coordenadas da janela (pixels lógicos).
#[derive(Message)]
struct RightClick(Vec2);

#[derive(Resource, Default)]
struct Script {
    clicks: Vec<(f32, Vec2)>,
    shots: Vec<(f32, String)>,
    exit_after: Option<f32>,
    fps_samples: Vec<f64>,
}

fn parse_pairs(var: &str) -> Vec<(f32, String)> {
    std::env::var(var)
        .unwrap_or_default()
        .split(';')
        .filter_map(|item| {
            let (time, rest) = item.split_once(':')?;
            Some((time.trim().parse().ok()?, rest.trim().to_owned()))
        })
        .collect()
}

fn script_from_env() -> Script {
    let clicks = parse_pairs("STUDIO_CLICKS")
        .into_iter()
        .filter_map(|(time, xz)| {
            let (x, z) = xz.split_once(',')?;
            Some((
                time,
                Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?),
            ))
        })
        .collect();
    Script {
        clicks,
        shots: parse_pairs("STUDIO_SHOTS"),
        exit_after: std::env::var("STUDIO_EXIT_AFTER")
            .ok()
            .and_then(|value| value.parse().ok()),
        fps_samples: Vec::new(),
    }
}

fn main() {
    let mut app = App::new();
    if let Ok(url) = std::env::var("STUDIO_ROJO") {
        match game::LuaWorld::connect(&url) {
            Ok(world) => {
                println!("Rojo conectado em {url}");
                app.insert_non_send(world);
            }
            Err(err) => {
                eprintln!("Não conectou ao Rojo em {url}: {err}\nAbrindo a cena provisória.")
            }
        }
    }
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Studio-like (MVP)".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
    ))
    .insert_resource(ClearColor(Color::srgb(0.53, 0.70, 0.92)))
    .insert_resource(script_from_env())
    .init_resource::<PlayerMover>()
    .add_message::<RightClick>()
    .add_systems(Startup, (setup, game::setup_console))
    .add_systems(
        Update,
        (
            game::step,
            (mouse_input, scripted_input),
            handle_clicks,
            move_player,
            camera_system,
            update_markers,
            script_output,
        )
            .chain(),
    )
    .run();
}

fn setup(
    lua_world: Option<NonSend<game::LuaWorld>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if lua_world.is_none() {
        // Baseplate provisório: 512×1×512, topo em y = 0 (medidas e cor "a confirmar").
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(512.0, 1.0, 512.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.38, 0.38, 0.4))),
            Transform::from_xyz(0.0, -0.5, 0.0),
        ));
        // Marcos provisórios nas posições do mapa de teste (resultado-esperado.md), só para dar
        // referência de escala e movimento até os scripts Luau criarem o mundo de verdade.
        let dummy = materials.add(Color::srgb(0.75, 0.62, 0.45));
        let dummy_mesh = meshes.add(Cuboid::new(2.0, 5.0, 1.0));
        for (x, z) in [(40.0, -10.0), (50.0, 5.0), (38.0, 15.0)] {
            commands.spawn((
                Mesh3d(dummy_mesh.clone()),
                MeshMaterial3d(dummy.clone()),
                Transform::from_xyz(x, HALF_HEIGHT, z),
            ));
        }
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 6.0, 4.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.35, 0.6, 0.35))),
            Transform::from_xyz(60.0, 3.0, 0.0),
        ));
    }
    // Personagem provisório (caixa 2×5×1), pés em y = 0.
    commands.spawn((
        Player,
        Mesh3d(meshes.add(Cuboid::new(2.0, 5.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.9))),
        Transform::from_xyz(0.0, HALF_HEIGHT, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        brightness: 400.0,
        ..default()
    });
    let focus = Vec3::Y * camera::FOCUS_HEIGHT;
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: camera::FOV_DEG.to_radians(),
            ..default()
        }),
        GameCamera {
            distance: camera::DEFAULT_DISTANCE,
        },
        Transform::from_translation(camera::camera_position(focus, camera::DEFAULT_DISTANCE))
            .looking_at(focus, Vec3::Y),
    ));
}

fn mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut clicks: MessageWriter<RightClick>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    if let Ok(window) = window.single()
        && let Some(cursor) = window.cursor_position()
    {
        clicks.write(RightClick(cursor));
    }
}

/// Clique injetado: ponto do mundo → posição na tela, para testar o mesmo caminho do mouse.
fn scripted_input(
    time: Res<Time>,
    mut script: ResMut<Script>,
    camera: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    mut clicks: MessageWriter<RightClick>,
) {
    let now = time.elapsed_secs();
    let Ok((camera, transform)) = camera.single() else {
        return;
    };
    let mut pending = Vec::new();
    script.clicks.retain(|(at, xz)| {
        if *at <= now {
            pending.push(*xz);
            false
        } else {
            true
        }
    });
    for xz in pending {
        match camera.world_to_viewport(transform, Vec3::new(xz.x, 0.0, xz.y)) {
            Ok(screen) => {
                clicks.write(RightClick(screen));
            }
            Err(_) => info!("clique injetado fora da tela: {xz}"),
        }
    }
}

#[allow(clippy::too_many_arguments)] // parâmetros de sistema do Bevy
fn handle_clicks(
    mut commands: Commands,
    mut clicks: MessageReader<RightClick>,
    time: Res<Time>,
    mut mover: ResMut<PlayerMover>,
    camera: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    player: Query<&Transform, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (Ok((camera, camera_transform)), Ok(player)) = (camera.single(), player.single()) else {
        return;
    };
    for RightClick(screen) in clicks.read() {
        let Ok(ray) = camera.viewport_to_world(camera_transform, *screen) else {
            continue;
        };
        // Por enquanto o único alvo do raio é o chão (y = 0). Raycast nas Parts entra com a física.
        let Some(hit) = movement::ray_ground(ray.origin, *ray.direction, 0.0) else {
            continue;
        };
        let now = f64::from(time.elapsed_secs());
        let feet = player.translation - Vec3::Y * HALF_HEIGHT;
        let result = mover.0.click(now, feet, hit);
        info!("clique direito em {hit:.1}: {result:?}");
        if result == Click::Ignored {
            // A bolinha é efeito local e aparece mesmo assim; o servidor só ignora o MoveTo.
        }
        spawn_marker(
            &mut commands,
            &mut meshes,
            &mut materials,
            hit,
            time.elapsed_secs(),
        );
    }
}

fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
    now: f32,
) {
    use movement::marker;
    // Neon branco: sem iluminação, cor cheia, transparência 0,3 (Material Neon; brilho real vem
    // com o bloom do studio-vfx).
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 1.0 - marker::START_TRANSPARENCY),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Marker {
            born: now,
            material: material.clone(),
        },
        Mesh3d(meshes.add(Cylinder::new(0.5, 1.0))),
        MeshMaterial3d(material),
        Transform::from_translation(at).with_scale(Vec3::new(
            marker::SIZE,
            marker::THICKNESS,
            marker::SIZE,
        )),
    ));
}

fn update_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: Query<(Entity, &Marker, &mut Transform)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, marker, mut transform) in &mut markers {
        match movement::marker::at(time.elapsed_secs() - marker.born) {
            Some((size, transparency)) => {
                transform.scale = Vec3::new(size, movement::marker::THICKNESS, size);
                if let Some(mut material) = materials.get_mut(&marker.material) {
                    material.base_color.set_alpha(1.0 - transparency);
                }
            }
            None => {
                materials.remove(&marker.material);
                commands.entity(entity).despawn();
            }
        }
    }
}

fn move_player(
    time: Res<Time>,
    mut mover: ResMut<PlayerMover>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = player.single_mut() else {
        return;
    };
    let before = transform.translation;
    let now = f64::from(time.elapsed_secs());
    transform.translation = mover.0.step(now, before, time.delta_secs());
    // O Humanoid vira para a direção do movimento (AutoRotate).
    let delta = transform.translation - before;
    if delta.x.abs() + delta.z.abs() > 1e-5 {
        let facing = transform.translation + Vec3::new(delta.x, 0.0, delta.z);
        transform.look_at(facing, Vec3::Y);
    }
}

fn camera_system(
    time: Res<Time>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    player: Query<&Transform, (With<Player>, Without<GameCamera>)>,
    mut cameras: Query<(&mut GameCamera, &mut Transform), Without<Player>>,
) {
    let wheel: f32 = scroll.read().map(|event| event.y).sum();
    let Ok(player) = player.single() else { return };
    let focus = player.translation + Vec3::Y * (camera::FOCUS_HEIGHT - HALF_HEIGHT);
    for (mut game_camera, mut transform) in &mut cameras {
        game_camera.distance = camera::zoom(game_camera.distance, wheel);
        let target = camera::camera_position(focus, game_camera.distance);
        transform.translation = camera::follow(transform.translation, target, time.delta_secs());
        // Ângulo fixo: olha na direção do Config, sem girar com o atraso da suavização.
        let look = camera::camera_position(Vec3::ZERO, 1.0);
        transform.look_to(-look, Vec3::Y);
    }
}

fn script_output(
    mut commands: Commands,
    time: Res<Time>,
    mut script: ResMut<Script>,
    diagnostics: Res<DiagnosticsStore>,
    player: Query<&Transform, With<Player>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    // FPS só depois de 3 s (o primeiro quadro compila shaders).
    if now > 3.0
        && let Some(fps) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
    {
        script.fps_samples.push(fps);
    }
    let mut due = Vec::new();
    script.shots.retain(|(at, path)| {
        if *at <= now {
            due.push(path.clone());
            false
        } else {
            true
        }
    });
    for path in due {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if let Some(limit) = script.exit_after
        && now >= limit
    {
        let samples = &script.fps_samples;
        let average = if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        };
        let minimum = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let position = player
            .single()
            .map(|transform| transform.translation - Vec3::Y * HALF_HEIGHT)
            .unwrap_or_default();
        println!("STUDIO_RESULT fps_avg={average:.1} fps_min={minimum:.1} feet={position:.2}");
        exit.write(AppExit::Success);
    }
}
