use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::app::AppExit;
use bevy::prelude::*;
use serde::Serialize;

use crate::asset_loader::constants::HEIGHT_SCALE;
use crate::game::WaterLevel;
use crate::game::state::GameState;
use crate::physics::PhysicsIO;
use crate::render::camera::GameCamera;
use crate::render::debug_cells::{RenderDebugCell, RenderDebugGrid, debug_flags};
use crate::render::shape_vector::ShapeVectorFrameStats;

use super::ascii_cell_grid::AsciiCellGrid;

const DEFAULT_DIFF_SAMPLE_LIMIT: usize = 64;

#[derive(Resource, Debug, Clone)]
pub struct VisualCaptureConfig {
    pub out_dir: Option<PathBuf>,
    pub delay_frames: u32,
    pub exit_after_capture: bool,
}

impl Default for VisualCaptureConfig {
    fn default() -> Self {
        Self {
            out_dir: std::env::var_os("ASCIICKER_VISUAL_CAPTURE_DIR").map(PathBuf::from),
            delay_frames: std::env::var("ASCIICKER_VISUAL_CAPTURE_DELAY_FRAMES")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10),
            exit_after_capture: env_flag("ASCIICKER_VISUAL_CAPTURE_EXIT"),
        }
    }
}

impl VisualCaptureConfig {
    pub fn enabled(&self) -> bool {
        self.out_dir.is_some()
    }
}

#[derive(Resource, Debug, Default)]
pub struct VisualCaptureState {
    pub playing_frames: u32,
    pub captured: bool,
}

