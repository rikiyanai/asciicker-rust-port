---
phase: 06-physics-and-character
plan: 02
subsystem: character
tags: [state-machine, equipment, input, animation, sprite-query, bevy-ecs, physics-io]

# Dependency graph
requires:
  - phase: 05-pipeline-integration
    provides: "GameCamera, SpriteRenderEntry/SpriteQueue, RuntimeTerrain, project_world_to_screen"
  - phase: 06-physics-and-character
    provides: "PhysicsIO resource, PhysicsPlugin, PhysicsState, collision/forces"
provides:
  - "CharacterPlugin with PreUpdate input and PostUpdate state/animation/sprite systems"
  - "ActionState enum (None, Attack, Block, Fall, Stand, Dead) with transition guards"
  - "SpriteReq component for 5D equipment lookup (kind, armor, helmet, shield, weapon + clr)"
  - "AnimationState frame counter with per-action timing constants"
  - "accumulate_player_input: keyboard -> PhysicsIO with camera-relative rotation"
  - "query_character_sprites: character entities -> SpriteRenderEntry in SpriteQueue"
  - "clear_sprite_queue_system in PreUpdate (CharacterPlugin owns clearing)"
  - "camera_input_system gated to spectator mode (no characters)"
  - "SystemSets: RenderSet::Pipeline, CharacterSet::{PreUpdateInput, SpritePush, PhysicsSync}"
  - "spawn_character(): single spawner function for all character entities"
  - "Character marker with Required Components"
affects: [06-03-benchmark, 07-game-systems]

# Tech tracking
tech-stack:
  added: []
  patterns: [required-components, system-sets-ordering, spectator-mode-gating, single-spawner-pattern]

key-files:
  created:
    - engine-port/src/character/state_machine.rs
    - engine-port/src/character/equipment.rs
    - engine-port/src/character/animation.rs
    - engine-port/src/character/input.rs
    - engine-port/src/character/sprite_query.rs
    - engine-port/src/system_sets.rs
    - engine-port/tests/ecs_character_integration.rs
  modified:
    - engine-port/src/character/mod.rs
    - engine-port/src/render/camera.rs
    - engine-port/src/render/mod.rs
    - engine-port/src/lib.rs

key-decisions:
  - "Block state is movement-locked and mutually exclusive with Attack (equipment guard in input.rs, not state machine)"
  - "AnimationState uses Model B (frame counter) for deterministic tests -- no Instant::now()"
  - "camera_input_system gated via custom has_characters() run condition (not any_with_component)"
  - "SpriteReq includes clr:u8 field (default 0) for Phase 7 multiplayer forward-compatibility"
  - "spawn_character() is the ONLY function that creates character entities (single source of truth)"
  - "Dead state is currently permanent (TODO for respawn flow documented in state_machine.rs)"
  - "SC-9 (sprite rendering) is PARTIAL: query_character_sprites works but blit_sprite is placeholder 'S'"

patterns-established:
  - "Required Components: Character marker auto-inserts ActionState, SpriteReq, AnimationState, Transform"
  - "SystemSets: CharacterSet/RenderSet for cross-plugin ordering"
  - "Spectator gating: camera_input_system disabled when Character entities exist"
  - "Single spawner: all character creation via spawn_character() function"
  - "Input reset pattern: forces zeroed at start of accumulate_player_input each frame"

requirements-completed: [CHAR-01, CHAR-02, CHAR-03, CHAR-04]

# Metrics
duration: 13min
completed: 2026-02-24
---

# Phase 6 Plan 02: Character State Machine, Equipment, Input, Animation Summary

**Character state machine with 6 action states (including Block), 5D equipment lookup, camera-relative WASD/Q/E input via PhysicsIO, frame-counter animation, and sprite queue bridge to render pipeline**

## Performance

- **Duration:** 13 min
- **Started:** 2026-02-24T15:32:30Z
- **Completed:** 2026-02-24T15:45:11Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Complete character state machine with 6 states (None, Attack, Block, Fall, Stand, Dead) and transition guards including Block/Attack mutual exclusion
- 5D equipment lookup (SpriteReq) with collision dimensions per mount, equipment change guards during Attack/Block, and clr field for Phase 7 multiplayer
- Camera-relative WASD input system writing to PhysicsIO with Q/E torque, space jump, shift half-speed, F block (shield-gated)
- Phase 5 camera_input_system gated to spectator mode -- WASD/Q/E now routed through character input path exclusively
- query_character_sprites bridges character entities to SpriteQueue for deferred sprite blit
- 49 new unit tests + 3 ECS integration tests, 324 total lib tests (up from 275)

