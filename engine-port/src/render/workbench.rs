use std::collections::BTreeSet;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::game::state::GameState;
use crate::game::weather::{
    PrecipitationType, Weather, WeatherFrameStats, WeatherState, set_weather_state,
};
use crate::output::ascii_cell_grid::AsciiCellGrid;

use super::camera::GameCamera;
use super::config::RenderConfig;
use super::debug_cells::{RenderDebugGrid, debug_flags};
use super::pipeline::PipelineTiming;
use super::shape_vector::{
    ShapeVectorAlphabetId, ShapeVectorConfig, ShapeVectorFrameStats, ShapeVectorGlyphCandidates,
    ShapeVectorMode,
};

#[derive(Resource, Debug, Clone)]
pub struct RenderWorkbenchState {
    pub resolution_scale: f32,
    pub palette_preview: WorkbenchPalettePreview,
    pub matching_preset: WorkbenchMatchingPreset,
    pub invert_colors: bool,
    pub show_terrain: bool,
    pub show_meshes: bool,
    pub show_sprites: bool,
    pub enable_shadows: bool,
    pub enable_reflections: bool,
    pub terrain_culling: bool,
    pub world_culling: bool,
    pub show_help: bool,
    pub spin_enabled: bool,
    pub spin_speed_deg_per_sec: f32,
    pub lane_enabled: [bool; 4],
    pub lane_solo: [bool; 4],
    pub lane_weight: [f32; 4],
    pub lane_contrast: [f32; 4],
    pub diffuse_preview_index: u8,
    pub mat_elev_preview: bool,
}

impl Default for RenderWorkbenchState {
    fn default() -> Self {
        Self {
            resolution_scale: 1.0,
            palette_preview: WorkbenchPalettePreview::AsciickerOriginal,
            matching_preset: WorkbenchMatchingPreset::LegibleTerrain,
            invert_colors: false,
            show_terrain: true,
            show_meshes: true,
            show_sprites: true,
            enable_shadows: true,
            enable_reflections: true,
            terrain_culling: true,
            world_culling: true,
            show_help: false,
            spin_enabled: false,
            spin_speed_deg_per_sec: 18.0,
            lane_enabled: [true; 4],
            lane_solo: [false; 4],
            lane_weight: [1.0; 4],
            lane_contrast: [1.0; 4],
            diffuse_preview_index: 8,
            mat_elev_preview: false,
        }
    }
}

impl RenderWorkbenchState {
    pub fn reset(
        &mut self,
        camera: &mut GameCamera,
        shape: &mut ShapeVectorConfig,
        glyph_candidates: &mut ShapeVectorGlyphCandidates,
    ) {
        *self = Self::default();
        *camera = GameCamera::default();
        *shape = ShapeVectorConfig::default();
        apply_matching_preset(self.matching_preset, shape, glyph_candidates);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchPalettePreview {
    AsciickerOriginal,
    SolarField,
    GruvboxEarth,
    NordIce,
    MonokaiSignal,
    DraculaNight,
    AccessibilityHighContrast,
}

impl WorkbenchPalettePreview {
    pub fn label(self) -> &'static str {
        match self {
            Self::AsciickerOriginal => "Asciicker Original",
            Self::SolarField => "Solar Field",
            Self::GruvboxEarth => "Gruvbox Earth",
            Self::NordIce => "Nord Ice",
            Self::MonokaiSignal => "Monokai Signal",
            Self::DraculaNight => "Dracula Night",
            Self::AccessibilityHighContrast => "Accessibility High Contrast",
        }
    }

    pub fn map_rgb(self, rgb: [u8; 4]) -> [u8; 4] {
        let [r, g, b, a] = rgb;
        if self == Self::AsciickerOriginal {
            return rgb;
        }

        let luma = luma_u8(r, g, b);
        let category = PaletteCategory::classify(r, g, b);
        let [r, g, b] = sample_theme_ramp(self, category, luma);
        [r, g, b, a]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchMatchingPreset {
    OriginalMaterial,
    LegibleTerrain,
    DenseDetail,
    SilhouetteSafe,
    RainShadowContrast,
    WaterStress,
    Custom,
}

impl WorkbenchMatchingPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::OriginalMaterial => "Original Material",
            Self::LegibleTerrain => "Legible Terrain",
            Self::DenseDetail => "Dense Detail",
            Self::SilhouetteSafe => "Silhouette Safe",
            Self::RainShadowContrast => "Rain/Shadow Contrast",
            Self::WaterStress => "Water Stress",
            Self::Custom => "Custom",
        }
    }

    pub fn purpose(self) -> &'static str {
        match self {
            Self::OriginalMaterial => "Use original material glyphs only.",
            Self::LegibleTerrain => "Stable terrain readability.",
            Self::DenseDetail => "More texture and density.",
            Self::SilhouetteSafe => "Protect semantic edges.",
            Self::RainShadowContrast => "Weather and dark pass clarity.",
            Self::WaterStress => "Inspect reflections and water edges.",
            Self::Custom => "Your edited setting bundle.",
        }
    }
}

fn luma_u8(r: u8, g: u8, b: u8) -> u8 {
    ((u16::from(r) * 54 + u16::from(g) * 183 + u16::from(b) * 19) / 256) as u8
}

#[derive(Debug, Clone, Copy)]
enum PaletteCategory {
    Sky,
    Water,
    Vegetation,
    Stone,
    Shadow,
    Accent,
}

impl PaletteCategory {
    fn classify(r: u8, g: u8, b: u8) -> Self {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let luma = luma_u8(r, g, b);
        if luma < 44 {
            Self::Shadow
        } else if max.saturating_sub(min) < 24 {
            Self::Stone
        } else if b > r.saturating_add(18) && b >= g {
            if luma > 150 { Self::Sky } else { Self::Water }
        } else if g > r && g >= b {
            Self::Vegetation
        } else if r > g.saturating_add(18) {
            Self::Accent
        } else {
            Self::Stone
        }
    }
}

