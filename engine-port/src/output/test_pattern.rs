use bevy::prelude::*;

use super::ascii_cell_grid::AsciiCellGrid;

/// Fill an `AsciiCellGrid` with a checkerboard test pattern.
///
/// This is a pure function (no ECS dependency) for testability.
///
/// Checker cells (where `(x + y) % 2 == 0`):
///   - Glyph: 0xDB (full block)
///   - Foreground: orange [255, 128, 0, 255]
///   - Background: dark blue [0, 0, 64, 255]
///
/// Non-checker cells:
///   - Glyph: 0xB1 (medium shade)
///   - Foreground: green [0, 255, 128, 255]
///   - Background: dark red [64, 0, 0, 255]
pub fn fill_test_pattern(grid: &mut AsciiCellGrid) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let checker = (x + y) % 2 == 0;
            if checker {
                grid.set_cell(
                    x,
                    y,
                    0xDB,               // full block
                    [255, 128, 0, 255], // orange foreground
                    [0, 0, 64, 255],    // dark blue background
                );
            } else {
                grid.set_cell(
                    x,
                    y,
                    0xB1,               // medium shade
                    [0, 255, 128, 255], // green foreground
                    [64, 0, 0, 255],    // dark red background
                );
            }
        }
    }
}

/// Bevy system wrapper that fills the `AsciiCellGrid` with the test pattern.
///
/// Added to the Update schedule during Phase 3 for visual validation.
/// Will be replaced by the CPU rasterizer output in Phase 5.
pub fn test_pattern_system(mut grid: ResMut<AsciiCellGrid>) {
    fill_test_pattern(&mut grid);
}
