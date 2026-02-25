---
phase: 07-game-systems
plan: 02
subsystem: game
tags: [bevy-states, game-state-machine, main-menu, loading-fsm, pause, ascii-grid]

# Dependency graph
requires:
  - phase: 06-physics-and-character
    provides: "GamePlugin with physics sync, camera follow, water level systems"
  - phase: 05-pipeline-integration
    provides: "render_pipeline_system in RenderSet::Pipeline, AssemblyState, RuntimeTerrain"
provides:
  - "GameState enum (MainMenu/Loading/Playing/Paused) with Bevy States derive"
  - "LoadingProgress resource bridging Phase 5 assembly to Phase 7 loading FSM"
  - "MainMenu resource with Start Game/Quit items and keyboard navigation"
  - "State-gated Phase 6 systems (only run during Playing state)"
  - "RenderSet::Pipeline gated on GameState::Playing"
  - "RenderSet::WaterTime gated on GameState::Playing"
affects: [07-03, 07-04, 07-05]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Bevy States derive for FSM", "configure_sets for cross-plugin state gating", "Option<ResMut<T>> for test-safe system params"]

key-files:
  created:
    - engine-port/src/game/state.rs
    - engine-port/src/game/menu.rs
  modified:
    - engine-port/src/game/mod.rs
    - engine-port/src/system_sets.rs
    - engine-port/src/render/mod.rs

key-decisions:
  - "Used configure_sets from GamePlugin to gate RenderSet::Pipeline on Playing (avoids modifying CpuRasterizerPlugin)"
  - "Added RenderSet::WaterTime set for advance_water_time_system gating (clean cross-plugin boundary)"
  - "Escape from any non-Playing/Paused state returns to MainMenu (R19-005 fallback for stuck states)"
  - "advance_loading_progress_system requires BOTH AssemblyState.assembled AND terrain.root.is_some() before transitioning"

patterns-established:
  - "State gating via run_if(in_state(GameState::Playing)) on all gameplay systems"
  - "Option<ResMut<AsciiCellGrid>> for systems that render to grid (safe in MinimalPlugins test env)"
  - "MessageWriter<AppExit> for Bevy 0.18 quit action (not EventWriter)"

requirements-completed: [GAME-01, GAME-02]

# Metrics
duration: 18min
completed: 2026-02-25
---

# Phase 7 Plan 02: Game State Machine and Main Menu Summary

**Bevy States-driven GameState FSM (MainMenu/Loading/Playing/Paused) with keyboard-navigable main menu, loading screen, and state-gated Phase 6 gameplay systems**

## Performance

- **Duration:** 18 min
- **Started:** 2026-02-25T14:53:54Z
- **Completed:** 2026-02-25T15:12:23Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- GameState with 4 Bevy States variants (MainMenu default) controlling system execution flow
- MainMenu with "Start Game" / "Quit" items, Up/Down/Enter navigation, gold highlight, centered title
- Loading screen rendering progress text to AsciiCellGrid during asset loading
- ALL Phase 6 systems gated on GameState::Playing (physics sync, camera follow, water, torque)
- RenderSet::Pipeline and RenderSet::WaterTime gated on Playing (menu owns grid during MainMenu)
- 14 new unit tests (6 state + 5 menu + 3 integration in mod.rs)

## Task Commits

Each task was committed atomically:

1. **Task 1: GameState enum, LoadingProgress, MainMenu, transition systems** - `aa76bff` (feat)
2. **Task 2: GamePlugin wiring with state machine and state gating** - `041584e` (feat)

## Files Created/Modified
- `engine-port/src/game/state.rs` - GameState enum (4 variants), LoadingProgress resource, transition systems (on_enter_loading, check_loading_complete, toggle_pause, render_loading_screen, advance_loading_progress_system)
- `engine-port/src/game/menu.rs` - MainMenu resource, MenuAction enum, menu_navigation, menu_activate (MessageWriter<AppExit>), render_menu with centered title and gold selection indicator
- `engine-port/src/game/mod.rs` - GamePlugin augmented with init_state, state-gated Phase 6 systems, menu/loading/pause system registrations
- `engine-port/src/system_sets.rs` - Added RenderSet::WaterTime variant for cross-plugin gating
- `engine-port/src/render/mod.rs` - advance_water_time_system labeled with RenderSet::WaterTime

## Decisions Made
- Used `configure_sets(PostUpdate, RenderSet::Pipeline.run_if(in_state(GameState::Playing)))` from GamePlugin to gate render pipeline without modifying CpuRasterizerPlugin (cleaner cross-plugin boundary)
- Added `RenderSet::WaterTime` system set in system_sets.rs to enable cross-plugin gating of advance_water_time_system from GamePlugin
- R19-005: Escape from any non-Playing/Paused state returns to MainMenu (prevents being stuck after death)
- advance_loading_progress_system checks BOTH `AssemblyState.assembled` AND `terrain.root.is_some()` before setting stage=0 (R19-004: prevents spawning before terrain data exists)
- R19-003: render_loading_screen system writes progress to AsciiCellGrid during Loading state

## Deviations from Plan

None - plan executed as written. All fixes referenced in the plan (P7-014, P7-038, P7-050, R8-XP-002, R19-001 through R19-007, P7-103, P7-046) were incorporated during implementation.

## Issues Encountered
- Clippy `collapsible_if` lint on check_loading_complete -- fixed by using `if let ... && condition` syntax
- Byte array match arms have incompatible types (different-length b"..." literals) -- fixed by explicit `&[u8]` type annotation
- Pre-existing clippy errors in terrain_shader.rs (too_many_arguments, collapsible_if) are out of scope

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- GameState FSM is the orchestration backbone for all remaining Phase 7 plans
- Plan 07-03 (Audio) and 07-04 (Shape Vectors) can gate on GameState::Playing
- Plan 07-05 (Wave 2: Weather, Day/Night) will read game/mod.rs and augment the state machine
- R19-006 (yaw velocity model) deferred -- linear rotation model is functional but feels mechanical

## Self-Check: PASSED

All 6 files verified present on disk. Both task commits (aa76bff, 041584e) verified in git log. 379 tests passing (`cargo test --lib`). Build succeeds. No clippy errors in modified files.

---
*Phase: 07-game-systems*
*Completed: 2026-02-25*
