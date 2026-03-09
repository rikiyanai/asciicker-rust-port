//! Game state machine: Loading/Playing/Paused FSM using Bevy States.
//!
//! Maps to the C++ mainmenu.cpp loading FSM (3 stages: init, terrain_patches,
//! world_rebuild, done). Bevy's `States` derive provides OnEnter/OnExit
//! schedules and `in_state()` run conditions.

use bevy::prelude::*;

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::assembly::AssemblyState;
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
    /// Active gameplay: all systems running.
    Playing,
    /// Gameplay paused: physics/character frozen, render still active.
    Paused,
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

/// Transition to Playing when loading is complete (stage == 0).
pub fn check_loading_complete(
    progress: Option<Res<LoadingProgress>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Some(progress) = progress
        && progress.stage == 0
    {
        next_state.set(GameState::Playing);
        info!("Loading complete (stage=0): transitioning to Playing");
    }
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
) {
    let Some(mut grid) = grid else { return };
    let Some(progress) = progress else { return };

    let w = grid.width as usize;
    let h = grid.height as usize;

    // Clear the grid
    for i in 0..(w * h) {
        grid.char_indices[i] = b' ' as u16;
        grid.fg_colors[i] = [0, 0, 0, 255];
        grid.bg_colors[i] = [0, 0, 0, 255];
    }

    // Center "LOADING..." text
    let text = b"LOADING...";
    let start_col = w.saturating_sub(text.len()) / 2;
    let row = h / 2;
    let fg = [255, 255, 255, 255]; // white
    for (i, &ch) in text.iter().enumerate() {
        let idx = row * w + start_col + i;
        if idx < w * h {
            grid.char_indices[idx] = ch as u16;
            grid.fg_colors[idx] = fg;
        }
    }

    // Progress indicator below loading text: show stage number
    let stage_text: &[u8] = match progress.stage {
        3 => b"Initializing...",
        2 => b"Loading terrain...",
        1 => b"Building world...",
        0 => b"Ready!",
        _ => b"Loading...",
    };
    let stage_start = w.saturating_sub(stage_text.len()) / 2;
    let stage_row = row + 2;
    let stage_fg = [180, 180, 180, 255]; // light gray
    for (i, &ch) in stage_text.iter().enumerate() {
        let idx = stage_row * w + stage_start + i;
        if idx < w * h {
            grid.char_indices[idx] = ch as u16;
            grid.fg_colors[idx] = stage_fg;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
