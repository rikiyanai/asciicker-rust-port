//! Resolve bridge: converts AnsiCell (xterm-256 palette) to AsciiCellGrid (RGBA).
//!
//! CRITICAL GAP #3 RESOLUTION: Bridges the Phase 4 output format (xterm-256
//! palette indices in `AnsiCell`) to the Phase 3 input format (RGBA `[u8; 4]`
//! in `AsciiCellGrid`).
//!
//! The `GlyphSelector` trait provides a pluggable glyph selection interface
//! for Phase 7 shape-vector extensibility.

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::material::Material;
use crate::render::resolve::resolve;
use crate::render::sample_buffer::SampleBuffer;
use crate::render::types::AnsiCell;

// ---------------------------------------------------------------------------
// Xterm-256 palette
// ---------------------------------------------------------------------------

/// Full xterm-256 color palette RGB values.
///
/// - Indices 0-15: standard terminal colors
/// - Indices 16-231: 6x6x6 color cube (`r = (idx-16)/36 * 51`, etc.)
/// - Indices 232-255: grayscale ramp (`v = 8 + (idx-232) * 10`)
///
/// The formula uses evenly-spaced levels `[0, 51, 102, 153, 204, 255]`.
/// This is INTENTIONAL -- the C++ engine's `rgb2pal()` uses `(c + 25) / 51`
/// which assumes evenly-spaced levels. The palette table must be the inverse
/// of `rgb2pal` for correct round-tripping.
pub static XTERM_256_PALETTE: [[u8; 3]; 256] = {
    let mut palette = [[0u8; 3]; 256];

    // Standard 16 colors (indices 0-15)
    palette[0] = [0, 0, 0]; // Black
    palette[1] = [128, 0, 0]; // Maroon
    palette[2] = [0, 128, 0]; // Green
    palette[3] = [128, 128, 0]; // Olive
    palette[4] = [0, 0, 128]; // Navy
    palette[5] = [128, 0, 128]; // Purple
    palette[6] = [0, 128, 128]; // Teal
    palette[7] = [192, 192, 192]; // Silver
    palette[8] = [128, 128, 128]; // Grey
    palette[9] = [255, 0, 0]; // Red
    palette[10] = [0, 255, 0]; // Lime
    palette[11] = [255, 255, 0]; // Yellow
    palette[12] = [0, 0, 255]; // Blue
    palette[13] = [255, 0, 255]; // Fuchsia
    palette[14] = [0, 255, 255]; // Aqua
    palette[15] = [255, 255, 255]; // White

    // 6x6x6 color cube (indices 16-231)
    let levels: [u8; 6] = [0, 51, 102, 153, 204, 255];
    let mut i = 16;
    let mut ri = 0;
    while ri < 6 {
        let mut gi = 0;
        while gi < 6 {
            let mut bi = 0;
            while bi < 6 {
                palette[i] = [levels[ri], levels[gi], levels[bi]];
                i += 1;
                bi += 1;
            }
            gi += 1;
        }
        ri += 1;
    }

    // Grayscale ramp (indices 232-255)
    let mut g = 0;
    while g < 24 {
        let v = (8 + g * 10) as u8;
        palette[232 + g] = [v, v, v];
        g += 1;
    }

    palette
};

// ---------------------------------------------------------------------------
// GlyphSelector trait
// ---------------------------------------------------------------------------

/// Trait for pluggable glyph selection at RESOLVE stage.
///
/// Default implementation uses auto_mat glyph. Phase 7 shape-vector
/// provides an alternative implementation that samples the SampleBuffer
/// for 6D shape vectors and matches via k-d tree.
pub trait GlyphSelector {
    /// Given a 2x2 cell region in the SampleBuffer, return a CP437 glyph index.
    /// Returns `None` to fall back to auto_mat glyph selection.
    ///
    /// `cell_x` and `cell_y` are ASCII GRID coordinates (0..ascii_w, 0..ascii_h),
    /// NOT sample-buffer coordinates. To access the corresponding 2x2 sample block,
    /// compute: `sx = 2 + 2 * cell_x`, `sy = 2 + 2 * cell_y`.
    /// This matches resolve.rs's internal offset logic (2-sample border + 2 samples
    /// per ASCII cell).
    ///
    /// `&mut self` is intentional -- allows stateful implementations in Phase 7
    /// (e.g., k-d tree caching). AutoMatGlyphSelector is stateless but uses
    /// `&mut self` for trait uniformity.
    fn select_glyph(
        &mut self,
        sample_buffer: &SampleBuffer,
        cell_x: usize,
        cell_y: usize,
    ) -> Option<u8>;
}

/// Default selector that always falls back to auto_mat (returns None).
pub struct AutoMatGlyphSelector;

impl GlyphSelector for AutoMatGlyphSelector {
    fn select_glyph(
        &mut self,
        _buf: &SampleBuffer,
        _cx: usize,
        _cy: usize,
    ) -> Option<u8> {
        None
    }
}

// ---------------------------------------------------------------------------
// resolve_to_grid
// ---------------------------------------------------------------------------

