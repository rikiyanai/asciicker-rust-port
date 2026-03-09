//! Equipment system: 5D equipment enums and SpriteReq component.
//!
//! Port of C++ game.h equipment enums. SpriteReq provides the 5D
//! (kind, armor, helmet, shield, weapon) equipment combination for sprite lookup.

use bevy::prelude::*;

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};

use super::state_machine::ActionState;

/// Weapon type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Weapon {
    #[default]
    None,
    RegularSword,
    RegularCrossbow,
}

/// Shield type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Shield {
    #[default]
    None,
    RegularShield,
}

/// Helmet type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Helmet {
    #[default]
    None,
    RegularHelmet,
}

/// Armor type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Armor {
    #[default]
    None,
    RegularArmor,
}

/// Mount type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Mount {
    #[default]
    None,
    Wolf,
    Bee,
}

/// Sprite kind (determines base sprite set).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum SpriteKind {
    #[default]
    Human,
    Wolf,
    Bee,
}

/// Sprite request component: 5D equipment combination for sprite lookup.
///
/// R19-M04 FIX: `clr` field (default 0) is forward-compatible for Phase 7
/// multiplayer team/skin color. C++ GetSprite takes `clr` as first dimension:
/// `player[clr][armor][helmet][shield][weapon]`. Single-player uses clr=0.
#[derive(Component, Default, Clone, Debug)]
pub struct SpriteReq {
    pub kind: SpriteKind,
    pub mount: Mount,
    pub action: ActionState,
    pub armor: Armor,
    pub helmet: Helmet,
    pub shield: Shield,
    pub weapon: Weapon,
    /// Team/skin color index (0 = default). Phase 7 multiplayer wires per-player.
    pub clr: u8,
}

impl SpriteReq {
    /// 5D indices for sprite array lookup.
    ///
    /// Returns `(kind, armor_idx, helmet_idx, shield_idx, weapon_idx)`.
    pub fn sprite_index(&self) -> (SpriteKind, usize, usize, usize, usize) {
        let armor_idx = match self.armor {
            Armor::None => 0,
            Armor::RegularArmor => 1,
        };
        let helmet_idx = match self.helmet {
            Helmet::None => 0,
            Helmet::RegularHelmet => 1,
        };
        let shield_idx = match self.shield {
            Shield::None => 0,
            Shield::RegularShield => 1,
        };
        let weapon_idx = match self.weapon {
            Weapon::None => 0,
            Weapon::RegularSword => 1,
            Weapon::RegularCrossbow => 2,
        };
        (self.kind, armor_idx, helmet_idx, shield_idx, weapon_idx)
    }

    /// Whether equipment can be changed in the current action state (TRAP-G01).
    ///
    /// False during Attack or Block.
    pub fn can_change_equipment(&self) -> bool {
        !matches!(self.action, ActionState::Attack | ActionState::Block)
    }

    /// Collision dimensions for the character based on mount.
    ///
    /// Returns `(world_radius, world_height)`.
    /// Rust intentionally recomputes per mount (C++ has a static const bug
    /// that always uses first call's height_cells).
    pub fn collision_dimensions(&self) -> (f32, f32) {
        let (radius_cells, height_cells) = match self.mount {
            Mount::None => (2.0_f32, 7.0_f32),
            Mount::Wolf | Mount::Bee => (3.0_f32, 9.0_f32),
        };
        let world_radius = radius_cells / (3.0 * HEIGHT_CELLS as f32) * VISUAL_CELLS as f32;
        let world_height =
            height_cells * 2.0 / 3.0 / (30.0_f32.to_radians().cos()) * HEIGHT_SCALE as f32;
        (world_radius, world_height)
    }
}
