//! Bevy App integration tests for asset loading through the AssetServer pipeline.
//!
//! Validates ASSET-06: assets load via `asset_server.load::<T>()` returning
//! typed `Handle<T>` with async loading and dependency tracking.

use bevy::prelude::*;

use asciicker_engine::asset_loader::AssetLoaderPlugin;
use asciicker_engine::asset_loader::a3d_terrain::A3dTerrain;
use asciicker_engine::asset_loader::akm_mesh::AkmMesh;
use asciicker_engine::asset_loader::bevy_loaders::A3dFile;
use asciicker_engine::asset_loader::xp_sprite::XpSprite;

/// Build a Bevy App with minimal plugins and the AssetLoaderPlugin.
///
/// The `file_path` is set to `tests/golden` so that asset paths are relative
/// to the golden test asset directory.
fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "tests/golden".to_string(),
            ..default()
        },
        AssetLoaderPlugin,
    ));
    app
}

/// Run the app until the given handle is loaded or max frames reached.
/// Returns true if the asset was loaded.
fn wait_for_load<A: Asset>(app: &mut App, handle: &Handle<A>, max_frames: usize) -> bool {
    let asset_server = app.world().resource::<AssetServer>().clone();
    for _ in 0..max_frames {
        app.update();
        if asset_server.is_loaded_with_dependencies(handle) {
            return true;
        }
    }
    false
}

#[test]
fn test_xp_sprite_loads_via_asset_server() {
    let mut app = build_test_app();

    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<XpSprite> = asset_server.load("xp/item-apple.xp");

    assert!(
        wait_for_load(&mut app, &handle, 200),
        "XpSprite should load within 200 frames"
    );

    let sprites = app.world().resource::<Assets<XpSprite>>();
    let sprite = sprites.get(&handle).expect("XpSprite should be accessible");
    assert_eq!(sprite.width, 2);
    assert_eq!(sprite.height, 2);
    assert!(sprite.layers.len() >= 3);
}

#[test]
fn test_akm_mesh_loads_via_asset_server() {
    let mut app = build_test_app();

    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<AkmMesh> = asset_server.load("akm/Cube.akm");

    assert!(
        wait_for_load(&mut app, &handle, 200),
        "AkmMesh should load within 200 frames"
    );

    let meshes = app.world().resource::<Assets<AkmMesh>>();
    let mesh = meshes.get(&handle).expect("AkmMesh should be accessible");
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.faces.len(), 12);
}

#[test]
fn test_a3d_file_loads_with_labeled_sub_assets() {
    let mut app = build_test_app();

    let asset_server = app.world().resource::<AssetServer>().clone();
    let handle: Handle<A3dFile> = asset_server.load("a3d/minimal_1x1.a3d");

    assert!(
        wait_for_load(&mut app, &handle, 200),
        "A3dFile should load within 200 frames"
    );

    // Verify root A3dFile is accessible
    let a3d_files = app.world().resource::<Assets<A3dFile>>();
    let a3d_file = a3d_files
        .get(&handle)
        .expect("A3dFile should be accessible");

    // Verify terrain sub-asset handle is valid and loaded
    let terrains = app.world().resource::<Assets<A3dTerrain>>();
    let terrain = terrains
        .get(&a3d_file.terrain)
        .expect("A3dTerrain sub-asset should be accessible");
    assert_eq!(terrain.patches.len(), 1, "minimal_1x1 has 1 terrain patch");
}
