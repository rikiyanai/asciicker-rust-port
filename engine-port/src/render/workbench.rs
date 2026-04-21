use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, egui};

use crate::game::state::GameState;
use crate::output::ascii_cell_grid::AsciiCellGrid;

use super::camera::GameCamera;
use super::pipeline::PipelineTiming;
use super::shape_vector::{
    ShapeVectorAlphabetId, ShapeVectorConfig, ShapeVectorFrameStats, ShapeVectorMode,
};

#[derive(Resource, Debug, Clone)]
pub struct RenderWorkbenchState {
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
    pub fn reset(&mut self, camera: &mut GameCamera, shape: &mut ShapeVectorConfig) {
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
            .add_systems(Update, render_workbench_ui_system);
    }
}

fn render_workbench_ui_system(
    mut contexts: EguiContexts,
    game_state: Option<Res<State<GameState>>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut workbench: ResMut<RenderWorkbenchState>,
    mut camera: ResMut<GameCamera>,
    mut shape_config: ResMut<ShapeVectorConfig>,
    cell_grid: Res<AsciiCellGrid>,
    timing: Res<PipelineTiming>,
    stats: Res<ShapeVectorFrameStats>,
) {
    let Some(game_state) = game_state.as_deref() else {
        return;
    };
    if *game_state.get() != GameState::Workbench {
        return;
    }

    let ctx = contexts.ctx_mut().expect("primary egui context");
    apply_workbench_visuals(ctx);

    let screen = ctx.content_rect();
    let frame_fill = egui::Color32::from_rgba_premultiplied(248, 245, 238, 236);
    let frame_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(204, 196, 182));
    let sample_width = 2 * cell_grid.width + 4;
    let sample_height = 2 * cell_grid.height + 4;

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
                                .add_sized([118.0, 30.0], egui::Button::new("Resume Scene"))
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
                    egui::ScrollArea::vertical()
                        .max_height((screen.height() - 40.0).max(320.0))
                        .show(ui, |ui| {
                            section_label(ui, "View");
                            slider_row(
                                ui,
                                "Resolution scale",
                                &mut workbench.resolution_scale,
                                0.25..=1.0,
                                0.01,
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
                            slider_row(ui, "Zoom", &mut camera.zoom, 0.5..=3.0, 0.01);
                            stepper_row(ui, "Yaw", &mut camera.yaw, 15.0, "deg");
                            metric_row(
                                ui,
                                "Camera pos",
                                format!(
                                    "{:.1}, {:.1}, {:.1}",
                                    camera.pos[0], camera.pos[1], camera.pos[2]
                                ),
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Visibility");
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(ui, "Terrain", &mut workbench.show_terrain);
                                toggle_button(ui, "Meshes", &mut workbench.show_meshes);
                                toggle_button(ui, "Sprites", &mut workbench.show_sprites);
                                toggle_button(ui, "Shadows", &mut workbench.enable_shadows);
                                toggle_button(ui, "Reflections", &mut workbench.enable_reflections);
                                toggle_button(ui, "Invert colors", &mut workbench.invert_colors);
                            });

                            ui.add_space(14.0);
                            section_label(ui, "Culling");
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Terrain culling",
                                    &mut workbench.terrain_culling,
                                );
                                toggle_button(ui, "World culling", &mut workbench.world_culling);
                            });

                            ui.add_space(14.0);
                            section_label(ui, "Glyph Matching");
                            enum_row(ui, "Mode", |ui| {
                                enum_button(
                                    ui,
                                    "Original",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::OriginalOnly,
                                );
                                enum_button(
                                    ui,
                                    "Combined",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::Combined,
                                );
                                enum_button(
                                    ui,
                                    "Harri",
                                    &mut shape_config.mode,
                                    ShapeVectorMode::HarriPriority,
                                );
                            });
                            enum_row(ui, "Alphabet", |ui| {
                                enum_button(
                                    ui,
                                    "Default",
                                    &mut shape_config.alphabet,
                                    ShapeVectorAlphabetId::Default,
                                );
                                enum_button(
                                    ui,
                                    "Minimal",
                                    &mut shape_config.alphabet,
                                    ShapeVectorAlphabetId::Minimal,
                                );
                            });
                            slider_row(
                                ui,
                                "Distance threshold",
                                &mut shape_config.distance_threshold,
                                0.0..=1.0,
                                0.005,
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Adaptive threshold",
                                    &mut shape_config.enable_contrast_adaptive_threshold,
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
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Structural fallback",
                                    &mut shape_config.enable_structural_fallback,
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
                            );
                            let mut sampling_quality = shape_config.sampling_quality as u32;
                            int_slider_row(ui, "Sampling quality", &mut sampling_quality, 1..=32);
                            shape_config.sampling_quality = sampling_quality as usize;
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Global crunch",
                                    &mut shape_config.enable_global_crunch,
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
                            );
                            ui.horizontal_wrapped(|ui| {
                                toggle_button(
                                    ui,
                                    "Directional crunch",
                                    &mut shape_config.enable_directional_crunch,
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
                            );

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
                                "Colored blanks",
                                percent(&stats, stats.colored_space_cells),
                            );

                            ui.add_space(14.0);
                            if ui
                                .add_sized(
                                    [320.0, 34.0],
                                    egui::Button::new("Reset To Documented Defaults"),
                                )
                                .clicked()
                            {
                                workbench.reset(&mut camera, &mut shape_config);
                            }
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
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{value:.2}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
        });
    });
    ui.add(
        egui::Slider::new(value, range)
            .step_by(step)
            .show_value(false),
    );
}

fn enabled_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
    enabled: bool,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{value:.2}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
        });
    });
    ui.add_enabled(
        enabled,
        egui::Slider::new(value, range)
            .step_by(step)
            .show_value(false),
    );
}

fn int_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{value}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
        });
    });
    ui.add(egui::Slider::new(value, range).show_value(false));
}

fn stepper_row(ui: &mut egui::Ui, label: &str, value: &mut f32, step: f32, suffix: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_sized([26.0, 24.0], egui::Button::new("+")).clicked() {
                *value += step;
            }
            ui.label(
                egui::RichText::new(format!("{value:.1} {suffix}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
            if ui.add_sized([26.0, 24.0], egui::Button::new("-")).clicked() {
                *value -= step;
            }
        });
    });
}

fn enum_row(ui: &mut egui::Ui, label: &str, add_controls: impl FnOnce(&mut egui::Ui)) {
    ui.label(label);
    ui.horizontal_wrapped(add_controls);
}

fn enum_button<T>(ui: &mut egui::Ui, label: &str, current: &mut T, value: T)
where
    T: PartialEq + Copy,
{
    let selected = *current == value;
    if ui
        .add_sized([96.0, 28.0], egui::Button::new(label).selected(selected))
        .clicked()
    {
        *current = value;
    }
}

fn toggle_button(ui: &mut egui::Ui, label: &str, value: &mut bool) {
    if ui
        .add_sized([136.0, 28.0], egui::Button::new(label).selected(*value))
        .clicked()
    {
        *value = !*value;
    }
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(11.0)
                .color(egui::Color32::from_rgb(102, 96, 88)),
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
