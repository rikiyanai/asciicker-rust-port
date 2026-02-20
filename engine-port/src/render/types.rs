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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_cell_is_4_bytes() {
        assert_eq!(std::mem::size_of::<AnsiCell>(), 4);
    }

    #[test]
    fn transparent_has_gl_255() {
        assert_eq!(AnsiCell::TRANSPARENT.gl, 255);
        assert!(AnsiCell::TRANSPARENT.is_transparent());
    }

    #[test]
    fn default_is_not_transparent() {
        let cell = AnsiCell::default();
        assert_eq!(cell.gl, 0);
        assert!(!cell.is_transparent());
    }

    #[test]
    fn custom_cell_transparency() {
        let opaque = AnsiCell {
            fg: 196,
            bk: 16,
            gl: 65,
            spare: 0xFF,
        };
        assert!(!opaque.is_transparent());

        let transparent = AnsiCell {
            fg: 196,
            bk: 16,
            gl: 255,
            spare: 0,
        };
        assert!(transparent.is_transparent());
    }
}
