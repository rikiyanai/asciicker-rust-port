use bevy::prelude::*;

pub mod bsp;
pub mod instance;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, _app: &mut App) {
        info!("WorldPlugin registered");
    }
}