fn sample_theme_ramp(
    theme: WorkbenchPalettePreview,
    category: PaletteCategory,
    luma: u8,
) -> [u8; 3] {
    let ramp = match (theme, category) {
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Sky) => [
            [38, 70, 83],
            [42, 106, 122],
            [91, 151, 159],
            [181, 207, 199],
        ],
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Water) => {
            [[7, 54, 66], [38, 139, 150], [42, 161, 152], [147, 211, 203]]
        }
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Vegetation) => {
            [[48, 71, 46], [88, 110, 64], [133, 153, 0], [201, 196, 118]]
        }
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Stone) => [
            [58, 68, 70],
            [101, 123, 131],
            [147, 161, 161],
            [238, 232, 213],
        ],
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Shadow) => {
            [[0, 24, 30], [7, 54, 66], [38, 70, 83], [88, 110, 117]]
        }
        (WorkbenchPalettePreview::SolarField, PaletteCategory::Accent) => {
            [[91, 54, 34], [181, 86, 0], [203, 75, 22], [253, 246, 227]]
        }
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Sky) => {
            [[40, 40, 40], [69, 80, 83], [131, 165, 152], [168, 153, 132]]
        }
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Water) => [
            [29, 48, 52],
            [69, 133, 136],
            [131, 165, 152],
            [213, 196, 161],
        ],
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Vegetation) => {
            [[50, 56, 36], [104, 119, 47], [152, 151, 26], [184, 187, 38]]
        }
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Stone) => {
            [[40, 40, 40], [80, 73, 69], [146, 131, 116], [235, 219, 178]]
        }
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Shadow) => {
            [[29, 32, 33], [40, 40, 40], [60, 56, 54], [80, 73, 69]]
        }
        (WorkbenchPalettePreview::GruvboxEarth, PaletteCategory::Accent) => {
            [[85, 48, 36], [204, 36, 29], [214, 93, 14], [251, 241, 199]]
        }
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Sky) => {
            [[46, 52, 64], [67, 76, 94], [129, 161, 193], [216, 222, 233]]
        }
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Water) => [
            [35, 56, 70],
            [94, 129, 172],
            [136, 192, 208],
            [229, 233, 240],
        ],
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Vegetation) => [
            [45, 66, 58],
            [86, 120, 105],
            [163, 190, 140],
            [220, 233, 202],
        ],
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Stone) => [
            [46, 52, 64],
            [76, 86, 106],
            [143, 155, 179],
            [236, 239, 244],
        ],
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Shadow) => {
            [[20, 24, 31], [36, 42, 54], [46, 52, 64], [67, 76, 94]]
        }
        (WorkbenchPalettePreview::NordIce, PaletteCategory::Accent) => [
            [75, 46, 57],
            [191, 97, 106],
            [208, 135, 112],
            [236, 239, 244],
        ],
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Sky) => {
            [[39, 40, 34], [73, 72, 62], [102, 217, 239], [248, 248, 242]]
        }
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Water) => [
            [25, 45, 50],
            [102, 217, 239],
            [166, 226, 244],
            [248, 248, 242],
        ],
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Vegetation) => [
            [42, 55, 26],
            [117, 160, 44],
            [166, 226, 46],
            [232, 255, 166],
        ],
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Stone) => {
            [[39, 40, 34], [85, 85, 78], [174, 174, 164], [248, 248, 242]]
        }
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Shadow) => {
            [[20, 20, 18], [39, 40, 34], [73, 72, 62], [117, 113, 94]]
        }
        (WorkbenchPalettePreview::MonokaiSignal, PaletteCategory::Accent) => [
            [80, 35, 60],
            [249, 38, 114],
            [253, 151, 31],
            [255, 255, 180],
        ],
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Sky) => {
            [[40, 42, 54], [68, 71, 90], [98, 114, 164], [248, 248, 242]]
        }
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Water) => [
            [31, 45, 62],
            [68, 71, 120],
            [139, 233, 253],
            [241, 250, 255],
        ],
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Vegetation) => [
            [42, 55, 55],
            [70, 120, 100],
            [80, 250, 123],
            [210, 255, 220],
        ],
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Stone) => {
            [[40, 42, 54], [68, 71, 90], [189, 147, 249], [248, 248, 242]]
        }
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Shadow) => {
            [[20, 22, 30], [32, 34, 45], [40, 42, 54], [68, 71, 90]]
        }
        (WorkbenchPalettePreview::DraculaNight, PaletteCategory::Accent) => [
            [80, 45, 85],
            [255, 85, 85],
            [255, 121, 198],
            [255, 184, 108],
        ],
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Sky) => {
            [[0, 35, 80], [0, 95, 150], [80, 190, 255], [245, 250, 255]]
        }
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Water) => {
            [[0, 20, 70], [0, 100, 220], [0, 210, 255], [230, 255, 255]]
        }
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Vegetation) => {
            [[0, 45, 0], [0, 130, 20], [120, 255, 0], [240, 255, 180]]
        }
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Stone) => {
            [[0, 0, 0], [65, 65, 65], [180, 180, 180], [255, 255, 255]]
        }
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Shadow) => {
            [[0, 0, 0], [20, 20, 20], [70, 70, 70], [135, 135, 135]]
        }
        (WorkbenchPalettePreview::AccessibilityHighContrast, PaletteCategory::Accent) => {
            [[90, 0, 0], [255, 0, 0], [255, 210, 0], [255, 255, 255]]
        }
        _ => [[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]],
    };
    sample_ramp(ramp, luma)
}

fn sample_ramp(ramp: [[u8; 3]; 4], luma: u8) -> [u8; 3] {
    let t = luma as f32 / 255.0 * 3.0;
    let idx = t.floor().min(2.0) as usize;
    let frac = t - idx as f32;
    lerp_rgb(ramp[idx], ramp[idx + 1], frac)
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t).round() as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t).round() as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t).round() as u8,
    ]
}

pub fn apply_palette_preview(rgb: [u8; 4], palette: WorkbenchPalettePreview) -> [u8; 4] {
    palette.map_rgb(rgb)
}

pub fn material_lane_preview_active(workbench: &RenderWorkbenchState) -> bool {
    workbench.mat_elev_preview
        || workbench.lane_solo.iter().any(|value| *value)
        || workbench.lane_enabled.iter().any(|value| !*value)
        || workbench
            .lane_weight
            .iter()
            .any(|value| (*value - 1.0).abs() > f32::EPSILON)
        || workbench
            .lane_contrast
            .iter()
            .any(|value| (*value - 1.0).abs() > f32::EPSILON)
}

pub fn apply_material_lane_preview(
    rgb: [u8; 4],
    material_lane: u8,
    diffuse_index: u8,
    workbench: &RenderWorkbenchState,
) -> [u8; 4] {
    if !material_lane_preview_active(workbench) {
        return rgb;
    }

    let lane = (material_lane as usize).min(3);
    let solo_active = workbench.lane_solo.iter().any(|value| *value);
    let mut color = [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32];

    if !workbench.lane_enabled[lane] || (solo_active && !workbench.lane_solo[lane]) {
        let luma = luma_u8(rgb[0], rgb[1], rgb[2]) as f32;
        let dim = if workbench.lane_enabled[lane] {
            0.34
        } else {
            0.22
        };
        for channel in &mut color {
            *channel = (*channel * 0.25 + luma * 0.75) * dim;
        }
    } else {
        let contrast = workbench.lane_contrast[lane].clamp(0.25, 2.5);
        let weight = workbench.lane_weight[lane].clamp(0.0, 2.0);
        for channel in &mut color {
            *channel = ((*channel - 128.0) * contrast + 128.0) * weight;
        }
        if workbench.mat_elev_preview {
            let tint = lane_tint(lane);
            let mix = (0.22 + (weight - 1.0).max(0.0) * 0.16).clamp(0.0, 0.44);
            let diffuse_delta = diffuse_index.abs_diff(workbench.diffuse_preview_index) as f32;
            let diffuse_focus = 1.0 - (diffuse_delta / 15.0).clamp(0.0, 1.0) * 0.28;
            for index in 0..3 {
                color[index] = color[index] * (1.0 - mix) + tint[index] as f32 * mix;
                color[index] *= diffuse_focus;
            }
        }
    }

    [
        color[0].round().clamp(0.0, 255.0) as u8,
        color[1].round().clamp(0.0, 255.0) as u8,
        color[2].round().clamp(0.0, 255.0) as u8,
        rgb[3],
    ]
}

