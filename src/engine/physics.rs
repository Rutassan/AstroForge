use glam::Vec3;

#[derive(Clone, Copy)]
pub struct Collider {
    pub half_extents: Vec3,
}

#[derive(Clone, Copy)]
pub struct RigidBody {
    pub velocity: Vec3,
    pub on_ground: bool,
}

pub fn apply_gravity(body: &mut RigidBody, dt: f32) {
    if !body.on_ground {
        body.velocity.y -= 25.0 * dt;
    }
}

pub fn integrate(pos: &mut Vec3, body: &mut RigidBody, dt: f32) {
    *pos += body.velocity * dt;
}
