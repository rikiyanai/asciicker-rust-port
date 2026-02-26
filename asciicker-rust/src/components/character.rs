use bevy::prelude::*;

/// Character state - maps to game.cpp Character states
#[derive(Component, Clone, Debug, PartialEq)]
pub enum CharacterState {
    None,
    Attack,
    Fall,
    Stand,
    Dead,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self::None
    }
}

/// Character stats
#[derive(Component, Clone, Debug)]
pub struct CharacterStats {
    pub hp: i32,
    pub mp: i32,
    pub xp: i32,
    pub level: i32,
}

impl Default for CharacterStats {
    fn default() -> Self {
        Self {
            hp: 100,
            mp: 50,
            xp: 0,
            level: 1,
        }
    }
}

/// Equipment - maps to 5D sprite lookup
#[derive(Component, Clone, Debug, Default)]
pub struct Equipment {
    pub weapon: u8,   // 0-255
    pub armor: u8,    // 0-255
    pub helmet: u8,   // 0-255
    pub shield: u8,  // 0-255
    pub color: u8,   // 0-255
}
