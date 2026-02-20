---
phase: 03-gpu-output
plan: 01
subsystem: gpu-output
tags: [wgsl, shader, gpu, font-atlas, cp437, bevy-render, bytemuck]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: AsciiCellGrid resource, AsciiOutputPlugin, RenderConfig
provides:
  - WGSL fragment shader with 4-texture binding layout for ASCII compositing
  - CP437 font atlas PNG asset (10x16 per glyph, 16x16 grid)
  - AsciiUniforms GPU struct (16-byte aligned, matches WGSL layout)
  - AsciiRenderConfig resource with font atlas handle
  - ExtractedAsciiGrid type and extract_grid_data conversion function
  - Checkerboard test pattern system populating AsciiCellGrid every frame
affects: [03-02-PLAN, 03-03-PLAN, 05-04-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns: [Mage Core 4-texture approach, GPU uniform 16-byte alignment, graceful AssetServer fallback]

key-files:
  created:
    - engine-port/assets/fonts/cp437_10x16.png
    - engine-port/src/output/shader.wgsl
    - engine-port/src/output/gpu_types.rs
    - engine-port/src/output/test_pattern.rs
  modified:
    - engine-port/src/output/mod.rs
    - engine-port/tests/resource_flow.rs

key-decisions:
  - "Use Mage Core font1.png (10x16 per glyph) directly as CP437 atlas"
  - "Graceful AssetServer fallback via get_resource instead of resource (supports MinimalPlugins in tests)"
  - "Shader imports Bevy fullscreen vertex output instead of custom vertex stage"

patterns-established:
  - "GPU uniform structs use #[repr(C)] + bytemuck::Pod with explicit padding for 16-byte alignment"
  - "extract_grid_data converts u16 char indices to Rgba8 bytes (R=index, G=0, B=0, A=255)"
  - "Plugin build uses get_resource for optional dependencies to support both full and minimal plugin sets"

requirements-completed: [GPU-02, GPU-03]

# Metrics
duration: 5min
completed: 2026-02-20
---

# Phase 3 Plan 01: GPU Output Building Blocks Summary

**WGSL shader with Mage Core 4-texture compositing, CP437 font atlas, GPU type definitions, and checkerboard test pattern system**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-20T18:33:42Z
- **Completed:** 2026-02-20T18:38:53Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- WGSL fragment shader adapted from Mage Core with 4-texture binding layout (fore, back, text, font) and pixel-center-corrected coordinate math
- CP437 font atlas (160x256 PNG, 10x16 per glyph, 16x16 grid) available as Bevy asset
- GPU type definitions: AsciiUniforms (16-byte aligned matching shader), AsciiRenderConfig (font handle + dimensions), ExtractedAsciiGrid (byte-ready for GPU upload)
- Checkerboard test pattern system providing synthetic visual data for Phase 3 render pipeline validation

## Task Commits

Each task was committed atomically:

1. **Task 1: Font atlas asset + WGSL shader + GPU types** - `6c9fc43` (feat)
2. **Task 2: Test pattern system + module wiring** - `3bbefb0` (feat)

## Files Created/Modified
- `engine-port/assets/fonts/cp437_10x16.png` - CP437 font atlas (Mage Core font1.png, 16x16 glyph grid, 10x16 pixels per glyph)
- `engine-port/src/output/shader.wgsl` - WGSL fragment shader with 4-texture bindings, Bevy fullscreen vertex import, integer coordinate math
- `engine-port/src/output/gpu_types.rs` - AsciiUniforms, AsciiRenderConfig, ExtractedAsciiGrid, extract_grid_data function + 5 unit tests
- `engine-port/src/output/test_pattern.rs` - fill_test_pattern pure function + test_pattern_system Bevy wrapper + 4 unit tests
- `engine-port/src/output/mod.rs` - Added gpu_types/test_pattern modules, font atlas loading, AsciiRenderConfig insertion, test pattern system registration
- `engine-port/tests/resource_flow.rs` - Fixed separate_gpu_arrays_verified test to snapshot values before mutation check

## Decisions Made
- Used Mage Core font1.png (10x16 per glyph) directly as the CP437 atlas rather than creating an 8x8 font. The research suggested 8x8 for optimal grid density, but the plan explicitly specified 10x16 and the Mage Core reference uses it. Grid dimensions will be computed from window size and font dimensions in Plan 02/03.
- Used `get_resource::<AssetServer>()` instead of `resource::<AssetServer>()` to avoid panicking when the plugin is used with MinimalPlugins in integration tests (returns default handle when unavailable).
- Shader imports `bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput` for the vertex stage, matching the plan's specification of fragment-only shader.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Wired gpu_types module in Task 1 (plan scheduled it for Task 2)**
- **Found during:** Task 1 verification
- **Issue:** `cargo test --lib output::gpu_types` found 0 tests because gpu_types.rs was not declared in mod.rs yet (Task 2 action)
- **Fix:** Added `pub mod gpu_types;` to output/mod.rs during Task 1 to unblock test verification
- **Files modified:** engine-port/src/output/mod.rs
- **Verification:** All 5 gpu_types tests discovered and pass
- **Committed in:** 6c9fc43 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed AssetServer panic with MinimalPlugins**
- **Found during:** Task 2 (full test suite run)
- **Issue:** `AsciiOutputPlugin::build` called `app.world().resource::<AssetServer>()` which panics when MinimalPlugins is used (no AssetPlugin). 4 integration tests in resource_flow.rs failed.
- **Fix:** Changed to `app.world().get_resource::<AssetServer>()` with fallback to default handle
- **Files modified:** engine-port/src/output/mod.rs
- **Verification:** All 4 resource_flow tests pass
- **Committed in:** 3bbefb0 (Task 2 commit)

**3. [Rule 1 - Bug] Fixed resource_flow test assertion after test pattern introduction**
- **Found during:** Task 2 (full test suite run)
- **Issue:** `separate_gpu_arrays_verified` test expected default black `[0,0,0,255]` colors after app.update(), but test_pattern_system now fills with orange/green. Assertion failed.
- **Fix:** Changed test to snapshot fg/bg values before mutating char_indices, then assert unchanged (tests array independence, not specific values)
- **Files modified:** engine-port/tests/resource_flow.rs
- **Verification:** Test passes, still validates array independence correctly
- **Committed in:** 3bbefb0 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking)
**Impact on plan:** All auto-fixes necessary for correctness and test passing. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All building blocks for Plan 02 (Bevy ViewNode render pipeline) are in place
- Shader file ready for pipeline to load
- Font atlas handle stored in AsciiRenderConfig for render world extraction
- ExtractedAsciiGrid type ready for Extract/Prepare system implementation
- Test pattern system provides synthetic visual data for render pipeline validation

---
*Phase: 03-gpu-output*
*Completed: 2026-02-20*
