//! Terrain quadtree runtime system.
//!
//! Provides `RuntimeTerrain` as a Bevy `Resource` wrapping the terrain quadtree.
//! The quadtree is built from parsed `A3dTerrain` patches and supports
//! frustum-culled traversal, coordinate lookup, and height interpolation.

use std::collections::HashMap;

use bevy::prelude::*;

pub mod patch_runtime;
pub mod quadtree;
pub mod shadow;

use crate::asset_loader::a3d_terrain::A3dTerrain;
use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};

use patch_runtime::RuntimePatch;
use quadtree::{QuadNode, build_quadtree, query_terrain_frustum};

// ---------------------------------------------------------------------------
// RuntimeTerrain Resource
// ---------------------------------------------------------------------------

/// Runtime terrain resource containing the quadtree spatial index.
///
/// Built from parsed `A3dTerrain` data. Provides frustum-culled patch
/// traversal, coordinate-based lookup, and height interpolation.
#[derive(Resource, Default)]
pub struct RuntimeTerrain {
    /// Root node of the terrain quadtree (None if no patches loaded).
    pub root: Option<QuadNode>,
    /// Tree depth: smallest power of 2 covering the terrain extent.
    pub level: i32,
    /// World-space X origin of the quadtree.
    pub base_x: i32,
    /// World-space Y origin of the quadtree.
    pub base_y: i32,
    /// Total number of terrain patches.
    pub patch_count: usize,
    /// O(1) patch lookup by (x, y) coordinates. Cloned from quadtree leaves.
    patch_map: HashMap<(i32, i32), RuntimePatch>,
}

impl RuntimeTerrain {
    /// Build the quadtree from parsed terrain data.
    ///
    /// P5-070 FIX: `level` is computed as the smallest power of 2 where
    /// `2^level >= max(x_extent, y_extent)`. `base_x = min(patch.x)`,
    /// `base_y = min(patch.y)`. Quadtree covers
    /// `[base_x, base_x + 2^level * VISUAL_CELLS) x [base_y, ...]`.
    pub fn build_from_parsed(terrain: &A3dTerrain) -> Self {
        let patch_count = terrain.patches.len();
        let (root, level, base_x, base_y) = build_quadtree(&terrain.patches);

        // Build O(1) lookup map from quadtree leaves
        let mut patch_map = HashMap::with_capacity(patch_count);
        if let Some(ref r) = root {
            Self::collect_into_map(r, &mut patch_map);
        }

        Self {
            root,
            level,
            base_x,
            base_y,
            patch_count,
            patch_map,
        }
    }

    /// Query visible patches using frustum culling with plane elimination.
    ///
    /// Clones `planes` into a local `Vec` to allow plane elimination during
    /// traversal. Calls `callback` for each visible `RuntimePatch`.
    pub fn query_visible<F>(&self, planes: &[[f64; 4]], mut callback: F)
    where
        F: FnMut(&RuntimePatch),
    {
        if let Some(ref root) = self.root {
            let planes_vec: Vec<[f64; 4]> = planes.to_vec();
            query_terrain_frustum(
                root,
                self.level,
                self.base_x,
                self.base_y,
                &planes_vec,
                &mut callback,
            );
        }
    }

    /// Walk all patches without frustum testing.
    ///
    /// Calls `callback` for each `RuntimePatch` in the quadtree.
    pub fn for_each_patch<F>(&self, mut callback: F)
    where
        F: FnMut(&RuntimePatch),
    {
        if let Some(ref root) = self.root {
            Self::walk_immutable(root, &mut callback);
        }
    }

