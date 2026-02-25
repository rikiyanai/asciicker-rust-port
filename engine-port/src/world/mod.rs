//! World system: BSP tree, runtime instances, and frustum-culled traversal.
//!
//! `RuntimeWorld` is the central resource that holds all world instances and
//! the BSP tree for spatial acceleration. It is built from parsed `A3dWorld`
//! data and provides frustum-culled visibility queries with near-child-first
//! ordering, plus sphere queries for physics geometry collection.

use bevy::prelude::*;
use bevy::math::{DMat4, DVec3};

pub mod bsp;
pub mod instance;

use bsp::{
    BspItem, BspNode, VisibleInstance, build_bsp, query_bsp_ray, query_bsp_sphere,
    query_world_frustum,
};
use instance::{InstanceId, RuntimeInstance};

use crate::asset_loader::a3d_world::A3dWorld;
use crate::asset_loader::akm_mesh::AkmMesh;
use crate::physics::collision::ray_triangle_intersection;
use crate::terrain::RuntimeTerrain;

/// The runtime world resource holding all instances and the BSP tree.
///
/// Built from parsed `A3dWorld` via `build_from_parsed`. Provides:
/// - `query_visible`: frustum-culled BSP traversal with near-child-first ordering
/// - `query_sphere`: sphere query for physics geometry collection
#[derive(Resource, Default)]
pub struct RuntimeWorld {
    /// BSP tree root (built from instances with USE_TREE flag).
    pub bsp_root: Option<BspNode>,
    /// All runtime instances (indexed by InstanceId).
    pub instances: Vec<RuntimeInstance>,
    /// Instance IDs not in the BSP tree (items, non-USE_TREE instances).
    pub flat_list: Vec<InstanceId>,
}

impl RuntimeWorld {
    /// Build the runtime world from parsed A3D world data.
    ///
    /// Converts all `WorldInstance`s to `RuntimeInstance`s, separates them into
    /// BSP-tree participants (USE_TREE flag set, non-Item) and flat-list instances,
    /// then builds the BSP tree via SAH.
    pub fn build_from_parsed(world: &A3dWorld, asset_server: Option<&AssetServer>) -> Self {
        let instances: Vec<RuntimeInstance> = world
            .instances
            .iter()
            .map(|wi| RuntimeInstance::from_world_instance(wi, asset_server))
            .collect();

        let mut tree_items = Vec::new();
        let mut flat_list = Vec::new();

        for (idx, inst) in instances.iter().enumerate() {
            let id = InstanceId(idx);
            // P5-066 FIX: Items always skip BSP (USE_TREE = 0 for items).
            // Only non-Item instances with USE_TREE flag go into the BSP.
            if inst.uses_tree() && !inst.is_item() {
                let bbox = *inst.bbox();
                let centroid = [
                    (bbox[0] + bbox[1]) / 2.0,
                    (bbox[2] + bbox[3]) / 2.0,
                    (bbox[4] + bbox[5]) / 2.0,
                ];
                tree_items.push(BspItem { id, bbox, centroid });
            } else {
                flat_list.push(id);
            }
        }

        let bsp_root = if tree_items.is_empty() {
            None
        } else {
            Some(build_bsp(&mut tree_items))
        };

        RuntimeWorld {
            bsp_root,
            instances,
            flat_list,
        }
    }

    /// Query visible instances via frustum-culled BSP traversal.
    ///
    /// `planes` are frustum planes in `[a, b, c, d]` form where `ax + by + cz + d >= 0`
    /// means inside.
    ///
    /// `camera_pos` determines near-child-first ordering for front-to-back rendering.
    ///
    /// Returns `VisibleInstance` for each visible instance (mesh or sprite).
    /// Instances without INST_VISIBLE are skipped.
    pub fn query_visible(&self, planes: &[[f64; 4]], camera_pos: [f64; 3]) -> Vec<VisibleInstance> {
        let mut results = Vec::new();

        // Query BSP tree
        if let Some(ref root) = self.bsp_root {
            query_world_frustum(root, planes, camera_pos, &mut |id: InstanceId| {
                if let Some(inst) = self.instances.get(id.0)
                    && inst.is_visible()
                {
                    let vis = match inst {
                        RuntimeInstance::Mesh { .. } => VisibleInstance::Mesh(id),
                        RuntimeInstance::Sprite { .. } => VisibleInstance::Sprite(id),
                        RuntimeInstance::Item { .. } => VisibleInstance::Sprite(id),
                    };
                    results.push(vis);
                }
            });
        }

        // Also iterate flat_list instances
        for &id in &self.flat_list {
            if let Some(inst) = self.instances.get(id.0)
                && inst.is_visible()
            {
                // Flat-list items still need frustum test (check bbox against planes)
                let bbox = inst.bbox();
                if Self::bbox_inside_frustum(bbox, planes) {
                    let vis = match inst {
                        RuntimeInstance::Mesh { .. } => VisibleInstance::Mesh(id),
                        RuntimeInstance::Sprite { .. } => VisibleInstance::Sprite(id),
                        RuntimeInstance::Item { .. } => VisibleInstance::Sprite(id),
                    };
                    results.push(vis);
                }
            }
        }

        results
    }

