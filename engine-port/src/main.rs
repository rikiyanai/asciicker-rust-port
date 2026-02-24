use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
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
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            AssetLoaderPlugin,
            WorldPlugin,
            TerrainPlugin,
            CpuRasterizerPlugin,
            AsciiOutputPlugin,
            PhysicsPlugin,
            CharacterPlugin,
            GamePlugin,
        ))
        .add_systems(Update, fps_title_system);

    #[cfg(feature = "schedule_dump")]
    app.add_plugins(bevy_mod_debugdump::CommandLineArgs);

    app.run();
}

fn fps_title_system(diagnostics: Res<DiagnosticsStore>, mut windows: Query<&mut Window>) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(value) = fps.smoothed() {
            if let Ok(mut window) = windows.single_mut() {
                window.title = format!("asciicker-engine | {:.0} fps", value);
            }
        }
    }
}
