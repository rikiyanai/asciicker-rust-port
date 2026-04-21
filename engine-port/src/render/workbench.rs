use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};

use crate::game::state::GameState;
use crate::output::ascii_cell_grid::AsciiCellGrid;

use super::camera::GameCamera;
use super::config::RenderConfig;
use super::pipeline::PipelineTiming;
use super::shape_vector::{
    ShapeVectorAlphabetId, ShapeVectorConfig, ShapeVectorFrameStats, ShapeVectorMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchFixture {
    Scene,
    Terrain,
    Meshes,
    Sprites,
    Water,
}

impl WorkbenchFixture {
    const ALL: [Self; 5] = [
        Self::Scene,
        Self::Terrain,
        Self::Meshes,
        Self::Sprites,
        Self::Water,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Terrain => "Terrain",
            Self::Meshes => "Meshes",
            Self::Sprites => "Sprites",
            Self::Water => "Water",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphPreset {
    Dense,
    Sparse,
}

impl GlyphPreset {
    fn label(self) -> &'static str {
        match self {
            Self::Dense => ".:-=+*#%@",
            Self::Sparse => ".-+*#",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct RenderWorkbenchState {
    pub visible: bool,
    pub fixture: WorkbenchFixture,
    pub glyph_preset: GlyphPreset,
    pub resolution_scale: f32,
    pub invert_colors: bool,
    pub show_terrain: bool,
    pub show_meshes: bool,
    pub show_sprites: bool,
    pub enable_shadows: bool,
    pub enable_reflections: bool,
    pub terrain_culling: bool,
    pub world_culling: bool,
}

impl Default for RenderWorkbenchState {
    fn default() -> Self {
        Self {
            visible: true,
            fixture: WorkbenchFixture::Scene,
            glyph_preset: GlyphPreset::Dense,
            resolution_scale: 1.0,
            invert_colors: false,
            show_terrain: true,
            show_meshes: true,
            show_sprites: true,
            enable_shadows: true,
            enable_reflections: true,
            terrain_culling: true,
            world_culling: true,
        }
    }
}

impl RenderWorkbenchState {
    fn apply_fixture(&mut self, fixture: WorkbenchFixture) {
        self.fixture = fixture;
        match fixture {
            WorkbenchFixture::Scene => {
                self.show_terrain = true;
                self.show_meshes = true;
                self.show_sprites = true;
                self.enable_shadows = true;
                self.enable_reflections = true;
            }
            WorkbenchFixture::Terrain => {
                self.show_terrain = true;
                self.show_meshes = false;
                self.show_sprites = false;
                self.enable_shadows = false;
                self.enable_reflections = false;
            }
            WorkbenchFixture::Meshes => {
                self.show_terrain = false;
                self.show_meshes = true;
                self.show_sprites = false;
                self.enable_shadows = false;
                self.enable_reflections = false;
            }
            WorkbenchFixture::Sprites => {
                self.show_terrain = false;
                self.show_meshes = false;
                self.show_sprites = true;
                self.enable_shadows = false;
                self.enable_reflections = false;
            }
            WorkbenchFixture::Water => {
                self.show_terrain = true;
                self.show_meshes = true;
                self.show_sprites = false;
                self.enable_shadows = false;
                self.enable_reflections = true;
            }
        }
    }

    fn reset(&mut self, camera: &mut GameCamera, shape: &mut ShapeVectorConfig) {
        *self = Self::default();
        *camera = GameCamera::default();
        *shape = ShapeVectorConfig::default();
    }
}

pub struct RenderWorkbenchPlugin;

impl Plugin for RenderWorkbenchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderWorkbenchState>()
            .add_plugins(EguiPlugin::default())
            .add_systems(Update, toggle_workbench_visibility_system)
            .add_systems(Update, render_workbench_ui_system);
    }
}

fn toggle_workbench_visibility_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<RenderWorkbenchState>,
) {
    if keys.just_pressed(KeyCode::Backquote) {
        state.visible = !state.visible;
    }
}

fn render_workbench_ui_system(
    mut contexts: EguiContexts,
    game_state: Option<Res<State<GameState>>>,
    mut workbench: ResMut<RenderWorkbenchState>,
    mut render_config: ResMut<RenderConfig>,
    mut camera: ResMut<GameCamera>,
    mut shape_config: ResMut<ShapeVectorConfig>,
    cell_grid: Res<AsciiCellGrid>,
    timing: Res<PipelineTiming>,
    stats: Res<ShapeVectorFrameStats>,
) {
    if matches!(
        game_state.as_deref().map(State::get),
        Some(GameState::MainMenu | GameState::Loading)
    ) {
        return;
    }

    if !workbench.visible {
        return;
    }

    let ctx = contexts.ctx_mut().expect("primary egui context");
    let mut visuals = egui::Visuals::light();
    visuals.window_fill = egui::Color32::from_rgba_premultiplied(252, 250, 246, 232);
    visuals.panel_fill = egui::Color32::from_rgba_premultiplied(252, 250, 246, 232);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(30, 30, 30);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 231, 222);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(246, 241, 233);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(70, 70, 70);
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
    ctx.set_visuals(visuals);

    let screen = ctx.content_rect();
    let panel_fill = egui::Color32::from_rgba_premultiplied(252, 250, 246, 232);
    let panel_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(214, 206, 194));

    egui::Area::new("render_workbench_left".into())
        .fixed_pos(egui::pos2(screen.left() + 24.0, screen.center().y - 120.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(panel_fill)
                .stroke(panel_stroke)
                .show(ui, |ui| {
                    ui.set_min_width(110.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Fixtures")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(106, 102, 96)),
                        );
                        ui.add_space(8.0);
                        for fixture in WorkbenchFixture::ALL {
                            let selected = workbench.fixture == fixture;
                            let response = ui.add_sized(
                                [86.0, 28.0],
                                egui::Button::new(fixture.label()).selected(selected),
                            );
                            if response.clicked() {
                                workbench.apply_fixture(fixture);
                            }
                            ui.add_space(6.0);
                        }
                    });
                });
        });

    egui::Area::new("render_workbench_right".into())
        .fixed_pos(egui::pos2(screen.right() - 248.0, screen.top() + 84.0))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(panel_fill)
                .stroke(panel_stroke)
                .show(ui, |ui| {
                    ui.set_min_width(224.0);
                    ui.vertical(|ui| {
                        section_label(ui, "Presets");
                        ui.horizontal(|ui| {
                            for preset in [GlyphPreset::Dense, GlyphPreset::Sparse] {
                                let selected = workbench.glyph_preset == preset;
                                let response = ui.add_sized(
                                    [96.0, 28.0],
                                    egui::Button::new(preset.label()).selected(selected),
                                );
                                if response.clicked() {
                                    workbench.glyph_preset = preset;
                                    apply_glyph_preset(preset, &mut shape_config);
                                }
                            }
                        });

                        ui.add_space(12.0);
                        section_label(ui, "Resolution");
                        slider_row(
                            ui,
                            "Resolution",
                            &mut workbench.resolution_scale,
                            0.25..=1.0,
                            0.01,
                        );

                        ui.add_space(12.0);
                        section_label(ui, "Scale");
                        slider_row(ui, "Scale", &mut camera.zoom, 0.5..=3.0, 0.01);

                        ui.add_space(12.0);
                        section_label(ui, "Render");
                        ui.horizontal_wrapped(|ui| {
                            toggle_button(ui, "Invert colors", &mut workbench.invert_colors);
                            toggle_button(ui, "Terrain", &mut workbench.show_terrain);
                            toggle_button(ui, "Meshes", &mut workbench.show_meshes);
                            toggle_button(ui, "Sprites", &mut workbench.show_sprites);
                            toggle_button(ui, "Shadows", &mut workbench.enable_shadows);
                            toggle_button(ui, "Reflections", &mut workbench.enable_reflections);
                        });

                        ui.add_space(12.0);
                        section_label(ui, "Culling");
                        ui.horizontal_wrapped(|ui| {
                            toggle_button(ui, "Terrain cull", &mut workbench.terrain_culling);
                            toggle_button(ui, "World cull", &mut workbench.world_culling);
                        });

                        ui.add_space(12.0);
                        section_label(ui, "Metrics");
                        metric_row(ui, "Grid", format!("{} x {}", cell_grid.width, cell_grid.height));
                        metric_row(
                            ui,
                            "Sample",
                            format!("{} x {}", render_config.sample_width(), render_config.sample_height()),
                        );
                        metric_row(ui, "Frame", format!("{:.2} ms", timing.total_us as f32 / 1000.0));
                        metric_row(ui, "Resolve", format!("{:.2} ms", timing.resolve_us as f32 / 1000.0));
                        metric_row(
                            ui,
                            "Overrides",
                            format!("{:.0}%", stats.percent_of_total(stats.selector_override_cells)),
                        );
                        metric_row(
                            ui,
                            "Fallback",
                            format!("{:.0}%", stats.percent_of_total(stats.resolve_fallback_cells)),
                        );

                        ui.add_space(14.0);
                        if ui
                            .add_sized([196.0, 32.0], egui::Button::new("Reset"))
                            .clicked()
                        {
                            workbench.reset(&mut camera, &mut shape_config);
                            render_config.ascii_width = cell_grid.width;
                            render_config.ascii_height = cell_grid.height;
                        }

                        ui.add_space(8.0);
                        ui.hyperlink_to(
                            "Credits",
                            "https://www.figma.com/community/file/1530223431472150953/3d-ascii-model-viewer",
                        );
                    });
                });
        });
}

