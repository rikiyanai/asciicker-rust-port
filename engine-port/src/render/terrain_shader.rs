//! Terrain shader: rasterizes terrain patch triangles into SampleBuffer.
//!
//! Ports the C++ `RenderPatch` logic from render.cpp:404-557.
//! TerrainShader implements `RasterShader` and writes material indices
//! (not RGB555) with `spare = 0` (no MESH_FLAG) so that the resolve stage
//! takes the material path.

use crate::asset_loader::constants::{HEIGHT_CELLS, VISUAL_CELLS};
use crate::render::math::transform_vertex;
use crate::render::rasterizer::{RasterShader, rasterize};
use crate::render::sample_buffer::Sample;
use crate::terrain::patch_runtime::RuntimePatch;

/// Terrain shader implementing `RasterShader` for terrain patch triangles.
///
/// Writes material index into `sample.visual` and sets `spare = 0`
/// (no MESH_FLAG), directing the resolve stage to the material path.
pub struct TerrainShader {
    /// Material index from `patch.visual[vy][vx]`.
    pub material_index: u16,
    /// Base diffuse lighting value, modulated by shadow state.
    pub diffuse_base: u8,
}

impl RasterShader for TerrainShader {
    fn blend(&self, sample: &mut Sample, z: f32, _bc: [f32; 3]) {
        // Depth test: SMALLER z = closer to camera (wins depth test).
        // Inline pattern -- do NOT use depth_test_ro() which has semantic inversion.
        if sample.height > z || sample.height == Sample::CLEAR_HEIGHT {
            sample.visual = self.material_index;
            sample.diffuse = self.diffuse_base;
            sample.spare = 0; // NO MESH_FLAG for terrain
            sample.height = z;
        }
    }
}

