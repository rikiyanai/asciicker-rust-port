---
phase: 02-asset-parsers
plan: 02
subsystem: asset-parsing
tags: [a3d, terrain, material-table, bytemuck, binary-parser, golden-file-tests]

# Dependency graph
requires:
  - phase: 02-asset-parsers
    provides: "Shared error types (AssetError), constants (A3D_MAGIC, FILE_PATCH_SIZE, MATERIAL_TABLE_SIZE, HEIGHT_CELLS, VISUAL_CELLS)"
provides:
  - "A3dTerrain type with Vec<TerrainPatch> (x, y, 8x8 visual, 5x5 height, diag)"
  - "MaterialTable type with 256 materials x 4 elevations x 16 diffuse MatCell entries"
  - "parse_terrain_section() and parse_material_section() functions"
  - "Golden-file tests for minimal_1x1.a3d (1 patch) and minimal_2x2.a3d (4 patches)"
affects: [04-a3d-composite-loader, terrain-rendering]

# Tech tracking
tech-stack:
  added: []
  patterns: [bytemuck-zero-copy-casting, packed-struct-to-owned-copy, compile-time-size-assertions]

key-files:
  created:
    - engine-port/src/asset_loader/a3d_terrain.rs
    - engine-port/tests/a3d_terrain_parser.rs
    - engine-port/tests/golden/a3d/minimal_1x1.a3d
    - engine-port/tests/golden/a3d/minimal_2x2.a3d
  modified:
    - engine-port/src/asset_loader/mod.rs

key-decisions:
  - "Used HEIGHT_CELLS_PLUS_ONE const for array type (Rust requires const in array sizes)"
  - "Copy data from packed FilePatch to owned TerrainPatch to avoid unaligned access UB"
  - "Manual byte-by-byte MatCell parsing (8-byte cells not suitable for bytemuck without derive)"

patterns-established:
  - "bytemuck::from_bytes for fixed-layout binary structs with compile-time size assertions"
  - "parse function returns (T, usize) tuple where usize = bytes consumed for sequential section parsing"

requirements-completed: [ASSET-03]

# Metrics
duration: 4min
completed: 2026-02-20
---

# Phase 2 Plan 02: A3D Terrain Parser Summary

**A3D terrain section parser with bytemuck zero-copy FileHeader/FilePatch casting and 131KB material table LUT, verified against minimal_1x1 (1 patch) and minimal_2x2 (4 patches) golden files**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-20T14:18:25Z
- **Completed:** 2026-02-20T14:22:51Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- FileHeader (16 bytes) and FilePatch (188 bytes) parsed via bytemuck with compile-time size assertions
- parse_terrain_section validates AS3D magic 0x44335341, reads N packed patches, returns bytes consumed
- parse_material_section reads 131,072-byte fixed LUT (256 materials x 4 elevations x 16 diffuse levels x 8-byte MatCell)
- 7 golden-file integration tests plus 3 unit tests all passing
- Zero clippy warnings, clean formatting

## Task Commits

Each task was committed atomically:

1. **Task 1: Copy A3D test assets and register terrain module** - `ba1205e` (chore)
2. **Task 2 RED: Add failing tests for A3D terrain parser** - `9083a73` (test)
3. **Task 2 GREEN: Implement A3D terrain and material table parsers** - `ecdfcc3` (feat)
4. **Task 2 REFACTOR: Clean up test imports and formatting** - `c1b02fa` (refactor)

## Files Created/Modified
- `engine-port/src/asset_loader/a3d_terrain.rs` - A3D terrain and material table parser (FileHeader, FilePatch, TerrainPatch, A3dTerrain, MatCell, MaterialTable, parse_terrain_section, parse_material_section)
- `engine-port/tests/a3d_terrain_parser.rs` - 7 golden-file integration tests
- `engine-port/tests/golden/a3d/minimal_1x1.a3d` - Test asset: 1 terrain patch
- `engine-port/tests/golden/a3d/minimal_2x2.a3d` - Test asset: 4 terrain patches
- `engine-port/src/asset_loader/mod.rs` - Added `pub mod a3d_terrain;` declaration

## Decisions Made
- Used `HEIGHT_CELLS_PLUS_ONE` const for array sizes since Rust requires const expressions in array types
- Copy data from packed `FilePatch` to owned `TerrainPatch` to avoid unaligned access UB when reading packed struct fields
- Manual byte-by-byte parsing for `MatCell` (8-byte cells with mixed field sizes not ideal for bytemuck derive)
- Parse functions return `(T, usize)` tuple pattern for sequential section parsing (terrain consumed offset feeds material table start)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created stub files for concurrent Plan 02-03 module declarations**
- **Found during:** Task 2 (RED phase)
- **Issue:** Plan 02-03 already added `pub mod a3d_world;` and `pub mod akm_mesh;` to mod.rs, but the actual source files did not exist yet, causing compilation failure
- **Fix:** Created empty stub files `a3d_world.rs` and `akm_mesh.rs` with comments noting they are placeholders for Plan 02-03
- **Files modified:** engine-port/src/asset_loader/a3d_world.rs, engine-port/src/asset_loader/akm_mesh.rs
- **Verification:** `cargo build` succeeds
- **Committed in:** 9083a73 (RED phase commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for compilation with concurrent Plan 02-03 module declarations. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- A3D terrain parser ready for use by Plan 04 (A3D composite loader)
- `parse_terrain_section` returns bytes consumed, enabling sequential parsing of material table and world section
- Material table parser ready for integration with terrain rendering in Phase 5

## Self-Check: PASSED

- [x] a3d_terrain.rs exists
- [x] a3d_terrain_parser.rs exists
- [x] minimal_1x1.a3d exists
- [x] minimal_2x2.a3d exists
- [x] 02-02-SUMMARY.md exists
- [x] Commit ba1205e found
- [x] Commit 9083a73 found
- [x] Commit ecdfcc3 found
- [x] Commit c1b02fa found

---
*Phase: 02-asset-parsers*
*Completed: 2026-02-20*
