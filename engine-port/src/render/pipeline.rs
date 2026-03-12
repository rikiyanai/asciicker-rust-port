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
use crate::render::debug_cells::{RenderDebugGrid, debug_flags};
use crate::render::mesh_shader::render_mesh;
use crate::render::quantize::rgb2pal;
use crate::render::resolve::resolve_with_debug;
use crate::render::resolve_bridge::{AutoMatGlyphSelector, GlyphSelector, XTERM_256_PALETTE};
use crate::render::sample_buffer::SampleBuffer;
use crate::render::shape_vector::{
    ShapeVectorAlphabetRegistry, ShapeVectorConfig, ShapeVectorDecision, ShapeVectorFrameStats,
    ShapeVectorGlyphSelector, ShapeVectorMatcher, ShapeVectorMode, ShapeVectorSkipReason,
    optimize_glyph_colors,
};
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

fn mat4_mul_vec4(m: &[f64; 16], v: [f64; 4]) -> [f64; 4] {
    [
        m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12] * v[3],
        m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13] * v[3],
        m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14] * v[3],
        m[3] * v[0] + m[7] * v[1] + m[11] * v[2] + m[15] * v[3],
    ]
}

fn pack_rgb888_to_rgb555(rgb: [u8; 3]) -> u16 {
    let r = ((rgb[0] as u32 * 249 + 1014) >> 11) as u16;
    let g = ((rgb[1] as u32 * 249 + 1014) >> 11) as u16;
    let b = ((rgb[2] as u32 * 249 + 1014) >> 11) as u16;
    r | (g << 5) | (b << 10)
}

fn palette_contrast(fg_rgb: [u8; 3], bg_rgb: [u8; 3]) -> u16 {
    u16::from(fg_rgb[0].abs_diff(bg_rgb[0]))
        + u16::from(fg_rgb[1].abs_diff(bg_rgb[1]))
        + u16::from(fg_rgb[2].abs_diff(bg_rgb[2]))
}

fn choose_structural_fallback_glyph(
    decision: &ShapeVectorDecision,
    resolve_glyph: u8,
    fg_rgb: [u8; 3],
    bg_rgb: [u8; 3],
    config: &ShapeVectorConfig,
) -> Option<u8> {
    if !config.enable_structural_fallback || resolve_glyph != b' ' {
        return None;
    }
    if decision.skip_reason != Some(ShapeVectorSkipReason::DistanceThreshold) {
        return None;
    }
    let distance = decision.distance?;
    if distance > config.structural_fallback_distance_threshold {
        return None;
    }
    if palette_contrast(fg_rgb, bg_rgb) < config.structural_fallback_contrast_threshold {
        return None;
    }
    decision.candidate_glyph
}

fn choose_material_structural_fallback(
    sample_buffer: &SampleBuffer,
    materials: &[crate::render::material::Material],
    cell_x: usize,
    cell_y: usize,
) -> Option<u8> {
    let sx = (2 + 2 * cell_x) as u32;
    let sy = (2 + 2 * cell_y) as u32;
    let samples = [
        sample_buffer.sample_at(sx, sy),
        sample_buffer.sample_at(sx + 1, sy),
        sample_buffer.sample_at(sx, sy + 1),
        sample_buffer.sample_at(sx + 1, sy + 1),
    ];

    let mut dominant: Option<&crate::render::sample_buffer::Sample> = None;
    for sample in samples {
        if sample.height == crate::render::sample_buffer::Sample::CLEAR_HEIGHT {
            return None;
        }
        if sample.spare & crate::render::sample_buffer::spare_bits::MESH_FLAG != 0 {
            return None;
        }
        match dominant {
            Some(prev) if prev.height >= sample.height => {}
            _ => dominant = Some(sample),
        }
    }

    let sample = dominant?;
    let mat_idx = (sample.visual & 0x00FF) as usize;
    let elevation = structural_fallback_elevation(sample_buffer, sx, sy);
    let glyph = materials.get(mat_idx)?.lookup(elevation, sample.diffuse).gl;
    (glyph != b' ').then_some(glyph)
}

