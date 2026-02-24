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
            ActionState::Attack => !matches!(self, ActionState::Fall | ActionState::Stand | ActionState::Dead | ActionState::Block),
            ActionState::Block => !matches!(self, ActionState::Fall | ActionState::Stand | ActionState::Dead | ActionState::Attack),
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Transition guard tests ---

    #[test]
    fn test_none_always_allowed() {
        for state in [ActionState::None, ActionState::Attack, ActionState::Block, ActionState::Fall, ActionState::Stand, ActionState::Dead] {
            assert!(state.can_transition_to(ActionState::None), "Should always transition to None from {:?}", state);
        }
    }

    #[test]
    fn test_attack_blocked_from_fall_stand_dead_block() {
        assert!(!ActionState::Fall.can_transition_to(ActionState::Attack));
        assert!(!ActionState::Stand.can_transition_to(ActionState::Attack));
        assert!(!ActionState::Dead.can_transition_to(ActionState::Attack));
        assert!(!ActionState::Block.can_transition_to(ActionState::Attack));
    }

    #[test]
    fn test_attack_allowed_from_none() {
        assert!(ActionState::None.can_transition_to(ActionState::Attack));
    }

    #[test]
    fn test_block_blocked_from_fall_stand_dead_attack() {
        assert!(!ActionState::Fall.can_transition_to(ActionState::Block));
        assert!(!ActionState::Stand.can_transition_to(ActionState::Block));
        assert!(!ActionState::Dead.can_transition_to(ActionState::Block));
        assert!(!ActionState::Attack.can_transition_to(ActionState::Block));
    }

    #[test]
    fn test_block_allowed_from_none() {
        assert!(ActionState::None.can_transition_to(ActionState::Block));
    }

    #[test]
    fn test_block_attack_mutual_exclusion() {
        // Cannot go from Attack to Block
        assert!(!ActionState::Attack.can_transition_to(ActionState::Block));
        // Cannot go from Block to Attack
        assert!(!ActionState::Block.can_transition_to(ActionState::Attack));
    }

    #[test]
    fn test_fall_blocked_only_from_dead() {
        assert!(!ActionState::Dead.can_transition_to(ActionState::Fall));
        // Allowed from all others
        assert!(ActionState::None.can_transition_to(ActionState::Fall));
        assert!(ActionState::Attack.can_transition_to(ActionState::Fall));
        assert!(ActionState::Block.can_transition_to(ActionState::Fall));
        assert!(ActionState::Stand.can_transition_to(ActionState::Fall));
    }

    #[test]
    fn test_stand_only_from_fall_or_dead() {
        assert!(ActionState::Fall.can_transition_to(ActionState::Stand));
        assert!(ActionState::Dead.can_transition_to(ActionState::Stand));
        assert!(!ActionState::None.can_transition_to(ActionState::Stand));
        assert!(!ActionState::Attack.can_transition_to(ActionState::Stand));
        assert!(!ActionState::Block.can_transition_to(ActionState::Stand));
    }

    #[test]
    fn test_dead_from_any_state() {
        for state in [ActionState::None, ActionState::Attack, ActionState::Block, ActionState::Fall, ActionState::Stand, ActionState::Dead] {
            assert!(state.can_transition_to(ActionState::Dead), "Should transition to Dead from {:?}", state);
        }
    }

    #[test]
    fn test_try_transition_success() {
        let mut state = ActionState::None;
        assert!(state.try_transition(ActionState::Attack));
        assert_eq!(state, ActionState::Attack);
    }

    #[test]
    fn test_try_transition_failure() {
        let mut state = ActionState::Fall;
        assert!(!state.try_transition(ActionState::Attack));
        assert_eq!(state, ActionState::Fall);
    }

    #[test]
    fn test_attack_auto_transitions_to_none() {
        let mut state = ActionState::Attack;
        let anim = AnimationState { frame_index: 8, elapsed_frames: 8 };
        assert!(state.check_animation_complete(&anim));
        assert_eq!(state, ActionState::None);
    }

    #[test]
    fn test_stand_auto_transitions_to_none() {
        let mut state = ActionState::Stand;
        let anim = AnimationState { frame_index: 5, elapsed_frames: 5 };
        assert!(state.check_animation_complete(&anim));
        assert_eq!(state, ActionState::None);
    }

    #[test]
    fn test_fall_no_auto_transition() {
        let mut state = ActionState::Fall;
        let anim = AnimationState { frame_index: 100, elapsed_frames: 100 };
        assert!(!state.check_animation_complete(&anim));
        assert_eq!(state, ActionState::Fall);
    }

    #[test]
    fn test_is_movement_locked() {
        assert!(!ActionState::None.is_movement_locked());
        assert!(ActionState::Attack.is_movement_locked());
        assert!(ActionState::Block.is_movement_locked());
        assert!(ActionState::Fall.is_movement_locked());
        assert!(ActionState::Stand.is_movement_locked());
        assert!(ActionState::Dead.is_movement_locked());
    }

    #[test]
    fn test_character_marker_is_component() {
        // Verify Character derives Component (compile-time check via type bound)
        fn _assert_component<T: Component>() {}
        _assert_component::<Character>();
    }
}
