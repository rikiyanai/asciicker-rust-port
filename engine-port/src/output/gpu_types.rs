use bevy::prelude::*;

use super::ascii_cell_grid::AsciiCellGrid;

/// GPU uniform data matching the WGSL `Uniforms` struct layout.
///
/// Must be 16-byte aligned for GPU uniform buffer requirements.
/// The `_padding` field ensures the struct is exactly 16 bytes.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AsciiUniforms {
    /// Width of a single glyph in pixels.
    pub font_width: u32,
    /// Height of a single glyph in pixels.
    pub font_height: u32,
    /// Padding for 16-byte GPU alignment.
    pub _padding: [u32; 2],
}

/// Main World resource holding font rendering configuration.
///
/// Created during plugin initialization with the loaded font atlas handle.
#[derive(Resource)]
pub struct AsciiRenderConfig {
    /// Width of a single glyph in pixels.
    pub font_width: u32,
    /// Height of a single glyph in pixels.
    pub font_height: u32,
    /// Handle to the font atlas image asset.
    pub font_atlas_handle: Handle<Image>,
}

/// Render World resource containing extracted grid data ready for GPU upload.
///
/// Created each frame by the extract system, consumed by the prepare system.
/// Data is stored as flat byte arrays matching Rgba8Unorm texture format.
#[derive(Resource)]
pub struct ExtractedAsciiGrid {
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// Character index data: 4 bytes per cell (R=index, G=0, B=0, A=255).
    pub char_data: Vec<u8>,
    /// Foreground color data: 4 bytes per cell (RGBA).
    pub fg_data: Vec<u8>,
    /// Background color data: 4 bytes per cell (RGBA).
    pub bg_data: Vec<u8>,
    /// Width of a single glyph in pixels.
    pub font_width: u32,
    /// Height of a single glyph in pixels.
    pub font_height: u32,
}

/// Convert an `AsciiCellGrid` and `AsciiRenderConfig` into GPU-ready byte data.
///
/// - `char_indices` (u16) are encoded as Rgba8 bytes: R = index as u8, G = 0, B = 0, A = 255.
/// - `fg_colors` and `bg_colors` ([u8; 4]) are flattened to contiguous byte arrays.
/// - Font dimensions are copied from the config.
pub fn extract_grid_data(grid: &AsciiCellGrid, config: &AsciiRenderConfig) -> ExtractedAsciiGrid {
    let cell_count = grid.cells_count();

    let mut char_data = Vec::with_capacity(cell_count * 4);
    for &idx in &grid.char_indices {
        debug_assert!(idx <= 255, "Glyph index {idx} exceeds u8 range (0-255)");
        char_data.push(idx as u8); // R channel: glyph index (0-255)
        char_data.push(0); // G channel: unused
        char_data.push(0); // B channel: unused
        char_data.push(255); // A channel: opaque
    }

    let mut fg_data = Vec::with_capacity(cell_count * 4);
    for color in &grid.fg_colors {
        fg_data.extend_from_slice(color);
    }

    let mut bg_data = Vec::with_capacity(cell_count * 4);
    for color in &grid.bg_colors {
        bg_data.extend_from_slice(color);
    }

    ExtractedAsciiGrid {
        width: grid.width,
        height: grid.height,
        char_data,
        fg_data,
        bg_data,
        font_width: config.font_width,
        font_height: config.font_height,
    }
}