/// Rasterize a terrain patch into the sample buffer.
///
/// Triangulates the HEIGHT_CELLS x HEIGHT_CELLS grid (5x5 vertices, 4x4 quads)
/// and calls `rasterize()` for each triangle (2 per quad).
///
/// # Arguments
/// * `buf` - Flat sample buffer slice (row-major, `buf_w * buf_h` elements)
/// * `buf_w` - SAMPLE buffer width (`2*ascii_w + 4`), NOT ASCII width
/// * `buf_h` - SAMPLE buffer height (`2*ascii_h + 4`), NOT ASCII height
/// * `patch` - Runtime terrain patch with height/visual/shadow data
/// * `patch_x` - Patch X coordinate in patch-grid space
/// * `patch_y` - Patch Y coordinate in patch-grid space
/// * `view_tm` - 4x4 row-major view matrix
pub fn render_patch(
    buf: &mut [Sample],
    buf_w: i32,
    buf_h: i32,
    patch: &RuntimePatch,
    patch_x: i32,
    patch_y: i32,
    view_tm: &[f64; 16],
) {
    let diffuse_base: u8 = 0xFF; // Full diffuse as default

    // Scale factor: each height cell spans this many visual cells
    let vis_per_height = VISUAL_CELLS / HEIGHT_CELLS; // = 2

    for hy in 0..HEIGHT_CELLS {
        for hx in 0..HEIGHT_CELLS {
            // Get 4 corner heights for this quad
            let h00 = patch.height[hy][hx] as f64;
            let h10 = patch.height[hy][hx + 1] as f64;
            let h01 = patch.height[hy + 1][hx] as f64;
            let h11 = patch.height[hy + 1][hx + 1] as f64;

            // Compute world-space vertex positions
            let base_x = (patch_x * VISUAL_CELLS as i32 + (hx * vis_per_height) as i32) as f64;
            let base_y = (patch_y * VISUAL_CELLS as i32 + (hy * vis_per_height) as i32) as f64;
            let step = vis_per_height as f64;

            // Look up material from visual grid
            // Each height cell maps to vis_per_height visual cells
            let vx = hx * vis_per_height;
            let vy = hy * vis_per_height;

            // Read shadow bitmask for the visual cells in this quad
            // Process 2x2 visual cells per height cell
            for dv in 0..vis_per_height {
                for du in 0..vis_per_height {
                    let u = vx + du;
                    let v = vy + dv;

                    // Material index: visual[row][col] = visual[vy][vx]
                    let mat_idx = patch.visual[v][u];

                    // Shadow modulation via patch.dark bitmask
                    let cell_bit = u + v * VISUAL_CELLS;
                    let is_shadowed = (patch.dark >> cell_bit) & 1 != 0;
                    let diffuse = if is_shadowed {
                        diffuse_base / 2
                    } else {
                        diffuse_base
                    };

                    let shader = TerrainShader {
                        material_index: mat_idx,
                        diffuse_base: diffuse,
                    };

                    // Compute sub-quad vertices by interpolating the height quad corners
                    // du/dv are 0 or 1, mapping to fractions 0.0 or 0.5 within the height cell
                    let fu0 = du as f64 / vis_per_height as f64;
                    let fu1 = (du + 1) as f64 / vis_per_height as f64;
                    let fv0 = dv as f64 / vis_per_height as f64;
                    let fv1 = (dv + 1) as f64 / vis_per_height as f64;

                    let interp = |fu: f64, fv: f64| -> [i32; 4] {
                        let wx = base_x + fu * step;
                        let wy = base_y + fv * step;
                        let wz = h00 * (1.0 - fu) * (1.0 - fv)
                            + h10 * fu * (1.0 - fv)
                            + h01 * (1.0 - fu) * fv
                            + h11 * fu * fv;
                        transform_vertex([wx, wy, wz], view_tm)
                    };

                    let sv00 = interp(fu0, fv0);
                    let sv10 = interp(fu1, fv0);
                    let sv01 = interp(fu0, fv1);
                    let sv11 = interp(fu1, fv1);

                    // Check diag bit to determine triangle split direction
                    let quad_idx = hx + hy * HEIGHT_CELLS;
                    let diag_bit = (patch.diag >> quad_idx) & 1 != 0;

                    if diag_bit {
                        // Split: (00, 10, 11) and (00, 11, 01)
                        rasterize(
                            buf,
                            buf_w,
                            buf_h,
                            &shader,
                            [&sv00, &sv10, &sv11],
                            false,
                        );
                        rasterize(
                            buf,
                            buf_w,
                            buf_h,
                            &shader,
                            [&sv00, &sv11, &sv01],
                            false,
                        );
                    } else {
                        // Split: (00, 10, 01) and (10, 11, 01)
                        rasterize(
                            buf,
                            buf_w,
                            buf_h,
                            &shader,
                            [&sv00, &sv10, &sv01],
                            false,
                        );
                        rasterize(
                            buf,
                            buf_w,
                            buf_h,
                            &shader,
                            [&sv10, &sv11, &sv01],
                            false,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::TerrainPatch;
    use crate::asset_loader::constants::{HEIGHT_CELLS_PLUS_ONE, VISUAL_CELLS};
    use crate::render::sample_buffer::spare_bits;
    use crate::terrain::patch_runtime::RuntimePatch;

    /// Create a flat test patch at given height.
    fn make_flat_patch(base_height: u16) -> RuntimePatch {
        let tp = TerrainPatch {
            x: 0,
            y: 0,
            height: [[base_height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        };
        RuntimePatch::from_terrain_patch(&tp)
    }

    /// Create a simple identity-like view matrix that maps world coords
    /// to sample buffer coords with offset for the 2-pixel border.
    fn make_test_view_tm() -> [f64; 16] {
        let mut tm = [0.0f64; 16];
        tm[0] = 1.0; // x -> screen x
        tm[5] = 1.0; // y -> screen y
        tm[10] = 1.0; // z -> screen z
        tm[15] = 1.0;
        tm[12] = 4.0; // offset to stay within buffer
        tm[13] = 4.0;
        tm
    }

    #[test]
    fn test_terrain_shader_writes_material_index() {
        let mut sample = Sample::clear_state();
        let shader = TerrainShader {
            material_index: 42,
            diffuse_base: 200,
        };
        shader.blend(&mut sample, 100.0, [0.33, 0.33, 0.34]);

        assert_eq!(sample.visual, 42, "Should write material index");
        assert_eq!(sample.diffuse, 200, "Should write diffuse");
        assert_eq!(
            sample.spare, 0,
            "Terrain must have spare=0 (no MESH_FLAG)"
        );
        assert_eq!(sample.height, 100.0, "Should write depth");
    }

    #[test]
    fn test_terrain_shader_depth_test() {
        // Write a near fragment
        let mut sample = Sample::clear_state();
        let shader_near = TerrainShader {
            material_index: 10,
            diffuse_base: 200,
        };
        shader_near.blend(&mut sample, 50.0, [0.33, 0.33, 0.34]);
        assert_eq!(sample.visual, 10);

        // Try to write a farther fragment -- should NOT overwrite
        let shader_far = TerrainShader {
            material_index: 20,
            diffuse_base: 100,
        };
        shader_far.blend(&mut sample, 200.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 10,
            "Farther fragment should not overwrite closer"
        );

        // Write a closer fragment -- SHOULD overwrite
        let shader_closer = TerrainShader {
            material_index: 30,
            diffuse_base: 255,
        };
        shader_closer.blend(&mut sample, 25.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 30,
            "Closer fragment should overwrite farther"
        );
    }

    #[test]
    fn test_render_patch_produces_samples() {
        // Create a flat patch at height=0
        let patch = make_flat_patch(0);
        let view_tm = make_test_view_tm();

        // Buffer large enough to hold the projected patch
        let buf_w = 32;
        let buf_h = 32;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &view_tm);

        // Count non-clear samples
        let non_clear = buf
            .iter()
            .filter(|s| s.height != Sample::CLEAR_HEIGHT)
            .count();

        // R16-F188 FIX: For a flat 8x8 patch, we expect at least
        // VISUAL_CELLS * VISUAL_CELLS = 64 non-clear samples
        assert!(
            non_clear >= VISUAL_CELLS * VISUAL_CELLS,
            "Expected at least {} non-clear samples, got {}",
            VISUAL_CELLS * VISUAL_CELLS,
            non_clear
        );
    }

    #[test]
    fn test_render_patch_material_mapping() {
        // Create a patch with distinct materials
        let tp = TerrainPatch {
            x: 0,
            y: 0,
            height: [[0u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: {
                let mut v = [[0u16; VISUAL_CELLS]; VISUAL_CELLS];
                v[0][0] = 5;
                v[3][3] = 10;
                v[7][7] = 15;
                v
            },
            diag: 0,
        };
        let patch = RuntimePatch::from_terrain_patch(&tp);
        let view_tm = make_test_view_tm();

        let buf_w = 32;
        let buf_h = 32;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &view_tm);

        // Collect all unique visual values from non-clear samples
        let visuals: std::collections::HashSet<u16> = buf
            .iter()
            .filter(|s| s.height != Sample::CLEAR_HEIGHT)
            .map(|s| s.visual)
            .collect();

        // Should contain the materials we set
        assert!(
            visuals.contains(&5),
            "Should contain material 5, found: {:?}",
            visuals
        );
        assert!(
            visuals.contains(&10),
            "Should contain material 10, found: {:?}",
            visuals
        );
        assert!(
            visuals.contains(&15),
            "Should contain material 15, found: {:?}",
            visuals
        );
    }

    #[test]
    fn test_render_patch_shadow_modulates_diffuse() {
        // Create a patch with shadow on some cells
        let tp = TerrainPatch {
            x: 0,
            y: 0,
            height: [[0u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        };
        let mut patch = RuntimePatch::from_terrain_patch(&tp);
        // Set shadow on cell (0,0): bit 0
        patch.dark = 1;

        let view_tm = make_test_view_tm();
        let buf_w = 32;
        let buf_h = 32;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &view_tm);

        // Collect diffuse values from non-clear samples
        let diffuse_values: std::collections::HashSet<u8> = buf
            .iter()
            .filter(|s| s.height != Sample::CLEAR_HEIGHT)
            .map(|s| s.diffuse)
            .collect();

        // Should have both full diffuse (0xFF) and half diffuse (0xFF/2 = 127)
        assert!(
            diffuse_values.contains(&0xFF),
            "Should have full diffuse for unshadowed cells"
        );
        assert!(
            diffuse_values.contains(&(0xFF / 2)),
            "Should have half diffuse for shadowed cells, found: {:?}",
            diffuse_values
        );
    }

    #[test]
    fn test_terrain_shader_no_mesh_flag() {
        // Verify that terrain samples never have MESH_FLAG
        let patch = make_flat_patch(100);
        let view_tm = make_test_view_tm();
        let buf_w = 32;
        let buf_h = 32;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &view_tm);

        for s in &buf {
            if s.height != Sample::CLEAR_HEIGHT {
                assert_eq!(
                    s.spare & spare_bits::MESH_FLAG,
                    0,
                    "Terrain samples must NOT have MESH_FLAG"
                );
            }
        }
    }
}
