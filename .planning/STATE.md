# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-20)

**Core value:** The CPU rasterizer must produce visually identical output to the C++ engine -- same glyphs, same colors, same depth ordering -- so that existing Asciicker worlds render correctly in the Rust port.
**Current focus:** Phase 4: CPU Rasterizer Core

## Current Position

Phase: 4 of 7 (CPU Rasterizer Core)
Plan: 3 of 4 in current phase
Status: Executing
Last activity: 2026-02-20 -- Completed 04-03-PLAN.md (Rasterizer core: Bresenham + barycentric triangle)

Progress: [#######...] 68%

## Performance Metrics

**Velocity:**
- Total plans completed: 10
- Average duration: ~5 min
- Total execution time: ~0.8 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 - Foundation | 2 | ~12 min | ~6 min |
| 2 - Asset Parsers | 4 | 21 min | ~5 min |
| 3 - GPU Output | 1 | 5 min | 5 min |
| 4 - CPU Rasterizer Core | 3 | 22 min | ~7 min |

**Recent Trend:**
- Last 5 plans: 03-01, 04-01, 04-02, 04-03
- Trend: Consistent ~6-8 min per plan

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- D001: Use Bevy 0.18 engine (ECS, input, audio, windowing)
- D003: CPU rasterizer first, GPU only for final ASCII output
- D010: Keep auto_mat initially, upgrade to Alex Harri 6D shape vectors later
- 02-01: Used i32 for XP version field (format version is -1)
- 02-01: Deferred full AverageGlyphTransp to Phase 5; basic swoosh merge (detect + lighten) in Phase 2
- 02-01: Stored half_block_mask for future Phase 5 per-quadrant blending
- 02-02: Used HEIGHT_CELLS_PLUS_ONE const for array sizes (Rust requires const in array types)
- 02-02: Copy from packed FilePatch to owned TerrainPatch to avoid unaligned access UB
- 02-02: Parse functions return (T, usize) for sequential section offset tracking
- 02-03: Cursor-based binary parsing with read helpers for variable-length world section
- 02-03: Hand-rolled PLY parser with property_code mapping instead of ply-rs
- 02-03: Default alpha=255 when PLY file lacks alpha property
- 02-04: TypePath derive required on loader structs (not just asset types) for Bevy 0.18 AssetLoader
- 02-04: add_labeled_asset for A3D composite sub-assets (synchronous within async load)
- 02-04: Bevy integration tests use MinimalPlugins + AssetPlugin with file_path for golden directory
- 04-01: Sample clear state uses sky-blue RGB555 (0x6D8C) with MESH_FLAG, matching C++ init
- 04-01: Removed supersample_factor; dimensions always 2*ascii+4 (implicit 2x + border)
- 04-01: Used unsafe blocks in unsafe fn for Rust 2024 edition compliance
- 03-01: Used Mage Core font1.png (10x16 per glyph) directly as CP437 atlas
- 03-01: Graceful AssetServer fallback via get_resource (supports MinimalPlugins in tests)
- 03-01: Shader imports Bevy fullscreen vertex output instead of custom vertex stage
- 04-02: auto_mat uses Box<[u8; 98304]> + LazyLock to avoid 98KB stack allocation
- 04-02: Pure black (0,0,0) correctly gets fg=52 dither partner with space glyph (invisible dither)
- 04-02: mcv_to_5 formula (mcv*5+2)/5 matches C++ integer rounding for MCV-to-palette conversion
- 04-03: Used (1.0 - f32::EPSILON) / area normalizer to match C++ FLT_EPSILON behavior
- 04-03: Extracted rasterize_ccw and rasterize_cw as separate functions matching C++ branch structure
- 04-03: RasterShader::blend takes &self for immutable shader state during rasterization

### Pending Todos

None yet.

### Blockers/Concerns

- bevy_kira_audio must be 0.25 (not 0.24) for Bevy 0.18 compatibility
- lightyear 0.24.x Bevy 0.18 compatibility unverified (Phase 7 concern, not blocking now)

## Session Continuity

Last session: 2026-02-20
Stopped at: Completed 04-03-PLAN.md (Rasterizer core: Bresenham + barycentric triangle)
Resume file: None