fn apply_glyph_preset(preset: GlyphPreset, shape: &mut ShapeVectorConfig) {
    match preset {
        GlyphPreset::Dense => {
            *shape = ShapeVectorConfig::default();
        }
        GlyphPreset::Sparse => {
            shape.mode = ShapeVectorMode::Combined;
            shape.alphabet = ShapeVectorAlphabetId::Minimal;
            shape.distance_threshold = 0.18;
            shape.global_crunch_exponent = 2.0;
            shape.directional_crunch_exponent = 4.0;
            shape.sampling_quality = 6;
            shape.enable_global_crunch = true;
            shape.enable_directional_crunch = true;
            shape.enable_contrast_adaptive_threshold = false;
            shape.enable_structural_fallback = false;
        }
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(12.0)
            .color(egui::Color32::from_rgb(106, 102, 96)),
    );
}

fn slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(
            egui::RichText::new(format!("{value:.2}"))
                .monospace()
                .color(egui::Color32::from_rgb(60, 60, 60)),
        );
    });
    ui.add(
        egui::Slider::new(value, range)
            .step_by(step)
            .show_value(false),
    );
}

fn toggle_button(ui: &mut egui::Ui, label: &str, value: &mut bool) {
    let response = ui.add_sized([96.0, 28.0], egui::Button::new(label).selected(*value));
    if response.clicked() {
        *value = !*value;
    }
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(11.0)
                .color(egui::Color32::from_rgb(106, 102, 96)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .monospace()
                    .color(egui::Color32::from_rgb(32, 32, 32)),
            );
        });
    });
}
