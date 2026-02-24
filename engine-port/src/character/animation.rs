//! Animation timing: frame advance per action type.
//!
//! Port of C++ game.cpp animation constants and frame computation.
//! Uses Model B (frame counter) for deterministic tests and Bevy fixed timestep.

use bevy::prelude::*;

use super::state_machine::ActionState;

// --- Timing constants (all u64, microseconds per frame) ---

/// Microseconds per frame for Stand-up animation.
pub const STAND_US_PER_FRAME: u64 = 30_000;

/// Microseconds per frame for Fall animation.
pub const FALL_US_PER_FRAME: u64 = 30_000;

/// Microseconds per frame for Attack animation.
pub const ATTACK_US_PER_FRAME: u64 = 20_000;

/// Microseconds per frame for Block animation.
pub const BLOCK_US_PER_FRAME: u64 = 30_000;

// --- Step animation constants ---

/// Divisor for converting player_stp to walk frame index.
pub const STEP_DIVISOR: i32 = 1024;

/// Mask for step counter (8*1024 - 1 = 8191). Used for FRAME EXTRACTION only.
/// R19-M05: player_stp increment in physics uses 0x7FFFFFFF (sign-bit mask), NOT this.
pub const STEP_MASK: i32 = 8 * 1024 - 1;

/// Offset added to player_stp before masking for walk frame.
pub const STEP_OFFSET: i32 = 3 * 1024;

// --- Frame count constants ---

/// Number of frames in the attack animation (160ms total at 20ms/frame).
pub const ATTACK_FRAME_COUNT: u32 = 8;

/// Number of frames in the stand-up animation (150ms total at 30ms/frame).
pub const STAND_FRAME_COUNT: u32 = 5;

/// Animation state component: frame counter model (Model B).
///
/// No `action_start_us`, no `Instant::now()`. Frame counting is deterministic
/// and aligns with Bevy's fixed timestep.
#[derive(Component, Default, Clone, Debug)]
pub struct AnimationState {
    /// Current frame index for the active animation.
    pub frame_index: u32,
    /// Elapsed frames (incremented per advance call).
    pub elapsed_frames: u32,
}

impl AnimationState {
    /// Advance the animation based on current action and elapsed microseconds.
    ///
    /// When `action` changes from the previous frame, resets elapsed_frames.
    /// Walk frame derived from player_stp (physics output), not elapsed time.
    ///
    /// R19-M05: player_stp increment uses 0x7FFFFFFF mask (in physics), but
    /// frame extraction here uses STEP_MASK (8191) only.
    pub fn advance(&mut self, action: ActionState, player_stp: i32, delta_us: u64) {
        match action {
            ActionState::None => {
                // Walk or idle based on player_stp
                if player_stp == -1 {
                    // Idle
                    self.frame_index = 0;
                    self.elapsed_frames = 0;
                } else {
                    // Walk: frame from player_stp
                    self.frame_index =
                        (((player_stp + STEP_OFFSET) & STEP_MASK) / STEP_DIVISOR) as u32;
                    self.elapsed_frames = 0; // reset for any subsequent action
                }
            }
            ActionState::Attack => {
                self.elapsed_frames += (delta_us / ATTACK_US_PER_FRAME) as u32;
                self.frame_index = self.elapsed_frames.min(ATTACK_FRAME_COUNT);
            }
            ActionState::Block => {
                self.elapsed_frames += (delta_us / BLOCK_US_PER_FRAME) as u32;
                // Hold on last frame while blocking (no cap, visual stays on last)
                self.frame_index = self.elapsed_frames;
            }
            ActionState::Fall => {
                self.elapsed_frames += (delta_us / FALL_US_PER_FRAME) as u32;
                self.frame_index = self.elapsed_frames;
            }
            ActionState::Stand => {
                self.elapsed_frames += (delta_us / STAND_US_PER_FRAME) as u32;
                self.frame_index = self.elapsed_frames.min(STAND_FRAME_COUNT);
            }
            ActionState::Dead => {
                // Frozen at last frame
            }
        }
    }

