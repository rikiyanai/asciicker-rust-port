//! Geometry collection for physics collision.
//!
//! Free functions that collect terrain and world mesh triangles into the
//! collision soup. No trait: two static geometry sources do not justify
//! abstraction.

use bevy::prelude::warn;

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};
use crate::terrain::RuntimeTerrain;
use crate::world::RuntimeWorld;
use crate::world::instance::RuntimeInstance;

use super::soup::{SoupItem, to_sphere_space};

/// Compute triangle normal from three vertices. Returns `[nx, ny, nz, w]`
/// where the plane equation is `nx*x + ny*y + nz*z + w = 0`.
fn compute_plane(v0: &[f32; 3], v1: &[f32; 3], v2: &[f32; 3]) -> [f32; 4] {
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];

    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-12 {
        return [0.0, 0.0, 1.0, 0.0]; // Degenerate triangle fallback
    }

    let nx = nx / len;
    let ny = ny / len;
    let nz = nz / len;
    let w = -(nx * v0[0] + ny * v0[1] + nz * v0[2]);

    [nx, ny, nz, w]
}

/// Collect terrain collision triangles near `center` within `radius` (world-space).
///
/// Transforms vertices to sphere space. Sets material from terrain vmap.
///
/// Port from C++ physics.cpp PatchCollect (lines 187-250):
/// 1. Compute patch coordinate range (each patch covers VISUAL_CELLS world units)
/// 2. For each patch, triangulate the 5x5 height grid (32 triangles per patch)
/// 3. Transform to sphere space, compute normals, push SoupItems
///
/// R19-M05: RuntimePatch.height is 2D array `[[u16; 5]; 5]`.
/// R16-F195: Each patch produces exactly 4x4x2 = 32 triangles.
pub fn collect_terrain_triangles(
    terrain: &RuntimeTerrain,
    center: &[f32; 3],
    radius: f32,
    mul_xy: f32,
    mul_z: f32,
    soup: &mut Vec<SoupItem>,
) {
    let px_min = ((center[0] - radius) / VISUAL_CELLS as f32).floor() as i32;
    let px_max = ((center[0] + radius) / VISUAL_CELLS as f32).ceil() as i32;
    let py_min = ((center[1] - radius) / VISUAL_CELLS as f32).floor() as i32;
    let py_max = ((center[1] + radius) / VISUAL_CELLS as f32).ceil() as i32;

    // Cap patch iteration count to prevent excessive work (R19-PERF)
    let patch_count = ((px_max - px_min + 1) * (py_max - py_min + 1)).max(0);
    if patch_count > 500 {
        warn!(
            "collect_terrain_triangles: excessive patch count {} (radius={:.1}), capping",
            patch_count, radius
        );
    }

    // Vertex stepping: sxy = VISUAL_CELLS / HEIGHT_CELLS = 2.0
    let sxy = VISUAL_CELLS as f32 / HEIGHT_CELLS as f32;

    for py in py_min..=py_max {
        for px in px_min..=px_max {
            let patch = match terrain.get_patch_at(px, py) {
                Some(p) => p,
                None => continue,
            };

            // Triangulate the HEIGHT_CELLS x HEIGHT_CELLS quad grid
            for row in 0..HEIGHT_CELLS {
                for col in 0..HEIGHT_CELLS {
                    // 4 vertices of this quad
                    let world_verts = |r: usize, c: usize| -> [f32; 3] {
                        let wx = px as f32 * VISUAL_CELLS as f32 + c as f32 * sxy;
                        let wy = py as f32 * VISUAL_CELLS as f32 + r as f32 * sxy;
                        let wz = patch.height[r][c] as f32 / HEIGHT_SCALE as f32;
                        [wx, wy, wz]
                    };

                    let v00 = world_verts(row, col);
                    let v10 = world_verts(row, col + 1);
                    let v01 = world_verts(row + 1, col);
                    let v11 = world_verts(row + 1, col + 1);

                    // Diagonal direction from patch.diag bitfield
                    let bit_idx = row * HEIGHT_CELLS + col;
                    let diag_bit = (patch.diag >> bit_idx) & 1;

                    // Get material from visual map (map height-cell to visual-cell)
                    let vc = (col * VISUAL_CELLS / HEIGHT_CELLS).min(VISUAL_CELLS - 1);
                    let vr = (row * VISUAL_CELLS / HEIGHT_CELLS).min(VISUAL_CELLS - 1);
                    let material = patch.visual[vr][vc] as i32;

                    let (tri_a, tri_b) = if diag_bit == 0 {
                        // Diagonal: v00-v10-v11, v00-v11-v01
                        ([v00, v10, v11], [v00, v11, v01])
                    } else {
                        // Alternate diagonal: v00-v10-v01, v10-v11-v01
                        ([v00, v10, v01], [v10, v11, v01])
                    };

                    // Transform to sphere space and push
                    for tri_verts in &[tri_a, tri_b] {
                        let sv0 = to_sphere_space(&tri_verts[0], center, mul_xy, mul_z);
                        let sv1 = to_sphere_space(&tri_verts[1], center, mul_xy, mul_z);
                        let sv2 = to_sphere_space(&tri_verts[2], center, mul_xy, mul_z);

                        let nrm = compute_plane(&sv0, &sv1, &sv2);

                        soup.push(SoupItem {
                            tri: [sv0, sv1, sv2],
                            material,
                            nrm,
                        });
                    }
                }
            }
        }
    }
}

