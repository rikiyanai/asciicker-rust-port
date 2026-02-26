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
use crate::render::WaterConfig;
use crate::render::assembly::{MeshRegistry, RuntimeMaterials};
use crate::render::camera::GameCamera;
use crate::render::config::RenderConfig;
use crate::render::mesh_shader::render_mesh;
use crate::render::resolve::resolve;
use crate::render::resolve_bridge::{AutoMatGlyphSelector, GlyphSelector, XTERM_256_PALETTE};
use crate::render::sample_buffer::SampleBuffer;
use crate::render::sprite_blit::{SpriteQueue, blit_sprite};
use crate::render::types::AnsiCell;
use crate::render::water;
use crate::terrain::RuntimeTerrain;
use crate::world::RuntimeWorld;

// ---------------------------------------------------------------------------
// PipelineTiming Resource
// ---------------------------------------------------------------------------

/// Per-stage timing data for the rendering pipeline (microseconds).
#[derive(Resource, Default, Debug, Clone)]
#[cfg_attr(
    feature = "inspector",
    derive(bevy_inspector_egui::prelude::InspectorOptions, Reflect)
)]
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
/// Applies architectural perspective division matching C++ render.cpp:1804-1846.
///
/// Returns `Some((screen_x, screen_y))` if the position is in front of the camera,
/// or `None` if behind.
pub fn project_world_to_screen(pos: &[f32; 3], camera: &GameCamera) -> Option<(i32, i32)> {
    let hc = crate::asset_loader::constants::HEIGHT_CELLS as f64;

    // Convert game position to visual-cell units (matching terrain/mesh shaders)
    let wx = pos[0] as f64 * hc;
    let wy = pos[1] as f64 * hc;
    let wz = pos[2] as f64;

    // Eye-to-vertex vector
    let eye_x = wx as f32 - camera.view_pos[0];
    let eye_y = wy as f32 - camera.view_pos[1];
    let eye_z = wz as f32 - camera.view_pos[2];

    // Distance along view direction (view_dir normalized by focal)
    let viewer_dist =
        eye_x * camera.view_dir[0] + eye_y * camera.view_dir[1] + eye_z * camera.view_dir[2];

    if viewer_dist <= 0.0 {
        return None; // behind camera
    }

    let recp_dist = 1.0 / viewer_dist;

    // Base screen position WITHOUT translation, scaled by 1/distance
    let fx = (camera.mul[0] * wx + camera.mul[2] * wy) as f32 * recp_dist;
    let fy = (camera.mul[1] * wx + camera.mul[3] * wy + camera.mul[5] * wz) as f32 * recp_dist;

    // Translated offset with perspective
    let qx = (camera.add[0] as f32 - camera.view_ofs[0]) * recp_dist + camera.view_ofs[0];
    let qy = (camera.add[1] as f32 - camera.view_ofs[1]) * recp_dist + camera.view_ofs[1];

    let sx = fx + qx;
    let sy = fy + qy;

    // Convert from sample-space to ASCII cell coordinates
    let ascii_x = ((sx - 2.0) / 2.0) as i32;
    let ascii_y = ((sy - 2.0) / 2.0) as i32;

    Some((ascii_x, ascii_y))
}

// ---------------------------------------------------------------------------
// Camera terrain initialization
// ---------------------------------------------------------------------------

/// One-shot system: after terrain first loads, set camera z to terrain surface height.
///
/// Without this, the camera starts at z=0 while terrain may be at elevation 40960,
/// causing all terrain to project off-screen. This mimics the C++ behavior where the
/// player spawns ON the terrain surface.
pub fn camera_terrain_init_system(
    mut camera: ResMut<GameCamera>,
    terrain: Res<RuntimeTerrain>,
    config: Res<RenderConfig>,
    mut initialized: Local<bool>,
) {
    if *initialized || terrain.root.is_none() {
        return;
    }
    *initialized = true;

    // Find the patch at camera's (x,y) or nearest patch
    let cam_x = camera.pos[0] as i32;
    let cam_y = camera.pos[1] as i32;

    let height = if let Some(patch) = terrain.get_patch_at(cam_x, cam_y) {
        // Use center height vertex of the patch
        patch.height[2][2]
    } else {
        // Find nearest patch
        let mut best_height = 0u16;
        let mut best_dist = f64::MAX;
        terrain.for_each_patch(|patch| {
            let dx = patch.x as f64 - camera.pos[0] as f64;
            let dy = patch.y as f64 - camera.pos[1] as f64;
            let d = dx * dx + dy * dy;
            if d < best_dist {
                best_dist = d;
                best_height = patch.height[2][2];
            }
        });
        best_height
    };

    camera.pos[2] = height as f32;
    info!(
        "Camera z initialized to terrain height: {} (raw u16) at pos=[{}, {}, {}]",
        height, camera.pos[0], camera.pos[1], camera.pos[2]
    );

    // Recompute view matrix with corrected z position
    let dw = config.sample_width() as f64;
    let dh = config.sample_height() as f64;
    camera.update(dw, dh);
    camera.extract_frustum_planes(dw, dh);
}

