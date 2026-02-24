//! System sets for cross-plugin ordering.
//!
//! Defines named sets for render pipeline and character systems so that
//! ordering constraints can be expressed without direct system references.

use bevy::prelude::*;

/// Render pipeline system sets.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderSet {
    /// The main render pipeline system.
    Pipeline,
}

/// Character system sets.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CharacterSet {
    /// PreUpdate: clear_sprite_queue, accumulate_player_input
    PreUpdateInput,
    /// PostUpdate: query_character_sprites
    SpritePush,
    /// PostUpdate: sync_physics_to_character (registered by GamePlugin, Phase 7)
    PhysicsSync,
}