#[derive(Debug, Clone)]
pub struct FrameCaptureMetadata {
    pub capture_kind: &'static str,
    pub frame_index: u32,
    pub stamp: u64,
    pub map_path: Option<String>,
    pub camera: CameraSnapshot,
    pub player: PlayerSnapshot,
    pub light: LightSnapshot,
    pub water: WaterSnapshot,
    pub grid: GridSummary,
    pub debug: Option<DebugSummary>,
    pub shape_vector: Option<ShapeVectorCaptureSummary>,
    pub previous_frame: Option<FrameDeltaSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraSnapshot {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub zoom: f32,
    pub perspective: bool,
    pub scene_shift: [i32; 2],
    pub cam_shift: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerSnapshot {
    pub pos: [f32; 3],
    pub dir: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LightSnapshot {
    pub dir: [f32; 3],
    pub ambience: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WaterSnapshot {
    pub raw: i32,
    pub world: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GridSummary {
    pub width: u32,
    pub height: u32,
    pub cells: usize,
    pub non_space_cells: usize,
    pub xp_hash64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugSummary {
    pub clear_cells: usize,
    pub reflection_cells: usize,
    pub normal_terrain_cells: usize,
    pub underwater_cells: usize,
    pub ripple_cells: usize,
    pub mixed_mesh_terrain_cells: usize,
    pub shape_override_cells: usize,
    pub linecase_cells: usize,
    pub silhouette_cells: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShapeVectorCaptureSummary {
    pub totals: ShapeVectorTotalsJson,
    pub regions: Vec<ShapeVectorRegionJson>,
    pub colored_blank_samples: Vec<AnnotatedCellSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShapeVectorTotalsJson {
    pub total_cells: u32,
    pub semantic_gate_cells: u32,
    pub clear_skip_cells: u32,
    pub underwater_skip_cells: u32,
    pub threshold_skip_cells: u32,
    pub selector_match_cells: u32,
    pub selector_override_cells: u32,
    pub resolve_fallback_cells: u32,
    pub fallback_space_cells: u32,
    pub fallback_structural_cells: u32,
    pub final_space_cells: u32,
    pub final_non_space_cells: u32,
    pub colored_space_cells: u32,
    pub avg_matched_distance: f32,
    pub max_matched_distance: f32,
    pub avg_threshold_distance: f32,
    pub max_threshold_distance: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShapeVectorRegionJson {
    pub region_x: u32,
    pub region_y: u32,
    pub total_cells: u32,
    pub semantic_gate_cells: u32,
    pub clear_skip_cells: u32,
    pub underwater_skip_cells: u32,
    pub threshold_skip_cells: u32,
    pub fallback_space_cells: u32,
    pub colored_blank_cells: u32,
    pub override_cells: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnnotatedCellSample {
    pub x: u32,
    pub y: u32,
    pub glyph: u16,
    pub fg: [u8; 4],
    pub bg: [u8; 4],
    pub contrast: u16,
    pub debug: Option<DebugCellJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameDeltaSummary {
    pub changed_cells: usize,
    pub glyph_changed_cells: usize,
    pub fg_changed_cells: usize,
    pub bg_changed_cells: usize,
    pub change_ratio: f32,
    pub bounds: Option<ChangeBounds>,
    pub samples: Vec<CellDeltaSample>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CellDeltaSample {
    pub x: u32,
    pub y: u32,
    pub before: CellSnapshot,
    pub after: CellSnapshot,
    pub before_debug: Option<DebugCellJson>,
    pub after_debug: Option<DebugCellJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CellSnapshot {
    pub glyph: u16,
    pub fg: [u8; 4],
    pub bg: [u8; 4],
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugCellJson {
    pub labels: Vec<&'static str>,
    pub sample_spares: [u8; 4],
    pub sample_heights: [f32; 4],
    pub dominant_visual: u16,
    pub shape_distance: f32,
    pub resolve_glyph: u16,
    pub final_glyph: u16,
}

#[derive(Debug, Clone)]
pub struct PreviousFrameGrid {
    pub width: u32,
    pub height: u32,
    pub char_indices: Vec<u16>,
    pub fg_colors: Vec<[u8; 4]>,
    pub bg_colors: Vec<[u8; 4]>,
    pub debug_cells: Option<Vec<RenderDebugCell>>,
}

impl PreviousFrameGrid {
    pub fn from_grid(grid: &AsciiCellGrid) -> Self {
        Self {
            width: grid.width,
            height: grid.height,
            char_indices: grid.char_indices.clone(),
            fg_colors: grid.fg_colors.clone(),
            bg_colors: grid.bg_colors.clone(),
            debug_cells: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct CaptureJson {
    version: u32,
    capture_kind: String,
    frame_index: u32,
    stamp: u64,
    size: SizeJson,
    map_path: Option<String>,
    camera: CameraSnapshot,
    player: PlayerSnapshot,
    light: LightSnapshot,
    water: WaterSnapshot,
    grid: GridSummary,
    debug: Option<DebugSummary>,
    shape_vector: Option<ShapeVectorCaptureSummary>,
    previous_frame: Option<FrameDeltaSummary>,
}

#[derive(Debug, Serialize)]
struct SizeJson {
    width: u32,
    height: u32,
}

pub fn visual_capture_system(
    state: Option<Res<State<GameState>>>,
    mut capture_state: ResMut<VisualCaptureState>,
    capture_config: Res<VisualCaptureConfig>,
    grid: Res<AsciiCellGrid>,
    camera: Res<GameCamera>,
    physics_io: Option<Res<PhysicsIO>>,
    water_level: Option<Res<WaterLevel>>,
    debug_grid: Option<Res<RenderDebugGrid>>,
    shape_vector_stats: Option<Res<ShapeVectorFrameStats>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !capture_config.enabled() || capture_state.captured {
        return;
    }

    let Some(state) = state else {
        return;
    };
    if *state.get() != GameState::Playing {
        capture_state.playing_frames = 0;
        return;
    }

    if grid.width == 0 || grid.height == 0 || grid.cells_count() == 0 {
        return;
    }

    capture_state.playing_frames += 1;
    if capture_state.playing_frames < capture_config.delay_frames {
        return;
    }

    let Some(out_dir) = capture_config.out_dir.as_ref() else {
        return;
    };

    let metadata = build_frame_capture_metadata(
        "single",
        capture_state.playing_frames,
        &grid,
        &camera,
        physics_io.as_deref(),
        water_level.as_deref(),
        None,
        debug_grid.as_deref(),
        shape_vector_stats.as_deref(),
        None,
    );

    match write_visual_capture_named(out_dir, "shot", &grid, &metadata) {
        Ok(()) => {
            capture_state.captured = true;
            info!(
                "visual capture written to {}",
                out_dir.as_os_str().to_string_lossy()
            );
            if capture_config.exit_after_capture {
                app_exit.write(AppExit::Success);
            }
        }
        Err(err) => {
            error!("visual capture failed: {err}");
        }
    }
}

pub fn build_frame_capture_metadata(
    capture_kind: &'static str,
    frame_index: u32,
    grid: &AsciiCellGrid,
    camera: &GameCamera,
    physics_io: Option<&PhysicsIO>,
    water_level: Option<&WaterLevel>,
    previous_grid: Option<&PreviousFrameGrid>,
    debug_grid: Option<&RenderDebugGrid>,
    shape_vector_stats: Option<&ShapeVectorFrameStats>,
    map_path_override: Option<String>,
) -> FrameCaptureMetadata {
    let player_pos = physics_io.map(|io| io.pos).unwrap_or(camera.pos);
    let player_dir = physics_io.map(|io| io.yaw).unwrap_or(camera.yaw);
    let water_world = water_level
        .map(|level| level.0)
        .unwrap_or(f32::NEG_INFINITY);
    let water_raw = if water_world.is_finite() {
        (water_world * HEIGHT_SCALE as f32).round() as i32
    } else {
        0
    };
    let map_path = map_path_override.or_else(|| std::env::var("A3D_MAP").ok());

    FrameCaptureMetadata {
        capture_kind,
        frame_index,
        stamp: unix_millis(),
        map_path,
        camera: CameraSnapshot {
            pos: camera.pos,
            yaw: camera.yaw,
            zoom: camera.zoom,
            perspective: camera.perspective,
            scene_shift: camera.scene_shift,
            cam_shift: 0,
        },
        player: PlayerSnapshot {
            pos: player_pos,
            dir: player_dir,
        },
        light: LightSnapshot {
            dir: camera.light_dir,
            ambience: camera.light_ambient,
        },
        water: WaterSnapshot {
            raw: water_raw,
            world: water_world,
        },
        grid: GridSummary {
            width: grid.width,
            height: grid.height,
            cells: grid.cells_count(),
            non_space_cells: count_non_space_cells(grid),
            xp_hash64: hash_grid64(grid),
        },
        debug: debug_grid.map(summarize_debug_grid),
        shape_vector: shape_vector_stats
            .map(|stats| summarize_shape_vector(grid, debug_grid, stats)),
        previous_frame: previous_grid.map(|prev| {
            diff_grids(
                prev,
                grid,
                debug_grid,
                prev.debug_cells.as_deref(),
                DEFAULT_DIFF_SAMPLE_LIMIT,
            )
        }),
    }
}

pub fn write_visual_capture_named(
    out_dir: &Path,
    base_name: &str,
    grid: &AsciiCellGrid,
    metadata: &FrameCaptureMetadata,
) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    let shot_xp = out_dir.join(format!("{base_name}.xp"));
    let shot_json = out_dir.join(format!("{base_name}.json"));

    let mut xp_file = File::create(shot_xp)?;
    write_shot_xp(&mut xp_file, grid)?;

    let mut json_file = File::create(shot_json)?;
    write_shot_json(&mut json_file, metadata)?;

    Ok(())
}

pub fn write_shot_xp<W: Write>(writer: &mut W, grid: &AsciiCellGrid) -> io::Result<()> {
    for value in [-1_i32, 1_i32, grid.width as i32, grid.height as i32] {
        writer.write_all(&value.to_le_bytes())?;
    }

    for x in 0..grid.width {
        for y in (0..grid.height).rev() {
            let (glyph, fg, bg) = grid.cell_at(x, y);
            writer.write_all(&(glyph as u32).to_le_bytes())?;
            writer.write_all(&[fg[2], fg[1], fg[0]])?;
            writer.write_all(&[bg[2], bg[1], bg[0]])?;
        }
    }

    Ok(())
}

fn write_shot_json<W: Write>(writer: &mut W, metadata: &FrameCaptureMetadata) -> io::Result<()> {
    let payload = CaptureJson {
        version: 4,
        capture_kind: metadata.capture_kind.to_string(),
        frame_index: metadata.frame_index,
        stamp: metadata.stamp,
        size: SizeJson {
            width: metadata.grid.width,
            height: metadata.grid.height,
        },
        map_path: metadata.map_path.clone(),
        camera: metadata.camera.clone(),
        player: metadata.player.clone(),
        light: metadata.light.clone(),
        water: metadata.water.clone(),
        grid: metadata.grid.clone(),
        debug: metadata.debug.clone(),
        shape_vector: metadata.shape_vector.clone(),
        previous_frame: metadata.previous_frame.clone(),
    };
    serde_json::to_writer_pretty(&mut *writer, &payload).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

pub fn diff_grids(
    previous: &PreviousFrameGrid,
    current: &AsciiCellGrid,
    current_debug: Option<&RenderDebugGrid>,
    previous_debug: Option<&[RenderDebugCell]>,
    sample_limit: usize,
) -> FrameDeltaSummary {
    let cell_count = current.cells_count().min(previous.char_indices.len());
    let mut changed_cells = 0usize;
    let mut glyph_changed_cells = 0usize;
    let mut fg_changed_cells = 0usize;
    let mut bg_changed_cells = 0usize;
    let mut samples = Vec::new();
    let mut bounds: Option<ChangeBounds> = None;

    if previous.width != current.width || previous.height != current.height {
        return FrameDeltaSummary {
            changed_cells: current.cells_count(),
            glyph_changed_cells: current.cells_count(),
            fg_changed_cells: current.cells_count(),
            bg_changed_cells: current.cells_count(),
            change_ratio: 1.0,
            bounds: Some(ChangeBounds {
                min_x: 0,
                min_y: 0,
                max_x: current.width.saturating_sub(1),
                max_y: current.height.saturating_sub(1),
            }),
            samples: collect_full_relayout_samples(current, sample_limit),
        };
    }

    for idx in 0..cell_count {
        let before = CellSnapshot {
            glyph: previous.char_indices[idx],
            fg: previous.fg_colors[idx],
            bg: previous.bg_colors[idx],
        };
        let after = CellSnapshot {
            glyph: current.char_indices[idx],
            fg: current.fg_colors[idx],
            bg: current.bg_colors[idx],
        };

        let glyph_changed = before.glyph != after.glyph;
        let fg_changed = before.fg != after.fg;
        let bg_changed = before.bg != after.bg;

        if !(glyph_changed || fg_changed || bg_changed) {
            continue;
        }

        changed_cells += 1;
        glyph_changed_cells += usize::from(glyph_changed);
        fg_changed_cells += usize::from(fg_changed);
        bg_changed_cells += usize::from(bg_changed);

        let x = (idx as u32) % current.width;
        let y = (idx as u32) / current.width;
        update_change_bounds(&mut bounds, x, y);

        if samples.len() < sample_limit {
            let before_debug = previous_debug
                .and_then(|cells| cells.get(idx))
                .map(debug_cell_to_json);
            let after_debug = current_debug
                .and_then(|grid| grid.cells.get(idx))
                .map(debug_cell_to_json);
            samples.push(CellDeltaSample {
                x,
                y,
                before,
                after,
                before_debug,
                after_debug,
            });
        }
    }

    FrameDeltaSummary {
        changed_cells,
        glyph_changed_cells,
        fg_changed_cells,
        bg_changed_cells,
        change_ratio: changed_cells as f32 / current.cells_count().max(1) as f32,
        bounds,
        samples,
    }
}

fn collect_full_relayout_samples(
    current: &AsciiCellGrid,
    sample_limit: usize,
) -> Vec<CellDeltaSample> {
    let mut samples = Vec::new();
    for idx in 0..current.cells_count().min(sample_limit) {
        let x = (idx as u32) % current.width;
        let y = (idx as u32) / current.width;
        samples.push(CellDeltaSample {
            x,
            y,
            before: CellSnapshot {
                glyph: 0,
                fg: [0, 0, 0, 0],
                bg: [0, 0, 0, 0],
            },
            after: CellSnapshot {
                glyph: current.char_indices[idx],
                fg: current.fg_colors[idx],
                bg: current.bg_colors[idx],
            },
            before_debug: None,
            after_debug: None,
        });
    }
    samples
}

fn summarize_debug_grid(grid: &RenderDebugGrid) -> DebugSummary {
    let mut summary = DebugSummary {
        clear_cells: 0,
        reflection_cells: 0,
        normal_terrain_cells: 0,
        underwater_cells: 0,
        ripple_cells: 0,
        mixed_mesh_terrain_cells: 0,
        shape_override_cells: 0,
        linecase_cells: 0,
        silhouette_cells: 0,
    };
    for cell in &grid.cells {
        summary.clear_cells += usize::from(cell.flags & debug_flags::CLEAR != 0);
        summary.reflection_cells += usize::from(cell.flags & debug_flags::HAS_REFLECTION != 0);
        summary.normal_terrain_cells +=
            usize::from(cell.flags & debug_flags::HAS_NORMAL_TERRAIN != 0);
        summary.underwater_cells += usize::from(cell.flags & debug_flags::ALL_UNDERWATER != 0);
        summary.ripple_cells += usize::from(cell.flags & debug_flags::APPLIED_RIPPLE != 0);
        summary.mixed_mesh_terrain_cells +=
            usize::from(cell.flags & debug_flags::MIXED_MESH_TERRAIN != 0);
        summary.shape_override_cells +=
            usize::from(cell.flags & debug_flags::SHAPE_VECTOR_OVERRIDE != 0);
        summary.linecase_cells +=
            usize::from(cell.flags & debug_flags::APPLIED_LINECASE_OVERLAY != 0);
        summary.silhouette_cells +=
            usize::from(cell.flags & debug_flags::APPLIED_SILHOUETTE_OVERLAY != 0);
    }
    summary
}

fn debug_cell_to_json(cell: &RenderDebugCell) -> DebugCellJson {
    DebugCellJson {
        labels: debug_labels(cell.flags),
        sample_spares: cell.sample_spares,
        sample_heights: cell.sample_heights,
        dominant_visual: cell.dominant_visual,
        shape_distance: cell.shape_distance,
        resolve_glyph: cell.resolve_glyph,
        final_glyph: cell.final_glyph,
    }
}

fn debug_labels(flags: u32) -> Vec<&'static str> {
    let mut labels = Vec::new();
    if flags & debug_flags::CLEAR != 0 {
        labels.push("clear");
    }
    if flags & debug_flags::MESH_PATH != 0 {
        labels.push("mesh_path");
    }
    if flags & debug_flags::MATERIAL_PATH != 0 {
        labels.push("material_path");
    }
    if flags & debug_flags::MIXED_MESH_TERRAIN != 0 {
        labels.push("mixed_mesh_terrain");
    }
    if flags & debug_flags::HAS_REFLECTION != 0 {
        labels.push("has_reflection");
    }
    if flags & debug_flags::HAS_NORMAL_TERRAIN != 0 {
        labels.push("has_normal_terrain");
    }
    if flags & debug_flags::ALL_UNDERWATER != 0 {
        labels.push("all_underwater");
    }
    if flags & debug_flags::USED_AUTO_MAT != 0 {
        labels.push("used_auto_mat");
    }
    if flags & debug_flags::APPLIED_RIPPLE != 0 {
        labels.push("applied_ripple");
    }
    if flags & debug_flags::APPLIED_GRID_OVERLAY != 0 {
        labels.push("grid_overlay");
    }
    if flags & debug_flags::APPLIED_LINECASE_OVERLAY != 0 {
        labels.push("linecase_overlay");
    }
    if flags & debug_flags::SHAPE_VECTOR_OVERRIDE != 0 {
        labels.push("shape_vector_override");
    }
    if flags & debug_flags::APPLIED_SILHOUETTE_OVERLAY != 0 {
        labels.push("silhouette_overlay");
    }
    if flags & debug_flags::SHAPE_SKIP_CLEAR != 0 {
        labels.push("shape_skip_clear");
    }
    if flags & debug_flags::SHAPE_SKIP_UNDERWATER != 0 {
        labels.push("shape_skip_underwater");
    }
    if flags & debug_flags::SHAPE_SKIP_THRESHOLD != 0 {
        labels.push("shape_skip_threshold");
    }
    if flags & debug_flags::SHAPE_FALLBACK_SPACE != 0 {
        labels.push("shape_fallback_space");
    }
    if flags & debug_flags::SHAPE_FALLBACK_STRUCTURAL != 0 {
        labels.push("shape_fallback_structural");
    }
    if flags & debug_flags::SHAPE_COLORED_SPACE != 0 {
        labels.push("shape_colored_space");
    }
    if flags & debug_flags::SHAPE_PRESERVED_RESOLVE != 0 {
        labels.push("shape_preserved_resolve");
    }
    if flags & debug_flags::SHAPE_GATED_SEMANTIC != 0 {
        labels.push("shape_gated_semantic");
    }
    labels
}

fn summarize_shape_vector(
    grid: &AsciiCellGrid,
    debug_grid: Option<&RenderDebugGrid>,
    stats: &ShapeVectorFrameStats,
) -> ShapeVectorCaptureSummary {
    let mut regions = Vec::new();
    let mut colored_blank_samples = Vec::new();
    let region_cols = 3u32;
    let region_rows = 3u32;

    if let Some(debug_grid) = debug_grid {
        for region_y in 0..region_rows {
            for region_x in 0..region_cols {
                let x0 = region_x * grid.width / region_cols;
                let x1 = ((region_x + 1) * grid.width / region_cols).max(x0 + 1);
                let y0 = region_y * grid.height / region_rows;
                let y1 = ((region_y + 1) * grid.height / region_rows).max(y0 + 1);
                let mut region = ShapeVectorRegionJson {
                    region_x,
                    region_y,
                    total_cells: 0,
                    semantic_gate_cells: 0,
                    clear_skip_cells: 0,
                    underwater_skip_cells: 0,
                    threshold_skip_cells: 0,
                    fallback_space_cells: 0,
                    colored_blank_cells: 0,
                    override_cells: 0,
                };

                for y in y0..y1.min(grid.height) {
                    for x in x0..x1.min(grid.width) {
                        let idx = (y * grid.width + x) as usize;
                        let cell = &debug_grid.cells[idx];
                        region.total_cells += 1;
                        region.semantic_gate_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_GATED_SEMANTIC != 0);
                        region.clear_skip_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_SKIP_CLEAR != 0);
                        region.underwater_skip_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_SKIP_UNDERWATER != 0);
                        region.threshold_skip_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_SKIP_THRESHOLD != 0);
                        region.fallback_space_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_FALLBACK_SPACE != 0);
                        region.colored_blank_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_COLORED_SPACE != 0);
                        region.override_cells +=
                            u32::from(cell.flags & debug_flags::SHAPE_VECTOR_OVERRIDE != 0);
                    }
                }
                regions.push(region);
            }
        }

        let mut candidates = Vec::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = (y * grid.width + x) as usize;
                let debug = &debug_grid.cells[idx];
                if debug.flags & debug_flags::SHAPE_COLORED_SPACE == 0 {
                    continue;
                }
                let fg = grid.fg_colors[idx];
                let bg = grid.bg_colors[idx];
                let contrast = color_contrast(fg, bg);
                candidates.push(AnnotatedCellSample {
                    x,
                    y,
                    glyph: grid.char_indices[idx],
                    fg,
                    bg,
                    contrast,
                    debug: Some(debug_cell_to_json(debug)),
                });
            }
        }
        candidates.sort_by(|a, b| b.contrast.cmp(&a.contrast));
        colored_blank_samples = candidates.into_iter().take(24).collect();
    }

    ShapeVectorCaptureSummary {
        totals: ShapeVectorTotalsJson {
            total_cells: stats.total_cells,
            semantic_gate_cells: stats.semantic_gate_cells,
            clear_skip_cells: stats.clear_skip_cells,
            underwater_skip_cells: stats.underwater_skip_cells,
            threshold_skip_cells: stats.threshold_skip_cells,
            selector_match_cells: stats.selector_match_cells,
            selector_override_cells: stats.selector_override_cells,
            resolve_fallback_cells: stats.resolve_fallback_cells,
            fallback_space_cells: stats.fallback_space_cells,
            fallback_structural_cells: stats.fallback_structural_cells,
            final_space_cells: stats.final_space_cells,
            final_non_space_cells: stats.final_non_space_cells,
            colored_space_cells: stats.colored_space_cells,
            avg_matched_distance: stats.avg_matched_distance(),
            max_matched_distance: stats.matched_distance_max,
            avg_threshold_distance: stats.avg_threshold_distance(),
            max_threshold_distance: stats.threshold_distance_max,
        },
        regions,
        colored_blank_samples,
    }
}

fn color_contrast(fg: [u8; 4], bg: [u8; 4]) -> u16 {
    u16::from(fg[0].abs_diff(bg[0]))
        + u16::from(fg[1].abs_diff(bg[1]))
        + u16::from(fg[2].abs_diff(bg[2]))
}

fn update_change_bounds(bounds: &mut Option<ChangeBounds>, x: u32, y: u32) {
    match bounds {
        Some(current) => {
            current.min_x = current.min_x.min(x);
            current.min_y = current.min_y.min(y);
            current.max_x = current.max_x.max(x);
            current.max_y = current.max_y.max(y);
        }
        None => {
            *bounds = Some(ChangeBounds {
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
            });
        }
    }
}

fn count_non_space_cells(grid: &AsciiCellGrid) -> usize {
    grid.char_indices
        .iter()
        .filter(|glyph| **glyph != 0 && **glyph != 32)
        .count()
}

fn hash_grid64(grid: &AsciiCellGrid) -> String {
    let mut hasher = DefaultHasher::new();
    grid.width.hash(&mut hasher);
    grid.height.hash(&mut hasher);
    grid.char_indices.hash(&mut hasher);
    grid.fg_colors.hash(&mut hasher);
    grid.bg_colors.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shot_xp_writer_matches_original_layout() {
        let mut grid = AsciiCellGrid::new(2, 2);
        grid.set_cell(0, 0, b'A' as u16, [1, 2, 3, 255], [4, 5, 6, 255]);
        grid.set_cell(0, 1, b'B' as u16, [7, 8, 9, 255], [10, 11, 12, 255]);
        grid.set_cell(1, 0, b'C' as u16, [13, 14, 15, 255], [16, 17, 18, 255]);
        grid.set_cell(1, 1, b'D' as u16, [19, 20, 21, 255], [22, 23, 24, 255]);

        let mut bytes = Vec::new();
        write_shot_xp(&mut bytes, &grid).unwrap();

        assert_eq!(&bytes[0..4], &(-1_i32).to_le_bytes());
        assert_eq!(&bytes[4..8], &(1_i32).to_le_bytes());
        assert_eq!(&bytes[8..12], &(2_i32).to_le_bytes());
        assert_eq!(&bytes[12..16], &(2_i32).to_le_bytes());

        let first_cell = &bytes[16..26];
        assert_eq!(&first_cell[0..4], &(b'B' as u32).to_le_bytes());
        assert_eq!(&first_cell[4..7], &[9, 8, 7]);
        assert_eq!(&first_cell[7..10], &[12, 11, 10]);

        let second_column_top = &bytes[36..46];
        assert_eq!(&second_column_top[0..4], &(b'D' as u32).to_le_bytes());
        assert_eq!(&second_column_top[4..7], &[21, 20, 19]);
        assert_eq!(&second_column_top[7..10], &[24, 23, 22]);
    }

    #[test]
    fn diff_grids_reports_exact_changed_cells() {
        let mut before = AsciiCellGrid::new(2, 2);
        before.set_cell(0, 0, b'A' as u16, [1, 1, 1, 255], [0, 0, 0, 255]);
        let prev = PreviousFrameGrid::from_grid(&before);

        let mut after = AsciiCellGrid::new(2, 2);
        after.set_cell(0, 0, b'B' as u16, [2, 2, 2, 255], [0, 0, 0, 255]);
        after.set_cell(1, 1, b'C' as u16, [3, 3, 3, 255], [4, 4, 4, 255]);

        let diff = diff_grids(&prev, &after, None, None, 8);
        assert_eq!(diff.changed_cells, 2);
        assert_eq!(diff.glyph_changed_cells, 2);
        assert_eq!(diff.fg_changed_cells, 2);
        assert_eq!(diff.bg_changed_cells, 1);
        assert_eq!(diff.samples.len(), 2);
    }
}
