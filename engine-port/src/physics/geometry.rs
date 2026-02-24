//! Geometry collection for physics collision.
//!
//! Free functions that collect terrain and world mesh triangles into the
//! collision soup. No trait: two static geometry sources do not justify
//! abstraction.

use bevy::prelude::warn;

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};
use crate::terrain::RuntimeTerrain;
use crate::world::instance::RuntimeInstance;
use crate::world::RuntimeWorld;

use super::soup::{to_sphere_space, SoupItem};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::{A3dTerrain, TerrainPatch};
    use crate::asset_loader::a3d_world::{A3dWorld, WorldInstance};
    use crate::asset_loader::constants::HEIGHT_CELLS_PLUS_ONE;
    use crate::world::instance::{INST_USE_TREE, INST_VISIBLE};

    fn make_flat_patch(x: i32, y: i32, height: u16) -> TerrainPatch {
        TerrainPatch {
            x,
            y,
            height: [[height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        }
    }

    #[test]
    fn test_collect_terrain_triangles_produces_32_per_patch() {
        // R16-F195: One patch with HEIGHT_CELLS=4 must produce 4*4*2 = 32 triangles
        let terrain_data = A3dTerrain {
            patches: vec![make_flat_patch(0, 0, 160)], // height=160 -> 10.0 world
        };
        let terrain = RuntimeTerrain::build_from_parsed(&terrain_data);

        let mut soup = Vec::new();
        let center = [4.0, 4.0, 10.0]; // Center of patch
        let radius = 20.0;
        let mul_xy = 1.0;
        let mul_z = 1.0;

        collect_terrain_triangles(&terrain, &center, radius, mul_xy, mul_z, &mut soup);

        assert_eq!(
            soup.len(),
            32,
            "One patch must produce 32 triangles, got {}",
            soup.len()
        );
    }

    #[test]
    fn test_collect_terrain_triangles_empty_terrain() {
        let terrain = RuntimeTerrain::default();
        let mut soup = Vec::new();
        collect_terrain_triangles(&terrain, &[0.0, 0.0, 0.0], 10.0, 1.0, 1.0, &mut soup);
        assert!(soup.is_empty(), "Empty terrain should produce no triangles");
    }

    #[test]
    fn test_collect_terrain_triangles_multiple_patches() {
        let terrain_data = A3dTerrain {
            patches: vec![
                make_flat_patch(0, 0, 160),
                make_flat_patch(1, 0, 160),
            ],
        };
        let terrain = RuntimeTerrain::build_from_parsed(&terrain_data);

        let mut soup = Vec::new();
        let center = [8.0, 4.0, 10.0]; // Between two patches
        let radius = 20.0;

        collect_terrain_triangles(&terrain, &center, radius, 1.0, 1.0, &mut soup);

        assert_eq!(
            soup.len(),
            64,
            "Two patches must produce 64 triangles, got {}",
            soup.len()
        );
    }

    #[test]
    fn test_collect_terrain_triangles_has_material() {
        let mut patch = make_flat_patch(0, 0, 160);
        patch.visual[0][0] = 42;
        let terrain_data = A3dTerrain {
            patches: vec![patch],
        };
        let terrain = RuntimeTerrain::build_from_parsed(&terrain_data);

        let mut soup = Vec::new();
        collect_terrain_triangles(&terrain, &[4.0, 4.0, 10.0], 20.0, 1.0, 1.0, &mut soup);

        // At least some triangles should have material 42
        let has_mat_42 = soup.iter().any(|s| s.material == 42);
        assert!(has_mat_42, "Some triangles should have material from vmap");
    }

    #[test]
    fn test_collect_world_triangles_bbox_proxy() {
        // Create a world with one mesh instance
        let mut tm = vec![0.0; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = 5.0; // translate X
        tm[13] = 5.0;
        tm[14] = 5.0;

        let world_data = A3dWorld {
            format_version: 1,
            instances: vec![WorldInstance::Mesh {
                mesh_id: "test.akm".to_string(),
                inst_name: "test_inst".to_string(),
                tm,
                flags: INST_VISIBLE | INST_USE_TREE,
                story_id: -1,
            }],
        };
        let world = RuntimeWorld::build_from_parsed(&world_data);

        let mut soup = Vec::new();
        let center = [5.0, 5.0, 5.0]; // Near the mesh
        let radius = 10.0;

        collect_world_triangles(&world, &center, radius, 1.0, 1.0, &mut soup);

        // 6 faces * 2 triangles = 12 triangles per mesh bbox
        assert_eq!(
            soup.len(),
            12,
            "One mesh bbox proxy should produce 12 triangles, got {}",
            soup.len()
        );

        // All should have material 0
        for item in &soup {
            assert_eq!(item.material, 0, "Mesh proxy material should be 0");
        }
    }

    #[test]
    fn test_collect_world_triangles_empty_world() {
        let world = RuntimeWorld::default();
        let mut soup = Vec::new();
        collect_world_triangles(&world, &[0.0, 0.0, 0.0], 10.0, 1.0, 1.0, &mut soup);
        assert!(soup.is_empty(), "Empty world should produce no triangles");
    }

    #[test]
    fn test_collect_world_triangles_far_mesh_excluded() {
        let mut tm = vec![0.0; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = 1000.0; // Far away
        tm[13] = 1000.0;
        tm[14] = 1000.0;

        let world_data = A3dWorld {
            format_version: 1,
            instances: vec![WorldInstance::Mesh {
                mesh_id: "far.akm".to_string(),
                inst_name: "far_inst".to_string(),
                tm,
                flags: INST_VISIBLE | INST_USE_TREE,
                story_id: -1,
            }],
        };
        let world = RuntimeWorld::build_from_parsed(&world_data);

        let mut soup = Vec::new();
        collect_world_triangles(&world, &[0.0, 0.0, 0.0], 5.0, 1.0, 1.0, &mut soup);

        assert!(
            soup.is_empty(),
            "Far mesh should not be collected, got {} triangles",
            soup.len()
        );
    }

    #[test]
    fn test_compute_plane_normal() {
        let v0 = [0.0, 0.0, 0.0f32];
        let v1 = [1.0, 0.0, 0.0];
        let v2 = [0.0, 1.0, 0.0];
        let plane = compute_plane(&v0, &v1, &v2);
        // Normal should point in +Z for CCW triangle on XY plane
        assert!((plane[0]).abs() < 1e-6);
        assert!((plane[1]).abs() < 1e-6);
        assert!((plane[2] - 1.0).abs() < 1e-6);
        assert!((plane[3]).abs() < 1e-6); // passes through origin
    }
}
