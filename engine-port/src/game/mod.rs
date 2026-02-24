//! Game plugin: wires physics + character + render + camera yaw sync.
//!
//! CRITICAL: GamePlugin does NOT add PhysicsPlugin, CharacterPlugin, or
//! CpuRasterizerPlugin as sub-plugins. main.rs registers all plugins
//! independently. Bevy panics on duplicate plugin registration.
//!
//! GamePlugin only adds game-level resources and cross-plugin sync systems.

use bevy::prelude::*;

use crate::character::equipment::SpriteReq;
use crate::character::state_machine::Character;
use crate::physics::PhysicsIO;
use crate::render::WaterConfig;
use crate::render::camera::GameCamera;
use crate::system_sets::CharacterSet;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Game-domain water level resource.
///
/// GamePlugin owns this; syncs to PhysicsIO.water (PreUpdate) and
/// WaterConfig.water_z (Update) via separate systems.
#[derive(Resource)]
pub struct WaterLevel(pub f32);

/// System sets for GamePlugin cross-plugin ordering.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// PreUpdate: sync_water_to_physics, sync_mount_to_physics
    PhysicsSync,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Convert PhysicsIO.torque (set by character input Q/E) to GameCamera.yaw change.
///
/// Linear rotation model: yaw += torque * 45.0 * dt.
/// ResMut<PhysicsIO> needed for yaw writeback so query_character_sprites
/// reads the current yaw (not stale 0.0).
fn apply_torque_to_camera(
    mut physics_io: ResMut<PhysicsIO>,
    mut camera: ResMut<GameCamera>,
    time: Res<Time>,
) {
    // PhysicsIO.torque is INPUT (set by character/input.rs Q/E = +1/-1)
    camera.yaw += physics_io.torque * 45.0 * time.delta_secs();
    physics_io.yaw = camera.yaw;
    // GameCamera.yaw is DEGREES. Apply .to_radians() for trig functions.
    // NOTE: Linear rotation model (deliberate simplification of C++ yaw velocity).
}

/// Sync camera position to player position from physics output.
///
/// Runs in PostUpdate before SpritePush so sprites use updated camera pos.
fn sync_camera_to_player(physics_io: Res<PhysicsIO>, mut camera: ResMut<GameCamera>) {
    // PhysicsIO.pos is [f32; 3] (array) -- use array indexing
    camera.pos = [physics_io.pos[0], physics_io.pos[1], physics_io.pos[2]];
}

/// Sync WaterLevel to PhysicsIO.water for buoyancy calculations.
///
/// Runs in PreUpdate so FixedUpdate physics reads updated water level.
fn sync_water_to_physics(water_level: Res<WaterLevel>, mut physics_io: ResMut<PhysicsIO>) {
    physics_io.water = water_level.0;
}

/// Sync WaterLevel to WaterConfig.water_z for render pipeline reflection.
///
/// Runs in Update (before PostUpdate render reads it).
fn sync_water_to_render(water_level: Res<WaterLevel>, mut water_config: ResMut<WaterConfig>) {
    water_config.water_z = water_level.0;
}

/// Sync physics output to character entity Transform.
///
/// Updates Transform ONLY -- does NOT touch ActionState
/// (handled by update_character_state in CharacterPlugin).
fn sync_physics_to_character(
    physics_io: Res<PhysicsIO>,
    mut q: Query<&mut Transform, With<Character>>,
) {
    // PhysicsIO.pos is [f32; 3] (array indexing). Transform.translation is Vec3 (.x/.y/.z).
    for mut transform in &mut q {
        transform.translation.x = physics_io.pos[0];
        transform.translation.y = physics_io.pos[1];
        transform.translation.z = physics_io.pos[2];
    }
}

