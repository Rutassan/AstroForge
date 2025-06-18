use bevy::prelude::*;

mod player;
use player::PlayerPlugin;

fn main() {
    App::new()
        .insert_resource(AmbientLight::NONE)
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .run();
}
