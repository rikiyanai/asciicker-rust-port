---
phase: 031-audit-remediation
plan: 01
subsystem: rendering, asset-parsers, gpu-output, core
tags: [TextureView, GameVec3, newtype, checked_mul, is_finite, warn, debug_assert, plugin-ordering, RGB555, auto_mat, sample-buffer, reflection]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: Plugin architecture, RenderConfig, SampleBuffer, AsciiCellGrid
  - phase: 02-asset-parsers
    provides: XP sprite parser, A3D world parser, AssetError type
  - phase: 03-gpu-output
    provides: AsciiGpuTextures, AsciiGpuPlugin, gpu_types extract_grid_data
  - phase: 04-cpu-rasterizer-core
    provides: SampleBuffer, quantize, material auto_mat, resolve stage
provides:
  - TextureView lifetime safety in GPU BindGroup creation (both prepare and resize paths)
  - GameVec3 newtype preventing implicit Vec3 assignment at compile time
  - Parser robustness via checked_mul and is_finite validation
  - GPU pipeline hardening with warn! and debug_assert
  - Plugin ordering integration tests (3 tests)
  - Exhaustive RGB555 32768-value validation
  - auto_mat LUT full-table consistency check
  - Dead unsafe accessor removal from SampleBuffer
  - SampleBuffer boundary tests
  - Reflection path tests for resolve stage
affects: [Phase 5 pipeline integration, Phase 6 physics, Phase 7 game systems]

# Tech tracking
tech-stack:
  added: []
  patterns: [newtype-for-type-safety, exhaustive-domain-testing, persistent-gpu-resource-lifetime]

key-files:
  created:
    - engine-port/tests/plugin_ordering.rs
  modified:
    - engine-port/src/output/gpu_plugin.rs
    - engine-port/src/core/coords.rs
    - engine-port/src/core/mod.rs
    - engine-port/src/asset_loader/xp_sprite.rs
    - engine-port/src/asset_loader/a3d_world.rs
    - engine-port/src/asset_loader/error.rs
    - engine-port/src/output/gpu_types.rs
    - engine-port/src/render/sample_buffer.rs
    - engine-port/src/render/quantize.rs
    - engine-port/src/render/material.rs
    - engine-port/src/render/resolve.rs

key-decisions:
  - "GameVec3 newtype uses Deref<Target=Vec3> for ergonomic read access while preventing implicit assignment"
  - "TextureView fields stored in AsciiGpuTextures struct outliving BindGroup (both prepare and resize paths)"
  - "Dead unsafe unchecked SampleBuffer accessors removed since no callers exist in codebase"
  - "Plugin ordering test uses AssetPlugin + ImagePlugin for full 8-plugin init verification"

patterns-established:
  - "Newtype pattern for coordinate safety: wrap Vec3 in GameVec3 to prevent cross-space mixing"
  - "Persistent GPU resource lifetime: store TextureView alongside Texture in resource struct"
  - "Exhaustive domain testing: validate full input space (32768 RGB555 values) not just spot checks"

requirements-completed: [AUDIT-01, AUDIT-02, AUDIT-03, AUDIT-04, AUDIT-05, GAP-02, GAP-03, GAP-06, GAP-10, GAP-11]

# Metrics
duration: 8min
completed: 2026-02-20
---

# Phase 3.1 Plan 01: Audit Remediation Summary

**TextureView lifetime fix, GameVec3 newtype, parser robustness (checked_mul + is_finite), GPU hardening, plugin ordering tests, plus exhaustive RGB555/auto_mat validation and SampleBuffer boundary tests**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-20T22:21:52Z
- **Completed:** 2026-02-20T22:29:45Z
- **Tasks:** 3
- **Files modified:** 11 modified, 1 created