fn structural_fallback_elevation(sample_buffer: &SampleBuffer, sx: u32, sy: u32) -> u8 {
    let bit15 = |x: u32, y: u32| -> i32 {
        let sample = sample_buffer.sample_at(x, y);
        ((sample.visual >> 15) & 1) as i32
    };

    let e_lo = bit15(sx, sy - 1) + bit15(sx + 1, sy - 1);
    let e_hi = bit15(sx, sy + 1) + bit15(sx + 1, sy + 1);

    if e_lo <= 1 {
        if e_hi <= 1 { 3 } else { 2 }
    } else if e_hi <= 1 {
        0
    } else {
        1
    }
}

fn should_preserve_resolve_glyph(
    decision: &ShapeVectorDecision,
    resolve_glyph: u8,
    fg_rgb: [u8; 3],
    bg_rgb: [u8; 3],
    config: &ShapeVectorConfig,
) -> bool {
    if !config.enable_structural_fallback || resolve_glyph == b' ' {
        return false;
    }
    if decision.skip_reason.is_some() {
        return false;
    }
    let contrast = palette_contrast(fg_rgb, bg_rgb);
    if contrast < config.structural_fallback_contrast_threshold {
        return false;
    }

    if decision.glyph == Some(b' ') {
        return true;
    }

    if let (Some(glyph), Some(distance)) = (decision.glyph, decision.distance)
        && glyph != resolve_glyph
        && distance > config.distance_threshold * 0.5
    {
        return true;
    }

    false
}

fn is_shape_vector_semantic_gate_cell(resolve_cell: &AnsiCell) -> bool {
    resolve_cell.spare == 0xFE
}

fn choose_final_glyph(
    sample_buffer: &SampleBuffer,
    materials: &[crate::render::material::Material],
    cell_x: usize,
    cell_y: usize,
    decision: &ShapeVectorDecision,
    resolve_glyph: u8,
    fg_rgb: [u8; 3],
    bg_rgb: [u8; 3],
    config: &ShapeVectorConfig,
    semantic_gate: bool,
) -> (u8, bool, bool) {
    if config.mode == ShapeVectorMode::OriginalOnly {
        return (resolve_glyph, false, true);
    }

    if config.mode == ShapeVectorMode::HarriPriority {
        return (decision.glyph.unwrap_or(resolve_glyph), false, false);
    }

    if semantic_gate {
        return (resolve_glyph, false, true);
    }

    if should_preserve_resolve_glyph(decision, resolve_glyph, fg_rgb, bg_rgb, config) {
        return (resolve_glyph, true, false);
    }

    let glyph = choose_structural_fallback_glyph(decision, resolve_glyph, fg_rgb, bg_rgb, config)
        .or_else(|| {
            if decision.skip_reason == Some(ShapeVectorSkipReason::DistanceThreshold)
                && resolve_glyph == b' '
                && palette_contrast(fg_rgb, bg_rgb) >= config.structural_fallback_contrast_threshold
            {
                choose_material_structural_fallback(sample_buffer, materials, cell_x, cell_y)
            } else {
                None
            }
        })
        .unwrap_or_else(|| decision.glyph.unwrap_or(resolve_glyph));
    (glyph, false, false)
}

fn choose_final_colors(
    sample_buffer: &SampleBuffer,
    materials: &[crate::render::material::Material],
    cell_x: usize,
    cell_y: usize,
    decision: &ShapeVectorDecision,
    resolve_glyph: u8,
    final_glyph: u8,
    resolve_fg_rgb: [u8; 3],
    resolve_bg_rgb: [u8; 3],
    preserved_resolve: bool,
    semantic_gate: bool,
) -> ([u8; 3], [u8; 3]) {
    if preserved_resolve || semantic_gate {
        return (resolve_fg_rgb, resolve_bg_rgb);
    }

    let Some(selected_glyph) = decision.glyph else {
        return (resolve_fg_rgb, resolve_bg_rgb);
    };
    if selected_glyph != final_glyph || final_glyph == resolve_glyph {
        return (resolve_fg_rgb, resolve_bg_rgb);
    }

    let Some((opt_fg, opt_bg)) =
        optimize_glyph_colors(sample_buffer, materials, cell_x, cell_y, final_glyph)
    else {
        return (resolve_fg_rgb, resolve_bg_rgb);
    };

    let fg_pal = rgb2pal(opt_fg);
    let bg_pal = rgb2pal(opt_bg);
    if fg_pal == bg_pal {
        return (resolve_fg_rgb, resolve_bg_rgb);
    }

    (
        XTERM_256_PALETTE[fg_pal as usize],
        XTERM_256_PALETTE[bg_pal as usize],
    )
}

