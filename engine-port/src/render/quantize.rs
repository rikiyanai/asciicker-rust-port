/// Convert an 8-bit color channel to 5-bit using the C++ mesh shader formula.
///
/// Formula: `(c8 * 249 + 1014) >> 11`
/// Maps 0..=255 to 0..=31.
#[inline]
pub fn rgb8_to_rgb5(c8: u8) -> u8 {
    ((c8 as u32 * 249 + 1014) >> 11) as u8
}

/// Expand a 5-bit color channel back to 8-bit using the C++ resolve formula.
///
/// Formula: `(c5 * 527 + 23) >> 6`
/// Maps 0..=31 to 0..=255.
#[inline]
pub fn rgb5_to_rgb8(c5: u16) -> u8 {
    ((c5 * 527 + 23) >> 6) as u8
}

/// Pack three 5-bit channel values into a single RGB555 `u16`.
///
/// Layout: `r5 | (g5 << 5) | (b5 << 10)`.
#[inline]
pub fn pack_rgb555(r5: u8, g5: u8, b5: u8) -> u16 {
    r5 as u16 | ((g5 as u16) << 5) | ((b5 as u16) << 10)
}

/// Unpack an RGB555 `u16` into three 5-bit channel values `(r5, g5, b5)`.
#[inline]
pub fn unpack_rgb555(rgb555: u16) -> (u8, u8, u8) {
    let r5 = (rgb555 & 0x1F) as u8;
    let g5 = ((rgb555 >> 5) & 0x1F) as u8;
    let b5 = ((rgb555 >> 10) & 0x1F) as u8;
    (r5, g5, b5)
}

/// Convert RGB888 to packed RGB555 (convenience wrapper).
#[inline]
pub fn rgb888_to_rgb555(r: u8, g: u8, b: u8) -> u16 {
    pack_rgb555(rgb8_to_rgb5(r), rgb8_to_rgb5(g), rgb8_to_rgb5(b))
}

/// Convert packed RGB555 to RGB888 (convenience wrapper).
#[inline]
pub fn rgb555_to_rgb888(rgb555: u16) -> (u8, u8, u8) {
    let (r5, g5, b5) = unpack_rgb555(rgb555);
    (
        rgb5_to_rgb8(r5 as u16),
        rgb5_to_rgb8(g5 as u16),
        rgb5_to_rgb8(b5 as u16),
    )
}

/// Map an RGB888 color to the nearest xterm-256 6x6x6 color cube index.
///
/// Formula: `16 + 36 * ((r + 25) / 51) + 6 * ((g + 25) / 51) + ((b + 25) / 51)`
/// Returns a palette index in the range 16..=231.
#[inline]
pub fn rgb2pal(rgb: [u8; 3]) -> u8 {
    let r = ((rgb[0] as u16 + 25) / 51) as u8;
    let g = ((rgb[1] as u16 + 25) / 51) as u8;
    let b = ((rgb[2] as u16 + 25) / 51) as u8;
    16 + 36 * r + 6 * g + b
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- rgb8_to_rgb5 tests ---

    #[test]
    fn rgb8_to_rgb5_boundaries() {
        assert_eq!(rgb8_to_rgb5(0), 0);
        assert_eq!(rgb8_to_rgb5(255), 31);
    }

    #[test]
    fn rgb8_to_rgb5_midpoint() {
        // 128 -> (128 * 249 + 1014) >> 11 = (31872 + 1014) >> 11 = 32886 >> 11 = 16
        // Actually: 32886 / 2048 = 16.06... -> 16
        let result = rgb8_to_rgb5(128);
        assert!(
            result == 15 || result == 16,
            "128 -> {result}, expected 15 or 16"
        );
    }

    // --- rgb5_to_rgb8 tests ---

    #[test]
    fn rgb5_to_rgb8_boundaries() {
        assert_eq!(rgb5_to_rgb8(0), 0);
        assert_eq!(rgb5_to_rgb8(31), 255);
    }

    #[test]
    fn rgb5_to_rgb8_midpoint() {
        // 15 -> (15 * 527 + 23) >> 6 = (7905 + 23) >> 6 = 7928 >> 6 = 123
        let result = rgb5_to_rgb8(15);
        assert_eq!(result, 123);
    }

    // --- pack/unpack roundtrip ---

    #[test]
    fn pack_unpack_roundtrip() {
        for r5 in [0u8, 7, 15, 23, 31] {
            for g5 in [0u8, 15, 31] {
                for b5 in [0u8, 15, 31] {
                    let packed = pack_rgb555(r5, g5, b5);
                    let (ur, ug, ub) = unpack_rgb555(packed);
                    assert_eq!(
                        (ur, ug, ub),
                        (r5, g5, b5),
                        "roundtrip failed for ({r5}, {g5}, {b5})"
                    );
                }
            }
        }
    }

    #[test]
    fn pack_rgb555_layout() {
        // r=1, g=2, b=3 => 1 | (2<<5) | (3<<10) = 1 | 64 | 3072 = 3137
        assert_eq!(pack_rgb555(1, 2, 3), 3137);
    }

    // --- rgb2pal tests ---

    #[test]
    fn rgb2pal_black() {
        assert_eq!(rgb2pal([0, 0, 0]), 16);
    }

    #[test]
    fn rgb2pal_white() {
        assert_eq!(rgb2pal([255, 255, 255]), 231);
    }

    #[test]
    fn rgb2pal_red() {
        assert_eq!(rgb2pal([255, 0, 0]), 196);
    }

    #[test]
    fn rgb2pal_green() {
        assert_eq!(rgb2pal([0, 255, 0]), 46);
    }

    #[test]
    fn rgb2pal_blue() {
        assert_eq!(rgb2pal([0, 0, 255]), 21);
    }

    #[test]
    fn rgb2pal_mid_grey() {
        // 128: (128+25)/51 = 153/51 = 3
        // So: 16 + 36*3 + 6*3 + 3 = 16 + 108 + 18 + 3 = 145
        assert_eq!(rgb2pal([128, 128, 128]), 145);
    }

    // --- end-to-end convenience ---

    #[test]
    fn rgb888_to_rgb555_black() {
        assert_eq!(rgb888_to_rgb555(0, 0, 0), 0);
    }

    #[test]
    fn rgb888_to_rgb555_white() {
        assert_eq!(rgb888_to_rgb555(255, 255, 255), pack_rgb555(31, 31, 31));
    }

    #[test]
    fn rgb555_to_rgb888_black() {
        assert_eq!(rgb555_to_rgb888(0), (0, 0, 0));
    }

    #[test]
    fn rgb555_to_rgb888_white() {
        assert_eq!(rgb555_to_rgb888(pack_rgb555(31, 31, 31)), (255, 255, 255));
    }

    #[test]
    fn rgb888_rgb555_roundtrip_approximate() {
        // Roundtrip through RGB555 should be close but not exact for mid-values
        let (r, g, b) = rgb555_to_rgb888(rgb888_to_rgb555(100, 150, 200));
        assert!((r as i16 - 99).unsigned_abs() <= 5, "r: {r}");
        assert!((g as i16 - 148).unsigned_abs() <= 5, "g: {g}");
        assert!((b as i16 - 197).unsigned_abs() <= 5, "b: {b}");
    }
}
