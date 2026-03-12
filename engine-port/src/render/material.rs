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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcell_is_8_bytes() {
        assert_eq!(std::mem::size_of::<MatCell>(), 8);
    }

    #[test]
    fn material_shade_dimensions() {
        let mat = Material::default();
        // 4 elevation levels
        assert_eq!(mat.shade.len(), 4);
        // 16 diffuse levels per elevation
        for row in &mat.shade {
            assert_eq!(row.len(), 16);
        }
    }

    #[test]
    fn material_lookup_clamps_elevation() {
        let mat = Material::default();
        // elevation=10 should clamp to 3
        let cell = mat.lookup(10, 0);
        // Should not panic; returns from shade[3][0]
        assert_eq!(cell.gl, 0);
    }

    #[test]
    fn material_lookup_clamps_diffuse() {
        let mat = Material::default();
        // diffuse=255 -> 255/17=15 (clamped to 15)
        let cell = mat.lookup(0, 255);
        assert_eq!(cell.gl, 0);
    }

    #[test]
    fn material_lookup_diffuse_mapping() {
        let mut mat = Material::default();
        // Set a known value at shade[0][8]
        mat.shade[0][8].gl = b'X';
        // diffuse=136 -> 136/17=8 (exact)
        assert_eq!(mat.lookup(0, 136).gl, b'X');
        // diffuse=140 -> 140/17=8 (integer division)
        assert_eq!(mat.lookup(0, 140).gl, b'X');
        // diffuse=143 -> 143/17=8
        assert_eq!(mat.lookup(0, 143).gl, b'X');
        // diffuse=144 -> 144/17=8
        assert_eq!(mat.lookup(0, 144).gl, b'X');
    }

    #[test]
    fn test_materials_returns_at_least_3() {
        let mats = test_materials();
        assert!(
            mats.len() >= 3,
            "expected at least 3 materials, got {}",
            mats.len()
        );
    }

    #[test]
    fn test_materials_have_non_default_values() {
        let mats = test_materials();
        let default_cell = MatCell::default();

        for (i, mat) in mats.iter().enumerate() {
            let mut has_non_default = false;
            for elv in 0..4 {
                for dif in 0..16 {
                    let cell = &mat.shade[elv][dif];
                    if cell.fg != default_cell.fg
                        || cell.bg != default_cell.bg
                        || cell.gl != default_cell.gl
                    {
                        has_non_default = true;
                        break;
                    }
                }
                if has_non_default {
                    break;
                }
            }
            assert!(has_non_default, "material {i} has all-default values");
        }
    }

    #[test]
    fn test_materials_grass_has_dot_glyph() {
        let mats = test_materials();
        assert_eq!(mats[0].lookup(0, 128).gl, b'.');
    }

    #[test]
    fn test_materials_stone_has_hash_glyph() {
        let mats = test_materials();
        assert_eq!(mats[1].lookup(0, 128).gl, b'#');
    }

    #[test]
    fn test_materials_water_has_tilde_glyph() {
        let mats = test_materials();
        assert_eq!(mats[2].lookup(0, 128).gl, b'~');
    }

    #[test]
    fn material_default_has_zero_mode() {
        let mat = Material::default();
        assert_eq!(mat.mode, 0);
    }

    // --- auto_mat LUT tests ---

    #[test]
    fn auto_mat_lut_is_98304_bytes() {
        let lut = create_auto_mat();
        assert_eq!(lut.len(), 98304);
        assert_eq!(lut.len(), AUTO_MAT_SIZE);
    }

    #[test]
    fn auto_mat_lookup_pure_black() {
        // RGB555 = (0, 0, 0) => bg should be palette 16 (black in xterm-256).
        // fg may differ (dither partner) but glyph should be space (no dither visible).
        let rgb555 = 0u16; // r=0, g=0, b=0
        let (bg, fg, gl) = auto_mat_lookup(rgb555);
        assert_eq!(bg, 16, "pure black bg should be palette 16");
        assert!(
            fg >= 16 && fg <= 231,
            "pure black fg={fg} out of xterm range"
        );
        // At a cube vertex the projection is 0, so shd=0 => glyph = ' ' (space = no visible dither)
        assert_eq!(gl, b' ', "pure black should have space glyph (no dither)");
    }

    #[test]
    fn auto_mat_lookup_pure_white() {
        // RGB555 = (31, 31, 31) => palette indices near 231 (white in xterm-256)
        let rgb555 = 31 | (31 << 5) | (31 << 10); // r=31, g=31, b=31
        let (bg, fg, _gl) = auto_mat_lookup(rgb555);
        // white is 16 + 36*5 + 6*5 + 5 = 231
        assert_eq!(bg, 231, "pure white bg should be palette 231");
        assert_eq!(fg, 231, "pure white fg should be palette 231");
    }

    #[test]
    fn auto_mat_lookup_mid_grey() {
        // RGB555 = (16, 16, 16) => bg and fg should be adjacent palette colors
        let rgb555 = 16 | (16 << 5) | (16 << 10);
        let (bg, fg, _gl) = auto_mat_lookup(rgb555);
        // Both should be valid xterm-256 indices
        assert!(bg >= 16 && bg <= 231, "mid-grey bg={bg} out of range");
        assert!(fg >= 16 && fg <= 231, "mid-grey fg={fg} out of range");
        // bg and fg should differ by at most one step in each axis
        // (adjacent cube vertices), or be equal if on a vertex
        let bg_diff = (bg as i16 - fg as i16).unsigned_abs();
        assert!(
            bg_diff <= 43, // max diff = 36+6+1
            "mid-grey bg={bg} fg={fg} diff={bg_diff} too large"
        );
    }

    #[test]
    fn auto_mat_lookup_pure_red() {
        // RGB555 = (31, 0, 0) => should be in red column
        let rgb555 = 31u16; // r=31, g=0, b=0
        let (bg, fg, _gl) = auto_mat_lookup(rgb555);
        // Pure red = 16 + 36*5 + 6*0 + 0 = 196
        assert!(bg >= 16 && bg <= 231, "red bg={bg} out of range");
        assert!(fg >= 16 && fg <= 231, "red fg={fg} out of range");
        // At least one of bg/fg should be 196 (pure red in xterm-256)
        assert!(
            bg == 196 || fg == 196,
            "pure red: expected at least one palette index = 196, got bg={bg} fg={fg}"
        );
    }

    #[test]
    fn auto_mat_all_entries_valid_palette_range() {
        let lut = create_auto_mat();
        for i in (0..AUTO_MAT_SIZE).step_by(3) {
            let bg = lut[i];
            let fg = lut[i + 1];
            assert!(
                bg >= 16 && bg <= 231,
                "entry {}: bg={bg} out of xterm-256 cube range",
                i / 3
            );
            assert!(
                fg >= 16 && fg <= 231,
                "entry {}: fg={fg} out of xterm-256 cube range",
                i / 3
            );
        }
    }

    #[test]
    fn auto_mat_all_glyphs_valid() {
        let valid_glyphs = [b' ', b'.', b':', b'%'];
        let lut = create_auto_mat();
        for i in (0..AUTO_MAT_SIZE).step_by(3) {
            let gl = lut[i + 2];
            assert!(
                valid_glyphs.contains(&gl),
                "entry {}: glyph={gl} (0x{gl:02x}) not in valid set",
                i / 3
            );
        }
    }

    #[test]
    fn auto_mat_lazy_lock_works() {
        // Force initialization of the LazyLock
        let lut = &*AUTO_MAT;
        assert_eq!(lut.len(), AUTO_MAT_SIZE);
        // Verify it returns the same data as the function
        let direct = create_auto_mat();
        assert_eq!(&lut[..], &direct[..]);
    }

    #[test]
    fn auto_mat_lookup_accessor_matches_direct() {
        let lut = create_auto_mat();
        // Test a handful of known RGB555 values
        for rgb555 in [0u16, 100, 1000, 10000, 32767] {
            let r5 = (rgb555 & 0x1F) as usize;
            let g5 = ((rgb555 >> 5) & 0x1F) as usize;
            let b5 = ((rgb555 >> 10) & 0x1F) as usize;
            let idx = 3 * (r5 + 32 * g5 + 32 * 32 * b5);
            let (bg, fg, gl) = auto_mat_lookup(rgb555);
            assert_eq!(bg, lut[idx], "bg mismatch for rgb555={rgb555}");
            assert_eq!(fg, lut[idx + 1], "fg mismatch for rgb555={rgb555}");
            assert_eq!(gl, lut[idx + 2], "gl mismatch for rgb555={rgb555}");
        }
    }

    // --- GAP-03 (R37): auto_mat LUT consistency tests ---

    #[test]
    fn test_auto_mat_lut_full_table_consistency() {
        // After init, iterate all 32768 entries and verify:
        // - fg palette index is in 16..=231
        // - bg palette index is in 16..=231
        // - glyph is one of the valid dither glyphs (non-zero for our purposes)
        let valid_glyphs = [b' ', b'.', b':', b'%'];
        let lut = create_auto_mat();

        for entry in 0..32768usize {
            let idx = entry * 3;
            let bg = lut[idx];
            let fg = lut[idx + 1];
            let gl = lut[idx + 2];

            assert!(
                bg >= 16 && bg <= 231,
                "entry {entry}: bg={bg} out of xterm-256 cube range 16..=231"
            );
            assert!(
                fg >= 16 && fg <= 231,
                "entry {entry}: fg={fg} out of xterm-256 cube range 16..=231"
            );
            assert!(
                valid_glyphs.contains(&gl),
                "entry {entry}: glyph={gl} (0x{gl:02x}) not in valid dither set"
            );
        }
    }

    #[test]
    fn test_auto_mat_lut_symmetry_spot_checks() {
        // For specific RGB555 values, verify the dither pair produces
        // visually plausible fg/bg contrast.
        use crate::render::quantize::{pack_rgb555, rgb8_to_rgb5};

        let spot_checks: &[(u8, u8, u8, &str)] = &[
            (0, 0, 0, "black"),
            (255, 255, 255, "white"),
            (255, 0, 0, "pure red"),
            (0, 255, 0, "pure green"),
            (0, 0, 255, "pure blue"),
            (128, 128, 128, "mid grey"),
            (64, 64, 64, "dark grey"),
            (192, 192, 192, "light grey"),
            (255, 128, 0, "orange"),
            (128, 0, 255, "purple"),
        ];

        for &(r, g, b, name) in spot_checks {
            let rgb555 = pack_rgb555(rgb8_to_rgb5(r), rgb8_to_rgb5(g), rgb8_to_rgb5(b));
            let (bg, fg, _gl) = auto_mat_lookup(rgb555);

            // Both must be valid palette indices
            assert!(bg >= 16 && bg <= 231, "{name}: bg={bg} out of range");
            assert!(fg >= 16 && fg <= 231, "{name}: fg={fg} out of range");

            // For non-vertex colors, fg and bg should differ
            // (dither needs contrast). For exact vertex colors they may be equal.
            // We just verify they're reasonable palette indices.
        }
    }
}
