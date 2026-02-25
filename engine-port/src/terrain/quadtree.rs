//! Terrain quadtree: spatial index for frustum-culled terrain patch traversal.
//!
//! Converts parsed `TerrainPatch` data into a spatial quadtree with
//! height-bounds propagation and efficient frustum culling with plane
//! elimination.

use crate::asset_loader::a3d_terrain::TerrainPatch;
use crate::asset_loader::constants::HEIGHT_SCALE;

use super::patch_runtime::RuntimePatch;

// ---------------------------------------------------------------------------
// QuadNode enum
// ---------------------------------------------------------------------------

/// A node in the terrain quadtree.
///
/// Interior nodes store 4 optional children (NW, NE, SW, SE) and propagated
/// height bounds. Leaf nodes contain a single `RuntimePatch`.
#[derive(Debug, Clone)]
pub enum QuadNode {
    /// Interior node with up to 4 children and height bounds.
    Interior {
        /// Children in order: NW (0), NE (1), SW (2), SE (3).
        children: [Option<Box<QuadNode>>; 4],
        /// Minimum height across all descendant patches.
        lo: u16,
        /// Maximum height across all descendant patches.
        hi: u16,
    },
    /// Leaf node containing a single terrain patch.
    Leaf(RuntimePatch),
}

impl QuadNode {
    /// Returns the (min, max) height bounds of this node.
    pub fn height_bounds(&self) -> (u16, u16) {
        match self {
            QuadNode::Interior { lo, hi, .. } => (*lo, *hi),
            QuadNode::Leaf(patch) => (patch.lo, patch.hi),
        }
    }
}

// ---------------------------------------------------------------------------
// Quadtree construction
// ---------------------------------------------------------------------------

/// Build a quadtree from parsed terrain patches.
///
/// Returns `(root, level, base_x, base_y)` where:
/// - `root`: the quadtree root node (None if patches is empty)
/// - `level`: tree depth (smallest power of 2 covering the extent)
/// - `base_x`, `base_y`: world-space origin of the quadtree
///
/// The quadtree covers `[base_x .. base_x + 2^level) x [base_y .. base_y + 2^level)`
/// in patch coordinates.
pub fn build_quadtree(patches: &[TerrainPatch]) -> (Option<QuadNode>, i32, i32, i32) {
    if patches.is_empty() {
        return (None, 0, 0, 0);
    }

    // Find bounding box of all patches
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for p in patches {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }

    let base_x = min_x;
    let base_y = min_y;

    // P5-070 FIX: Compute level as smallest power of 2 where 2^level >= max extent
    let x_extent = (max_x - min_x + 1) as u32;
    let y_extent = (max_y - min_y + 1) as u32;
    let max_extent = x_extent.max(y_extent);
    let level = if max_extent <= 1 {
        0
    } else {
        (max_extent - 1).ilog2() as i32 + 1
    };

    // Convert to RuntimePatches
    let runtime_patches: Vec<RuntimePatch> = patches
        .iter()
        .map(RuntimePatch::from_terrain_patch)
        .collect();

    let root = build_node(&runtime_patches, level, base_x, base_y);

    (root, level, base_x, base_y)
}

