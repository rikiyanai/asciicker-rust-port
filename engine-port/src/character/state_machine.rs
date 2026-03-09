//! Character state machine: ActionState enum with transition guards.
//!
//! Port of C++ game.h ACTION enum with added Block state per CHAR-01.
//! Transitions are guarded at the state level; equipment guards (e.g. shield
//! required for Block) are enforced by the caller in input.rs.

use bevy::prelude::*;

use super::animation::AnimationState;
use super::equipment::SpriteReq;

/// Marker component for character entities.
///
/// Uses Bevy 0.18 Required Components to auto-insert ActionState, SpriteReq,
/// AnimationState, and Transform when spawning with just `Character`.
#[derive(Component, Default)]
#[require(Transform, ActionState, SpriteReq, AnimationState)]
pub struct Character;

/// Character action state.
///
/// Ported from C++ game.h ACTION enum with added Block variant (CHAR-01).
/// `None` covers idle/walk/run -- speed determined by PhysicsIO velocity magnitude.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ActionState {
    /// Idle, walking, or running (speed from PhysicsIO velocity).
    #[default]
    None,
    /// Melee/ranged attack animation.
    Attack,
    /// Shield block (movement-locked). Added per CHAR-01 (C++ lacks explicit block state).
    Block,
    /// Falling (not grounded, vel_z < -Z_THRESH).
    Fall,
    /// Standing up from fall or death.
    Stand,
    /// Dead (permanent until respawn).
    Dead,
}

impl ActionState {
    /// Check if transitioning to `target` is allowed from the current state.
    ///
    /// State-level guards only. Equipment guards (shield for Block) are caller responsibility.
    pub fn can_transition_to(&self, target: ActionState) -> bool {
        match target {
            ActionState::None => true,
            ActionState::Attack => !matches!(
                self,
                ActionState::Fall | ActionState::Stand | ActionState::Dead | ActionState::Block
            ),
            ActionState::Block => !matches!(
                self,
                ActionState::Fall | ActionState::Stand | ActionState::Dead | ActionState::Attack
            ),
            ActionState::Fall => !matches!(self, ActionState::Dead),
            ActionState::Stand => matches!(self, ActionState::Fall | ActionState::Dead),
            ActionState::Dead => true,
        }
    }

    /// Attempt transition to `target`. Returns true if transition was applied.
    pub fn try_transition(&mut self, target: ActionState) -> bool {
        if self.can_transition_to(target) {
            *self = target;
            true
        } else {
            false
        }
    }

    /// Check if the current action's animation has finished and auto-transition.
    ///
    /// Returns true if an auto-transition occurred:
    /// - Attack -> None (attack animation finished)
    /// - Stand -> None (stand-up animation finished)
    /// - Fall: does NOT auto-transition (stays until grounded)
    /// - Block: does NOT auto-transition (stays until key released)
    /// - Dead: does NOT auto-transition (permanent)
    ///   // TODO: Dead state needs respawn/menu-return flow -- currently permanent.
    pub fn check_animation_complete(&mut self, anim: &AnimationState) -> bool {
        match self {
            ActionState::Attack => {
                if anim.is_attack_complete() {
                    *self = ActionState::None;
                    true
                } else {
                    false
                }
            }
            ActionState::Stand => {
                if anim.is_stand_complete() {
                    *self = ActionState::None;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Whether the character is movement-locked in this state.
    ///
    /// True for Attack, Block, Dead, Stand, Fall.
    pub fn is_movement_locked(&self) -> bool {
        !matches!(self, ActionState::None)
    }
}
