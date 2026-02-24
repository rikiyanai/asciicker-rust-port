//! Character module: state machine, equipment, animation, input, sprite query.
//!
//! Port of C++ game.cpp/game.h character subsystem.

pub mod animation;
pub mod equipment;
pub mod state_machine;

use bevy::prelude::*;

pub use animation::AnimationState;
pub use equipment::SpriteReq;
pub use state_machine::{ActionState, Character};

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, _app: &mut App) {
        info!("CharacterPlugin registered");
    }
}
