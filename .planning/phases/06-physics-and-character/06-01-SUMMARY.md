---
phase: 06-physics-and-character
plan: 01
subsystem: physics
tags: [collision, forces, gravity, buoyancy, grounded, bevy-fixedupdate, sphere-sweep]

# Dependency graph
requires:
  - phase: 05-pipeline-integration
    provides: "RuntimeTerrain (quadtree, get_patch_at), RuntimeWorld (BSP, query_sphere), asset_loader constants"
provides:
  - "PhysicsPlugin with FixedUpdate at 66Hz"
  - "PhysicsIO resource (game<->physics bridge)"
  - "PhysicsState resource (velocity, grounded accumulator)"
  - "Sphere-triangle collision (face/vertex/edge 3-test cascade)"
  - "Force accumulation (unified gravity/buoyancy, impulse, damping)"
  - "Grounded detection via contact normal accumulation"
  - "collect_terrain_triangles (32 tris/patch, sphere-space transform)"
  - "collect_world_triangles (bbox proxy, 12 tris/mesh)"
  - "Collision sweep with MAX_SUBSTEPS=10, slide response, velocity recompute"
affects: [06-02-input-character, 06-03-benchmark, 07-game-systems]

# Tech tracking
tech-stack:
  added: []
  patterns: [bevy-fixedupdate-systems, sphere-space-collision, free-function-geometry-collection]

key-files:
  created:
    - engine-port/src/physics/constants.rs
    - engine-port/src/physics/soup.rs
    - engine-port/src/physics/collision.rs
    - engine-port/src/physics/forces.rs
    - engine-port/src/physics/geometry.rs
  modified:
    - engine-port/src/physics/mod.rs
    - engine-port/src/main.rs
    - engine-port/src/render/pipeline.rs

key-decisions:
  - "Used existing RuntimeWorld.query_sphere for BSP-accelerated mesh lookup (plan said option b, but query_sphere already implemented)"
  - "Unified gravity/buoyancy formula with static cnt=0.78 (wave modulation deferred to pre-Phase 7)"
  - "Collision search radius uses world_radius (entity) not world_height (R19-PERF mitigation)"
  - "PhysicsIO::default has safe non-zero world_radius/world_height to prevent div-by-zero"
  - "update_output_system reads PhysicsState via Res (immutable) not ResMut"

patterns-established:
  - "Physics free functions: geometry collection uses free functions (not trait) for terrain/world"
  - "Sphere-space transform: all collision geometry transformed to sphere space where radius=1"
  - "FixedUpdate chain: accumulate_forces -> collision_sweep -> update_output"
  - "Contact normal MAX within substeps, accumulate after loop (R19-M02 pattern)"

requirements-completed: [PHYS-01, PHYS-02, PHYS-03, PHYS-04]

# Metrics
duration: 12min
completed: 2026-02-24
---

# Phase 6 Plan 01: Physics Core Summary

**Sphere collision with face/vertex/edge cascade, unified gravity/buoyancy forces, 66Hz FixedUpdate with geometry collection from RuntimeTerrain and RuntimeWorld**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-24T15:13:40Z
- **Completed:** 2026-02-24T15:26:24Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Complete sphere-triangle collision detection (face/vertex/edge 3-test cascade) ported from C++ CheckCollision
- Force accumulation with unified gravity/buoyancy formula, impulse, damping, velocity clamping
- Grounded detection via contact normal accumulation with decay pattern
- Terrain geometry collection: 32 triangles per patch from 5x5 height grid
- World geometry collection: 12 bbox proxy triangles per mesh instance via BSP query_sphere
- Collision sweep with up to 10 substeps, slide response, and velocity recompute from position delta
- PhysicsPlugin with FixedUpdate at 66Hz, formula-derived world_radius/world_height defaults
- 35 new tests all passing, 275 total lib tests (up from 240)

## Task Commits

Each task was committed atomically:

1. **Task 1: Physics constants, SoupItem, collision algorithm** - `2a7bb6d` (feat)
2. **Task 2: Forces, geometry collection, PhysicsIO/Plugin** - `1086436` (feat)

## Files Created/Modified
- `engine-port/src/physics/constants.rs` - All physics constants matching C++ (PHYSICS_HZ, MAX_SUBSTEPS, etc.)
- `engine-port/src/physics/soup.rs` - SoupItem struct and to_sphere_space transform
- `engine-port/src/physics/collision.rs` - check_collision with face/vertex/edge cascade, CollisionResult enum
- `engine-port/src/physics/forces.rs` - accumulate_forces, apply_jump, update_grounded
- `engine-port/src/physics/geometry.rs` - collect_terrain_triangles, collect_world_triangles
- `engine-port/src/physics/mod.rs` - PhysicsPlugin, PhysicsIO, PhysicsState, FixedUpdate systems
- `engine-port/src/main.rs` - Fixed pre-existing clippy collapsible_if warning
- `engine-port/src/render/pipeline.rs` - Fixed pre-existing clippy collapsible_if warning

## Decisions Made
- Used existing RuntimeWorld.query_sphere for BSP-accelerated mesh lookup instead of linear iteration (plan option b). RuntimeWorld already had this method implemented in Phase 5, so using it is simpler and more efficient.
- Applied static cnt=0.78 for buoyancy formula (wave modulation documented as TODO for pre-Phase 7).
- Collision search radius uses world_radius * 2 (entity radius), not world_height, per R19-PERF to prevent scanning hundreds of patches.
- PhysicsIO Default trait gives safe non-zero world_radius=1.0, world_height=1.0; PhysicsPlugin build() overrides with formula values.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing clippy warnings in main.rs and pipeline.rs**
- **Found during:** Task 2 verification (cargo clippy -- -D warnings)
- **Issue:** Pre-existing collapsible_if warnings in main.rs (fps_title_system) and pipeline.rs (mesh rendering) blocked clean clippy
- **Fix:** Collapsed nested if-let chains using Rust let-chain syntax
- **Files modified:** engine-port/src/main.rs, engine-port/src/render/pipeline.rs
- **Verification:** cargo clippy -- -D warnings clean
- **Committed in:** 1086436 (Task 2 commit)

**2. [Rule 1 - Bug] Used query_sphere instead of linear iteration**
- **Found during:** Task 2 (collect_world_triangles implementation)
- **Issue:** Plan suggested option (b) linear iteration, but RuntimeWorld already has query_sphere with BSP acceleration
- **Fix:** Used existing query_sphere method, which is more correct and performant
- **Files modified:** engine-port/src/physics/geometry.rs
- **Verification:** test_collect_world_triangles_bbox_proxy passes, far mesh excluded test passes

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both changes improve correctness. No scope creep.

## Issues Encountered
None - plan executed smoothly.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- PhysicsIO resource ready for Plan 06-02 (input accumulation, character state machine)
- PhysicsState accessible for Plan 06-03 (physics benchmark)
- All TRAP warnings addressed (P01-P05)
- Geometry collection functions available for collision sweep

---
*Phase: 06-physics-and-character*
*Completed: 2026-02-24*
