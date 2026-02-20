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

    fn make_config() -> AsciiRenderConfig {
        AsciiRenderConfig {
            font_width: 10,
            font_height: 16,
            font_atlas_handle: Handle::default(),
        }
    }

    #[test]
    fn extract_2x2_grid_char_data_encoding() {
        let mut grid = make_grid(2, 2);
        grid.char_indices[0] = 65; // 'A'
        grid.char_indices[1] = 0xDB; // Full block (219)
        grid.char_indices[2] = 0xB1; // Medium shade (177)
        grid.char_indices[3] = 32; // Space

        let config = make_config();
        let extracted = extract_grid_data(&grid, &config);

        // 4 cells * 4 bytes = 16 bytes
        assert_eq!(extracted.char_data.len(), 16);

        // Cell 0: A (65) -> R=65, G=0, B=0, A=255
        assert_eq!(extracted.char_data[0], 65);
        assert_eq!(extracted.char_data[1], 0);
        assert_eq!(extracted.char_data[2], 0);
        assert_eq!(extracted.char_data[3], 255);

        // Cell 1: Full block (219) -> R=219, G=0, B=0, A=255
        assert_eq!(extracted.char_data[4], 219);
        assert_eq!(extracted.char_data[5], 0);
        assert_eq!(extracted.char_data[6], 0);
        assert_eq!(extracted.char_data[7], 255);

        // Cell 2: Medium shade (177) -> R=177, G=0, B=0, A=255
        assert_eq!(extracted.char_data[8], 177);

        // Cell 3: Space (32) -> R=32
        assert_eq!(extracted.char_data[12], 32);
    }

    #[test]
    fn extract_2x2_grid_fg_bg_flattening() {
        let mut grid = make_grid(2, 2);
        grid.fg_colors[0] = [255, 128, 0, 255];
        grid.fg_colors[1] = [0, 255, 128, 255];
        grid.bg_colors[0] = [0, 0, 64, 255];
        grid.bg_colors[1] = [64, 0, 0, 255];

        let config = make_config();
        let extracted = extract_grid_data(&grid, &config);

        // FG data: 4 cells * 4 bytes = 16 bytes
        assert_eq!(extracted.fg_data.len(), 16);
        assert_eq!(&extracted.fg_data[0..4], &[255, 128, 0, 255]);
        assert_eq!(&extracted.fg_data[4..8], &[0, 255, 128, 255]);

        // BG data
        assert_eq!(extracted.bg_data.len(), 16);
        assert_eq!(&extracted.bg_data[0..4], &[0, 0, 64, 255]);
        assert_eq!(&extracted.bg_data[4..8], &[64, 0, 0, 255]);
    }

    #[test]
    fn extract_preserves_dimensions_and_font_sizes() {
        let grid = make_grid(2, 2);
        let config = make_config();
        let extracted = extract_grid_data(&grid, &config);

        assert_eq!(extracted.width, 2);
        assert_eq!(extracted.height, 2);
        assert_eq!(extracted.font_width, 10);
        assert_eq!(extracted.font_height, 16);
    }

    #[test]
    fn uniforms_struct_is_16_bytes() {
        assert_eq!(std::mem::size_of::<AsciiUniforms>(), 16);
    }

    #[test]
    fn uniforms_pod_cast() {
        let uniforms = AsciiUniforms {
            font_width: 10,
            font_height: 16,
            _padding: [0; 2],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&uniforms);
        assert_eq!(bytes.len(), 16);
        // First 4 bytes: font_width = 10 (little-endian)
        assert_eq!(bytes[0], 10);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], 0);
        // Next 4 bytes: font_height = 16 (little-endian)
        assert_eq!(bytes[4], 16);
        assert_eq!(bytes[5], 0);
        assert_eq!(bytes[6], 0);
        assert_eq!(bytes[7], 0);
    }
}
