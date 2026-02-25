//! Runtime terrain patch with computed height bounds and shadow bitmask.
//!
//! Constructed from parsed `TerrainPatch` (asset_loader) for use in the
//! terrain quadtree spatial index.

use crate::asset_loader::a3d_terrain::TerrainPatch;
use crate::asset_loader::constants::{
    HEIGHT_CELLS, HEIGHT_CELLS_PLUS_ONE, HEIGHT_SCALE, VISUAL_CELLS,
};
use crate::physics::collision::ray_triangle_intersection;

/// A runtime terrain patch with precomputed height bounds and shadow state.
///
/// Fields mirror `TerrainPatch` from the asset loader, plus computed `lo`/`hi`
/// height bounds and a 64-bit shadow bitmask (`dark`).
#[derive(Debug, Clone)]
pub struct RuntimePatch {
    /// World X coordinate of this patch.
    pub x: i32,
    /// World Y coordinate of this patch.
    pub y: i32,
    /// 5x5 height vertices (HEIGHT_CELLS+1 per edge).
    pub height: [[u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
    /// 8x8 material cells (VISUAL_CELLS per edge).
    pub visual: [[u16; VISUAL_CELLS]; VISUAL_CELLS],
    /// Triangle orientation bitfield.
    pub diag: u16,
    /// Shadow bitmask: 64 bits for 8x8 cells. Initially 0 (no shadows).
    pub dark: u64,
    /// Minimum height value across the 5x5 height array.
    pub lo: u16,
    /// Maximum height value across the 5x5 height array.
    pub hi: u16,
}

impl RuntimePatch {
    /// Construct a `RuntimePatch` from a parsed `TerrainPatch`.
    ///
    /// Copies all fields and computes `lo`/`hi` by scanning the 5x5 height
    /// array for min/max values. Initializes `dark = 0` (no shadows).
    pub fn from_terrain_patch(patch: &TerrainPatch) -> Self {
        let mut lo = u16::MAX;
        let mut hi = u16::MIN;

        for row in &patch.height {
            for &h in row {
                if h < lo {
                    lo = h;
                }
                if h > hi {
                    hi = h;
                }
            }
        }

        Self {
            x: patch.x,
            y: patch.y,
            height: patch.height,
            visual: patch.visual,
            diag: patch.diag,
            dark: 0,
            lo,
            hi,
        }
    }

    /// Compute world-space coordinates of the center of visual cell (u, v).
    ///
    /// Each patch covers `VISUAL_CELLS` (8) visual cells per edge. The visual
    /// cell grid maps onto the height grid via `HEIGHT_CELLS / VISUAL_CELLS`
    /// ratio. World position is offset by `(px, py)` which are the patch
    /// world coordinates scaled by `VISUAL_CELLS`.
    ///
    /// Returns `[world_x, world_y, world_z]` where Z is the interpolated
    /// height at the cell center, scaled by `HEIGHT_SCALE`.
    pub fn sample_cell_center(&self, u: usize, v: usize, px: i32, py: i32) -> [f64; 3] {
        // FIX TERRAIN-002/003: C++ used wrong variable in boundary check.
        // Both u and v must be checked against VISUAL_CELLS independently.
        let u_clamped = if u < VISUAL_CELLS {
            u
        } else {
            VISUAL_CELLS - 1
        };
        let v_clamped = if v < VISUAL_CELLS {
            v
        } else {
            VISUAL_CELLS - 1
        };

        // Center of cell in [0..VISUAL_CELLS) mapped to [0..1) fractional patch space
        let fu = (u_clamped as f64 + 0.5) / VISUAL_CELLS as f64;
        let fv = (v_clamped as f64 + 0.5) / VISUAL_CELLS as f64;

        // Map fractional position to height grid coordinates
        let hx = fu * HEIGHT_CELLS as f64;
        let hy = fv * HEIGHT_CELLS as f64;

        let ix = hx.floor() as usize;
        let iy = hy.floor() as usize;

        let fx = hx - ix as f64;
        let fy = hy - iy as f64;

        // FIX TERRAIN-004: inclusive boundary (C++ had > instead of >=)
        // Clamp indices to valid range for bilinear interpolation
        let ix0 = ix.min(HEIGHT_CELLS - 1);
        let iy0 = iy.min(HEIGHT_CELLS - 1);
        let ix1 = (ix0 + 1).min(HEIGHT_CELLS);
        let iy1 = (iy0 + 1).min(HEIGHT_CELLS);

        // Bilinear interpolation of height
        let h00 = self.height[iy0][ix0] as f64;
        let h10 = self.height[iy0][ix1] as f64;
        let h01 = self.height[iy1][ix0] as f64;
        let h11 = self.height[iy1][ix1] as f64;

        let height = h00 * (1.0 - fx) * (1.0 - fy)
            + h10 * fx * (1.0 - fy)
            + h01 * (1.0 - fx) * fy
            + h11 * fx * fy;

        let world_x = px as f64 + u_clamped as f64 + 0.5;
        let world_y = py as f64 + v_clamped as f64 + 0.5;
        let world_z = height * HEIGHT_SCALE as f64;

        [world_x, world_y, world_z]
    }

    /// Intersect a ray with the patch's triangle grid.
    ///
    /// Returns Option<toi> for the first hit within [0, max_dist].
    pub fn ray_intersect(&self, origin: [f32; 3], dir: [f32; 3], max_dist: f32) -> Option<f32> {
        let mut best_toi = None;
        let mut current_max = max_dist;

        // Patch covers 4x4 quads. Vertices are 5x5.
        // Vertex spacing is VISUAL_CELLS / HEIGHT_CELLS = 8 / 4 = 2.0 units.
        let spacing = (VISUAL_CELLS / HEIGHT_CELLS) as f32;
        let px = self.x as f32 * VISUAL_CELLS as f32;
        let py = self.y as f32 * VISUAL_CELLS as f32;

        for y in 0..HEIGHT_CELLS {
            for x in 0..HEIGHT_CELLS {
                let x0 = px + x as f32 * spacing;
                let y0 = py + y as f32 * spacing;
                let x1 = x0 + spacing;
                let y1 = y0 + spacing;

                let h00 = self.height[y][x] as f32 / HEIGHT_SCALE as f32;
                let h10 = self.height[y][x + 1] as f32 / HEIGHT_SCALE as f32;
                let h01 = self.height[y + 1][x] as f32 / HEIGHT_SCALE as f32;
                let h11 = self.height[y + 1][x + 1] as f32 / HEIGHT_SCALE as f32;

                let v00 = [x0, y0, h00];
                let v10 = [x1, y0, h10];
                let v01 = [x0, y1, h01];
                let v11 = [x1, y1, h11];

                // Triangle orientation from diag bitfield (1 bit per quad)
                let bit = 1 << (y * HEIGHT_CELLS + x);
                let (tri1, tri2) = if self.diag & bit != 0 {
                    // Split /
                    ([[v00, v10, v11], [v00, v11, v01]])
                } else {
                    // Split \
                    ([[v00, v10, v01], [v10, v11, v01]])
                };

                if let Some(toi) = ray_triangle_intersection(&origin, &dir, &tri1, current_max) {
                    best_toi = Some(toi);
                    current_max = toi;
                }
                if let Some(toi) = ray_triangle_intersection(&origin, &dir, &tri2, current_max) {
                    best_toi = Some(toi);
                    current_max = toi;
                }
            }
        }

        best_toi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_patch(x: i32, y: i32, base_height: u16) -> TerrainPatch {
        TerrainPatch {
            x,
            y,
            height: [[base_height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        }
    }

    #[test]
    fn test_runtime_patch_from_terrain_patch() {
        let mut tp = make_test_patch(10, 20, 100);
        // Set one cell low and one high to test bounds
        tp.height[0][0] = 50;
        tp.height[4][4] = 200;
        tp.visual[3][5] = 42;

        let rp = RuntimePatch::from_terrain_patch(&tp);

        assert_eq!(rp.x, 10);
        assert_eq!(rp.y, 20);
        assert_eq!(rp.lo, 50, "lo must be min of height array");
        assert_eq!(rp.hi, 200, "hi must be max of height array");
        assert_eq!(rp.dark, 0, "dark must be initialized to 0");
        assert_eq!(rp.visual[3][5], 42, "visual materials must match input");
        assert_eq!(rp.height[0][0], 50);
        assert_eq!(rp.height[4][4], 200);
        assert_eq!(rp.diag, 0);
    }

    #[test]
    fn test_terrain_002_003_boundary_check() {
        // Call sample_cell_center at u=7, v=7 (max boundary).
        // No panic, produces valid coordinates.
        let tp = make_test_patch(0, 0, 100);
        let rp = RuntimePatch::from_terrain_patch(&tp);

        let pos = rp.sample_cell_center(7, 7, 0, 0);
        assert!(pos[0].is_finite(), "x must be finite");
        assert!(pos[1].is_finite(), "y must be finite");
        assert!(pos[2].is_finite(), "z must be finite");
        // Cell center at u=7 should be at 7.5
        assert!((pos[0] - 7.5).abs() < 1e-6);
        assert!((pos[1] - 7.5).abs() < 1e-6);
    }

    #[test]
    fn test_terrain_004_inclusive_boundary() {
        // Adjacent patches with shared edge heights. Verify interpolation
        // at exact boundary does not panic and uses inclusive comparison.
        let mut tp = make_test_patch(0, 0, 100);
        // Set boundary edge to different heights
        for i in 0..HEIGHT_CELLS_PLUS_ONE {
            tp.height[i][HEIGHT_CELLS] = 200; // right edge
            tp.height[HEIGHT_CELLS][i] = 150; // bottom edge
        }
        let rp = RuntimePatch::from_terrain_patch(&tp);

        // Sample near the right boundary (u near VISUAL_CELLS-1)
        let pos = rp.sample_cell_center(7, 0, 0, 0);
        assert!(pos[2].is_finite(), "height at boundary must be finite");
        // Height should be interpolated between center values and boundary
        assert!(pos[2] > 0.0, "height must be positive");

        // Sample near the bottom boundary
        let pos2 = rp.sample_cell_center(0, 7, 0, 0);
        assert!(
            pos2[2].is_finite(),
            "height at bottom boundary must be finite"
        );
    }
}
