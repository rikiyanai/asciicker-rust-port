//! XP sprite file parser.
//!
//! Parses gzip-compressed REXPaint .xp files into structured sprite data.
//! Cell data is stored in column-major order per the REXPaint format.
//! Layer semantics: L0 = colorkey/transparency, L1 = height, L2 = visual,
//! L3+ = overwrites, last layer = swoosh merge.

use std::io::Read;

use flate2::read::GzDecoder;

use super::constants::{
    SPRITE_CYAN, SPRITE_GLYPH_HALF_LEFT, SPRITE_GLYPH_HALF_LOWER, SPRITE_GLYPH_HALF_RIGHT,
    SPRITE_GLYPH_HALF_UPPER, SPRITE_HEIGHT_UNDEFINED, SPRITE_LIGHTEN_AMOUNT, SPRITE_MIN_LAYERS,
};
use super::error::AssetError;

/// A single cell in an XP sprite layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XpCell {
    /// CP437 glyph code point (0-255 in practice, stored as u32 in the format).
    pub glyph: u32,
    /// Foreground color (RGB888).
    pub fg: [u8; 3],
    /// Background color (RGB888).
    pub bg: [u8; 3],
}

/// A single layer of an XP sprite.
#[derive(Debug, Clone)]
pub struct XpLayer {
    /// Layer width in cells.
    pub width: u32,
    /// Layer height in cells.
    pub height: u32,
    /// Cell data in column-major order: index = col * height + row.
    pub cells: Vec<XpCell>,
}

/// A parsed XP sprite with all layers.
#[derive(Debug, Clone, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct XpSprite {
    /// Format version (typically -1).
    pub version: i32,
    /// Sprite width in cells.
    pub width: u32,
    /// Sprite height in cells.
    pub height: u32,
    /// Layers in order: L0=colorkey, L1=height, L2=visual, L3+=overlay.
    pub layers: Vec<XpLayer>,
}

/// A merged cell combining data from all layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergedCell {
    /// Final glyph after merge.
    pub glyph: u32,
    /// Final foreground color after merge.
    pub fg: [u8; 3],
    /// Final background color after merge.
    pub bg: [u8; 3],
    /// Height value decoded from layer 1 (0-35, or SPRITE_HEIGHT_UNDEFINED).
    pub height: u8,
    /// Whether this cell is transparent (bg matches layer 0 colorkey).
    pub transparent: bool,
}

/// Read a little-endian i32 from a byte slice at the given offset.
/// Returns an error if there are not enough bytes.
fn read_i32_le(data: &[u8], offset: usize) -> Result<i32, AssetError> {
    let end = offset + 4;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(offset));
    }
    let bytes: [u8; 4] = data[offset..end]
        .try_into()
        .map_err(|_| AssetError::UnexpectedEof(offset))?;
    Ok(i32::from_le_bytes(bytes))
}

/// Read a single XpCell (10 bytes) from the decompressed data at the given offset.
fn read_cell(data: &[u8], offset: usize) -> Result<XpCell, AssetError> {
    let end = offset + 10;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(offset));
    }
    let glyph = u32::from_le_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| AssetError::UnexpectedEof(offset))?,
    );
    let fg = [data[offset + 4], data[offset + 5], data[offset + 6]];
    let bg = [data[offset + 7], data[offset + 8], data[offset + 9]];
    Ok(XpCell { glyph, fg, bg })
}

/// Parse a gzip-compressed .xp sprite file from raw bytes.
///
/// The file format is:
/// - Gzip envelope
/// - 16-byte global header: version (i32), num_layers (i32), width (i32), height (i32)
/// - Per layer: 8-byte header (width, height -- skipped for layer 0 which shares global header)
///   followed by width*height cells in column-major order (10 bytes per cell)
///
/// # Errors
///
/// Returns `AssetError` if decompression fails, the header is invalid, or there
/// are too few layers or insufficient data.
pub fn parse_xp(bytes: &[u8]) -> Result<XpSprite, AssetError> {
    // Decompress gzip
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    // Need at least 16 bytes for the global header
    if decompressed.len() < 16 {
        return Err(AssetError::UnexpectedEof(decompressed.len()));
    }

    // Parse global header
    let version = read_i32_le(&decompressed, 0)?;
    let num_layers = read_i32_le(&decompressed, 4)? as usize;
    let width = read_i32_le(&decompressed, 8)?;
    let height = read_i32_le(&decompressed, 12)?;

    if num_layers < SPRITE_MIN_LAYERS {
        return Err(AssetError::TooFewLayers(num_layers));
    }
    if width < 1 || height < 1 {
        return Err(AssetError::InvalidDimensions(
            width as usize,
            height as usize,
        ));
    }

    let width = width as u32;
    let height = height as u32;
    let cells_per_layer = width
        .checked_mul(height)
        .ok_or(AssetError::InvalidDimensions(
            width as usize,
            height as usize,
        ))? as usize;

    let mut layers = Vec::with_capacity(num_layers);
    let mut offset: usize = 16; // Start after global header

    for layer_idx in 0..num_layers {
        if layer_idx > 0 {
            // Skip per-layer width/height header (8 bytes)
            if offset + 8 > decompressed.len() {
                return Err(AssetError::UnexpectedEof(offset));
            }
            offset += 8;
        }

        // Read cells in column-major order
        let mut cells = Vec::with_capacity(cells_per_layer);
        for cell_idx in 0..cells_per_layer {
            let cell_offset = offset + cell_idx * 10;
            let cell = read_cell(&decompressed, cell_offset)?;
            cells.push(cell);
        }

        layers.push(XpLayer {
            width,
            height,
            cells,
        });

        offset += cells_per_layer * 10;
    }

    Ok(XpSprite {
        version,
        width,
        height,
        layers,
    })
}

