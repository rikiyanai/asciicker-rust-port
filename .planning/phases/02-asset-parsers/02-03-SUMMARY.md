---
phase: 02-asset-parsers
plan: 03
subsystem: asset-loading
tags: [a3d, world, akm, mesh, ply, binary-parser, golden-file]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: Bevy project structure, plugin architecture, AssetLoaderPlugin stub
  - phase: 02-asset-parsers (plan 01)
    provides: Shared AssetError enum (UnknownInstanceType, NotPly, UnsupportedPlyFormat), constants (A3D_HEADER_SIZE, FILE_PATCH_SIZE, MATERIAL_TABLE_SIZE)
provides:
  - parse_world_section() for A3D world instance parsing (3 variants)
  - parse_akm() for ASCII PLY mesh parsing with flexible property mapping
  - A3dWorld, WorldInstance, AkmMesh, AkmVertex, AkmFace, AkmEdge types
  - Golden-file test assets (test_map.a3d, test_map_no_terrain.a3d, Cube.akm)
affects: [02-04-bevy-integration, 05-pipeline-integration]

# Tech tracking
tech-stack:
  added: []
  patterns: [cursor-based binary parsing, flexible PLY property mapping, golden-file TDD]

key-files:
  created:
    - engine-port/src/asset_loader/a3d_world.rs
    - engine-port/src/asset_loader/akm_mesh.rs
    - engine-port/tests/a3d_world_parser.rs
    - engine-port/tests/akm_mesh_parser.rs
    - engine-port/tests/golden/a3d/test_map.a3d
    - engine-port/tests/golden/a3d/test_map_no_terrain.a3d
    - engine-port/tests/golden/akm/Cube.akm
  modified:
    - engine-port/src/asset_loader/mod.rs

key-decisions:
  - "Used cursor-based binary parsing with explicit read helpers instead of bytemuck (world section has variable-length fields)"
  - "Hand-rolled PLY parser (~230 lines) with property_code mapping instead of ply-rs crate"
  - "Default alpha=255 when PLY file lacks alpha property"

patterns-established:
  - "Cursor-advancing read helpers (read_i32, read_f32, read_f64, read_string, read_len_prefixed_string) for variable-length binary formats"
  - "Property code mapping for flexible PLY vertex parsing (skip unknown properties)"

requirements-completed: [ASSET-04, ASSET-05]

# Metrics
duration: 6min
completed: 2026-02-20
---

# Phase 2 Plan 3: A3D World and AKM Mesh Parsers Summary

**A3D world parser with format-version detection and 3-variant instance dispatch, plus ASCII PLY mesh parser with flexible property mapping and freestyle/edge support**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-20T14:19:13Z
- **Completed:** 2026-02-20T14:24:48Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- World section parser handles format version detection (negative first i32 = versioned)
- Three instance variants correctly parsed: Mesh (mesh_id_len >= 0), Sprite (-1), Item (-2)
- .ply to .akm extension conversion applied to mesh IDs
- story_id conditionally read when format_version > 0
- AKM parser handles full Cube.akm with normals/UVs skipped via property mapping
- Freestyle faces (negative vertex count) and 2-vertex edges supported
- 12 total tests (6 world + 6 AKM), all passing with zero clippy warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy test assets, register modules, TDD world parser** - `e43a6c3` (feat)
2. **Task 2: TDD AKM mesh parser (RED -> GREEN -> REFACTOR)** - `4dc7a00` (feat)

## Files Created/Modified
- `engine-port/src/asset_loader/a3d_world.rs` - World section parser with 3 instance variants, format version detection, cursor-based binary reading
- `engine-port/src/asset_loader/akm_mesh.rs` - ASCII PLY parser with flexible property mapping, freestyle marks, edge support
- `engine-port/src/asset_loader/mod.rs` - Added a3d_world and akm_mesh module declarations
- `engine-port/tests/a3d_world_parser.rs` - 6 golden-file tests for world parsing
- `engine-port/tests/akm_mesh_parser.rs` - 6 golden-file tests for AKM parsing
- `engine-port/tests/golden/a3d/test_map.a3d` - Test asset: 3 mesh instances, format v1
- `engine-port/tests/golden/a3d/test_map_no_terrain.a3d` - Test asset: 19 mesh instances, format v1
- `engine-port/tests/golden/akm/Cube.akm` - Test asset: 24 vertices, 12 faces

## Decisions Made
- Used cursor-based binary parsing with explicit read helpers instead of bytemuck for the world section, since it has variable-length fields (strings, discriminant-based dispatch) that do not map well to fixed-layout structs
- Hand-rolled PLY parser (~230 lines) with property_code mapping instead of ply-rs crate, since .akm is a restricted PLY subset and hand-rolling avoids pulling in unnecessary dependencies
- Default alpha=255 when PLY file lacks alpha property declaration, matching the C++ engine behavior

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- World parser and AKM parser ready for Bevy AssetLoader integration in Plan 02-04
- All golden-file test assets in place for integration testing
- parse_world_section and parse_akm functions are pure (no Bevy dependency) and ready to be called from AssetLoader::load()

## Self-Check: PASSED

- All 7 created files verified on disk
- Both task commits (e43a6c3, 4dc7a00) found in git log
- Line counts: a3d_world.rs=281 (min 100), akm_mesh.rs=301 (min 80), tests=160+131 (min 40+30)
- 12/12 tests passing, 0 clippy warnings

---
*Phase: 02-asset-parsers*
*Completed: 2026-02-20*
