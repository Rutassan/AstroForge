use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub speed: f32,
    pub jump_power: f32,
    pub on_ground: bool,
}

#[derive(Component)]
pub struct Spaceship;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
            .add_systems(Update, (player_movement, player_physics, camera_follow));
    }
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("🛸 Создание космического корабля...");
    
    // Создаем корабль
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.5, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.9, 1.0),
            metallic: 0.8,
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Player {
            velocity: Vec3::ZERO,
            speed: 8.0,
            jump_power: 12.0,
            on_ground: false,
        },
        Spaceship,
        Name::new("SpaceShip"),
    ));
    
    println!("🛸 Космический корабль создан!");
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Player), With<Spaceship>>,
) {
    for (mut transform, mut player) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        // WASD управление
        if keyboard_input.pressed(KeyCode::KeyW) {
            direction.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            direction.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
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
            let target_rotation = Quat::from_rotation_y(player.velocity.z.atan2(player.velocity.x) + std::f32::consts::PI / 2.0);
            transform.rotation = transform.rotation.slerp(target_rotation, time.delta_secs() * 5.0);
        }
    }
}

fn player_physics(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Player), With<Spaceship>>,
) {
    for (mut transform, mut player) in query.iter_mut() {
        // Гравитация
        if !player.on_ground {
            player.velocity.y -= 25.0 * time.delta_secs();
        }

        // Применяем скорость к позиции
        transform.translation += player.velocity * time.delta_secs();
        
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
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Spaceship>)>,
    time: Res<Time>,
) {
    if let (Ok(player_transform), Ok(mut camera_transform)) = (player_query.single(), camera_query.single_mut()) {
        let target_position = player_transform.translation + Vec3::new(0.0, 5.0, 10.0);
        camera_transform.translation = camera_transform.translation.lerp(target_position, time.delta_secs() * 2.0);
        camera_transform.look_at(player_transform.translation, Vec3::Y);
    }
}