/// Decode a height glyph from layer 1.
/// '0'-'9' maps to 0-9, 'A'-'Z' maps to 10-35, anything else is SPRITE_HEIGHT_UNDEFINED.
fn decode_height(glyph: u32) -> u8 {
    match glyph {
        48..=57 => (glyph - 48) as u8, // '0'-'9'
        65..=90 => (glyph - 55) as u8, // 'A'-'Z' -> 10-35
        _ => SPRITE_HEIGHT_UNDEFINED,
    }
}

/// Check if a cell is a swoosh cell (cyan fg + half-block glyph).
fn is_swoosh_cell(cell: &XpCell) -> bool {
    let fg_is_cyan =
        cell.fg[0] == SPRITE_CYAN.0 && cell.fg[1] == SPRITE_CYAN.1 && cell.fg[2] == SPRITE_CYAN.2;
    let is_half_block = matches!(
        cell.glyph,
        SPRITE_GLYPH_HALF_LOWER
            | SPRITE_GLYPH_HALF_LEFT
            | SPRITE_GLYPH_HALF_RIGHT
            | SPRITE_GLYPH_HALF_UPPER
    );
    fg_is_cyan && is_half_block
}

/// Get the quadrant bitmask for a half-block glyph.
fn half_block_mask(glyph: u32) -> u8 {
    use super::constants::{
        SPRITE_MASK_FULL, SPRITE_MASK_LEFT, SPRITE_MASK_LOWER, SPRITE_MASK_RIGHT, SPRITE_MASK_UPPER,
    };
    match glyph {
        SPRITE_GLYPH_HALF_LOWER => SPRITE_MASK_LOWER,
        SPRITE_GLYPH_HALF_LEFT => SPRITE_MASK_LEFT,
        SPRITE_GLYPH_HALF_RIGHT => SPRITE_MASK_RIGHT,
        SPRITE_GLYPH_HALF_UPPER => SPRITE_MASK_UPPER,
        _ => SPRITE_MASK_FULL,
    }
}

/// Lighten an RGB color by adding SPRITE_LIGHTEN_AMOUNT to each channel (clamped to 255).
fn lighten_color(color: [u8; 3]) -> [u8; 3] {
    [
        color[0].saturating_add(SPRITE_LIGHTEN_AMOUNT),
        color[1].saturating_add(SPRITE_LIGHTEN_AMOUNT),
        color[2].saturating_add(SPRITE_LIGHTEN_AMOUNT),
    ]
}

