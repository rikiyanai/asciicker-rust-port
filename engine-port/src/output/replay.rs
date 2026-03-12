use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use bevy::app::AppExit;
use bevy::prelude::*;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, Rgba, RgbaImage, load_from_memory};
use serde::{Deserialize, Serialize};

use crate::game::WaterLevel;
use crate::game::state::GameState;
use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::physics::{PhysicsIO, PhysicsState};
use crate::render::camera::GameCamera;
use crate::render::debug_cells::RenderDebugGrid;
use crate::render::pipeline::render_pipeline_system;
use crate::render::shape_vector::{ShapeVectorConfig, ShapeVectorFrameStats, ShapeVectorMode};

use super::capture::{PreviousFrameGrid, build_frame_capture_metadata, write_visual_capture_named};

const GIF_FONT_PNG: &[u8] = include_bytes!("../../assets/fonts/cp437_10x16.png");
const GIF_FONT_W: u32 = 10;
const GIF_FONT_H: u32 = 16;
const GIF_RENDER_W: u32 = 4;
const GIF_RENDER_H: u32 = 6;
const GIF_RENDER_PIXELS: usize = (GIF_RENDER_W as usize) * (GIF_RENDER_H as usize);
const BUILD_GIT_HASH: &str = env!("ASCIICKER_GIT_HASH");
const BUILD_ITERATION: &str = env!("ASCIICKER_BUILD_ITERATION");

#[derive(Resource, Debug, Clone)]
pub struct ReplayHarnessConfig {
    pub out_dir: Option<PathBuf>,
    pub record_trace: bool,
    pub replay_trace: Option<PathBuf>,
    pub max_frames: u32,
    pub auto_start: bool,
    pub exit_when_done: bool,
    pub orbit: Option<OrbitCaptureConfig>,
    pub variant: Option<VariantReplayConfig>,
}

impl Default for ReplayHarnessConfig {
    fn default() -> Self {
        Self {
            out_dir: std::env::var_os("ASCIICKER_BASELINE_DIR").map(PathBuf::from),
            record_trace: env_flag("ASCIICKER_BASELINE_RECORD"),
            replay_trace: std::env::var_os("ASCIICKER_BASELINE_REPLAY").map(PathBuf::from),
            max_frames: std::env::var("ASCIICKER_BASELINE_MAX_FRAMES")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(120),
            auto_start: !matches!(
                std::env::var("ASCIICKER_BASELINE_AUTO_START")
                    .ok()
                    .as_deref(),
                Some("0") | Some("false") | Some("FALSE") | Some("no") | Some("NO")
            ),
            exit_when_done: env_flag("ASCIICKER_BASELINE_EXIT"),
            orbit: OrbitCaptureConfig::from_env(),
            variant: VariantReplayConfig::from_env(),
        }
    }
}