    /// Sphere query for physics geometry collection (F034 FIX).
    ///
    /// Traverses the BSP tree, pruning branches whose AABB does not intersect
    /// the query sphere. Returns references to instances within the sphere.
    /// O(log n) average case via BSP pruning.
    ///
    /// **XP-047 NOTE:** Use entity position (`PhysicsIO.pos`) for the center,
    /// NOT the camera position. Camera position is for `query_visible` only.
    pub fn query_sphere(&self, center: [f64; 3], radius: f64) -> Vec<&RuntimeInstance> {
        let radius_sq = radius * radius;
        let mut candidate_ids = Vec::new();

        // Query BSP tree for candidates
        if let Some(ref root) = self.bsp_root {
            query_bsp_sphere(root, center, radius_sq, &mut candidate_ids);
        }

        // Also check flat_list instances
        for &id in &self.flat_list {
            if let Some(inst) = self.instances.get(id.0) {
                let bbox = inst.bbox();
                if Self::bbox_intersects_sphere(bbox, center, radius_sq) {
                    candidate_ids.push(id);
                }
            }
        }

        // Per-instance bbox filtering (leaf nodes may contain instances outside sphere)
        candidate_ids
            .into_iter()
            .filter_map(|id| {
                let inst = self.instances.get(id.0)?;
                let bbox = inst.bbox();
                if Self::bbox_intersects_sphere(bbox, center, radius_sq) {
                    Some(inst)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Test whether an AABB is not fully outside any frustum plane.
    fn bbox_inside_frustum(bbox: &[f64; 6], planes: &[[f64; 4]]) -> bool {
        let corners = [
            [bbox[0], bbox[2], bbox[4]],
            [bbox[1], bbox[2], bbox[4]],
            [bbox[0], bbox[3], bbox[4]],
            [bbox[1], bbox[3], bbox[4]],
            [bbox[0], bbox[2], bbox[5]],
            [bbox[1], bbox[2], bbox[5]],
            [bbox[0], bbox[3], bbox[5]],
            [bbox[1], bbox[3], bbox[5]],
        ];

        for plane in planes {
            let all_outside = corners
                .iter()
                .all(|c| plane[0] * c[0] + plane[1] * c[1] + plane[2] * c[2] + plane[3] < 0.0);
            if all_outside {
                return false;
            }
        }
        true
    }

    /// Test whether an AABB intersects a sphere.
    pub fn bbox_intersects_sphere(bbox: &[f64; 6], center: [f64; 3], radius_sq: f64) -> bool {
        let mut dist_sq = 0.0;
        let closest_x = center[0].clamp(bbox[0], bbox[1]);
        dist_sq += (center[0] - closest_x).powi(2);
        let closest_y = center[1].clamp(bbox[2], bbox[3]);
        dist_sq += (center[1] - closest_y).powi(2);
        let closest_z = center[2].clamp(bbox[4], bbox[5]);
        dist_sq += (center[2] - closest_z).powi(2);
        dist_sq <= radius_sq
    }

    /// Raycast against the world geometry (meshes in BSP tree and flat list).
    ///
    /// Returns Option<(toi, instance_id)> for the first hit within [0, max_dist].
    pub fn raycast_world(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_dist: f32,
        meshes: &Assets<AkmMesh>,
    ) -> Option<(f32, InstanceId)> {
        let origin_arr = [origin.x as f64, origin.y as f64, origin.z as f64];
        let dir = direction.normalize();
        let inv_dir = [1.0 / dir.x as f64, 1.0 / dir.y as f64, 1.0 / dir.z as f64];
        let origin_f32 = [origin.x, origin.y, origin.z];
        let dir_f32 = [dir.x, dir.y, dir.z];

        let mut best_hit: Option<(f32, InstanceId)> = None;
        let mut current_max = max_dist;

        // 1. Query BSP tree
        if let Some(ref root) = self.bsp_root {
            if let Some((id, toi)) = query_bsp_ray(
                root,
                origin_arr,
                [dir.x as f64, dir.y as f64, dir.z as f64],
                inv_dir,
                current_max as f64,
                &mut |id, limit| {
                    self.ray_vs_instance(id, origin_f32, dir_f32, limit as f32, meshes)
                        .map(|t| t as f64)
                },
            ) {
                best_hit = Some((toi as f32, id));
                current_max = toi as f32;
            }
        }

        // 2. Check flat list
        for &id in &self.flat_list {
            if let Some(toi) = self.ray_vs_instance(id, origin_f32, dir_f32, current_max, meshes) {
                best_hit = Some((toi, id));
                current_max = toi;
            }
        }

        best_hit
    }

    /// Unified static raycast against world geometry and terrain.
    ///
    /// Returns Option<toi> for the first hit within [0, max_dist].
    pub fn raycast_static(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_dist: f32,
        meshes: &Assets<AkmMesh>,
        terrain: &RuntimeTerrain,
    ) -> Option<f32> {
        let world_hit = self.raycast_world(origin, direction, max_dist, meshes);
        let terrain_hit = terrain.raycast_terrain(origin, direction, max_dist);

        match (world_hit, terrain_hit) {
            (Some((w_toi, _)), Some(t_toi)) => Some(w_toi.min(t_toi)),
            (Some((w_toi, _)), None) => Some(w_toi),
            (None, Some(t_toi)) => Some(t_toi),
            (None, None) => None,
        }
    }

    /// Intersect a ray with a single instance's geometry.
    fn ray_vs_instance(
        &self,
        id: InstanceId,
        origin: [f32; 3],
        dir: [f32; 3],
        max_dist: f32,
        meshes: &Assets<AkmMesh>,
    ) -> Option<f32> {
        let inst = self.instances.get(id.0)?;
        match inst {
            RuntimeInstance::Mesh {
                mesh_id: _,
                mesh_handle,
                inst_name: _,
                tm,
                bbox: _,
                flags: _,
            } => {
                let mesh = meshes.get(mesh_handle)?;

                // Transform ray to local space
                let local_to_world = DMat4::from_cols_array(tm);
                let world_to_local = local_to_world.inverse();

                let origin_world = DVec3::new(origin[0] as f64, origin[1] as f64, origin[2] as f64);
                let dir_world = DVec3::new(dir[0] as f64, dir[1] as f64, dir[2] as f64);

                let origin_local = world_to_local.transform_point3(origin_world);
                // For direction, we only want rotation/scale, not translation
                let dir_local = world_to_local.transform_vector3(dir_world);

                // Re-normalize local direction and adjust max_dist
                let dir_local_len = dir_local.length();
                if dir_local_len < 1e-6 {
                    return None;
                }
                let dir_local_norm = dir_local / dir_local_len;
                let max_dist_local = max_dist as f64 * dir_local_len;

                let origin_f32 = [origin_local.x as f32, origin_local.y as f32, origin_local.z as f32];
                let dir_f32 = [dir_local_norm.x as f32, dir_local_norm.y as f32, dir_local_norm.z as f32];

                let mut best_toi_local = None;
                let mut current_max_local = max_dist_local as f32;

                for face in &mesh.faces {
                    let v0 = &mesh.vertices[face.indices[0] as usize];
                    let v1 = &mesh.vertices[face.indices[1] as usize];
                    let v2 = &mesh.vertices[face.indices[2] as usize];

                    let tri = [
                        [v0.x, v0.y, v0.z],
                        [v1.x, v1.y, v1.z],
                        [v2.x, v2.y, v2.z],
                    ];

                    if let Some(toi_local) = ray_triangle_intersection(&origin_f32, &dir_f32, &tri, current_max_local) {
                        best_toi_local = Some(toi_local);
                        current_max_local = toi_local;
                    }
                }

                // Convert local TOI back to world TOI
                best_toi_local.map(|t| t / dir_local_len as f32)
            }
            RuntimeInstance::Sprite { .. } | RuntimeInstance::Item { .. } => {
                None
            }
        }
    }
}

/// Bevy plugin that registers the `RuntimeWorld` resource.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // XP-114 FIX: Explicitly init resource so other plugins can access it
        app.init_resource::<RuntimeWorld>();
        info!("WorldPlugin registered (RuntimeWorld resource initialized)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_world::{A3dWorld, WorldInstance};
    use instance::{INST_USE_TREE, INST_VISIBLE};

    fn make_mesh_at(x: f64, y: f64, z: f64, flags: i32) -> WorldInstance {
        let mut tm = vec![0.0; 16];
        // Scale 1, translate to (x, y, z)
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = x;
        tm[13] = y;
        tm[14] = z;
        WorldInstance::Mesh {
            mesh_id: "test.akm".to_string(),
            inst_name: format!("inst_{x}_{y}_{z}"),
            tm,
            flags,
            story_id: -1,
        }
    }

    fn huge_frustum() -> Vec<[f64; 4]> {
        vec![
            [1.0, 0.0, 0.0, 10000.0],
            [-1.0, 0.0, 0.0, 10000.0],
            [0.0, 1.0, 0.0, 10000.0],
            [0.0, -1.0, 0.0, 10000.0],
            [0.0, 0.0, 1.0, 10000.0],
            [0.0, 0.0, -1.0, 10000.0],
        ]
    }

    #[test]
    fn test_build_from_parsed() {
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(10.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(20.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(30.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(40.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);

        // R17-F222 FIX: Assert instance count > 0, BSP root not leaf for multi-instance
        assert_eq!(rt.instances.len(), 5);
        assert!(rt.bsp_root.is_some(), "BSP root should be built");

        // For 5 tree instances, BSP should have some depth
        match rt.bsp_root.as_ref().unwrap() {
            BspNode::Leaf { instances, .. } => {
                // 5 instances fits in a leaf (MAX_LEAF_SIZE=4, but 5 > 4 so should split)
                // Actually 5 > MAX_LEAF_SIZE so it should be Node/NodeShare
                assert!(
                    instances.len() <= 4,
                    "5 instances should trigger a split, not a leaf with {}",
                    instances.len()
                );
            }
            BspNode::Node { .. } | BspNode::NodeShare { .. } => {
                // Expected: tree has depth >= 1
            }
            BspNode::Inst { .. } => {
                panic!("BSP root should not be a single Inst for 5 instances");
            }
        }
    }

    #[test]
    fn test_query_visible_all_inside() {
        // R17-F223 FIX: 5 visible instances all inside frustum -> returns exactly 5
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(10.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(20.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(30.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(40.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);
        let results = rt.query_visible(&huge_frustum(), [0.0, 0.0, 0.0]);

        assert_eq!(
            results.len(),
            5,
            "All 5 visible instances should be returned, got {}",
            results.len()
        );
    }

    #[test]
    fn test_query_visible_none() {
        // R17-F223 FIX: 5 instances all outside frustum -> returns exactly 0
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(10.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(20.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(30.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
                make_mesh_at(40.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);

        // Frustum that is entirely in negative space (all instances at x >= 0)
        let narrow_frustum = vec![
            [-1.0, 0.0, 0.0, -100.0], // x <= -100
        ];
        let results = rt.query_visible(&narrow_frustum, [0.0, 0.0, 0.0]);

        assert_eq!(
            results.len(),
            0,
            "No instances should be visible when frustum excludes all, got {}",
            results.len()
        );
    }

    #[test]
    fn test_inst_visible_flag() {
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE), // visible
                make_mesh_at(10.0, 0.0, 0.0, INST_USE_TREE),               // NOT visible
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);
        let results = rt.query_visible(&huge_frustum(), [0.0, 0.0, 0.0]);

        assert_eq!(results.len(), 1, "Only visible instance should be returned");
    }

    #[test]
    fn test_inst_use_tree_flag() {
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE), // in BSP
                make_mesh_at(10.0, 0.0, 0.0, INST_VISIBLE),                // in flat_list
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);

        // One in BSP, one in flat_list
        assert_eq!(rt.flat_list.len(), 1);

        // Both should be visible in huge frustum
        let results = rt.query_visible(&huge_frustum(), [0.0, 0.0, 0.0]);
        assert_eq!(
            results.len(),
            2,
            "Both BSP and flat-list instances should be visible"
        );
    }

    #[test]
    fn test_query_sphere_returns_nearby_instances() {
        // R17-F219 FIX: Place two instances, one inside and one outside sphere radius
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE), // near origin
                make_mesh_at(100.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE), // far away
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);
        let results = rt.query_sphere([0.0, 0.0, 0.0], 5.0); // radius 5

        // Instance at origin (bbox [-1,1]^3) should be inside sphere of radius 5
        assert!(
            results.len() >= 1,
            "At least the near instance should be found"
        );

        // Instance at x=100 (bbox [99,101]^3) should NOT be inside sphere of radius 5
        let has_far = results.iter().any(|inst| match inst {
            RuntimeInstance::Mesh { tm, .. } => tm[12] > 50.0,
            _ => false,
        });
        assert!(!has_far, "Far instance should not be found");
    }

    #[test]
    fn test_items_skip_bsp() {
        // P5-066 FIX: Items should go to flat_list, not BSP
        let world = A3dWorld {
            format_version: 1,
            instances: vec![
                WorldInstance::Item {
                    item_proto_index: 1,
                    count: 1,
                    pos: [5.0, 5.0, 0.0],
                    yaw: 0.0,
                    flags: INST_VISIBLE, // no USE_TREE
                    story_id: -1,
                },
                make_mesh_at(0.0, 0.0, 0.0, INST_VISIBLE | INST_USE_TREE),
            ],
        };

        let rt = RuntimeWorld::build_from_parsed(&world, None);

        // Item should be in flat_list
        assert!(rt.flat_list.len() >= 1, "Item should be in flat_list");

        // Item should still be visible in query_visible
        let results = rt.query_visible(&huge_frustum(), [0.0, 0.0, 0.0]);
        assert_eq!(results.len(), 2, "Both mesh and item should be visible");
    }
}
