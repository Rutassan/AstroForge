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

#[derive(Clone, Copy)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

pub fn apply_gravity(body: &mut RigidBody, dt: f32) {
    if !body.on_ground {
        body.velocity.y -= 25.0 * dt;
    }
}

pub fn integrate(pos: &mut Vec3, body: &mut RigidBody, dt: f32) {
    *pos += body.velocity * dt;
}

pub fn resolve_aabb_collisions(
    pos: &mut Vec3,
    body: &mut RigidBody,
    collider: &Collider,
    obstacles: &[Aabb],
) {
    for obs in obstacles {
        let delta = *pos - obs.center;
        let overlap = collider.half_extents + obs.half_extents - delta.abs();
        if overlap.x > 0.0 && overlap.y > 0.0 && overlap.z > 0.0 {
            if overlap.x < overlap.y && overlap.x < overlap.z {
                let sign = if delta.x > 0.0 { 1.0 } else { -1.0 };
                pos.x = obs.center.x + sign * (obs.half_extents.x + collider.half_extents.x);
                body.velocity.x = 0.0;
            } else if overlap.y < overlap.z {
                let sign = if delta.y > 0.0 { 1.0 } else { -1.0 };
                pos.y = obs.center.y + sign * (obs.half_extents.y + collider.half_extents.y);
                body.velocity.y = 0.0;
                if sign > 0.0 {
                    body.on_ground = true;
                }
            } else {
                let sign = if delta.z > 0.0 { 1.0 } else { -1.0 };
                pos.z = obs.center.z + sign * (obs.half_extents.z + collider.half_extents.z);
                body.velocity.z = 0.0;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct PhysicsObject<'a> {
    pub position: &'a mut Vec3,
    pub body: &'a mut RigidBody,
    pub collider: Collider,
}

pub fn resolve_pair(a: &mut PhysicsObject, b: &mut PhysicsObject) -> bool {
    let delta = *a.position - *b.position;
    let overlap = a.collider.half_extents + b.collider.half_extents - delta.abs();
    if overlap.x > 0.0 && overlap.y > 0.0 && overlap.z > 0.0 {
        if overlap.x < overlap.y && overlap.x < overlap.z {
            let sign = if delta.x > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.x * 0.5;
            a.position.x += sign * push;
            b.position.x -= sign * push;
            a.body.velocity.x = 0.0;
            b.body.velocity.x = 0.0;
        } else if overlap.y < overlap.z {
            let sign = if delta.y > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.y * 0.5;
            a.position.y += sign * push;
            b.position.y -= sign * push;
            a.body.velocity.y = 0.0;
            b.body.velocity.y = 0.0;
            if sign > 0.0 {
                a.body.on_ground = true;
            } else {
                b.body.on_ground = true;
            }
        } else {
            let sign = if delta.z > 0.0 { 1.0 } else { -1.0 };
            let push = overlap.z * 0.5;
            a.position.z += sign * push;
            b.position.z -= sign * push;
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
        apply_gravity(obj.body, dt);
        integrate(obj.position, obj.body, dt);
        resolve_aabb_collisions(obj.position, obj.body, &obj.collider, static_obs);
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
