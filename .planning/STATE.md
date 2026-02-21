# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-20)

**Core value:** The CPU rasterizer must produce visually identical output to the C++ engine -- same glyphs, same colors, same depth ordering -- so that existing Asciicker worlds render correctly in the Rust port.
**Current focus:** Phase 5: Pipeline Integration (next after Phase 3.1 audit remediation)

## Current Position

Phase: 3.1 of 7 (Audit Remediation) -- COMPLETE
Plan: 1 of 1 in current phase (031-01 complete)
Status: Phase 3.1 complete (commits `281301e`, `d8039ea`, `78170a3`). Ready for Phase 5.
Last activity: 2026-02-20 -- Completed audit remediation: TextureView fix, GameVec3 newtype, parser robustness, plugin ordering tests, Phase 4 gap closures

Progress: [########..] 80%

**Note:** Phase 3.1 inserted between Phase 4 and Phase 5. All audit items and Phase 4 execution gaps resolved. 188 tests passing (140 lib + 48 integration). Phase 3 human visual verification checkpoint (03-03 Task 2) still pending from prior session.

## Performance Metrics

**Velocity:**
- Total plans completed: 13
- Average duration: ~6 min
- Total execution time: ~1.1 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 - Foundation | 2 | ~12 min | ~6 min |
| 2 - Asset Parsers | 4 | 21 min | ~5 min |
| 3 - GPU Output | 2 | 15 min | ~8 min |
| 3.1 - Audit Remediation | 1 | 8 min | 8 min |
| 4 - CPU Rasterizer Core | 4 | 30 min | ~8 min |

**Recent Trend:**
- Last 5 plans: 04-03, 04-04, 03-03 (Task 1 only), 031-01
- Trend: Consistent ~8 min per plan

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
- 031-01: GameVec3 newtype uses Deref<Target=Vec3> for ergonomic read access while preventing implicit assignment
- 031-01: TextureView fields stored in AsciiGpuTextures struct outliving BindGroup (both prepare and resize paths)
- 031-01: Dead unsafe unchecked SampleBuffer accessors removed (zero callers in codebase)
- 031-01: Plugin ordering test uses AssetPlugin + ImagePlugin for full 8-plugin init verification

### Pending Todos

- [ ] Phase 3 plan 03-03 Task 2: Human visual verification checkpoint (run `cargo run` from engine-port/, verify checkerboard renders, test window resize)
- [ ] Commit uncommitted Msaa::Off fix on Camera2d in `engine-port/src/output/mod.rs` (F005 resolution)
- [ ] Phase 4 REND-10: Release-mode performance benchmark not yet executed (human-verification item from 04-VERIFICATION.md)
- [ ] Commit STATE.md and config.json uncommitted changes

### Blockers/Concerns

- bevy_kira_audio must be 0.25 (not 0.24) for Bevy 0.18 compatibility
- bevy_replicon_renet2 must be 0.13 for Bevy 0.18 compatibility (Phase 7, not blocking now)

## Session Continuity

Last session: 2026-02-20
Stopped at: Completed 031-01-PLAN.md (Phase 3.1 Audit Remediation). All 10 requirements addressed. Ready for Phase 5 pipeline integration.
Resume file: None
