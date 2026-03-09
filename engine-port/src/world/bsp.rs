//! BSP tree construction and frustum-culled traversal.
//!
//! Implements a SAH-based BSP tree with 4 node types matching the C++ engine:
//! - `Node`: Interior node with 2 children and a split plane
//! - `NodeShare`: Interior node with straddling instances between children
//! - `Leaf`: Terminal node with a list of instances
//! - `Inst`: Promoted single instance (no further subdivision)
//!
//! Traversal uses near-child-first ordering based on camera position along the
//! split axis, reducing overdraw by processing closer geometry first.

use super::instance::{Bbox, InstanceId};

/// Maximum number of instances in a leaf before attempting subdivision.
const MAX_LEAF_SIZE: usize = 4;

/// BSP tree node types (matches C++ BSP_TYPE_NODE, BSP_TYPE_NODE_SHARE,
/// BSP_TYPE_LEAF, BSP_TYPE_INST).
#[derive(Debug, Clone)]
pub enum BspNode {
    /// Interior node: 2 children separated by a split plane.
    Node {
        children: [Option<Box<BspNode>>; 2],
        bbox: Bbox,
        split_plane: f64,
        split_axis: u8,
    },
    /// Interior node with straddling instances that span the split plane.
    /// Children are traversed in fixed order [0, 1] -- no near-child-first
    /// ordering (R7-009 FIX: NodeShare has no split_plane/split_axis).
    NodeShare {
        children: [Option<Box<BspNode>>; 2],
        bbox: Bbox,
        instances: Vec<InstanceId>,
    },
    /// Terminal node containing a list of instances.
    Leaf {
        bbox: Bbox,
        instances: Vec<InstanceId>,
    },
    /// Promoted single instance (no subdivision needed).
    Inst { bbox: Bbox, instance: InstanceId },
}

/// Item used during BSP construction: ID + precomputed bbox and centroid.
#[derive(Debug, Clone)]
pub struct BspItem {
    pub id: InstanceId,
    pub bbox: Bbox,
    pub centroid: [f64; 3],
}

/// Result of a visibility query callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibleInstance {
    Mesh(InstanceId),
    Sprite(InstanceId),
}

/// Compute the surface area of an AABB (used for SAH cost estimation).
pub fn compute_surface_area(bbox: &Bbox) -> f64 {
    let dx = (bbox[1] - bbox[0]).max(0.0);
    let dy = (bbox[3] - bbox[2]).max(0.0);
    let dz = (bbox[5] - bbox[4]).max(0.0);
    2.0 * (dx * dy + dy * dz + dz * dx)
}

/// Compute the combined AABB of two bounding boxes.
pub fn combined_bbox(a: &Bbox, b: &Bbox) -> Bbox {
    [
        a[0].min(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].max(b[3]),
        a[4].min(b[4]),
        a[5].max(b[5]),
    ]
}

/// Compute the AABB enclosing all items.
fn items_bbox(items: &[BspItem]) -> Bbox {
    let mut bbox = [f64::MAX, f64::MIN, f64::MAX, f64::MIN, f64::MAX, f64::MIN];
    for item in items {
        bbox[0] = bbox[0].min(item.bbox[0]);
        bbox[1] = bbox[1].max(item.bbox[1]);
        bbox[2] = bbox[2].min(item.bbox[2]);
        bbox[3] = bbox[3].max(item.bbox[3]);
        bbox[4] = bbox[4].min(item.bbox[4]);
        bbox[5] = bbox[5].max(item.bbox[5]);
    }
    bbox
}

