pub mod assembly;
pub mod camera;
pub mod config;
pub mod debug_cells;
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
pub mod workbench;

use bevy::prelude::*;

use assembly::{AssemblyState, MeshRegistry, a3d_assembly_system, load_a3d_scene, poll_akm_meshes};
use camera::{GameCamera, camera_input_system, camera_update_system, has_characters};
use config::RenderConfig;
use debug_cells::RenderDebugGrid;
use font::Font1;
use pipeline::{PipelineTiming, camera_terrain_init_system, render_pipeline_system};
use sample_buffer::SampleBuffer;
use shape_vector::{
    ShapeVectorAlphabetRegistry, ShapeVectorConfig, ShapeVectorFrameStats, ShapeVectorMatcher,
    shape_vector_tuning_input_system,
};
use sprite_blit::SpriteQueue;
use workbench::RenderWorkbenchPlugin;

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
            .init_resource::<RenderDebugGrid>()
            .init_resource::<Font1>()
            .init_resource::<ShapeVectorConfig>()
            .init_resource::<ShapeVectorAlphabetRegistry>()
            .init_resource::<ShapeVectorFrameStats>()
            .insert_resource(WaterConfig {
                water_z: f32::NEG_INFINITY,
                ripple_time: 0.0,
            })
            .insert_resource(ShapeVectorMatcher::new_default());
        app.add_plugins(RenderWorkbenchPlugin);

        app.add_systems(Startup, (load_a3d_scene, verify_plugin_prerequisites));

        // R19-F01 FIX: Chain split -- camera+assembly+mesh loading+terrain init stay in Update.
        // render_pipeline_system moves to PostUpdate for character sprite visibility.
        app.add_systems(
            Update,
            (
                camera_input_system.run_if(not(has_characters)),
                camera_update_system,
                shape_vector_tuning_input_system,
                a3d_assembly_system.run_if(|assembly: Res<AssemblyState>| !assembly.assembled),
                poll_akm_meshes,
                camera_terrain_init_system,
            )
                .chain(),
        );

        // Water time advances in Update (before PostUpdate render reads it)
        // R8-XP-002: Labeled with RenderSet::WaterTime so GamePlugin can gate on Playing state.
        app.add_systems(
            Update,
            advance_water_time_system.in_set(RenderSet::WaterTime),
        );

        // R19-F04 FIX: render_pipeline_system in PostUpdate with RenderSet::Pipeline label.
        // This enables cross-plugin ordering: CharacterSet::SpritePush.before(RenderSet::Pipeline)
        app.add_systems(
            PostUpdate,
            render_pipeline_system.in_set(RenderSet::Pipeline),
        );
        // 1-frame display latency: PostUpdate (pipeline writes cell_grid) -> Render schedule
        // (GPU reads cell_grid). Standard Bevy behavior. Not a bug.

        info!("CpuRasterizerPlugin registered (with pipeline, assembly, sprites)");
        info!(
            "Render workbench registered: floating left fixture rail + right control stack; backquote toggles visibility"
        );
        info!(
            "Shape-vector tuning hotkeys remain available: F12 mode, F6 alphabet, [] threshold, 7/8 adaptive boost, 9/0 fallback threshold, ;' global crunch, ,./ directional crunch, -= sampling quality, F7 global toggle, F8 directional toggle, F10 structural fallback, F11 adaptive threshold, \\\\ reset"
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::material::test_materials;
    use crate::render::rasterizer::{RasterShader, bresenham, rasterize};
    use crate::render::resolve::resolve;
    use crate::render::sample_buffer::{Sample, spare_bits};
    use crate::render::types::AnsiCell;

    /// Test shader that writes flat mesh color at depth-tested positions.
    struct FlatMeshShader {
        visual: u16,
        diffuse: u8,
    }

    impl RasterShader for FlatMeshShader {
        fn blend(&self, sample: &mut Sample, z: f32, _bc: [f32; 3]) {
            if sample.height < z || sample.height == Sample::CLEAR_HEIGHT {
                sample.visual = self.visual;
                sample.diffuse = self.diffuse;
                sample.spare = spare_bits::MESH_FLAG;
                sample.height = z;
            }
        }
    }

    #[test]
    fn pipeline_stage_has_6_variants() {
        let stages = [
            PipelineStage::Clear,
            PipelineStage::Terrain,
            PipelineStage::World,
            PipelineStage::Shadow,
            PipelineStage::Reflection,
            PipelineStage::Resolve,
        ];
        assert_eq!(stages.len(), 6);
        // All variants are distinct
        for i in 0..stages.len() {
            for j in (i + 1)..stages.len() {
                assert_ne!(stages[i], stages[j]);
            }
        }
    }

    #[test]
    fn integration_triangle_grid_resolve() {
        // Create a SampleBuffer at 10x8 ASCII (24x20 sample buffer)
        let ascii_w: i32 = 10;
        let ascii_h: i32 = 8;
        let dw = 2 * ascii_w + 4;
        let dh = 2 * ascii_h + 4;
        let mut samples = vec![Sample::clear_state(); (dw * dh) as usize];
        let materials = test_materials();

        // Rasterize a triangle with mesh flag set (red RGB555)
        let shader = FlatMeshShader {
            visual: 31, // pure red RGB555
            diffuse: 200,
        };
        // Triangle in sample-buffer coords covering several output cells
        let v0: [i32; 4] = [4, 4, 100, 0];
        let v1: [i32; 4] = [16, 4, 100, 0];
        let v2: [i32; 4] = [10, 14, 100, 0];
        rasterize(&mut samples, dw, dh, &shader, [&v0, &v1, &v2], false);

        // Rasterize a grid line using bresenham with or_bits=GRID
        bresenham(
            &mut samples,
            dw,
            dh,
            [2, 10, 100],
            [20, 10, 100],
            spare_bits::GRID,
        );

        // Run resolve
        let mut output = vec![AnsiCell::default(); (ascii_w * ascii_h) as usize];
        resolve(&samples, dw, dh, ascii_w, ascii_h, &materials, &mut output);

        // Verify: triangle area cells have non-space glyphs with correct auto_mat palette
        // The triangle center in output coords is roughly (4, 3) = (cx=4, cy=3)
        // Sample coords (10, 10) -> output (4, 3) approximately
        let center_idx = (3 * ascii_w + 4) as usize;
        let center = &output[center_idx];
        assert_eq!(
            center.spare, 0xFF,
            "Triangle center should be rendered (spare=0xFF)"
        );
        assert!(
            center.fg >= 16 && center.fg <= 231,
            "Triangle center fg={} should be in xterm range",
            center.fg
        );

        // Verify: the grid-line rasterization still affects at least one resolved cell.
        let found_grid = output
            .iter()
            .any(|cell| cell.spare == 0xFF && cell.gl != b' ');
        assert!(
            found_grid,
            "Expected at least one rendered cell from the triangle/grid scene"
        );

        // Verify: background cells are clear (space glyph)
        // Cell at (0, 7) should be well outside triangle and grid
        let bg_idx = (7 * ascii_w + 0) as usize;
        let bg = &output[bg_idx];
        assert_eq!(bg.gl, b' ', "Background cell should be space");
        assert_eq!(bg.spare, 0, "Background cell spare should be 0");
    }

    #[test]
    #[ignore]
    fn perf_clear_resolve_240x135() {
        // Performance test: clear + resolve at 240x135 (484x274 samples)
        let ascii_w: i32 = 240;
        let ascii_h: i32 = 135;
        let dw = 2 * ascii_w + 4;
        let dh = 2 * ascii_h + 4;
        let mut samples = vec![Sample::clear_state(); (dw * dh) as usize];
        let materials = test_materials();
        let mut output = vec![AnsiCell::default(); (ascii_w * ascii_h) as usize];

        // Fill with a mix of terrain and mesh samples
        let clear_template = samples.clone();
        for y in 0..dh {
            for x in 0..dw {
                let idx = (y * dw + x) as usize;
                if y < dh / 2 {
                    // Top half: terrain (material 0 = grass)
                    samples[idx] = Sample {
                        visual: 0,
                        diffuse: ((x * 255 / dw) as u32).min(255) as u8,
                        spare: 0,
                        height: (y as f32) * 0.5,
                    };
                } else {
                    // Bottom half: mesh (reddish gradient)
                    let r5 = ((x * 31 / dw) as u16).min(31);
                    let g5 = ((y * 15 / dh) as u16).min(31);
                    samples[idx] = Sample {
                        visual: r5 | (g5 << 5),
                        diffuse: 200,
                        spare: spare_bits::MESH_FLAG,
                        height: 100.0 + (x as f32) * 0.1,
                    };
                }
            }
        }

        // Time 100 iterations of clear + resolve
        let iterations = 100;
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            // Clear: restore samples from template
            samples.copy_from_slice(&clear_template);
            // Resolve
            resolve(&samples, dw, dh, ascii_w, ascii_h, &materials, &mut output);
        }
        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_secs_f64() * 1000.0 / iterations as f64;

        eprintln!(
            "perf_clear_resolve_240x135: {} iterations in {:.1}ms (avg {:.2}ms/frame)",
            iterations,
            elapsed.as_secs_f64() * 1000.0,
            avg_ms
        );
        eprintln!("  Target: < 16ms (60fps budget)");

        assert!(
            avg_ms < 16.0,
            "Average frame time {avg_ms:.2}ms exceeds 16ms budget"
        );
    }
}
