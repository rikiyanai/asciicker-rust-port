use bevy::prelude::*;

pub mod patch_runtime;
pub mod quadtree;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, _app: &mut App) {
        info!("TerrainPlugin registered");
    }
}