fn lane_tint(lane: usize) -> [u8; 3] {
    match lane {
        0 => [120, 255, 140],
        1 => [255, 220, 72],
        2 => [80, 210, 255],
        _ => [255, 112, 206],
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RenderWorkbenchFrameStats {
    pub terrain_considered: u32,
    pub terrain_drawn: u32,
    pub terrain_culled: u32,
    pub world_considered: u32,
    pub world_visible_after_culling: u32,
    pub world_rendered: u32,
    pub world_culled: u32,
    pub sprites_queued: u32,
    pub shadow_affected_samples: u32,
    pub reflection_affected_samples: u32,
}

impl RenderWorkbenchFrameStats {
    pub fn begin_frame(&mut self) {
        *self = Self::default();
    }
}

pub struct RenderWorkbenchPlugin;

impl Plugin for RenderWorkbenchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderWorkbenchState>()
            .init_resource::<RenderWorkbenchFrameStats>()
            .add_plugins(EguiPlugin::default())
            .add_systems(
                Update,
                (
                    return_to_workbench_input_system,
                    render_workbench_spin_system,
                ),
            )
            .add_systems(EguiPrimaryContextPass, render_workbench_ui_system);
    }
}

fn return_to_workbench_input_system(
    game_state: Option<Res<State<GameState>>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(game_state) = game_state.as_deref() else {
        return;
    };
    if !matches!(game_state.get(), GameState::Playing | GameState::Paused) {
        return;
    }
    if keys.just_pressed(KeyCode::F2) {
        next_state.set(GameState::Workbench);
        info!("Returning to Render Tuning Workbench via F2");
    }
}

fn render_workbench_spin_system(
    game_state: Option<Res<State<GameState>>>,
    time: Res<Time>,
    workbench: Res<RenderWorkbenchState>,
    mut camera: ResMut<GameCamera>,
    config: Option<Res<RenderConfig>>,
) {
    let Some(game_state) = game_state.as_deref() else {
        return;
    };
    if *game_state.get() != GameState::Workbench || !workbench.spin_enabled {
        return;
    }

    camera.yaw += workbench.spin_speed_deg_per_sec * time.delta_secs();
    normalize_degrees(&mut camera.yaw);
    if let Some(config) = config {
        let dw = config.sample_width() as f64;
        let dh = config.sample_height() as f64;
        camera.update(dw, dh);
        camera.extract_frustum_planes(dw, dh);
    }
}