/// Recursively build a quadtree node for patches within the given region.
///
/// Region covers `[bx .. bx + 2^level) x [by .. by + 2^level)` in patch coords.
fn build_node(patches: &[RuntimePatch], level: i32, bx: i32, by: i32) -> Option<QuadNode> {
    if patches.is_empty() {
        return None;
    }

    // Base case: single patch at level 0
    if level == 0 {
        // There should be exactly one patch here
        if let Some(p) = patches.iter().find(|p| p.x == bx && p.y == by) {
            return Some(QuadNode::Leaf(p.clone()));
        }
        return None;
    }

    let half = 1i32 << (level - 1);
    let mid_x = bx + half;
    let mid_y = by + half;

    // Partition patches into 4 quadrants
    let mut quadrants: [Vec<&RuntimePatch>; 4] = [vec![], vec![], vec![], vec![]];

    for p in patches {
        // FIX TERRAIN-001: C++ terrain.cpp:613 used 'x' instead of 'y' for quadrant
        // Both X and Y must use their respective variables for correct quadrant placement.
        let east = p.x >= mid_x;
        let south = p.y >= mid_y;

        let idx = match (east, south) {
            (false, false) => 0, // NW
            (true, false) => 1,  // NE
            (false, true) => 2,  // SW
            (true, true) => 3,   // SE
        };
        quadrants[idx].push(p);
    }

    // Build children recursively
    let child_offsets = [
        (bx, by),       // NW
        (mid_x, by),    // NE
        (bx, mid_y),    // SW
        (mid_x, mid_y), // SE
    ];

    let mut children: [Option<Box<QuadNode>>; 4] = [None, None, None, None];
    let mut any_child = false;

    for i in 0..4 {
        let child_patches: Vec<RuntimePatch> = quadrants[i].iter().map(|p| (*p).clone()).collect();
        if let Some(node) = build_node(
            &child_patches,
            level - 1,
            child_offsets[i].0,
            child_offsets[i].1,
        ) {
            children[i] = Some(Box::new(node));
            any_child = true;
        }
    }

    if !any_child {
        return None;
    }

    // Propagate height bounds from children
    let mut lo = u16::MAX;
    let mut hi = u16::MIN;
    for node in children.iter().flatten() {
        let (child_lo, child_hi) = node.height_bounds();
        if child_lo < lo {
            lo = child_lo;
        }
        if child_hi > hi {
            hi = child_hi;
        }
    }

    Some(QuadNode::Interior { children, lo, hi })
}

// ---------------------------------------------------------------------------
// Frustum culling
// ---------------------------------------------------------------------------

/// Result of testing an AABB against a frustum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrustumResult {
    /// Entirely outside the frustum (no intersection).
    Outside,
    /// Entirely inside the frustum (all corners on inside of all planes).
    Inside,
    /// Partially overlapping (some corners inside, some outside).
    Partial,
}

/// Compute the 8 corners of an axis-aligned bounding box.
///
/// Box spans `[x0..x1] x [y0..y1] x [z0..z1]` in world space.
fn aabb_corners(x0: f64, x1: f64, y0: f64, y1: f64, z0: f64, z1: f64) -> [[f64; 3]; 8] {
    [
        [x0, y0, z0],
        [x1, y0, z0],
        [x0, y1, z0],
        [x1, y1, z0],
        [x0, y0, z1],
        [x1, y0, z1],
        [x0, y1, z1],
        [x1, y1, z1],
    ]
}

/// Dot product of a frustum plane `[a, b, c, d]` with a point `[x, y, z]`.
///
/// Returns `a*x + b*y + c*z + d`. Positive means inside (or on the plane).
#[inline]
fn dot_plane(plane: &[f64; 4], point: &[f64; 3]) -> f64 {
    plane[0] * point[0] + plane[1] * point[1] + plane[2] * point[2] + plane[3]
}

/// Test an AABB against a set of frustum planes.
///
/// Returns `FrustumResult` and a reduced set of planes (eliminating planes
/// that the AABB is fully inside of, per Pitfall 5: plane elimination).
fn test_aabb_frustum(
    corners: &[[f64; 3]; 8],
    planes: &[[f64; 4]],
) -> (FrustumResult, Vec<[f64; 4]>) {
    let mut remaining_planes = Vec::with_capacity(planes.len());
    let mut all_inside = true;

    for plane in planes {
        let mut inside_count = 0;
        let mut outside_count = 0;

        for corner in corners {
            if dot_plane(plane, corner) >= 0.0 {
                inside_count += 1;
            } else {
                outside_count += 1;
            }
        }

        if inside_count == 0 {
            // All corners outside this plane -> entire AABB outside frustum
            return (FrustumResult::Outside, vec![]);
        }

        if outside_count > 0 {
            // Some corners outside -> partial; keep this plane for children
            remaining_planes.push(*plane);
            all_inside = false;
        }
        // If all 8 corners inside this plane -> eliminate plane (don't add to remaining)
    }

    if all_inside {
        (FrustumResult::Inside, vec![])
    } else {
        (FrustumResult::Partial, remaining_planes)
    }
}

