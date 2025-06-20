use crate::engine::input::InputState;
use crate::engine::physics::{Aabb, Collider, RigidBody};
use glam::{Quat, Vec3};
use winit::event::VirtualKeyCode;

pub struct Player {
    pub position: Vec3,
    pub rotation: Quat,
    yaw: f32,
    pitch: f32,
    pub body: RigidBody,
    pub movement_force: f32,
    pub jump_impulse: f32,
    pub friction: f32,
    pub collider: Collider,
}

impl Player {
    pub fn new() -> Self {
        let start_pos = Vec3::new(0.0, 1.0, 2.0);
        Self {
            position: start_pos,
            rotation: Quat::IDENTITY,
            yaw: 0.0,
            pitch: 0.0,
            body: RigidBody::new(80.0, start_pos),
            movement_force: 300.0,
            jump_impulse: 500.0,
            friction: 5.0,
            collider: Collider {
                half_extents: Vec3::new(0.5, 0.75, 0.5),
            },
        }
    }

    pub fn artifact_aabbs() -> Vec<Aabb> {
        const COUNT: usize = 28;
        const RADIUS: f32 = 3.0;
        let mut blocks = Vec::with_capacity(COUNT);
        for i in 0..COUNT {
            let angle = i as f32 / COUNT as f32 * std::f32::consts::TAU;
            let x = RADIUS * angle.cos();
            let z = RADIUS * angle.sin();
            blocks.push(Aabb {
                center: Vec3::new(x, 0.5, z),
                half_extents: Vec3::splat(0.5),
            });
        }
        blocks
    }

    pub fn update(&mut self, input: &InputState, dt: f32) {
        let sensitivity = 0.002;
        self.yaw -= input.mouse_delta.0 * sensitivity;
        self.pitch = (self.pitch - input.mouse_delta.1 * sensitivity).clamp(-1.54, 1.54);
        self.rotation =
            Quat::from_axis_angle(Vec3::Y, self.yaw) * Quat::from_axis_angle(Vec3::X, self.pitch);

        let forward = self.rotation * Vec3::Z * -1.0;
        let right = self.rotation * Vec3::X;
        let mut direction = Vec3::ZERO;
        if input.pressed(VirtualKeyCode::W) {
            direction += forward;
        }
        if input.pressed(VirtualKeyCode::S) {
            direction -= forward;
        }
        if input.pressed(VirtualKeyCode::A) {
            direction -= right;
        }
        if input.pressed(VirtualKeyCode::D) {
            direction += right;
        }
        if input.pressed(VirtualKeyCode::Space) && self.body.on_ground {
            self.body.apply_impulse(Vec3::Y * self.jump_impulse);
            self.body.on_ground = false;
        }

        // Accelerate in the pressed direction without overriding existing
        // velocity so that external impulses (like knockback) continue to
        // influence the player.
        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            self.body.apply_force(direction * self.movement_force);
        }

        // Простое затухание скорости через силу трения
        self.body
            .apply_force(-self.body.velocity * self.friction * self.body.mass);

        // Синхронизируем позицию игрока с физическим телом
        self.position = self.body.position;
    }
}

pub struct Enemy {
    pub bullet_timer: f32,
    pub body: RigidBody,
    pub collider: Collider,
}

const ENEMY_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

impl Enemy {
    pub fn new() -> Self {
        Self {
            bullet_timer: 2.0,
            body: RigidBody::new(80.0, Vec3::new(8.0, 0.75, -8.0)),
            collider: Collider {
                half_extents: Vec3::new(0.5, 0.75, 0.5),
            },
        }
    }

    pub fn update(&mut self, dt: f32) {
        // Для тестов враг остаётся на месте, но обновляем таймер выстрела
        self.bullet_timer -= dt;
    }

    pub fn append_cubes(&self, cubes: &mut Vec<crate::engine::renderer::CubeInstance>) {
        let base = self.body.position;
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(0.0, 0.3, 0.0),
            size: 0.4,
            color: ENEMY_COLOR,
        });
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(0.0, 0.65, 0.0),
            size: 0.22,
            color: ENEMY_COLOR,
        });
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(-0.12, 0.08, 0.0),
            size: 0.16,
            color: ENEMY_COLOR,
        });
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(0.12, 0.08, 0.0),
            size: 0.16,
            color: ENEMY_COLOR,
        });
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(-0.23, 0.38, 0.0),
            size: 0.13,
            color: ENEMY_COLOR,
        });
        cubes.push(crate::engine::renderer::CubeInstance {
            position: base + Vec3::new(0.23, 0.38, 0.0),
            size: 0.13,
            color: ENEMY_COLOR,
        });
    }
}
