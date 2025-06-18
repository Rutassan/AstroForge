use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

mod player;
use player::{CameraController, PlayerPlugin};

#[derive(Component)]
struct Flicker {
    base_intensity: f32,
    amplitude: f32,
    speed: f32,
}

fn main() {
    println!("🚀 AstroForge запускается...");

    App::new()
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
        .add_systems(Update, flicker_light)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    println!("✅ Настройка 3D сцены...");

    // Камера
    commands.spawn((
        Camera3d::default(),
        CameraController {
            distance: 10.0,
            sensitivity: 0.002,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        ));
    }

    // Древняя конструкция в центре
    spawn_mysterious_structure(&mut commands, &mut meshes, &mut materials);

    // Мерцающий свет в центре конструкции
    commands.spawn((
        PointLight {
            intensity: 300.0,
            range: 8.0,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 0.0),
        Flicker {
            base_intensity: 300.0,
            amplitude: 100.0,
            speed: 5.0,
        },
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
            ));
        }
    }
}

fn flicker_light(time: Res<Time>, mut query: Query<(&mut PointLight, &Flicker)>) {
    for (mut light, flicker) in &mut query {
        let phase = (time.elapsed_seconds() * flicker.speed).sin().abs();
        light.intensity = flicker.base_intensity + phase * flicker.amplitude;
    }
}
