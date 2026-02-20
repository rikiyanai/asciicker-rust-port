use bevy::prelude::*;

use asciicker_engine::asset_loader::AssetLoaderPlugin;
use asciicker_engine::character::CharacterPlugin;
use asciicker_engine::game::GamePlugin;
use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::physics::PhysicsPlugin;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::terrain::TerrainPlugin;
use asciicker_engine::world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            AssetLoaderPlugin,
            WorldPlugin,
            TerrainPlugin,
            CpuRasterizerPlugin,
            AsciiOutputPlugin,
            PhysicsPlugin,
            CharacterPlugin,
            GamePlugin,
        ))
        .run();
}
