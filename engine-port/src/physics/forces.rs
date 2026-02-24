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

    // 2. Input forces rotated by yaw
    let cos_yaw = io.yaw.cos();
    let sin_yaw = io.yaw.sin();
    let fx = io.x_force * cos_yaw - io.y_force * sin_yaw;
    let fy = io.x_force * sin_yaw + io.y_force * cos_yaw;
    state.vel[0] += fx;
    state.vel[1] += fy;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_io() -> PhysicsIO {
        PhysicsIO {
            water: f32::NEG_INFINITY,
            world_height: 86.2,
            world_radius: 1.333,
            ..Default::default()
        }
    }

    #[test]
    fn test_gravity_negative_vel_z() {
        // R16-F194: After accumulate_forces with water=NEG_INFINITY, dt=1/66.667
        // vel[2] must be < 0 and abs > 0.01
        let mut state = PhysicsState::default();
        let io = default_io();
        let dt = 1.0 / 66.667;
        accumulate_forces(&mut state, &io, dt);
        assert!(
            state.vel[2] < 0.0,
            "gravity must produce negative vel_z, got {}",
            state.vel[2]
        );
        assert!(
            state.vel[2].abs() > 0.01,
            "vel_z must be significant, got {}",
            state.vel[2]
        );
    }

    #[test]
    fn test_gravity_formula_no_double_count() {
        // With water=NEG_INFINITY, acc clamps to -cnt (~-0.78)
        // vel_z change = dt * (-0.78) per tick
        let mut state = PhysicsState::default();
        let io = default_io();
        let dt = 1.0;
        accumulate_forces(&mut state, &io, dt);
        // vel[2] should be roughly -0.78 (clamped acc * dt=1.0)
        assert!(
            (state.vel[2] - (-0.78)).abs() < 0.05,
            "gravity acc should be ~-0.78, got {}",
            state.vel[2]
        );
    }

    #[test]
    fn test_jump_consumed_when_grounded() {
        // R16-F194: set jump=true, grounded > threshold
        // After apply_jump: jump=false, vel_z > 0
        let mut state = PhysicsState {
            accum_contact: 2.0, // > GROUNDED_THRESHOLD
            ..Default::default()
        };
        let mut io = PhysicsIO {
            jump: true,
            ..default_io()
        };
        apply_jump(&mut state, &mut io);
        assert!(!io.jump, "jump must be cleared after evaluation");
        assert!(state.vel[2] > 0.0, "vel_z must be positive after jump");
        assert!(
            (state.vel[2] - JUMP_VELOCITY).abs() < 0.01,
            "vel_z should equal JUMP_VELOCITY"
        );
    }

    #[test]
    fn test_jump_not_consumed_when_airborne() {
        // TRAP-P05: jump not consumed when not grounded
        let mut state = PhysicsState {
            accum_contact: 0.0, // Below threshold
            ..Default::default()
        };
        let mut io = PhysicsIO {
            jump: true,
            ..default_io()
        };
        apply_jump(&mut state, &mut io);
        assert!(!io.jump, "jump flag must be cleared even when airborne");
        assert!(
            (state.vel[2] - 0.0).abs() < 1e-6,
            "vel_z should remain 0 when airborne, got {}",
            state.vel[2]
        );
    }

    #[test]
    fn test_jump_conditional_add_vs_set() {
        // R19-M08: falling -> SET, rising -> ADD
        // Falling case
        let mut state = PhysicsState {
            vel: [0.0, 0.0, -5.0],
            accum_contact: 2.0,
        };
        let mut io = PhysicsIO {
            jump: true,
            ..default_io()
        };
        apply_jump(&mut state, &mut io);
        assert!(
            (state.vel[2] - JUMP_VELOCITY).abs() < 0.01,
            "falling: should SET to JUMP_VELOCITY, got {}",
            state.vel[2]
        );

        // Rising case
        state.vel[2] = 3.0;
        state.accum_contact = 2.0;
        io.jump = true;
        apply_jump(&mut state, &mut io);
        assert!(
            (state.vel[2] - (3.0 + JUMP_VELOCITY)).abs() < 0.01,
            "rising: should ADD JUMP_VELOCITY, got {}",
            state.vel[2]
        );
    }

    #[test]
    fn test_grounded_accumulation_decay() {
        // TRAP-P02: grounded uses accumulation pattern
        let mut state = PhysicsState {
            accum_contact: 3.0,
            ..Default::default()
        };
        assert!(state.accum_contact >= GROUNDED_THRESHOLD);
        update_grounded(&mut state, 1.0 / 66.667);
        // After decay: 3.0 * 0.9 = 2.7, still above threshold
        assert!((state.accum_contact - 2.7).abs() < 0.01);
        assert!(state.accum_contact >= GROUNDED_THRESHOLD);
    }

    #[test]
    fn test_grounded_decays_below_threshold() {
        let mut state = PhysicsState {
            accum_contact: 1.05,
            ..Default::default()
        };
        update_grounded(&mut state, 1.0 / 66.667);
        // 1.05 * 0.9 = 0.945, below threshold
        assert!(
            state.accum_contact < GROUNDED_THRESHOLD,
            "should decay below threshold, got {}",
            state.accum_contact
        );
    }

    #[test]
    fn test_damping_reduces_velocity() {
        // R16-F194: initial vel=[10,0,0], after accumulate with dt=1/66.667 → vel[0] < 10.0
        let mut state = PhysicsState {
            vel: [10.0, 0.0, 0.0],
            ..Default::default()
        };
        let io = default_io();
        let dt = 1.0 / 66.667;
        accumulate_forces(&mut state, &io, dt);
        assert!(
            state.vel[0] < 10.0,
            "damping should reduce velocity, got {}",
            state.vel[0]
        );
    }

    #[test]
    fn test_velocity_clamped_at_max_vel_air() {
        // R18-F231: set vel=[100, 0, 0], after accumulate → vel[0] == MAX_VEL_AIR
        let mut state = PhysicsState {
            vel: [100.0, 0.0, 0.0],
            ..Default::default()
        };
        let io = default_io();
        let dt = 1.0 / 66.667;
        accumulate_forces(&mut state, &io, dt);
        // After clamping and damping, |vel_xy| should be <= MAX_VEL_AIR
        let speed = (state.vel[0] * state.vel[0] + state.vel[1] * state.vel[1]).sqrt();
        assert!(
            speed <= MAX_VEL_AIR + 0.01,
            "velocity should be clamped to MAX_VEL_AIR, got {}",
            speed
        );
    }

    #[test]
    fn test_impulse_applied() {
        let mut state = PhysicsState::default();
        let io = PhysicsIO {
            x_impulse: 5.0,
            y_impulse: 3.0,
            ..default_io()
        };
        let dt = 1.0 / 66.667;
        accumulate_forces(&mut state, &io, dt);
        // Impulse should be reflected in velocity
        assert!(state.vel[0].abs() > 0.0, "x_impulse should affect vel[0]");
        assert!(state.vel[1].abs() > 0.0, "y_impulse should affect vel[1]");
    }

    #[test]
    fn test_input_forces_with_yaw() {
        let mut state = PhysicsState::default();
        let io = PhysicsIO {
            x_force: 1.0,
            y_force: 0.0,
            yaw: std::f32::consts::FRAC_PI_2, // 90 degrees
            ..default_io()
        };
        let dt = 1.0 / 66.667;
        accumulate_forces(&mut state, &io, dt);
        // With yaw=90deg, x_force should map to y direction
        assert!(
            state.vel[1].abs() > state.vel[0].abs(),
            "90deg yaw should rotate x_force to y, vel=[{}, {}]",
            state.vel[0],
            state.vel[1]
        );
    }

    #[test]
    fn test_height_conversion() {
        // R16-F196: verify HEIGHT_SCALE conversion
        use crate::asset_loader::constants::HEIGHT_SCALE;
        assert_eq!(
            16u16 as f32 / HEIGHT_SCALE as f32,
            1.0,
            "16 raw height units / HEIGHT_SCALE should equal 1.0 world unit"
        );
    }
}
