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
        // cycle_weather_debug_system: F5 cycles weather states for testing (gap closure 07-06)
        app.add_systems(
            Update,
            (
                weather::cycle_weather_debug_system,
                weather::weather_update_system,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
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
