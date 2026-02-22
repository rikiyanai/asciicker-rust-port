use bevy::prelude::*;

use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::output::ascii_cell_grid::AsciiCellGrid;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::render::config::RenderConfig;
use asciicker_engine::render::sample_buffer::SampleBuffer;
use asciicker_engine::terrain::TerrainPlugin;
use asciicker_engine::world::WorldPlugin;

/// Helper: build an app with all required plugins in correct order.
/// Does NOT call app.update() — the pipeline systems need a real asset server.
fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((
        WorldPlugin,
        TerrainPlugin,
        CpuRasterizerPlugin,
        AsciiOutputPlugin,
    ));
    app
}

#[test]
fn sample_buffer_and_grid_coexist() {
    let app = build_app();

    let buffer = app.world().resource::<SampleBuffer>();
    assert_eq!(
        buffer.width, 484,
        "SampleBuffer width should be 484 (240 * 2 + 4)"
    );
    assert_eq!(
        buffer.height, 274,
        "SampleBuffer height should be 274 (135 * 2 + 4)"
    );
    assert_eq!(buffer.samples.len(), 484 * 274);

    let grid = app.world().resource::<AsciiCellGrid>();
    assert_eq!(grid.width, 240, "AsciiCellGrid width should be 240");
    assert_eq!(grid.height, 135, "AsciiCellGrid height should be 135");
    assert_eq!(grid.cells_count(), 240 * 135);
}

#[test]
fn render_config_controls_dimensions() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Insert custom config before plugins -- dimensions are always 2*ascii+4
    app.insert_resource(RenderConfig {
        ascii_width: 80,
        ascii_height: 40,
    });
    app.add_plugins((
        WorldPlugin,
        TerrainPlugin,
        CpuRasterizerPlugin,
        AsciiOutputPlugin,
    ));

    let buffer = app.world().resource::<SampleBuffer>();
    assert_eq!(
        buffer.width, 164,
        "SampleBuffer width should be 80 * 2 + 4 = 164"
    );
    assert_eq!(
        buffer.height, 84,
        "SampleBuffer height should be 40 * 2 + 4 = 84"
    );

    let grid = app.world().resource::<AsciiCellGrid>();
    assert_eq!(grid.width, 80);
    assert_eq!(grid.height, 40);
}

#[test]
fn write_sample_read_cell_same_frame() {
    let mut app = build_app();

    // Write to SampleBuffer using new field names
    let mut buffer = app.world_mut().resource_mut::<SampleBuffer>();
    buffer.sample_at_mut(100, 50).height = 10.0;
    buffer.sample_at_mut(100, 50).visual = 0x7FFF;

    // Read from AsciiCellGrid (verify it exists and is accessible in same world)
    let mut grid = app.world_mut().resource_mut::<AsciiCellGrid>();
    grid.set_cell(50, 25, 65, [255, 255, 255, 255], [0, 0, 0, 255]);
    let (char_idx, fg, bg) = grid.cell_at(50, 25);
    assert_eq!(char_idx, 65);
    assert_eq!(fg, [255, 255, 255, 255]);
    assert_eq!(bg, [0, 0, 0, 255]);
}

#[test]
fn separate_gpu_arrays_verified() {
    let mut app = build_app();

    let grid = app.world().resource::<AsciiCellGrid>();

    // Verify all three arrays exist and have correct lengths
    let expected_cells = 240 * 135;
    assert_eq!(grid.char_indices.len(), expected_cells);
    assert_eq!(grid.fg_colors.len(), expected_cells);
    assert_eq!(grid.bg_colors.len(), expected_cells);

    // Verify arrays are separate (modifying one doesn't affect others)
    let mut grid = app.world_mut().resource_mut::<AsciiCellGrid>();
    let fg_before = grid.fg_colors[0];
    let bg_before = grid.bg_colors[0];
    grid.char_indices[0] = 42;
    assert_eq!(grid.fg_colors[0], fg_before); // unchanged by char_indices mutation
    assert_eq!(grid.bg_colors[0], bg_before); // unchanged by char_indices mutation
}
