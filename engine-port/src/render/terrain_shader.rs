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
    /// Water surface height (raw u16 units). When set, per-pixel heights
    /// below this are clamped UP to water_z. C++ render.cpp:1584.
    /// This is PER-PIXEL clamping (not vertex-level) so screen position
    /// is preserved while the z-buffer fills at water level.
    pub water_z: Option<f32>,
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
            // C++ render.cpp:1584: clamp underwater terrain height to water level.
            // Per-pixel clamping preserves screen coverage (triangles stay in place)
            // while making underwater terrain visible as a flat z-buffer surface.
            if let Some(wz) = self.water_z {
                if sample.height < wz {
                    sample.height = wz;
                }
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

            // Compute per-quad Lambertian diffuse from actual heights
            let dzdx = (h10_raw - h00_raw) as i32;
            let dzdy = (h01_raw - h00_raw) as i32;
            let diffuse_base = compute_diffuse(dzdx, dzdy);

            // Use raw heights for vertex positions. Water clamping is per-pixel
            // in blend() — preserves screen coverage while filling z-buffer at
            // water level. Vertex-level clamping was wrong (shifted triangles).
            let (h00, h10, h01, h11) = (h00_raw, h10_raw, h01_raw, h11_raw);

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
                        water_z,
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
