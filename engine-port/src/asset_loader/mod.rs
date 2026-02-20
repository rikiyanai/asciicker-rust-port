use bevy::prelude::*;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, _app: &mut App) {
        info!("AssetLoaderPlugin registered");
    }
}
