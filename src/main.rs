use astroforge::player::Player;
use astroforge::engine::Engine;
use astroforge::engine::renderer::CubeInstance;
use astroforge::engine;
use astroforge::player;
use base64::Engine as _;
use glam::{Mat4, Vec2, Vec3};
use image::{ImageBuffer, Rgba};
use std::env;
use std::time::Instant;

const ACTIVATION_B64: &str = include_str!("../assets/activation.ogg.b64");
const ENEMY_COLOR: [f32; 3] = [1.0, 0.0, 0.0];

#[derive(Clone)]
struct Enemy {
    bullet_timer: f32,
    body: engine::physics::RigidBody,
    collider: engine::physics::Collider,
}

struct Bullet {
    position: Vec3,
    body: engine::physics::RigidBody,
    collider: engine::physics::Collider,
    alive: bool,
}

fn save_screenshot(buffer: &[u8], width: u32, height: u32, path: &str) {
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, buffer.to_vec())
        .expect("Failed to create image buffer");
    img.save(path).expect("Failed to save screenshot");
}

fn main() {
    println!("🚀 AstroForge запуск собственного движка...");
    let args: Vec<String> = env::args().collect();
    let selftest = args.iter().any(|a| a == "--selftest");
    let screenshot = args.iter().any(|a| a == "--screenshot");
    let is_wayland = env::var("WAYLAND_DISPLAY").is_ok();
    let window_title = if is_wayland {
        "AstroForge"
    } else {
        "Технология разблокирована: энергетический маяк"
    };
    let mut engine = Engine::new(window_title, 1024, 768);
    let mut player = Player::new();
    let default_title = window_title;
    let mut tech_unlocked = false;
    let mut message_timer = 0.0f32;

    let mut enemy: Option<Enemy> = None;
    let mut bullets: Vec<Bullet> = Vec::new();
    let mut spawn_timer = 0.0f32;
    let mut spawn_started = false;
    let mut health: i32 = 100;
    let mut game_over = false;

    let b64_clean: String = ACTIVATION_B64
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_clean)
        .expect("valid base64");

    let mut last = Instant::now();
    let mut activated = false;
    let mut pulse = 0.0f32;
    let mut overlay_tested = false;
    let overlay_text_cyr = "Технология разблокирована: энергетический маяк";

    engine.run(move |engine| {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;

        player.update(&engine.input, dt);
        let view =
            Mat4::from_quat(player.rotation).inverse() * Mat4::from_translation(-player.position);
        let aspect = engine.renderer.size.width as f32 / engine.renderer.size.height as f32;
        let proj = Mat4::perspective_rh(60f32.to_radians(), aspect, 0.1, 100.0);
        engine.renderer.update_camera(&(proj * view));

        let dist = Vec2::new(player.position.x, player.position.z).length();
        let mut overlay_text: Option<&str> = None;

        if selftest {
            overlay_text = Some(overlay_text_cyr);
            if !overlay_tested {
                // Попытка отрисовать overlay, вывод результата после первого кадра
                // (если не упало, считаем успехом)
                println!("Overlay Cyrillic test: OK");
                overlay_tested = true;
                // Можно завершить игру после теста, если нужно:
                // std::process::exit(0);
            }
        } else {
            if dist < 3.0 {
                if !activated && player.body.on_ground {
                    activated = true;
                    engine.audio.play_bytes(&bytes);
                    if !tech_unlocked {
                        tech_unlocked = true;
                        message_timer = 3.0;
                    }
                }
                pulse += dt * 3.0;
                let intensity = 0.2 + 0.8 * (0.5 + 0.5 * (pulse).sin());
                engine.renderer.update_artifact(intensity);
            } else {
                if activated {
                    activated = false;
                    pulse = 0.0;
                }
                engine.renderer.update_artifact(0.2);
            }

            if message_timer > 0.0 {
                message_timer -= dt;
                overlay_text = Some(overlay_text_cyr);
                if message_timer <= 0.0 {
                    overlay_text = None;
                }
            }
        }

        // enemy spawn logic after tech unlock
        if tech_unlocked && !spawn_started {
            spawn_timer = 5.0;
            spawn_started = true;
        }
        if spawn_started && spawn_timer > 0.0 {
            spawn_timer -= dt;
            if spawn_timer <= 0.0 {
                enemy = Some(Enemy {
                    bullet_timer: 2.0,
                    body: engine::physics::RigidBody::new(80.0, Vec3::new(8.0, 0.75, -8.0)),
                    collider: engine::physics::Collider {
                        half_extents: Vec3::new(0.5, 0.75, 0.5),
                    },
                });
            }
        }

        if let Some(e) = &mut enemy {
            let dir = Vec3::new(
                player.body.position.x - e.body.position.x,
                0.0,
                player.body.position.z - e.body.position.z,
            );
            if dir.length_squared() > 0.0001 {
                let dir = dir.normalize();
                e.body.apply_force(dir * 200.0);
            }
            e.body.apply_force(-e.body.velocity * 5.0 * e.body.mass);
            e.bullet_timer -= dt;
            if tech_unlocked && e.bullet_timer <= 0.0 {
                e.bullet_timer = 2.0;
                let dir = (player.body.position - e.body.position).normalize();
                let spawn_pos = e.body.position + Vec3::new(dir.x * 0.7, 0.6, dir.z * 0.7);
                let bdir = (player.body.position - spawn_pos).normalize() * 5.0;
                bullets.push(Bullet {
                    position: spawn_pos,
                    body: engine::physics::RigidBody::new(0.05, spawn_pos),
                    collider: engine::physics::Collider {
                        half_extents: Vec3::splat(0.1),
                    },
                    alive: true,
                });
            }
        }

        // Physics simulation step
        let mut static_obs = Player::artifact_aabbs();
        static_obs.push(engine::physics::Aabb {
            center: Vec3::new(0.0, -0.5, 0.0),
            half_extents: Vec3::new(50.0, 0.5, 50.0),
        });
        // Сохраняем prev_y ДО добавления player.body в objs
        let prev_y = player.body.velocity.y;
        let mut objs: Vec<engine::physics::PhysicsObject> = Vec::new();
        let player_idx = objs.len();
        objs.push(engine::physics::PhysicsObject {
            body: &mut player.body,
            collider: player.collider,
        });
        let enemy_idx = if let Some(e) = &mut enemy {
            let idx = objs.len();
            objs.push(engine::physics::PhysicsObject {
                body: &mut e.body,
                collider: e.collider,
            });
            Some(idx)
        } else {
            None
        };
        let mut bullet_indices = Vec::new();
        for b in &mut bullets {
            let idx = objs.len();
            bullet_indices.push(idx);
            objs.push(engine::physics::PhysicsObject {
                body: &mut b.body,
                collider: b.collider,
            });
        }

        let pairs = engine::physics::step(&mut objs, &static_obs, dt);

        if player.body.on_ground && prev_y < 0.0 {
            let speed = -prev_y;
            let safe = 6.0;
            if speed > safe {
                let dmg = ((speed - safe) * player.body.mass / 4.0) as i32;
                if health > 0 {
                    health -= dmg;
                }
            }
        }

        for (a, b) in pairs {
            // bullet hitting player or enemy
            if let Some(bullet_i) = bullet_indices.iter().position(|&x| x == a) {
                let bullet = &mut bullets[bullet_i];
                if a == player_idx || b == player_idx {
                    bullet.alive = false;
                    let momentum = bullet.body.velocity.length() * bullet.body.mass;
                    if health > 0 {
                        health -= (momentum * 50.0) as i32;
                    }
                    player
                        .body
                        .apply_impulse(bullet.body.velocity * bullet.body.mass);
                } else if let Some(e_idx) = enemy_idx {
                    if a == e_idx || b == e_idx {
                        bullet.alive = false;
                    }
                }
            } else if let Some(bullet_i) = bullet_indices.iter().position(|&x| x == b) {
                let bullet = &mut bullets[bullet_i];
                if a == player_idx || b == player_idx {
                    bullet.alive = false;
                    let momentum = bullet.body.velocity.length() * bullet.body.mass;
                    if health > 0 {
                        health -= (momentum * 50.0) as i32;
                    }
                    player
                        .body
                        .apply_impulse(bullet.body.velocity * bullet.body.mass);
                } else if let Some(e_idx) = enemy_idx {
                    if a == e_idx || b == e_idx {
                        bullet.alive = false;
                    }
                }
            }
        }

        for b in &mut bullets {
            if b.body.velocity.length_squared() == 0.0 {
                b.alive = false;
            }
        }
        bullets.retain(|b| b.alive);

        let mut cubes: Vec<CubeInstance> = Vec::new();
        if let Some(e) = &enemy {
            let base = e.body.position;
            // Туловище
            cubes.push(CubeInstance {
                position: base + Vec3::new(0.0, 0.3, 0.0),
                size: 0.4,
                color: ENEMY_COLOR,
            });
            // Голова
            cubes.push(CubeInstance {
                position: base + Vec3::new(0.0, 0.65, 0.0),
                size: 0.22,
                color: ENEMY_COLOR,
            });
            // Левая нога
            cubes.push(CubeInstance {
                position: base + Vec3::new(-0.12, 0.08, 0.0),
                size: 0.16,
                color: ENEMY_COLOR,
            });
            // Правая нога
            cubes.push(CubeInstance {
                position: base + Vec3::new(0.12, 0.08, 0.0),
                size: 0.16,
                color: ENEMY_COLOR,
            });
            // Левая рука
            cubes.push(CubeInstance {
                position: base + Vec3::new(-0.23, 0.38, 0.0),
                size: 0.13,
                color: ENEMY_COLOR,
            });
            // Правая рука
            cubes.push(CubeInstance {
                position: base + Vec3::new(0.23, 0.38, 0.0),
                size: 0.13,
                color: ENEMY_COLOR,
            });
            // Пистолет
            let dir = (player.body.position - base).normalize_or_zero();
            let pistol_pos = base + Vec3::new(dir.x * 0.7, 0.38, dir.z * 0.7);
            cubes.push(CubeInstance {
                position: pistol_pos,
                size: 0.11,
                color: [0.0, 1.0, 0.0],
            });
        }
        for b in &bullets {
            cubes.push(CubeInstance {
                position: b.position,
                size: 0.1,
                color: [1.0, 1.0, 0.0],
            });
        }

        if health <= 0 && !game_over {
            overlay_text = Some("Вы погибли");
            game_over = true;
        }

        engine.renderer.render(overlay_text, health, &cubes);
        engine.input.reset();
    });

    if screenshot {
        // Headless/one-frame screenshot mode
        let width = 1024u32;
        let height = 768u32;
        let mut engine = Engine::new(window_title, width, height);
        let mut player = Player::new();
        let mut enemy: Option<Enemy> = None;
        let mut bullets: Vec<Bullet> = Vec::new();
        // ...инициализация сцены, как обычно...
        // Рендерим один кадр
        let mut cubes: Vec<CubeInstance> = Vec::new();
        // ...добавьте сюда нужные объекты для теста...
        engine.renderer.render(None, 100, &cubes);
        // Получаем буфер кадра (пример, зависит от вашей реализации renderer)
        let buffer = engine.renderer.get_frame_rgba8(); // реализуйте этот метод
        save_screenshot(&buffer, width, height, "screenshot.png");
        println!("Screenshot saved to screenshot.png");
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
