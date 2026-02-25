//! Client-side networking systems.
//!
//! Handles sending local player pose updates and applying remote player poses.

use bevy::prelude::*;

use crate::character::Character;
use crate::network::protocol::PoseUpdate;
use crate::network::server::NetworkPlayer;

/// Resource identifying the local player entity.
#[derive(Resource, Debug)]
pub struct LocalPlayer {
    pub entity: Entity,
}

/// Send local player's Transform as a PoseUpdate component update.
///
/// P7-108 FIX: System returns () (no ? operator). Uses explicit matching.
pub fn send_local_pose(
    mut query: Query<(&Transform, &mut PoseUpdate), (With<Character>, Without<NetworkPlayer>)>,
) {
    for (transform, mut pose) in &mut query {
        // R19-006 FIX: Convert Vec3 -> [f32; 3]
        pose.pos = [
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ];
        // Convert rotation to yaw angle (radians around Z axis)
        let (_axis, angle) = transform.rotation.to_axis_angle();
        pose.dir = angle;
    }
}

/// Apply remote players' PoseUpdate to their Transform.
///
/// Runs on entities that have both PoseUpdate and Transform (network players).
/// P7-108 FIX: System returns () (no ? operator).
/// R19-006 FIX: Explicit [f32;3] -> Vec3 conversion.
pub fn apply_remote_poses(
    mut query: Query<(&PoseUpdate, &mut Transform), With<NetworkPlayer>>,
) {
    for (pose, mut transform) in &mut query {
        // R19-006 FIX: Convert [f32; 3] -> Vec3
        transform.translation = Vec3::new(pose.pos[0], pose.pos[1], pose.pos[2]);
        // Convert direction to rotation around Z axis
        transform.rotation = Quat::from_rotation_z(pose.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_player_resource() {
        let lp = LocalPlayer {
            entity: Entity::PLACEHOLDER,
        };
        assert_eq!(lp.entity, Entity::PLACEHOLDER);
    }
}
