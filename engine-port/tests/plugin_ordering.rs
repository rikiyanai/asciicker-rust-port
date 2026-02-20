//! Integration tests verifying plugin initialization ordering.
//!
//! AUDIT-05: Ensures that plugins can be added in the correct order
//! without panics, and that expected resources are present after init.

use bevy::prelude::*;

use asciicker_engine::output::AsciiOutputPlugin;
use asciicker_engine::output::ascii_cell_grid::AsciiCellGrid;
use asciicker_engine::render::CpuRasterizerPlugin;
use asciicker_engine::render::config::RenderConfig;

/// Test that CpuRasterizerPlugin + AsciiOutputPlugin in correct order
/// produces the expected resources (RenderConfig and AsciiCellGrid).
#[test]
fn correct_plugin_order_succeeds() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((CpuRasterizerPlugin, AsciiOutputPlugin));

    app.update();

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

/// Test that all 8 plugins from main.rs init in order without panic.
///
/// AssetLoaderPlugin requires AssetPlugin, so we include it explicitly
/// since MinimalPlugins does not provide it.
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

    // Should not panic during update
    app.update();
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
