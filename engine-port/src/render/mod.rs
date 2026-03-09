pub mod assembly;
pub mod camera;
pub mod config;
pub mod font;
pub mod material;
pub mod math;
pub mod mesh_shader;
pub mod pipeline;
pub mod quantize;
pub mod rasterizer;
pub mod resolve;
pub mod resolve_bridge;
pub mod sample_buffer;
pub mod shape_vector;
pub mod sprite_blit;
pub mod terrain_shader;
pub mod types;
pub mod water;

use bevy::prelude::*;

use assembly::{AssemblyState, MeshRegistry, a3d_assembly_system, load_a3d_scene, poll_akm_meshes};
use camera::{GameCamera, camera_input_system, camera_update_system, has_characters};
use config::RenderConfig;
use pipeline::{PipelineTiming, camera_terrain_init_system, render_pipeline_system};
use sample_buffer::SampleBuffer;
use shape_vector::ShapeVectorMatcher;
use sprite_blit::SpriteQueue;

use crate::system_sets::RenderSet;

/// The 6-stage CPU rasterization pipeline, matching the C++ render loop.
///
/// Stages execute in order: Clear -> Terrain -> World -> Shadow -> Reflection -> Resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Stage 1: memcpy clear of the SampleBuffer.
    Clear,
    /// Stage 2: terrain patch rasterization.
    Terrain,
    /// Stage 3: mesh/sprite rasterization.
    World,
    /// Stage 4: player shadow projection.
    Shadow,
    /// Stage 5: re-render below water plane for reflections.
    Reflection,
    /// Stage 6: 2x2 downsample SampleBuffer -> AnsiCell grid.
    Resolve,
}

/// Water configuration resource owned by CpuRasterizerPlugin.
///
/// Controls water reflection rendering and ripple animation.
/// Read by render_pipeline_system (PostUpdate) for Stage 5 REFLECTION
/// and resolve-stage ripple pass. Written by advance_water_time_system (Update)
/// and sync_water_to_render (Update, GamePlugin).
#[derive(Resource)]
pub struct WaterConfig {
    /// Water surface height (NEG_INFINITY = no water).
    pub water_z: f32,
    /// Animation time for Perlin noise ripple effect.
    pub ripple_time: f32,
}

/// Advance water ripple animation time each frame.
///
/// Registered in Update by CpuRasterizerPlugin (owns WaterConfig resource).
/// Update always precedes PostUpdate -- no explicit ordering needed.
fn advance_water_time_system(time: Res<Time>, mut water_config: ResMut<WaterConfig>) {
    water_config.ripple_time += time.delta_secs();
}

pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderConfig>()
            .init_resource::<SampleBuffer>()
            .init_resource::<GameCamera>()
            .init_resource::<AssemblyState>()
            .init_resource::<PipelineTiming>()
            .init_resource::<MeshRegistry>()
            .init_resource::<SpriteQueue>()
            .insert_resource(WaterConfig {
                water_z: f32::NEG_INFINITY,
                ripple_time: 0.0,
            })
            .insert_resource(ShapeVectorMatcher::new_default());

        app.add_systems(Startup, (load_a3d_scene, verify_plugin_prerequisites));

        // R19-F01 FIX: Chain split -- camera+assembly+mesh loading+terrain init stay in Update.
        // render_pipeline_system moves to PostUpdate for character sprite visibility.
        app.add_systems(
            Update,
            (
                camera_input_system.run_if(not(has_characters)),
                camera_update_system,
                a3d_assembly_system.run_if(|assembly: Res<AssemblyState>| !assembly.assembled),
                poll_akm_meshes,
                camera_terrain_init_system,
            )
                .chain(),
        );

        // Water time advances in Update (before PostUpdate render reads it)
        // R8-XP-002: Labeled with RenderSet::WaterTime so GamePlugin can gate on Playing state.
        app.add_systems(Update, advance_water_time_system.in_set(RenderSet::WaterTime));

        // R19-F04 FIX: render_pipeline_system in PostUpdate with RenderSet::Pipeline label.
        // This enables cross-plugin ordering: CharacterSet::SpritePush.before(RenderSet::Pipeline)
        app.add_systems(
            PostUpdate,
            render_pipeline_system.in_set(RenderSet::Pipeline),
        );
        // 1-frame display latency: PostUpdate (pipeline writes cell_grid) -> Render schedule
        // (GPU reads cell_grid). Standard Bevy behavior. Not a bug.

        #[cfg(feature = "inspector")]
        {
            use bevy_inspector_egui::quick::ResourceInspectorPlugin;
            app.add_plugins(ResourceInspectorPlugin::<PipelineTiming>::default());
            app.add_plugins(ResourceInspectorPlugin::<RenderConfig>::default());
        }

        info!("CpuRasterizerPlugin registered (with pipeline, assembly, sprites)");
    }
}

/// Startup system that verifies required plugins are registered before CpuRasterizerPlugin.
///
/// Note: AsciiOutputPlugin must come AFTER CpuRasterizerPlugin (needs RenderConfig for
/// AsciiCellGrid::from_world), so we check for it at Startup time when both have built.
fn verify_plugin_prerequisites(world: &World) {
    assert!(
        world.contains_resource::<crate::terrain::RuntimeTerrain>(),
        "CpuRasterizerPlugin requires TerrainPlugin to be registered FIRST. \
         RuntimeTerrain resource is missing."
    );
    assert!(
        world.contains_resource::<crate::world::RuntimeWorld>(),
        "CpuRasterizerPlugin requires WorldPlugin to be registered FIRST. \
         RuntimeWorld resource is missing."
    );
    assert!(
        world.contains_resource::<crate::output::ascii_cell_grid::AsciiCellGrid>(),
        "CpuRasterizerPlugin requires AsciiOutputPlugin to be registered AFTER it. \
         AsciiCellGrid resource is missing."
    );
}