impl ReplayHarnessConfig {
    pub fn enabled(&self) -> bool {
        self.out_dir.is_some() || self.replay_trace.is_some() || self.orbit.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct OrbitCaptureConfig {
    pub lock_camera_pos: Option<[f32; 3]>,
    pub lock_player_pos: Option<[f32; 3]>,
    pub lock_water_level: Option<f32>,
    pub zoom: Option<f32>,
    pub yaw_start: Option<f32>,
    pub yaw_step: f32,
}

#[derive(Debug, Clone)]
pub struct VariantReplayConfig {
    pub modes: Vec<ShapeVectorMode>,
    pub frames_per_mode: u32,
}

impl VariantReplayConfig {
    fn from_env() -> Option<Self> {
        let raw_modes = std::env::var("ASCIICKER_BASELINE_VARIANT_MODES").ok()?;
        let modes = raw_modes
            .split(',')
            .filter_map(ShapeVectorMode::parse)
            .collect::<Vec<_>>();
        if modes.is_empty() {
            return None;
        }
        let frames_per_mode = std::env::var("ASCIICKER_BASELINE_VARIANT_FRAMES")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(120);
        Some(Self {
            modes,
            frames_per_mode,
        })
    }

    fn total_frames(&self) -> u32 {
        self.frames_per_mode * self.modes.len() as u32
    }
}

#[derive(Debug, Clone, Copy)]
struct VariantFrameInfo {
    global_frame: u32,
    segment_index: usize,
    frame_in_segment: u32,
    mode: ShapeVectorMode,
}

impl OrbitCaptureConfig {
    fn from_env() -> Option<Self> {
        let yaw_step = std::env::var("ASCIICKER_BASELINE_YAW_STEP")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())?;
        Some(Self {
            lock_camera_pos: parse_vec3_env("ASCIICKER_BASELINE_LOCK_CAMERA_POS")
                .or_else(|| parse_vec3_env("ASCIICKER_BASELINE_LOCK_POS")),
            lock_player_pos: parse_vec3_env("ASCIICKER_BASELINE_LOCK_PLAYER_POS")
                .or_else(|| parse_vec3_env("ASCIICKER_BASELINE_LOCK_POS")),
            lock_water_level: std::env::var("ASCIICKER_BASELINE_LOCK_WATER_LEVEL")
                .ok()
                .and_then(|value| value.parse::<f32>().ok()),
            zoom: std::env::var("ASCIICKER_BASELINE_LOCK_ZOOM")
                .ok()
                .and_then(|value| value.parse::<f32>().ok()),
            yaw_start: std::env::var("ASCIICKER_BASELINE_YAW_START")
                .ok()
                .and_then(|value| value.parse::<f32>().ok()),
            yaw_step,
        })
    }
}

#[derive(Resource, Debug, Default)]
pub struct ReplayHarnessState {
    pub loaded: bool,
    pub playing_frame: u32,
    pub trace: Vec<ReplayFrame>,
    pub completed: bool,
    pub previous_grid: Option<PreviousFrameGrid>,
    pub orbit_anchor: Option<ReplayFrame>,
    pub orbit_started: bool,
    pub desktop_exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub frame: u32,
    pub camera_pos: [f32; 3],
    pub camera_yaw: f32,
    pub camera_zoom: f32,
    pub player_pos: [f32; 3],
    pub physics_yaw: f32,
    pub water_level: f32,
}

pub fn baseline_auto_start_system(
    state: Option<Res<State<GameState>>>,
    next_state: Option<ResMut<NextState<GameState>>>,
    config: Res<ReplayHarnessConfig>,
) {
    if !config.enabled() || !config.auto_start {
        return;
    }

    let (Some(state), Some(mut next_state)) = (state, next_state) else {
        return;
    };

    if *state.get() == GameState::MainMenu {
        next_state.set(GameState::Loading);
    }
}

pub fn baseline_orbit_trigger_system(
    state: Option<Res<State<GameState>>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    config: Res<ReplayHarnessConfig>,
    mut harness_state: ResMut<ReplayHarnessState>,
) {
    if config.replay_trace.is_some() || config.orbit.is_none() || harness_state.completed {
        return;
    }
    let Some(state) = state else {
        return;
    };
    if *state.get() != GameState::Playing {
        return;
    }

    let orbit = config.orbit.as_ref().expect("orbit checked above");
    let auto_start_orbit = orbit.lock_camera_pos.is_some() || orbit.lock_player_pos.is_some();
    if auto_start_orbit && !harness_state.orbit_started {
        harness_state.orbit_started = true;
        return;
    }

    if !harness_state.orbit_started && keyboard.just_pressed(KeyCode::F9) {
        harness_state.orbit_started = true;
        harness_state.playing_frame = 0;
        harness_state.previous_grid = None;
        harness_state.orbit_anchor = None;
        info!("baseline orbit capture armed from current pose (F9)");
    }
}

pub fn baseline_load_trace_system(
    mut harness_state: ResMut<ReplayHarnessState>,
    config: Res<ReplayHarnessConfig>,
) {
    if harness_state.loaded {
        return;
    }
    harness_state.loaded = true;

    let Some(trace_path) = config.replay_trace.as_ref() else {
        if config.record_trace
            && let Some(out_dir) = config.out_dir.as_ref()
        {
            let trace_path = out_dir.join("trace.jsonl");
            if let Err(err) = fs::remove_file(&trace_path)
                && err.kind() != io::ErrorKind::NotFound
            {
                warn!(
                    "baseline trace cleanup failed at {}: {}",
                    trace_path.display(),
                    err
                );
            }
        }
        return;
    };

    match load_trace(trace_path) {
        Ok(trace) => {
            info!(
                "baseline replay trace loaded: {} frames from {}",
                trace.len(),
                trace_path.display()
            );
            harness_state.trace = trace;
        }
        Err(err) => {
            error!(
                "baseline replay trace failed to load from {}: {}",
                trace_path.display(),
                err
            );
        }
    }
}

pub fn baseline_apply_replay_system(
    state: Option<Res<State<GameState>>>,
    config: Res<ReplayHarnessConfig>,
    mut harness_state: ResMut<ReplayHarnessState>,
    mut camera: ResMut<GameCamera>,
    mut physics_io: Option<ResMut<PhysicsIO>>,
    mut physics_state: Option<ResMut<PhysicsState>>,
    mut water_level: Option<ResMut<WaterLevel>>,
    mut shape_vector_config: ResMut<ShapeVectorConfig>,
) {
    if (config.replay_trace.is_none() && config.orbit.is_none()) || harness_state.completed {
        return;
    }
    let Some(state) = state else {
        return;
    };
    if *state.get() != GameState::Playing {
        return;
    }
    if config.replay_trace.is_none() && config.orbit.is_some() && !harness_state.orbit_started {
        return;
    }

    let variant_info = config
        .variant
        .as_ref()
        .and_then(|variant| variant_frame_info(variant, harness_state.playing_frame));
    if config.variant.is_some() && variant_info.is_none() {
        harness_state.completed = true;
        return;
    }
    if let Some(info) = variant_info {
        shape_vector_config.mode = info.mode;
    }

    let source_frame_index = variant_info
        .map(|info| info.frame_in_segment as usize)
        .unwrap_or(harness_state.playing_frame as usize);

    let frame = if config.replay_trace.is_some() {
        let Some(frame) = harness_state.trace.get(source_frame_index) else {
            harness_state.completed = true;
            return;
        };
        frame.clone()
    } else {
        let orbit = config.orbit.as_ref().expect("orbit config checked above");
        let playing_frame = variant_info
            .map(|info| info.frame_in_segment)
            .unwrap_or(harness_state.playing_frame);
        let anchor = harness_state
            .orbit_anchor
            .get_or_insert_with(|| {
                let player_pos = physics_io.as_ref().map(|io| io.pos).unwrap_or(camera.pos);
                let player_yaw = physics_io.as_ref().map(|io| io.yaw).unwrap_or(camera.yaw);
                let water = water_level
                    .as_ref()
                    .map(|level| level.0)
                    .unwrap_or(f32::NEG_INFINITY);
                ReplayFrame {
                    frame: 0,
                    camera_pos: orbit.lock_camera_pos.unwrap_or(camera.pos),
                    camera_yaw: orbit.yaw_start.unwrap_or(camera.yaw),
                    camera_zoom: orbit.zoom.unwrap_or(camera.zoom),
                    player_pos: orbit.lock_player_pos.unwrap_or(player_pos),
                    physics_yaw: orbit.yaw_start.unwrap_or(player_yaw),
                    water_level: orbit.lock_water_level.unwrap_or(water),
                }
            })
            .clone();
        ReplayFrame {
            frame: playing_frame,
            camera_pos: orbit.lock_camera_pos.unwrap_or(anchor.camera_pos),
            camera_yaw: orbit.yaw_start.unwrap_or(anchor.camera_yaw)
                + playing_frame as f32 * orbit.yaw_step,
            camera_zoom: orbit.zoom.unwrap_or(anchor.camera_zoom),
            player_pos: orbit.lock_player_pos.unwrap_or(anchor.player_pos),
            physics_yaw: orbit.yaw_start.unwrap_or(anchor.physics_yaw)
                + playing_frame as f32 * orbit.yaw_step,
            water_level: orbit.lock_water_level.unwrap_or(anchor.water_level),
        }
    };

    camera.pos = frame.camera_pos;
    camera.yaw = frame.camera_yaw;
    camera.zoom = frame.camera_zoom;

    if let Some(ref mut io) = physics_io {
        io.pos = frame.player_pos;
        io.yaw = frame.physics_yaw;
        io.x_force = 0.0;
        io.y_force = 0.0;
        io.z_force = 0.0;
        io.torque = 0.0;
        io.x_impulse = 0.0;
        io.y_impulse = 0.0;
        io.jump = false;
        io.vel_z = 0.0;
        io.grounded = false;
    }
    if let Some(ref mut state) = physics_state {
        state.reset_motion();
    }

    if let Some(ref mut level) = water_level {
        level.0 = frame.water_level;
    }
}

pub fn baseline_capture_system(
    state: Option<Res<State<GameState>>>,
    config: Res<ReplayHarnessConfig>,
    mut harness_state: ResMut<ReplayHarnessState>,
    grid: Res<AsciiCellGrid>,
    camera: Res<GameCamera>,
    physics_io: Option<Res<PhysicsIO>>,
    water_level: Option<Res<WaterLevel>>,
    debug_grid: Option<Res<RenderDebugGrid>>,
    shape_vector_stats: Option<Res<ShapeVectorFrameStats>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !config.enabled() || harness_state.completed {
        maybe_export_variant_bundle_to_desktop(&config, &mut harness_state);
        return;
    }
    let Some(state) = state else {
        return;
    };
    if *state.get() != GameState::Playing {
        return;
    }
    if config.replay_trace.is_none() && config.orbit.is_some() && !harness_state.orbit_started {
        return;
    }
    if grid.width == 0 || grid.height == 0 || grid.cells_count() == 0 {
        return;
    }
    if harness_state.playing_frame >= capture_frame_limit(&config) {
        harness_state.completed = true;
        maybe_export_variant_bundle_to_desktop(&config, &mut harness_state);
        if config.exit_when_done {
            app_exit.write(AppExit::Success);
        }
        return;
    }

    let Some(out_dir) = config.out_dir.as_ref() else {
        harness_state.playing_frame += 1;
        if config.replay_trace.is_some()
            && harness_state.playing_frame >= harness_state.trace.len() as u32
        {
            harness_state.completed = true;
            maybe_export_variant_bundle_to_desktop(&config, &mut harness_state);
            if config.exit_when_done {
                app_exit.write(AppExit::Success);
            }
        }
        return;
    };

    let frame_name = format!("frame_{:06}", harness_state.playing_frame);
    let metadata = build_frame_capture_metadata(
        if config.variant.is_some() {
            "variant"
        } else if config.replay_trace.is_some() {
            "replay"
        } else if config.record_trace {
            "record"
        } else {
            "capture"
        },
        harness_state.playing_frame,
        &grid,
        &camera,
        physics_io.as_deref(),
        water_level.as_deref(),
        harness_state.previous_grid.as_ref(),
        debug_grid.as_deref(),
        shape_vector_stats.as_deref(),
        None,
    );

    match write_frame_bundle(out_dir, &frame_name, &grid, &metadata) {
        Ok(()) => {}
        Err(err) => error!("baseline capture failed on {}: {}", frame_name, err),
    }

    if config.record_trace {
        let player_pos = physics_io.as_ref().map(|io| io.pos).unwrap_or(camera.pos);
        let water_world = water_level
            .as_ref()
            .map(|level| level.0)
            .unwrap_or(f32::NEG_INFINITY);
        let trace_frame = ReplayFrame {
            frame: harness_state.playing_frame,
            camera_pos: camera.pos,
            camera_yaw: camera.yaw,
            camera_zoom: camera.zoom,
            player_pos,
            physics_yaw: physics_io.as_ref().map(|io| io.yaw).unwrap_or(camera.yaw),
            water_level: water_world,
        };
        if let Err(err) = append_trace_frame(out_dir, &trace_frame) {
            error!("baseline trace append failed on {}: {}", frame_name, err);
        }
    }

    let mut previous = PreviousFrameGrid::from_grid(&grid);
    previous.debug_cells = debug_grid.as_ref().map(|grid| grid.cells.clone());
    harness_state.previous_grid = Some(previous);

    harness_state.playing_frame += 1;

    if harness_state.playing_frame >= capture_frame_limit(&config)
        || (config.replay_trace.is_some()
            && config.variant.is_none()
            && harness_state.playing_frame >= harness_state.trace.len() as u32)
    {
        harness_state.completed = true;
        maybe_export_variant_bundle_to_desktop(&config, &mut harness_state);
        if config.exit_when_done {
            app_exit.write(AppExit::Success);
        }
    }
}

pub fn baseline_variant_overlay_system(
    state: Option<Res<State<GameState>>>,
    config: Res<ReplayHarnessConfig>,
    harness_state: Res<ReplayHarnessState>,
    shape_vector_config: Res<ShapeVectorConfig>,
    shape_vector_stats: Option<Res<ShapeVectorFrameStats>>,
    mut grid: ResMut<AsciiCellGrid>,
) {
    let Some(variant) = config.variant.as_ref() else {
        return;
    };
    let Some(state) = state else {
        return;
    };
    if *state.get() != GameState::Playing || harness_state.completed || grid.height < 2 {
        return;
    }
    let Some(info) = variant_frame_info(variant, harness_state.playing_frame) else {
        return;
    };
    let capture_tag = variant_capture_tag(&config);

    let line1 = format!(
        " VAR {}/{} it{} {} tag={} {} {}/{} g{}/{} ",
        info.segment_index + 1,
        variant.modes.len(),
        BUILD_ITERATION,
        BUILD_GIT_HASH,
        capture_tag,
        info.mode.as_str(),
        info.frame_in_segment + 1,
        variant.frames_per_mode,
        info.global_frame + 1,
        variant.total_frames()
    );
    let line2 = if let Some(stats) = shape_vector_stats.as_deref() {
        format!(
            " alpha={} thr={:.3} fb={:.3} adapt={} boost={:.2} blank={} gate={} over={} ",
            shape_vector_config.alphabet.as_str(),
            shape_vector_config.distance_threshold,
            shape_vector_config.structural_fallback_distance_threshold,
            shape_vector_config.enable_contrast_adaptive_threshold,
            shape_vector_config.contrast_adaptive_threshold_boost,
            stats.colored_space_cells,
            stats.semantic_gate_cells,
            stats.selector_override_cells,
        )
    } else {
        format!(
            " alpha={} thr={:.3} fb={:.3} adapt={} boost={:.2} ",
            shape_vector_config.alphabet.as_str(),
            shape_vector_config.distance_threshold,
            shape_vector_config.structural_fallback_distance_threshold,
            shape_vector_config.enable_contrast_adaptive_threshold,
            shape_vector_config.contrast_adaptive_threshold_boost,
        )
    };

    let panel_bg = [8, 12, 18, 255];
    let panel_fg = [235, 235, 235, 255];
    let accent_fg = match info.mode {
        ShapeVectorMode::OriginalOnly => [255, 210, 120, 255],
        ShapeVectorMode::Combined => [150, 235, 170, 255],
        ShapeVectorMode::HarriPriority => [140, 200, 255, 255],
    };
    let y0 = grid.height - 2;
    let y1 = grid.height - 1;
    fill_panel_row(&mut grid, y0, panel_bg);
    fill_panel_row(&mut grid, y1, panel_bg);
    write_panel_text(&mut grid, 0, y0, &line1, accent_fg, panel_bg);
    write_panel_text(&mut grid, 0, y1, &line2, panel_fg, panel_bg);
}

fn write_frame_bundle(
    out_dir: &Path,
    frame_name: &str,
    grid: &AsciiCellGrid,
    metadata: &super::capture::FrameCaptureMetadata,
) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    write_visual_capture_named(out_dir, frame_name, grid, metadata)
}

