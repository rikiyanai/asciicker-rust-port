---
phase: 02-asset-parsers
plan: 04
subsystem: asset-loading
tags: [bevy-asset-loader, async-loading, handle, labeled-sub-assets, golden-file-tests, integration-tests]

# Dependency graph
requires:
  - phase: 02-asset-parsers (plan 01)
    provides: "XpSprite type, parse_xp() function, AssetError enum, constants module"
  - phase: 02-asset-parsers (plan 02)
    provides: "A3dTerrain, MaterialTable types, parse_terrain_section(), parse_material_section()"
  - phase: 02-asset-parsers (plan 03)
    provides: "A3dWorld, AkmMesh types, parse_world_section(), parse_akm()"
provides:
  - "XpSpriteLoader: Bevy AssetLoader for .xp files via asset_server.load::<XpSprite>()"
  - "A3dFileLoader: Bevy AssetLoader for composite .a3d files with labeled sub-assets (terrain/materials/world)"
  - "A3dFile composite asset type with Handle<A3dTerrain>, Handle<MaterialTable>, Handle<A3dWorld>"
  - "AkmMeshLoader: Bevy AssetLoader for .akm files via asset_server.load::<AkmMesh>()"
  - "AssetLoaderPlugin registers all loaders via init_asset + register_asset_loader"
  - "14 new tests: 11 golden-file integration + 3 Bevy AssetServer pipeline tests"
affects: [05-pipeline-integration, terrain-rendering, world-loading]

# Tech tracking
tech-stack:
  added: []
  patterns: [bevy-asset-loader-trait, composite-labeled-sub-assets, async-handle-resolution, bevy-app-integration-testing]

key-files:
  created:
    - engine-port/src/asset_loader/bevy_loaders.rs
    - engine-port/tests/asset_integration.rs
    - engine-port/tests/bevy_asset_loading.rs
  modified:
    - engine-port/src/asset_loader/mod.rs
    - engine-port/src/asset_loader/xp_sprite.rs
    - engine-port/src/asset_loader/a3d_terrain.rs
    - engine-port/src/asset_loader/a3d_world.rs
    - engine-port/src/asset_loader/akm_mesh.rs

key-decisions:
  - "Added TypePath derive to loader structs (XpSpriteLoader, A3dFileLoader, AkmMeshLoader) as required by Bevy 0.18 AssetLoader trait bound"
  - "Used add_labeled_asset for A3D composite sub-assets rather than nested loader, keeping parsing synchronous within the async load"
  - "Bevy integration tests use MinimalPlugins + AssetPlugin with file_path pointing to tests/golden directory"

patterns-established:
  - "Bevy AssetLoader pattern: read_to_end -> parse -> return asset (simple loaders) or add_labeled_asset (composite)"
  - "Bevy App integration test pattern: build_test_app() + wait_for_load() polling loop for async asset resolution"

requirements-completed: [ASSET-06, ASSET-07]

# Metrics
duration: 5min
completed: 2026-02-20
---

# Phase 2 Plan 04: Bevy Asset Integration Summary

**Three Bevy AssetLoader implementations (XpSprite, A3dFile composite, AkmMesh) with labeled sub-assets and 14 golden-file + AssetServer pipeline integration tests**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-20T14:31:19Z
- **Completed:** 2026-02-20T14:36:05Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- XpSpriteLoader, A3dFileLoader, AkmMeshLoader all implement Bevy AssetLoader trait with correct async pattern
- A3dFile composite asset uses labeled sub-assets ("terrain", "materials", "world") for Handle-based access to sections
- AssetLoaderPlugin registers all 6 asset types and 3 loaders in build()
- 11 golden-file integration tests exercise all parsers end-to-end with real game assets
- 3 Bevy App integration tests confirm Handle<XpSprite>, Handle<AkmMesh>, Handle<A3dFile> resolve through AssetServer pipeline
- Total test count: 80 tests, all passing, zero clippy warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement Bevy AssetLoader wrappers and register in plugin** - `2405bc3` (feat)
2. **Task 2: Golden-file integration tests and Bevy AssetServer integration test** - `7d39f3f` (test)

## Files Created/Modified
- `engine-port/src/asset_loader/bevy_loaders.rs` - Three AssetLoader implementations (XpSpriteLoader, A3dFileLoader, AkmMeshLoader) + A3dFile composite asset type (~165 lines)
- `engine-port/tests/asset_integration.rs` - 11 golden-file integration tests covering XP, A3D terrain/material/world, AKM, and error handling (~235 lines)
- `engine-port/tests/bevy_asset_loading.rs` - 3 Bevy App integration tests verifying Handle-typed async loading (~105 lines)
- `engine-port/src/asset_loader/mod.rs` - AssetLoaderPlugin registers all asset types and loaders
- `engine-port/src/asset_loader/xp_sprite.rs` - Added Asset/TypePath derives to XpSprite
- `engine-port/src/asset_loader/a3d_terrain.rs` - Added Asset/TypePath derives to A3dTerrain, MaterialTable
- `engine-port/src/asset_loader/a3d_world.rs` - Added Asset/TypePath derives to A3dWorld
- `engine-port/src/asset_loader/akm_mesh.rs` - Added Asset/TypePath derives to AkmMesh

## Decisions Made
- Added `TypePath` derive to all three loader structs because Bevy 0.18's `AssetLoader` trait has `TypePath` as a supertrait bound (not just on the asset types)
- Used `add_labeled_asset` (synchronous registration) for A3D sub-assets rather than nested loader, since all three sections are parsed from the same byte buffer in sequence
- Bevy integration tests use `MinimalPlugins + AssetPlugin { file_path: "tests/golden" }` to load real files through the full pipeline without requiring a windowed app

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added TypePath derive to loader structs**
- **Found during:** Task 1 (compilation)
- **Issue:** Bevy 0.18 AssetLoader trait requires `TypePath` on the loader struct itself, not just the asset type. The plan only mentioned `#[derive(Default)]`.
- **Fix:** Added `bevy::reflect::TypePath` derive to XpSpriteLoader, A3dFileLoader, AkmMeshLoader
- **Files modified:** engine-port/src/asset_loader/bevy_loaders.rs
- **Verification:** `cargo build` succeeds
- **Committed in:** 2405bc3

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial derive addition required by Bevy API. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All Phase 2 asset parsers complete: XP sprites, A3D terrain, A3D materials, A3D world, AKM meshes
- All parsers integrated with Bevy AssetServer via Handle-typed async loading
- 80 total tests across 8 test files, all passing
- Ready for Phase 3+ systems to load assets via `asset_server.load::<XpSprite>("sprite.xp")` etc.

## Self-Check: PASSED

- [x] bevy_loaders.rs exists (168 lines, min 80)
- [x] asset_integration.rs exists (295 lines, min 60)
- [x] bevy_asset_loading.rs exists (105 lines, min 40)
- [x] 02-04-SUMMARY.md exists
- [x] Commit 2405bc3 found
- [x] Commit 7d39f3f found
- [x] 80 tests passing, 0 failures
- [x] Zero clippy warnings
- [x] Formatting clean

---
*Phase: 02-asset-parsers*
*Completed: 2026-02-20*
