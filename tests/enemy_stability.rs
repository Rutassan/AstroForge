// Автотест для проверки стабильности позиций врага на протяжении 300 кадров
// Запускать через: cargo test --test enemy_stability

use astroforge::engine::physics::{RigidBody, Collider, Aabb, PhysicsObject, step};
use glam::Vec3;

#[test]
fn enemy_position_stability() {
    let mut body = RigidBody::new(80.0, Vec3::new(8.0, 0.75, -8.0));
    let collider = Collider { half_extents: Vec3::new(0.5, 0.75, 0.5) };
    let static_obs = vec![Aabb { center: Vec3::new(0.0, -0.5, 0.0), half_extents: Vec3::new(50.0, 0.5, 50.0) }];
    let mut positions = Vec::new();
    for _ in 0..300 {
        let mut obj = PhysicsObject { body: &mut body, collider };
        let mut objs = vec![obj];
        step(&mut objs, &static_obs, 1.0/60.0);
        positions.push(body.position);
    }
    // Проверяем, что позиция не "скачет" по y
    let mut max_diff = 0.0;
    for w in positions.windows(2) {
        let dy = (w[1].y - w[0].y).abs();
        if dy > max_diff { max_diff = dy; }
    }
    assert!(max_diff < 0.01, "Enemy Y position is unstable: max diff = {}", max_diff);
}