fn render_workbench_ui_system(
    mut contexts: EguiContexts,
    game_state: Option<Res<State<GameState>>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut workbench: ResMut<RenderWorkbenchState>,
    mut camera: ResMut<GameCamera>,
    mut shape_config: ResMut<ShapeVectorConfig>,
    mut glyph_candidates: ResMut<ShapeVectorGlyphCandidates>,
    cell_grid: Res<AsciiCellGrid>,
    debug_grid: Res<RenderDebugGrid>,
    timing: Res<PipelineTiming>,
    stats: Res<ShapeVectorFrameStats>,
    workbench_stats: Res<RenderWorkbenchFrameStats>,
    mut weather: ResMut<Weather>,
    weather_stats: Res<WeatherFrameStats>,
) {
    let Some(game_state) = game_state.as_deref() else {
        return;
    };
    let ctx = contexts.ctx_mut().expect("primary egui context");
    apply_workbench_visuals(ctx);
    if *game_state.get() != GameState::Workbench {
        render_return_chip(ctx, game_state.get(), &mut next_state);
        return;
    }

    let screen = ctx.content_rect();
    let frame_fill = egui::Color32::from_rgba_premultiplied(248, 245, 238, 236);
    let frame_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(204, 196, 182));
    let sample_width = 2 * cell_grid.width + 4;
    let sample_height = 2 * cell_grid.height + 4;
    let max_panel_height = (screen.height() - 40.0).clamp(320.0, 920.0);
    normalize_degrees(&mut camera.yaw);

    egui::Area::new("render_workbench_header".into())
        .fixed_pos(egui::pos2(screen.left() + 20.0, screen.top() + 20.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(frame_fill)
                .stroke(frame_stroke)
                .corner_radius(8.0)
                .show(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Render Tuning Workbench")
                                .size(20.0)
                                .strong()
                                .color(egui::Color32::from_rgb(28, 28, 28)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Explicit mode with live renderer controls and diagnostics.",
                            )
                            .size(12.0)
                            .color(egui::Color32::from_rgb(92, 88, 82)),
                        );
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([152.0, 30.0], egui::Button::new("Resume Scene"))
                                .on_hover_text("Return to gameplay. Use F2 or the Workbench chip to come back.")
                                .clicked()
                            {
                                next_state.set(GameState::Playing);
                            }
                            if ui
                                .add_sized([118.0, 30.0], egui::Button::new("Main Menu"))
                                .clicked()
                            {
                                next_state.set(GameState::MainMenu);
                            }
                        });
                    });
                });
        });

    egui::Area::new("render_workbench_controls".into())
        .fixed_pos(egui::pos2(screen.right() - 388.0, screen.top() + 20.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(frame_fill)
                .stroke(frame_stroke)
                .corner_radius(10.0)
                .show(ui, |ui| {
                    ui.set_min_width(360.0);
                    ui.set_max_width(360.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(max_panel_height)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add_sized(
                                        [154.0, 28.0],
                                        egui::Button::new(if workbench.show_help {
                                            "Hide Control Help"
                                        } else {
                                            "Show Control Help"
                                        }),
                                    )
                                    .on_hover_text("Open a compact reference for every workbench control and its matching hotkey.")
                                    .clicked()
                                {
                                    workbench.show_help = !workbench.show_help;
                                }
                                if ui
                                    .add_sized([154.0, 28.0], egui::Button::new("Reset Defaults"))
                                    .on_hover_text("Restore renderer, camera, and glyph-matching settings to their documented defaults.")
                                    .clicked()
                                {
                                    workbench.reset(
                                        &mut camera,
                                        &mut shape_config,
                                        &mut glyph_candidates,
                                    );
                                }
                            });
                            if workbench.show_help {
                                help_panel(ui);
                                ui.add_space(14.0);
                            }

                            section_label(ui, "View");
                            slider_row(
                                ui,
                                "Resolution scale",
                                &mut workbench.resolution_scale,
                                0.25..=1.0,
                                0.01,
                                "Changes the ASCII grid size used by the render pipeline. Lower values render fewer cells faster.",
                            );
                            metric_row(
                                ui,
                                "Grid",
                                format!("{} x {}", cell_grid.width, cell_grid.height),
                            );
                            metric_row(
                                ui,
                                "Sample",
                                format!("{} x {}", sample_width, sample_height),
                            );
                            slider_row(
                                ui,
                                "Zoom",
                                &mut camera.zoom,
                                0.5..=3.0,
                                0.01,
                                "Changes the camera scale used by projection.",
                            );
                            slider_row(
                                ui,
                                "Yaw",
                                &mut camera.yaw,
                                -180.0..=180.0,
                                1.0,
                                "Rotates the camera around the scene. Keyboard parity: Q/E.",
                            );
                            stepper_row(ui, "Yaw step", &mut camera.yaw, 45.0, "deg");
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Spin",
                                    &mut workbench.spin_enabled,
                                    "Automatically orbits the camera yaw for ASCIIID-style render inspection.",
                                );
                            });
                            let spin_enabled = workbench.spin_enabled;
                            enabled_slider_row(
                                ui,
                                "Spin speed",
                                &mut workbench.spin_speed_deg_per_sec,
                                -120.0..=120.0,
                                1.0,
                                spin_enabled,
                                "Yaw degrees per second while Spin is enabled.",
                            );
                            slider_row(
                                ui,
                                "Camera X",
                                &mut camera.pos[0],
                                -256.0..=256.0,
                                0.25,
                                "Moves the camera east/west. Keyboard parity: A/D.",
                            );
                            slider_row(
                                ui,
                                "Camera Y",
                                &mut camera.pos[1],
                                -256.0..=256.0,
                                0.25,
                                "Moves the camera north/south. Keyboard parity: W/S.",
                            );
                            slider_row(
                                ui,
                                "Camera Z",
                                &mut camera.pos[2],
                                -64.0..=256.0,
                                0.25,
                                "Raises or lowers the diagnostic camera height.",
                            );
                            camera_nudge_row(ui, &mut camera);
                            metric_row(
                                ui,
                                "Camera pos",
                                format!(
                                    "{:.1}, {:.1}, {:.1}",
                                    camera.pos[0], camera.pos[1], camera.pos[2]
                                ),
                            );
                            metric_row(ui, "Return path", "F2 / Workbench chip".to_string());

                            ui.add_space(14.0);
                            section_label(ui, "Visibility");
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(ui, "Terrain", &mut workbench.show_terrain, "Turns terrain rasterization on or off.");
                                toggle_button(ui, "Meshes", &mut workbench.show_meshes, "Turns world mesh rasterization on or off.");
                                toggle_button(ui, "Sprites", &mut workbench.show_sprites, "Turns sprite queue rendering on or off.");
                                toggle_button(ui, "Shadows", &mut workbench.enable_shadows, "Turns the shadow stage on or off.");
                                toggle_button(ui, "Reflections", &mut workbench.enable_reflections, "Turns the water reflection stage on or off.");
                                toggle_button(ui, "Invert colors", &mut workbench.invert_colors, "Inverts resolved foreground/background colors for inspection.");
                            });

                            ui.add_space(14.0);
                            section_label(ui, "Palette Preview");
                            enum_row(ui, "Default palettes", |ui| {
                                palette_button(
                                    ui,
                                    "Original",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::AsciickerOriginal,
                                    "Use the renderer's resolved colors without preview remapping.",
                                );
                                palette_button(
                                    ui,
                                    "Solar",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::SolarField,
                                    "Preview a Solarized-inspired field palette with distinct terrain, water, stone, and accent ramps.",
                                );
                                palette_button(
                                    ui,
                                    "Gruvbox",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::GruvboxEarth,
                                    "Preview a warm editor-inspired earth palette with separate greens, blues, neutrals, and accents.",
                                );
                                palette_button(
                                    ui,
                                    "Nord",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::NordIce,
                                    "Preview a cool high-readability palette inspired by Nord.",
                                );
                                palette_button(
                                    ui,
                                    "Monokai",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::MonokaiSignal,
                                    "Preview a saturated editor palette for testing signal separation.",
                                );
                                palette_button(
                                    ui,
                                    "Dracula",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::DraculaNight,
                                    "Preview a dark high-saturation palette for nighttime contrast checks.",
                                );
                                palette_button(
                                    ui,
                                    "A11y",
                                    &mut workbench.palette_preview,
                                    WorkbenchPalettePreview::AccessibilityHighContrast,
                                    "Preview a multi-color high-contrast palette for accessibility stress testing.",
                                );
                            });
                            metric_row(
                                ui,
                                "Palette",
                                workbench.palette_preview.label().to_string(),
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Culling");
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Terrain culling",
                                    &mut workbench.terrain_culling,
                                    "Uses the terrain frustum query instead of drawing every patch.",
                                );
                                toggle_button(ui, "World culling", &mut workbench.world_culling, "Uses BSP/frustum visibility instead of drawing every visible world instance.");
                            });

                            ui.add_space(14.0);
                            section_label(ui, "Weather");
                            enum_row(ui, "State", |ui| {
                                weather_button(ui, "Clear", &mut weather, WeatherState::Clear);
                                weather_button(
                                    ui,
                                    "Light snow",
                                    &mut weather,
                                    WeatherState::LightSnow,
                                );
                                weather_button(
                                    ui,
                                    "Heavy snow",
                                    &mut weather,
                                    WeatherState::HeavySnow,
                                );
                                weather_button(
                                    ui,
                                    "Blizzard",
                                    &mut weather,
                                    WeatherState::Blizzard,
                                );
                            });
                            enum_row(ui, "Precipitation", |ui| {
                                enum_button(
                                    ui,
                                    "Snow",
                                    &mut weather.precipitation,
                                    PrecipitationType::Snow,
                                    "Uses snow glyphs and speeds for weather particles.",
                                );
                                enum_button(
                                    ui,
                                    "Rain",
                                    &mut weather.precipitation,
                                    PrecipitationType::Rain,
                                    "Uses rain glyphs and speed for weather particles.",
                                );
                            });
                            metric_row(ui, "Intensity", format!("{:.2}", weather.intensity));
                            metric_row(
                                ui,
                                "Particles live/visible",
                                format!(
                                    "{} / {}",
                                    weather_stats.live_particles, weather_stats.visible_particles
                                ),
                            );
                            metric_row(
                                ui,
                                "Weather cells",
                                format!("{}", weather_stats.composited_cells),
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Glyph Matching");
                            enum_row(ui, "Presets", |ui| {
                                matching_preset_button(
                                    ui,
                                    "Original",
                                    WorkbenchMatchingPreset::OriginalMaterial,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Legible",
                                    WorkbenchMatchingPreset::LegibleTerrain,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Dense",
                                    WorkbenchMatchingPreset::DenseDetail,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Silhouette",
                                    WorkbenchMatchingPreset::SilhouetteSafe,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Rain/Shadow",
                                    WorkbenchMatchingPreset::RainShadowContrast,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Water",
                                    WorkbenchMatchingPreset::WaterStress,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                                matching_preset_button(
                                    ui,
                                    "Custom",
                                    WorkbenchMatchingPreset::Custom,
                                    &mut workbench.matching_preset,
                                    &mut shape_config,
                                    &mut glyph_candidates,
                                );
                            });
                            metric_row(
                                ui,
                                "Preset",
                                workbench.matching_preset.label().to_string(),
                            );
                            ui.label(
                                egui::RichText::new(workbench.matching_preset.purpose())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(92, 88, 82)),
                            );
                            ui.add_space(8.0);
                            section_label(ui, "Advanced Glyph Controls");
                            enum_row(ui, "Mode", |ui| {
                                enum_button(
                                    ui,
                                    "Original",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::OriginalOnly,
                                    "Uses the original resolve glyph only. Keyboard parity: F12 cycles modes.",
                                );
                                enum_button(
                                    ui,
                                    "Combined",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::Combined,
                                    "Uses shape-vector glyphs only when semantic gating allows replacement. Keyboard parity: F12 cycles modes.",
                                );
                                enum_button(
                                    ui,
                                    "Harri",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::HarriPriority,
                                    "Prefers the Alex Harri shape-vector match for eligible cells. Keyboard parity: F12 cycles modes.",
                                );
                            });
                            enum_row(ui, "Alphabet", |ui| {
                                enum_button(
                                    ui,
                                    "Default",
                                    &mut shape_config.alphabet,
                                    ShapeVectorAlphabetId::Default,
                                    "Uses the full default glyph alphabet. Keyboard parity: F6 cycles alphabets.",
                                );
                                enum_button(
                                    ui,
                                    "Minimal",
                                    &mut shape_config.alphabet,
                                    ShapeVectorAlphabetId::Minimal,
                                    "Uses the smaller comparison alphabet. Keyboard parity: F6 cycles alphabets.",
                                );
                            });
                            ui.add_space(8.0);
                            glyph_candidate_panel(ui, &mut glyph_candidates);
                            slider_row(
                                ui,
                                "Distance threshold",
                                &mut shape_config.distance_threshold,
                                0.0..=1.0,
                                0.005,
                                "Rejects shape-vector glyph replacements above this match distance. Keyboard parity: [ and ].",
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Adaptive threshold",
                                    &mut shape_config.enable_contrast_adaptive_threshold,
                                    "Enables contrast-aware threshold expansion. Keyboard parity: F11.",
                                );
                            });
                            let adaptive_threshold_enabled =
                                shape_config.enable_contrast_adaptive_threshold;
                            enabled_slider_row(
                                ui,
                                "Adaptive boost",
                                &mut shape_config.contrast_adaptive_threshold_boost,
                                0.0..=4.0,
                                0.05,
                                adaptive_threshold_enabled,
                                "Amount added by contrast-aware thresholding. Keyboard parity: 7 and 8.",
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Structural fallback",
                                    &mut shape_config.enable_structural_fallback,
                                    "Allows a structural fallback glyph when distance thresholding rejects the best match. Keyboard parity: F10.",
                                );
                            });
                            let structural_fallback_enabled =
                                shape_config.enable_structural_fallback;
                            enabled_slider_row(
                                ui,
                                "Fallback threshold",
                                &mut shape_config.structural_fallback_distance_threshold,
                                0.0..=2.5,
                                0.01,
                                structural_fallback_enabled,
                                "Distance limit for structural fallback glyphs. Keyboard parity: 9 and 0.",
                            );
                            let mut sampling_quality = shape_config.sampling_quality as u32;
                            int_slider_row(
                                ui,
                                "Sampling quality",
                                &mut sampling_quality,
                                1..=32,
                                "Controls shape-vector sampling density. Keyboard parity: - and =.",
                            );
                            shape_config.sampling_quality = sampling_quality as usize;
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Global crunch",
                                    &mut shape_config.enable_global_crunch,
                                    "Enables global vector exponent weighting. Keyboard parity: F7.",
                                );
                            });
                            let global_crunch_enabled = shape_config.enable_global_crunch;
                            enabled_slider_row(
                                ui,
                                "Global exponent",
                                &mut shape_config.global_crunch_exponent,
                                0.25..=16.0,
                                0.25,
                                global_crunch_enabled,
                                "Exponent for global vector contrast weighting. Keyboard parity: ; and '.",
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Directional crunch",
                                    &mut shape_config.enable_directional_crunch,
                                    "Enables directional vector exponent weighting. Keyboard parity: F8.",
                                );
                            });
                            let directional_crunch_enabled = shape_config.enable_directional_crunch;
                            enabled_slider_row(
                                ui,
                                "Directional exponent",
                                &mut shape_config.directional_crunch_exponent,
                                0.5..=20.0,
                                0.5,
                                directional_crunch_enabled,
                                "Exponent for directional vector contrast weighting. Keyboard parity: , and ..",
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Pass Proof");
                            metric_row(
                                ui,
                                "Terrain patches",
                                format!(
                                    "{} drawn / {} considered / {} culled",
                                    workbench_stats.terrain_drawn,
                                    workbench_stats.terrain_considered,
                                    workbench_stats.terrain_culled
                                ),
                            );
                            metric_row(
                                ui,
                                "World instances",
                                format!(
                                    "{} rendered / {} visible / {} considered / {} culled",
                                    workbench_stats.world_rendered,
                                    workbench_stats.world_visible_after_culling,
                                    workbench_stats.world_considered,
                                    workbench_stats.world_culled
                                ),
                            );
                            metric_row(
                                ui,
                                "Sprites queued",
                                format!("{}", workbench_stats.sprites_queued),
                            );
                            metric_row(
                                ui,
                                "Shadow affected",
                                format!("{} samples", workbench_stats.shadow_affected_samples),
                            );
                            metric_row(
                                ui,
                                "Reflection affected",
                                format!("{} samples", workbench_stats.reflection_affected_samples),
                            );
                            metric_row(
                                ui,
                                "Weather affected",
                                format!("{} cells", weather_stats.composited_cells),
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Material Probe");
                            material_probe_panel(ui, ctx, &cell_grid, &debug_grid, &camera);

                            ui.add_space(14.0);
                            section_label(ui, "Material Lanes");
                            material_lane_panel(ui, &mut workbench);

                            ui.add_space(14.0);
                            section_label(ui, "Diagnostics");
                            metric_row(ui, "Mode", shape_mode_label(shape_config.mode).to_string());
                            metric_row(
                                ui,
                                "Alphabet",
                                alphabet_label(shape_config.alphabet).to_string(),
                            );
                            metric_row(ui, "Frame", micros_to_ms(timing.total_us));
                            metric_row(ui, "Terrain", micros_to_ms(timing.terrain_us));
                            metric_row(ui, "World", micros_to_ms(timing.world_us));
                            metric_row(ui, "Shadow", micros_to_ms(timing.shadow_us));
                            metric_row(ui, "Reflection", micros_to_ms(timing.reflection_us));
                            metric_row(ui, "Resolve", micros_to_ms(timing.resolve_us));
                            metric_row(
                                ui,
                                "Overrides",
                                percent(&stats, stats.selector_override_cells),
                            );
                            metric_row(
                                ui,
                                "Fallback",
                                percent(&stats, stats.resolve_fallback_cells),
                            );
                            metric_row(
                                ui,
                                "Threshold skips",
                                percent(&stats, stats.threshold_skip_cells),
                            );
                            metric_row(
                                ui,
                                "Semantic gate",
                                percent(&stats, stats.semantic_gate_cells),
                            );
                            metric_row(ui, "Clear skips", percent(&stats, stats.clear_skip_cells));
                            metric_row(
                                ui,
                                "Underwater skips",
                                percent(&stats, stats.underwater_skip_cells),
                            );
                            metric_row(ui, "Blank cells", percent(&stats, stats.final_space_cells));
                            metric_row(
                                ui,
                                "Colored blanks",
                                percent(&stats, stats.colored_space_cells),
                            );
                        });
                });
        });
}

