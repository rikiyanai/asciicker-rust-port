pub mod ascii_cell_grid;
pub mod capture;
pub mod gpu_plugin;
pub mod gpu_types;
pub mod replay;
pub mod test_pattern;

use bevy::image::ImageLoaderSettings;
use bevy::prelude::*;
use bevy::window::Window;

use ascii_cell_grid::AsciiCellGrid;
use capture::{VisualCaptureConfig, VisualCaptureState, visual_capture_system};
use gpu_types::AsciiRenderConfig;
use replay::{
    ReplayHarnessConfig, ReplayHarnessState, baseline_apply_replay_system,
    baseline_auto_start_system, baseline_capture_system, baseline_load_trace_system,
    baseline_orbit_trigger_system, baseline_variant_overlay_system,
};

use crate::game::weather::weather_composite_system;
use crate::render::pipeline::render_pipeline_system;
use crate::render::workbench::RenderWorkbenchState;

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
        app.init_resource::<VisualCaptureConfig>();
        app.init_resource::<VisualCaptureState>();
        app.init_resource::<ReplayHarnessConfig>();
        app.init_resource::<ReplayHarnessState>();

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
        app.add_systems(
            Update,
            (
                baseline_load_trace_system,
                baseline_auto_start_system,
                baseline_orbit_trigger_system,
                baseline_apply_replay_system,
                visual_capture_system,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                baseline_variant_overlay_system
                    .after(render_pipeline_system)
                    .after(weather_composite_system),
                baseline_capture_system
                    .after(baseline_variant_overlay_system)
                    .after(render_pipeline_system)
                    .after(weather_composite_system),
            ),
        );
        #[cfg(feature = "test_pattern")]
        app.add_systems(
            Update,
            (sync_grid_dimensions, test_pattern::test_pattern_system).chain(),
        );
        #[cfg(not(feature = "test_pattern"))]
        app.add_systems(Update, sync_grid_dimensions);
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
fn sync_grid_dimensions(
    mut grid: ResMut<AsciiCellGrid>,
    config: Res<AsciiRenderConfig>,
    windows: Query<&Window>,
    workbench: Option<Res<RenderWorkbenchState>>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some((base_w, base_h)) = compute_grid_dimensions(
        window.physical_width() as f32,
        window.physical_height() as f32,
        config.font_width,
        config.font_height,
    ) else {
        return;
    };
    let resolution_scale = workbench
        .as_deref()
        .map(|state| state.resolution_scale.clamp(0.25, 1.0))
        .unwrap_or(1.0);
    let new_w = ((base_w as f32) * resolution_scale).round().max(1.0) as u32;
    let new_h = ((base_h as f32) * resolution_scale).round().max(1.0) as u32;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_1280x720_with_10x16_font() {
        // 1280 / 10 = 128, 720 / 16 = 45
        let result = compute_grid_dimensions(1280.0, 720.0, 10, 16);
        assert_eq!(result, Some((128, 45)));
    }

    #[test]
    fn resize_1920x1080_with_10x16_font() {
        // 1920 / 10 = 192, 1080 / 16 = 67
        let result = compute_grid_dimensions(1920.0, 1080.0, 10, 16);
        assert_eq!(result, Some((192, 67)));
    }

    #[test]
    fn resize_zero_width_returns_none() {
        let result = compute_grid_dimensions(5.0, 720.0, 10, 16);
        assert_eq!(result, None);
    }

    #[test]
    fn resize_zero_height_returns_none() {
        let result = compute_grid_dimensions(1280.0, 10.0, 10, 16);
        assert_eq!(result, None);
    }

    #[test]
    fn resize_default_2400x2160_with_10x16_font() {
        // Physical pixel size that produces the default 240x135 grid
        let result = compute_grid_dimensions(2400.0, 2160.0, 10, 16);
        assert_eq!(result, Some((240, 135)));
    }

    #[test]
    fn resize_retina_2x_1280x720_logical_with_10x16_font() {
        // 2x Retina: logical 1280x720 → physical 2560x1440
        // Physical: 2560 / 10 = 256, 1440 / 16 = 90
        let result = compute_grid_dimensions(2560.0, 1440.0, 10, 16);
        assert_eq!(result, Some((256, 90)));
    }

    #[test]
    fn resize_retina_1_5x_1280x720_logical_with_10x16_font() {
        // 1.5x scale: logical 1280x720 → physical 1920x1080
        // Physical: 1920 / 10 = 192, 1080 / 16 = 67
        let result = compute_grid_dimensions(1920.0, 1080.0, 10, 16);
        assert_eq!(result, Some((192, 67)));
    }
}
