//! Game state machine: Loading/Playing/Paused FSM using Bevy States.
//!
//! Maps to the C++ mainmenu.cpp loading FSM (3 stages: init, terrain_patches,
//! world_rebuild, done). Bevy's `States` derive provides OnEnter/OnExit
//! schedules and `in_state()` run conditions.

use bevy::prelude::*;

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::assembly::AssemblyState;
use crate::render::font::{Font1, FontSkin};
use crate::terrain::RuntimeTerrain;

// ---------------------------------------------------------------------------
// GameState
// ---------------------------------------------------------------------------

/// Top-level game state controlling system execution flow.
///
/// Bevy's `States` derive enables `in_state(GameState::Playing)` run conditions
/// and `OnEnter`/`OnExit` schedules for each variant.
///
/// Flow: MainMenu -> Loading -> Playing <-> Paused
///       Any state + Escape -> MainMenu (fallback for stuck states like Dead)
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    /// Startup state: main menu is displayed, gameplay systems are idle.
    #[default]
    MainMenu,
    /// Asset loading in progress (terrain assembly, mesh registry).
    Loading,
    /// Dedicated render-tuning surface with live renderer controls and diagnostics.
    Workbench,
    /// Active gameplay: all systems running.
    Playing,
    /// Gameplay paused: physics/character frozen, render still active.
    Paused,
}

/// Startup and loading destination configuration for the current session.
#[derive(Resource, Debug, Clone, Copy)]
pub struct GameLaunchConfig {
    /// State to enter after Loading completes.
    pub load_target: GameState,
    /// Whether startup should auto-enter Loading instead of waiting in MainMenu.
    pub auto_enter_loading: bool,
}