fn apply_workbench_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = egui::Color32::from_rgba_premultiplied(248, 245, 238, 236);
    visuals.panel_fill = egui::Color32::from_rgba_premultiplied(248, 245, 238, 236);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(28, 28, 28);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 230, 220);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(244, 239, 231);
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(46, 46, 46);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(70, 70, 70);
    visuals.selection.bg_fill = egui::Color32::from_rgb(38, 38, 38);
    visuals.selection.stroke.color = egui::Color32::WHITE;
    ctx.set_visuals(visuals);
}

fn render_return_chip(
    ctx: &egui::Context,
    state: &GameState,
    next_state: &mut ResMut<NextState<GameState>>,
) {
    let screen = ctx.content_rect();
    egui::Area::new("render_workbench_return_chip".into())
        .fixed_pos(egui::pos2(screen.right() - 172.0, screen.top() + 18.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_premultiplied(248, 245, 238, 214))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(204, 196, 182),
                ))
                .corner_radius(8.0)
                .show(ui, |ui| {
                    if ui
                        .add_sized(
                            [142.0, 28.0],
                            egui::Button::new(match state {
                                GameState::Paused => "Workbench  F2",
                                _ => "Workbench  F2",
                            }),
                        )
                        .on_hover_text("Return to the Render Tuning Workbench without restarting.")
                        .clicked()
                    {
                        next_state.set(GameState::Workbench);
                    }
                });
        });
}

