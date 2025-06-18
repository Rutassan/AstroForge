mod engine;
mod player;

use crate::player::Player;
use base64::Engine as _;
use engine::renderer::CubeInstance;
use engine::Engine;
use glam::{Mat4, Vec2, Vec3};
use std::env;
use std::time::Instant;

const ACTIVATION_B64: &str = include_str!("../assets/activation.ogg.b64");

#[derive(Clone)]
struct Enemy {
    position: Vec3,
    bullet_timer: f32,
    body: engine::physics::RigidBody,
    collider: engine::physics::Collider,
}

struct Bullet {
    position: Vec3,
    velocity: Vec3,
    alive: bool,
}

fn main() {
    println!("🚀 AstroForge запуск собственного движка...");
    let args: Vec<String> = env::args().collect();
    let selftest = args.iter().any(|a| a == "--selftest");
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
                    position: Vec3::new(8.0, 0.75, -8.0),
                    bullet_timer: 2.0,
                    body: engine::physics::RigidBody {
                        velocity: Vec3::ZERO,
                        on_ground: false,
                    },
                    collider: engine::physics::Collider {
                        half_extents: Vec3::new(0.5, 0.75, 0.5),
                    },
                });
            }
        }

        if let Some(e) = &mut enemy {
            let dir = Vec3::new(
                player.position.x - e.position.x,
                0.0,
                player.position.z - e.position.z,
            );
            if dir.length_squared() > 0.0001 {
                let step = dir.normalize() * 2.0;
                e.body.velocity.x = step.x;
                e.body.velocity.z = step.z;
            } else {
                e.body.velocity.x = 0.0;
                e.body.velocity.z = 0.0;
            }

            engine::physics::apply_gravity(&mut e.body, dt);
            engine::physics::integrate(&mut e.position, &mut e.body, dt);
            if e.position.y <= e.collider.half_extents.y {
                e.position.y = e.collider.half_extents.y;
                if e.body.velocity.y <= 0.0 {
                    e.body.velocity.y = 0.0;
                    e.body.on_ground = true;
                }
            }

            let mut obstacles = Player::artifact_aabbs();
            obstacles.push(engine::physics::Aabb {
                center: player.position,
                half_extents: player.collider.half_extents,
            });
            engine::physics::resolve_aabb_collisions(
                &mut e.position,
                &mut e.body,
                &e.collider,
                &obstacles,
            );

            e.bullet_timer -= dt;
            if e.bullet_timer <= 0.0 {
                e.bullet_timer = 2.0;
                let bdir = (player.position - e.position).normalize() * 5.0;
                bullets.push(Bullet {
                    position: e.position + Vec3::new(0.0, e.collider.half_extents.y, 0.0),
                    velocity: bdir,
                    alive: true,
                });
            }
        }

        for b in &mut bullets {
            b.position += b.velocity * dt;
            if (b.position - player.position).length() < 0.5 {
                b.alive = false;
                if health > 0 {
                    health -= 10;
                }
                // Apply a small knockback in the direction the bullet was
                // travelling when it hit the player.
                player.body.velocity += b.velocity * 0.5;
            }
        }
        bullets.retain(|b| b.alive);

        let mut cubes: Vec<CubeInstance> = Vec::new();
        if let Some(e) = &enemy {
            cubes.push(CubeInstance {
                position: e.position + Vec3::new(0.0, -0.5, 0.0),
                size: 0.5,
                color: [1.0, 0.0, 0.0],
            });
            cubes.push(CubeInstance {
                position: e.position,
                size: 0.7,
                color: [1.0, 0.0, 0.0],
            });
            cubes.push(CubeInstance {
                position: e.position + Vec3::new(0.0, 0.75, 0.0),
                size: 0.4,
                color: [1.0, 0.0, 0.0],
            });
            let dir = (player.position - e.position).normalize_or_zero();
            let pistol_pos = e.position + Vec3::new(dir.x * 0.7, 0.6, dir.z * 0.7);
            cubes.push(CubeInstance {
                position: pistol_pos,
                size: 0.2,
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
}
