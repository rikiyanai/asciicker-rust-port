use bevy::prelude::*;

use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::output::ascii_cell_grid::AsciiCellGrid;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::render::config::RenderConfig;
use asciicker_engine::render::sample_buffer::SampleBuffer;

#[test]
fn sample_buffer_and_grid_coexist() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((CpuRasterizerPlugin, AsciiOutputPlugin));

    app.update();

    let buffer = app.world().resource::<SampleBuffer>();
    assert_eq!(
        buffer.width, 480,
        "SampleBuffer width should be 480 (240 * 2)"
    );
    assert_eq!(
        buffer.height, 270,
        "SampleBuffer height should be 270 (135 * 2)"
    );
    assert_eq!(buffer.samples.len(), 480 * 270);

    let grid = app.world().resource::<AsciiCellGrid>();
    assert_eq!(grid.width, 240, "AsciiCellGrid width should be 240");
    assert_eq!(grid.height, 135, "AsciiCellGrid height should be 135");
    assert_eq!(grid.cells_count(), 240 * 135);
}

#[test]
fn render_config_controls_dimensions() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Insert custom config before plugins
    app.insert_resource(RenderConfig {
        ascii_width: 80,
        ascii_height: 40,
        supersample_factor: 3,
    });
    app.add_plugins((CpuRasterizerPlugin, AsciiOutputPlugin));

    app.update();

    let buffer = app.world().resource::<SampleBuffer>();
    assert_eq!(
        buffer.width, 240,
        "SampleBuffer width should be 80 * 3 = 240"
    );
    assert_eq!(
        buffer.height, 120,
        "SampleBuffer height should be 40 * 3 = 120"
    );

    let grid = app.world().resource::<AsciiCellGrid>();
    assert_eq!(grid.width, 80);
    assert_eq!(grid.height, 40);
}

#[test]
fn write_sample_read_cell_same_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((CpuRasterizerPlugin, AsciiOutputPlugin));

    app.update();

    // Write to SampleBuffer
    let mut buffer = app.world_mut().resource_mut::<SampleBuffer>();
    buffer.sample_at_mut(100, 50).depth = 10.0;
    buffer.sample_at_mut(100, 50).glyph = 65;
    buffer.sample_at_mut(100, 50).color_rgb555 = 0x7FFF;

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
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((CpuRasterizerPlugin, AsciiOutputPlugin));

    app.update();

    let grid = app.world().resource::<AsciiCellGrid>();

    // Verify all three arrays exist and have correct lengths
    let expected_cells = 240 * 135;
    assert_eq!(grid.char_indices.len(), expected_cells);
    assert_eq!(grid.fg_colors.len(), expected_cells);
    assert_eq!(grid.bg_colors.len(), expected_cells);

    // Verify arrays are separate (modifying one doesn't affect others)
    let mut grid = app.world_mut().resource_mut::<AsciiCellGrid>();
    grid.char_indices[0] = 42;
    assert_eq!(grid.fg_colors[0], [0, 0, 0, 255]); // unchanged
    assert_eq!(grid.bg_colors[0], [0, 0, 0, 255]); // unchanged
}
