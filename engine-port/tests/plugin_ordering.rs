//! Integration tests verifying plugin initialization ordering.
//!
//! AUDIT-05: Ensures that plugins can be added in the correct order
//! without panics, and that expected resources are present after init.

use bevy::prelude::*;

use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::output::ascii_cell_grid::AsciiCellGrid;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::render::config::RenderConfig;

/// Test that plugins in correct order produce the expected resources after build.
/// CpuRasterizerPlugin requires TerrainPlugin, WorldPlugin before it,
/// and AsciiOutputPlugin after (needs RenderConfig for AsciiCellGrid::from_world).
///
/// Note: We do NOT call app.update() because the pipeline systems require
/// a running asset server with real files. Resource presence after build()
/// is the correct thing to test for plugin ordering.
#[test]
fn correct_plugin_order_succeeds() {
    use asciicker_engine::terrain::TerrainPlugin;
    use asciicker_engine::world::WorldPlugin;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((
        WorldPlugin,
        TerrainPlugin,
        CpuRasterizerPlugin,
        AsciiOutputPlugin,
    ));

    // Verify resources exist after build (no update needed)
    // RenderConfig should exist (inserted by CpuRasterizerPlugin)
    assert!(
        app.world().contains_resource::<RenderConfig>(),
        "RenderConfig should exist after CpuRasterizerPlugin init"
    );

    // AsciiCellGrid should exist (inserted by AsciiOutputPlugin)
    assert!(
        app.world().contains_resource::<AsciiCellGrid>(),
        "AsciiCellGrid should exist after AsciiOutputPlugin init"
    );
}

/// Test that all 8 plugins from main.rs init (build) in order without panic.
///
/// AssetLoaderPlugin requires AssetPlugin, so we include it explicitly
/// since MinimalPlugins does not provide it.
///
/// Note: We do NOT call app.update() because the pipeline systems require
/// a running asset server with real files. Plugin build() ordering is what
/// we are testing here.
#[test]
fn all_plugins_init_in_main_order() {
    use asciicker_engine::asset_loader::AssetLoaderPlugin;
    use asciicker_engine::character::CharacterPlugin;
    use asciicker_engine::game::GamePlugin;
    use asciicker_engine::physics::PhysicsPlugin;
    use asciicker_engine::terrain::TerrainPlugin;
    use asciicker_engine::world::WorldPlugin;

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
    ));
    app.add_plugins((
        AssetLoaderPlugin,
        WorldPlugin,
        TerrainPlugin,
        CpuRasterizerPlugin,
        AsciiOutputPlugin,
        PhysicsPlugin,
        CharacterPlugin,
        GamePlugin,
    ));

    // Should not panic during plugin build
    // All resources from all plugins should exist
    assert!(app.world().contains_resource::<RenderConfig>());
    assert!(app.world().contains_resource::<AsciiCellGrid>());
}

/// Test that AsciiOutputPlugin without CpuRasterizerPlugin panics
/// because RenderConfig (a dependency) is missing.
#[test]
#[should_panic]
fn missing_render_config_panics() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Only add AsciiOutputPlugin without CpuRasterizerPlugin
    app.add_plugins(AsciiOutputPlugin);

    // This should panic because AsciiOutputPlugin depends on
    // RenderConfig which CpuRasterizerPlugin provides.
    app.update();
}
