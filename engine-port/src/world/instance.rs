//! Runtime instance types for the world system.
//!
//! Converts parsed `WorldInstance` variants (from the asset loader) into
//! runtime representations with bounding boxes and flag accessors.
//! Instance flags control visibility and BSP membership.

use bevy::prelude::*;
use crate::asset_loader::a3d_world::WorldInstance;
use crate::asset_loader::akm_mesh::AkmMesh;

// --- Instance Flags (matches C++ INST_FLAGS enum) ---

/// Instance is rendered (hidden instances skip query callbacks).
pub const INST_VISIBLE: i32 = 0x1;
/// Instance participates in BSP tree (vs. flat list).
pub const INST_USE_TREE: i32 = 0x2;
/// Temporary instance (NPCs, projectiles) -- excluded from save.
pub const INST_VOLATILE: i32 = 0x4;
/// Editor selection highlight.
pub const INST_SELECTED: i32 = 0x8;

/// Newtype index into the `RuntimeWorld.instances` vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceId(pub usize);

/// Axis-aligned bounding box: `[xmin, xmax, ymin, ymax, zmin, zmax]`.
pub type Bbox = [f64; 6];

/// A runtime instance converted from a parsed `WorldInstance`.
#[derive(Debug, Clone)]
pub enum RuntimeInstance {
    Mesh {
        mesh_id: String,
        mesh_handle: Handle<AkmMesh>,
        inst_name: String,
        tm: [f64; 16],
        bbox: Bbox,
        flags: i32,
    },
    Sprite {
        sprite_name: String,
        pos: [f32; 3],
        yaw: f32,
        anim: i32,
        frame: i32,
        reps: [i32; 4],
        bbox: Bbox,
        flags: i32,
    },
    Item {
        item_proto_index: i32,
        count: i32,
        pos: [f32; 3],
        yaw: f32,
        bbox: Bbox,
        flags: i32,
    },
}

/// Small half-extent for point-like instances (sprites, items).
const POINT_HALF_EXTENT: f64 = 0.5;

/// Compute the AABB of a unit cube `[-1,1]^3` transformed by a 4x4 column-major matrix.
///
/// The 8 corners of the unit cube are transformed, and the AABB is the min/max
/// across all transformed corners. This matches the C++ `GetInstBBox` behavior
/// for mesh instances.
fn compute_transformed_unit_cube_bbox(tm: &[f64; 16]) -> Bbox {
    let mut bbox = [f64::MAX, f64::MIN, f64::MAX, f64::MIN, f64::MAX, f64::MIN];

    for &sx in &[-1.0_f64, 1.0] {
        for &sy in &[-1.0_f64, 1.0] {
            for &sz in &[-1.0_f64, 1.0] {
                // Column-major 4x4: col0=[0..3], col1=[4..7], col2=[8..11], col3=[12..15]
                let x = tm[0] * sx + tm[4] * sy + tm[8] * sz + tm[12];
                let y = tm[1] * sx + tm[5] * sy + tm[9] * sz + tm[13];
                let z = tm[2] * sx + tm[6] * sy + tm[10] * sz + tm[14];

                bbox[0] = bbox[0].min(x);
                bbox[1] = bbox[1].max(x);
                bbox[2] = bbox[2].min(y);
                bbox[3] = bbox[3].max(y);
                bbox[4] = bbox[4].min(z);
                bbox[5] = bbox[5].max(z);
            }
        }
    }

    bbox
}

/// Compute a small AABB around a point position.
fn point_bbox(pos: &[f32; 3]) -> Bbox {
    let x = pos[0] as f64;
    let y = pos[1] as f64;
    let z = pos[2] as f64;
    [
        x - POINT_HALF_EXTENT,
        x + POINT_HALF_EXTENT,
        y - POINT_HALF_EXTENT,
        y + POINT_HALF_EXTENT,
        z - POINT_HALF_EXTENT,
        z + POINT_HALF_EXTENT,
    ]
}

impl RuntimeInstance {
    /// Convert a parsed `WorldInstance` into a `RuntimeInstance`.
    ///
    /// # Panics
    ///
    /// Panics if a `WorldInstance::Mesh` has a `tm` vector with length != 16
    /// (F033 FIX: validate tm length before copying).
    pub fn from_world_instance(wi: &WorldInstance, asset_server: Option<&AssetServer>) -> Self {
        match wi {
            WorldInstance::Mesh {
                mesh_id,
                inst_name,
                tm,
                flags,
                story_id: _,
            } => {
                let tm_arr: [f64; 16] = tm.as_slice().try_into().unwrap_or_else(|_| {
                    panic!(
                        "WorldInstance tm must have exactly 16 elements, got {}",
                        tm.len()
                    );
                });
                let bbox = compute_transformed_unit_cube_bbox(&tm_arr);
                let mesh_handle = if let Some(server) = asset_server {
                    server.load(format!("meshes/{mesh_id}"))
                } else {
                    Handle::default()
                };
                RuntimeInstance::Mesh {
                    mesh_id: mesh_id.clone(),
                    mesh_handle,
                    inst_name: inst_name.clone(),
                    tm: tm_arr,
                    bbox,
                    flags: *flags,
                }
            }
            WorldInstance::Sprite {
                sprite_name,
                pos,
                yaw,
                anim,
                frame,
                reps,
                flags,
                story_id: _,
            } => {
                let bbox = point_bbox(pos);
                RuntimeInstance::Sprite {
                    sprite_name: sprite_name.clone(),
                    pos: *pos,
                    yaw: *yaw,
                    anim: *anim,
                    frame: *frame,
                    reps: *reps,
                    bbox,
                    flags: *flags,
                }
            }
            WorldInstance::Item {
                item_proto_index,
                count,
                pos,
                yaw,
                flags,
                story_id: _,
            } => {
                let bbox = point_bbox(pos);
                RuntimeInstance::Item {
                    item_proto_index: *item_proto_index,
                    count: *count,
                    pos: *pos,
                    yaw: *yaw,
                    bbox,
                    flags: *flags,
                }
            }
        }
    }

    /// Get the axis-aligned bounding box.
    pub fn bbox(&self) -> &Bbox {
        match self {
            RuntimeInstance::Mesh { bbox, .. } => bbox,
            RuntimeInstance::Sprite { bbox, .. } => bbox,
            RuntimeInstance::Item { bbox, .. } => bbox,
        }
    }

    /// Get instance flags.
    pub fn flags(&self) -> i32 {
        match self {
            RuntimeInstance::Mesh { flags, .. } => *flags,
            RuntimeInstance::Sprite { flags, .. } => *flags,
            RuntimeInstance::Item { flags, .. } => *flags,
        }
    }

    /// Check if the instance is visible (INST_VISIBLE flag set).
    pub fn is_visible(&self) -> bool {
        self.flags() & INST_VISIBLE != 0
    }

    /// Check if the instance participates in the BSP tree (INST_USE_TREE flag set).
    pub fn uses_tree(&self) -> bool {
        self.flags() & INST_USE_TREE != 0
    }

    /// Returns true if this is an Item variant.
    pub fn is_item(&self) -> bool {
        matches!(self, RuntimeInstance::Item { .. })
    }
}
