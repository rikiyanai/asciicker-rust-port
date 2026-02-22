//! 6-stage rendering pipeline orchestrator.
//!
//! Executes the full rendering pipeline each frame:
//! CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE
//!
//! Post-RESOLVE: deferred sprite blit (far-to-near).
//!
//! Per-stage timing is recorded in `PipelineTiming` for profiling.

use std::time::Instant;

use bevy::prelude::*;

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::assembly::{MeshRegistry, RuntimeMaterials};
use crate::render::camera::GameCamera;
use crate::render::config::RenderConfig;
use crate::render::mesh_shader::render_mesh;
use crate::render::resolve_bridge::{AutoMatGlyphSelector, resolve_to_grid};
use crate::render::sample_buffer::SampleBuffer;
use crate::render::sprite_blit::{SpriteQueue, blit_sprite};
use crate::render::types::AnsiCell;
use crate::terrain::RuntimeTerrain;
use crate::world::RuntimeWorld;

// ---------------------------------------------------------------------------
// PipelineTiming Resource
// ---------------------------------------------------------------------------

/// Per-stage timing data for the rendering pipeline (microseconds).
#[derive(Resource, Default, Debug, Clone)]
#[cfg_attr(feature = "inspector", derive(bevy_inspector_egui::prelude::InspectorOptions, Reflect))]
pub struct PipelineTiming {
    /// Stage 1: CLEAR duration in microseconds.
    pub clear_us: u64,
    /// Stage 2: TERRAIN duration in microseconds.
    pub terrain_us: u64,
    /// Stage 3: WORLD duration in microseconds.
    pub world_us: u64,
    /// Stage 4: SHADOW duration in microseconds.
    pub shadow_us: u64,
    /// Stage 5: REFLECTION duration in microseconds.
    pub reflection_us: u64,
    /// Stage 6: RESOLVE duration in microseconds.
    pub resolve_us: u64,
    /// Post-RESOLVE sprite blit duration in microseconds.
    pub sprite_us: u64,
    /// Total frame duration in microseconds.
    pub total_us: u64,
}

// ---------------------------------------------------------------------------
// Pipeline helpers
// ---------------------------------------------------------------------------

/// Ensure the SampleBuffer matches the current RenderConfig dimensions.
///
/// If dimensions mismatch (e.g. window resize), reallocate the buffer.
/// SampleBuffer.width stores SAMPLE width (2*ascii_w+4), NOT ascii width.
fn ensure_buffer_size(buf: &mut SampleBuffer, config: &RenderConfig) {
    let expected_sample_w = 2 * config.ascii_width + 4;
    let expected_sample_h = 2 * config.ascii_height + 4;
    if buf.width != expected_sample_w || buf.height != expected_sample_h {
        *buf = SampleBuffer::new(config.ascii_width, config.ascii_height);
    }
}

/// Project a world-space position to screen-space ASCII cell coordinates.
///
/// Returns `Some((screen_x, screen_y))` if the position is in front of the camera,
/// or `None` if behind.
pub fn project_world_to_screen(pos: &[f32; 3], camera: &GameCamera) -> Option<(i32, i32)> {
    // Compute distance along view direction
    let dx = pos[0] - camera.view_pos[0];
    let dy = pos[1] - camera.view_pos[1];
    let dist = dx * camera.view_dir[0] + dy * camera.view_dir[1];

    // Behind camera check (view_dir is normalized by focal, so dist > 0 means in front)
    if dist <= 0.0 {
        return None;
    }

    // Project using the isometric view matrix
    let hc = crate::asset_loader::constants::HEIGHT_CELLS as f64;
    let px = pos[0] as f64;
    let py = pos[1] as f64;
    let pz = pos[2] as f64;

    let screen_x = camera.view_tm[0] * px * hc
        + camera.view_tm[4] * py * hc
        + camera.view_tm[8] * pz
        + camera.view_tm[12];
    let screen_y = camera.view_tm[1] * px * hc
        + camera.view_tm[5] * py * hc
        + camera.view_tm[9] * pz
        + camera.view_tm[13];

    // Convert from sample-space to ASCII cell coordinates
    // Sample coords -> ASCII: (sample - 2) / 2
    let ascii_x = ((screen_x - 2.0) / 2.0) as i32;
    let ascii_y = ((screen_y - 2.0) / 2.0) as i32;

    Some((ascii_x, ascii_y))
}

// ---------------------------------------------------------------------------
// Pipeline system
// ---------------------------------------------------------------------------

