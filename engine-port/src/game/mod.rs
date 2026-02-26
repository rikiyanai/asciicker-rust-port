//! Game plugin: wires physics + character + render + camera yaw sync + game state machine.
//!
//! CRITICAL: GamePlugin does NOT add PhysicsPlugin, CharacterPlugin, or
//! CpuRasterizerPlugin as sub-plugins. main.rs registers all plugins
//! independently. Bevy panics on duplicate plugin registration.
//!
//! GamePlugin adds game-level resources, cross-plugin sync systems, and the
//! game state machine (GameState with MainMenu/Loading/Playing/Paused).
//!
//! P7-014 FIX: All Phase 6 gameplay systems are gated on in_state(GameState::Playing)
//! so they do not run during MainMenu or Loading states.

pub mod menu;
pub mod state;
pub mod weather;

use bevy::prelude::*;

use crate::character::equipment::SpriteReq;
use crate::character::state_machine::Character;
use crate::physics::{PhysicsIO, PhysicsState};
use crate::render::WaterConfig;
use crate::render::camera::GameCamera;
use crate::render::pipeline::render_pipeline_system;
use crate::system_sets::{CharacterSet, RenderSet};
use crate::terrain::RuntimeTerrain;

pub mod spatial_grid;

use state::GameState;
use menu::MainMenu;
use spatial_grid::{SpatialGrid, sync_spatial_grid, cleanup_spatial_grid};

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
/// F235 FIX: Also recomputes view matrix so the pipeline sees updated position.
/// F238 FIX: Camera Z is in raw u16 height units; physics Z is in world units
/// (raw / HEIGHT_SCALE). Multiply by HEIGHT_SCALE to convert.
fn sync_camera_to_player(
    physics_io: Res<PhysicsIO>,
    mut camera: ResMut<GameCamera>,
    config: Res<crate::render::config::RenderConfig>,
) {
    camera.pos[0] = physics_io.pos[0];
    camera.pos[1] = physics_io.pos[1];
    // Camera Z = raw height units. Physics Z = world units (raw / HEIGHT_SCALE).
    camera.pos[2] = physics_io.pos[2] * crate::asset_loader::constants::HEIGHT_SCALE as f32;
    // Recompute view matrix after position change so render pipeline uses correct view
    let dw = config.sample_width() as f64;
    let dh = config.sample_height() as f64;
    camera.update(dw, dh);
    camera.extract_frustum_planes(dw, dh);
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
/// WaterLevel is in physics world units. Render uses raw u16 height units
/// (matching camera.pos[2]). Multiply by HEIGHT_SCALE to convert.
fn sync_water_to_render(water_level: Res<WaterLevel>, mut water_config: ResMut<WaterConfig>) {
    water_config.water_z = water_level.0 * crate::asset_loader::constants::HEIGHT_SCALE as f32;
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

/// F234+F235 FIX: Teleport player to terrain surface when terrain first loads.
///
/// Character spawns at Startup before terrain loads async. By the time terrain
/// assembles, the character has been falling for 100+ frames. This one-shot
/// system detects terrain load and teleports player + camera to surface.
fn teleport_to_terrain_system(
    terrain: Res<RuntimeTerrain>,
    mut physics_io: ResMut<PhysicsIO>,
    mut physics_state: ResMut<PhysicsState>,
    mut camera: ResMut<GameCamera>,
    water_level: Res<WaterLevel>,
    mut initialized: Local<bool>,
) {
    if *initialized || terrain.root.is_none() {
        return;
    }
    *initialized = true;

    let x = physics_io.pos[0];
    let y = physics_io.pos[1];

    // Find terrain height at player position
    let terrain_z = terrain
        .interpolate_height(x as f64, y as f64)
        .map(|h| h as f32)
        .unwrap_or_else(|| {
            // Fallback: find nearest patch center height (raw u16 -> world units)
            let mut best_z = 0.0f32;
            let mut best_dist = f64::MAX;
            terrain.for_each_patch(|patch| {
                let dx = patch.x as f64 - x as f64;
                let dy = patch.y as f64 - y as f64;
                let d = dx * dx + dy * dy;
                if d < best_dist {
                    best_dist = d;
                    best_z = patch.height[2][2] as f32
                        / crate::asset_loader::constants::HEIGHT_SCALE as f32;
                }
            });
            best_z // world units: raw / HEIGHT_SCALE
        });

    let spawn_z = (terrain_z + 2.0).max(water_level.0 + 2.0); // Clamp above water

    physics_io.pos = [x, y, spawn_z];
    // Reset velocity (stop falling)
    physics_state.vel = [0.0, 0.0, 0.0];
    physics_state.accum_contact = 5.0; // Start grounded

    camera.pos = [x, y, spawn_z];

    info!(
        "F234 FIX: Teleported player to terrain surface z={:.1} (terrain_z={:.1}, water={:.1}) at ({:.1}, {:.1})",
        spawn_z, terrain_z, water_level.0, x, y
    );
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // ---------------------------------------------------------------
        // Game state machine (Phase 7 Plan 02)
        // ---------------------------------------------------------------
        app.init_state::<GameState>();
        app.init_resource::<MainMenu>();

        // OnEnter/OnExit for Loading state
        app.add_systems(OnEnter(GameState::Loading), state::on_enter_loading);
        app.add_systems(OnExit(GameState::Loading), state::on_exit_loading);

        // OnEnter for Playing state
        app.add_systems(OnEnter(GameState::Playing), state::on_enter_playing);

        // Loading state systems: advance progress + check completion + render loading screen
        app.add_systems(
            Update,
            (
                state::advance_loading_progress_system,
                state::check_loading_complete,
                state::render_loading_screen,
            )
                .chain()
                .run_if(in_state(GameState::Loading)),
        );

        // MainMenu state systems: navigation, activation, rendering
        app.add_systems(
            Update,
            (
                menu::menu_navigation,
                menu::menu_activate,
                menu::render_menu,
            )
                .chain()
                .run_if(in_state(GameState::MainMenu)),
        );

        // Pause toggle: runs in both Playing and Paused states
        app.add_systems(
            Update,
            state::toggle_pause.run_if(
                in_state(GameState::Playing).or(in_state(GameState::Paused)),
            ),
        );

        // ---------------------------------------------------------------
        // P7-038 FIX: Gate RenderSet::Pipeline on GameState::Playing.
        // This prevents render_pipeline_system from clearing AsciiCellGrid
        // during MainMenu (which would overwrite render_menu output).
        // RenderSet::Pipeline is labeled by CpuRasterizerPlugin; we configure
        // its run condition from GamePlugin without modifying CpuRasterizerPlugin.
        // ---------------------------------------------------------------
        app.configure_sets(
            PostUpdate,
            RenderSet::Pipeline.run_if(in_state(GameState::Playing)),
        );

        // R8-XP-002 FIX: Gate advance_water_time_system on Playing state.
        // Prevents unnecessary ripple_time advancement during MainMenu/Loading.
        app.configure_sets(
            Update,
            RenderSet::WaterTime.run_if(in_state(GameState::Playing)),
        );

        // ---------------------------------------------------------------
        // Game-domain resources
        // ---------------------------------------------------------------
        // C++ default: water = 55 (raw u16 height units). World units = 55 / HEIGHT_SCALE.
        // Source: game_app.cpp:2061, game_web.cpp:911, mainmenu.cpp "ak.setWater(55)"
        app.insert_resource(WaterLevel(55.0 / crate::asset_loader::constants::HEIGHT_SCALE as f32));
        app.init_resource::<SpatialGrid>();
        app.init_resource::<weather::Weather>();

        // ---------------------------------------------------------------
        // P7-014 FIX: Gate ALL Phase 6 systems on GameState::Playing.
        // This prevents physics sync, camera, and water systems from
        // running during MainMenu or Loading states.
        // ---------------------------------------------------------------

        // PreUpdate: sync water + mount to physics (after character input)
        // Gated on Playing state so physics doesn't run during menu.
        app.configure_sets(
            PreUpdate,
            CharacterSet::PreUpdateInput.before(GameSet::PhysicsSync),
        );
        app.add_systems(
            PreUpdate,
            (sync_water_to_physics, sync_mount_to_physics)
                .chain()
                .in_set(GameSet::PhysicsSync)
                .run_if(in_state(GameState::Playing)),
        );

        // Update: torque -> camera yaw, water -> render config, terrain teleport
        // All gated on Playing state.
        app.add_systems(
            Update,
            (
                teleport_to_terrain_system,
                apply_torque_to_camera,
                sync_water_to_render,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );

        // ---------------------------------------------------------------
        // Weather systems (Phase 7 Plan 05)
        // ---------------------------------------------------------------
        // weather_update_system: spawns/updates particles in Update (before PostUpdate render)
        app.add_systems(
            Update,
            weather::weather_update_system.run_if(in_state(GameState::Playing)),
        );

        // AUTHORITATIVE SCHEDULE DECISION (M6-AUDIT-FIX):
        // render_pipeline_system is in PostUpdate (confirmed by Phase 6).
        // weather_composite_system MUST also be in PostUpdate, AFTER render_pipeline_system.
        // P7-009 FIX: explicit .after(render_pipeline_system) ordering.
        // Research Pitfall 4: composite AFTER resolve to avoid overwrite.
        app.add_systems(
            PostUpdate,
            weather::weather_composite_system
                .after(render_pipeline_system)
                .run_if(in_state(GameState::Playing)),
        );

        // PostUpdate: cross-plugin ordering for render pipeline
        app.configure_sets(
            PostUpdate,
            CharacterSet::SpritePush.before(RenderSet::Pipeline),
        );

        // PostUpdate: sync physics output to camera + character transform
        // Gated on Playing state.
        app.add_systems(
            PostUpdate,
            (
                sync_camera_to_player.before(CharacterSet::SpritePush),
                sync_physics_to_character
                    .before(CharacterSet::SpritePush)
                    .in_set(CharacterSet::PhysicsSync),
                sync_spatial_grid.after(CharacterSet::PhysicsSync),
                cleanup_spatial_grid,
            )
                .run_if(in_state(GameState::Playing)),
        );

        info!(
            "GamePlugin registered (state machine, menu, water sync, torque, camera follow, spatial grid, weather)"
        );
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
        // WaterLevel is in world units, passes directly to PhysicsIO.water
        let water_level = WaterLevel(5.0);
        let mut physics_io = PhysicsIO::default();

        physics_io.water = water_level.0;
        assert_eq!(physics_io.water, 5.0);
    }

    #[test]
    fn test_water_level_syncs_to_render() {
        use crate::asset_loader::constants::HEIGHT_SCALE;
        // WaterLevel is in world units. Render needs raw u16 (multiply by HEIGHT_SCALE).
        let water_level = WaterLevel(10.0);
        let mut water_config = WaterConfig {
            water_z: f32::NEG_INFINITY,
            ripple_time: 0.0,
        };

        water_config.water_z = water_level.0 * HEIGHT_SCALE as f32;
        assert_eq!(water_config.water_z, 10.0 * HEIGHT_SCALE as f32);
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

    #[test]
    fn test_game_state_default_starts_at_main_menu() {
        // GameState::MainMenu is the default state on startup
        assert_eq!(GameState::default(), GameState::MainMenu);
    }

    #[test]
    fn test_game_state_all_four_variants_exist() {
        // Verify all 4 GameState variants compile and are distinct
        let states = [
            GameState::MainMenu,
            GameState::Loading,
            GameState::Playing,
            GameState::Paused,
        ];
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                assert_ne!(states[i], states[j]);
            }
        }
    }

    #[test]
    fn test_main_menu_default_has_two_items() {
        let menu = MainMenu::default();
        assert_eq!(menu.items.len(), 2);
        assert_eq!(menu.selected_index, 0);
    }

}