fn apply_matching_preset(
    preset: WorkbenchMatchingPreset,
    shape_config: &mut ShapeVectorConfig,
    candidates: &mut ShapeVectorGlyphCandidates,
) {
    match preset {
        WorkbenchMatchingPreset::OriginalMaterial => {
            shape_config.mode = ShapeVectorMode::OriginalOnly;
            shape_config.alphabet = ShapeVectorAlphabetId::Default;
            shape_config.distance_threshold = 0.08;
            shape_config.enable_contrast_adaptive_threshold = false;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = false;
        }
        WorkbenchMatchingPreset::LegibleTerrain => {
            shape_config.mode = ShapeVectorMode::Combined;
            shape_config.alphabet = ShapeVectorAlphabetId::Minimal;
            shape_config.distance_threshold = 0.075;
            shape_config.contrast_adaptive_threshold_boost = 0.25;
            shape_config.structural_fallback_distance_threshold = 0.20;
            shape_config.sampling_quality = 16;
            shape_config.enable_global_crunch = true;
            shape_config.enable_directional_crunch = true;
            shape_config.enable_contrast_adaptive_threshold = false;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = true;
            candidates.glyphs = glyph_set(b" .:-=+*#%@");
        }
        WorkbenchMatchingPreset::DenseDetail => {
            shape_config.mode = ShapeVectorMode::HarriPriority;
            shape_config.alphabet = ShapeVectorAlphabetId::Default;
            shape_config.distance_threshold = 0.12;
            shape_config.contrast_adaptive_threshold_boost = 0.45;
            shape_config.structural_fallback_distance_threshold = 0.28;
            shape_config.sampling_quality = 24;
            shape_config.enable_global_crunch = true;
            shape_config.enable_directional_crunch = true;
            shape_config.enable_contrast_adaptive_threshold = true;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = true;
            candidates.glyphs = glyph_set(b" .,:;irsXA253hMHGS#9B&@");
        }
        WorkbenchMatchingPreset::SilhouetteSafe => {
            shape_config.mode = ShapeVectorMode::Combined;
            shape_config.alphabet = ShapeVectorAlphabetId::Minimal;
            shape_config.distance_threshold = 0.055;
            shape_config.contrast_adaptive_threshold_boost = 0.15;
            shape_config.structural_fallback_distance_threshold = 0.16;
            shape_config.sampling_quality = 18;
            shape_config.enable_global_crunch = true;
            shape_config.enable_directional_crunch = true;
            shape_config.enable_contrast_adaptive_threshold = false;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = true;
            candidates.glyphs = glyph_set(b" .:-=+*#%@/\\|_[]()");
        }
        WorkbenchMatchingPreset::RainShadowContrast => {
            shape_config.mode = ShapeVectorMode::Combined;
            shape_config.alphabet = ShapeVectorAlphabetId::Default;
            shape_config.distance_threshold = 0.095;
            shape_config.contrast_adaptive_threshold_boost = 0.60;
            shape_config.structural_fallback_distance_threshold = 0.24;
            shape_config.sampling_quality = 20;
            shape_config.enable_global_crunch = true;
            shape_config.enable_directional_crunch = true;
            shape_config.enable_contrast_adaptive_threshold = true;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = true;
            candidates.glyphs = glyph_set(b" .,:;|/\\'`*+xX#%@");
        }
        WorkbenchMatchingPreset::WaterStress => {
            shape_config.mode = ShapeVectorMode::Combined;
            shape_config.alphabet = ShapeVectorAlphabetId::Default;
            shape_config.distance_threshold = 0.10;
            shape_config.contrast_adaptive_threshold_boost = 0.50;
            shape_config.structural_fallback_distance_threshold = 0.26;
            shape_config.sampling_quality = 22;
            shape_config.enable_global_crunch = true;
            shape_config.enable_directional_crunch = true;
            shape_config.enable_contrast_adaptive_threshold = true;
            shape_config.enable_structural_fallback = true;
            candidates.enabled = true;
            candidates.glyphs = glyph_set(b" ~_-=:;,.\\/|+*#%M");
        }
        WorkbenchMatchingPreset::Custom => {}
    }
}

fn glyph_set(bytes: &[u8]) -> BTreeSet<u8> {
    bytes.iter().copied().collect()
}

fn glyph_candidate_panel(ui: &mut egui::Ui, candidates: &mut ShapeVectorGlyphCandidates) {
    ui.horizontal_wrapped(|ui| {
        toggle_button(
            ui,
            "Custom glyphs",
            &mut candidates.enabled,
            "Use the selected CP437 glyphs as the shape-vector candidate set.",
        );
        if ui
            .add_sized([136.0, 28.0], egui::Button::new("Restore minimal"))
            .on_hover_text("Restore the compact ASCIIID-style glyph candidate set.")
            .clicked()
        {
            candidates.restore_minimal();
        }
        if ui
            .add_sized([82.0, 28.0], egui::Button::new("Clear"))
            .on_hover_text("Clear the custom candidate set. The renderer falls back to the selected alphabet when the set is empty.")
            .clicked()
        {
            candidates.glyphs.clear();
        }
    });
    metric_row(
        ui,
        "Active candidates",
        if candidates.is_active() {
            format!("{}", candidates.glyphs.len())
        } else {
            "alphabet".to_string()
        },
    );
    ui.horizontal_wrapped(|ui| {
        for glyph in candidates.glyphs.iter().copied().take(24) {
            ui.label(
                egui::RichText::new(glyph_label(glyph))
                    .monospace()
                    .color(egui::Color32::from_rgb(32, 32, 32)),
            );
        }
        if candidates.glyphs.len() > 24 {
            ui.label(format!("+{}", candidates.glyphs.len() - 24));
        }
    });
    ui.add_space(4.0);
    egui::Grid::new("cp437_candidate_grid")
        .num_columns(16)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for glyph in 0u16..=255 {
                let glyph = glyph as u8;
                let selected = candidates.glyphs.contains(&glyph);
                if ui
                    .add_sized(
                        [18.0, 18.0],
                        egui::Button::new(
                            egui::RichText::new(glyph_label(glyph))
                                .size(9.0)
                                .monospace(),
                        )
                        .selected(selected),
                    )
                    .on_hover_text(format!("CP437 glyph 0x{glyph:02X} / {glyph}"))
                    .clicked()
                {
                    if selected {
                        candidates.glyphs.remove(&glyph);
                    } else {
                        candidates.glyphs.insert(glyph);
                        candidates.enabled = true;
                    }
                }
                if glyph % 16 == 15 {
                    ui.end_row();
                }
            }
        });
}