fn apply_player_shadow(
    samples: &mut [crate::render::sample_buffer::Sample],
    dw: i32,
    dh: i32,
    ascii_width: u32,
    camera: &GameCamera,
    materials: &[crate::render::material::Material],
) {
    let shadow_center_x = ascii_width as i32 + 1 + camera.scene_shift[0] * 2;
    let left = (shadow_center_x - 5).max(0);
    let right = (shadow_center_x + 5).min(dw - 1);
    let player_z = camera.pos[2];
    let hc = crate::asset_loader::constants::HEIGHT_CELLS as f64;

    for y in 0..dh {
        for x in left..=right {
            let idx = (x + y * dw) as usize;
            let sample = &mut samples[idx];
            if (sample.height - player_z).abs() > 64.0 {
                continue;
            }

            let world = mat4_mul_vec4(
                &camera.inv_tm,
                [x as f64, y as f64, sample.height as f64, 1.0],
            );
            let dx = world[0] / hc - camera.pos[0] as f64;
            let dy = world[1] / hc - camera.pos[1] as f64;
            let sq_xy = dx * dx + dy * dy;
            if sq_xy > 2.0 {
                continue;
            }

            let mut dz = (2.0 * (player_z as f64 - sample.height as f64) + 2.0 * sq_xy) as i32;
            if dz < 180 {
                dz = 180;
            }
            if dz > 180 {
                dz = 255;
            }

            if sample.spare & crate::render::sample_buffer::spare_bits::MESH_FLAG != 0 {
                sample.diffuse = ((sample.diffuse as u16 * dz as u16) / 255) as u8;
                continue;
            }

            let mat_idx = (sample.visual & 0x00FF) as usize;
            let Some(material) = materials.get(mat_idx) else {
                continue;
            };
            let bg_rgb = material.lookup(1, sample.diffuse).bg;
            sample.visual = pack_rgb888_to_rgb555(bg_rgb);
            sample.spare |= crate::render::sample_buffer::spare_bits::MESH_FLAG;
            sample.spare &= !(crate::render::sample_buffer::spare_bits::GRID
                | crate::render::sample_buffer::spare_bits::WIREFRAME);
            sample.diffuse = dz as u8;
        }
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
    shape_vector_config: Res<ShapeVectorConfig>,
    shape_vector_alphabets: Res<ShapeVectorAlphabetRegistry>,
    shape_vec_matcher: Option<ResMut<ShapeVectorMatcher>>,
    mut shape_vector_stats: ResMut<ShapeVectorFrameStats>,
    mut debug_grid: ResMut<RenderDebugGrid>,
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
                    buf, buf_w, buf_h, patch, patch.x, patch.y, &camera, wz, true,
                );
            });
        }
        timing.terrain_us = t1.elapsed().as_micros() as u64;

        // Stage 3: WORLD (real rasterization for meshes, placeholder for sprites)
        // Cleared by clear_sprite_queue_system in PreUpdate (Phase 6)
        let t2 = Instant::now();

        let visible_instances = world_data.query_visible(
            &camera.frustum_planes,
            [
                camera.pos[0] as f64,
                camera.pos[1] as f64,
                camera.pos[2] as f64,
            ],
        );
        for visible in visible_instances {
            match visible {
                crate::world::bsp::VisibleInstance::Mesh(id) => {
                    let Some(crate::world::instance::RuntimeInstance::Mesh { mesh_id, tm, .. }) =
                        world_data.instances.get(id.0)
                    else {
                        continue;
                    };
                    if let Some(mesh) = mesh_registry.loaded.get(mesh_id) {
                        render_mesh(
                            buf,
                            buf_w,
                            buf_h,
                            mesh,
                            tm,
                            &camera,
                            if water_config.water_z > f32::NEG_INFINITY {
                                Some(water_config.water_z)
                            } else {
                                None
                            },
                            false,
                        );
                    }
                }
                crate::world::bsp::VisibleInstance::Sprite(id) => {
                    let Some(inst) = world_data.instances.get(id.0) else {
                        continue;
                    };
                    match inst {
                        crate::world::instance::RuntimeInstance::Sprite {
                            sprite_name,
                            pos,
                            yaw,
                            anim,
                            frame,
                            ..
                        } => {
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
                        crate::world::instance::RuntimeInstance::Item { pos, yaw, .. } => {
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
                        crate::world::instance::RuntimeInstance::Mesh { .. } => {}
                    }
                }
            }
        }
        timing.world_us = t2.elapsed().as_micros() as u64;

        // Stage 4: SHADOW (player blob shadow, ported from C++ Stage 4)
        let t3 = Instant::now();
        if let Some(mats) = materials.as_ref() {
            apply_player_shadow(buf, buf_w, buf_h, config.ascii_width, &camera, &mats.0);
        }
        timing.shadow_us = t3.elapsed().as_micros() as u64;
    } // mutable borrow of sample_buffer.samples ENDS here

    // Stage 5: REFLECTION (water)
    let t4 = Instant::now();
    if water_config.water_z > f32::NEG_INFINITY {
        water::render_water_reflections(
            &mut sample_buffer,
            &terrain,
            &world_data,
            &mesh_registry,
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
                water_config.water_z, camera.pos[2], underwater, reflected, timing.reflection_us,
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
        debug_grid.ensure_size(ascii_w as u32, ascii_h as u32);
        shape_vector_stats.begin_frame(ascii_w * ascii_h);

        // Step 1: resolve() fills resolve_buf with xterm-256 palette AnsiCells
        resolve_with_debug(
            &sample_buffer.samples,
            dw,
            dh,
            ascii_w as i32,
            ascii_h as i32,
            &mats.0,
            &mut resolve_buf,
            &mut debug_grid.cells,
            water_config.water_z,
        );

        // Step 2: Water ripple modifies palette indices BEFORE RGBA conversion
        if water_config.water_z > f32::NEG_INFINITY {
            water::apply_water_ripple_pass(
                &sample_buffer.samples,
                &mut resolve_buf,
                ascii_w as i32,
                ascii_h as i32,
                water_config.ripple_time,
                Some(&mut debug_grid.cells),
            );
        }

        // Step 3: Glyph selection + RGBA conversion to cell_grid
        // R7-CRIT-001 FIX: Use ShapeVectorGlyphSelector when matcher is available,
        // otherwise fall back to AutoMatGlyphSelector.
        // R8-XP-001 FIX: Water ripple (Step 2) is PRESERVED above — ordering correct.
        // R7-XP-005 FIX: &mut sample_buffer.samples borrow DROPPED before this block.
        // R20-F01: This is the inlined 3-step loop, NOT resolve_to_grid.

        if let Some(mut matcher) = shape_vec_matcher {
            if matcher.active_alphabet() != shape_vector_config.alphabet {
                matcher.rebuild_from_alphabet(
                    shape_vector_config.alphabet,
                    shape_vector_alphabets.get(shape_vector_config.alphabet),
                );
            }
            let mut shape_sel = ShapeVectorGlyphSelector {
                alphabet: shape_vector_alphabets.get(shape_vector_config.alphabet),
                matcher: &mut matcher,
                materials: &mats.0,
                water_z: water_config.water_z,
                distance_threshold: shape_vector_config.distance_threshold,
                global_crunch_exponent: shape_vector_config.global_crunch_exponent,
                directional_crunch_exponent: shape_vector_config.directional_crunch_exponent,
                sampling_quality: shape_vector_config.sampling_quality,
                enable_global_crunch: shape_vector_config.enable_global_crunch,
                enable_directional_crunch: shape_vector_config.enable_directional_crunch,
                contrast_adaptive_threshold_boost: shape_vector_config
                    .contrast_adaptive_threshold_boost,
                enable_contrast_adaptive_threshold: shape_vector_config
                    .enable_contrast_adaptive_threshold,
            };
            for cy in 0..ascii_h {
                for cx in 0..ascii_w {
                    let i = cy * ascii_w + cx;
                    let cell = &resolve_buf[i];
                    let semantic_gate =
                        matches!(shape_vector_config.mode, ShapeVectorMode::Combined)
                            && is_shape_vector_semantic_gate_cell(cell);
                    let decision = if shape_vector_config.mode == ShapeVectorMode::OriginalOnly
                        || semantic_gate
                    {
                        ShapeVectorDecision::resolve_fallback()
                    } else {
                        shape_sel.select_glyph_with_debug(&sample_buffer, cx, cy)
                    };
                    let resolve_fg_rgb = XTERM_256_PALETTE[cell.fg as usize];
                    let resolve_bk_rgb = XTERM_256_PALETTE[cell.bk as usize];
                    let (gl, preserved_resolve, semantic_gate) = choose_final_glyph(
                        &sample_buffer,
                        &mats.0,
                        cx,
                        cy,
                        &decision,
                        cell.gl,
                        resolve_fg_rgb,
                        resolve_bk_rgb,
                        &shape_vector_config,
                        semantic_gate,
                    );
                    let (fg_rgb, bk_rgb) = choose_final_colors(
                        &sample_buffer,
                        &mats.0,
                        cx,
                        cy,
                        &decision,
                        cell.gl,
                        gl,
                        resolve_fg_rgb,
                        resolve_bk_rgb,
                        preserved_resolve,
                        semantic_gate,
                    );
                    if gl != cell.gl {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_VECTOR_OVERRIDE;
                    }
                    if preserved_resolve {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_PRESERVED_RESOLVE;
                    }
                    if semantic_gate {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_GATED_SEMANTIC;
                        shape_vector_stats.semantic_gate_cells += 1;
                    }

                    shape_vector_stats.note_selection(decision, cell.gl, gl, fg_rgb, bk_rgb);

                    match decision.skip_reason {
                        Some(crate::render::shape_vector::ShapeVectorSkipReason::Clear) => {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_SKIP_CLEAR;
                        }
                        Some(crate::render::shape_vector::ShapeVectorSkipReason::Underwater) => {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_SKIP_UNDERWATER;
                        }
                        Some(
                            crate::render::shape_vector::ShapeVectorSkipReason::DistanceThreshold,
                        ) => {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_SKIP_THRESHOLD;
                        }
                        None => {}
                    }
                    if decision.glyph.is_none() {
                        if gl == b' ' {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_FALLBACK_SPACE;
                        } else {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_FALLBACK_STRUCTURAL;
                        }
                    }
                    if gl == b' ' && (bk_rgb != [0, 0, 0] || fg_rgb != bk_rgb) {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_COLORED_SPACE;
                    }
                    debug_grid.cells[i].shape_distance = decision.distance.unwrap_or(0.0);
                    debug_grid.cells[i].resolve_glyph = cell.gl as u16;
                    debug_grid.cells[i].final_glyph = gl as u16;

                    cell_grid.char_indices[i] = gl as u16;
                    cell_grid.fg_colors[i] = [fg_rgb[0], fg_rgb[1], fg_rgb[2], 255];
                    cell_grid.bg_colors[i] = [bk_rgb[0], bk_rgb[1], bk_rgb[2], 255];
                }
            }
        } else {
            let mut auto_sel = AutoMatGlyphSelector;
            for cy in 0..ascii_h {
                for cx in 0..ascii_w {
                    let i = cy * ascii_w + cx;
                    let cell = &resolve_buf[i];
                    let resolve_fg_rgb = XTERM_256_PALETTE[cell.fg as usize];
                    let resolve_bk_rgb = XTERM_256_PALETTE[cell.bk as usize];
                    let semantic_gate =
                        matches!(shape_vector_config.mode, ShapeVectorMode::Combined)
                            && is_shape_vector_semantic_gate_cell(cell);
                    let decision = if shape_vector_config.mode == ShapeVectorMode::OriginalOnly
                        || semantic_gate
                    {
                        ShapeVectorDecision::resolve_fallback()
                    } else {
                        match auto_sel.select_glyph(&sample_buffer, cx, cy) {
                            Some(glyph) => ShapeVectorDecision {
                                glyph: Some(glyph),
                                ..ShapeVectorDecision::default()
                            },
                            None => ShapeVectorDecision::resolve_fallback(),
                        }
                    };
                    let (gl, preserved_resolve, semantic_gate) = choose_final_glyph(
                        &sample_buffer,
                        &mats.0,
                        cx,
                        cy,
                        &decision,
                        cell.gl,
                        resolve_fg_rgb,
                        resolve_bk_rgb,
                        &shape_vector_config,
                        semantic_gate,
                    );
                    let (fg_rgb, bk_rgb) = choose_final_colors(
                        &sample_buffer,
                        &mats.0,
                        cx,
                        cy,
                        &decision,
                        cell.gl,
                        gl,
                        resolve_fg_rgb,
                        resolve_bk_rgb,
                        preserved_resolve,
                        semantic_gate,
                    );
                    if gl != cell.gl {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_VECTOR_OVERRIDE;
                    }
                    if preserved_resolve {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_PRESERVED_RESOLVE;
                    }
                    if semantic_gate {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_GATED_SEMANTIC;
                        shape_vector_stats.semantic_gate_cells += 1;
                    }

                    shape_vector_stats.note_selection(decision, cell.gl, gl, fg_rgb, bk_rgb);

                    if decision.glyph.is_none() {
                        if gl == b' ' {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_FALLBACK_SPACE;
                        } else {
                            debug_grid.cells[i].flags |= debug_flags::SHAPE_FALLBACK_STRUCTURAL;
                        }
                    }
                    if gl == b' ' && (bk_rgb != [0, 0, 0] || fg_rgb != bk_rgb) {
                        debug_grid.cells[i].flags |= debug_flags::SHAPE_COLORED_SPACE;
                    }
                    debug_grid.cells[i].shape_distance = decision.distance.unwrap_or(0.0);
                    debug_grid.cells[i].resolve_glyph = cell.gl as u16;
                    debug_grid.cells[i].final_glyph = gl as u16;

                    cell_grid.char_indices[i] = gl as u16;
                    cell_grid.fg_colors[i] = [fg_rgb[0], fg_rgb[1], fg_rgb[2], 255];
                    cell_grid.bg_colors[i] = [bk_rgb[0], bk_rgb[1], bk_rgb[2], 255];
                }
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
                "Resolve[frame {}]: {}/{} samples filled ({} mesh), {}/{} glyphs visible, {} meshes loaded | sv ov={} fb={} thr={} clr={} uw={} blank={} colored_blank={} avg_d={:.4} rej_d={:.4}",
                frame,
                non_clear,
                sample_buffer.samples.len(),
                mesh_samples,
                non_space,
                cell_grid.char_indices.len(),
                mesh_registry.loaded.len(),
                shape_vector_stats.selector_override_cells,
                shape_vector_stats.resolve_fallback_cells,
                shape_vector_stats.threshold_skip_cells,
                shape_vector_stats.clear_skip_cells,
                shape_vector_stats.underwater_skip_cells,
                shape_vector_stats.final_space_cells,
                shape_vector_stats.colored_space_cells,
                shape_vector_stats.avg_matched_distance(),
                shape_vector_stats.avg_threshold_distance(),
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
    use crate::render::material::test_materials;

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
    fn test_apply_player_shadow_darkens_mesh_diffuse() {
        let materials = test_materials();
        let config = RenderConfig {
            ascii_width: 10,
            ascii_height: 8,
        };
        let mut camera = GameCamera::default();
        let dw = config.sample_width() as f64;
        let dh = config.sample_height() as f64;
        camera.update(dw, dh);
        camera.extract_frustum_planes(dw, dh);

        let mut buf = SampleBuffer::new(config.ascii_width, config.ascii_height);
        let sx = config.ascii_width + 1;
        let sy = config.sample_height() / 2;
        *buf.sample_at_mut(sx, sy) = crate::render::sample_buffer::Sample {
            visual: 0x7FFF,
            diffuse: 255,
            spare: crate::render::sample_buffer::spare_bits::MESH_FLAG,
            height: camera.pos[2],
        };

        apply_player_shadow(
            &mut buf.samples,
            buf.width as i32,
            buf.height as i32,
            config.ascii_width,
            &camera,
            &materials,
        );

        assert!(
            buf.sample_at(sx, sy).diffuse < 255,
            "Shadowed mesh sample should have reduced diffuse"
        );
    }

    #[test]
    fn test_apply_player_shadow_converts_terrain_to_rgb_mesh() {
        let materials = test_materials();
        let config = RenderConfig {
            ascii_width: 10,
            ascii_height: 8,
        };
        let mut camera = GameCamera::default();
        let dw = config.sample_width() as f64;
        let dh = config.sample_height() as f64;
        camera.update(dw, dh);
        camera.extract_frustum_planes(dw, dh);

        let mut buf = SampleBuffer::new(config.ascii_width, config.ascii_height);
        let sx = config.ascii_width + 1;
        let sy = config.sample_height() / 2;
        *buf.sample_at_mut(sx, sy) = crate::render::sample_buffer::Sample {
            visual: 1,
            diffuse: 200,
            spare: crate::render::sample_buffer::spare_bits::GRID
                | crate::render::sample_buffer::spare_bits::WIREFRAME,
            height: camera.pos[2],
        };

        apply_player_shadow(
            &mut buf.samples,
            buf.width as i32,
            buf.height as i32,
            config.ascii_width,
            &camera,
            &materials,
        );

        let sample = buf.sample_at(sx, sy);
        assert_ne!(sample.visual, 1);
        assert_ne!(
            sample.spare & crate::render::sample_buffer::spare_bits::MESH_FLAG,
            0
        );
        assert_eq!(
            sample.spare
                & (crate::render::sample_buffer::spare_bits::GRID
                    | crate::render::sample_buffer::spare_bits::WIREFRAME),
            0
        );
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
            None,
            false,
        );

        // Verify at least one sample was written by render_mesh.
        // Clear state also has MESH_FLAG (sky-blue), so we distinguish
        // rendered samples by checking height != CLEAR_HEIGHT.
        let rendered_samples = buf
            .samples
            .iter()
            .filter(|s| s.height != Sample::CLEAR_HEIGHT && (s.spare & spare_bits::MESH_FLAG) != 0)
            .count();
        assert!(
            rendered_samples > 0,
            "render_mesh must write at least one sample (height != CLEAR_HEIGHT), got 0 out of {}",
            buf.samples.len()
        );
    }

    #[test]
    fn test_choose_final_glyph_preserves_structural_resolve_when_match_is_space() {
        let config = ShapeVectorConfig::default();
        let decision = ShapeVectorDecision {
            glyph: Some(b' '),
            candidate_glyph: Some(b' '),
            distance: Some(0.0008),
            skip_reason: None,
        };

        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            b',',
            [255, 255, 153],
            [0, 102, 0],
            &config,
            false,
        );

        assert_eq!(glyph, b',');
        assert!(preserved);
        assert!(!gated);
    }

    #[test]
    fn test_choose_final_glyph_uses_candidate_for_threshold_space_fallback() {
        let config = ShapeVectorConfig::default();
        let decision = ShapeVectorDecision {
            glyph: None,
            candidate_glyph: Some(b'#'),
            distance: Some(0.12),
            skip_reason: Some(ShapeVectorSkipReason::DistanceThreshold),
        };

        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            b' ',
            [255, 255, 255],
            [0, 0, 0],
            &config,
            false,
        );

        assert_eq!(glyph, b'#');
        assert!(!preserved);
        assert!(!gated);
    }

    #[test]
    fn test_choose_material_structural_fallback_uses_dominant_material_glyph() {
        let mut buf = SampleBuffer::new(1, 1);
        for &(x, y) in &[(2u32, 2u32), (3, 2), (2, 3), (3, 3)] {
            let sample = buf.sample_at_mut(x, y);
            sample.height = 10.0;
            sample.visual = 1;
            sample.diffuse = 128;
            sample.spare = 0;
        }

        let mut mats = vec![crate::render::material::Material::default(); 2];
        mats[1].shade[3][7].gl = b',';

        assert_eq!(
            choose_material_structural_fallback(&buf, &mats, 0, 0),
            Some(b',')
        );
    }

    #[test]
    fn test_choose_material_structural_fallback_uses_computed_elevation() {
        let mut buf = SampleBuffer::new(1, 1);
        for &(x, y) in &[(2u32, 2u32), (3, 2), (2, 3), (3, 3)] {
            let sample = buf.sample_at_mut(x, y);
            sample.height = 10.0;
            sample.visual = 1;
            sample.diffuse = 128;
            sample.spare = 0;
        }
        // Above row bit15 clear, bottom row bit15 set -> elevation 2.
        buf.sample_at_mut(2, 3).visual = 0x8001;
        buf.sample_at_mut(3, 3).visual = 0x8001;

        let mut mats = vec![crate::render::material::Material::default(); 2];
        mats[1].shade[3][7].gl = b',';
        mats[1].shade[2][7].gl = b'^';

        assert_eq!(
            choose_material_structural_fallback(&buf, &mats, 0, 0),
            Some(b'^')
        );
    }

    #[test]
    fn test_choose_final_glyph_preserves_resolve_for_low_confidence_structural_swap() {
        let config = ShapeVectorConfig::default();
        let decision = ShapeVectorDecision {
            glyph: Some(b'/'),
            candidate_glyph: Some(b'/'),
            distance: Some(0.06),
            skip_reason: None,
        };

        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            b',',
            [255, 255, 153],
            [0, 102, 0],
            &config,
            false,
        );

        assert_eq!(glyph, b',');
        assert!(preserved);
        assert!(!gated);
    }

    #[test]
    fn test_choose_final_glyph_gates_silhouette_cells() {
        let config = ShapeVectorConfig::default();
        let decision = ShapeVectorDecision {
            glyph: Some(b'/'),
            candidate_glyph: Some(b'/'),
            distance: Some(0.001),
            skip_reason: None,
        };
        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            b'-',
            [255, 255, 255],
            [0, 0, 0],
            &config,
            true,
        );

        assert_eq!(glyph, b'-');
        assert!(!preserved);
        assert!(gated);
    }

    #[test]
    fn test_choose_final_glyph_gates_mixed_auto_mat_reflection_cells() {
        let config = ShapeVectorConfig::default();
        let decision = ShapeVectorDecision {
            glyph: Some(b'%'),
            candidate_glyph: Some(b'%'),
            distance: Some(0.001),
            skip_reason: None,
        };
        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            0xDE,
            [255, 255, 255],
            [0, 0, 0],
            &config,
            true,
        );

        assert_eq!(glyph, 0xDE);
        assert!(!preserved);
        assert!(gated);
    }

    #[test]
    fn test_choose_final_glyph_harri_priority_bypasses_semantic_gate() {
        let mut config = ShapeVectorConfig::default();
        config.mode = ShapeVectorMode::HarriPriority;
        let decision = ShapeVectorDecision {
            glyph: Some(b'/'),
            candidate_glyph: Some(b'/'),
            distance: Some(0.001),
            skip_reason: None,
        };
        let (glyph, preserved, gated) = choose_final_glyph(
            &SampleBuffer::new(1, 1),
            &[],
            0,
            0,
            &decision,
            b'-',
            [255, 255, 255],
            [0, 0, 0],
            &config,
            true,
        );

        assert_eq!(glyph, b'/');
        assert!(!preserved);
        assert!(!gated);
    }
}
