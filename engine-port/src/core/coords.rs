use std::ops::Deref;

use bevy::math::Vec3;

/// Asciicker uses a Z-up coordinate system.
/// Bevy uses Y-up internally.
/// All game logic operates in Z-up space; conversion happens at the Bevy rendering boundary.
pub const UP: Vec3 = Vec3::Z;

/// Forward direction in Z-up game space (+Y axis).
pub const FORWARD: Vec3 = Vec3::Y;

/// Right direction in Z-up game space (+X axis).
pub const RIGHT: Vec3 = Vec3::X;

/// Newtype wrapper marking a Vec3 as being in game space (Z-up coordinate system).
///
/// Prevents accidental assignment of raw Bevy Vec3 (Y-up) values to game-space
/// positions. Derefs to Vec3 for read-only math operations; explicit conversion
/// is required to cross the coordinate boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameVec3(pub Vec3);

impl GameVec3 {
    /// The zero vector in game space.
    pub const ZERO: Self = Self(Vec3::ZERO);

    /// Create a new GameVec3 from components.
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(Vec3::new(x, y, z))
    }

    /// Convert this game-space vector to Bevy render space (Y-up).
    #[inline]
    pub fn to_bevy(self) -> Vec3 {
        Vec3::new(self.0.x, self.0.z, -self.0.y)
    }

    /// Create a GameVec3 from a Bevy render-space vector (Y-up).
    #[inline]
    pub fn from_bevy(v: Vec3) -> Self {
        Self(Vec3::new(v.x, -v.z, v.y))
    }

    /// Access the inner Vec3 value.
    #[inline]
    pub fn inner(self) -> Vec3 {
        self.0
    }
}

impl Deref for GameVec3 {
    type Target = Vec3;

    #[inline]
    fn deref(&self) -> &Vec3 {
        &self.0
    }
}

/// Convert from game space (Z-up) to Bevy render space (Y-up).
///
/// Game: +X right, +Y forward, +Z up
/// Bevy: +X right, +Y up, -Z forward
#[inline]
pub fn game_to_bevy(v: GameVec3) -> Vec3 {
    v.to_bevy()
}

/// Convert from Bevy render space (Y-up) to game space (Z-up).
///
/// Bevy: +X right, +Y up, -Z forward
/// Game: +X right, +Y forward, +Z up
#[inline]
pub fn bevy_to_game(v: Vec3) -> GameVec3 {
    GameVec3::from_bevy(v)
}
