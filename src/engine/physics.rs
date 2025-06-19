use glam::Vec3;

pub const GRAVITY: f32 = 9.81;

#[derive(Clone, Copy)]
pub struct Collider {
    pub half_extents: Vec3,
}

#[derive(Clone, Copy)]
pub struct RigidBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub mass: f32,
    pub force: Vec3,
}

impl RigidBody {
    pub fn new(mass: f32, position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            on_ground: false,
            mass,
            force: Vec3::ZERO,
        }
    }

    pub fn apply_force(&mut self, force: Vec3) {
        self.force += force;
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        self.velocity += impulse / self.mass;
    }
}

#[derive(Clone, Copy)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

pub fn apply_gravity(body: &mut RigidBody) {
    if !body.on_ground {
        body.force.y -= body.mass * GRAVITY;
    }
}

pub fn integrate(body: &mut RigidBody, dt: f32) {
    let acceleration = body.force / body.mass;
    body.velocity += acceleration * dt;
    body.position += body.velocity * dt;
    body.force = Vec3::ZERO;
}

pub fn resolve_aabb_collisions(
    body: &mut RigidBody,
    collider: &Collider,
    obstacles: &[Aabb],
) {
    for obs in obstacles {
        let delta = body.position - obs.center;
        let overlap = collider.half_extents + obs.half_extents - delta.abs();
        if overlap.x > 0.0 && overlap.y > 0.0 && overlap.z > 0.0 {
            if overlap.x < overlap.y && overlap.x < overlap.z {
                let sign = if delta.x > 0.0 { 1.0 } else { -1.0 };
                body.position.x = obs.center.x + sign * (obs.half_extents.x + collider.half_extents.x);
                body.velocity.x = 0.0;
            } else if overlap.y < overlap.z {
                let sign = if delta.y > 0.0 { 1.0 } else { -1.0 };
                body.position.y = obs.center.y + sign * (obs.half_extents.y + collider.half_extents.y);
                body.velocity.y = 0.0;
                if sign > 0.0 {
                    body.on_ground = true;
                }
            } else {
                let sign = if delta.z > 0.0 { 1.0 } else { -1.0 };
                body.position.z = obs.center.z + sign * (obs.half_extents.z + collider.half_extents.z);
                body.velocity.z = 0.0;
            }
        }
    }
}

pub struct PhysicsObject<'a> {
    pub body: &'a mut RigidBody,
    pub collider: Collider,
}

pub fn resolve_pair(a: &mut PhysicsObject, b: &mut PhysicsObject) -> bool {
    let delta = a.body.position - b.body.position;
    let overlap = a.collider.half_extents + b.collider.half_extents - delta.abs();
    if overlap.x > 0.0 && overlap.y > 0.0 && overlap.z > 0.0 {
        if overlap.x < overlap.y && overlap.x < overlap.z {
            let sign = if delta.x > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.x * 0.5;
            a.body.position.x += sign * push;
            b.body.position.x -= sign * push;
            a.body.velocity.x = 0.0;
            b.body.velocity.x = 0.0;
        } else if overlap.y < overlap.z {
            let sign = if delta.y > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.y * 0.5;
            a.body.position.y += sign * push;
            b.body.position.y -= sign * push;
            a.body.velocity.y = 0.0;
            b.body.velocity.y = 0.0;
        } else {
            let sign = if delta.z > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.z * 0.5;
            a.body.position.z += sign * push;
            b.body.position.z -= sign * push;
            a.body.velocity.z = 0.0;
            b.body.velocity.z = 0.0;
        }
        true
    } else {
        false
    }
}

pub fn step(objects: &mut [PhysicsObject], static_obs: &[Aabb], dt: f32) -> Vec<(usize, usize)> {
    for obj in objects.iter_mut() {
        apply_gravity(obj.body);
        integrate(obj.body, dt);
        resolve_aabb_collisions(obj.body, &obj.collider, static_obs);
    }

    let mut pairs = Vec::new();
    for i in 0..objects.len() {
        for j in (i + 1)..objects.len() {
            // Split borrow to avoid double mutable borrow
            let (left, right) = objects.split_at_mut(j);
            let a = &mut left[i];
            let b = &mut right[0];
            if resolve_pair(a, b) {
                pairs.push((i, j));
            }
        }
    }
    pairs
}
