//! Player input accumulation system.
//!
//! Maps keyboard input (WASD/arrows/space/shift/Q/E/F) to PhysicsIO force
//! fields. Q/E goes through PhysicsIO.torque exclusively (not camera_input_system).
//!
//! Runs in PreUpdate so FixedUpdate physics can consume input in the same frame.

use bevy::prelude::*;

use crate::physics::PhysicsIO;
use crate::render::camera::GameCamera;

use super::equipment::{Shield, SpriteReq};
use super::state_machine::{ActionState, Character};

/// Accumulate player input from keyboard into PhysicsIO.
///
/// System signature reads keyboard, camera (for yaw-relative WASD), and
/// writes to PhysicsIO. Also queries the player entity for block input guard.
///
/// # Input Mapping
/// - WASD/Arrows: camera-relative movement forces
/// - Shift: half speed (multiply forces by 0.5)
/// - Q/E: yaw torque (through PhysicsIO.torque, NOT camera.yaw directly)
/// - Space (just_pressed): jump flag
/// - F (just_pressed): block (requires shield)
///
/// # R19-M06 NOTE
/// C++ applies yaw damping TWICE per physics step (intentional double-damping).
/// Plan 06-03 `apply_torque_to_camera` must replicate this.
///
/// # TODO(Phase 7): Add mouse drag for absolute yaw
pub fn accumulate_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    camera: Res<GameCamera>,
    mut physics_io: ResMut<PhysicsIO>,
    mut query: Query<(&mut ActionState, &SpriteReq), With<Character>>,
) {
    // Step 1: Reset input fields (prevents stale values when keys released)
    physics_io.x_force = 0.0;
    physics_io.y_force = 0.0;
    physics_io.torque = 0.0;
    // jump uses just_pressed (one-shot) -- cleared by physics after consumption
    // Do NOT write physics-owned fields: pos, vel_z, grounded

    // Step 2: WASD/Arrows raw input
    let mut raw_x = 0.0f32;
    let mut raw_y = 0.0f32;

    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        raw_x += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        raw_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        raw_y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        raw_y -= 1.0;
    }

    // Normalize diagonal
    let len = (raw_x * raw_x + raw_y * raw_y).sqrt();
    if len > 1.0 {
        raw_x /= len;
        raw_y /= len;
    }

    // Step 3: Camera-relative rotation (GameCamera.yaw is in DEGREES)
    let yaw_rad = camera.yaw.to_radians();
    let cos_yaw = yaw_rad.cos();
    let sin_yaw = yaw_rad.sin();
    physics_io.x_force = raw_x * cos_yaw - raw_y * sin_yaw;
    physics_io.y_force = raw_x * sin_yaw + raw_y * cos_yaw;

    // Step 4: Modifiers
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        physics_io.x_force *= 0.5;
        physics_io.y_force *= 0.5;
    }

    // Q/E -> torque (CRITICAL: goes through PhysicsIO.torque exclusively)
    if keyboard.pressed(KeyCode::KeyQ) {
        physics_io.torque += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        physics_io.torque -= 1.0;
    }

    // Space -> jump (just_pressed = one-shot)
    if keyboard.just_pressed(KeyCode::Space) {
        physics_io.jump = true;
    }

    // Block: F key when shield equipped (checked via player query)
    // R16-F197: Equipment guard rejects input if no shield
    if keyboard.just_pressed(KeyCode::KeyF) {
        for (mut action_state, sprite_req) in &mut query {
            if sprite_req.shield != Shield::None {
                action_state.try_transition(ActionState::Block);
            }
        }
    }

    // Block release: stop blocking when F is released
    if keyboard.just_released(KeyCode::KeyF) {
        for (mut action_state, _sprite_req) in &mut query {
            if *action_state == ActionState::Block {
                action_state.try_transition(ActionState::None);
            }
        }
    }
}
