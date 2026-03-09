/// Output cell for the ASCII terminal grid.
///
/// Matches the C++ engine's 4-byte ANSI cell layout:
/// `fg(u8) | bk(u8) | gl(u8) | spare(u8)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct AnsiCell {
    /// Foreground color as xterm-256 palette index.
    pub fg: u8,
    /// Background color as xterm-256 palette index.
    pub bk: u8,
    /// CP437 glyph code. `255` = transparent (no glyph).
    pub gl: u8,
    /// Flags. `0xFF` = rendered cell.
    pub spare: u8,
}

impl AnsiCell {
    /// A transparent cell (no visible glyph).
    pub const TRANSPARENT: Self = Self {
        fg: 0,
        bk: 0,
        gl: 255,
        spare: 0,
    };

    /// Returns `true` if this cell has no visible glyph.
    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.gl == 255
    }
}
