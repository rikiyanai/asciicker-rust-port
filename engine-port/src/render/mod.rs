pub mod camera;
pub mod config;
pub mod material;
pub mod math;
pub mod mesh_shader;
pub mod quantize;
pub mod rasterizer;
pub mod resolve;
pub mod resolve_bridge;
pub mod sample_buffer;
pub mod terrain_shader;
pub mod types;

use bevy::prelude::*;

use camera::{GameCamera, camera_input_system, camera_update_system};
use config::RenderConfig;
use sample_buffer::SampleBuffer;

/// The 6-stage CPU rasterization pipeline, matching the C++ render loop.
///
/// Stages execute in order: Clear -> Terrain -> World -> Shadow -> Reflection -> Resolve.
/// Stages 2-5 (Terrain through Reflection) are stubs until Phase 5 integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Stage 1: memcpy clear of the SampleBuffer.
    Clear,
    /// Stage 2: terrain patch rasterization (Phase 5).
    Terrain,
    /// Stage 3: mesh/sprite rasterization (Phase 5).
    World,
    /// Stage 4: player shadow projection (Phase 5).
    Shadow,
    /// Stage 5: re-render below water plane for reflections (Phase 5).
    Reflection,
    /// Stage 6: 2x2 downsample SampleBuffer -> AnsiCell grid.
    Resolve,
}

pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderConfig>()
            .init_resource::<SampleBuffer>()
            .init_resource::<GameCamera>()
            .add_systems(Update, (camera_input_system, camera_update_system).chain());
        info!("CpuRasterizerPlugin registered");
    }
}

/// Stub render pipeline system: runs Clear + Resolve (stages 2-5 are Phase 5).
///
/// Not added to a schedule yet -- Phase 5 wires the full pipeline.
/// Exists to verify the system signature compiles.
#[allow(dead_code)]
fn render_pipeline(mut sample_buf: ResMut<SampleBuffer>) {
    // Stage 1: Clear
    sample_buf.clear();
    // Stages 2-5: Stubs (Phase 5)
    // Stage 6: Resolve
    // resolve::resolve(&sample_buf.samples, ...);
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
            if sample.height > z || sample.height == Sample::CLEAR_HEIGHT {
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

        // Verify: grid line cells have grid glyph override
        // Grid line at y=10 in sample space -> output row cy = (10-2)/2 = 4
        let grid_row = 4;
        let mut found_grid = false;
        for cx in 0..ascii_w {
            let cell = &output[(grid_row * ascii_w + cx) as usize];
            if cell.spare == 0xFF {
                let grid_glyphs = [b'+', b'-', b'|'];
                if grid_glyphs.contains(&cell.gl) {
                    found_grid = true;
                }
            }
        }
        assert!(
            found_grid,
            "Should find at least one grid glyph on the grid line row"
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
