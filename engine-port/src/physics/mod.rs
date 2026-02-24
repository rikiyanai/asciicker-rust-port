//! Physics system: sphere collision, force integration, and Bevy FixedUpdate.
//!
//! Port of C++ physics.cpp. Provides sphere-based collision detection,
//! force accumulation (gravity/buoyancy/impulse), grounded detection,
//! and a 66Hz fixed timestep simulation loop.

use bevy::prelude::*;

pub mod collision;
pub mod constants;
pub mod forces;
pub mod geometry;
pub mod soup;

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};
use crate::terrain::RuntimeTerrain;
use crate::world::RuntimeWorld;
use collision::{check_collision, CollisionResult};
use constants::*;
use geometry::{collect_terrain_triangles, collect_world_triangles};
use soup::SoupItem;

// ---------------------------------------------------------------------------
// PhysicsState (internal, not part of I/O boundary)
// ---------------------------------------------------------------------------

/// Internal physics state holding velocity and grounded accumulator.
///
/// Fields are `pub(crate)` for encapsulation. External access via `vel()`.
#[derive(Resource, Default)]
pub struct PhysicsState {
    pub(crate) vel: [f32; 3],
    pub(crate) accum_contact: f32,
}

impl PhysicsState {
    /// Public accessor for velocity (used by benchmarks outside crate).
    pub fn vel(&self) -> [f32; 3] {
        self.vel
    }
}

// ---------------------------------------------------------------------------
// PhysicsIO (definitive field list)
// ---------------------------------------------------------------------------

/// The physics I/O resource bridging game systems and the physics simulation.
///
/// Fields are categorized as INPUT (game -> physics), I/O (bidirectional),
/// or OUTPUT (physics -> game).
#[derive(Resource)]
pub struct PhysicsIO {
    // INPUT (game -> physics): written by input system, read by physics
    pub x_force: f32,
    pub y_force: f32,
    pub z_force: f32,
    pub torque: f32,
    /// Water surface Z coordinate; `f32::NEG_INFINITY` means no water.
    pub water: f32,

    // I/O (bidirectional):
    /// Set by input, cleared by physics after evaluation.
    pub jump: bool,
    pub fly: bool,
    pub x_impulse: f32,
    pub y_impulse: f32,

    // OUTPUT (physics -> game): written by physics, read by game/character
    pub pos: [f32; 3],
    /// Yaw angle. Written by `apply_torque_to_camera` (Update), NOT by physics.
    pub yaw: f32,
    pub player_dir: i32,
    pub player_stp: i32,
    pub dt: f32,
    pub grounded: bool,
    /// Vertical velocity output for character state transitions.
    pub vel_z: f32,
    pub world_radius: f32,
    pub world_height: f32,
}

impl Default for PhysicsIO {
    fn default() -> Self {
        Self {
            x_force: 0.0,
            y_force: 0.0,
            z_force: 0.0,
            torque: 0.0,
            water: f32::NEG_INFINITY,
            jump: false,
            fly: false,
            x_impulse: 0.0,
            y_impulse: 0.0,
            pos: [0.0; 3],
            yaw: 0.0,
            player_dir: 0,
            player_stp: -1,
            dt: 0.0,
            grounded: false,
            vel_z: 0.0,
            world_radius: 1.0, // Safe non-zero default (overridden by plugin init)
            world_height: 1.0, // Safe non-zero default
        }
    }
}

// ---------------------------------------------------------------------------
// PhysicsPlugin
// ---------------------------------------------------------------------------

/// Bevy plugin for the physics simulation.
///
/// Registers PhysicsIO and PhysicsState resources, sets up FixedUpdate
/// at 66Hz, and chains physics systems.
///
/// CRITICAL: Does NOT add CharacterPlugin or GamePlugin as sub-plugins.
/// main.rs registers all three independently; duplicates cause Bevy panic.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Formula-derived defaults (Human character)
        let height_cells: f32 = 7.0;
        let radius_cells: f32 = 2.0;
        let world_height = height_cells * 2.0 / 3.0
            / (30.0_f32.to_radians().cos())
            * HEIGHT_SCALE as f32;
        let world_radius =
            radius_cells / (3.0 * HEIGHT_CELLS as f32) * VISUAL_CELLS as f32;

        app.insert_resource(PhysicsIO {
            world_radius,
            world_height,
            water: f32::NEG_INFINITY,
            ..Default::default()
        });
        app.insert_resource(PhysicsState::default());
        app.insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ));

        app.add_systems(
            FixedUpdate,
            (
                accumulate_forces_system,
                collision_sweep_system,
                update_output_system,
            )
                .chain(),
        );

        info!("PhysicsPlugin registered (FixedUpdate at {PHYSICS_HZ}Hz)");
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn accumulate_forces_system(
    mut state: ResMut<PhysicsState>,
    mut io: ResMut<PhysicsIO>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    io.dt = dt;
    forces::accumulate_forces(&mut state, &io, dt);
    forces::apply_jump(&mut state, &mut io);
}