/// Collect world mesh collision triangles near `center` within `radius` (world-space).
///
/// Uses RuntimeWorld's `query_sphere` for BSP-accelerated spatial lookup.
/// For each mesh instance, generates bbox proxy (12 triangles from AABB faces).
/// Transforms to sphere space.
///
/// R19-F003: Uses query_sphere which already exists on RuntimeWorld.
///
/// TODO (Phase 7): Replace bbox proxy with actual AkmMesh triangles.
pub fn collect_world_triangles(
    world: &RuntimeWorld,
    center: &[f32; 3],
    radius: f32,
    mul_xy: f32,
    mul_z: f32,
    soup: &mut Vec<SoupItem>,
) {
    let center_f64 = [center[0] as f64, center[1] as f64, center[2] as f64];
    let nearby = world.query_sphere(center_f64, radius as f64);

    for inst in nearby {
        if let RuntimeInstance::Mesh { bbox, .. } = inst {
            // Generate 12 triangles (2 per AABB face) as collision proxy
            let [xmin, xmax, ymin, ymax, zmin, zmax] = [
                bbox[0] as f32,
                bbox[1] as f32,
                bbox[2] as f32,
                bbox[3] as f32,
                bbox[4] as f32,
                bbox[5] as f32,
            ];

            // 8 corners of AABB
            let corners = [
                [xmin, ymin, zmin], // 0: ---
                [xmax, ymin, zmin], // 1: +--
                [xmax, ymax, zmin], // 2: ++-
                [xmin, ymax, zmin], // 3: -+-
                [xmin, ymin, zmax], // 4: --+
                [xmax, ymin, zmax], // 5: +-+
                [xmax, ymax, zmax], // 6: +++
                [xmin, ymax, zmax], // 7: -++
            ];

            // 6 faces, 2 triangles each (CCW winding from outside)
            let faces: [[usize; 4]; 6] = [
                [0, 3, 2, 1], // bottom (-Z)
                [4, 5, 6, 7], // top (+Z)
                [0, 1, 5, 4], // front (-Y)
                [2, 3, 7, 6], // back (+Y)
                [0, 4, 7, 3], // left (-X)
                [1, 2, 6, 5], // right (+X)
            ];

            for face in &faces {
                let quads = [
                    [corners[face[0]], corners[face[1]], corners[face[2]]],
                    [corners[face[0]], corners[face[2]], corners[face[3]]],
                ];

                for tri_verts in &quads {
                    let sv0 = to_sphere_space(&tri_verts[0], center, mul_xy, mul_z);
                    let sv1 = to_sphere_space(&tri_verts[1], center, mul_xy, mul_z);
                    let sv2 = to_sphere_space(&tri_verts[2], center, mul_xy, mul_z);

                    let nrm = compute_plane(&sv0, &sv1, &sv2);

                    soup.push(SoupItem {
                        tri: [sv0, sv1, sv2],
                        material: 0, // mesh material = 0
                        nrm,
                    });
                }
            }
        }
    }
}