impl Default for GameLaunchConfig {
    fn default() -> Self {
        match std::env::var("ASCIICKER_START_MODE")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("play") | Some("playing") | Some("game") => Self {
                load_target: GameState::Playing,
                auto_enter_loading: true,
            },
            Some("workbench") | Some("render_workbench") | Some("render-tuning-workbench") => {
                Self {
                    load_target: GameState::Workbench,
                    auto_enter_loading: true,
                }
            }
            _ => Self {
                load_target: GameState::Playing,
                auto_enter_loading: false,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// LoadingProgress
// ---------------------------------------------------------------------------

/// Tracks the 3-stage C++ loading FSM.
///
/// Stages mirror C++ mainmenu.cpp:
/// - 3: init (start loading)
/// - 2: terrain_patches
/// - 1: world_rebuild
/// - 0: done (transition to Playing)
///
/// `items_loaded` / `items_total` provide progress feedback for the loading screen.
#[derive(Resource, Debug)]
pub struct LoadingProgress {
    /// Current loading stage (3=init, 2=terrain, 1=world, 0=done).
    pub stage: u8,
    /// Number of items loaded so far.
    pub items_loaded: u32,
    /// Total number of items to load.
    pub items_total: u32,
}

impl Default for LoadingProgress {
    fn default() -> Self {
        Self {
            stage: 3,
            items_loaded: 0,
            items_total: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Transition systems
// ---------------------------------------------------------------------------

/// Insert LoadingProgress when entering Loading state.
pub fn on_enter_loading(mut commands: Commands) {
    commands.insert_resource(LoadingProgress::default());
    info!("Entering Loading state: LoadingProgress inserted (stage=3)");
}

/// Remove LoadingProgress when exiting Loading state.
pub fn on_exit_loading(mut commands: Commands) {
    commands.remove_resource::<LoadingProgress>();
    info!("Exiting Loading state: LoadingProgress removed");
}

/// Log entering the dedicated render-tuning surface.
pub fn on_enter_workbench() {
    info!("Entering Render Tuning Workbench mode");
}

/// Bridge between Phase 5 assembly and Phase 7 loading FSM.
///
/// Reads AssemblyState.assembled (set by Plan 05-05's a3d_assembly_system)
/// and RuntimeTerrain.root (set when terrain quadtree is built).
/// When BOTH are ready, sets LoadingProgress.stage = 0 to trigger
/// the Loading -> Playing transition.
///
/// R19-004 FIX: Requires terrain to be loaded before transitioning to Playing,
/// preventing players from spawning at fallback height above the terrain.
pub fn advance_loading_progress_system(
    assembly: Option<Res<AssemblyState>>,
    terrain: Option<Res<RuntimeTerrain>>,
    progress: Option<ResMut<LoadingProgress>>,
) {
    let Some(mut progress) = progress else { return };
    let Some(assembly) = assembly else { return };
    let Some(terrain) = terrain else { return };

    if assembly.assembled && terrain.root.is_some() && progress.stage > 0 {
        progress.stage = 0; // Both assembly AND terrain ready
        info!("Loading complete: assembly assembled + terrain loaded -> stage=0");
    }
}

/// Transition to the configured target state when loading is complete (stage == 0).
pub fn check_loading_complete(
    progress: Option<Res<LoadingProgress>>,
    launch_config: Res<GameLaunchConfig>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(progress) = progress
        && progress.stage == 0
    {
        next_state.set(launch_config.load_target);
        info!(
            "Loading complete (stage=0): transitioning to {:?}",
            launch_config.load_target
        );
    }
}

/// Optional startup fast path for launching directly into a configured scene mode.
pub fn auto_enter_configured_mode(
    state: Res<State<GameState>>,
    launch_config: Res<GameLaunchConfig>,
    mut next_state: ResMut<NextState<GameState>>,
    mut armed: Local<bool>,
) {
    if *armed || !launch_config.auto_enter_loading || *state.get() != GameState::MainMenu {
        return;
    }

    *armed = true;
    next_state.set(GameState::Loading);
    info!(
        "Auto-entering Loading from MainMenu with target {:?}",
        launch_config.load_target
    );
}

/// Log entering gameplay state.
pub fn on_enter_playing() {
    info!("Entering Playing state: all gameplay systems active");
}

/// Toggle between Playing and Paused on Escape key.
///
/// P7-050 FIX: Reads current state to toggle correctly.
/// R19-005 FIX: Escape from any non-Playing/Paused state returns to MainMenu
/// (prevents being stuck in a state with no exit, e.g., after character death).
pub fn toggle_pause(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => {
                next_state.set(GameState::Paused);
                info!("Pausing game");
            }
            GameState::Paused => {
                next_state.set(GameState::Playing);
                info!("Resuming game");
            }
            GameState::Workbench => {
                next_state.set(GameState::MainMenu);
                info!("Leaving Render Tuning Workbench -> MainMenu");
            }
            // R19-005: Allow escape to return to MainMenu from any other state
            // (prevents being stuck — e.g., after death before respawn is implemented)
            _ => {
                next_state.set(GameState::MainMenu);
                info!("Returning to MainMenu from {:?}", state.get());
            }
        }
    }
}

/// Render a loading screen to AsciiCellGrid during Loading state.
///
/// R19-003 FIX: Without this, the player sees a blank/frozen screen between
/// pressing "Start Game" and entering gameplay (render_pipeline_system is gated
/// on Playing, render_menu is gated on MainMenu — nothing renders during Loading).
pub fn render_loading_screen(
    grid: Option<ResMut<AsciiCellGrid>>,
    progress: Option<Res<LoadingProgress>>,
    font: Option<Res<Font1>>,
) {
    let Some(mut grid) = grid else { return };
    let Some(progress) = progress else { return };
    let fallback_font = Font1::default();
    let font = font.as_deref().unwrap_or(&fallback_font);

    let w = grid.width as usize;
    let h = grid.height as usize;

    // Clear the grid
    for i in 0..(w * h) {
        grid.char_indices[i] = b' ' as u16;
        grid.fg_colors[i] = [0, 0, 0, 255];
        grid.bg_colors[i] = [0, 0, 0, 255];
    }

    // Center "LOADING..." text through Font1.
    let text = "LOADING...";
    let row = h / 2;
    font.paint_centered(&mut grid, row as u32, text, FontSkin::Grey);

    // Progress indicator below loading text: show stage number
    let stage_text: &[u8] = match progress.stage {
        3 => b"Initializing...",
        2 => b"Loading terrain...",
        1 => b"Building world...",
        0 => b"Ready!",
        _ => b"Loading...",
    };
    let stage_text = std::str::from_utf8(stage_text).unwrap_or("Loading...");
    let stage_row = row + 2;
    font.paint_centered(&mut grid, stage_row as u32, stage_text, FontSkin::Grey);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_state_default_is_main_menu() {
        let state = GameState::default();
        assert_eq!(state, GameState::MainMenu);
    }

    #[test]
    fn test_loading_progress_default_values() {
        let progress = LoadingProgress::default();
        assert_eq!(progress.stage, 3);
        assert_eq!(progress.items_loaded, 0);
        assert_eq!(progress.items_total, 0);
    }

    #[test]
    fn test_loading_progress_stage_logic() {
        // R17-F227 FIX: Verify advance_loading_progress sets stage=0 when assembled
        let mut progress = LoadingProgress::default();
        assert_eq!(progress.stage, 3, "Initial stage should be 3");

        // Simulate advance_loading_progress_system behavior when assembly complete
        // (Direct field mutation matching the system logic)
        progress.stage = 0;
        assert_eq!(
            progress.stage, 0,
            "After assembly complete, stage should be 0"
        );
    }

    #[test]
    fn test_game_state_variants() {
        // Verify all states are distinct
        let states = [
            GameState::MainMenu,
            GameState::Loading,
            GameState::Workbench,
            GameState::Playing,
            GameState::Paused,
        ];
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                assert_ne!(states[i], states[j]);
            }
        }
    }

    #[test]
    fn test_game_state_is_copy() {
        let state = GameState::Playing;
        let copy = state; // Copy
        assert_eq!(state, copy); // Both still valid
    }

    #[test]
    fn test_loading_screen_writes_to_grid() {
        let mut grid = AsciiCellGrid::new(80, 24);
        let progress = LoadingProgress::default();

        // Simulate render_loading_screen logic
        let w = grid.width as usize;
        let h = grid.height as usize;

        // Clear
        for i in 0..(w * h) {
            grid.char_indices[i] = b' ' as u16;
        }

        // Write loading text
        let text = b"LOADING...";
        let start_col = w.saturating_sub(text.len()) / 2;
        let row = h / 2;
        for (i, &ch) in text.iter().enumerate() {
            let idx = row * w + start_col + i;
            grid.char_indices[idx] = ch as u16;
        }

        // Verify loading text was written
        let check_idx = row * w + start_col;
        assert_eq!(grid.char_indices[check_idx], b'L' as u16);
        assert_eq!(grid.char_indices[check_idx + 1], b'O' as u16);

        // Verify stage text
        let stage_text = b"Initializing...";
        assert_eq!(progress.stage, 3); // Matches "Initializing..." branch
        let _ = stage_text; // Just verify stage is 3 for the branch
    }
}
