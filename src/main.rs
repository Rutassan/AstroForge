use bevy::prelude::*;

mod player;
use player::PlayerPlugin;

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
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("✅ Настройка 3D сцены...");
    
    // Камера
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

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
    
    println!("🌍 Сцена создана!");
}
