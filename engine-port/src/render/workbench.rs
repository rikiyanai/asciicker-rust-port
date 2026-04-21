use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use crate::game::state::GameState;
use crate::game::weather::{PrecipitationType, Weather, WeatherState, set_weather_state};
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
    pub show_help: bool,
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
            show_help: false,
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
            .add_systems(EguiPrimaryContextPass, render_workbench_ui_system);
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
    mut weather: ResMut<Weather>,
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
                                    workbench.reset(&mut camera, &mut shape_config);
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
                                "Particles",
                                format!("{}", weather.pool.active_count()),
                            );

                            ui.add_space(14.0);
                            section_label(ui, "Glyph Matching");
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
                "Glyph mode/alphabet",
                "Controls original/combined/Harri glyph selection and alphabet choice. Keyboard parity: F12 and F6.",
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
                "Diagnostics",
                "Reports the same runtime state summarized in the window title plus stage timings and shape-vector counters.",
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
        ui.label(label).on_hover_text(help);
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
        ui.label(label).on_hover_text(help);
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
        ui.label(label).on_hover_text(help);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{value}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
        });
    });
    ui.add(egui::Slider::new(value, range).show_value(false))
        .on_hover_text(help);
}

fn stepper_row(ui: &mut egui::Ui, label: &str, value: &mut f32, step: f32, suffix: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized([26.0, 24.0], egui::Button::new("+"))
                .on_hover_text("Increase by the keyboard step amount.")
                .clicked()
            {
                *value += step;
            }
            ui.label(
                egui::RichText::new(format!("{value:.1} {suffix}"))
                    .monospace()
                    .color(egui::Color32::from_rgb(42, 42, 42)),
            );
            if ui
                .add_sized([26.0, 24.0], egui::Button::new("-"))
                .on_hover_text("Decrease by the keyboard step amount.")
                .clicked()
            {
                *value -= step;
            }
        });
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
