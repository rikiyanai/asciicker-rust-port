use bevy::prelude::*;

/// Position component - maps to Asciicker's x, y, z coordinates
#[derive(Component, Clone, Debug)]
pub struct Position(pub Vec3);

impl Default for Position {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}
