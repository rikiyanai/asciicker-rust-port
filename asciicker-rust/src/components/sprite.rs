use bevy::prelude::*;
use std::sync::Arc;

/// Sprite component - references loaded sprite data
#[derive(Component, Clone, Debug)]
pub struct Sprite {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<SpriteLayer>,
}

/// Single layer of sprite data
#[derive(Clone, Debug)]
pub struct SpriteLayer {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<XpCell>,
}

/// XPCell - 10 bytes per cell (glyph + fg + bg)
#[derive(Clone, Debug)]
pub struct XpCell {
    pub glyph: u32,     // CP437 code point
    pub fg: [u8; 3],   // Foreground RGB
    pub bg: [u8; 3],   // Background RGB
}

/// Sprite instance in world
#[derive(Component, Clone, Debug)]
pub struct SpriteInstance {
    pub sprite: Arc<Sprite>,
    pub anim: u32,
    pub frame: u32,
    pub yaw: f32,
    pub reps: [u32; 4],  // Animation timing
}

impl Default for SpriteInstance {
    fn default() -> Self {
        Self {
            sprite: Arc::new(Sprite::default()),
            anim: 0,
            frame: 0,
            yaw: 0.0,
            reps: [0, 0, 0, 0],
        }
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            layers: vec![],
        }
    }
}
