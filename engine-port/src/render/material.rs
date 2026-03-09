//! Material system: MatCell, Material structs, auto_mat LUT, and test materials.
//!
//! The resolve stage uses two code paths:
//! - Terrain samples look up `matlib[sample.visual].shade[elevation][diffuse]`
//! - Mesh samples use `auto_mat[rgb555]` to get `{bg, fg, dither_glyph}`
//!
//! This module implements both data types and the LUT generation.

/// A single material cell, matching C++ `render.h:53`.
///
/// Encodes foreground RGB, background RGB, a CP437 glyph, and blend flags.
/// Total size: 8 bytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatCell {
    /// Foreground color (RGB888).
    pub fg: [u8; 3],
    /// CP437 glyph for dithering/pattern.
    pub gl: u8,
    /// Background color (RGB888).
    pub bg: [u8; 3],
    /// Flags: bits 0-1 = fg_blend, bit 2 = gl_mask, bits 3-4 = bg_blend.
    pub flags: u8,
}

/// A terrain material with shade lookup tables, matching C++ `render.h:82`.
///
/// Indexed by `shade[elevation 0-3][diffuse/17 = 0-15]`.
#[derive(Debug, Clone, Default)]
pub struct Material {
    /// Shade table: `[elevation][diffuse_level]`.
    pub shade: [[MatCell; 16]; 4],
    /// Animation mode flags.
    pub mode: i32,
}

impl Material {
    /// Look up the MatCell for a given elevation and diffuse value.
    ///
    /// Clamps elevation to 0..=3 and maps diffuse (0-255) to index 0..=15
    /// via integer division by 17.
    #[inline]
    pub fn lookup(&self, elevation: u8, diffuse: u8) -> &MatCell {
        let elv = (elevation as usize).min(3);
        let dif = ((diffuse / 17) as usize).min(15);
        &self.shade[elv][dif]
    }
}

/// Create a set of test materials for Phase 4 testing.
///
/// Returns 3 materials: grass, stone, water -- each with plausible
/// elevation/diffuse variation for render pipeline verification.
pub fn test_materials() -> Vec<Material> {
    vec![
        create_grass_material(),
        create_stone_material(),
        create_water_material(),
    ]
}

/// Grass material: green shades with '.' glyph.
/// Darker at low diffuse, brighter at high diffuse.
/// Different elevation levels produce slight hue shifts.
fn create_grass_material() -> Material {
    let mut mat = Material::default();
    for elv in 0..4u8 {
        for dif in 0..16u8 {
            let brightness = (dif as u16 * 16).min(255) as u8;
            let green_base = 80u8.saturating_add(brightness / 2);
            let red_shift = elv * 8;
            mat.shade[elv as usize][dif as usize] = MatCell {
                fg: [
                    20u8.saturating_add(red_shift)
                        .saturating_add(brightness / 4),
                    green_base,
                    10u8.saturating_add(brightness / 8),
                ],
                gl: b'.',
                bg: [
                    10u8.saturating_add(red_shift / 2),
                    40u8.saturating_add(brightness / 3),
                    5,
                ],
                flags: 0,
            };
        }
    }
    mat
}

/// Stone material: grey shades with '#' glyph.
/// Uniform desaturation across elevations.
fn create_stone_material() -> Material {
    let mut mat = Material::default();
    for elv in 0..4u8 {
        for dif in 0..16u8 {
            let brightness = (dif as u16 * 16).min(255) as u8;
            let grey = 40u8.saturating_add(brightness / 2);
            let elv_offset = elv * 4;
            mat.shade[elv as usize][dif as usize] = MatCell {
                fg: [
                    grey.saturating_add(elv_offset),
                    grey,
                    grey.saturating_sub(elv_offset),
                ],
                gl: b'#',
                bg: [
                    grey.saturating_sub(20),
                    grey.saturating_sub(20),
                    grey.saturating_sub(20),
                ],
                flags: 0,
            };
        }
    }
    mat
}

/// Water material: blue shades with '~' glyph.
/// Higher elevation = lighter blue (shoreline effect).
fn create_water_material() -> Material {
    let mut mat = Material::default();
    for elv in 0..4u8 {
        for dif in 0..16u8 {
            let brightness = (dif as u16 * 16).min(255) as u8;
            let blue_base = 100u8.saturating_add(brightness / 2);
            let elv_lighten = elv * 15;
            mat.shade[elv as usize][dif as usize] = MatCell {
                fg: [
                    20u8.saturating_add(elv_lighten),
                    40u8.saturating_add(elv_lighten)
                        .saturating_add(brightness / 4),
                    blue_base,
                ],
                gl: b'~',
                bg: [
                    5u8.saturating_add(elv_lighten / 2),
                    15u8.saturating_add(elv_lighten / 2),
                    60u8.saturating_add(brightness / 3),
                ],
                flags: 0,
            };
        }
    }
    mat
}

use std::sync::LazyLock;

/// Total byte count of the auto_mat LUT: 32 * 32 * 32 * 3 = 98,304.
pub const AUTO_MAT_SIZE: usize = 32 * 32 * 32 * 3;

/// Global lazily-initialized auto_mat LUT.
///
/// Computed once on first access. Maps every RGB555 color to a
/// `(bg_palette, fg_palette, dither_glyph)` triple for xterm-256 rendering.
pub static AUTO_MAT: LazyLock<Box<[u8; AUTO_MAT_SIZE]>> =
    LazyLock::new(|| Box::new(create_auto_mat()));

