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
    fn select_glyph(&mut self, _buf: &SampleBuffer, _cx: usize, _cy: usize) -> Option<u8> {
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
            let gl = match glyph_selector.select_glyph(sample_buffer, cx as usize, cy as usize) {
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
