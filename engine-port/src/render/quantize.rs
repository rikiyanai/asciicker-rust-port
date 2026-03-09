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
