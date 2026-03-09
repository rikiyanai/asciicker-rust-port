pub mod ascii_cell_grid;
pub mod gpu_plugin;
pub mod gpu_types;
pub mod test_pattern;

use bevy::image::ImageLoaderSettings;
use bevy::prelude::*;
use bevy::window::{Window, WindowResized};

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
        app.add_message::<WindowResized>();
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
        #[cfg(feature = "test_pattern")]
        app.add_systems(
            Update,
            (handle_window_resize, test_pattern::test_pattern_system).chain(),
        );
        #[cfg(not(feature = "test_pattern"))]
        app.add_systems(Update, handle_window_resize);
    }
}

/// Spawns a Camera2d entity required for the ViewNode render pipeline.
///
/// The ASCII output shader replaces the camera's normal view content,
/// but a camera entity must exist for the Core2d render graph to execute.
fn spawn_camera(mut commands: Commands) {
    // Msaa::Off disables multisampling — our fullscreen ASCII shader
    // uses sample_count=1 and doesn't benefit from MSAA on geometry edges.
    commands.spawn((Camera2d, Msaa::Off));
}

/// Recalculates `AsciiCellGrid` dimensions when the window is resized.
///
/// Uses the window's **physical** pixel dimensions (not logical) because the WGSL
/// shader's `@builtin(position)` operates in physical pixel space. On HiDPI/Retina
/// displays the physical size is `logical * scale_factor`. Zero-dimension resizes
/// (window minimized or tiny) are ignored. When dimensions change, all three cell
/// arrays (char_indices, fg_colors, bg_colors) are reallocated at the new cell count.
fn handle_window_resize(
    resize_events: Option<MessageReader<WindowResized>>,
    mut grid: ResMut<AsciiCellGrid>,
    config: Res<AsciiRenderConfig>,
    windows: Query<&Window>,
) {
    let Some(mut resize_events) = resize_events else {
        return;
    };
    // Drain all resize events; only act if at least one arrived this frame.
    if resize_events.read().last().is_none() {
        return;
    }

    // WindowResized reports logical pixels, but the shader samples in physical
    // pixel space. Read physical dimensions directly from the Window component.
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some((new_w, new_h)) = compute_grid_dimensions(
        window.physical_width() as f32,
        window.physical_height() as f32,
        config.font_width,
        config.font_height,
    ) else {
        return;
    };

    if new_w != grid.width || new_h != grid.height {
        let cell_count = (new_w * new_h) as usize;
        grid.width = new_w;
        grid.height = new_h;
        grid.char_indices = vec![0; cell_count];
        grid.fg_colors = vec![[0, 0, 0, 255]; cell_count];
        grid.bg_colors = vec![[0, 0, 0, 255]; cell_count];
    }
}

/// Compute new grid dimensions from window pixel size and font glyph dimensions.
///
/// Returns `None` if either dimension would be zero (e.g., minimized window).
fn compute_grid_dimensions(
    window_width: f32,
    window_height: f32,
    font_width: u32,
    font_height: u32,
) -> Option<(u32, u32)> {
    let w = window_width as u32 / font_width;
    let h = window_height as u32 / font_height;
    if w == 0 || h == 0 { None } else { Some((w, h)) }
}
