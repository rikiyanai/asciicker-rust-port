use bevy::prelude::*;

pub mod a3d_terrain;
pub mod a3d_world;
pub mod akm_mesh;
pub mod bevy_loaders;
pub mod constants;
pub mod error;
pub mod xp_sprite;

pub use error::AssetError;

use a3d_terrain::{A3dTerrain, MaterialTable};
use a3d_world::A3dWorld;
use akm_mesh::AkmMesh;
use bevy_loaders::{A3dFile, A3dFileLoader, AkmMeshLoader, XpSpriteLoader};
use xp_sprite::XpSprite;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        // XP sprites
        app.init_asset::<XpSprite>();
        app.register_asset_loader(XpSpriteLoader);

        // A3D composite files (terrain + materials + world)
        app.init_asset::<A3dFile>();
        app.init_asset::<A3dTerrain>();
        app.init_asset::<MaterialTable>();
        app.init_asset::<A3dWorld>();
        app.register_asset_loader(A3dFileLoader);

        // AKM meshes
        app.init_asset::<AkmMesh>();
        app.register_asset_loader(AkmMeshLoader);

        info!("AssetLoaderPlugin registered: XpSpriteLoader, A3dFileLoader, AkmMeshLoader");
    }
}
