use bevy::prelude::*;

use crate::render::config::RenderConfig;

/// GPU-ready grid of ASCII cells for the output shader.
///
/// Uses separate arrays for the Mage Core 4-texture approach:
/// - char_indices: CP437 glyph index per cell
/// - fg_colors: foreground RGBA per cell
/// - bg_colors: background RGBA per cell
///
/// This layout maps directly to GPU textures without restructuring.
#[derive(Resource)]
pub struct AsciiCellGrid {
    /// Width of the grid in cells.
    pub width: u32,
    /// Height of the grid in cells.
    pub height: u32,
    /// CP437 glyph index for each cell (row-major).
    pub char_indices: Vec<u16>,
    /// Foreground color (RGBA) for each cell (row-major).
    pub fg_colors: Vec<[u8; 4]>,
    /// Background color (RGBA) for each cell (row-major).
    pub bg_colors: Vec<[u8; 4]>,
}

impl FromWorld for AsciiCellGrid {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<RenderConfig>();
        let w = config.ascii_width;
        let h = config.ascii_height;
        let cell_count = (w * h) as usize;
        Self {
            width: w,
            height: h,
            char_indices: vec![0; cell_count],
            fg_colors: vec![[0, 0, 0, 255]; cell_count],
            bg_colors: vec![[0, 0, 0, 255]; cell_count],
        }
    }
}

impl AsciiCellGrid {
    /// Create a standalone AsciiCellGrid with given dimensions.
    ///
    /// Initializes all cells to space glyph (32), black foreground, black background.
    /// Usable outside Bevy ECS for unit tests and golden pipeline tests.
    pub fn new(width: u32, height: u32) -> Self {
        let cell_count = (width * height) as usize;
        Self {
            width,
            height,
            char_indices: vec![32; cell_count],
            fg_colors: vec![[0, 0, 0, 255]; cell_count],
            bg_colors: vec![[0, 0, 0, 255]; cell_count],
        }
    }

    /// Total number of cells in the grid.
    pub fn cells_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Get the cell data at (x, y).
    ///
    /// Returns (char_index, fg_color, bg_color).
    ///
    /// # Panics
    /// Panics if x >= width or y >= height.
    pub fn cell_at(&self, x: u32, y: u32) -> (u16, [u8; 4], [u8; 4]) {
        let idx = (y * self.width + x) as usize;
        (
            self.char_indices[idx],
            self.fg_colors[idx],
            self.bg_colors[idx],
        )
    }

    /// Set the cell data at (x, y).
    ///
    /// # Panics
    /// Panics if x >= width or y >= height.
    pub fn set_cell(&mut self, x: u32, y: u32, char_index: u16, fg: [u8; 4], bg: [u8; 4]) {
        let idx = (y * self.width + x) as usize;
        self.char_indices[idx] = char_index;
        self.fg_colors[idx] = fg;
        self.bg_colors[idx] = bg;
    }
}