/// Merge all layers of an XP sprite into a flat array of MergedCells.
///
/// Layer merge semantics (per C++ sprite.cpp):
/// 1. Layer 2 is the visual base.
/// 2. Layer 0 bg color is the transparency colorkey.
/// 3. Layer 1 glyph encodes height ('0'-'9'=0-9, 'A'-'Z'=10-35).
/// 4. Layers 3..N-2 (intermediate): simple overwrite if cell bg differs from colorkey.
/// 5. Layer N-1 (last): swoosh merge. Swoosh cells (cyan fg + half-block glyph) lighten
///    the base cell's fg color. Non-swoosh cells overwrite normally.
///
/// Returns a Vec of MergedCells in the same column-major order as the layer cell data.
pub fn merge_layers(sprite: &XpSprite) -> Vec<MergedCell> {
    let num_cells = (sprite.width * sprite.height) as usize;
    let num_layers = sprite.layers.len();

    // Initialize merged from layer 2 (visual base)
    let layer0 = &sprite.layers[0];
    let layer1 = &sprite.layers[1];
    let layer2 = &sprite.layers[2];

    let mut merged: Vec<MergedCell> = (0..num_cells)
        .map(|i| {
            let colorkey_bg = layer0.cells[i].bg;
            let visual_cell = &layer2.cells[i];
            let is_transparent = visual_cell.bg == colorkey_bg;
            let height = decode_height(layer1.cells[i].glyph);

            MergedCell {
                glyph: visual_cell.glyph,
                fg: visual_cell.fg,
                bg: visual_cell.bg,
                height,
                transparent: is_transparent,
            }
        })
        .collect();

    // Apply intermediate layers (3..N-2) as simple overwrites
    let last_layer_idx = num_layers - 1;
    for layer_idx in 3..last_layer_idx {
        let layer = &sprite.layers[layer_idx];
        let colorkey_cells = &sprite.layers[0].cells;
        for (merged_cell, (overlay_cell, colorkey_cell)) in merged
            .iter_mut()
            .zip(layer.cells.iter().zip(colorkey_cells.iter()))
        {
            // Overwrite if cell bg differs from colorkey (i.e., cell is not transparent)
            if overlay_cell.bg != colorkey_cell.bg {
                *merged_cell = MergedCell {
                    glyph: overlay_cell.glyph,
                    fg: overlay_cell.fg,
                    bg: overlay_cell.bg,
                    height: merged_cell.height,
                    transparent: false,
                };
            }
        }
    }

    // Apply last layer with swoosh merge logic (only if there are 4+ layers)
    if num_layers > SPRITE_MIN_LAYERS {
        let last_layer = &sprite.layers[last_layer_idx];
        let colorkey_cells = &sprite.layers[0].cells;
        for (merged_cell, (overlay_cell, colorkey_cell)) in merged
            .iter_mut()
            .zip(last_layer.cells.iter().zip(colorkey_cells.iter()))
        {
            if is_swoosh_cell(overlay_cell) {
                // Swoosh: lighten the base cell's fg for affected quadrants.
                // The mask determines which quadrants are affected, but for the
                // basic implementation we lighten the entire fg color.
                let _mask = half_block_mask(overlay_cell.glyph);
                *merged_cell = MergedCell {
                    fg: lighten_color(merged_cell.fg),
                    ..*merged_cell
                };
            } else if overlay_cell.bg != colorkey_cell.bg {
                // Non-swoosh: simple overwrite
                *merged_cell = MergedCell {
                    glyph: overlay_cell.glyph,
                    fg: overlay_cell.fg,
                    bg: overlay_cell.bg,
                    height: merged_cell.height,
                    transparent: false,
                };
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_height_digits() {
        assert_eq!(decode_height(48), 0); // '0'
        assert_eq!(decode_height(57), 9); // '9'
    }

    #[test]
    fn test_decode_height_letters() {
        assert_eq!(decode_height(65), 10); // 'A'
        assert_eq!(decode_height(90), 35); // 'Z'
    }

    #[test]
    fn test_decode_height_undefined() {
        assert_eq!(decode_height(32), SPRITE_HEIGHT_UNDEFINED); // space
        assert_eq!(decode_height(0), SPRITE_HEIGHT_UNDEFINED);
    }

    #[test]
    fn test_is_swoosh_cell_true() {
        let cell = XpCell {
            glyph: SPRITE_GLYPH_HALF_LOWER,
            fg: [SPRITE_CYAN.0, SPRITE_CYAN.1, SPRITE_CYAN.2],
            bg: [0, 0, 0],
        };
        assert!(is_swoosh_cell(&cell));
    }

    #[test]
    fn test_is_swoosh_cell_wrong_fg() {
        let cell = XpCell {
            glyph: SPRITE_GLYPH_HALF_LOWER,
            fg: [255, 0, 0], // red, not cyan
            bg: [0, 0, 0],
        };
        assert!(!is_swoosh_cell(&cell));
    }

    #[test]
    fn test_is_swoosh_cell_wrong_glyph() {
        let cell = XpCell {
            glyph: 65, // 'A', not a half-block
            fg: [SPRITE_CYAN.0, SPRITE_CYAN.1, SPRITE_CYAN.2],
            bg: [0, 0, 0],
        };
        assert!(!is_swoosh_cell(&cell));
    }

    #[test]
    fn test_lighten_color() {
        assert_eq!(lighten_color([100, 100, 100]), [151, 151, 151]);
        assert_eq!(lighten_color([250, 200, 0]), [255, 251, 51]);
    }

    #[test]
    fn test_checked_mul_overflow_dimensions() {
        // Build a minimal valid gzip-compressed XP header with 65536x65536 dims.
        // That overflows u32 (65536 * 65536 = 2^32, wraps to 0).
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut header = Vec::new();
        header.extend_from_slice(&(-1i32).to_le_bytes()); // version = -1
        header.extend_from_slice(&3i32.to_le_bytes()); // num_layers = 3
        header.extend_from_slice(&65536i32.to_le_bytes()); // width = 65536
        header.extend_from_slice(&65536i32.to_le_bytes()); // height = 65536

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&header).unwrap();
        let compressed = encoder.finish().unwrap();

        let result = parse_xp(&compressed);
        assert!(result.is_err(), "65536x65536 should fail with overflow");
        let err = result.unwrap_err();
        match err {
            AssetError::InvalidDimensions(_, _) => {} // Expected
            other => panic!("Expected InvalidDimensions, got: {other}"),
        }
    }

    #[test]
    fn test_read_cell() {
        let mut data = vec![0u8; 10];
        // glyph = 44 (LE)
        data[0..4].copy_from_slice(&44u32.to_le_bytes());
        // fg = (85, 255, 85)
        data[4] = 85;
        data[5] = 255;
        data[6] = 85;
        // bg = (0, 170, 0)
        data[7] = 0;
        data[8] = 170;
        data[9] = 0;

        let cell = read_cell(&data, 0).unwrap();
        assert_eq!(cell.glyph, 44);
        assert_eq!(cell.fg, [85, 255, 85]);
        assert_eq!(cell.bg, [0, 170, 0]);
    }
}
