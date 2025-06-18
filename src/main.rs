use base64::{engine::general_purpose, Engine as _};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
use player::{CameraController, Collider, PlayerPlugin};

#[derive(Resource, Default)]
struct Paused(pub bool);

#[derive(Component)]
struct Flicker {
    base_intensity: f32,
    amplitude: f32,
    speed: f32,
    active: bool,
}

#[derive(Component)]
struct Glow {
    material: Handle<StandardMaterial>,
    base_emissive: Color,
    amplitude: f32,
    speed: f32,
    active: bool,
}

#[derive(Component)]
struct Artifact;

#[derive(Resource)]
struct ActivationSound(Handle<AudioSource>);

const ACTIVATION_B64: &str = include_str!("../assets/activation.ogg.b64");

fn main() {
    println!("🚀 AstroForge запускается...");

    App::new()
        .insert_resource(player::ControlSettings::default())
        .insert_resource(Paused(false))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "🚀 AstroForge - Космическая игра".to_string(),
                resolution: (1024.0, 768.0).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, toggle_pause)
        .add_systems(Update, artifact_reaction)
        .add_systems(Update, flicker_light)
        .add_systems(Update, glow_blocks)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
    settings: Res<player::ControlSettings>,
) {
    println!("✅ Настройка 3D сцены...");

    // Камера
    commands.spawn((
        Camera3d::default(),
        CameraController {
            distance: 0.0,
            sensitivity: settings.mouse_sensitivity,
            ..default()
        },
        Transform::from_xyz(0.0, 1.5, 0.0),
    ));

    if let Ok(mut window) = windows.single_mut() {
        window.cursor_options.visible = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }

    // Направленный свет (солнце)
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 0.5, -0.5)),
    ));

    // Земля/платформа
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.8, 0.3),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider {
            half_extents: Vec3::new(10.0, 0.1, 10.0),
        },
    ));

    // Несколько кубов для красоты
    for i in 0..5 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.2, 0.2),
                ..default()
            })),
            Transform::from_xyz(i as f32 * 3.0 - 6.0, 0.5, 3.0),
            Collider {
                half_extents: Vec3::splat(0.5),
            },
        ));
    }

    // Древняя конструкция в центре
    spawn_mysterious_structure(&mut commands, &mut meshes, &mut materials);

    // Загружаем звук активации артефакта из встроенных данных
    let b64_clean: String = ACTIVATION_B64
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&b64_clean)
        .expect("valid base64 (проверьте файл assets/activation.ogg.b64)");
    let handle = audio_assets.add(AudioSource {
        bytes: bytes.into(),
    });
    commands.insert_resource(ActivationSound(handle));

    // Мерцающий свет в центре конструкции (неактивен при старте)
    commands.spawn((
        PointLight {
            intensity: 0.0,
            range: 8.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 0.0),
        Flicker {
            base_intensity: 50.0,
            amplitude: 100.0,
            speed: 5.0,
            active: false,
        },
        Artifact,
    ));

    println!("🌍 Сцена создана!");
}

fn spawn_mysterious_structure(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let block_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let block_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Колоннада из темных блоков
    let z_positions = [-3.0, -1.0, 1.0, 3.0];
    for &z in &z_positions {
        for &x in &[-2.0_f32, 2.0] {
            for y in 0..3 {
                // Пропускаем несколько верхних блоков для эффекта разрушения
                if (z == -1.0 && x > 0.0 && y == 2) || (z == 3.0 && x < 0.0 && y == 2) {
                    continue;
                }
                commands.spawn((
                    Mesh3d(block_mesh.clone()),
                    MeshMaterial3d(block_material.clone()),
                    Transform::from_xyz(x, 0.5 + y as f32, z),
                    Collider {
                        half_extents: Vec3::splat(0.5),
                    },
                ));
            }
        }
    }

    // Перекладины между колоннами
    let beam_z = [-3.0, 0.0, 3.0];
    for &z in &beam_z {
        for &x in &[-0.5_f32, 0.5] {
            commands.spawn((
                Mesh3d(block_mesh.clone()),
                MeshMaterial3d(block_material.clone()),
                Transform::from_xyz(x, 3.5, z),
                Collider {
                    half_extents: Vec3::splat(0.5),
                },
            ));
        }
    }

    // Центральные блоки артефакта с подсветкой
    for i in 0..3 {
        let mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.05, 0.05),
            perceptual_roughness: 0.9,
            emissive: Color::BLACK.into(),
            ..default()
        });
        commands.spawn((
            Mesh3d(block_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(0.0, 0.5 + i as f32, 0.0),
            Collider {
                half_extents: Vec3::splat(0.5),
            },
            Glow {
                material: mat,
                base_emissive: Color::srgb(0.2, 0.6, 1.0),
                amplitude: 1.0,
                speed: 3.0,
                active: false,
            },
            Artifact,
        ));
    }
}
fn artifact_reaction(
    player: Query<&Transform, With<player::Spaceship>>,
    mut flicker: Query<&mut Flicker, With<Artifact>>,
    mut glows: Query<&mut Glow>,
    sound: Res<ActivationSound>,
    mut commands: Commands,
    mut state: Local<bool>,
) {
    let player_pos = if let Ok(t) = player.get_single() {
        t.translation
    } else {
        return;
    };

    let near = player_pos.truncate().length() < 3.0;

    if near && !*state {
        *state = true;
        for mut f in &mut flicker {
            f.active = true;
        }
        for mut g in &mut glows {
            g.active = true;
        }
        commands.spawn(AudioPlayer::new(sound.0.clone()));
    } else if !near && *state {
        *state = false;
        for mut f in &mut flicker {
            f.active = false;
        }
        for mut g in &mut glows {
            g.active = false;
        }
    }
}

fn flicker_light(time: Res<Time>, mut query: Query<(&mut PointLight, &Flicker)>) {
    for (mut light, flicker) in &mut query {
        if flicker.active {
            let phase = (time.elapsed_secs() * flicker.speed).sin().abs();
            light.intensity = flicker.base_intensity + phase * flicker.amplitude;
        } else {
            light.intensity = 0.0;
        }
    }
}

fn glow_blocks(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&Glow>,
) {
    for glow in &query {
        if let Some(mat) = materials.get_mut(&glow.material) {
            if glow.active {
                let t = (time.elapsed_secs() * glow.speed).sin() * 0.5 + 0.5;
                let intensity = glow.amplitude * t;
                let base = glow.base_emissive.to_linear();
                mat.emissive = Color::linear_rgb(
                    base.red * intensity,
                    base.green * intensity,
                    base.blue * intensity,
                ).into();
            } else {
                mat.emissive = Color::BLACK.into();
            }
        }
    }
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut paused: ResMut<Paused>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        paused.0 = !paused.0;
    }

    if paused.0 && (mouse.just_pressed(MouseButton::Left) || keyboard.just_pressed(KeyCode::Enter))
    {
        paused.0 = false;
    }

    if let Ok(mut window) = windows.single_mut() {
        if paused.0 {
            window.cursor_options.visible = true;
            window.cursor_options.grab_mode = CursorGrabMode::None;
        } else {
            window.cursor_options.visible = false;
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
        }
    }
}
