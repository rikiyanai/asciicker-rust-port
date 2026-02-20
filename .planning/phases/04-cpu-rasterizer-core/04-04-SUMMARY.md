---
phase: 04-cpu-rasterizer-core
plan: 04
subsystem: render
tags: [resolve, downsample, ansi-cell, pipeline-stage, auto-mat, material, elevation, grid-overlay]

# Dependency graph
requires:
  - phase: 04-cpu-rasterizer-core
    plan: 01
    provides: Sample struct, SampleBuffer, AnsiCell, RGB555 quantize, spare_bits constants
  - phase: 04-cpu-rasterizer-core
    plan: 02
    provides: MatCell, Material, auto_mat LUT, test_materials()
  - phase: 04-cpu-rasterizer-core
    plan: 03
    provides: RasterShader trait, bresenham(), rasterize()
provides:
  - resolve() function producing AnsiCell grid from SampleBuffer with material and mesh paths
  - PipelineStage enum defining 6-stage Clear/Terrain/World/Shadow/Reflection/Resolve ordering
  - Elevation detection via row-above height comparison (0-3 range)
  - Grid/wireframe glyph overlay system
  - Reflection darkening path (diffuse divisor 400 vs 255)
  - Integration test proving full rasterize -> resolve -> AnsiCell data path
affects: [05-pipeline-integration]

# Tech tracking
tech-stack:
  added: []
  patterns: [MaterialResolveCtx struct to group resolve parameters, 2x2 block sampling with border skip, dominant-sample selection for mixed blocks]

key-files:
  created:
    - engine-port/src/render/resolve.rs
  modified:
    - engine-port/src/render/mod.rs

key-decisions:
  - "Used MaterialResolveCtx struct to avoid clippy too-many-arguments on resolve_material (8 params -> 3)"
  - "Mesh flag in combined spare OR determines mesh vs material path (matches C++ behavior)"
  - "Elevation thresholds: 0.5/2.0/5.0 height-difference breakpoints for 0-3 elevation mapping"
  - "Wireframe overlay uses '/' glyph; grid overlay uses '+'/'-'/'|' based on cell position parity"

patterns-established:
  - "Context struct pattern: group related parameters into a struct to stay under clippy argument limits"
  - "2x2 block resolve: skip +2 border, read 4 samples, average/OR, pick dominant, resolve via path"
  - "Integration test pattern: rasterize geometry -> resolve -> verify AnsiCell output end-to-end"

requirements-completed: [REND-04, REND-07, REND-10]

# Metrics
duration: 8min
completed: 2026-02-20
---

# Phase 4 Plan 4: RESOLVE Stage and Pipeline Skeleton Summary

**RESOLVE stage downsampling 2x2 sample blocks to AnsiCell grid with material/mesh dual paths, elevation detection, grid overlays, and 6-stage PipelineStage enum**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-20T18:52:06Z
- **Completed:** 2026-02-20T19:00:06Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Full resolve() function converting 2x-supersampled SampleBuffer into AnsiCell output grid with correct material path (shade[elevation][diffuse] -> MatCell -> rgb2pal) and mesh path (RGB555 diffuse scaling -> auto_mat_lookup -> dither glyph)
- PipelineStage enum with all 6 stages in correct order, stub render_pipeline system that compiles (not scheduled until Phase 5)
- Integration test proving end-to-end data path: rasterize triangle with FlatMeshShader + grid line via bresenham -> resolve -> verify correct AnsiCell output
- Performance benchmark (ignored, release-mode) asserting < 16ms for clear+resolve at 240x135
- 11 new tests (8 resolve unit + 3 pipeline/integration) pass alongside 111 pre-existing tests (122 total)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement RESOLVE stage** - `8d9e8a1` (feat)
2. **Task 2: Pipeline stage enum, integration test, and performance benchmark** - `1885ccf` (feat)

## Files Created/Modified
- `engine-port/src/render/resolve.rs` - resolve() function with material/mesh dual paths, elevation detection, grid/wireframe overlays, 8 unit tests
- `engine-port/src/render/mod.rs` - PipelineStage enum, stub render_pipeline system, resolve module registration, integration test, perf benchmark

## Decisions Made
- Used `MaterialResolveCtx` struct to group 6 related parameters into a single context object, staying under clippy's 7-argument limit while keeping the hot-path resolve function clean
- Mesh flag dominance in mixed 2x2 blocks: OR of all 4 spare bytes, then MESH_FLAG check, matches C++ behavior where mesh samples override terrain in mixed cells
- Elevation thresholds at 0.5 / 2.0 / 5.0 height difference breakpoints mapping to elevation 0-3 -- these are approximate and will be tuned when real terrain data flows through in Phase 5
- Grid overlay uses positional parity (cx%2, cy%2) to select '+'/'-'/'|' characters -- simplified for Phase 4, Phase 5 will use actual grid line direction information

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused import rgb5_to_rgb8**
- **Found during:** Task 1
- **Issue:** Import of `rgb5_to_rgb8` was unused; resolve_mesh uses `rgb555_to_rgb888` (which calls rgb5_to_rgb8 internally)
- **Fix:** Removed the unused import
- **Files modified:** engine-port/src/render/resolve.rs
- **Verification:** `cargo clippy -- -D warnings` passes clean
- **Committed in:** 8d9e8a1 (Task 1 commit)

**2. [Rule 1 - Bug] Refactored resolve_material to avoid too-many-arguments clippy error**
- **Found during:** Task 1
- **Issue:** `resolve_material` had 8 parameters, exceeding clippy's 7-argument limit
- **Fix:** Created `MaterialResolveCtx` struct to group samples/dw/sx/sy/avg_height/materials into a single parameter
- **Files modified:** engine-port/src/render/resolve.rs
- **Verification:** `cargo clippy -- -D warnings` passes clean
- **Committed in:** 8d9e8a1 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Minor import cleanup and parameter grouping to satisfy clippy. No scope creep.

## Issues Encountered
- Pre-existing test failures in `resource_flow.rs` (4 Bevy integration tests with `MessageReader` not initialized) -- these are NOT caused by this plan's changes. They are pre-existing Bevy system-level test issues from uncommitted output module changes.
- Release build for performance benchmark takes very long (10+ minutes) due to full Bevy dependency tree compilation in optimized mode. The benchmark test is correctly marked `#[ignore]` for manual execution.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The full Phase 4 CPU Rasterizer Core is now complete: Sample/SampleBuffer, AnsiCell, RGB quantization, Material/auto_mat, Bresenham/triangle rasterizer, and RESOLVE stage
- Phase 5 (Pipeline Integration) can wire terrain/world/shadow/reflection into the pipeline stages 2-5
- The PipelineStage enum and stub render_pipeline system provide the skeleton for Phase 5 to fill in
- No blockers for subsequent phases

---
*Phase: 04-cpu-rasterizer-core*
*Completed: 2026-02-20*