/// Build a BSP tree from a mutable slice of items using SAH heuristic.
///
/// Tests 3 axes, picks the split that minimizes SAH cost. Creates `NodeShare`
/// when instances straddle the split plane (P5-060 FIX). Sets `split_plane`
/// to the median centroid (P5-074 FIX).
pub fn build_bsp(items: &mut [BspItem]) -> BspNode {
    let bbox = items_bbox(items);

    // Base cases
    match items.len() {
        0 => {
            return BspNode::Leaf {
                bbox,
                instances: Vec::new(),
            };
        }
        1 => {
            return BspNode::Inst {
                bbox,
                instance: items[0].id,
            };
        }
        n if n <= MAX_LEAF_SIZE => {
            return BspNode::Leaf {
                bbox,
                instances: items.iter().map(|it| it.id).collect(),
            };
        }
        _ => {}
    }

    // SAH: test 3 axes, find best split
    let parent_sa = compute_surface_area(&bbox);
    if parent_sa <= 0.0 {
        // Degenerate bbox: all items at same point
        return BspNode::Leaf {
            bbox,
            instances: items.iter().map(|it| it.id).collect(),
        };
    }

    let mut best_cost = f64::MAX;
    let mut best_axis = 0u8;
    let mut best_split_pos = 0usize; // index in sorted array

    for axis in 0..3u8 {
        // Sort items by centroid along this axis
        let axis_idx = axis as usize;
        let mut sorted_indices: Vec<usize> = (0..items.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            items[a].centroid[axis_idx]
                .partial_cmp(&items[b].centroid[axis_idx])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Try each possible split position
        for split_pos in 1..items.len() {
            let mut left_bbox = [f64::MAX, f64::MIN, f64::MAX, f64::MIN, f64::MAX, f64::MIN];
            let mut right_bbox = [f64::MAX, f64::MIN, f64::MAX, f64::MIN, f64::MAX, f64::MIN];

            for &idx in &sorted_indices[..split_pos] {
                let ib = &items[idx].bbox;
                left_bbox[0] = left_bbox[0].min(ib[0]);
                left_bbox[1] = left_bbox[1].max(ib[1]);
                left_bbox[2] = left_bbox[2].min(ib[2]);
                left_bbox[3] = left_bbox[3].max(ib[3]);
                left_bbox[4] = left_bbox[4].min(ib[4]);
                left_bbox[5] = left_bbox[5].max(ib[5]);
            }

            for &idx in &sorted_indices[split_pos..] {
                let ib = &items[idx].bbox;
                right_bbox[0] = right_bbox[0].min(ib[0]);
                right_bbox[1] = right_bbox[1].max(ib[1]);
                right_bbox[2] = right_bbox[2].min(ib[2]);
                right_bbox[3] = right_bbox[3].max(ib[3]);
                right_bbox[4] = right_bbox[4].min(ib[4]);
                right_bbox[5] = right_bbox[5].max(ib[5]);
            }

            let left_sa = compute_surface_area(&left_bbox);
            let right_sa = compute_surface_area(&right_bbox);
            let cost = (left_sa * split_pos as f64 + right_sa * (items.len() - split_pos) as f64)
                / parent_sa;

            if cost < best_cost {
                best_cost = cost;
                best_axis = axis;
                best_split_pos = split_pos;
            }
        }
    }

    // SAH says splitting isn't worth it
    if best_cost >= items.len() as f64 {
        return BspNode::Leaf {
            bbox,
            instances: items.iter().map(|it| it.id).collect(),
        };
    }

    // Sort items by best axis
    let axis_idx = best_axis as usize;
    items.sort_by(|a, b| {
        a.centroid[axis_idx]
            .partial_cmp(&b.centroid[axis_idx])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // P5-074 FIX: Split plane is median centroid between the two groups
    let split_plane = (items[best_split_pos - 1].centroid[axis_idx]
        + items[best_split_pos].centroid[axis_idx])
        / 2.0;

    // P5-060 FIX: Classify items into LEFT, RIGHT, and STRADDLING groups
    // An item straddles if its bbox spans across the split plane on the split axis
    let mut left_items = Vec::new();
    let mut right_items = Vec::new();
    let mut straddling_items = Vec::new();

    for item in items.iter() {
        let lo = item.bbox[axis_idx * 2];
        let hi = item.bbox[axis_idx * 2 + 1];

        if hi <= split_plane {
            left_items.push(item.clone());
        } else if lo >= split_plane {
            right_items.push(item.clone());
        } else {
            // Item straddles the split plane
            straddling_items.push(item.clone());
        }
    }

    // Edge case: if all items ended up on one side or all straddle, make a leaf
    if (left_items.is_empty() && right_items.is_empty())
        || (left_items.is_empty() && straddling_items.len() == items.len())
        || (right_items.is_empty() && straddling_items.len() == items.len())
    {
        return BspNode::Leaf {
            bbox,
            instances: items.iter().map(|it| it.id).collect(),
        };
    }

    let left_child = if left_items.is_empty() {
        None
    } else {
        Some(Box::new(build_bsp(&mut left_items)))
    };

    let right_child = if right_items.is_empty() {
        None
    } else {
        Some(Box::new(build_bsp(&mut right_items)))
    };

    if straddling_items.is_empty() {
        // No straddling instances: create a Node
        BspNode::Node {
            children: [left_child, right_child],
            bbox,
            split_plane,
            split_axis: best_axis,
        }
    } else {
        // Has straddling instances: create a NodeShare
        BspNode::NodeShare {
            children: [left_child, right_child],
            bbox,
            instances: straddling_items.iter().map(|it| it.id).collect(),
        }
    }
}

// --- Frustum query ---

/// Test an AABB against a set of frustum planes.
///
/// Returns:
/// - `None` if the AABB is fully outside any plane (should be culled)
/// - `Some(remaining_planes)` with planes that weren't fully inside eliminated
///
/// Uses the 8-corner test with plane elimination optimization
/// (Pattern 3 from research).
fn frustum_test_bbox(bbox: &Bbox, planes: &[[f64; 4]]) -> Option<Vec<[f64; 4]>> {
    let corners = aabb_corners(bbox);
    let mut remaining = Vec::with_capacity(planes.len());

    for plane in planes {
        let mut all_outside = true;
        let mut all_inside = true;

        for corner in &corners {
            let d = plane[0] * corner[0] + plane[1] * corner[1] + plane[2] * corner[2] + plane[3];
            if d >= 0.0 {
                all_outside = false;
            } else {
                all_inside = false;
            }
        }

        if all_outside {
            return None; // Fully outside this plane
        }

        if !all_inside {
            remaining.push(*plane); // Partially inside -- keep testing
        }
        // If all_inside, eliminate this plane (all descendants also inside)
    }

    Some(remaining)
}

/// Generate the 8 corners of an AABB.
fn aabb_corners(bbox: &Bbox) -> [[f64; 3]; 8] {
    let (xmin, xmax) = (bbox[0], bbox[1]);
    let (ymin, ymax) = (bbox[2], bbox[3]);
    let (zmin, zmax) = (bbox[4], bbox[5]);
    [
        [xmin, ymin, zmin],
        [xmax, ymin, zmin],
        [xmin, ymax, zmin],
        [xmax, ymax, zmin],
        [xmin, ymin, zmax],
        [xmax, ymin, zmax],
        [xmin, ymax, zmax],
        [xmax, ymax, zmax],
    ]
}

/// Frustum-culled BSP traversal with near-child-first ordering.
///
/// `camera_pos` determines which side of each split plane is "near"
/// for front-to-back traversal (reduces overdraw).
///
/// `callback` is invoked for each instance whose containing node passes
/// the frustum test. The callback receives the `InstanceId`.
///
/// **XP-047 NOTE:** `camera_pos` is the rendering camera position for
/// near-child-first ordering. Physics geometry collection uses entity
/// position (`PhysicsIO.pos`) instead -- do not confuse them at call sites.
pub fn query_world_frustum<F>(
    node: &BspNode,
    planes: &[[f64; 4]],
    camera_pos: [f64; 3],
    callback: &mut F,
) where
    F: FnMut(InstanceId),
{
    match node {
        BspNode::Node {
            children,
            bbox,
            split_plane,
            split_axis,
        } => {
            // Frustum test
            let remaining = match frustum_test_bbox(bbox, planes) {
                Some(p) => p,
                None => return, // Fully outside
            };

            // Near-child-first ordering (matches C++ RecurseWorldBSP):
            // Camera position determines which side of the split plane is "near".
            let near = if camera_pos[*split_axis as usize] < *split_plane {
                0
            } else {
                1
            };
            let far = 1 - near;

            // Recurse near child first -> front-to-back
            if let Some(child) = &children[near] {
                query_world_frustum(child, &remaining, camera_pos, callback);
            }
            if let Some(child) = &children[far] {
                query_world_frustum(child, &remaining, camera_pos, callback);
            }
        }

        BspNode::NodeShare {
            children,
            bbox,
            instances,
        } => {
            // Frustum test
            let remaining = match frustum_test_bbox(bbox, planes) {
                Some(p) => p,
                None => return,
            };

            // R7-009 FIX: Fixed order [0, 1] -- no split plane, no near-child-first
            for inst_id in instances {
                callback(*inst_id);
            }
            if let Some(c) = &children[0] {
                query_world_frustum(c, &remaining, camera_pos, callback);
            }
            if let Some(c) = &children[1] {
                query_world_frustum(c, &remaining, camera_pos, callback);
            }
        }

        BspNode::Leaf {
            bbox, instances, ..
        } => {
            if frustum_test_bbox(bbox, planes).is_none() {
                return;
            }
            for inst_id in instances {
                callback(*inst_id);
            }
        }

        BspNode::Inst { bbox, instance, .. } => {
            if frustum_test_bbox(bbox, planes).is_none() {
                return;
            }
            callback(*instance);
        }
    }
}

// --- Sphere query (for physics geometry collection) ---

/// Test whether an AABB intersects a sphere.
///
/// For each axis, find the closest point on the AABB to the sphere center,
/// then check if the squared distance is within radius^2.
fn bbox_intersects_sphere(bbox: &Bbox, center: [f64; 3], radius_sq: f64) -> bool {
    let mut dist_sq = 0.0;

    // X axis: bbox[0]=xmin, bbox[1]=xmax
    let closest_x = center[0].clamp(bbox[0], bbox[1]);
    let dx = center[0] - closest_x;
    dist_sq += dx * dx;

    // Y axis: bbox[2]=ymin, bbox[3]=ymax
    let closest_y = center[1].clamp(bbox[2], bbox[3]);
    let dy = center[1] - closest_y;
    dist_sq += dy * dy;

    // Z axis: bbox[4]=zmin, bbox[5]=zmax
    let closest_z = center[2].clamp(bbox[4], bbox[5]);
    let dz = center[2] - closest_z;
    dist_sq += dz * dz;

    dist_sq <= radius_sq
}

/// Sphere query on the BSP tree for physics geometry collection.
///
/// Traverses the BSP, pruning branches whose AABB does not intersect the
/// query sphere. Returns `InstanceId`s of instances within the sphere.
/// O(log n) average case via BSP pruning (F034 FIX).
pub fn query_bsp_sphere(
    node: &BspNode,
    center: [f64; 3],
    radius_sq: f64,
    results: &mut Vec<InstanceId>,
) {
    match node {
        BspNode::Node { children, bbox, .. } | BspNode::NodeShare { children, bbox, .. } => {
            if !bbox_intersects_sphere(bbox, center, radius_sq) {
                return;
            }

            // For NodeShare, also collect straddling instances
            if let BspNode::NodeShare { instances, .. } = node {
                for inst_id in instances {
                    results.push(*inst_id);
                }
            }

            if let Some(c) = &children[0] {
                query_bsp_sphere(c, center, radius_sq, results);
            }
            if let Some(c) = &children[1] {
                query_bsp_sphere(c, center, radius_sq, results);
            }
        }

        BspNode::Leaf {
            bbox, instances, ..
        } => {
            if !bbox_intersects_sphere(bbox, center, radius_sq) {
                return;
            }
            // Node bbox passed -- but individual instances may still be outside.
            // Caller must provide instance bboxes for per-instance filtering.
            // At this level we collect all candidates; per-instance filtering
            // happens in RuntimeWorld::query_sphere which has access to instance data.
            results.extend_from_slice(instances);
        }

        BspNode::Inst { bbox, instance, .. } => {
            if !bbox_intersects_sphere(bbox, center, radius_sq) {
                return;
            }
            results.push(*instance);
        }
    }
}

// --- Ray query ---

/// Test whether a ray intersects an AABB.
///
/// Returns Option<toi> if the ray intersects the AABB within [0, max_dist].
pub fn ray_intersects_bbox(
    bbox: &Bbox,
    origin: [f64; 3],
    inv_dir: [f64; 3],
    max_dist: f64,
) -> Option<f64> {
    let mut tmin = 0.0f64;
    let mut tmax = max_dist;

    for i in 0..3 {
        let t1 = (bbox[i * 2] - origin[i]) * inv_dir[i];
        let t2 = (bbox[i * 2 + 1] - origin[i]) * inv_dir[i];

        tmin = tmin.max(t1.min(t2));
        tmax = tmax.min(t1.max(t2));
    }

    if tmax >= tmin && tmin < max_dist {
        Some(tmin)
    } else {
        None
    }
}

/// Ray query on the BSP tree.
///
/// Traverses the BSP in near-to-far order along the ray.
/// Prunes branches whose AABB does not intersect the ray.
/// Callback returns Option<toi> for a hit; traversal stops at the first hit.
pub fn query_bsp_ray<F>(
    node: &BspNode,
    origin: [f64; 3],
    dir: [f64; 3],
    inv_dir: [f64; 3],
    max_dist: f64,
    callback: &mut F,
) -> Option<(InstanceId, f64)>
where
    F: FnMut(InstanceId, f64) -> Option<f64>,
{
    match node {
        BspNode::Node {
            children,
            bbox,
            split_plane,
            split_axis,
        } => {
            if ray_intersects_bbox(bbox, origin, inv_dir, max_dist).is_none() {
                return None;
            }

            let axis = *split_axis as usize;
            let near = if origin[axis] < *split_plane { 0 } else { 1 };
            let far = 1 - near;

            // Recurse near child
            if let Some(child) = &children[near] {
                if let Some(hit) = query_bsp_ray(child, origin, dir, inv_dir, max_dist, callback) {
                    return Some(hit);
                }
            }

            // Recurse far child
            if let Some(child) = &children[far] {
                if let Some(hit) = query_bsp_ray(child, origin, dir, inv_dir, max_dist, callback) {
                    return Some(hit);
                }
            }
        }

        BspNode::NodeShare {
            children,
            bbox,
            instances,
        } => {
            if ray_intersects_bbox(bbox, origin, inv_dir, max_dist).is_none() {
                return None;
            }

            let mut best_hit: Option<(InstanceId, f64)> = None;

            // Check straddling instances
            for &id in instances {
                if let Some(toi) = callback(id, max_dist) {
                    if best_hit.is_none() || toi < best_hit.unwrap().1 {
                        best_hit = Some((id, toi));
                    }
                }
            }

            // If we hit a straddling instance, we can't stop yet because a child
            // might have a closer hit. But we can limit max_dist.
            let mut current_max = best_hit.map(|h| h.1).unwrap_or(max_dist);

            if let Some(c) = &children[0] {
                if let Some((id, toi)) = query_bsp_ray(c, origin, dir, inv_dir, current_max, callback) {
                    best_hit = Some((id, toi));
                    current_max = toi;
                }
            }
            if let Some(c) = &children[1] {
                if let Some((id, toi)) = query_bsp_ray(c, origin, dir, inv_dir, current_max, callback) {
                    best_hit = Some((id, toi));
                }
            }

            return best_hit;
        }

        BspNode::Leaf {
            bbox, instances, ..
        } => {
            if ray_intersects_bbox(bbox, origin, inv_dir, max_dist).is_none() {
                return None;
            }

            let mut best_hit: Option<(InstanceId, f64)> = None;
            let mut current_max = max_dist;

            for &id in instances {
                if let Some(toi) = callback(id, current_max) {
                    best_hit = Some((id, toi));
                    current_max = toi;
                }
            }
            return best_hit;
        }

        BspNode::Inst { bbox, instance, .. } => {
            if ray_intersects_bbox(bbox, origin, inv_dir, max_dist).is_none() {
                return None;
            }
            if let Some(toi) = callback(*instance, max_dist) {
                return Some((*instance, toi));
            }
        }
    }
    None
}
