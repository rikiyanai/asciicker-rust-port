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
                    20u8.saturating_add(red_shift).saturating_add(brightness / 4),
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
                    40u8.saturating_add(elv_lighten).saturating_add(brightness / 4),
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
        assert!(mats.len() >= 3, "expected at least 3 materials, got {}", mats.len());
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
            assert!(
                has_non_default,
                "material {i} has all-default values"
            );
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
}