/// Bridge Phase 4 resolve output (AnsiCell with xterm-256 palette indices)
/// to Phase 3 AsciiCellGrid (RGBA `[u8; 4]` colors).
///
/// # Arguments
/// * `sample_buffer` - The 2x supersampled SampleBuffer
/// * `materials` - Material library for terrain shade lookups
/// * `grid` - Output AsciiCellGrid to write into
/// * `glyph_selector` - Pluggable glyph selection (pass `&mut AutoMatGlyphSelector` for default)
/// * `resolve_buf` - Reusable buffer for AnsiCell intermediaries (avoids per-frame allocation)
///
/// # Panics
/// Panics if `grid.width` or `grid.height` do not match the derived ASCII dimensions.
pub fn resolve_to_grid<G: GlyphSelector>(
    sample_buffer: &SampleBuffer,
    materials: &[Material],
    grid: &mut AsciiCellGrid,
    glyph_selector: &mut G,
    resolve_buf: &mut Vec<AnsiCell>,
) {
    let dw = sample_buffer.width as i32;
    let dh = sample_buffer.height as i32;
    let ascii_w = (sample_buffer.width as i32 - 4) / 2;
    let ascii_h = (sample_buffer.height as i32 - 4) / 2;

    assert_eq!(
        grid.width as i32, ascii_w,
        "AsciiCellGrid width mismatch: grid={}, expected={}",
        grid.width, ascii_w
    );
    assert_eq!(
        grid.height as i32, ascii_h,
        "AsciiCellGrid height mismatch: grid={}, expected={}",
        grid.height, ascii_h
    );

    // Resize resolve_buf to EXACTLY the target size (NOT reserve)
    let target_len = (ascii_w * ascii_h) as usize;
    resolve_buf.resize(target_len, AnsiCell::default());

    // Call Phase 4 resolve
    resolve(
        &sample_buffer.samples,
        dw,
        dh,
        ascii_w,
        ascii_h,
        materials,
        resolve_buf,
    );

    // Convert AnsiCell palette indices to RGBA and write to grid
    for cy in 0..ascii_h {
        for cx in 0..ascii_w {
            let i = (cy * ascii_w + cx) as usize;
            let cell = &resolve_buf[i];

            // Check glyph selector for override
            let gl = match glyph_selector.select_glyph(
                sample_buffer,
                cx as usize,
                cy as usize,
            ) {
                Some(glyph) => glyph,
                None => cell.gl,
            };

            // Convert xterm-256 palette indices to RGBA
            let fg_rgb = XTERM_256_PALETTE[cell.fg as usize];
            let bk_rgb = XTERM_256_PALETTE[cell.bk as usize];

            grid.char_indices[i] = gl as u16;
            grid.fg_colors[i] = [fg_rgb[0], fg_rgb[1], fg_rgb[2], 255];
            grid.bg_colors[i] = [bk_rgb[0], bk_rgb[1], bk_rgb[2], 255];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::material::test_materials;
    use crate::render::sample_buffer::{Sample, spare_bits};

    #[test]
    fn test_xterm_palette_black() {
        // Index 16 is the first color cube entry: (0,0,0) = black
        assert_eq!(XTERM_256_PALETTE[16], [0, 0, 0]);
    }

    #[test]
    fn test_xterm_palette_white() {
        // Index 231 = 16 + 36*5 + 6*5 + 5 = 231 => (255, 255, 255)
        assert_eq!(XTERM_256_PALETTE[231], [255, 255, 255]);
    }

    #[test]
    fn test_xterm_palette_red() {
        // Index 196 = 16 + 36*5 + 6*0 + 0 = 196 => (255, 0, 0)
        assert_eq!(XTERM_256_PALETTE[196], [255, 0, 0]);
    }

    #[test]
    fn test_xterm_palette_green() {
        // Index 46 = 16 + 36*0 + 6*5 + 0 = 46 => (0, 255, 0)
        assert_eq!(XTERM_256_PALETTE[46], [0, 255, 0]);
    }

    #[test]
    fn test_xterm_palette_blue() {
        // Index 21 = 16 + 36*0 + 6*0 + 5 = 21 => (0, 0, 255)
        assert_eq!(XTERM_256_PALETTE[21], [0, 0, 255]);
    }

    #[test]
    fn test_xterm_palette_cube_structure() {
        // Verify the 6x6x6 cube structure
        let levels: [u8; 6] = [0, 51, 102, 153, 204, 255];
        for ri in 0..6u8 {
            for gi in 0..6u8 {
                for bi in 0..6u8 {
                    let idx = 16 + 36 * ri as usize + 6 * gi as usize + bi as usize;
                    let expected = [levels[ri as usize], levels[gi as usize], levels[bi as usize]];
                    assert_eq!(
                        XTERM_256_PALETTE[idx], expected,
                        "Palette mismatch at index {} (r={}, g={}, b={})",
                        idx, ri, gi, bi
                    );
                }
            }
        }
    }

    #[test]
    fn test_xterm_palette_grayscale() {
        // Verify grayscale ramp (indices 232-255)
        for g in 0..24 {
            let expected_v = (8 + g * 10) as u8;
            assert_eq!(
                XTERM_256_PALETTE[232 + g],
                [expected_v, expected_v, expected_v],
                "Grayscale mismatch at index {}",
                232 + g
            );
        }
    }

    #[test]
    fn test_resolve_to_grid_clear_cells() {
        // R16-F189 FIX: Clear buffer produces space/black cells
        let buf = SampleBuffer::new(4, 4);
        let materials = test_materials();
        let mut grid = AsciiCellGrid::new(4, 4);
        let mut selector = AutoMatGlyphSelector;
        let mut resolve_buf = Vec::new();

        resolve_to_grid(&buf, &materials, &mut grid, &mut selector, &mut resolve_buf);

        for i in 0..grid.cells_count() {
            assert_eq!(
                grid.char_indices[i], 32,
                "Clear cell should have space glyph (32), got {}",
                grid.char_indices[i]
            );
            assert_eq!(
                grid.fg_colors[i],
                [0, 0, 0, 255],
                "Clear cell fg should be black"
            );
            assert_eq!(
                grid.bg_colors[i],
                [0, 0, 0, 255],
                "Clear cell bg should be black"
            );
        }
    }

    #[test]
    fn test_resolve_to_grid_produces_rgba() {
        // Create SampleBuffer with known mesh content
        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();

        // Write a mesh sample at cell (1, 1)
        let sx = 2 + 2 * 1;
        let sy = 2 + 2 * 1;
        let mesh_sample = Sample {
            visual: 31, // pure red RGB555
            diffuse: 255,
            spare: spare_bits::MESH_FLAG,
            height: 10.0,
        };
        // Fill the 2x2 block
        for dy in 0..2u32 {
            for dx in 0..2u32 {
                *buf.sample_at_mut(sx as u32 + dx, sy as u32 + dy) = mesh_sample;
            }
        }

        let mut grid = AsciiCellGrid::new(4, 4);
        let mut selector = AutoMatGlyphSelector;
        let mut resolve_buf = Vec::new();

        resolve_to_grid(&buf, &materials, &mut grid, &mut selector, &mut resolve_buf);

        // Cell (1, 1) should have RGBA colors, not palette indices
        let idx = 1 * 4 + 1;
        let fg = grid.fg_colors[idx];
        let bg = grid.bg_colors[idx];

        // fg and bg should be valid RGBA (alpha = 255)
        assert_eq!(fg[3], 255, "fg alpha should be 255");
        assert_eq!(bg[3], 255, "bg alpha should be 255");

        // At least one of fg/bg should be non-black for a red mesh
        assert!(
            fg[0] > 0 || fg[1] > 0 || fg[2] > 0 || bg[0] > 0 || bg[1] > 0 || bg[2] > 0,
            "Mesh cell should produce non-black RGBA, fg={:?} bg={:?}",
            fg,
            bg
        );
    }

    #[test]
    fn test_glyph_selector_none_uses_automat() {
        let mut selector = AutoMatGlyphSelector;
        let buf = SampleBuffer::new(4, 4);
        let result = selector.select_glyph(&buf, 0, 0);
        assert_eq!(result, None, "AutoMatGlyphSelector should always return None");
    }

    #[test]
    fn test_glyph_selector_override() {
        // Custom GlyphSelector that returns a specific glyph
        struct TestGlyphSelector;
        impl GlyphSelector for TestGlyphSelector {
            fn select_glyph(
                &mut self,
                _buf: &SampleBuffer,
                _cx: usize,
                _cy: usize,
            ) -> Option<u8> {
                Some(b'X')
            }
        }

        let mut buf = SampleBuffer::new(4, 4);
        let materials = test_materials();

        // Write a mesh sample at cell (1, 1)
        let sx = 2 + 2 * 1;
        let sy = 2 + 2 * 1;
        let mesh_sample = Sample {
            visual: 31,
            diffuse: 255,
            spare: spare_bits::MESH_FLAG,
            height: 10.0,
        };
        for dy in 0..2u32 {
            for dx in 0..2u32 {
                *buf.sample_at_mut(sx as u32 + dx, sy as u32 + dy) = mesh_sample;
            }
        }

        let mut grid = AsciiCellGrid::new(4, 4);
        let mut selector = TestGlyphSelector;
        let mut resolve_buf = Vec::new();

        resolve_to_grid(&buf, &materials, &mut grid, &mut selector, &mut resolve_buf);

        // Cell (1, 1) should have the overridden glyph 'X'
        let idx = 1 * 4 + 1;
        assert_eq!(
            grid.char_indices[idx],
            b'X' as u16,
            "GlyphSelector override should set glyph to 'X'"
        );

        // fg/bg colors should still come from auto_mat (not affected by glyph override)
        assert_eq!(grid.fg_colors[idx][3], 255, "fg alpha should be 255");
        assert_eq!(grid.bg_colors[idx][3], 255, "bg alpha should be 255");
    }
}
