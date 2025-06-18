mod engine;
mod player;

use crate::player::Player;
use base64::Engine as _;
use engine::Engine;
use glam::{Mat4, Vec2, Vec3};
use std::time::Instant;

const ACTIVATION_B64: &str = include_str!("../assets/activation.ogg.b64");

fn main() {
    println!("🚀 AstroForge запуск собственного движка...");
    let mut engine = Engine::new("AstroForge", 1024, 768);
    let mut player = Player::new();

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
    engine.run(move |engine| {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;

        player.update(&engine.input, dt);
        let view =
            Mat4::from_quat(player.rotation).inverse() * Mat4::from_translation(-player.position);
        let aspect = engine.renderer.size.width as f32 / engine.renderer.size.height as f32;
        // WGPU expects a projection matrix with a `[0, 1]` depth range.
        let proj = Mat4::perspective_rh(60f32.to_radians(), aspect, 0.1, 100.0);
        engine.renderer.update_camera(&(proj * view));

        let dist = Vec2::new(player.position.x, player.position.z).length();
        if dist < 3.0 {
            if !activated && player.body.on_ground {
                activated = true;
                engine.audio.play_bytes(&bytes);
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

        engine.input.reset();
    });
}