fn append_trace_frame(out_dir: &Path, frame: &ReplayFrame) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    let trace_path = out_dir.join("trace.jsonl");
    let mut file = File::options().create(true).append(true).open(trace_path)?;
    serde_json::to_writer(&mut file, frame).map_err(io::Error::other)?;
    file.write_all(b"\n")
}

fn load_trace(path: &Path) -> io::Result<Vec<ReplayFrame>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut frames = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let frame: ReplayFrame = serde_json::from_str(&line).map_err(io::Error::other)?;
        frames.push(frame);
    }
    Ok(frames)
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn capture_frame_limit(config: &ReplayHarnessConfig) -> u32 {
    config
        .variant
        .as_ref()
        .map(VariantReplayConfig::total_frames)
        .unwrap_or(config.max_frames)
}

fn variant_frame_info(config: &VariantReplayConfig, global_frame: u32) -> Option<VariantFrameInfo> {
    if global_frame >= config.total_frames() {
        return None;
    }
    let segment_index = (global_frame / config.frames_per_mode) as usize;
    let frame_in_segment = global_frame % config.frames_per_mode;
    Some(VariantFrameInfo {
        global_frame,
        segment_index,
        frame_in_segment,
        mode: config.modes[segment_index],
    })
}

fn maybe_export_variant_bundle_to_desktop(
    config: &ReplayHarnessConfig,
    harness_state: &mut ReplayHarnessState,
) {
    if harness_state.desktop_exported || !harness_state.completed {
        return;
    }
    let (Some(variant), Some(out_dir)) = (config.variant.as_ref(), config.out_dir.as_ref()) else {
        return;
    };
    if let Err(err) = export_variant_bundle_to_desktop(out_dir, variant) {
        warn!(
            "variant Desktop export failed for {}: {}",
            out_dir.display(),
            err
        );
        return;
    }
    harness_state.desktop_exported = true;
}

