# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-20)

**Core value:** The CPU rasterizer must produce visually identical output to the C++ engine -- same glyphs, same colors, same depth ordering -- so that existing Asciicker worlds render correctly in the Rust port.
**Current focus:** Phase 3: GPU Output (03-03 Task 2 human checkpoint pending)

## Current Position

Phase: 3 of 7 (GPU Output) -- IN PROGRESS (Task 2 human checkpoint pending)
Plan: 3 of 3 in current phase (03-03-PLAN.md Task 1 committed, Task 2 pending human verification)
Status: Phase 4 complete (commit `0cdfc24`), Phase 3 awaiting human visual verification checkpoint
Last activity: 2026-02-20 -- Applied MSAA fix (F005), committed resize handler (03-03 Task 1)

Progress: [########..] 76%

**Note:** Phase 4 completed in parallel with Phase 3 (both depend only on Phase 1). Phase 3 is blocked on human visual verification (03-03 Task 2). Three uncommitted changes exist: Msaa::Off fix on Camera2d, STATE.md decision entries, config.json trailing newline.

## Performance Metrics

**Velocity:**
- Total plans completed: 12
- Average duration: ~5 min
- Total execution time: ~1.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 - Foundation | 2 | ~12 min | ~6 min |
| 2 - Asset Parsers | 4 | 21 min | ~5 min |
| 3 - GPU Output | 2 | 15 min | ~8 min |
| 4 - CPU Rasterizer Core | 4 | 30 min | ~8 min |

**Recent Trend:**
- Last 5 plans: 03-02, 04-03, 04-04, 03-03 (Task 1 only)
- Trend: Consistent ~7-8 min per plan
- Note: 03-03 Task 2 is a human checkpoint (not automated)

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
- 03-02: Guard AsciiGpuPlugin with RenderApp existence check before embedded_asset! (supports MinimalPlugins)
- 03-02: ExtractedFontAtlasHandle as separate render-world resource for font atlas between extract and prepare
- 03-02: RenderStartup schedule for pipeline init instead of Plugin::finish (Bevy 0.18 BlitPipeline pattern)
- 03-03: Msaa::Off as Component on Camera2d (Bevy 0.18 moved MSAA from Resource to per-camera Component)
- 03-03: Bevy 0.18 renamed EventReader to MessageReader (events are now "Messages")
- 04-04: MaterialResolveCtx struct to group resolve parameters (clippy too-many-arguments)
- 04-04: Mesh flag in combined spare OR determines mesh vs material path (matches C++ behavior)
- 04-04: Elevation thresholds 0.5/2.0/5.0 for height-difference to 0-3 mapping
- 04-04: Grid overlay uses positional parity for +/-/| selection (simplified for Phase 4)

### Pending Todos

- [ ] Phase 3 plan 03-03 Task 2: Human visual verification checkpoint (run `cargo run` from engine-port/, verify checkerboard renders, test window resize)
- [ ] Commit uncommitted Msaa::Off fix on Camera2d in `engine-port/src/output/mod.rs` (F005 resolution)
- [ ] Phase 4 REND-10: Release-mode performance benchmark not yet executed (human-verification item from 04-VERIFICATION.md)
- [ ] Commit STATE.md and config.json uncommitted changes

### Blockers/Concerns

- bevy_kira_audio must be 0.25 (not 0.24) for Bevy 0.18 compatibility
- lightyear 0.24.x Bevy 0.18 compatibility unverified (Phase 7 concern, not blocking now)

## Session Continuity

Last session: 2026-02-20
Stopped at: Phase 4 complete (commit `0cdfc24`). Phase 3 plan 03-03 Task 1 committed (`0dfe33d`), Task 2 (human visual checkpoint) pending. Three uncommitted changes: Msaa::Off fix, STATE.md updates, config.json trailing newline.
Resume file: None