/// Generate the auto_mat LUT mapping RGB555 to `{bg_palette, fg_palette, dither_glyph}`.
///
/// Follows the exact algorithm from C++ `render.cpp:710-840`:
/// 1. For each RGB555 `(r, g, b)` in `0..32`, compute floor/remainder in MCV-space (0-5).
/// 2. Enumerate all pairs of 8 cube vertices on the xterm 6x6x6 color cube.
/// 3. Find the pair with minimum perpendicular distance to the input color.
/// 4. Project the input onto the best edge to get a dither shade level (0-11).
/// 5. Map vertices to xterm-256 palette indices via `16 + 36*r5 + 6*g5 + b5`.
///
/// Returns a 98,304-byte array indexed by `3 * (r5 + 32 * g5 + 32 * 32 * b5)`.
pub fn create_auto_mat() -> [u8; AUTO_MAT_SIZE] {
    const MCV: i32 = 5;

    // floor(MCV * x / 31) for x in 0..32
    let flo: [i32; 32] = core::array::from_fn(|x| (MCV * x as i32) / 31);
    // remainder: MCV*x - 31*flo[x]
    let rem: [i32; 32] = core::array::from_fn(|x| MCV * x as i32 - 31 * flo[x]);

    let glyph = [b' ', b'.', b'.', b':', b':', b'%'];

    let mcv_to_5 = |mcv: i32| -> i32 { (mcv * 5 + MCV / 2) / MCV };

    let mut mat = [0u8; AUTO_MAT_SIZE];

    for b in 0..32i32 {
        let pb = rem[b as usize];
        let b_vals = [flo[b as usize], (flo[b as usize] + 1).min(MCV)];

        for g in 0..32i32 {
            let pg = rem[g as usize];
            let g_vals = [flo[g as usize], (flo[g as usize] + 1).min(MCV)];

            for r in 0..32i32 {
                let pr = rem[r as usize];
                let r_vals = [flo[r as usize], (flo[r as usize] + 1).min(MCV)];
                let p = [pr, pg, pb];

                let mut best_sd: f32 = -1.0;
                let mut best_pr: f32 = 0.0;
                let mut best_lo: usize = 0;
                let mut best_hi: usize = 0;

                // Check all pairs of 8 cube vertices
                for lo in 0..7usize {
                    let v0 = [r_vals[lo & 1], g_vals[(lo >> 1) & 1], b_vals[(lo >> 2) & 1]];
                    let pv0 = [
                        r_vals[0] * 31 + p[0] - v0[0] * 31,
                        g_vals[0] * 31 + p[1] - v0[1] * 31,
                        b_vals[0] * 31 + p[2] - v0[2] * 31,
                    ];

                    for hi in (lo + 1)..8usize {
                        let v1 = [r_vals[hi & 1], g_vals[(hi >> 1) & 1], b_vals[(hi >> 2) & 1]];
                        let v10 = [
                            31 * (v1[0] - v0[0]),
                            31 * (v1[1] - v0[1]),
                            31 * (v1[2] - v0[2]),
                        ];
                        let v10_sqrlen = v10[0] * v10[0] + v10[1] * v10[1] + v10[2] * v10[2];

                        let projection = if v10_sqrlen != 0 {
                            (v10[0] * pv0[0] + v10[1] * pv0[1] + v10[2] * pv0[2]) as f32
                                / v10_sqrlen as f32
                        } else {
                            0.0
                        };

                        let prp = [
                            v10[0] as f32 * projection,
                            v10[1] as f32 * projection,
                            v10[2] as f32 * projection,
                        ];
                        let prv = [
                            pv0[0] as f32 - prp[0],
                            pv0[1] as f32 - prp[1],
                            pv0[2] as f32 - prp[2],
                        ];
                        let sd = (prv[0] * prv[0] + prv[1] * prv[1] + prv[2] * prv[2]).sqrt();

                        if sd < best_sd || best_sd < 0.0 {
                            best_sd = sd;
                            best_pr = projection;
                            best_lo = lo;
                            best_hi = hi;
                        }
                    }
                }

                let idx = 3 * (r + 32 * g + 32 * 32 * b) as usize;
                let shd = ((best_pr * 11.0 + 0.5).floor() as i32).clamp(0, 11);

                let palette_idx = |vert: usize| -> u8 {
                    (16 + 36 * mcv_to_5(r_vals[vert & 1])
                        + 6 * mcv_to_5(g_vals[(vert >> 1) & 1])
                        + mcv_to_5(b_vals[(vert >> 2) & 1])) as u8
                };

                if shd < 6 {
                    mat[idx] = palette_idx(best_lo);
                    mat[idx + 1] = palette_idx(best_hi);
                    mat[idx + 2] = glyph[shd as usize];
                } else {
                    mat[idx] = palette_idx(best_hi);
                    mat[idx + 1] = palette_idx(best_lo);
                    mat[idx + 2] = glyph[(11 - shd) as usize];
                }
            }
        }
    }
    mat
}

/// Look up the auto_mat entry for an RGB555 color value.
///
/// Returns `(bg_palette, fg_palette, dither_glyph)` where palette indices
/// are in the xterm-256 range (16-231) and dither_glyph is one of
/// `b' '`, `b'.'`, `b':'`, or `b'%'`.
#[inline]
pub fn auto_mat_lookup(rgb555: u16) -> (u8, u8, u8) {
    let r5 = (rgb555 & 0x1F) as usize;
    let g5 = ((rgb555 >> 5) & 0x1F) as usize;
    let b5 = ((rgb555 >> 10) & 0x1F) as usize;
    let idx = 3 * (r5 + 32 * g5 + 32 * 32 * b5);
    let lut = &*AUTO_MAT;
    (lut[idx], lut[idx + 1], lut[idx + 2])
}
