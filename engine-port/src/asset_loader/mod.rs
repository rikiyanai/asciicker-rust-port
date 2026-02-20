use bevy::prelude::*;

pub mod a3d_terrain;
pub mod constants;
pub mod error;
pub mod xp_sprite;

pub use error::AssetError;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, _app: &mut App) {
        info!("AssetLoaderPlugin registered");
    }
}