## Accomplishments
- Fixed TextureView use-after-free risk in GPU BindGroup creation (both prepare and resize code paths)
- Replaced GameVec3 type alias with newtype struct preventing implicit coordinate space mixing at compile time
- Added overflow protection (checked_mul) for sprite dimensions and NaN/Inf validation (is_finite) for A3D transforms
- Added warn! for font atlas loading failure and debug_assert for glyph index u16-to-u8 cast
- Created 3 plugin ordering integration tests verifying correct init order, full 8-plugin init, and missing dependency panic
- Validated all 32768 RGB555 values return valid xterm-256 palette indices
- Verified auto_mat LUT full-table consistency (32768 entries, valid fg/bg/glyph)
- Removed dead unsafe unchecked SampleBuffer accessors (no callers in codebase)
- Added SampleBuffer boundary tests (zero-size, border pixels, last valid index)
- Added reflection path tests for the resolve stage

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix all 5 audit items (AUDIT-01 through AUDIT-04)** - `281301e` (fix)
2. **Task 2: Plugin ordering integration test (AUDIT-05)** - `d8039ea` (test)
3. **Task 3: Phase 4 execution gap fixes (GAP-02, GAP-03, GAP-06, GAP-10, GAP-11)** - `78170a3` (test)

## Files Created/Modified
- `engine-port/src/output/gpu_plugin.rs` - Added TextureView fields to AsciiGpuTextures, store views in both prepare and resize paths, warn! on font atlas miss
- `engine-port/src/core/coords.rs` - Replaced GameVec3 type alias with newtype struct, added Deref, conversion methods, updated tests
- `engine-port/src/core/mod.rs` - Re-export continues working (no change needed)
- `engine-port/src/asset_loader/xp_sprite.rs` - Added checked_mul for sprite dimensions, overflow test
- `engine-port/src/asset_loader/a3d_world.rs` - Added is_finite validation on transform matrix, NaN/Inf tests
- `engine-port/src/asset_loader/error.rs` - Added InvalidTransform variant
- `engine-port/src/output/gpu_types.rs` - Added debug_assert for glyph index range check
- `engine-port/tests/plugin_ordering.rs` - New: 3 integration tests for plugin init ordering
- `engine-port/src/render/sample_buffer.rs` - Removed dead unsafe accessors, added 3 boundary tests
- `engine-port/src/render/quantize.rs` - Added exhaustive RGB555 range validation (32768 values) and roundtrip test
- `engine-port/src/render/material.rs` - Added auto_mat LUT full-table consistency and symmetry spot checks
- `engine-port/src/render/resolve.rs` - Added reflection path tests for terrain and mesh resolve

## Decisions Made
- GameVec3 newtype uses `Deref<Target=Vec3>` for ergonomic read access while preventing implicit `Vec3 -> GameVec3` assignment at compile time
- TextureView fields stored in AsciiGpuTextures struct (not as locals) so they outlive the BindGroup in both the initial prepare path and the resize-triggered recreation path
- Dead unsafe unchecked SampleBuffer accessors were removed entirely since grep confirms zero callers in the codebase
- Plugin ordering test for all 8 plugins requires AssetPlugin + ImagePlugin in addition to MinimalPlugins because AssetLoaderPlugin depends on AssetServer and AsciiOutputPlugin loads a font atlas Image

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added AssetPlugin + ImagePlugin to all_plugins_init_in_main_order test**
- **Found during:** Task 2 (plugin_ordering.rs)
- **Issue:** Test panicked because AssetLoaderPlugin requires AssetPlugin (not in MinimalPlugins) and AsciiOutputPlugin requires Image asset type registration
- **Fix:** Added `AssetPlugin::default()` and `bevy::image::ImagePlugin::default()` to the test's plugin set
- **Files modified:** engine-port/tests/plugin_ordering.rs
- **Verification:** All 3 integration tests pass
- **Committed in:** d8039ea (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for the test to compile and run. No scope creep.

## Issues Encountered
None beyond the deviation documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 AUDIT requirements and 5 Phase 4 execution gaps are addressed
- 188 total tests passing (140 lib + 48 integration), zero failures
- Clippy clean with `-D warnings`
- Code is hardened and ready for Phase 5 pipeline integration
- GameVec3 newtype is backward-compatible via Deref; Phase 5 converters will use the newtype

## Self-Check: PASSED

All 12 files verified present. All 3 commit hashes verified in git log.

---
*Phase: 031-audit-remediation*
*Completed: 2026-02-20*
