//! Bevy AssetLoader implementations for game asset formats.
//!
//! Provides three loaders that integrate the Phase 2 parsers with Bevy's
//! async asset pipeline:
//! - [`XpSpriteLoader`]: loads `.xp` sprite files into [`XpSprite`] assets
//! - [`A3dFileLoader`]: loads `.a3d` composite files into [`A3dFile`] with
//!   labeled sub-assets for terrain, materials, and world
//! - [`AkmMeshLoader`]: loads `.akm` mesh files into [`AkmMesh`] assets

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy::tasks::ConditionalSendFuture;

use super::a3d_terrain::{
    A3dTerrain, MaterialTable, parse_material_section, parse_terrain_section,
};
use super::a3d_world::{A3dWorld, parse_world_section};
use super::akm_mesh::{AkmMesh, parse_akm};
use super::error::AssetError;
use super::xp_sprite::{XpSprite, parse_xp};

// ---------------------------------------------------------------------------
// XpSpriteLoader
// ---------------------------------------------------------------------------

/// Bevy asset loader for `.xp` sprite files.
///
/// Reads gzip-compressed REXPaint sprite data and returns a typed
/// [`XpSprite`] asset accessible via `Handle<XpSprite>`.
#[derive(Default, bevy::reflect::TypePath)]
pub struct XpSpriteLoader;

impl AssetLoader for XpSpriteLoader {
    type Asset = XpSprite;
    type Settings = ();
    type Error = AssetError;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(AssetError::Io)?;
            parse_xp(&bytes)
        }
    }

    fn extensions(&self) -> &[&str] {
        &["xp"]
    }
}

// ---------------------------------------------------------------------------
// A3dFile (composite asset) + A3dFileLoader
// ---------------------------------------------------------------------------

/// Composite asset for `.a3d` files.
///
/// An A3D file contains three sequential sections: terrain patches, material
/// table, and world instances. Each section is parsed into its own sub-asset
/// and accessible via labeled handles:
/// - `"terrain"` -> [`Handle<A3dTerrain>`]
/// - `"materials"` -> [`Handle<MaterialTable>`]
/// - `"world"` -> [`Handle<A3dWorld>`]
#[derive(Asset, TypePath, Debug, Clone)]
pub struct A3dFile {
    /// Handle to the terrain section (patches + heightmaps).
    pub terrain: Handle<A3dTerrain>,
    /// Handle to the material lookup table.
    pub materials: Handle<MaterialTable>,
    /// Handle to the world instance list.
    pub world: Handle<A3dWorld>,
}

/// Bevy asset loader for composite `.a3d` files.
///
/// Parses the three sequential sections (terrain, materials, world) and
/// registers each as a labeled sub-asset. The root asset is an [`A3dFile`]
/// containing handles to all three.
#[derive(Default, bevy::reflect::TypePath)]
pub struct A3dFileLoader;

impl AssetLoader for A3dFileLoader {
    type Asset = A3dFile;
    type Settings = ();
    type Error = AssetError;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(AssetError::Io)?;

            // Parse three sequential sections with offset tracking
            let (terrain, terrain_consumed) = parse_terrain_section(&bytes)?;
            let (materials, mat_consumed) = parse_material_section(&bytes[terrain_consumed..])?;
            let world = parse_world_section(&bytes[terrain_consumed + mat_consumed..])?;

            // Register labeled sub-assets
            let terrain_handle = load_context.add_labeled_asset("terrain".to_string(), terrain);
            let materials_handle =
                load_context.add_labeled_asset("materials".to_string(), materials);
            let world_handle = load_context.add_labeled_asset("world".to_string(), world);

            Ok(A3dFile {
                terrain: terrain_handle,
                materials: materials_handle,
                world: world_handle,
            })
        }
    }

    fn extensions(&self) -> &[&str] {
        &["a3d"]
    }
}

// ---------------------------------------------------------------------------
// AkmMeshLoader
// ---------------------------------------------------------------------------

/// Bevy asset loader for `.akm` mesh files (ASCII PLY format).
///
/// Reads the text content and parses it into a typed [`AkmMesh`] asset
/// accessible via `Handle<AkmMesh>`.
#[derive(Default, bevy::reflect::TypePath)]
pub struct AkmMeshLoader;

impl AssetLoader for AkmMeshLoader {
    type Asset = AkmMesh;
    type Settings = ();
    type Error = AssetError;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        async {
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(AssetError::Io)?;
            let text = String::from_utf8_lossy(&bytes);
            parse_akm(&text)
        }
    }

    fn extensions(&self) -> &[&str] {
        &["akm"]
    }
}
