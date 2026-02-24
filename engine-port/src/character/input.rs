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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::AnimationState;

    /// Helper: create a minimal Bevy App with input resources and accumulate_player_input system.
    fn make_input_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<GameCamera>();
        app.insert_resource(PhysicsIO::default());
        app.add_systems(Update, accumulate_player_input);
        app
    }

    #[test]
    fn test_w_sets_y_force() {
        let mut app = make_input_app();
        // yaw=0, W -> raw_y=1.0, cos(0)=1, sin(0)=0 -> y_force = raw_x*sin + raw_y*cos = 1.0
        {
            let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
            cam.yaw = 0.0;
        }
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyW);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(
            (io.y_force - 1.0).abs() < 0.01,
            "W at yaw=0 -> y_force=1.0, got {}",
            io.y_force
        );
        assert!(
            io.x_force.abs() < 0.01,
            "W at yaw=0 -> x_force=0, got {}",
            io.x_force
        );
    }

    #[test]
    fn test_diagonal_normalized() {
        let mut app = make_input_app();
        {
            let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
            cam.yaw = 0.0;
        }
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyW);
            input.press(KeyCode::KeyD);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        let mag = (io.x_force * io.x_force + io.y_force * io.y_force).sqrt();
        assert!(
            (mag - 1.0).abs() < 0.01,
            "Diagonal should be normalized to ~1.0, got {mag}"
        );
    }

    #[test]
    fn test_shift_halves_force() {
        let mut app = make_input_app();
        {
            let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
            cam.yaw = 0.0;
        }
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyW);
            input.press(KeyCode::ShiftLeft);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(
            (io.y_force - 0.5).abs() < 0.01,
            "Shift+W -> y_force=0.5, got {}",
            io.y_force
        );
    }

    #[test]
    fn test_space_sets_jump() {
        let mut app = make_input_app();
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::Space);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(io.jump, "Space should set jump=true");
    }

    #[test]
    fn test_q_sets_positive_torque() {
        let mut app = make_input_app();
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyQ);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(
            (io.torque - 1.0).abs() < 0.01,
            "Q -> torque=1.0, got {}",
            io.torque
        );
    }

    #[test]
    fn test_e_sets_negative_torque() {
        let mut app = make_input_app();
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyE);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(
            (io.torque - (-1.0)).abs() < 0.01,
            "E -> torque=-1.0, got {}",
            io.torque
        );
    }

    #[test]
    fn test_wasd_camera_relative() {
        let mut app = make_input_app();
        // yaw=90: W->raw_y=1.0, cos(90)≈0, sin(90)≈1
        // x_force = raw_x*cos - raw_y*sin = 0*0 - 1*1 = -1.0
        // y_force = raw_x*sin + raw_y*cos = 0*1 + 1*0 = 0.0
        {
            let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
            cam.yaw = 90.0;
        }
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyW);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        // R17-F220: yaw=90, cos≈0, sin≈1, W=raw_y=1.0 -> x_force = -1.0
        assert!(
            (io.x_force - (-1.0)).abs() < 0.01,
            "yaw=90, W -> x_force=-1.0, got {}",
            io.x_force
        );
        assert!(
            io.y_force.abs() < 0.01,
            "yaw=90, W -> y_force≈0, got {}",
            io.y_force
        );
    }

    #[test]
    fn test_block_input_requires_shield() {
        let mut app = make_input_app();
        // Spawn character with shield
        app.world_mut().spawn((
            Character,
            SpriteReq {
                shield: Shield::RegularShield,
                ..Default::default()
            },
            ActionState::None,
            AnimationState::default(),
            Transform::default(),
        ));
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyF);
        }
        app.update();
        // Check ActionState changed to Block
        let mut query = app.world_mut().query::<&ActionState>();
        let action = *query.single(app.world()).expect("player entity");
        assert_eq!(
            action,
            ActionState::Block,
            "F with shield should transition to Block"
        );
    }

    #[test]
    fn test_block_no_shield_stays_none() {
        // R16-F197 FIX: Negative case
        let mut app = make_input_app();
        // Spawn character WITHOUT shield
        app.world_mut().spawn((
            Character,
            SpriteReq::default(), // shield = Shield::None
            ActionState::None,
            AnimationState::default(),
            Transform::default(),
        ));
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyF);
        }
        app.update();
        let mut query = app.world_mut().query::<&ActionState>();
        let action = *query.single(app.world()).expect("player entity");
        assert_eq!(
            action,
            ActionState::None,
            "F without shield should stay None"
        );
    }

    #[test]
    fn test_forces_reset_each_frame() {
        let mut app = make_input_app();
        // First frame: press W
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyW);
        }
        app.update();
        // Second frame: release W (clear_just_pressed)
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.release(KeyCode::KeyW);
        }
        app.update();
        let io = app.world().get_resource::<PhysicsIO>().unwrap();
        assert!(
            io.y_force.abs() < 0.01,
            "Forces should reset when W released, got {}",
            io.y_force
        );
    }
}