## Task Commits

Each task was committed atomically:

1. **Task 1: State Machine, Equipment, Animation** - `455f119` (feat)
2. **Task 2: Input, Camera Transfer, Sprite Query, Plugin Wiring** - `d01981e` (feat)

## Files Created/Modified
- `engine-port/src/character/state_machine.rs` - ActionState enum, Character marker with Required Components, transition guards
- `engine-port/src/character/equipment.rs` - 5D equipment enums, SpriteReq component, collision_dimensions()
- `engine-port/src/character/animation.rs` - AnimationState frame counter, advance() with per-action timing
- `engine-port/src/character/input.rs` - accumulate_player_input system, WASD/Q/E/space/shift/F input
- `engine-port/src/character/sprite_query.rs` - query_character_sprites system, SpriteRenderEntry creation
- `engine-port/src/character/mod.rs` - CharacterPlugin, system registration, spawn_character(), spawn_player()
- `engine-port/src/system_sets.rs` - RenderSet and CharacterSet system set enums
- `engine-port/src/lib.rs` - Added system_sets module declaration
- `engine-port/src/render/camera.rs` - Gated camera_input_system, added has_characters() run condition
- `engine-port/src/render/mod.rs` - Imported has_characters, applied run_if gating
- `engine-port/tests/ecs_character_integration.rs` - 3 ECS integration tests

## Decisions Made
- Block state uses equipment guard in input.rs (caller responsibility), NOT inside can_transition_to() -- matches plan and C++ pattern where blocking depends on shield state.
- AnimationState Model B (frame counter with elapsed_frames) chosen for deterministic testing and Bevy FixedUpdate alignment.
- Custom `has_characters()` run condition instead of `any_with_component::<Character>` for Bevy 0.18 compatibility safety.
- SpriteReq includes `clr: u8` (default 0) for forward-compatibility with Phase 7 multiplayer team colors.
- Dead state has NO respawn flow (documented as TODO). Respawn/menu-return deferred to Phase 7.
- SC-9 marked PARTIAL: query_character_sprites correctly pushes SpriteRenderEntry items, but downstream blit_sprite() is still Phase 5 placeholder ('S' glyph). Full XP sprite rendering is deferred.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Bevy 0.18 query.single() API change**
- **Found during:** Task 2 (input.rs test compilation)
- **Issue:** Bevy 0.18 changed `query.single()` to return `Result<&T, QuerySingleError>` instead of `&T` directly. Tests called `.clone()` on the Result.
- **Fix:** Changed to `query.single(world).expect("player entity")` with dereference.
- **Files modified:** engine-port/src/character/input.rs
- **Verification:** All input tests pass.
- **Committed in:** d01981e (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Bevy API adaptation, no scope change.

## Deferred Items

1. **Dead state respawn flow** -- Dead is permanent; needs respawn-to-spawn or menu-return. Deferred to Phase 7.
2. **Full XP sprite rendering** -- blit_sprite() is placeholder 'S'. Character sprites will render as yellow 'S' glyph until real XP frame blit is implemented. Deferred to future plan.
3. **8-direction sprite index** -- Camera-relative facing direction for sprite sheet column lookup not computed. Placeholder blit doesn't need it. Deferred to sprite rendering plan.
4. **Character sprites invisible until 06-03** -- R19-M03: PostUpdate sprite push + Update pipeline clear = sprites pushed after pipeline already rendered. Plan 06-03 fixes scheduling.

## Issues Encountered
None - plan executed smoothly.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CharacterPlugin ready for Plan 06-03 (physics benchmark, apply_torque_to_camera, pipeline scheduling fix)
- PhysicsIO.torque written by accumulate_player_input, ready for 06-03 apply_torque_to_camera
- SpriteQueue bridge ready -- 06-03 needs to fix scheduling so sprites become visible
- All character systems registered in correct schedules (PreUpdate input, PostUpdate state/animation/sprite)

---
*Phase: 06-physics-and-character*
*Completed: 2026-02-24*
