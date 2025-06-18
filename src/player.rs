use crate::engine::input::InputState;
use crate::engine::physics::{apply_gravity, integrate, Collider, RigidBody};
use glam::{Quat, Vec3};
use winit::event::VirtualKeyCode;

pub struct Player {
    pub position: Vec3,
    pub rotation: Quat,
    pub body: RigidBody,
    pub speed: f32,
    pub jump_power: f32,
    pub collider: Collider,
}

impl Player {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            body: RigidBody {
                velocity: Vec3::ZERO,
                on_ground: false,
            },
            speed: 8.0,
            jump_power: 12.0,
            collider: Collider {
                half_extents: Vec3::new(0.5, 0.75, 0.5),
            },
        }
    }

    pub fn update(&mut self, input: &InputState, dt: f32) {
        let mut direction = Vec3::ZERO;
        if input.pressed(VirtualKeyCode::W) {
            direction.z -= 1.0;
        }
        if input.pressed(VirtualKeyCode::S) {
            direction.z += 1.0;
        }
        if input.pressed(VirtualKeyCode::A) {
            direction.x -= 1.0;
        }
        if input.pressed(VirtualKeyCode::D) {
            direction.x += 1.0;
        }
        if input.pressed(VirtualKeyCode::Space) && self.body.on_ground {
            self.body.velocity.y = self.jump_power;
            self.body.on_ground = false;
        }
        if direction.length_squared() > 0.0 {
            direction = direction.normalize();
            self.body.velocity.x = direction.x * self.speed;
            self.body.velocity.z = direction.z * self.speed;
        } else {
            self.body.velocity.x *= 0.8;
            self.body.velocity.z *= 0.8;
        }

        apply_gravity(&mut self.body, dt);
        integrate(&mut self.position, &mut self.body, dt);
        if self.position.y <= self.collider.half_extents.y {
            self.position.y = self.collider.half_extents.y;
            if self.body.velocity.y <= 0.0 {
                self.body.velocity.y = 0.0;
                self.body.on_ground = true;
            }
        }
    }
}
