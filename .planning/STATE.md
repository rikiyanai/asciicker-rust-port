# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-20)

**Core value:** The CPU rasterizer must produce visually identical output to the C++ engine -- same glyphs, same colors, same depth ordering -- so that existing Asciicker worlds render correctly in the Rust port.
**Current focus:** Phase 7 in progress: Game Systems.

## Current Position

Phase: 7 of 15 (Game Systems)
Plan: 2 of 5 in current phase (07-02 complete)
Status: Phase 7 in progress. Plans 07-01, 07-02 complete. 3 plans remaining (07-03, 07-04, 07-05).
Last activity: 2026-02-25 -- Completed 07-02: GameState FSM (MainMenu/Loading/Playing/Paused), main menu, state-gated gameplay systems

Progress: [#########=] 88%

**Note:** 07-02 complete. 379 lib tests passing (14 new state/menu tests). GameState controls all system execution flow.

## Performance Metrics

**Velocity:**
- Total plans completed: 22
- Average duration: ~7 min
- Total execution time: ~2.6 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 - Foundation | 2 | ~12 min | ~6 min |
| 2 - Asset Parsers | 4 | 21 min | ~5 min |
| 3 - GPU Output | 2 | 15 min | ~8 min |
| 3.1 - Audit Remediation | 1 | 8 min | 8 min |
| 4 - CPU Rasterizer Core | 4 | 30 min | ~8 min |
| 5 - Pipeline Integration | 8 | 93 min | ~12 min |

**Recent Trend:**
- Last 5 plans: 06-01, 06-02, 06-03, 07-01, 07-02
- Trend: Consistent ~6-25 min per plan

*Updated after each plan completion*
| Phase 05 P01 | 17min | 2 tasks | 5 files |
| Phase 05 P04 | 11min | 2 tasks | 6 files |
| Phase 05 P05 | 13min | 3 tasks | 7 files |
| Phase 05 P06 | 20min | 2 tasks | 4 files |
| Phase 05 P08 | 6min | 1 tasks | 1 files |
| Phase 05 P07 | 10min | 2 tasks | 3 files |
| Phase 06 P01 | 12min | 2 tasks | 8 files |
| Phase 06 P02 | 13min | 2 tasks | 11 files |
| Phase 06 P03 | 25min | 2 tasks | 32 files |
| Phase 07 P01 | 16min | 2 tasks | 6 files |
| Phase 07 P02 | 18min | 2 tasks | 5 files |

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
- 05-03: Ported C++ view matrix exactly: DBL_SCALE=3.0, ds=2*zoom*scale/VISUAL_CELLS, sin30=0.5
- 05-03: Frustum uses PlaneFromPoints (perspective) and TransposeProduct (isometric) matching C++ branches
- 05-03: ButtonInput<KeyCode> confirmed as Bevy 0.18 API; MinimalPlugins test needs manual resource init
- 05-03: Camera stores mul[6]/add[3] arrays for terrain/world query compatibility
- 05-01: HEIGHT_CELLS_PLUS_ONE promoted to constants.rs for shared access (F032 FIX)
- 05-01: interpolate_height returns Option<f64>; Phase 5 shadow uses f64, Phase 6 physics casts to f32 at call site
- 05-01: TerrainPlugin explicitly calls init_resource::<RuntimeTerrain>() (XP-114 FIX)
- 05-01: QuadNode uses Option<Box<QuadNode>> children for sparse quadtree representation
- 05-02: BspNode::NodeShare uses fixed-order [0,1] traversal (no near-child-first) matching C++ behavior
- 05-02: Items always skip BSP tree (P5-066 FIX), go to flat_list
- 05-02: Split plane set to median centroid coordinate (P5-074 FIX)
- 05-02: WorldPlugin::build() calls app.init_resource::<RuntimeWorld>() (XP-114 FIX)
- 05-04: TerrainShader uses inline depth test (not depth_test_ro) due to semantic inversion
- 05-04: render_patch subdivides height-cell quads into 2x2 visual sub-quads for material mapping
- 05-04: resolve_to_grid uses generic GlyphSelector (not dyn) for monomorphization
- 05-04: XTERM_256_PALETTE uses evenly-spaced levels matching rgb2pal round-tripping
- 05-04: transform_vertex extracted to render/math.rs as shared utility
- 05-05: ResolveBuffer allocated locally in pipeline (not as Resource) per R14-F148
- 05-05: SpriteQueue cleared at Stage 3 WORLD start (Phase 5; Phase 6 migrates to PreUpdate)
- 05-05: test_pattern_system gated behind cfg(feature = test_pattern) to prevent overwriting pipeline output
- 05-05: Plugin order fixed: AsciiOutputPlugin before CpuRasterizerPlugin (R14-F124)
- 05-05: Mesh rendering deferred until AKM mesh data loaded via MeshRegistry
- 05-06: Light direction Z positive (toward light above terrain); plan had Z=-2.0 which shadowed everything
- 05-06: Two-pass shadow: immutable collect + mutable write avoids borrow conflict
- 05-06: compare_rgba_grids for determinism (no round-trip); compare_ansi_grids for C++ reference only
- 05-06: R14-SYNTH-BAN enforced: all C++ reference tests are #[ignore], no synthetic baselines
- 05-08: VIS-02 changed from [x] Complete to [ ] Partial -- infrastructure built but C++ reference data capture is outside Rust codebase scope
- [Phase 05]: MeshRegistry.loaded lookup gates mesh rendering; unloaded meshes logged at trace level
- 06-01: Used existing RuntimeWorld.query_sphere for BSP-accelerated mesh lookup (not linear iteration)
- 06-01: Unified gravity/buoyancy with static cnt=0.78 (wave modulation deferred)
- 06-01: Collision search radius uses world_radius * 2 (entity) not world_height (R19-PERF)
- 06-01: PhysicsIO Default has safe non-zero world_radius/world_height to prevent div-by-zero
- 06-02: Block state is movement-locked, mutually exclusive with Attack, equipment guard in input.rs
- 06-02: AnimationState Model B (frame counter, no Instant::now()) for deterministic tests
- 06-02: camera_input_system gated via has_characters() custom run condition for spectator mode
- 06-02: SpriteReq includes clr:u8 (default 0) for Phase 7 multiplayer forward-compatibility
- 06-02: spawn_character() is single source of truth for character entity creation
- 06-02: Dead state permanent (respawn deferred to Phase 7)
- 06-03: WaterConfig owned by CpuRasterizerPlugin; WaterLevel owned by GamePlugin (separate concerns)
- 06-03: render_pipeline_system migrated from Update to PostUpdate with RenderSet::Pipeline label
- 06-03: 3-step resolve split: resolve() -> apply_water_ripple_pass() -> RGBA conversion (preserves resolve_bridge.rs)
- 06-03: C++ RGB cube decomposition bug replicated intentionally for visual fidelity (R19-F07)
- 06-03: Linear torque model: yaw += torque * 45.0 * dt (deliberate simplification)
- 06-03: GamePlugin does NOT add sub-plugins -- main.rs registers all independently
- 07-01: Volume stored as linear amplitude (0.0-1.0), converted to kira::Decibels at play time
- 07-01: Startup system creates all 16 DynamicAudioChannels (not lazy init)
- 07-01: AsciickerAudioPlugin registered BEFORE GamePlugin in main.rs (R17-F214)
- 07-01: PlaySoundEvent drained unconditionally (P7-055) to prevent accumulation
- 07-02: configure_sets from GamePlugin gates RenderSet::Pipeline on Playing (avoids modifying CpuRasterizerPlugin)
- 07-02: RenderSet::WaterTime set added for cross-plugin gating of advance_water_time_system
- 07-02: Escape from any non-Playing/Paused state returns to MainMenu (R19-005 stuck-state fallback)
- 07-02: advance_loading_progress checks BOTH AssemblyState.assembled AND terrain.root.is_some()

### Pending Todos

- [ ] Phase 3 plan 03-03 Task 2: Human visual verification checkpoint (run `cargo run` from engine-port/, verify checkerboard renders, test window resize)
- [ ] Commit uncommitted Msaa::Off fix on Camera2d in `engine-port/src/output/mod.rs` (F005 resolution)
- [ ] Phase 4 REND-10: Release-mode performance benchmark not yet executed (human-verification item from 04-VERIFICATION.md)
- [ ] Commit STATE.md and config.json uncommitted changes

### Blockers/Concerns

- ~~bevy_kira_audio must be 0.25 (not 0.24) for Bevy 0.18 compatibility~~ RESOLVED in 07-01
- bevy_replicon_renet2 must be 0.13 for Bevy 0.18 compatibility (Phase 7, Plan 07-03)

## Session Continuity

Last session: 2026-02-25
Stopped at: Completed 07-02-PLAN.md (GameState FSM, main menu, state-gated gameplay systems). 379 lib tests passing.
Resume file: None