    /// Reset animation state (e.g. when action changes).
    pub fn reset(&mut self) {
        self.frame_index = 0;
        self.elapsed_frames = 0;
    }

    /// Whether the attack animation has completed.
    pub fn is_attack_complete(&self) -> bool {
        self.elapsed_frames >= ATTACK_FRAME_COUNT
    }

    /// Whether the stand-up animation has completed.
    pub fn is_stand_complete(&self) -> bool {
        self.elapsed_frames >= STAND_FRAME_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_frame_zero() {
        let mut anim = AnimationState::default();
        anim.advance(ActionState::None, -1, 16_000);
        assert_eq!(anim.frame_index, 0);
    }

    #[test]
    fn test_walk_frame_from_player_stp() {
        let mut anim = AnimationState::default();
        // player_stp=0, STEP_OFFSET=3072, STEP_MASK=8191
        // (0 + 3072) & 8191 = 3072, 3072 / 1024 = 3
        anim.advance(ActionState::None, 0, 16_000);
        assert_eq!(anim.frame_index, 3);
    }

    #[test]
    fn test_walk_frame_wraps_with_step_mask() {
        let mut anim = AnimationState::default();
        // player_stp=6*1024=6144, (6144+3072) & 8191 = 9216 & 8191 = 1025, 1025/1024 = 1
        anim.advance(ActionState::None, 6 * 1024, 16_000);
        assert_eq!(anim.frame_index, 1);
    }

    #[test]
    fn test_attack_frame_advance() {
        let mut anim = AnimationState::default();
        // 20_000us per frame; feed 60_000us = 3 frames
        anim.advance(ActionState::Attack, -1, 60_000);
        assert_eq!(anim.frame_index, 3);
        assert_eq!(anim.elapsed_frames, 3);
    }

    #[test]
    fn test_attack_frame_capped() {
        let mut anim = AnimationState::default();
        // Feed enough for 12 frames (240_000us at 20_000/frame)
        anim.advance(ActionState::Attack, -1, 240_000);
        assert_eq!(anim.frame_index, ATTACK_FRAME_COUNT);
    }

    #[test]
    fn test_block_frame_advance() {
        let mut anim = AnimationState::default();
        anim.advance(ActionState::Block, -1, 90_000);
        assert_eq!(anim.frame_index, 3);
    }

    #[test]
    fn test_fall_frame_advance() {
        let mut anim = AnimationState::default();
        anim.advance(ActionState::Fall, -1, 90_000);
        assert_eq!(anim.frame_index, 3);
    }

    #[test]
    fn test_stand_frame_advance_capped() {
        let mut anim = AnimationState::default();
        // Feed enough for 10 frames = 300_000us, capped at STAND_FRAME_COUNT=5
        anim.advance(ActionState::Stand, -1, 300_000);
        assert_eq!(anim.frame_index, STAND_FRAME_COUNT);
    }

    #[test]
    fn test_dead_frozen() {
        let mut anim = AnimationState {
            frame_index: 3,
            elapsed_frames: 3,
        };
        anim.advance(ActionState::Dead, -1, 1_000_000);
        assert_eq!(anim.frame_index, 3, "Dead should freeze at last frame");
    }

    #[test]
    fn test_is_attack_complete() {
        let anim = AnimationState {
            frame_index: 8,
            elapsed_frames: 8,
        };
        assert!(anim.is_attack_complete());
    }

    #[test]
    fn test_is_stand_complete() {
        let anim = AnimationState {
            frame_index: 5,
            elapsed_frames: 5,
        };
        assert!(anim.is_stand_complete());
    }

    #[test]
    fn test_reset() {
        let mut anim = AnimationState {
            frame_index: 5,
            elapsed_frames: 10,
        };
        anim.reset();
        assert_eq!(anim.frame_index, 0);
        assert_eq!(anim.elapsed_frames, 0);
    }
}
