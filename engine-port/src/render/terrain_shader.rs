//! Terrain shader: rasterizes terrain patch triangles into SampleBuffer.
//!
//! Ports the C++ `RenderPatch` logic from render.cpp:404-557.
//! TerrainShader implements `RasterShader` and writes material indices
//! (not RGB555) with `spare = 0` (no MESH_FLAG) so that the resolve stage
//! takes the material path.

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};
use crate::render::camera::GameCamera;
use crate::render::math::transform_vertex;
use crate::render::rasterizer::{RasterShader, rasterize};
use crate::render::sample_buffer::Sample;
use crate::terrain::patch_runtime::RuntimePatch;

/// Default light direction `[lx, ly, lz, ambient]`.
/// Sun from upper-left, 30% ambient. Matches typical C++ scene setup.
const LIGHT_DIR: [f32; 4] = [0.3, -0.3, 1.0, 0.3];

/// Compute per-quad Lambertian diffuse from height gradients.
///
/// Ports C++ render.cpp:1680-1686 `Diffuse(dzdx, dzdy)`.
/// `dzdx` and `dzdy` are integer height differences between quad corners.
///
/// Returns a diffuse intensity 0-255.
fn compute_diffuse(dzdx: i32, dzdy: i32) -> u8 {
    let hs = HEIGHT_SCALE as f32;
    let nl = ((dzdx * dzdx + dzdy * dzdy) as f32 + hs * hs).sqrt();
    let df = (dzdx as f32 * LIGHT_DIR[0] + dzdy as f32 * LIGHT_DIR[1] + hs * LIGHT_DIR[2]) / nl;
    let df = df * (1.0 - 0.5 * LIGHT_DIR[3]) + 0.5 * LIGHT_DIR[3];
    if df <= 0.0 {
        0
    } else {
        (df * 255.0).min(255.0) as u8
    }
}

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
        // Depth test: LARGER z = closer/on top (Z-up world; higher objects occlude lower).
        // C++ render.cpp uses `if(sam.z < z)` — write if new fragment is higher.
        if sample.height < z || sample.height == Sample::CLEAR_HEIGHT {
            // Preserve full material index including bit 15 (elevation flag)
            sample.visual = self.material_index;
            sample.diffuse = self.diffuse_base;
            sample.spare = 0; // NO MESH_FLAG for terrain
            // C++ render.cpp:1602-1603: if bit 15 set, add HEIGHT_SCALE to height
            if self.material_index & 0x8000 != 0 {
                sample.height = z + HEIGHT_SCALE as f32;
            } else {
                sample.height = z;
            }
        }
    }
}