/// Collision sweep system: collects geometry soup and resolves collisions.
///
/// R19-C01: Uses bare `Res<RuntimeTerrain>` and `Res<RuntimeWorld>` (not Option).
fn collision_sweep_system(
    mut state: ResMut<PhysicsState>,
    mut io: ResMut<PhysicsIO>,
    terrain: Res<RuntimeTerrain>,
    world: Res<RuntimeWorld>,
) {
    let dt = io.dt;
    if dt <= 0.0 || io.world_radius <= 0.0 || io.world_height <= 0.0 {
        return;
    }

    let mul_xy = 1.0 / io.world_radius;
    let mul_z = 2.0 / io.world_height;

    // R19-PERF: Use world_radius (entity radius) for search, not world_height
    let search_radius = io.world_radius * 2.0;

    let old_pos = io.pos;

    // Per-step max contact normal Z for grounded detection (R19-M02)
    let mut per_step_max_normal_z: f32 = 0.0;

    // Collision sweep with up to MAX_SUBSTEPS iterations
    let mut remaining = 1.0f32; // fraction of velocity still to consume

    for _substep in 0..MAX_SUBSTEPS {
        if remaining <= 0.0 {
            break;
        }

        let step_vel = [
            state.vel[0] * XY_SPEED * dt * remaining,
            state.vel[1] * XY_SPEED * dt * remaining,
            state.vel[2] * dt * remaining,
        ];

        // Collect collision soup
        let mut soup: Vec<SoupItem> = Vec::new();
        let center = io.pos;

        // Only collect if terrain/world have data loaded
        if terrain.root.is_some() {
            collect_terrain_triangles(&terrain, &center, search_radius, mul_xy, mul_z, &mut soup);
        }
        if !world.instances.is_empty() {
            collect_world_triangles(&world, &center, search_radius, mul_xy, mul_z, &mut soup);
        }

        // Transform velocity to sphere space
        let ss_vel = [
            step_vel[0] * mul_xy,
            step_vel[1] * mul_xy,
            step_vel[2] * mul_z,
        ];

        // Sphere position in sphere space is always origin (geometry is
        // relative to sphere center)
        let ss_pos = [0.0f32; 3];

        // Find earliest collision
        let mut earliest_toi = 2.0f32;
        let mut _earliest_contact = [0.0f32; 3];
        let mut earliest_nrm = [0.0f32; 4];

        for item in &soup {
            match check_collision(&item.tri, &item.nrm, &ss_pos, &ss_vel) {
                CollisionResult::Hit { toi, contact } => {
                    if toi < earliest_toi {
                        earliest_toi = toi;
                        _earliest_contact = contact;
                        earliest_nrm = item.nrm;
                    }
                }
                CollisionResult::Miss => {}
            }
        }

        if earliest_toi <= 1.0 {
            // Advance position by toi fraction of step velocity
            let advance = (earliest_toi - SAFE_DISTANCE).max(0.0);
            io.pos[0] += step_vel[0] * advance;
            io.pos[1] += step_vel[1] * advance;
            io.pos[2] += step_vel[2] * advance;

            // Compute slide normal in world space from sphere-space normal
            let slide_nrm = [
                earliest_nrm[0] * mul_xy,
                earliest_nrm[1] * mul_xy,
                earliest_nrm[2] * mul_z,
            ];
            let nrm_len = (slide_nrm[0].powi(2) + slide_nrm[1].powi(2) + slide_nrm[2].powi(2)).sqrt();
            let slide_normal = if nrm_len > 1e-9 {
                [slide_nrm[0] / nrm_len, slide_nrm[1] / nrm_len, slide_nrm[2] / nrm_len]
            } else {
                [0.0, 0.0, 1.0]
            };

            // Slide: remove velocity component along collision normal
            let v_dot_n = state.vel[0] * XY_SPEED * slide_normal[0]
                + state.vel[1] * XY_SPEED * slide_normal[1]
                + state.vel[2] * slide_normal[2];
            state.vel[0] -= v_dot_n * slide_normal[0] / XY_SPEED;
            state.vel[1] -= v_dot_n * slide_normal[1] / XY_SPEED;
            state.vel[2] -= v_dot_n * slide_normal[2];

            // R19-M02: MAX (not SUM) for contact normal Z within substep
            per_step_max_normal_z = per_step_max_normal_z.max(slide_normal[2]);

            // TODO: R19-M09: auto-jump on wall collision (collision_time < 0.2
            // && slide_normal[2] < 0.8 => io.jump = true for step-climbing)

            remaining *= 1.0 - earliest_toi;
        } else {
            // No collision: advance full remaining step
            io.pos[0] += step_vel[0];
            io.pos[1] += step_vel[1];
            io.pos[2] += step_vel[2];
            break;
        }
    }

    // R19-M07: Recompute velocity from actual position delta
    if dt > 0.0 {
        let xy_dt = XY_SPEED * dt;
        if xy_dt.abs() > 1e-12 {
            state.vel[0] = (io.pos[0] - old_pos[0]) / xy_dt;
            state.vel[1] = (io.pos[1] - old_pos[1]) / xy_dt;
        }
        state.vel[2] = (io.pos[2] - old_pos[2]) / dt;
    }

    // R19-M02: Accumulate per-step max AFTER substep loop (once per timestep)
    state.accum_contact += per_step_max_normal_z.max(0.0);
    state.accum_contact = state.accum_contact.min(GROUNDED_MAX_ACCUM);

    // Update grounded state (decay)
    forces::update_grounded(&mut state, dt);
}

fn update_output_system(state: Res<PhysicsState>, mut io: ResMut<PhysicsIO>) {
    // vel_z output for character state transitions
    io.vel_z = state.vel[2];

    // player_stp: step animation counter from XY velocity magnitude
    let vx = state.vel[0] * XY_SPEED;
    let vy = state.vel[1] * XY_SPEED;
    let xy_mag = (vx * vx + vy * vy).sqrt();

    if xy_mag > XY_THRESH {
        let step_inc = (xy_mag * 64.0) as i32;
        io.player_stp = (io.player_stp.wrapping_add(step_inc)) & STEP_MASK;
    } else {
        io.player_stp = -1; // Idle
    }

    // Do NOT set io.yaw (camera-side only, written by apply_torque_to_camera)

    // Grounded output
    io.grounded = state.accum_contact >= GROUNDED_THRESHOLD;
}