/// The 6-stage rendering pipeline system.
///
/// Runs each frame. Stages:
/// 1. CLEAR: memcpy-clear the SampleBuffer
/// 2. TERRAIN: rasterize visible terrain patches (real TerrainShader)
/// 3. WORLD: rasterize visible mesh instances (real MeshShader) + queue sprites
/// 4. SHADOW: stub (deferred to Phase 6)
/// 5. REFLECTION: stub (deferred to Phase 6)
/// 6. RESOLVE: downsample SampleBuffer to AsciiCellGrid via resolve_to_grid
///
/// Post-RESOLVE: sort and blit deferred sprites.
///
/// Performance escape hatches (activate if frame budget exceeded):
/// 1. Resolution scaling: reduce RenderConfig ascii_width/ascii_height
/// 2. Shadow skip: skip terrain shadow computation at load time
/// 3. Tighter frustum: reduce far plane distance
/// 4. LOD (future): skip distant terrain patches
#[allow(clippy::too_many_arguments)]
pub fn render_pipeline_system(
    terrain: Res<RuntimeTerrain>,
    world_data: Res<RuntimeWorld>,
    camera: Res<GameCamera>,
    materials: Option<Res<RuntimeMaterials>>,
    mesh_registry: Res<MeshRegistry>,
    mut config: ResMut<RenderConfig>,
    mut sample_buffer: ResMut<SampleBuffer>,
    mut cell_grid: ResMut<AsciiCellGrid>,
    mut sprite_queue: ResMut<SpriteQueue>,
    mut timing: ResMut<PipelineTiming>,
) {
    let frame_start = Instant::now();

    // STEP 1: Sync RenderConfig with AsciiCellGrid on window resize.
    // handle_window_resize (output/mod.rs) updates AsciiCellGrid but not RenderConfig.
    if config.ascii_width != cell_grid.width || config.ascii_height != cell_grid.height {
        config.ascii_width = cell_grid.width;
        config.ascii_height = cell_grid.height;
    }

    // STEP 2: Buffer resize (exclusive &mut SampleBuffer borrow).
    ensure_buffer_size(&mut sample_buffer, &config);
    // Borrow DROPPED here.

    // STEP 3: Stage 1 CLEAR
    let t0 = Instant::now();
    sample_buffer.clear();
    timing.clear_us = t0.elapsed().as_micros() as u64;

    // STEP 4: Destructure for render stages (field borrows)
    // Read width/height BEFORE taking &mut samples to avoid borrow conflict.
    let buf_w = sample_buffer.width as i32;
    let buf_h = sample_buffer.height as i32;
    {
        let buf = &mut sample_buffer.samples;

        // Stage 2: TERRAIN (real rasterization)
        let t1 = Instant::now();
        if terrain.root.is_some() {
            let planes = &camera.frustum_planes;
            terrain.query_visible(planes, |patch| {
                crate::render::terrain_shader::render_patch(
                    buf,
                    buf_w,
                    buf_h,
                    patch,
                    patch.x,
                    patch.y,
                    &camera.view_tm,
                );
            });
        }
        timing.terrain_us = t1.elapsed().as_micros() as u64;

        // Stage 3: WORLD (real rasterization for meshes, placeholder for sprites)
        let t2 = Instant::now();
        sprite_queue.clear(); // Phase 5 standalone; Phase 6 moves this to PreUpdate

        if world_data.bsp_root.is_some() || !world_data.flat_list.is_empty() {
            let planes = &camera.frustum_planes;
            let camera_pos_f64: [f64; 3] = camera.pos.map(|x| x as f64);
            let visible = world_data.query_visible(planes, camera_pos_f64);

            for vis_inst in &visible {
                match vis_inst {
                    crate::world::bsp::VisibleInstance::Mesh(id) => {
                        if let Some(crate::world::instance::RuntimeInstance::Mesh {
                            mesh_id, tm, ..
                        }) = world_data.instances.get(id.0)
                        {
                            if let Some(mesh) = mesh_registry.loaded.get(mesh_id) {
                                render_mesh(buf, buf_w, buf_h, mesh, tm, &camera.view_tm);
                            } else {
                                trace!("Pipeline: mesh '{}' visible but AKM not yet loaded", mesh_id);
                            }
                        }
                    }
                    crate::world::bsp::VisibleInstance::Sprite(id) => {
                        if let Some(inst) = world_data.instances.get(id.0) {
                            let (sprite_name, pos, yaw, anim, frame) = match inst {
                                crate::world::instance::RuntimeInstance::Sprite {
                                    sprite_name,
                                    pos,
                                    yaw,
                                    anim,
                                    frame,
                                    ..
                                } => (
                                    sprite_name.clone(),
                                    *pos,
                                    *yaw,
                                    *anim as u32,
                                    *frame as u32,
                                ),
                                crate::world::instance::RuntimeInstance::Item {
                                    pos, yaw, ..
                                } => ("item".to_string(), *pos, *yaw, 0, 0),
                                _ => continue,
                            };

                            // Compute distance along view direction
                            let dx = pos[0] - camera.view_pos[0];
                            let dy = pos[1] - camera.view_pos[1];
                            let dist =
                                dx * camera.view_dir[0] + dy * camera.view_dir[1];

                            if let Some((sx, sy)) =
                                project_world_to_screen(&pos, &camera)
                            {
                                sprite_queue.push(
                                    crate::render::sprite_blit::SpriteRenderEntry {
                                        dist,
                                        screen_x: sx,
                                        screen_y: sy,
                                        sprite_name,
                                        pos,
                                        yaw,
                                        anim,
                                        frame,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        timing.world_us = t2.elapsed().as_micros() as u64;

        // Stage 4: SHADOW (stub -- deferred to Phase 6)
        let t3 = Instant::now();
        timing.shadow_us = t3.elapsed().as_micros() as u64;

        // Stage 5: REFLECTION (stub -- deferred to Phase 6)
        let t4 = Instant::now();
        timing.reflection_us = t4.elapsed().as_micros() as u64;
    } // mutable borrow of sample_buffer.samples ENDS here

    // STEP 5: Stage 6 RESOLVE (immutable &SampleBuffer borrow)
    let t5 = Instant::now();
    if let Some(mats) = materials.as_ref() {
        let ascii_w = config.ascii_width as usize;
        let ascii_h = config.ascii_height as usize;
        let mut resolve_buf = vec![AnsiCell::default(); ascii_w * ascii_h];
        let mut glyph_sel = AutoMatGlyphSelector;
        resolve_to_grid(
            &sample_buffer,
            &mats.0,
            &mut cell_grid,
            &mut glyph_sel,
            &mut resolve_buf,
        );
    }
    timing.resolve_us = t5.elapsed().as_micros() as u64;

    // STEP 6: Post-RESOLVE Deferred Sprite Blit
    let t6 = Instant::now();
    sprite_queue.sort_far_to_near();
    for entry in sprite_queue.drain() {
        blit_sprite(&mut cell_grid, &entry, &sample_buffer);
    }
    timing.sprite_us = t6.elapsed().as_micros() as u64;
    timing.total_us = frame_start.elapsed().as_micros() as u64;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_buffer_size_resizes() {
        // Config says 10x10, buffer is wrong size
        let config = RenderConfig {
            ascii_width: 10,
            ascii_height: 10,
        };
        let mut buf = SampleBuffer::new(5, 5); // wrong size: 14x14

        ensure_buffer_size(&mut buf, &config);

        assert_eq!(buf.width, 2 * 10 + 4);
        assert_eq!(buf.height, 2 * 10 + 4);
    }

    #[test]
    fn test_ensure_buffer_size_noop() {
        let config = RenderConfig {
            ascii_width: 10,
            ascii_height: 10,
        };
        let buf_before = SampleBuffer::new(10, 10);
        let expected_w = buf_before.width;
        let expected_h = buf_before.height;
        let expected_len = buf_before.samples.len();

        let mut buf = SampleBuffer::new(10, 10);
        ensure_buffer_size(&mut buf, &config);

        assert_eq!(buf.width, expected_w);
        assert_eq!(buf.height, expected_h);
        assert_eq!(buf.samples.len(), expected_len);
    }

    #[test]
    fn test_project_world_to_screen() {
        use crate::render::camera::GameCamera;

        let mut camera = GameCamera::default();
        camera.pos = [10.0, 10.0, 0.0];
        camera.yaw = 0.0;
        camera.zoom = 1.0;
        camera.perspective = true;
        camera.update(484.0, 274.0);
        camera.extract_frustum_planes(484.0, 274.0);

        // Camera origin should project near screen center
        let result = project_world_to_screen(&[10.0, 10.0, 0.0], &camera);
        assert!(result.is_some(), "Camera origin should project to screen");

        let (sx, sy) = result.unwrap();
        // Should be near the center (240/2=120, 135/2=67 approximately)
        assert!(
            sx > 50 && sx < 200,
            "Screen X {sx} should be near center"
        );
        assert!(
            sy > 20 && sy < 120,
            "Screen Y {sy} should be near center"
        );
    }

    #[test]
    fn test_pipeline_timing_default() {
        let timing = PipelineTiming::default();
        assert_eq!(timing.total_us, 0);
        assert_eq!(timing.clear_us, 0);
        assert_eq!(timing.terrain_us, 0);
        assert_eq!(timing.resolve_us, 0);
    }

    #[test]
    fn test_pipeline_clears_buffer() {
        // Verify the pipeline clears the buffer (empty scene)
        let _config = RenderConfig {
            ascii_width: 4,
            ascii_height: 4,
        };
        let mut buf = SampleBuffer::new(4, 4);

        // Dirty a sample
        buf.sample_at_mut(5, 5).visual = 0xBEEF;

        // Run clear stage
        let t0 = Instant::now();
        buf.clear();
        let clear_us = t0.elapsed().as_micros() as u64;

        // Verify cleared
        assert_eq!(
            buf.sample_at(5, 5).visual,
            crate::render::sample_buffer::Sample::clear_state().visual
        );
        assert!(clear_us < 1_000_000, "Clear should complete quickly");
    }

    #[test]
    fn test_pipeline_mesh_branch_calls_render_mesh() {
        use crate::asset_loader::akm_mesh::{AkmFace, AkmMesh, AkmVertex};
        use crate::render::sample_buffer::{Sample, spare_bits};
        use std::collections::HashMap;

        // Build a small AkmMesh with 1 triangle.
        // Vertices are placed directly in sample-buffer coordinates via
        // a custom view_tm, so we control exact pixel positions.
        let mesh = AkmMesh {
            vertices: vec![
                AkmVertex { x: 0.0, y: 0.0, z: 0.0, r: 200, g: 100, b: 50, alpha: 255 },
                AkmVertex { x: 1.0, y: 0.0, z: 0.0, r: 200, g: 100, b: 50, alpha: 255 },
                AkmVertex { x: 0.0, y: 1.0, z: 0.0, r: 200, g: 100, b: 50, alpha: 255 },
            ],
            faces: vec![AkmFace {
                indices: [0, 1, 2],
                visual: 0,
                freestyle: false,
            }],
            edges: vec![],
        };

        // Create a MeshRegistry with the mesh loaded under key "test_mesh"
        let mut loaded = HashMap::new();
        loaded.insert("test_mesh".to_string(), mesh);
        let registry = MeshRegistry {
            meshes: HashMap::new(),
            loaded,
        };

        // Create a small SampleBuffer (20x20 ASCII -> 44x44 sample)
        let ascii_w: u32 = 20;
        let ascii_h: u32 = 20;
        let mut buf = SampleBuffer::new(ascii_w, ascii_h);
        let buf_w = buf.width as i32;
        let buf_h = buf.height as i32;

        // Identity instance transform (no model->world transformation)
        let instance_tm: [f64; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        // Custom view_tm that scales world coords into sample-buffer center.
        // The triangle occupies world [0,1]x[0,1], we want it to cover
        // roughly sample-space [10,30]x[10,30] in a 44x44 buffer.
        // view_tm is row-major: screen_x = col0*wx + col1*wy + col2*wz + col3
        // We want: screen_x = 20*wx + 10, screen_y = 20*wy + 10, screen_z = 100
        let view_tm: [f64; 16] = [
            20.0, 0.0,   0.0, 0.0,   // column 0: X scale
            0.0,  20.0,  0.0, 0.0,   // column 1: Y scale
            0.0,  0.0,   0.0, 0.0,   // column 2: Z (flat)
            10.0, 10.0, 100.0, 1.0,  // column 3: translation
        ];

        // Call render_mesh directly (same as pipeline Stage 3 would)
        let akm_mesh = registry.loaded.get("test_mesh").unwrap();
        render_mesh(
            &mut buf.samples,
            buf_w,
            buf_h,
            akm_mesh,
            &instance_tm,
            &view_tm,
        );

        // Verify at least one sample was written by render_mesh.
        // Clear state also has MESH_FLAG (sky-blue), so we distinguish
        // rendered samples by checking height != CLEAR_HEIGHT.
        let rendered_samples = buf
            .samples
            .iter()
            .filter(|s| s.spare == spare_bits::MESH_FLAG && s.height != Sample::CLEAR_HEIGHT)
            .count();
        assert!(
            rendered_samples > 0,
            "render_mesh must write at least one sample (height != CLEAR_HEIGHT), got 0 out of {}",
            buf.samples.len()
        );
    }
}
