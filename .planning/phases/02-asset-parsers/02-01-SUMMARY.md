---
phase: 02-asset-parsers
plan: 01
subsystem: asset-loading
tags: [xp-sprite, gzip, flate2, cp437, column-major, swoosh-merge, golden-file-tests]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: compiling Bevy 0.18 project with asset_loader module stub
provides:
  - AssetError shared error enum for all loaders
  - constants module with sprite and terrain constants
  - XpSprite/XpLayer/XpCell types for parsed .xp sprite data
  - parse_xp() function: gzip decompress, column-major cell reading, layer validation
  - merge_layers() function: L0 colorkey, L1 height decode, L2 visual base, swoosh merge
  - MergedCell type with transparency, height, and final glyph/color data
  - Golden-file test infrastructure with real C++ game assets
affects: [02-asset-parsers, 05-pipeline-integration]

# Tech tracking
tech-stack:
  added: [flate2 1.0 (gzip), bytemuck 1.x (binary struct casting)]
  patterns: [column-major cell indexing, per-layer 8-byte gap navigation, swoosh detection via cyan fg + half-block glyphs]

key-files:
  created:
    - engine-port/src/asset_loader/error.rs
    - engine-port/src/asset_loader/constants.rs
    - engine-port/src/asset_loader/xp_sprite.rs
    - engine-port/tests/xp_parser.rs
    - engine-port/tests/golden/xp/item-apple.xp
    - engine-port/tests/golden/xp/grid-water.xp
  modified:
    - engine-port/Cargo.toml
    - engine-port/src/asset_loader/mod.rs

key-decisions:
  - "Used i32 for version field since XP format version is -1 (signed)"
  - "Deferred full AverageGlyphTransp sub-cell blending to Phase 5 as planned"
  - "Stored half_block_mask value for future Phase 5 use despite basic lighten-only implementation"

patterns-established:
  - "read_i32_le/read_cell helpers for safe little-endian binary parsing with offset tracking"
  - "Golden-file test pattern: include_bytes from tests/golden/ directory"
  - "Synthetic XP payload construction via make_xp_bytes helper for complex test scenarios"

requirements-completed: [ASSET-01, ASSET-02]

# Metrics
duration: 6min
completed: 2026-02-20
---

# Phase 2 Plan 1: XP Sprite Parser Summary

**Gzip-compressed .xp sprite parser with column-major cell reading, 3-layer validation, height decoding, and last-layer swoosh merge detection using flate2**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-20T14:09:25Z
- **Completed:** 2026-02-20T14:15:36Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- parse_xp() correctly decompresses gzip, reads 16-byte header, validates 3+ layers, and parses column-major cells with 8-byte inter-layer gaps
- merge_layers() implements full layer merge: L0 colorkey transparency, L1 height decoding ('0'-'9'=0-9, 'A'-'Z'=10-35), L2 visual base, intermediate overwrites, last-layer swoosh detection (cyan fg + half-block glyphs) with lighten effect
- Shared AssetError enum and constants module established for all subsequent parsers
- 8 golden-file integration tests + 8 unit tests all pass against real C++ game assets

## Task Commits

Each task was committed atomically:

1. **Task 1: Create shared error types, constants, add dependencies, copy test assets** - `93268b9` (chore)
2. **Task 2 RED: Failing XP parser tests** - `712af52` (test)
3. **Task 2 GREEN: XP parser implementation** - `9c0465b` (feat)
4. **Task 2 REFACTOR: Clippy/fmt cleanup** - `dec4ba1` (refactor)

## Files Created/Modified
- `engine-port/src/asset_loader/error.rs` - Shared AssetError enum with 11 variants covering all loader error types
- `engine-port/src/asset_loader/constants.rs` - Sprite constants (SPRITE_MIN_LAYERS, swoosh markers, half-block glyphs, masks, lighten amount) and terrain constants (HEIGHT_CELLS, A3D_MAGIC, FILE_PATCH_SIZE, MATERIAL_TABLE_SIZE)
- `engine-port/src/asset_loader/xp_sprite.rs` - XpSprite/XpLayer/XpCell types, parse_xp function, MergedCell type, merge_layers function (~390 lines)
- `engine-port/tests/xp_parser.rs` - 8 integration tests: header parsing, column-major layout, layer count, cell structure, invalid input, height encoding, swoosh merge
- `engine-port/tests/golden/xp/item-apple.xp` - 69-byte test sprite (2x2, 3 layers)
- `engine-port/tests/golden/xp/grid-water.xp` - 141-byte test sprite (7x7, 3 layers)
- `engine-port/Cargo.toml` - Added flate2 and bytemuck dependencies
- `engine-port/src/asset_loader/mod.rs` - Added error, constants, xp_sprite module declarations

## Decisions Made
- Used i32 for XP version field since the format version is -1 (negative signed integer)
- Deferred full AverageGlyphTransp (sub-cell blending with partial transparency) to Phase 5 rendering integration as recommended by research -- the basic swoosh merge handles detection, half-block masking, and lighten effect
- Stored half_block_mask return value as _mask for future Phase 5 use when per-quadrant blending is needed

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- AssetError and constants modules are ready for A3D terrain parser (plan 02-02)
- Golden-file test infrastructure pattern established for subsequent parsers
- bytemuck dependency added and available for FilePatch struct casting in terrain parser

---
*Phase: 02-asset-parsers*
*Completed: 2026-02-20*