/// Query the terrain quadtree with frustum culling and plane elimination.
///
/// Calls `callback` for each visible `RuntimePatch`. Uses plane elimination
/// (Pitfall 5): when a node's AABB is fully inside a plane, that plane is
/// not tested against children.
///
/// `planes` is a slice of frustum planes `[a, b, c, d]` where `ax+by+cz+d >= 0`
/// means inside.
pub fn query_terrain_frustum<F>(
    node: &QuadNode,
    level: i32,
    bx: i32,
    by: i32,
    planes: &[[f64; 4]],
    callback: &mut F,
) where
    F: FnMut(&RuntimePatch),
{
    // Compute AABB in camera-pos-space (same as frustum planes).
    // Frustum planes are in pos-space: inv(view_tm) divided by HEIGHT_CELLS.
    // Patch coordinate bx maps directly to pos-space x (since world vertex
    // x = bx * HEIGHT_CELLS, divided by HEIGHT_CELLS = bx).
    let size = (1i32 << level) as f64;
    let x0 = bx as f64;
    let y0 = by as f64;
    let x1 = x0 + size;
    let y1 = y0 + size;

    let (lo, hi) = node.height_bounds();
    // Z in raw height units (same space as frustum planes, which are derived
    // from inv(view_tm) — the view matrix operates on raw heightmap z values).
    // +HEIGHT_SCALE accounts for terrain shader's bit-15 elevation boost.
    let z0 = lo as f64;
    let z1 = hi as f64 + HEIGHT_SCALE as f64;

    let corners = aabb_corners(x0, x1, y0, y1, z0, z1);

    if planes.is_empty() {
        // No planes to test -> node is fully inside frustum
        visit_all(node, callback);
        return;
    }

    let (result, remaining) = test_aabb_frustum(&corners, planes);

    match result {
        FrustumResult::Outside => {
            // Skip entire subtree
        }
        FrustumResult::Inside => {
            // All descendants visible
            visit_all(node, callback);
        }
        FrustumResult::Partial => {
            match node {
                QuadNode::Leaf(patch) => {
                    callback(patch);
                }
                QuadNode::Interior { children, .. } => {
                    let half = 1i32 << (level - 1);
                    let mid_x = bx + half;
                    let mid_y = by + half;

                    let child_offsets = [
                        (bx, by),       // NW
                        (mid_x, by),    // NE
                        (bx, mid_y),    // SW
                        (mid_x, mid_y), // SE
                    ];

                    for (i, child) in children.iter().enumerate() {
                        if let Some(child_node) = child {
                            // Pitfall 5: clone remaining planes for each child
                            query_terrain_frustum(
                                child_node,
                                level - 1,
                                child_offsets[i].0,
                                child_offsets[i].1,
                                &remaining,
                                callback,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Visit all patches in a subtree (no frustum testing).
fn visit_all<F>(node: &QuadNode, callback: &mut F)
where
    F: FnMut(&RuntimePatch),
{
    match node {
        QuadNode::Leaf(patch) => callback(patch),
        QuadNode::Interior { children, .. } => {
            for child_node in children.iter().flatten() {
                visit_all(child_node, callback);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ray query
// ---------------------------------------------------------------------------

/// Ray query on the terrain quadtree.
///
/// Traverses the quadtree, culling branches whose AABB does not intersect the ray.
/// Callback returns Option<toi> for a hit on a patch.
pub fn query_terrain_ray<F>(
    node: &QuadNode,
    level: i32,
    bx: i32,
    by: i32,
    origin: [f64; 3],
    inv_dir: [f64; 3],
    max_dist: f64,
    callback: &mut F,
) -> Option<f64>
where
    F: FnMut(&RuntimePatch, f64) -> Option<f64>,
{
    let size = (1i32 << level) as f64;
    let x0 = bx as f64;
    let y0 = by as f64;
    let x1 = x0 + size;
    let y1 = y0 + size;

    let (lo, hi) = node.height_bounds();
    let z0 = lo as f64 / HEIGHT_SCALE as f64;
    let z1 = (hi as f64 + HEIGHT_SCALE as f64) / HEIGHT_SCALE as f64;

    // Ray vs node AABB test
    let mut tmin = 0.0f64;
    let mut tmax = max_dist;

    // X axis
    let tx1 = (x0 - origin[0]) * inv_dir[0];
    let tx2 = (x1 - origin[0]) * inv_dir[0];
    tmin = tmin.max(tx1.min(tx2));
    tmax = tmax.min(tx1.max(tx2));

    // Y axis
    let ty1 = (y0 - origin[1]) * inv_dir[1];
    let ty2 = (y1 - origin[1]) * inv_dir[1];
    tmin = tmin.max(ty1.min(ty2));
    tmax = tmax.min(ty1.max(ty2));

    // Z axis
    let tz1 = (z0 - origin[2]) * inv_dir[2];
    let tz2 = (z1 - origin[2]) * inv_dir[2];
    tmin = tmin.max(tz1.min(tz2));
    tmax = tmax.min(tz1.max(tz2));

    if tmax < tmin || tmin > max_dist {
        return None;
    }

    match node {
        QuadNode::Leaf(patch) => callback(patch, max_dist),
        QuadNode::Interior { children, .. } => {
            let half = 1i32 << (level - 1);
            let mid_x = bx + half;
            let mid_y = by + half;

            let child_offsets = [
                (bx, by),       // NW
                (mid_x, by),    // NE
                (bx, mid_y),    // SW
                (mid_x, mid_y), // SE
            ];

            // Order children by distance along ray? For now, just visit all candidates.
            let mut best_toi = None;
            let mut current_max = max_dist;

            for (i, child) in children.iter().enumerate() {
                if let Some(child_node) = child {
                    if let Some(toi) = query_terrain_ray(
                        child_node,
                        level - 1,
                        child_offsets[i].0,
                        child_offsets[i].1,
                        origin,
                        inv_dir,
                        current_max,
                        callback,
                    ) {
                        best_toi = Some(toi);
                        current_max = toi;
                    }
                }
            }
            best_toi
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::TerrainPatch;
    use crate::asset_loader::constants::{HEIGHT_CELLS_PLUS_ONE, VISUAL_CELLS};

    fn make_patch(x: i32, y: i32, base_height: u16) -> TerrainPatch {
        TerrainPatch {
            x,
            y,
            height: [[base_height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        }
    }

    #[test]
    fn test_empty_patches() {
        let (root, level, bx, by) = build_quadtree(&[]);
        assert!(root.is_none());
        assert_eq!(level, 0);
        assert_eq!(bx, 0);
        assert_eq!(by, 0);
    }

    #[test]
    fn test_single_patch_quadtree() {
        let patches = [make_patch(5, 3, 100)];
        let (root, level, bx, by) = build_quadtree(&patches);

        assert!(root.is_some());
        assert_eq!(level, 0);
        assert_eq!(bx, 5);
        assert_eq!(by, 3);

        // Should be a leaf
        if let Some(QuadNode::Leaf(ref patch)) = root {
            assert_eq!(patch.x, 5);
            assert_eq!(patch.y, 3);
            assert_eq!(patch.lo, 100);
            assert_eq!(patch.hi, 100);
        } else {
            panic!("Expected single patch to be a Leaf node");
        }
    }

    #[test]
    fn test_multiple_patches_quadtree() {
        let patches = [
            make_patch(0, 0, 50),
            make_patch(1, 0, 100),
            make_patch(0, 1, 75),
            make_patch(1, 1, 200),
        ];
        let (root, level, bx, by) = build_quadtree(&patches);

        assert!(root.is_some());
        assert!(level >= 1, "2x2 grid needs level >= 1");
        assert_eq!(bx, 0);
        assert_eq!(by, 0);

        // Count leaves
        let mut count = 0;
        if let Some(ref node) = root {
            count_leaves(node, &mut count);
        }
        assert_eq!(count, 4, "All 4 patches should be in the tree");
    }

    #[test]
    fn test_height_bounds_propagation() {
        let patches = [
            make_patch(0, 0, 50),
            make_patch(1, 0, 100),
            make_patch(0, 1, 75),
            make_patch(1, 1, 200),
        ];
        let (root, _level, _bx, _by) = build_quadtree(&patches);

        let (lo, hi) = root.as_ref().unwrap().height_bounds();
        assert_eq!(lo, 50, "Root lo should be min across all patches");
        assert_eq!(hi, 200, "Root hi should be max across all patches");
    }

    #[test]
    fn test_terrain_001_y_axis_check() {
        // Patches at different Y but same X. Verify quadrant placement uses Y correctly.
        let patches = [make_patch(0, 0, 100), make_patch(0, 1, 200)];
        #[allow(unused_variables)]
        let (root, level, bx, by) = build_quadtree(&patches);

        assert!(root.is_some());
        assert!(level >= 1);

        // Both patches should be present and in different quadrants
        let mut found = Vec::new();
        if let Some(ref node) = root {
            collect_patches(node, &mut found);
        }

        let ys: Vec<i32> = found.iter().map(|p| p.1).collect();
        assert!(ys.contains(&0), "Patch at y=0 must be in tree");
        assert!(ys.contains(&1), "Patch at y=1 must be in tree");

        // They should be in different subtrees since they differ in Y
        if let Some(QuadNode::Interior { ref children, .. }) = root {
            // NW (0) and SW (2) differ by Y axis
            let nw_has_patch = children[0].is_some();
            let sw_has_patch = children[2].is_some();
            assert!(
                nw_has_patch && sw_has_patch,
                "Patches at same X but different Y should be in NW and SW quadrants"
            );
        }
    }

    #[test]
    fn test_frustum_contains_all() {
        let patches = [
            make_patch(0, 0, 100),
            make_patch(1, 0, 100),
            make_patch(0, 1, 100),
            make_patch(1, 1, 100),
        ];
        let (root, level, bx, by) = build_quadtree(&patches);

        // Giant frustum that contains everything.
        // Z range must accommodate height * HEIGHT_SCALE (100 * 16 = 1600).
        let planes = [
            [1.0, 0.0, 0.0, 10000.0],  // x >= -10000
            [-1.0, 0.0, 0.0, 10000.0], // x <= 10000
            [0.0, 1.0, 0.0, 10000.0],  // y >= -10000
            [0.0, -1.0, 0.0, 10000.0], // y <= 10000
            [0.0, 0.0, 1.0, 10000.0],  // z >= -10000
            [0.0, 0.0, -1.0, 10000.0], // z <= 10000
        ];

        let mut visible = Vec::new();
        if let Some(ref node) = root {
            query_terrain_frustum(node, level, bx, by, &planes, &mut |patch| {
                visible.push((patch.x, patch.y));
            });
        }

        assert_eq!(visible.len(), 4, "All 4 patches should be visible");
    }

    #[test]
    fn test_frustum_excludes_all() {
        let patches = [make_patch(0, 0, 100), make_patch(1, 0, 100)];
        let (root, level, bx, by) = build_quadtree(&patches);

        // Frustum entirely to the left of all patches (x < -100 in world space)
        let planes = [
            [-1.0, 0.0, 0.0, -100.0], // -x - 100 >= 0 -> x <= -100
        ];

        let mut visible = Vec::new();
        if let Some(ref node) = root {
            query_terrain_frustum(node, level, bx, by, &planes, &mut |patch| {
                visible.push((patch.x, patch.y));
            });
        }

        assert_eq!(visible.len(), 0, "No patches should be visible");
    }

    #[test]
    fn test_plane_elimination() {
        // Build 4 patches in 2x2 grid, use a frustum that partially overlaps
        let patches = [
            make_patch(0, 0, 100),
            make_patch(1, 0, 100),
            make_patch(0, 1, 100),
            make_patch(1, 1, 100),
        ];
        let (root, level, bx, by) = build_quadtree(&patches);

        // Plane elimination: planes that fully contain a node should be
        // removed from the child test set. We verify by checking that
        // partial overlap produces correct count.
        // AABB is now in pos-space: patch(0,*) covers [0..1), patch(1,*) covers [1..2)
        // Plane: x <= 0.5 should include only left column (partial overlap on patch 0)
        let planes = [
            [-1.0, 0.0, 0.0, 0.5],     // -x + 0.5 >= 0 -> x <= 0.5
            [0.0, 1.0, 0.0, 10000.0],  // always true
            [0.0, -1.0, 0.0, 10000.0], // always true
            [0.0, 0.0, 1.0, 10000.0],  // always true
            [0.0, 0.0, -1.0, 10000.0], // always true
        ];

        let mut visible = Vec::new();
        if let Some(ref node) = root {
            query_terrain_frustum(node, level, bx, by, &planes, &mut |patch| {
                visible.push((patch.x, patch.y));
            });
        }

        // Only patches at x=0 should be visible (AABB [0,1) overlaps x<=0.5)
        // Patches at x=1 have AABB [1,2) entirely outside x<=0.5
        for &(px, _py) in &visible {
            assert_eq!(px, 0, "Only patches at x=0 should be visible, got x={}", px);
        }
        assert_eq!(
            visible.len(),
            2,
            "Exactly 2 patches at x=0 should be visible"
        );
    }

    #[test]
    fn test_frustum_partial_overlap() {
        // R16-F184 FIX: Given 4 patches in 2x2 grid, frustum covering top-left
        // quadrant -> exactly 1 patch visible.
        let patches = [
            make_patch(0, 0, 100),
            make_patch(1, 0, 100),
            make_patch(0, 1, 100),
            make_patch(1, 1, 100),
        ];
        let (root, level, bx, by) = build_quadtree(&patches);

        // Frustum covering only top-left: x <= 0.5, y <= 0.5
        // AABB is in pos-space: Patch(0,0) covers [0..1)x[0..1)
        // Patch(1,0) covers [1..2)x[0..1) -> outside (x>0.5)
        // Patch(0,1) covers [0..1)x[1..2) -> outside (y>0.5)
        // Patch(1,1) covers [1..2)x[1..2) -> outside
        let planes = [
            [-1.0, 0.0, 0.0, 0.5],     // x <= 0.5
            [0.0, -1.0, 0.0, 0.5],     // y <= 0.5
            [0.0, 0.0, 1.0, 10000.0],  // z always inside
            [0.0, 0.0, -1.0, 10000.0], // z always inside
        ];

        let mut visible = Vec::new();
        if let Some(ref node) = root {
            query_terrain_frustum(node, level, bx, by, &planes, &mut |patch| {
                visible.push((patch.x, patch.y));
            });
        }

        assert_eq!(visible.len(), 1, "Exactly 1 patch should be visible");
        assert_eq!(visible[0], (0, 0), "Only top-left patch should be visible");
    }

    // Helper: count leaf nodes
    fn count_leaves(node: &QuadNode, count: &mut usize) {
        match node {
            QuadNode::Leaf(_) => *count += 1,
            QuadNode::Interior { children, .. } => {
                for child in children {
                    if let Some(child_node) = child {
                        count_leaves(child_node, count);
                    }
                }
            }
        }
    }

    // Helper: collect all (x, y) from patches
    fn collect_patches(node: &QuadNode, out: &mut Vec<(i32, i32)>) {
        match node {
            QuadNode::Leaf(patch) => out.push((patch.x, patch.y)),
            QuadNode::Interior { children, .. } => {
                for child in children {
                    if let Some(child_node) = child {
                        collect_patches(child_node, out);
                    }
                }
            }
        }
    }
}
