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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(width: u32, height: u32) -> AsciiCellGrid {
        let cell_count = (width * height) as usize;
        AsciiCellGrid {
            width,
            height,
            char_indices: vec![0; cell_count],
            fg_colors: vec![[0, 0, 0, 255]; cell_count],
            bg_colors: vec![[0, 0, 0, 255]; cell_count],
        }
    }

    #[test]
    fn checker_cell_at_origin() {
        let mut grid = make_grid(4, 4);
        fill_test_pattern(&mut grid);

        // (0,0): (0+0)%2 == 0 -> checker cell
        let (char_idx, fg, bg) = grid.cell_at(0, 0);
        assert_eq!(char_idx, 0xDB);
        assert_eq!(fg, [255, 128, 0, 255]);
        assert_eq!(bg, [0, 0, 64, 255]);
    }

    #[test]
    fn non_checker_cell() {
        let mut grid = make_grid(4, 4);
        fill_test_pattern(&mut grid);

        // (1,0): (1+0)%2 == 1 -> non-checker cell
        let (char_idx, fg, bg) = grid.cell_at(1, 0);
        assert_eq!(char_idx, 0xB1);
        assert_eq!(fg, [0, 255, 128, 255]);
        assert_eq!(bg, [64, 0, 0, 255]);
    }

    #[test]
    fn checker_alternates_rows() {
        let mut grid = make_grid(4, 4);
        fill_test_pattern(&mut grid);

        // (0,1): (0+1)%2 == 1 -> non-checker
        let (char_idx, _, _) = grid.cell_at(0, 1);
        assert_eq!(char_idx, 0xB1);

        // (1,1): (1+1)%2 == 0 -> checker
        let (char_idx, _, _) = grid.cell_at(1, 1);
        assert_eq!(char_idx, 0xDB);
    }

    #[test]
    fn all_cells_filled() {
        let mut grid = make_grid(4, 4);
        fill_test_pattern(&mut grid);

        // Every cell should have either 0xDB or 0xB1
        for y in 0..4u32 {
            for x in 0..4u32 {
                let (char_idx, _, _) = grid.cell_at(x, y);
                assert!(
                    char_idx == 0xDB || char_idx == 0xB1,
                    "Cell ({x},{y}) has unexpected glyph {char_idx:#x}"
                );
            }
        }
    }
}
