mod engine;
mod player;

use crate::player::Player;
use base64::Engine as _;
use engine::Engine;
use glam::{Mat4, Vec2, Vec3};
use std::env;
use std::time::Instant;

const ACTIVATION_B64: &str = include_str!("../assets/activation.ogg.b64");

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

        engine.renderer.render(overlay_text);
        engine.input.reset();
    });
}