fn material_probe_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cell_grid: &AsciiCellGrid,
    debug_grid: &RenderDebugGrid,
    camera: &GameCamera,
) {
    let Some(pointer) = ctx.pointer_hover_pos() else {
        metric_row(ui, "Hovered cell", "none".to_string());
        return;
    };
    let screen = ctx.content_rect();
    if cell_grid.width == 0 || cell_grid.height == 0 {
        metric_row(ui, "Hovered cell", "grid unavailable".to_string());
        return;
    }

    let cell_w = screen.width() / cell_grid.width as f32;
    let cell_h = screen.height() / cell_grid.height as f32;
    if cell_w <= 0.0 || cell_h <= 0.0 {
        metric_row(ui, "Hovered cell", "grid unavailable".to_string());
        return;
    }

    let cx = ((pointer.x - screen.left()) / cell_w).floor() as i32;
    let cy = ((pointer.y - screen.top()) / cell_h).floor() as i32;
    if cx < 0 || cy < 0 || cx >= cell_grid.width as i32 || cy >= cell_grid.height as i32 {
        metric_row(ui, "Hovered cell", "outside".to_string());
        return;
    }

    let idx = cy as usize * cell_grid.width as usize + cx as usize;
    let Some(debug) = debug_grid.cells.get(idx) else {
        metric_row(ui, "Hovered cell", format!("{cx}, {cy}"));
        return;
    };
    let glyph = cell_grid.char_indices.get(idx).copied().unwrap_or_default() as u8;
    let fg = cell_grid.fg_colors.get(idx).copied().unwrap_or_default();
    let bg = cell_grid.bg_colors.get(idx).copied().unwrap_or_default();
    let mat_id = debug.dominant_visual & 0x00FF;
    let mat_elev = (debug.dominant_visual & 0x8000) != 0;
    let height = debug
        .sample_heights
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > -999_999.0)
        .fold(None, |best: Option<f32>, value| {
            Some(best.map_or(value, |b| b.max(value)))
        });

    metric_row(ui, "Hovered cell", format!("{cx}, {cy}"));
    metric_row(ui, "Camera yaw", format!("{:.1} deg", camera.yaw));
    metric_row(ui, "Path", debug_path_label(debug.flags).to_string());
    metric_row(ui, "MAT-id", format!("0x{mat_id:02X} ({mat_id})"));
    metric_row(
        ui,
        "MAT-elev",
        format!("{}", if mat_elev { "set" } else { "clear" }),
    );
    metric_row(
        ui,
        "Dominant visual",
        format!("0x{:04X}", debug.dominant_visual),
    );
    metric_row(
        ui,
        "Lane / diffuse",
        format!("{} / {}", debug.material_lane, debug.diffuse_index),
    );
    metric_row(
        ui,
        "Height",
        height.map_or("clear".to_string(), |h| format!("{h:.1}")),
    );
    metric_row(
        ui,
        "Glyph resolve/final",
        format!(
            "{} -> {}",
            glyph_label(debug.resolve_glyph as u8),
            glyph_label(debug.final_glyph as u8)
        ),
    );
    metric_row(
        ui,
        "Cell glyph",
        format!("{} / 0x{glyph:02X}", glyph_label(glyph)),
    );
    metric_row(ui, "FG RGB", format!("{}, {}, {}", fg[0], fg[1], fg[2]));
    metric_row(ui, "BG RGB", format!("{}, {}, {}", bg[0], bg[1], bg[2]));
}

fn debug_path_label(flags: u32) -> &'static str {
    if flags & debug_flags::CLEAR != 0 {
        "clear/sky"
    } else if flags & debug_flags::USED_AUTO_MAT != 0 {
        "auto-mat"
    } else if flags & debug_flags::MESH_PATH != 0 {
        "mesh"
    } else if flags & debug_flags::MATERIAL_PATH != 0 {
        "material"
    } else {
        "unknown"
    }
}

fn glyph_label(glyph: u8) -> String {
    match glyph {
        b' ' => "sp".to_string(),
        33..=126 => char::from(glyph).to_string(),
        _ => format!("{glyph:02X}"),
    }
}

fn normalize_degrees(value: &mut f32) {
    while *value > 180.0 {
        *value -= 360.0;
    }
    while *value < -180.0 {
        *value += 360.0;
    }
}

fn help_panel(ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Control Reference")
        .default_open(true)
        .show(ui, |ui| {
            help_row(
                ui,
                "Resolution scale",
                "Changes render grid density; lower values reduce cell count.",
            );
            help_row(ui, "Zoom", "Changes projection scale.");
            help_row(
                ui,
                "Yaw",
                "Rotates the view in degrees. Keyboard parity: Q/E.",
            );
            help_row(
                ui,
                "Spin",
                "Automatically changes yaw over time for ASCIIID-style orbit inspection.",
            );
            help_row(
                ui,
                "Return path",
                "After Resume Scene, use F2 or the Workbench chip to return here.",
            );
            help_row(
                ui,
                "Camera X/Y/Z",
                "Moves the workbench camera. Keyboard parity for X/Y: A/D and W/S.",
            );
            help_row(
                ui,
                "Visibility",
                "Enables or disables terrain, mesh, sprite, shadow, reflection, and color-inversion passes.",
            );
            help_row(
                ui,
                "Palette Preview",
                "Remaps final cell colors through multi-color editor-inspired palettes without editing material or palette assets.",
            );
            help_row(
                ui,
                "Culling",
                "Switches between frustum/BSP culling and full traversal for terrain/world diagnostics.",
            );
            help_row(
                ui,
                "Weather",
                "Selects the debug precipitation state. Keyboard parity: F5 cycles weather states.",
            );
            help_row(
                ui,
                "Matching presets",
                "Applies bundled glyph mode, alphabet, threshold, fallback, and custom candidate settings.",
            );
            help_row(
                ui,
                "Advanced glyph controls",
                "Controls original/combined/Harri glyph selection, alphabets, custom CP437 candidates, thresholds, and fallback tuning.",
            );
            help_row(
                ui,
                "Threshold sliders",
                "Tune shape-vector distance, adaptive boost, fallback distance, sampling quality, and crunch exponents. Keyboard parity: [], 7/8, 9/0, -=, ;', and ,/.",
            );
            help_row(
                ui,
                "Binary glyph toggles",
                "Enable adaptive threshold, structural fallback, global crunch, and directional crunch. Keyboard parity: F11, F10, F7, and F8.",
            );
            help_row(
                ui,
                "Material lanes",
                "Shows and adjusts the four material/elevation shade lanes with tint, solo, enable, weight, contrast, and diffuse preview controls.",
            );
            help_row(
                ui,
                "Diagnostics",
                "Reports stage timings, pass-effect proof, shape-vector counters, and hovered-cell material probe data.",
            );
        });
}

fn help_row(ui: &mut egui::Ui, label: &str, description: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(label)
                .strong()
                .color(egui::Color32::from_rgb(48, 48, 48)),
        );
        ui.label(description);
    });
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(13.0)
            .strong()
            .color(egui::Color32::from_rgb(88, 82, 74)),
    );
    ui.add_space(6.0);
}

fn slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
    help: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([190.0, 18.0], egui::Label::new(label))
            .on_hover_text(help);
        value_label(ui, format!("{value:.2}"));
    });
    ui.add(
        egui::Slider::new(value, range)
            .step_by(step)
            .show_value(false),
    )
    .on_hover_text(help);
}

fn enabled_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
    enabled: bool,
    help: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([190.0, 18.0], egui::Label::new(label))
            .on_hover_text(help);
        value_label(ui, format!("{value:.2}"));
    });
    ui.add_enabled(
        enabled,
        egui::Slider::new(value, range)
            .step_by(step)
            .show_value(false),
    )
    .on_hover_text(help);
}

fn int_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    help: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([190.0, 18.0], egui::Label::new(label))
            .on_hover_text(help);
        value_label(ui, format!("{value}"));
    });
    ui.add(egui::Slider::new(value, range).show_value(false))
        .on_hover_text(help);
}

