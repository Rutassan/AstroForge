use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use crate::Paused;

#[derive(Resource)]
pub struct ControlSettings {
    pub mouse_sensitivity: f32,
    pub movement_speed: f32,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.001,
            movement_speed: 8.0,
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub speed: f32,
    pub jump_power: f32,
    pub on_ground: bool,
}

#[derive(Component)]
pub struct Spaceship;

#[derive(Component)]
pub struct Collider {
    pub half_extents: Vec3,
}

#[derive(Component, Default)]
pub struct CameraController {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub sensitivity: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_player)
            .add_systems(
                Update,
                (camera_input, player_movement, player_physics, camera_follow)
                    .chain()
                    .run_if(not_paused),
            );
    }
}

fn not_paused(paused: Res<Paused>) -> bool {
    !paused.0
}

fn setup_player(
    mut commands: Commands,
    settings: Res<ControlSettings>,
) {
    println!("🛸 Создание космического корабля...");

    // Создаем невидимую физическую модель игрока
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0),
        Player {
            velocity: Vec3::ZERO,
            speed: settings.movement_speed,
            jump_power: 12.0,
            on_ground: false,
        },
        Spaceship,
        Name::new("SpaceShip"),
    ));

    println!("🛸 Космический корабль создан!");
}

fn camera_input(
    mut mouse_motion: EventReader<MouseMotion>,
    mut query: Query<&mut CameraController>,
) {
    let delta: Vec2 = mouse_motion.read().map(|e| e.delta).sum();
    if delta == Vec2::ZERO {
        return;
    }
    for mut controller in &mut query {
        controller.yaw -= delta.x * controller.sensitivity;
        controller.pitch = (controller.pitch - delta.y * controller.sensitivity).clamp(-1.54, 1.54);
    }
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut queries: ParamSet<(
        Query<&Transform, With<Camera3d>>,
        Query<(&mut Transform, &mut Player), With<Spaceship>>,
    )>,
) {
    let camera_transform = if let Ok(t) = queries.p0().get_single() {
        *t
    } else {
        Transform::default()
    };

    let forward = {
        let f = camera_transform.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
    };
    let right = {
        let r = camera_transform.right();
        Vec3::new(r.x, 0.0, r.z).normalize_or_zero()
    };

    for (mut transform, mut player) in queries.p1().iter_mut() {
        let mut direction = Vec3::ZERO;

        // WASD управление
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction -= right;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction += right;
        }

        // Прыжок
        if keyboard_input.just_pressed(KeyCode::Space) && player.on_ground {
            player.velocity.y = player.jump_power;
            player.on_ground = false;
        }

        // Применяем горизонтальное движение
        if direction.length() > 0.0 {
            direction = direction.normalize();
            player.velocity.x = direction.x * player.speed;
            player.velocity.z = direction.z * player.speed;
        } else {
            // Затухание скорости
            player.velocity.x *= 0.8;
            player.velocity.z *= 0.8;
        }

        // Поворот корабля в направлении движения
        if player.velocity.length() > 0.1 {
            let target_rotation = Quat::from_rotation_y(
                player.velocity.z.atan2(player.velocity.x) + std::f32::consts::PI / 2.0,
            );
            transform.rotation = transform
                .rotation
                .slerp(target_rotation, time.delta_secs() * 5.0);
        }
    }
}

fn player_physics(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Player), With<Spaceship>>,
    colliders: Query<&Transform, (With<Collider>, Without<Spaceship>)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut player) in query.iter_mut() {
        // Гравитация
        if !player.on_ground {
            player.velocity.y -= 25.0 * dt;
        }

        // Применяем скорость к позиции
        let mut new_translation = transform.translation;
        new_translation.y += player.velocity.y * dt;

        // Горизонтальные перемещения с проверкой коллизий
        let mut proposed_x = new_translation.x + player.velocity.x * dt;
        let mut proposed_z = new_translation.z + player.velocity.z * dt;

        for collider in &colliders {
            let half = Vec3::splat(0.5);
            let player_half = Vec3::new(0.5, 0.25, 1.0);

            // Проверяем ось X
            if (proposed_x - collider.translation.x).abs() < player_half.x + half.x
                && (new_translation.y - collider.translation.y).abs() < player_half.y + half.y
                && (new_translation.z - collider.translation.z).abs() < player_half.z + half.z
            {
                proposed_x = transform.translation.x;
            }

            // Проверяем ось Z
            if (new_translation.x - collider.translation.x).abs() < player_half.x + half.x
                && (new_translation.y - collider.translation.y).abs() < player_half.y + half.y
                && (proposed_z - collider.translation.z).abs() < player_half.z + half.z
            {
                proposed_z = transform.translation.z;
            }
        }

        new_translation.x = proposed_x;
        new_translation.z = proposed_z;
        transform.translation = new_translation;

        // Проверка земли
        if transform.translation.y <= 0.75 {
            transform.translation.y = 0.75;
            if player.velocity.y <= 0.0 {
                player.velocity.y = 0.0;
                player.on_ground = true;
            }
        } else {
            player.on_ground = false;
        }

        // Ограничиваем область движения
        transform.translation.x = transform.translation.x.clamp(-9.0, 9.0);
        transform.translation.z = transform.translation.z.clamp(-9.0, 9.0);
    }
}

fn camera_follow(
    player_query: Query<&Transform, (With<Spaceship>, Without<Camera3d>)>,
    mut camera_query: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
    if let (Ok(player_transform), Ok((mut camera_transform, controller))) =
        (player_query.single(), camera_query.single_mut())
    {
        let rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
        camera_transform.translation = player_transform.translation + Vec3::Y * 1.5;
        camera_transform.rotation = rotation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;
    use bevy_tasks::{ComputeTaskPool, TaskPool};
    use std::time::Duration;

    fn setup_app() -> App {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut app = App::new();
        app.add_systems(Update, (player_movement, player_physics).chain());
        app.world_mut().insert_resource(Time::<()>::default());
        app.world_mut()
            .insert_resource(ButtonInput::<KeyCode>::default());
        app
    }

    fn spawn_entities(app: &mut App, yaw: f32) {
        app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_rotation(Quat::from_rotation_y(yaw)),
            CameraController {
                yaw,
                ..Default::default()
            },
        ));

        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.75, 0.0),
            Player {
                velocity: Vec3::ZERO,
                speed: 1.0,
                jump_power: 0.0,
                on_ground: true,
            },
            Spaceship,
        ));
    }

    fn step(app: &mut App) -> Vec3 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0));
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<Spaceship>>();
        query.single(app.world()).unwrap().translation
    }

    #[test]
    fn move_forward() {
        let mut app = setup_app();
        spawn_entities(&mut app, std::f32::consts::FRAC_PI_2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        let pos = step(&mut app);
        assert!(pos.x < 0.0);
    }

    #[test]
    fn move_backward() {
        let mut app = setup_app();
        spawn_entities(&mut app, std::f32::consts::FRAC_PI_2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyS);
        let pos = step(&mut app);
        assert!(pos.x > 0.0);
    }

    #[test]
    fn move_left() {
        let mut app = setup_app();
        spawn_entities(&mut app, std::f32::consts::FRAC_PI_2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyA);
        let pos = step(&mut app);
        assert!(pos.z > 0.0);
    }

    #[test]
    fn move_right() {
        let mut app = setup_app();
        spawn_entities(&mut app, std::f32::consts::FRAC_PI_2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        let pos = step(&mut app);
        assert!(pos.z < 0.0);
    }
}
