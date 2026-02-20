pub mod ascii_cell_grid;
pub mod gpu_plugin;
pub mod gpu_types;
pub mod test_pattern;

use bevy::image::ImageLoaderSettings;
use bevy::prelude::*;

use ascii_cell_grid::AsciiCellGrid;
use gpu_types::AsciiRenderConfig;

/// Plugin that sets up the ASCII output pipeline.
///
/// Initializes:
/// - `AsciiCellGrid` resource (CPU-side grid data)
/// - `AsciiRenderConfig` resource (font atlas handle + glyph dimensions)
/// - `AsciiGpuPlugin` sub-plugin (render pipeline in RenderApp)
/// - Camera2d entity (required for ViewNode rendering)
/// - Test pattern system (fills grid with checkerboard each frame)
pub struct AsciiOutputPlugin;

impl Plugin for AsciiOutputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AsciiCellGrid>();

        // Load font atlas with is_srgb=false to ensure Rgba8Unorm format (Pitfall 1).
        // This prevents gamma correction artifacts on data textures.
        let font_atlas_handle: Handle<Image> =
            if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
                asset_server.load_with_settings(
                    "fonts/cp437_10x16.png",
                    |settings: &mut ImageLoaderSettings| {
                        settings.is_srgb = false;
                    },
                )
            } else {
                Handle::default()
            };

        app.insert_resource(AsciiRenderConfig {
            font_width: 10,
            font_height: 16,
            font_atlas_handle,
        });

        app.add_plugins(gpu_plugin::AsciiGpuPlugin);
        app.add_systems(Startup, spawn_camera);
        app.add_systems(Update, test_pattern::test_pattern_system);
    }
}

/// Spawns a Camera2d entity required for the ViewNode render pipeline.
///
/// The ASCII output shader replaces the camera's normal view content,
/// but a camera entity must exist for the Core2d render graph to execute.
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