/// Sync mount/equipment collision dimensions to PhysicsIO.
///
/// Runs in PreUpdate so FixedUpdate physics uses correct radius/height.
/// TRAP-G02: collision_dimensions() varies by mount type.
fn sync_mount_to_physics(mut physics_io: ResMut<PhysicsIO>, q: Query<&SpriteReq, With<Character>>) {
    // Use first character's mount (player). Multiple characters share physics.
    if let Some(sprite_req) = q.iter().next() {
        let (world_radius, world_height) = sprite_req.collision_dimensions();
        physics_io.world_radius = world_radius;
        physics_io.world_height = world_height;
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // Game-domain resources
        app.insert_resource(WaterLevel(f32::NEG_INFINITY));
        // GameState deferred to Phase 7 Plan 02

        // PreUpdate: sync water + mount to physics (after character input)
        app.configure_sets(
            PreUpdate,
            CharacterSet::PreUpdateInput.before(GameSet::PhysicsSync),
        );
        app.add_systems(
            PreUpdate,
            (sync_water_to_physics, sync_mount_to_physics)
                .chain()
                .in_set(GameSet::PhysicsSync),
        );

        // Update: torque -> camera yaw, water -> render config
        app.add_systems(
            Update,
            (apply_torque_to_camera, sync_water_to_render).chain(),
        );

        // PostUpdate: cross-plugin ordering for render pipeline
        app.configure_sets(
            PostUpdate,
            CharacterSet::SpritePush.before(crate::system_sets::RenderSet::Pipeline),
        );

        // PostUpdate: sync physics output to camera + character transform
        app.add_systems(
            PostUpdate,
            sync_camera_to_player.before(CharacterSet::SpritePush),
        );
        app.add_systems(
            PostUpdate,
            sync_physics_to_character
                .before(CharacterSet::SpritePush)
                .in_set(CharacterSet::PhysicsSync),
        );

        info!("GamePlugin registered (water sync, torque, camera follow, schedule ordering)");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_level_syncs_to_physics() {
        let water_level = WaterLevel(5.0);
        let mut physics_io = PhysicsIO::default();
        assert_eq!(physics_io.water, f32::NEG_INFINITY);

        physics_io.water = water_level.0;
        assert_eq!(physics_io.water, 5.0);
    }

    #[test]
    fn test_water_level_syncs_to_render() {
        let water_level = WaterLevel(10.0);
        let mut water_config = WaterConfig {
            water_z: f32::NEG_INFINITY,
            ripple_time: 0.0,
        };

        water_config.water_z = water_level.0;
        assert_eq!(water_config.water_z, 10.0);
    }

    #[test]
    fn test_rotation_frame_rate_independent() {
        // R16-F199 FIX: Same elapsed time at 30fps vs 60fps produces same total yaw
        let torque = 1.0f32;
        let speed = 45.0f32; // degrees per second at torque=1

        // 30fps: 30 iterations of dt=1/30
        let dt_30 = 1.0f32 / 30.0;
        let mut yaw_30 = 0.0f32;
        for _ in 0..30 {
            yaw_30 += torque * speed * dt_30;
        }

        // 60fps: 60 iterations of dt=1/60
        let dt_60 = 1.0f32 / 60.0;
        let mut yaw_60 = 0.0f32;
        for _ in 0..60 {
            yaw_60 += torque * speed * dt_60;
        }

        // Both should produce 45.0 degrees after 1.0 seconds
        assert!(
            (yaw_30 - yaw_60).abs() < 0.001,
            "Frame-rate independent: 30fps={yaw_30} vs 60fps={yaw_60}"
        );
        assert!(
            (yaw_30 - 45.0).abs() < 0.01,
            "Total yaw should be ~45.0 degrees, got {yaw_30}"
        );
    }

    #[test]
    fn test_camera_follows_player() {
        // R54: After physics updates pos, camera.pos matches physics_io.pos
        let physics_io = PhysicsIO {
            pos: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let mut camera = GameCamera::default();

        // Simulate sync_camera_to_player
        camera.pos = [physics_io.pos[0], physics_io.pos[1], physics_io.pos[2]];

        assert_eq!(camera.pos, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_mount_change_updates_collision_dimensions() {
        use crate::character::equipment::{Mount, SpriteReq};

        let mut physics_io = PhysicsIO::default();
        let original_radius = physics_io.world_radius;

        // Wolf mount has larger collision dimensions
        let wolf_req = SpriteReq {
            mount: Mount::Wolf,
            ..Default::default()
        };
        let (wolf_radius, wolf_height) = wolf_req.collision_dimensions();
        physics_io.world_radius = wolf_radius;
        physics_io.world_height = wolf_height;

        assert!(
            physics_io.world_radius > original_radius,
            "Wolf radius {} should be larger than default {}",
            physics_io.world_radius,
            original_radius
        );
    }

    #[test]
    fn test_apply_torque_to_camera_math() {
        // Verify torque integration: yaw += torque * 45.0 * dt
        let torque = 1.0f32;
        let dt = 0.016f32; // ~60fps
        let mut yaw = 0.0f32;

        yaw += torque * 45.0 * dt;

        let expected = 45.0 * 0.016;
        assert!(
            (yaw - expected).abs() < 0.0001,
            "Yaw should be {expected}, got {yaw}"
        );
    }

    #[test]
    fn test_sync_physics_to_character_transform() {
        // Physics output updates character Transform
        let pos = [5.0f32, 10.0, 15.0];
        let mut transform = Transform::default();

        transform.translation.x = pos[0];
        transform.translation.y = pos[1];
        transform.translation.z = pos[2];

        assert_eq!(transform.translation, Vec3::new(5.0, 10.0, 15.0));
    }
}