fn stepper_row(ui: &mut egui::Ui, label: &str, value: &mut f32, step: f32, suffix: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([130.0, 18.0], egui::Label::new(label));
        if ui
            .add_sized([26.0, 24.0], egui::Button::new("-"))
            .on_hover_text("Decrease by the keyboard step amount.")
            .clicked()
        {
            *value -= step;
        }
        value_label(ui, format!("{value:.1} {suffix}"));
        if ui
            .add_sized([26.0, 24.0], egui::Button::new("+"))
            .on_hover_text("Increase by the keyboard step amount.")
            .clicked()
        {
            *value += step;
        }
    });
}

fn camera_nudge_row(ui: &mut egui::Ui, camera: &mut GameCamera) {
    let step = 0.5;
    ui.label("Camera nudge");
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_sized([42.0, 28.0], egui::Button::new("W"))
            .on_hover_text("Move forward, matching W keyboard input.")
            .clicked()
        {
            camera.pos[1] += step;
        }
        if ui
            .add_sized([42.0, 28.0], egui::Button::new("A"))
            .on_hover_text("Move left, matching A keyboard input.")
            .clicked()
        {
            camera.pos[0] -= step;
        }
        if ui
            .add_sized([42.0, 28.0], egui::Button::new("S"))
            .on_hover_text("Move backward, matching S keyboard input.")
            .clicked()
        {
            camera.pos[1] -= step;
        }
        if ui
            .add_sized([42.0, 28.0], egui::Button::new("D"))
            .on_hover_text("Move right, matching D keyboard input.")
            .clicked()
        {
            camera.pos[0] += step;
        }
    });
}

fn enum_row(ui: &mut egui::Ui, label: &str, add_controls: impl FnOnce(&mut egui::Ui)) {
    ui.label(label);
    ui.horizontal_wrapped(add_controls);
}

fn enum_button<T>(ui: &mut egui::Ui, label: &str, current: &mut T, value: T, help: &str)
where
    T: PartialEq + Copy,
{
    let selected = *current == value;
    if ui
        .add_sized([96.0, 28.0], egui::Button::new(label).selected(selected))
        .on_hover_text(help)
        .clicked()
    {
        *current = value;
    }
}

fn matching_preset_button(
    ui: &mut egui::Ui,
    label: &str,
    value: WorkbenchMatchingPreset,
    current: &mut WorkbenchMatchingPreset,
    shape_config: &mut ShapeVectorConfig,
    candidates: &mut ShapeVectorGlyphCandidates,
) {
    let selected = *current == value;
    if ui
        .add_sized([104.0, 30.0], egui::Button::new(label).selected(selected))
        .on_hover_text(value.purpose())
        .clicked()
    {
        *current = value;
        apply_matching_preset(value, shape_config, candidates);
    }
}

fn palette_button(
    ui: &mut egui::Ui,
    label: &str,
    current: &mut WorkbenchPalettePreview,
    value: WorkbenchPalettePreview,
    help: &str,
) {
    let selected = *current == value;
    if ui
        .add_sized([96.0, 28.0], egui::Button::new(label).selected(selected))
        .on_hover_text(help)
        .clicked()
    {
        *current = value;
    }
}

fn material_lane_panel(ui: &mut egui::Ui, workbench: &mut RenderWorkbenchState) {
    ui.horizontal_wrapped(|ui| {
        toggle_button(
            ui,
            "MAT-elev tint",
            &mut workbench.mat_elev_preview,
            "Tints resolved colors by material lane so elevation-ramp routing is visible.",
        );
        if ui
            .add_sized([136.0, 28.0], egui::Button::new("Reset lanes"))
            .on_hover_text("Restore all material lanes to enabled, unsoloed, neutral weight, and neutral contrast.")
            .clicked()
        {
            workbench.lane_enabled = [true; 4];
            workbench.lane_solo = [false; 4];
            workbench.lane_weight = [1.0; 4];
            workbench.lane_contrast = [1.0; 4];
            workbench.mat_elev_preview = false;
        }
    });
    let mut diffuse = u32::from(workbench.diffuse_preview_index);
    int_slider_row(
        ui,
        "Diffuse preview",
        &mut diffuse,
        0..=15,
        "Scrubs the documented material shade column for lane comparison and probe readback.",
    );
    workbench.diffuse_preview_index = diffuse.min(15) as u8;

    for lane in 0..4 {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let color = lane_tint(lane);
            let swatch = egui::RichText::new("  ")
                .background_color(egui::Color32::from_rgb(color[0], color[1], color[2]));
            ui.add_sized([20.0, 18.0], egui::Label::new(swatch))
                .on_hover_text(format!("Lane {lane} tint"));
            ui.add_sized([60.0, 18.0], egui::Label::new(format!("Lane {lane}")));
            compact_toggle_button(
                ui,
                "On",
                &mut workbench.lane_enabled[lane],
                "Enables this material/elevation lane in the preview.",
            );
            compact_toggle_button(
                ui,
                "Solo",
                &mut workbench.lane_solo[lane],
                "Dims every other lane so this one can be inspected by itself.",
            );
        });
        slider_row(
            ui,
            "Lane weight",
            &mut workbench.lane_weight[lane],
            0.0..=2.0,
            0.05,
            "Brightness weight for this material/elevation lane preview.",
        );
        slider_row(
            ui,
            "Lane contrast",
            &mut workbench.lane_contrast[lane],
            0.25..=2.5,
            0.05,
            "Contrast multiplier for this material/elevation lane preview.",
        );
    }
}

fn weather_button(ui: &mut egui::Ui, label: &str, weather: &mut Weather, value: WeatherState) {
    let selected = weather.state == value;
    if ui
        .add_sized([96.0, 28.0], egui::Button::new(label).selected(selected))
        .on_hover_text("Sets the debug weather state. Keyboard parity: F5 cycles this list.")
        .clicked()
    {
        set_weather_state(weather, value);
    }
}

fn toggle_button(ui: &mut egui::Ui, label: &str, value: &mut bool, help: &str) {
    if ui
        .add_sized([136.0, 28.0], egui::Button::new(label).selected(*value))
        .on_hover_text(help)
        .clicked()
    {
        *value = !*value;
    }
}

fn compact_toggle_button(ui: &mut egui::Ui, label: &str, value: &mut bool, help: &str) {
    if ui
        .add_sized([58.0, 24.0], egui::Button::new(label).selected(*value))
        .on_hover_text(help)
        .clicked()
    {
        *value = !*value;
    }
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [132.0, 18.0],
            egui::Label::new(
                egui::RichText::new(label)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(102, 96, 88)),
            ),
        );
        value_label(ui, value);
    });
}

fn value_label(ui: &mut egui::Ui, value: String) {
    ui.add_sized(
        [190.0, 18.0],
        egui::Label::new(
            egui::RichText::new(value)
                .monospace()
                .color(egui::Color32::from_rgb(32, 32, 32)),
        ),
    );
}

fn micros_to_ms(value: u64) -> String {
    format!("{:.2} ms", value as f32 / 1000.0)
}

fn percent(stats: &ShapeVectorFrameStats, value: u32) -> String {
    format!("{:.1}%", stats.percent_of_total(value))
}

fn shape_mode_label(mode: ShapeVectorMode) -> &'static str {
    match mode {
        ShapeVectorMode::OriginalOnly => "Original only",
        ShapeVectorMode::Combined => "Combined",
        ShapeVectorMode::HarriPriority => "Harri priority",
    }
}

fn alphabet_label(alphabet: ShapeVectorAlphabetId) -> &'static str {
    match alphabet {
        ShapeVectorAlphabetId::Default => "Default",
        ShapeVectorAlphabetId::Minimal => "Minimal",
    }
}
