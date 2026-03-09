//! Force accumulation, jump handling, and grounded detection.
//!
//! Port of C++ physics.cpp force accumulation logic. Uses the unified
//! gravity/buoyancy formula (no separate 9.8 constant) and the
//! accumulation-based grounded detection pattern.

use super::constants::*;
use super::{PhysicsIO, PhysicsState};

/// Accumulate forces on the physics state.
///
/// `io` is IMMUTABLE. Reads only `x_force`, `y_force` (rotated by `io.yaw`,
/// scaled by `XY_SPEED`), `water`, `pos`, `world_height`, `x_impulse`,
/// `y_impulse`. Does NOT read `torque` (consumed by camera-side
/// `apply_torque_to_camera`).
///
/// # Forces applied:
/// 1. Unified gravity/buoyancy (TRAP-P04: static cnt=0.78 for MVP)
/// 2. Input forces (rotated by yaw, scaled by XY_SPEED)
/// 3. Impulse (add then drain by IMPULSE_DRAIN)
/// 4. Velocity clamping (MAX_VEL_AIR / MAX_VEL_WATER)
/// 5. Damping (VEL_DAMPING.powf(dt))
pub fn accumulate_forces(state: &mut PhysicsState, io: &PhysicsIO, dt: f32) {
    // 1. Unified gravity/buoyancy formula (C++ physics.cpp:1488-1539)
    // TRAP-P04: static cnt=0.78 for MVP; wave modulation deferred
    let h = io.world_height;
    let cnt: f32 = 0.78;
    let acc = (io.water - (io.pos[2] + cnt * h)) / (2.0 * cnt * h);
    let acc = acc.clamp(-cnt, 1.0 - cnt);
    state.vel[2] += dt * acc;

    // 2. Input forces (already camera-relative from accumulate_player_input)
    // F233 FIX: input.rs already rotates by camera.yaw -- do NOT rotate again here.
    state.vel[0] += io.x_force;
    state.vel[1] += io.y_force;

    // 3. Impulse: add then drain
    state.vel[0] += io.x_impulse;
    state.vel[1] += io.y_impulse;

    // 4. Velocity clamping
    let max_vel = if io.water > io.pos[2] {
        MAX_VEL_WATER
    } else {
        MAX_VEL_AIR
    };
    let xy_speed_sq = state.vel[0] * state.vel[0] + state.vel[1] * state.vel[1];
    if xy_speed_sq > max_vel * max_vel {
        let scale = max_vel / xy_speed_sq.sqrt();
        state.vel[0] *= scale;
        state.vel[1] *= scale;
    }
    state.vel[2] = state.vel[2].clamp(-max_vel, max_vel);

    // 5. Damping
    let damp = VEL_DAMPING.powf(dt);
    state.vel[0] *= damp;
    state.vel[1] *= damp;
}

/// Apply jump if grounded.
///
/// TRAP-P05: Jump consumed ONLY when grounded.
/// R19-M08: Conditional add-vs-set:
///   - If vel_z < 0 (falling): SET vel_z = JUMP_VELOCITY
///   - Else (rising): ADD JUMP_VELOCITY to vel_z
///
/// Always clears `io.jump = false` after evaluation (prevents multi-substep
/// re-reads with MAX_SUBSTEPS=10).
pub fn apply_jump(state: &mut PhysicsState, io: &mut PhysicsIO) {
    if io.jump {
        if state.accum_contact >= GROUNDED_THRESHOLD {
            // R19-M08: conditional add-vs-set
            if state.vel[2] < 0.0 {
                state.vel[2] = JUMP_VELOCITY;
            } else {
                state.vel[2] += JUMP_VELOCITY;
            }
        }
        // Always clear jump flag after evaluation
        io.jump = false;
    }
}

/// Update grounded state: decay-only form called once per physics tick
/// after all substeps.
///
/// TRAP-P02: accumulation-based grounded detection.
/// R19-M02: Accumulation of per_step_max_normal_z happens in
/// `collision_sweep_system`, not here. This function only handles decay.
///
/// - Decay: `accum_contact *= GROUNDED_DECAY`
/// - Grounded: `accum_contact >= GROUNDED_THRESHOLD`
pub fn update_grounded(state: &mut PhysicsState, _dt: f32) {
    state.accum_contact *= GROUNDED_DECAY;
}
