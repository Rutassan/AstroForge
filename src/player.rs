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
        Self {
            // Spawn the player a bit away from the origin so the initial view
            // isn't clipped by the cube at the center of the scene.
            position: Vec3::new(0.0, 1.0, 2.0),
            rotation: Quat::IDENTITY,
            yaw: 0.0,
            pitch: 0.0,
            body: RigidBody::new(80.0),
            movement_force: 300.0,
            // Импульс прыжка задаётся в ньютон-секундах
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
    }
}
