use bevy::{
    prelude::*,
    input::mouse::AccumulatedMouseMotion,
    window::CursorGrabMode,
    render::mesh::{SphereKind, SphereMeshBuilder, Mesh3d},
    pbr::prelude::{MeshMaterial3d, StandardMaterial, DistanceFog, FogFalloff},
};

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
            .add_systems(Update, (camera_movement, capture_cursor));
    }
}

fn setup_player(
    mut commands: Commands,
    mut windows: Query<&mut Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut window = windows.single_mut().unwrap();
    window.cursor_options.visible = false;
    window.cursor_options.grab_mode = CursorGrabMode::Locked;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        DistanceFog {
            color: Color::srgb(0.05, 0.05, 0.05),
            falloff: FogFalloff::Exponential { density: 0.15 },
            ..default()
        },
        Player,
    ));

    commands.spawn((
        PointLight {
            intensity: 3000.0,
            range: 10.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(SphereMeshBuilder::new(
            0.5,
            SphereKind::Ico { subdivisions: 5 },
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            emissive: Color::srgb(1.0, 0.5, 0.0).into(),
            ..default()
        })),
        Transform::IDENTITY,
    ));
}

fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let mut transform = match query.get_single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };

    let sensitivity = 0.003;
    transform.rotate_y(-mouse.delta.x * sensitivity);
    transform.rotate_local_x(-mouse.delta.y * sensitivity);

    let mut dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        dir += *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        dir -= *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        dir -= *transform.right();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        dir += *transform.right();
    }

    if dir != Vec3::ZERO {
        transform.translation += dir.normalize() * 5.0 * time.delta_secs();
    }
}

fn capture_cursor(
    mut windows: Query<&mut Window>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut().unwrap();
    if mouse_button.just_pressed(MouseButton::Left) {
        window.cursor_options.visible = false;
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        window.cursor_options.visible = true;
        window.cursor_options.grab_mode = CursorGrabMode::None;
    }
}