/// Rasterize a terrain patch into the sample buffer.
///
/// Triangulates the HEIGHT_CELLS x HEIGHT_CELLS grid (5x5 vertices, 4x4 quads)
/// and calls `rasterize()` for each triangle (2 per quad).
///
/// # Perspective projection
/// When `camera.perspective` is true, applies per-vertex perspective scaling
/// matching C++ render.cpp:1804-1846. Each vertex screen position is scaled
/// by `1/viewer_distance` relative to the screen center, creating depth foreshortening.
///
/// # Arguments
/// * `buf` - Flat sample buffer slice (row-major, `buf_w * buf_h` elements)
/// * `buf_w` - SAMPLE buffer width (`2*ascii_w + 4`), NOT ASCII width
/// * `buf_h` - SAMPLE buffer height (`2*ascii_h + 4`), NOT ASCII height
/// * `patch` - Runtime terrain patch with height/visual/shadow data
/// * `patch_x` - Patch X coordinate in patch-grid space
/// * `patch_y` - Patch Y coordinate in patch-grid space
/// * `camera` - Camera with view matrix and perspective parameters
pub fn render_patch(
    buf: &mut [Sample],
    buf_w: i32,
    buf_h: i32,
    patch: &RuntimePatch,
    patch_x: i32,
    patch_y: i32,
    camera: &GameCamera,
    water_z: Option<f32>,
) {
    let view_tm = &camera.view_tm;

    // Scale factor: each height cell spans this many visual cells
    let vis_per_height = VISUAL_CELLS / HEIGHT_CELLS; // = 2

    for hy in 0..HEIGHT_CELLS {
        for hx in 0..HEIGHT_CELLS {
            // Get 4 corner heights for this quad
            let h00_raw = patch.height[hy][hx] as f64;
            let h10_raw = patch.height[hy][hx + 1] as f64;
            let h01_raw = patch.height[hy + 1][hx] as f64;
            let h11_raw = patch.height[hy + 1][hx + 1] as f64;

            // Compute per-quad Lambertian diffuse from RAW heights (before water clamp)
            let dzdx = (h10_raw - h00_raw) as i32;
            let dzdy = (h01_raw - h00_raw) as i32;
            let diffuse_base = compute_diffuse(dzdx, dzdy);

            // Clamp vertex heights to water_z for projection. This makes underwater
            // terrain project as a flat surface at water level, filling screen space
            // that would otherwise be black. C++ render.cpp:1584 equivalent.
            let (h00, h10, h01, h11) = if let Some(wz) = water_z {
                let wz = wz as f64;
                (h00_raw.max(wz), h10_raw.max(wz), h01_raw.max(wz), h11_raw.max(wz))
            } else {
                (h00_raw, h10_raw, h01_raw, h11_raw)
            };

            // Compute world-space vertex positions.
            // C++ formula: vx = x * HEIGHT_CELLS + dx * VISUAL_CELLS (render.cpp:1723)
            // Each height vertex is VISUAL_CELLS apart; patch offset is HEIGHT_CELLS.
            let base_x = (patch_x * HEIGHT_CELLS as i32 + hx as i32 * VISUAL_CELLS as i32) as f64;
            let base_y = (patch_y * HEIGHT_CELLS as i32 + hy as i32 * VISUAL_CELLS as i32) as f64;
            let step = VISUAL_CELLS as f64;

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

                    // Early return closure for behind-camera vertices in perspective mode
                    let mut behind_camera = false;

                    let interp = |fu: f64, fv: f64, behind: &mut bool| -> [i32; 4] {
                        let wx = base_x + fu * step;
                        let wy = base_y + fv * step;
                        let wz = h00 * (1.0 - fu) * (1.0 - fv)
                            + h10 * fu * (1.0 - fv)
                            + h01 * (1.0 - fu) * fv
                            + h11 * fu * fv;

                        if camera.perspective {
                            // C++ perspective path (render.cpp:1804-1846):
                            // 1. Compute eye-to-vertex in world units
                            let eye_x = wx as f32 - camera.view_pos[0];
                            let eye_y = wy as f32 - camera.view_pos[1];
                            let eye_z = wz as f32 - camera.view_pos[2];

                            // 2. Distance along view direction
                            let viewer_dist = eye_x * camera.view_dir[0]
                                + eye_y * camera.view_dir[1]
                                + eye_z * camera.view_dir[2];

                            if viewer_dist <= 0.0 {
                                *behind = true;
                                return [0, 0, 0, 0xF]; // fully culled
                            }

                            let recp_dist = 1.0 / viewer_dist;

                            // 3. Base screen position WITHOUT translation
                            let fx = (camera.mul[0] * wx + camera.mul[2] * wy) as f32;
                            let fy = (camera.mul[1] * wx + camera.mul[3] * wy + camera.mul[5] * wz)
                                as f32;

                            // 4. Scale by 1/dist
                            let fx = fx * recp_dist;
                            let fy = fy * recp_dist;

                            // 5. Apply translated offset with perspective
                            let qx = (camera.add[0] as f32 - camera.view_ofs[0]) * recp_dist
                                + camera.view_ofs[0];
                            let qy = (camera.add[1] as f32 - camera.view_ofs[1]) * recp_dist
                                + camera.view_ofs[1];

                            let sx = fx + qx;
                            let sy = fy + qy;

                            let ix = (sx + 0.5).floor() as i32;
                            let iy = (sy + 0.5).floor() as i32;
                            let iz = wz.floor() as i32;

                            let mut cull = 0i32;
                            if ix < 0 {
                                cull |= 1;
                            }
                            if ix > buf_w {
                                cull |= 2;
                            }
                            if iy < 0 {
                                cull |= 4;
                            }
                            if iy > buf_h {
                                cull |= 8;
                            }

                            [ix, iy, iz, cull]
                        } else {
                            transform_vertex([wx, wy, wz], view_tm)
                        }
                    };

                    let sv00 = interp(fu0, fv0, &mut behind_camera);
                    if behind_camera {
                        continue;
                    }
                    let sv10 = interp(fu1, fv0, &mut behind_camera);
                    if behind_camera {
                        continue;
                    }
                    let sv01 = interp(fu0, fv1, &mut behind_camera);
                    if behind_camera {
                        continue;
                    }
                    let sv11 = interp(fu1, fv1, &mut behind_camera);
                    if behind_camera {
                        continue;
                    }

                    // Check diag bit to determine triangle split direction
                    let quad_idx = hx + hy * HEIGHT_CELLS;
                    let diag_bit = (patch.diag >> quad_idx) & 1 != 0;

                    if diag_bit {
                        // Split: (00, 10, 11) and (00, 11, 01)
                        rasterize(buf, buf_w, buf_h, &shader, [&sv00, &sv10, &sv11], false);
                        rasterize(buf, buf_w, buf_h, &shader, [&sv00, &sv11, &sv01], false);
                    } else {
                        // Split: (00, 10, 01) and (10, 11, 01)
                        rasterize(buf, buf_w, buf_h, &shader, [&sv00, &sv10, &sv01], false);
                        rasterize(buf, buf_w, buf_h, &shader, [&sv10, &sv11, &sv01], false);
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
    /// Buffer must be >= 48x48 to hold a patch at (0,0) with C++ vertex scaling
    /// (vertices span 0..32 + offset 4 = 4..36).
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

    /// Create a non-perspective camera with a test view matrix.
    /// Used by existing tests that don't need perspective projection.
    fn make_test_camera() -> GameCamera {
        let tm = make_test_view_tm();
        GameCamera {
            perspective: false,
            view_tm: tm,
            ..Default::default()
        }
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
        assert_eq!(sample.spare, 0, "Terrain must have spare=0 (no MESH_FLAG)");
        assert_eq!(sample.height, 100.0, "Should write depth");
    }

    #[test]
    fn test_terrain_shader_depth_test() {
        // Write an initial fragment (via CLEAR_HEIGHT path)
        let mut sample = Sample::clear_state();
        let shader_low = TerrainShader {
            material_index: 10,
            diffuse_base: 200,
        };
        shader_low.blend(&mut sample, 50.0, [0.33, 0.33, 0.34]);
        assert_eq!(sample.visual, 10);

        // Write a HIGHER z fragment -- SHOULD overwrite (higher = on top in Z-up world)
        let shader_high = TerrainShader {
            material_index: 20,
            diffuse_base: 100,
        };
        shader_high.blend(&mut sample, 200.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 20,
            "Higher z fragment should overwrite lower (on top in Z-up)"
        );

        // Write a LOWER z fragment -- should NOT overwrite
        let shader_below = TerrainShader {
            material_index: 30,
            diffuse_base: 255,
        };
        shader_below.blend(&mut sample, 25.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 20,
            "Lower z fragment should not overwrite higher (underneath in Z-up)"
        );
    }

    #[test]
    fn test_render_patch_produces_samples() {
        // Create a flat patch at height=0
        let patch = make_flat_patch(0);
        let camera = make_test_camera();

        // Buffer large enough to hold the projected patch
        let buf_w = 48;
        let buf_h = 48;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &camera, None);

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
        let camera = make_test_camera();

        let buf_w = 48;
        let buf_h = 48;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &camera, None);

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

        let camera = make_test_camera();
        let buf_w = 48;
        let buf_h = 48;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &camera, None);

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
        let camera = make_test_camera();
        let buf_w = 48;
        let buf_h = 48;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 0, &camera, None);

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

    #[test]
    fn test_render_patch_with_real_camera_tm() {
        // Reproduce the exact runtime scenario: camera at pos=[0,15,0], yaw=45°,
        // buffer 516x184, rendering patch at grid (0,15).
        use crate::render::camera::GameCamera;

        let mut camera = GameCamera {
            pos: [0.0, 15.0, 0.0],
            yaw: 45.0,
            zoom: 1.0,
            perspective: true,
            ..Default::default()
        };
        camera.update(516.0, 184.0);

        let patch = make_flat_patch(100); // flat at height 100
        let buf_w: i32 = 516;
        let buf_h: i32 = 184;
        let mut buf = vec![Sample::clear_state(); (buf_w * buf_h) as usize];

        // Render patch at grid (0, 15) — near the camera
        render_patch(&mut buf, buf_w, buf_h, &patch, 0, 15, &camera, None);

        let non_clear = buf
            .iter()
            .filter(|s| s.height != Sample::CLEAR_HEIGHT)
            .count();

        // Debug: print first 4 sub-quad vertices
        let hc = HEIGHT_CELLS;
        let vc = VISUAL_CELLS;
        let base_x = (0 * hc as i32 + 0 * vc as i32) as f64;
        let base_y = (15 * hc as i32 + 0 * vc as i32) as f64;
        let sv00 = transform_vertex([base_x, base_y, 100.0], &camera.view_tm);
        let sv10 = transform_vertex([base_x + 4.0, base_y, 100.0], &camera.view_tm);
        let sv01 = transform_vertex([base_x, base_y + 4.0, 100.0], &camera.view_tm);
        eprintln!(
            "TEST: base=({},{}) sv00=({},{},{}) sv10=({},{},{}) sv01=({},{},{})",
            base_x,
            base_y,
            sv00[0],
            sv00[1],
            sv00[2],
            sv10[0],
            sv10[1],
            sv10[2],
            sv01[0],
            sv01[1],
            sv01[2],
        );
        let area = 2
            * ((sv10[0] - sv00[0]) * (sv01[1] - sv00[1])
                - (sv10[1] - sv00[1]) * (sv01[0] - sv00[0]));
        eprintln!("TEST: triangle area={}, non_clear={}", area, non_clear);

        assert!(
            non_clear > 0,
            "Patch at (0,15) with real camera should produce samples, got {} non-clear out of {}",
            non_clear,
            buf.len()
        );
    }
}
