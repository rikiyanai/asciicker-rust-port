---
phase: 03-gpu-output
plan: 02
subsystem: gpu-output
tags: [bevy-render, viewnode, render-pipeline, extract-prepare, gpu-textures, bind-groups, fullscreen-shader]

# Dependency graph
requires:
  - phase: 03-gpu-output
    plan: 01
    provides: WGSL shader, GPU types (AsciiUniforms, ExtractedAsciiGrid, AsciiRenderConfig), CP437 font atlas, test pattern system
provides:
  - AsciiGpuPlugin with full Bevy render pipeline (extract/prepare/render)
  - ViewNode drawing fullscreen triangle with ASCII compositing shader
  - Camera2d entity for Core2d render graph execution
  - Font atlas loaded with is_srgb=false for linear Rgba8Unorm format
affects: [03-03-PLAN, 05-04-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns: [ViewNode render pipeline, RenderStartup pipeline init, embedded_asset shader loading, RenderApp guard for MinimalPlugins]

key-files:
  created:
    - engine-port/src/output/gpu_plugin.rs
  modified:
    - engine-port/src/output/mod.rs

key-decisions:
  - "Guard AsciiGpuPlugin with RenderApp existence check before calling embedded_asset! (supports MinimalPlugins in tests)"
  - "Store ExtractedFontAtlasHandle as separate render-world resource to pass font atlas between extract and prepare systems"
  - "Use RenderStartup schedule for pipeline initialization instead of Plugin::finish (matches Bevy 0.18 BlitPipeline pattern)"

patterns-established:
  - "ViewNode with get_color_attachment for fullscreen rendering in Core2d graph"
  - "RenderApp guard: check get_sub_app(RenderApp).is_none() before embedded_asset! to support test environments"
  - "BindGroupLayoutDescriptor stored in pipeline resource for deferred bind group creation via pipeline_cache.get_bind_group_layout"

requirements-completed: [GPU-01, GPU-04]

# Metrics
duration: 10min
completed: 2026-02-20
---

# Phase 3 Plan 02: GPU Render Pipeline Summary

**Bevy ViewNode render pipeline connecting AsciiCellGrid to GPU via Extract/Prepare/Render with 4-texture bind groups and fullscreen shader**

## Performance

- **Duration:** 10 min
- **Started:** 2026-02-20T18:42:02Z
- **Completed:** 2026-02-20T18:52:58Z
- **Tasks:** 2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments
- AsciiGpuPlugin with ViewNode render pipeline: extract system copies grid unconditionally, prepare system uploads to 3 GPU textures via write_texture, render node draws fullscreen triangle
- Pipeline initialization in RenderStartup with cached pipeline, 4-texture bind group layout, and uniform bind group layout matching shader bindings
- Camera2d spawned at startup, font atlas loaded with is_srgb=false, test pattern system registered in Update schedule
- Graceful handling of missing font atlas (skip rendering) and MinimalPlugins test environment (skip GPU setup)

## Task Commits

Each task was committed atomically:

1. **Task 1: AsciiGpuPlugin with ViewNode render pipeline** - `8aa47dc` (feat)
2. **Task 2: Wire AsciiGpuPlugin into app + spawn Camera2d** - `a48480f` (feat)

## Files Created/Modified
- `engine-port/src/output/gpu_plugin.rs` - AsciiGpuPlugin with extract/prepare/render systems, AsciiPipeline resource, AsciiGpuTextures resource, AsciiNode ViewNode (407 lines)
- `engine-port/src/output/mod.rs` - Added gpu_plugin/test_pattern modules, AsciiGpuPlugin sub-plugin, Camera2d startup system, AsciiRenderConfig with font atlas, font atlas loaded with is_srgb=false

## Decisions Made
- Used `RenderStartup` schedule for pipeline initialization (matching Bevy 0.18 BlitPipeline pattern) instead of `Plugin::finish` which the plan suggested. The RenderStartup pattern is the canonical Bevy 0.18 approach and provides access to all required resources.
- Created `ExtractedFontAtlasHandle` as a separate render-world resource rather than storing it in AsciiPipeline. This keeps the font atlas handle extraction clean since AsciiPipeline is created once in RenderStartup and the handle needs to be updated from the main world each frame.
- Guarded AsciiGpuPlugin with `get_sub_app(RenderApp).is_none()` check before `embedded_asset!` to prevent panics in test environments using MinimalPlugins.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Wired gpu_plugin module in Task 1 (plan scheduled it for Task 2)**
- **Found during:** Task 1 verification
- **Issue:** `cargo build` cannot verify gpu_plugin.rs compilation without declaring it in mod.rs
- **Fix:** Added `pub mod gpu_plugin;` to output/mod.rs during Task 1 (identical to Plan 01's deviation with gpu_types)
- **Files modified:** engine-port/src/output/mod.rs
- **Verification:** cargo build succeeds with gpu_plugin module compiled
- **Committed in:** 8aa47dc (Task 1 commit)

**2. [Rule 1 - Bug] Guard embedded_asset! for MinimalPlugins test environment**
- **Found during:** Task 2 (integration test failure)
- **Issue:** `embedded_asset!(app, "shader.wgsl")` panics with MinimalPlugins because EmbeddedAssetRegistry resource does not exist without AssetPlugin. 4 resource_flow integration tests failed.
- **Fix:** Added guard `if app.get_sub_app(RenderApp).is_none() { return; }` before the embedded_asset! call
- **Files modified:** engine-port/src/output/gpu_plugin.rs
- **Verification:** All 4 resource_flow tests pass, all 107 unit tests pass
- **Committed in:** a48480f (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both auto-fixes necessary for compilation and test correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full render pipeline is wired: AsciiCellGrid -> Extract -> Prepare (GPU textures) -> ViewNode (fullscreen triangle) -> screen
- Plan 03 (window resize + visual verification) can proceed immediately
- Test pattern system provides synthetic visual data for runtime verification
- Camera2d and font atlas are ready for the render pipeline to produce visible output

---
*Phase: 03-gpu-output*
*Completed: 2026-02-20*