fn export_variant_bundle_to_desktop(
    out_dir: &Path,
    variant: &VariantReplayConfig,
) -> io::Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::other("HOME not set"))?;
    let desktop_dir = home.join("Desktop");
    fs::create_dir_all(&desktop_dir)?;

    let link_path = desktop_dir.join("asciicker-latest-variant");
    if let Ok(existing) = fs::symlink_metadata(&link_path) {
        let file_type = existing.file_type();
        if file_type.is_symlink() || file_type.is_file() {
            fs::remove_file(&link_path)?;
        } else if file_type.is_dir() {
            fs::remove_dir_all(&link_path)?;
        }
    }
    symlink(out_dir, &link_path)?;

    let summary_path = desktop_dir.join("asciicker-latest-variant.txt");
    let mut summary = File::create(summary_path)?;
    writeln!(summary, "Asciicker stitched variant capture")?;
    writeln!(summary, "source: {}", out_dir.display())?;
    writeln!(summary, "modes: {}", format_variant_modes(&variant.modes))?;
    writeln!(summary, "frames_per_mode: {}", variant.frames_per_mode)?;
    writeln!(summary, "total_frames: {}", variant.total_frames())?;
    match export_variant_gif(out_dir) {
        Ok(gif_path) => {
            let gif_link = desktop_dir.join("asciicker-latest-variant.gif");
            if let Ok(existing) = fs::symlink_metadata(&gif_link) {
                let file_type = existing.file_type();
                if file_type.is_symlink() || file_type.is_file() {
                    fs::remove_file(&gif_link)?;
                } else if file_type.is_dir() {
                    fs::remove_dir_all(&gif_link)?;
                }
            }
            symlink(&gif_path, &gif_link)?;
            writeln!(summary, "gif: {}", gif_path.display())?;
        }
        Err(err) => {
            writeln!(summary, "gif_error: {}", err)?;
        }
    }
    writeln!(summary, "bundle: .xp + .json frame sequence")?;
    Ok(())
}

