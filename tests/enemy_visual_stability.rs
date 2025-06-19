// Автотест: проверяет, мигает ли враг (визуальная стабильность)
// Запуск: cargo test --test enemy_visual_stability

use astroforge::engine::renderer::CubeInstance;
use astroforge::engine::Engine;
use astroforge::player::Enemy;

#[test]
fn enemy_visual_stability() {
    let width = 1024u32;
    let height = 768u32;
    // Инициализация движка и врага
    let mut engine = Engine::new_headless(width, height); // Требуется headless-режим
    let mut enemy = Enemy::new();
    let mut cubes = Vec::new();
    // Первый кадр
    cubes.clear();
    enemy.update(0.0); // или нужная логика для первого кадра
    enemy.append_cubes(&mut cubes);
    engine.renderer.render(None, 100, &cubes);
    let frame1 = engine.renderer.get_frame_rgba8();
    // Второй кадр
    cubes.clear();
    enemy.update(1.0/60.0); // следующий кадр
    enemy.append_cubes(&mut cubes);
    engine.renderer.render(None, 100, &cubes);
    let frame2 = engine.renderer.get_frame_rgba8();
    // Сравнение кадров
    assert_eq!(frame1.len(), frame2.len(), "Frame buffer sizes differ");
    let diff = frame1.iter().zip(frame2.iter()).filter(|(a,b)| a != b).count();
    let percent = diff as f32 / frame1.len() as f32;
    assert!(percent < 0.01, "Enemy is visually unstable: {}% pixels differ", percent*100.0);
}