    /// Walk all patches with mutable access.
    ///
    /// R7-010 FIX: The shadow read pass in 05-06 uses INDEX-BASED iteration
    /// (not closure-based `for_each_patch`) to avoid borrow conflict.
    /// `for_each_patch_mut` is used only for the WRITE pass (setting `patch.dark`).
    pub fn for_each_patch_mut<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut RuntimePatch),
    {
        if let Some(ref mut root) = self.root {
            Self::walk_mutable(root, &mut callback);
        }
        // Sync patch_map after mutation (e.g., shadow dark values).
        self.patch_map.clear();
        if let Some(ref root) = self.root {
            Self::collect_into_map(root, &mut self.patch_map);
        }
    }

    /// Look up a patch by its world coordinates. O(1) via HashMap.
    ///
    /// Returns `Some(&RuntimePatch)` if a patch exists at `(x, y)`, else `None`.
    pub fn get_patch_at(&self, x: i32, y: i32) -> Option<&RuntimePatch> {
        self.patch_map.get(&(x, y))
    }

    /// Bilinear height interpolation at arbitrary world coordinates.
    ///
    /// Returns `None` if `(world_x, world_y)` is outside all loaded terrain
    /// patches. Phase 5 shadow uses f64 directly. Phase 6 physics callers
    /// must cast to f32 at call site:
    /// `let h = terrain.interpolate_height(x, y).map(|v| v as f32)`.
    ///
    /// P5-120 FIX: All callers MUST handle the `None` return (position outside
    /// all terrain patches). The shadow system (Plan 05-06) calls this inside
    /// a raycast loop -- if the ray exits terrain bounds, `interpolate_height`
    /// returns `None` and the caller must `continue` the loop.
    ///
    /// P5-312 FIX: Returns `Option<f64>`. Phase 5 shadow uses f64 directly.
    /// Phase 6 physics callers must cast to f32 at call site.
    pub fn interpolate_height(&self, world_x: f64, world_y: f64) -> Option<f64> {
        // Convert world coordinates to patch coordinates
        let patch_x = (world_x / VISUAL_CELLS as f64).floor() as i32;
        let patch_y = (world_y / VISUAL_CELLS as f64).floor() as i32;

        let patch = self.get_patch_at(patch_x, patch_y)?;

        // Local coordinates within the patch [0..VISUAL_CELLS)
        let local_x = world_x - (patch_x as f64 * VISUAL_CELLS as f64);
        let local_y = world_y - (patch_y as f64 * VISUAL_CELLS as f64);

        // Map local coords to height grid [0..HEIGHT_CELLS]
        let hx = local_x / VISUAL_CELLS as f64 * HEIGHT_CELLS as f64;
        let hy = local_y / VISUAL_CELLS as f64 * HEIGHT_CELLS as f64;

        let ix = hx.floor() as usize;
        let iy = hy.floor() as usize;
        let fx = hx - ix as f64;
        let fy = hy - iy as f64;

        // Clamp to valid indices for bilinear interpolation
        let ix0 = ix.min(HEIGHT_CELLS - 1);
        let iy0 = iy.min(HEIGHT_CELLS - 1);
        let ix1 = ix0 + 1;
        let iy1 = iy0 + 1;

        let h00 = patch.height[iy0][ix0] as f64;
        let h10 = patch.height[iy0][ix1] as f64;
        let h01 = patch.height[iy1][ix0] as f64;
        let h11 = patch.height[iy1][ix1] as f64;

        let height = h00 * (1.0 - fx) * (1.0 - fy)
            + h10 * fx * (1.0 - fy)
            + h01 * (1.0 - fx) * fy
            + h11 * fx * fy;

        Some(height * HEIGHT_SCALE as f64)
    }

    // --- Private helpers ---

    /// Collect all patches from the quadtree into a HashMap for O(1) lookup.
    fn collect_into_map(node: &QuadNode, map: &mut HashMap<(i32, i32), RuntimePatch>) {
        match node {
            QuadNode::Leaf(patch) => {
                map.insert((patch.x, patch.y), patch.clone());
            }
            QuadNode::Interior { children, .. } => {
                for child_node in children.iter().flatten() {
                    Self::collect_into_map(child_node, map);
                }
            }
        }
    }

    fn walk_immutable<F>(node: &QuadNode, callback: &mut F)
    where
        F: FnMut(&RuntimePatch),
    {
        match node {
            QuadNode::Leaf(patch) => callback(patch),
            QuadNode::Interior { children, .. } => {
                for child_node in children.iter().flatten() {
                    Self::walk_immutable(child_node, callback);
                }
            }
        }
    }

    fn walk_mutable<F>(node: &mut QuadNode, callback: &mut F)
    where
        F: FnMut(&mut RuntimePatch),
    {
        match node {
            QuadNode::Leaf(patch) => callback(patch),
            QuadNode::Interior { children, .. } => {
                for child in children.iter_mut().flatten() {
                    Self::walk_mutable(child, callback);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TerrainPlugin
// ---------------------------------------------------------------------------

/// Bevy plugin that registers the `RuntimeTerrain` resource.
///
/// XP-049 FIX: Must be registered BEFORE `CpuRasterizerPlugin` in the app
/// plugin ordering.
///
/// XP-114 FIX: Explicitly calls `app.init_resource::<RuntimeTerrain>()` so
/// that `a3d_assembly_system` can access `ResMut<RuntimeTerrain>` without panic.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // XP-114 FIX: register RuntimeTerrain resource
        app.init_resource::<RuntimeTerrain>();
        info!("TerrainPlugin registered (RuntimeTerrain resource initialized)");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::TerrainPatch;
    use crate::asset_loader::constants::HEIGHT_CELLS_PLUS_ONE;

    fn make_patch(x: i32, y: i32, base_height: u16) -> TerrainPatch {
        TerrainPatch {
            x,
            y,
            height: [[base_height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        }
    }

    fn make_runtime_terrain(patches: &[TerrainPatch]) -> RuntimeTerrain {
        let terrain = A3dTerrain {
            patches: patches.to_vec(),
        };
        RuntimeTerrain::build_from_parsed(&terrain)
    }

    #[test]
    fn test_for_each_patch_visits_all() {
        let patches = vec![
            make_patch(0, 0, 100),
            make_patch(1, 0, 100),
            make_patch(0, 1, 100),
            make_patch(1, 1, 100),
        ];
        let rt = make_runtime_terrain(&patches);

        let mut visited = Vec::new();
        rt.for_each_patch(|p| {
            visited.push((p.x, p.y));
        });

        assert_eq!(visited.len(), 4, "for_each_patch must visit all 4 patches");
        for &(x, y) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            assert!(
                visited.contains(&(x, y)),
                "Patch ({}, {}) must be visited",
                x,
                y
            );
        }
    }

    #[test]
    fn test_get_patch_at_existing() {
        let patches = vec![make_patch(5, 3, 100), make_patch(6, 3, 200)];
        let rt = make_runtime_terrain(&patches);

        let p = rt.get_patch_at(5, 3);
        assert!(p.is_some(), "Patch at (5,3) must exist");
        assert_eq!(p.unwrap().x, 5);
        assert_eq!(p.unwrap().y, 3);
        assert_eq!(p.unwrap().lo, 100);

        let p2 = rt.get_patch_at(6, 3);
        assert!(p2.is_some(), "Patch at (6,3) must exist");
        assert_eq!(p2.unwrap().lo, 200);
    }

    #[test]
    fn test_get_patch_at_missing() {
        let patches = vec![make_patch(0, 0, 100)];
        let rt = make_runtime_terrain(&patches);

        assert!(
            rt.get_patch_at(99, 99).is_none(),
            "Patch at (99,99) should not exist"
        );
    }

    #[test]
    fn test_interpolate_height_center() {
        // Single patch at (0,0) with uniform height 100
        // World coord (4.0, 4.0) is center of patch
        let patches = vec![make_patch(0, 0, 100)];
        let rt = make_runtime_terrain(&patches);

        let h = rt.interpolate_height(4.0, 4.0);
        assert!(h.is_some(), "Height at patch center must return Some");
        let expected = 100.0 * HEIGHT_SCALE as f64;
        assert!(
            (h.unwrap() - expected).abs() < 1e-6,
            "Height should be 100 * HEIGHT_SCALE = {}, got {}",
            expected,
            h.unwrap()
        );
    }

    #[test]
    fn test_interpolate_height_outside() {
        let patches = vec![make_patch(0, 0, 100)];
        let rt = make_runtime_terrain(&patches);

        // Way outside any patch
        assert!(
            rt.interpolate_height(1000.0, 1000.0).is_none(),
            "interpolate_height must return None outside terrain bounds"
        );
    }

    #[test]
    fn test_build_from_parsed() {
        let terrain = A3dTerrain {
            patches: vec![
                make_patch(0, 0, 50),
                make_patch(1, 0, 100),
                make_patch(0, 1, 75),
            ],
        };
        let rt = RuntimeTerrain::build_from_parsed(&terrain);

        assert_eq!(rt.patch_count, 3);
        assert!(rt.root.is_some());
        assert_eq!(rt.base_x, 0);
        assert_eq!(rt.base_y, 0);
    }

    #[test]
    fn test_for_each_patch_mut_sets_dark() {
        let patches = vec![make_patch(0, 0, 100), make_patch(1, 0, 100)];
        let mut rt = make_runtime_terrain(&patches);

        // Set dark bitmask on all patches
        rt.for_each_patch_mut(|p| {
            p.dark = 0xFFFF_FFFF_FFFF_FFFF;
        });

        // Verify dark was set
        rt.for_each_patch(|p| {
            assert_eq!(
                p.dark, 0xFFFF_FFFF_FFFF_FFFF,
                "dark must be set by for_each_patch_mut"
            );
        });
    }
}
