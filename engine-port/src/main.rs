use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use asciicker_engine::asset_loader::AssetLoaderPlugin;
use asciicker_engine::audio::AsciickerAudioPlugin;
use asciicker_engine::character::CharacterPlugin;
use asciicker_engine::game::GamePlugin;
use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::physics::PhysicsPlugin;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::terrain::TerrainPlugin;
use asciicker_engine::world::WorldPlugin;

fn main() {
    let mut app = App::new();
    // IMPORTANT: Each plugin is registered independently.
    // None of these plugins add each other as sub-plugins.
    // GamePlugin/PhysicsPlugin/CharacterPlugin only add their
    // own resources and systems -- Bevy panics on duplicate plugins.
    // CpuRasterizerPlugin BEFORE AsciiOutputPlugin -- AsciiOutputPlugin
    // needs RenderConfig (inserted by CpuRasterizerPlugin.build()).
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
            AsciickerAudioPlugin,
            GamePlugin,
        ))
        .add_systems(Update, fps_title_system);

    #[cfg(feature = "schedule_dump")]
    app.add_plugins(bevy_mod_debugdump::CommandLineArgs);

    app.run();
}

fn fps_title_system(diagnostics: Res<DiagnosticsStore>, mut windows: Query<&mut Window>) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
        && let Some(value) = fps.smoothed()
        && let Ok(mut window) = windows.single_mut()
    {
        window.title = format!("asciicker-engine | {:.0} fps", value);
    }
}
