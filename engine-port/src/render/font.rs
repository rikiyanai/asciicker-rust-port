//! Font1 system: text rendering with 3 CP437 glyph skins.
//!
//! Port of C++ `font1.cpp` — provides text painting to `AsciiCellGrid`
//! with Grey, Gold, and Pink skins.
//!
//! Font1 is a Resource providing an API — it does NOT register any systems.
//! Calling systems (HUD, chat overlay) must be ordered AFTER
//! render_pipeline_system in PostUpdate.

use bevy::prelude::*;

use crate::output::ascii_cell_grid::AsciiCellGrid;

// ---------------------------------------------------------------------------
// FontSkin enum
// ---------------------------------------------------------------------------

/// Available text skins for Font1 rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSkin {
    /// Default terminal colors (CP437 font-1.xp as-is, no recolor).
    Grey = 0,
    /// Grey-to-yellow recolor (per C++ font1.cpp recolor table).
    Gold = 1,
    /// Grey-to-magenta recolor (per C++ font1.cpp recolor table).
    Pink = 2,
}

// ---------------------------------------------------------------------------
// Recolor tables (from C++ font1.cpp:243-250)
// ---------------------------------------------------------------------------

/// A single color replacement rule: source RGB -> destination RGB.
#[derive(Debug, Clone, Copy)]
struct ColorReplace {
    src: [u8; 3],
    dst: [u8; 3],
}

/// Gold skin recolor table (C++ font1.cpp:243-246).
/// Format: {count=3, src_r,src_g,src_b, dst_r,dst_g,dst_b, ...}
/// Trailing 0,0 is a C++ sentinel (R14-M03 FIX), ignored.
const GOLD_RECOLORS: [ColorReplace; 3] = [
    ColorReplace { src: [85, 85, 85], dst: [255, 255, 85] },
    ColorReplace { src: [170, 170, 170], dst: [255, 204, 0] },
    ColorReplace { src: [255, 255, 255], dst: [255, 204, 0] },
];

/// Pink skin recolor table (C++ font1.cpp:247-250).
const PINK_RECOLORS: [ColorReplace; 3] = [
    ColorReplace { src: [85, 85, 85], dst: [255, 153, 255] },
    ColorReplace { src: [170, 170, 170], dst: [255, 0, 255] },
    ColorReplace { src: [255, 255, 255], dst: [255, 51, 255] },
];

/// Apply recolor table to a color. Returns the replacement if matched,
/// or the original color if no match.
fn apply_recolor(color: [u8; 3], table: &[ColorReplace]) -> [u8; 3] {
    for rule in table {
        if color == rule.src {
            return rule.dst;
        }
    }
    color
}

// ---------------------------------------------------------------------------
// Font1 Resource
// ---------------------------------------------------------------------------

/// Font1 resource: provides text painting to AsciiCellGrid with 3 skins.
///
/// Stores fg/bg color mapping per skin. The glyph index is the ASCII byte
/// value directly (CP437 compatible for printable ASCII 0x20-0x7E).
///
/// R20-F08: Font1 paint calls write directly to AsciiCellGrid, bypassing
/// resolve and shape-vector. They MUST execute AFTER render_pipeline_system
/// and sprite blit. The calling systems enforce ordering, not Font1 itself.
#[derive(Resource)]
pub struct Font1 {
    /// Default foreground color for Grey skin.
    pub default_fg: [u8; 3],
    /// Default background color for all skins.
    pub default_bg: [u8; 3],
}

impl Default for Font1 {
    fn default() -> Self {
        Self {
            // Standard terminal white-on-black
            default_fg: [170, 170, 170],
            default_bg: [0, 0, 0],
        }
    }
}

impl Font1 {
    /// Get the foreground color for a given skin.
    ///
    /// Grey: uses default_fg (terminal grey/white).
    /// Gold/Pink: applies recolor table to default_fg.
    pub fn fg_color(&self, skin: FontSkin) -> [u8; 3] {
        match skin {
            FontSkin::Grey => self.default_fg,
            FontSkin::Gold => apply_recolor(self.default_fg, &GOLD_RECOLORS),
            FontSkin::Pink => apply_recolor(self.default_fg, &PINK_RECOLORS),
        }
    }

    /// Get the background color for a given skin.
    pub fn bg_color(&self, _skin: FontSkin) -> [u8; 3] {
        self.default_bg
    }

    /// Paint a single character to the grid at (x, y) with the given skin.
    ///
    /// P7-207 FIX: Uses u32 parameters matching AsciiCellGrid::set_cell.
    /// Out-of-bounds coordinates are silently ignored (boundary safety).
    pub fn paint_char(
        &self,
        grid: &mut AsciiCellGrid,
        x: u32,
        y: u32,
        ch: u8,
        skin: FontSkin,
    ) {
        if x >= grid.width || y >= grid.height {
            return; // boundary safety
        }
        let fg = self.fg_color(skin);
        let bg = self.bg_color(skin);
        grid.set_cell(x, y, ch as u16, [fg[0], fg[1], fg[2], 255], [bg[0], bg[1], bg[2], 255]);
    }

    /// Paint a string to the grid starting at (x, y), left-to-right.
    ///
    /// Characters beyond the grid width are clipped.
    pub fn paint_string(
        &self,
        grid: &mut AsciiCellGrid,
        x: u32,
        y: u32,
        text: &str,
        skin: FontSkin,
    ) {
        if y >= grid.height {
            return;
        }
        for (i, byte) in text.bytes().enumerate() {
            let cx = x + i as u32;
            if cx >= grid.width {
                break; // clip at right edge
            }
            self.paint_char(grid, cx, y, byte, skin);
        }
    }

    /// Measure the width of a string in cells.
    ///
    /// Each byte = 1 cell (CP437 is a fixed-width encoding).
    pub fn measure_string(text: &str) -> u32 {
        text.len() as u32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
