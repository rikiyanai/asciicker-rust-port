use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use asciicker_engine::asset_loader::AssetLoaderPlugin;
use asciicker_engine::audio::AsciickerAudioPlugin;
use asciicker_engine::character::CharacterPlugin;
use asciicker_engine::game::GamePlugin;
use asciicker_engine::network::NetworkPlugin;
use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::physics::PhysicsPlugin;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::render::camera::GameCamera;
use asciicker_engine::render::shape_vector::{ShapeVectorConfig, ShapeVectorFrameStats};
use asciicker_engine::terrain::TerrainPlugin;
use asciicker_engine::world::WorldPlugin;

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
const BUILD_GIT_HASH: &str = env!("ASCIICKER_GIT_HASH");
const BUILD_ITERATION: &str = env!("ASCIICKER_BUILD_ITERATION");

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
            NetworkPlugin,
            GamePlugin,
        ))
        .add_systems(Update, fps_title_system);

    #[cfg(feature = "schedule_dump")]
    app.add_plugins(bevy_mod_debugdump::CommandLineArgs);

    app.run();
}

fn fps_title_system(
    diagnostics: Res<DiagnosticsStore>,
    camera: Option<Res<GameCamera>>,
    shape_vector_config: Option<Res<ShapeVectorConfig>>,
    shape_vector_stats: Option<Res<ShapeVectorFrameStats>>,
    mut windows: Query<&mut Window>,
) {
    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
        && let Some(value) = fps.smoothed()
        && let Ok(mut window) = windows.single_mut()
    {
        let (cam_x, cam_y, cam_z, yaw, zoom) = if let Some(camera) = camera.as_deref() {
            (
                camera.pos[0],
                camera.pos[1],
                camera.pos[2],
                camera.yaw,
                camera.zoom,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        };
        let shape_vector_summary = if let (Some(cfg), Some(stats)) = (
            shape_vector_config.as_deref(),
            shape_vector_stats.as_deref(),
        ) {
            format!(
                " | sv {} {} th {:.3} ab {:.2} at {} fd {:.3} gc {:.2} dc {:.2} q {} gg {} dg {} sf {} | gate {:.0}% ov {:.0}% fb {:.0}% thr {:.0}% clr {:.0}% uw {:.0}% blank {:.0}% cblank {:.0}%",
                cfg.mode.as_str(),
                cfg.alphabet.as_str(),
                cfg.distance_threshold,
                cfg.contrast_adaptive_threshold_boost,
                if cfg.enable_contrast_adaptive_threshold {
                    "on"
                } else {
                    "off"
                },
                cfg.structural_fallback_distance_threshold,
                cfg.global_crunch_exponent,
                cfg.directional_crunch_exponent,
                cfg.sampling_quality,
                if cfg.enable_global_crunch {
                    "on"
                } else {
                    "off"
                },
                if cfg.enable_directional_crunch {
                    "on"
                } else {
                    "off"
                },
                if cfg.enable_structural_fallback {
                    "on"
                } else {
                    "off"
                },
                stats.percent_of_total(stats.semantic_gate_cells),
                stats.percent_of_total(stats.selector_override_cells),
                stats.percent_of_total(stats.resolve_fallback_cells),
                stats.percent_of_total(stats.threshold_skip_cells),
                stats.percent_of_total(stats.clear_skip_cells),
                stats.percent_of_total(stats.underwater_skip_cells),
                stats.percent_of_total(stats.final_space_cells),
                stats.percent_of_total(stats.colored_space_cells),
            )
        } else {
            String::new()
        };

        window.title = format!(
            "asciicker-engine v{} iter {} {} | {:.0} fps | cam {:.2},{:.2},{:.2} | yaw {:.1} | zoom {:.2}{}",
            BUILD_VERSION,
            BUILD_ITERATION,
            BUILD_GIT_HASH,
            value,
            cam_x,
            cam_y,
            cam_z,
            yaw,
            zoom,
            shape_vector_summary,
        );
    }
}