// ---------------------------------------------------------------------------
// Pipeline system
// ---------------------------------------------------------------------------

/// The 6-stage rendering pipeline system.
///
/// Runs each frame in PostUpdate (after character sprites are pushed). Stages:
/// 1. CLEAR: memcpy-clear the SampleBuffer
/// 2. TERRAIN: rasterize visible terrain patches (real TerrainShader)
/// 3. WORLD: rasterize visible mesh instances (real MeshShader) + queue sprites
/// 4. SHADOW: stub (future)
/// 5. REFLECTION: re-render below water plane with flipped view matrix
/// 6. RESOLVE: downsample SampleBuffer to AsciiCellGrid with water ripple
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
    water_config: Res<WaterConfig>,
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

    // One-time diagnostic: log camera, terrain, and material info
    {
        static DIAG_LOGGED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if terrain.root.is_some() && !DIAG_LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
            DIAG_LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
            info!(
                "Pipeline: camera pos={:?} yaw={} | terrain patches={} | grid={}x{} sample={}x{}",
                camera.pos,
                camera.yaw,
                terrain.patch_count,
                cell_grid.width,
                cell_grid.height,
                sample_buffer.width,
                sample_buffer.height,
            );
            // Material and height distribution across all patches
            let mut height_min = u16::MAX;
            let mut height_max = 0u16;
            let mut visual_zero = 0u64;
            let mut visual_nonzero = 0u64;
            let mut unique_vis = std::collections::HashSet::new();
            terrain.for_each_patch(|patch| {
                for row in &patch.height {
                    for &h in row {
                        if h < height_min {
                            height_min = h;
                        }
                        if h > height_max {
                            height_max = h;
                        }
                    }
                }
                for row in &patch.visual {
                    for &v in row {
                        if v == 0 {
                            visual_zero += 1;
                        } else {
                            visual_nonzero += 1;
                        }
                        unique_vis.insert(v);
                    }
                }
            });
            info!(
                "Terrain: heights [{},{}] | visual: {} zero, {} nonzero, {} unique indices",
                height_min,
                height_max,
                visual_zero,
                visual_nonzero,
                unique_vis.len()
            );
            // Log first few non-zero material colors
            if let Some(mats) = materials.as_ref() {
                let mut shown = 0;
                for idx in unique_vis.iter().take(8) {
                    if (*idx as usize) < mats.0.len() {
                        let mc = &mats.0[*idx as usize].shade[0][8]; // elevation 0, mid diffuse
                        info!(
                            "Material[{}]: fg={:?} gl={} bg={:?}",
                            idx, mc.fg, mc.gl, mc.bg
                        );
                        shown += 1;
                    }
                }
                if shown == 0 {
                    info!("No material samples to show");
                }
            }
        }
    }

    // STEP 4: Destructure for render stages (field borrows)
    // Read width/height BEFORE taking &mut samples to avoid borrow conflict.
    let buf_w = sample_buffer.width as i32;
    let buf_h = sample_buffer.height as i32;
    {
        let buf = &mut sample_buffer.samples;

        // Stage 2: TERRAIN (real rasterization with frustum culling)
        let t1 = Instant::now();
        let mut _terrain_patch_count = 0u32;
        if terrain.root.is_some() {
            terrain.query_visible(&camera.frustum_planes, |patch| {
                _terrain_patch_count += 1;
                let wz = if water_config.water_z > f32::NEG_INFINITY {
                    Some(water_config.water_z)
                } else {
                    None
                };
                crate::render::terrain_shader::render_patch(
                    buf, buf_w, buf_h, patch, patch.x, patch.y, &camera, wz,
                );
            });
        }
        timing.terrain_us = t1.elapsed().as_micros() as u64;

        // Stage 3: WORLD (real rasterization for meshes, placeholder for sprites)
        // Cleared by clear_sprite_queue_system in PreUpdate (Phase 6)
        let t2 = Instant::now();

        // TODO(frustum): BSP frustum culling disabled -- coordinate system mismatch
        // between camera-pos-space frustum planes (game units) and visual-cell-space
        // BSP bounding boxes (instance tm positions). Same issue as terrain.
        // Fix: convert frustum planes to visual-cell units or convert bbox to game units.
        // For now, iterate ALL instances directly (matching terrain approach).
        for inst in world_data.instances.iter() {
            match inst {
                crate::world::instance::RuntimeInstance::Mesh { mesh_id, tm, .. } => {
                    if inst.is_visible()
                        && let Some(mesh) = mesh_registry.loaded.get(mesh_id)
                    {
                        render_mesh(buf, buf_w, buf_h, mesh, tm, &camera);
                    }
                }
                crate::world::instance::RuntimeInstance::Sprite {
                    sprite_name,
                    pos,
                    yaw,
                    anim,
                    frame,
                    ..
                } => {
                    if inst.is_visible() {
                        let dx = pos[0] - camera.view_pos[0];
                        let dy = pos[1] - camera.view_pos[1];
                        let dist = dx * camera.view_dir[0] + dy * camera.view_dir[1];

                        if let Some((sx, sy)) = project_world_to_screen(pos, &camera) {
                            sprite_queue.push(crate::render::sprite_blit::SpriteRenderEntry {
                                dist,
                                screen_x: sx,
                                screen_y: sy,
                                sprite_name: sprite_name.clone(),
                                pos: *pos,
                                yaw: *yaw,
                                anim: *anim as u32,
                                frame: *frame as u32,
                            });
                        }
                    }
                }
                crate::world::instance::RuntimeInstance::Item { pos, yaw, .. } => {
                    if inst.is_visible() {
                        let dx = pos[0] - camera.view_pos[0];
                        let dy = pos[1] - camera.view_pos[1];
                        let dist = dx * camera.view_dir[0] + dy * camera.view_dir[1];

                        if let Some((sx, sy)) = project_world_to_screen(pos, &camera) {
                            sprite_queue.push(crate::render::sprite_blit::SpriteRenderEntry {
                                dist,
                                screen_x: sx,
                                screen_y: sy,
                                sprite_name: "item".to_string(),
                                pos: *pos,
                                yaw: *yaw,
                                anim: 0,
                                frame: 0,
                            });
                        }
                    }
                }
            }
        }
        timing.world_us = t2.elapsed().as_micros() as u64;

        // Stage 4: SHADOW (stub -- future)
        let t3 = Instant::now();
        timing.shadow_us = t3.elapsed().as_micros() as u64;
    } // mutable borrow of sample_buffer.samples ENDS here

    // Stage 5: REFLECTION (water)
    let t4 = Instant::now();
    if water_config.water_z > f32::NEG_INFINITY {
        water::render_water_reflections(
            &mut sample_buffer,
            &terrain,
            &world_data,
            &camera,
            water_config.water_z,
        );
    }
    timing.reflection_us = t4.elapsed().as_micros() as u64;

    // F242 diagnostic: log water state for first frame
    {
        static WATER_DIAG: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if terrain.root.is_some() && !WATER_DIAG.load(std::sync::atomic::Ordering::Relaxed) {
            WATER_DIAG.store(true, std::sync::atomic::Ordering::Relaxed);
            let underwater = sample_buffer
                .samples
                .iter()
                .filter(|s| {
                    s.height > crate::render::sample_buffer::Sample::CLEAR_HEIGHT
                        && s.height < water_config.water_z
                })
                .count();
            let reflected = sample_buffer
                .samples
                .iter()
                .filter(|s| {
                    s.spare & crate::render::sample_buffer::spare_bits::PARITY_MASK
                        == crate::render::sample_buffer::spare_bits::REFLECTION
                })
                .count();
            info!(
                "F242 water: water_z={} camera_z={} underwater_samples={} reflected_samples={} reflection_us={}",
                water_config.water_z,
                camera.pos[2],
                underwater,
                reflected,
                timing.reflection_us,
            );
        }
    }

    // STEP 5: Stage 6 RESOLVE
    // R19-F02/F09 FIX: 3-step split -- resolve -> water ripple -> RGBA conversion.
    // DO NOT call resolve_to_grid as single function (ripple must run between
    // resolve and RGBA conversion, in the palette-index domain).
    let t5 = Instant::now();
    if let Some(mats) = materials.as_ref() {
        let ascii_w = config.ascii_width as usize;
        let ascii_h = config.ascii_height as usize;
        let dw = sample_buffer.width as i32;
        let dh = sample_buffer.height as i32;
        let mut resolve_buf = vec![AnsiCell::default(); ascii_w * ascii_h];

        // Step 1: resolve() fills resolve_buf with xterm-256 palette AnsiCells
        resolve(
            &sample_buffer.samples,
            dw,
            dh,
            ascii_w as i32,
            ascii_h as i32,
            &mats.0,
            &mut resolve_buf,
        );

        // Step 2: Water ripple modifies palette indices BEFORE RGBA conversion
        if water_config.water_z > f32::NEG_INFINITY {
            water::apply_water_ripple_pass(
                &sample_buffer.samples,
                &mut resolve_buf,
                ascii_w as i32,
                ascii_h as i32,
                water_config.ripple_time,
            );
        }

        // Step 3: Glyph selection + RGBA conversion to cell_grid
        // Dimming for reflected cells is already applied in resolve_material() (255/400).
        // Perlin ripple (Step 2) provides water animation.
        let mut glyph_sel = AutoMatGlyphSelector;
        for cy in 0..ascii_h {
            for cx in 0..ascii_w {
                let i = cy * ascii_w + cx;
                let cell = &resolve_buf[i];
                let gl = match glyph_sel.select_glyph(&sample_buffer, cx, cy) {
                    Some(glyph) => glyph,
                    None => cell.gl,
                };
                let fg_rgb = XTERM_256_PALETTE[cell.fg as usize];
                let bk_rgb = XTERM_256_PALETTE[cell.bk as usize];

                cell_grid.char_indices[i] = gl as u16;
                cell_grid.fg_colors[i] = [fg_rgb[0], fg_rgb[1], fg_rgb[2], 255];
                cell_grid.bg_colors[i] = [bk_rgb[0], bk_rgb[1], bk_rgb[2], 255];
            }
        }
    }
    timing.resolve_us = t5.elapsed().as_micros() as u64;

    // Diagnostic: log resolve stats for first 3 frames
    {
        static RESOLVE_FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let frame = RESOLVE_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if materials.is_some() && frame < 3 {
            let non_clear = sample_buffer
                .samples
                .iter()
                .filter(|s| s.height != crate::render::sample_buffer::Sample::CLEAR_HEIGHT)
                .count();
            let mesh_samples = sample_buffer
                .samples
                .iter()
                .filter(|s| {
                    s.height != crate::render::sample_buffer::Sample::CLEAR_HEIGHT
                        && s.spare & crate::render::sample_buffer::spare_bits::MESH_FLAG != 0
                })
                .count();
            let non_space = cell_grid
                .char_indices
                .iter()
                .filter(|&&c| c != 0 && c != 32)
                .count();
            info!(
                "Resolve[frame {}]: {}/{} samples filled ({} mesh), {}/{} glyphs visible, {} meshes loaded",
                frame,
                non_clear,
                sample_buffer.samples.len(),
                mesh_samples,
                non_space,
                cell_grid.char_indices.len(),
                mesh_registry.loaded.len(),
            );
        }
    }

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
        assert!(sx > 50 && sx < 200, "Screen X {sx} should be near center");
        assert!(sy > 20 && sy < 120, "Screen Y {sy} should be near center");
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
        use crate::render::camera::GameCamera;
        use crate::render::sample_buffer::{Sample, spare_bits};
        use std::collections::HashMap;

        // Build a small AkmMesh with 1 triangle.
        // Vertices are placed directly in sample-buffer coordinates via
        // a custom view_tm, so we control exact pixel positions.
        let mesh = AkmMesh {
            vertices: vec![
                AkmVertex {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    r: 200,
                    g: 100,
                    b: 50,
                    alpha: 255,
                },
                AkmVertex {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                    r: 200,
                    g: 100,
                    b: 50,
                    alpha: 255,
                },
                AkmVertex {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    r: 200,
                    g: 100,
                    b: 50,
                    alpha: 255,
                },
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

        // Larger buffer so the perspective-projected mesh fits comfortably.
        let ascii_w: u32 = 200;
        let ascii_h: u32 = 200;
        let mut buf = SampleBuffer::new(ascii_w, ascii_h);
        let buf_w = buf.width as i32;
        let buf_h = buf.height as i32;

        // Scale mesh up: vertices (0,0,0),(1,0,0),(0,1,0) -> (10,10,0),(110,10,0),(10,110,0)
        let instance_tm: [f64; 16] = [
            100.0, 0.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.0, 100.0, 0.0, 10.0, 10.0, 0.0, 1.0,
        ];

        // Camera behind the mesh looking toward +Y (yaw=0)
        let mut camera = GameCamera::default();
        camera.pos = [0.0, -20.0, 0.0];
        camera.yaw = 0.0;
        camera.update(buf_w as f64, buf_h as f64);

        // Call render_mesh directly (same as pipeline Stage 3 would)
        let akm_mesh = registry.loaded.get("test_mesh").unwrap();
        render_mesh(
            &mut buf.samples,
            buf_w,
            buf_h,
            akm_mesh,
            &instance_tm,
            &camera,
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