fn format_variant_modes(modes: &[ShapeVectorMode]) -> String {
    modes
        .iter()
        .map(|mode| mode.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn variant_capture_tag(config: &ReplayHarnessConfig) -> String {
    let raw = config
        .out_dir
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("variant");
    const MAX_LEN: usize = 18;
    if raw.len() <= MAX_LEN {
        raw.to_string()
    } else {
        raw[..MAX_LEN].to_string()
    }
}

fn export_variant_gif(out_dir: &Path) -> io::Result<PathBuf> {
    let mut frame_paths = fs::read_dir(out_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("xp"))
        .collect::<Vec<_>>();
    frame_paths.sort();
    if frame_paths.is_empty() {
        return Err(io::Error::other("no .xp frames found for GIF export"));
    }

    let font = load_from_memory(GIF_FONT_PNG)
        .map_err(io::Error::other)?
        .to_rgba8();
    let glyph_masks = build_gif_glyph_masks(&font);
    let gif_path = out_dir.join("variant.gif");
    let file = File::create(&gif_path)?;
    let mut encoder = GifEncoder::new(file);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(io::Error::other)?;

    for frame_path in frame_paths {
        let shot = read_shot_xp(&frame_path)?;
        let image = render_shot_to_rgba(&shot, &glyph_masks);
        let frame = Frame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(33, 1));
        encoder.encode_frame(frame).map_err(io::Error::other)?;
    }

    Ok(gif_path)
}

struct ShotGrid {
    width: u32,
    height: u32,
    char_indices: Vec<u16>,
    fg_colors: Vec<[u8; 4]>,
    bg_colors: Vec<[u8; 4]>,
}

fn read_shot_xp(path: &Path) -> io::Result<ShotGrid> {
    let bytes = fs::read(path)?;
    if bytes.len() < 16 {
        return Err(io::Error::other("xp file too small"));
    }
    let width = i32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let height = i32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if width <= 0 || height <= 0 {
        return Err(io::Error::other("invalid xp dimensions"));
    }
    let width = width as u32;
    let height = height as u32;
    let cell_count = (width * height) as usize;
    let expected = 16 + cell_count * 10;
    if bytes.len() < expected {
        return Err(io::Error::other("xp file truncated"));
    }

    let mut char_indices = vec![0u16; cell_count];
    let mut fg_colors = vec![[0, 0, 0, 255]; cell_count];
    let mut bg_colors = vec![[0, 0, 0, 255]; cell_count];
    let mut offset = 16usize;

    for x in 0..width {
        for y in (0..height).rev() {
            let idx = (y * width + x) as usize;
            let glyph = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            offset += 4;
            let fg_b = bytes[offset];
            let fg_g = bytes[offset + 1];
            let fg_r = bytes[offset + 2];
            offset += 3;
            let bg_b = bytes[offset];
            let bg_g = bytes[offset + 1];
            let bg_r = bytes[offset + 2];
            offset += 3;

            char_indices[idx] = glyph as u16;
            fg_colors[idx] = [fg_r, fg_g, fg_b, 255];
            bg_colors[idx] = [bg_r, bg_g, bg_b, 255];
        }
    }

    Ok(ShotGrid {
        width,
        height,
        char_indices,
        fg_colors,
        bg_colors,
    })
}

fn build_gif_glyph_masks(font: &RgbaImage) -> Vec<u8> {
    let mut masks = vec![0u8; 256 * GIF_RENDER_PIXELS];
    for glyph in 0..256u32 {
        let glyph_x = (glyph % 16) * GIF_FONT_W;
        let glyph_y = (glyph / 16) * GIF_FONT_H;
        let mask_offset = glyph as usize * GIF_RENDER_PIXELS;
        for py in 0..GIF_RENDER_H {
            for px in 0..GIF_RENDER_W {
                let src_x = glyph_x + (px * GIF_FONT_W) / GIF_RENDER_W;
                let src_y = glyph_y + (py * GIF_FONT_H) / GIF_RENDER_H;
                let font_pixel = font.get_pixel(src_x, src_y);
                let mask_idx = mask_offset + (py * GIF_RENDER_W + px) as usize;
                masks[mask_idx] = u8::from(font_pixel[0] >= 128);
            }
        }
    }
    masks
}

fn render_shot_to_rgba(shot: &ShotGrid, glyph_masks: &[u8]) -> RgbaImage {
    let mut image = RgbaImage::new(shot.width * GIF_RENDER_W, shot.height * GIF_RENDER_H);
    for cell_y in 0..shot.height {
        for cell_x in 0..shot.width {
            let idx = (cell_y * shot.width + cell_x) as usize;
            let glyph = shot.char_indices[idx] as u8;
            let fg = shot.fg_colors[idx];
            let bg = shot.bg_colors[idx];
            let mask_offset = glyph as usize * GIF_RENDER_PIXELS;
            let screen_cell_y = shot.height - 1 - cell_y;

            for py in 0..GIF_RENDER_H {
                for px in 0..GIF_RENDER_W {
                    let out_x = cell_x * GIF_RENDER_W + px;
                    let out_y = screen_cell_y * GIF_RENDER_H + py;
                    let mask_idx = mask_offset + (py * GIF_RENDER_W + px) as usize;
                    let rgba = if glyph_masks[mask_idx] == 0 {
                        Rgba(bg)
                    } else {
                        Rgba(fg)
                    };
                    image.put_pixel(out_x, out_y, rgba);
                }
            }
        }
    }
    image
}

fn fill_panel_row(grid: &mut AsciiCellGrid, y: u32, bg: [u8; 4]) {
    for x in 0..grid.width {
        grid.set_cell(x, y, b' ' as u16, bg, bg);
    }
}

fn write_panel_text(
    grid: &mut AsciiCellGrid,
    start_x: u32,
    y: u32,
    text: &str,
    fg: [u8; 4],
    bg: [u8; 4],
) {
    for (idx, byte) in text.bytes().enumerate() {
        let x = start_x + idx as u32;
        if x >= grid.width {
            break;
        }
        let glyph = if byte.is_ascii() { byte } else { b'?' };
        grid.set_cell(x, y, glyph as u16, fg, bg);
    }
}

fn parse_vec3_env(name: &str) -> Option<[f32; 3]> {
    let raw = std::env::var(name).ok()?;
    let mut parts = raw.split(',').map(|part| part.trim().parse::<f32>().ok());
    Some([parts.next()??, parts.next()??, parts.next()??])
}

#[allow(dead_code)]
fn _typecheck_render_dep() {
    let _ = render_pipeline_system;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_shot_to_rgba_matches_live_shader_y_flip() {
        let shot = ShotGrid {
            width: 1,
            height: 2,
            char_indices: vec![b'A' as u16, b'B' as u16],
            fg_colors: vec![[255, 0, 0, 255], [0, 255, 0, 255]],
            bg_colors: vec![[0, 0, 0, 255], [0, 0, 0, 255]],
        };
        let glyph_masks = vec![1u8; 256 * GIF_RENDER_PIXELS];
        let image = render_shot_to_rgba(&shot, &glyph_masks);

        assert_eq!(*image.get_pixel(0, 0), Rgba([0, 255, 0, 255]));
        assert_eq!(*image.get_pixel(0, GIF_RENDER_H), Rgba([255, 0, 0, 255]));
    }
}
