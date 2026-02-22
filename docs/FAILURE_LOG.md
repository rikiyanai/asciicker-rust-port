# FAILURE LOG

## Issues, Blockers, and Problems

**Last Updated:** 2026-02-21

---

## ACTIVE ISSUES

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F005 | MSAA sample count mismatch: pipeline sample_count=1 vs Bevy default Msaa::Sample4=4 | High | RESOLVED | Set Msaa::Off component on Camera2d entity (Bevy 0.18 per-camera MSAA). Fix in uncommitted output/mod.rs (pending 03-03 Task 2 human checkpoint). |
| F006 | STATE.md drift: claims Phase 4 complete while Phase 3 execution still in progress | Medium | RESOLVED | Corrected STATE.md and ROADMAP.md via maintainer-reliability audit 2026-02-20. Phase 4 correctly marked complete (commit `0cdfc24`); Phase 3 correctly marked in-progress (03-03 Task 2 pending). |
| F007 | ROADMAP doc drift: Phase 3 marked 2/3 In Progress but actually 3/3 Complete | Medium | RESOLVED | Fixed ROADMAP.md during Phase 1-4 risk audit 2026-02-20. |
| F008 | System ordering framework undefined: no SystemSet for pipeline stage ordering | High | MONITORING | Mitigated by architecture: Phase 5 Plan 05 uses monolithic `render_pipeline_system` (stages are function calls within one system, not separate systems). R01 re-emerges only if stages are split into separate systems. See RISK-ASSESSMENT.md R01. |
| F009 | Coordinate conversion dead code: game_to_bevy() never called in codebase | High | MONITORING | Architecturally not needed: CPU rasterizer works entirely in game-space (Z-up) via custom `view_tm` matrix. Bevy's 3D renderer is never used for scene geometry. `game_to_bevy()` is dead code but harmless. Phase 3.1 AUDIT-02 (GameVec3 newtype) makes coordinate spaces explicit at compile time. See RISK-ASSESSMENT.md R02. |
| F010 | TextureView use-after-free: local views stored in persisted BindGroup | Medium | PARTIAL | Phase 3.1 AUDIT-01 planned to fix. NOTE: 03-03 Task 1 (resize handler, commit `0dfe33d`) adds a second TextureView creation path in the resize branch — AUDIT-01 executor must fix BOTH the original prepare path AND the resize-triggered BindGroup recreation path (~5 extra LOC). See RISK-ASSESSMENT.md R04. |
| F011 | BSP traversal order: no near-child-first ordering for overdraw reduction | Medium | MONITORING | Phase 5 Plan 02 (replan) adds `camera_pos` parameter and near-child-first ordering with `test_near_child_first_ordering`. See RISK-ASSESSMENT.md R17. Pending execution. |
| F012 | Frame budget: no profiling infrastructure or degradation strategy | Medium | MONITORING | Phase 5 Plans 05+06 (replan) add `PipelineTiming` resource with per-stage `Instant::now()` timing, escape hatches documented, and `test_pipeline_budget_240x135` asserting < 12ms. See RISK-ASSESSMENT.md R18. Pending execution. |
| F013 | Phase 7: bevy_kira_audio 0.25 unverified for Bevy 0.18 | High | OPEN | Must verify on crates.io before Phase 7 Plan 01 starts. See RISK-ASSESSMENT.md R20. |
| F014 | Phase 7: Character entity dependency blocks networking test | High | OPEN | Phase 6 must ship before Phase 7 Plan 03 Task 2. See RISK-ASSESSMENT.md R22. |
| F015 | Phase 7: Combined subsystem load may exceed frame budget (~6.5ms new) | High | OPEN | Need per-subsystem timing + budget targets. See RISK-ASSESSMENT.md R25. |
| F016 | Phase 7: RESOLVE API change risks golden-file regression | Medium | OPEN | Shape-vector parameter addition must preserve None path. See RISK-ASSESSMENT.md R23. |
| F017 | Phase 7: Loading FSM stage countdown not wired to asset loading | Medium | OPEN | Game hangs in Loading state without decrement logic. See RISK-ASSESSMENT.md R27. |
| F018 | Phase 4: No golden-file snapshot tests — SC claims C++ matching but no reference data in repo | Critical | PARTIAL | Phase 5 Plan 06 builds comparison infrastructure. C++ dump utility still needed for full closure. See R35. |
| F019 | Phase 4: RGB555 quantization only 5 of 32768 values tested — SC-4 claims full coverage | Critical | MONITORING | Phase 3.1 Task 3 adds exhaustive range validation (all 32768). Full C++ comparison deferred. See R36. |
| F020 | Phase 4: auto_mat LUT not validated against C++ engine output | Critical | MONITORING | Phase 3.1 Task 3 adds full-table consistency check. C++ comparison deferred. See R37. |
| F021 | Phase 4: Performance benchmark never run in release mode | High | MONITORING | Phase 5 Plan 06 budget assertion test validates < 12ms at 240x135. See R38. |
| F022 | Phase 4: "<1% cell difference" metric defined but never measured | High | MONITORING | Phase 5 Plan 06 golden-file threshold test. See R39. |
| F023 | Phase 5: SampleBuffer has no resize(); window resize causes dimension desync with AsciiCellGrid | Critical | PARTIAL | Plan 05-05 adds ensure_buffer_size() check at frame start. See R44. |
| F024 | Phase 5: Circular dep — Plan 05-05 calls update_terrain_dark from Plan 05-06 (same wave) | High | PARTIAL | Plan 05-05 defers shadow to 05-06's own system. See R45. |
| F025 | Phase 5: AkmMesh loading path undefined — world stage renders zero meshes | High | PARTIAL | Plan 05-05 adds MeshRegistry resource. See R46. |
| F026 | Phase 5: resolve() not extensible for Phase 7 shape-vector | High | PARTIAL | Plan 05-04 adds GlyphSelector trait. See R47. |
| F027 | Phase 6: No camera-follows-player system — camera stays at origin | High | PARTIAL | Plan 06-03 adds sync_camera_to_player. See R54. |
| F028 | Phase 6: Camera rotation frame-rate-dependent (45 deg/frame, not deg/sec) | High | PARTIAL | Plan 06-03 multiplies by time.delta_secs(). See R55. |
| F029 | Phase 6: PhysicsIO.yaw never written — WASD forces in world-space not camera-space | High | PARTIAL | Plan 06-03 adds camera-relative WASD rotation. See R57. |
| F030 | Phase 7: Shape-vector cache unbounded HashMap grows without limit | High | PARTIAL | Plan 07-04 uses LRU cache (8192 cap). See R61. |

### Round 13 Audit Findings (2026-02-21)

87 findings across 5 audit scopes. All CRITICAL/HIGH fixes applied to plan files.

**Phase 5 — Pipeline Integration (24 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F031 | 05-04/05-06: AsciiCellGrid::new() constructor not assigned to any plan's files_modified | High | RESOLVED | Added ascii_cell_grid.rs to 05-04 files_modified with pub fn new(w,h) mandate. |
| F032 | 05-01: HEIGHT_CELLS_PLUS_ONE not pub — compile error in patch_runtime.rs | Medium | RESOLVED | Plan updated: add pub const to constants.rs. |
| F033 | 05-02: WorldInstance.tm Vec<f64> length not validated before [f64;16] copy | Medium | RESOLVED | Plan updated: add try_into() or assert_eq!(tm.len(), 16). |
| F034 | 05-02: query_sphere lacks BSP traversal implementation guidance | Medium | RESOLVED | Plan updated: added BSP-accelerated sphere query traversal algorithm. |
| F035 | 05-03: ButtonInput<KeyCode> may have changed in Bevy 0.18 | High | RESOLVED | Plan updated: added Day-1 verification step. |
| F036 | 05-03: Frustum plane extraction has no concrete perspective-mode algorithm | High | RESOLVED | Plan updated: added 4-step perspective frustum derivation sketch. |
| F037 | 05-03: camera_update_system signature not specified | Low | RESOLVED | Plan updated: added explicit system signature. |
| F038 | 05-04: transform_vertex sharing strategy between terrain/mesh shaders unresolved | High | RESOLVED | Plan updated: definitively picks option (b) extract to render/math.rs. |
| F039 | 05-04: render/math.rs missing from files_modified | High | RESOLVED | Plan updated: added render/math.rs to files_modified. |
| F040 | 05-05: ensure_buffer_size code block placed in wrong task section (Task 1 vs Task 2) | Critical | RESOLVED | Plan updated: moved code block to Task 2, Task 1 has reference note only. |
| F041 | 05-05: Window resize sync approach not definitively chosen | High | RESOLVED | Plan updated: option (a) chosen — sync in pipeline system. |
| F042 | 05-05: Missing Res<AssetServer> in a3d_assembly_system — AKM meshes never load | Critical | RESOLVED | Plan updated: added asset_server: Res<AssetServer> to system signature. |
| F043 | 05-05: bevy-inspector-egui and bevy_mod_debugdump already in Cargo.toml | Medium | RESOLVED | Plan updated: Task 3 verifies compatibility rather than re-adding. |
| F044 | 05-06: update_terrain_dark BSP shadow borrow resolution pattern not specified | High | RESOLVED | Plan updated: specified flatten-then-write-back pattern for recursive tree. |
| F045 | 05-06: compare_ansi_grids in integration test, not reusable | Medium | RESOLVED | Plan updated: noted as test-only, acceptable. |
| F046 | 05-05: Excessive FIX note layering (6+ overlapping notes per topic) | Low | RESOLVED | Plan updated: consolidated into AUTHORITATIVE blocks where possible. |
| F047 | 05-05: MaterialTable conversion must show Material construction including mode: 0 | Medium | RESOLVED | Plan updated: explicit Material { shade, mode: 0 } construction shown. |
| F048 | 05-01: TerrainPlugin stub doesn't register RuntimeTerrain | Low | RESOLVED | Already addressed in plan XP-114 FIX. |
| F049 | 05-02: WorldPlugin stub doesn't register RuntimeWorld | Low | RESOLVED | Already addressed in plan XP-114 FIX. |
| F050 | 05-03: Camera pos [f32;3] vs view_tm [f64;16] boundary documented | Medium | RESOLVED | Already addressed in plan P5-126 FIX. |
| F051 | 05-04: GlyphSelector generic vs dyn resolved to generic form | Medium | RESOLVED | Already addressed in plan P5-306 FIX. |
| F052 | 05-05: a3d_assembly_system Commands + Option<Res<>> pattern correct | Medium | RESOLVED | Verified correct design. |
| F053 | 05-05: add_systems chain ordering correct per Bevy 0.18 | Medium | RESOLVED | Verified correct per R5-015. |
| F054 | 05-06: HEIGHT_SCALE light_dir Z scaling matches C++ pattern | Low | RESOLVED | Verified correct. |

**Phase 6 — Physics and Character (26 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F055 | 06-01: world_height=7.0 is WRONG — should be formula (~86.2 for Human) with HEIGHT_SCALE | Critical | RESOLVED | Plan updated: replaced with full C++ formula. |
| F056 | 06-01: world_radius=2.0 is WRONG for Human — should be ~1.333 (2/12*8) | Critical | RESOLVED | Plan updated: replaced with full C++ formula. |
| F057 | 06-03: Perlin noise id computation uses simple clamp instead of C++ wrap logic | Critical | RESOLVED | Plan updated: replaced clamp with C++ wrap pattern (id<-1→2, id>1→-2). |
| F058 | 06-01: SoupItem missing material: i32 field — forces Phase 7 breaking change | High | RESOLVED | Plan updated: added pub material: i32 to SoupItem. |
| F059 | 06-01: CollisionResult::Hit needs explicit contact: [f32;3] type definition | High | RESOLVED | Plan updated: explicit enum with Hit{toi: f32, contact: [f32;3]}. |
| F060 | 06-01/06-02: C++ static const world_height is a mount-insensitive bug — must document | High | RESOLVED | Plan updated: added note documenting intentional fix. |
| F061 | 06-02: Input C++ line reference (5721-5781) points to wrong code section | High | RESOLVED | Plan updated: marked as needing verification at execution time. |
| F062 | 06-03: sync_mount_to_physics propagates wrong values from collision_dimensions() | High | RESOLVED | Fixed by F055/F056 (correct formulas in collision_dimensions). |
| F063 | 06-03: system_sets.rs not in files_modified for any Phase 6 plan | High | RESOLVED | Plan updated: added system_sets.rs to 06-02 files_modified. |
| F064 | 06-03: C++ Perlin cb decomposition has bug (cr*6 should be cg*6) | Low | RESOLVED | Plan updated: corrected decomposition formula in Rust. |
| F065 | 06-01: PatchCollect uses HEIGHT_CELLS+1 vertices (5x5), plan says HEIGHT_CELLS | Medium | RESOLVED | Plan updated: pseudocode corrected to 5x5 grid with 32 triangles. |
| F066 | 06-01: Vertex stepping factor sxy=VISUAL_CELLS/HEIGHT_CELLS not in pseudocode | Medium | RESOLVED | Plan updated: added sxy=2.0 stepping in vertex computation. |
| F067 | 06-01: PhysicsIO field additions (world_radius, world_height, vel_z) not in C++ | Medium | RESOLVED | Plan updated: documented as intentional Bevy Resource pattern deviation. |
| F068 | 06-01: Phase 5 RuntimePatch API names are assumptions | Medium | RESOLVED | Plan updated: noted verify-at-execution-time. |
| F069 | 06-02: GetSprite C++ line reference wrong (3531-3662 is sprite loading) | Medium | RESOLVED | Plan updated: marked as needing verification at execution time. |
| F070 | 06-02: SetAction* C++ line reference wrong (4853-4998 is DropItem) | Medium | RESOLVED | Plan updated: marked as needing verification at execution time. |
| F071 | 06-02: spawn_character_world Required Components claim needs Bevy 0.18 verification | Medium | RESOLVED | Plan updated: added Bevy 0.18 verification step. |
| F072 | 06-02: SpriteRenderEntry field layout assumed (Phase 5 not yet executed) | Medium | RESOLVED | Plan updated: noted verify-at-execution-time against Phase 5 actual. |
| F073 | 06-02: WASD rotation formula sign convention unverified against C++ | Medium | RESOLVED | Plan updated: added test with known yaw angles for validation. |
| F074 | 06-03: Water reflection matrix math is pseudocode only | Medium | RESOLVED | Acceptable — pseudocode refined at execution time with Phase 5 matrix format. |
| F075 | 06-03: Plugin order has CpuRasterizer before AsciiOutput (pre-existing bug) | Low | RESOLVED | Already tracked in XP-207 FIX — fix applied during Phase 5/6 execution. |
| F076 | 06-02: Block state added as Rust-only extension (no C++ equivalent) | Low | RESOLVED | Correctly documented as intentional. |
| F077 | 06-02: game.h line count off by 8 (575 not 567) | Low | RESOLVED | Plan updated: corrected to 575. |
| F078 | 06-01: accumulate_forces read-only io parameter is correct design | Low | RESOLVED | Verified correct separation. |
| F079 | 06-01: CollisionResult enum correctly eliminates 2.0 sentinel | Low | RESOLVED | Verified correct. |
| F080 | 06-01: Option<Res<>> for RuntimeTerrain/RuntimeWorld is defensive but acceptable | Medium | RESOLVED | Verified acceptable defensive coding. |

**Phase 7 — Game Systems (29 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F081 | 07-01: bevy_kira_audio 0.25 existence unverified for Bevy 0.18 | Medium | RESOLVED | Plan updated: added explicit fallback (bevy_seedling or kira backend). |
| F082 | 07-01: Volume::Amplitude API not verified | Low | RESOLVED | Plan updated: verify at execution time. |
| F083 | 07-01: No test for concurrent 16-track playback | Low | RESOLVED | Plan updated: added 16-channel integration test. |
| F084 | 07-01: #[derive(Message)] import path unverified for Bevy 0.18 | Low | RESOLVED | Plan updated: verify with cargo doc. |
| F085 | 07-02: #[derive(States)] import path unverified for Bevy 0.18 | High | RESOLVED | Plan updated: added Day-1 verification step. |
| F086 | 07-02: LoadingProgress bridge to Phase 5 AssemblyState is speculative | Medium | RESOLVED | Plan updated: noted forward dependency verification. |
| F087 | 07-02: NextState API may differ in Bevy 0.18 | Medium | RESOLVED | Plan updated: added API verification step. |
| F088 | 07-02: main.rs plugin ordering issue (CpuRasterizer before AsciiOutput) | Medium | RESOLVED | Plan updated: verify and fix plugin order at execution time. |
| F089 | 07-03: bevy_replicon 0.38 may not exist for Bevy 0.18 | Critical | RESOLVED | Plan updated: added fallback (raw renet2 if needed). |
| F090 | 07-03: Command::apply() signature may differ in Bevy 0.18 | Critical | RESOLVED | Plan updated: added Day-1 trait verification. |
| F091 | 07-03: C++ has 10+ message types, plan implements only 4 | High | RESOLVED | Plan updated: documented combat messages as explicit OUT_OF_SCOPE. |
| F092 | 07-03: spawn_character_world() doesn't exist in Phase 6 plans | High | RESOLVED | Plan updated: 07-03 creates variant itself if 06-02 lacks it. |
| F093 | 07-03: Replication component import path unverified | Medium | RESOLVED | Plan updated: verify at execution time. |
| F094 | 07-03: In-process networking test fragile (#[ignore]) | High | RESOLVED | Plan updated: added mock transport fallback for CI. |
| F095 | 07-03: StatesPlugin import path for MinimalPlugins test | Medium | RESOLVED | Plan updated: verify at execution time. |
| F096 | 07-04: kiddo 5.2 API significantly different from description | High | RESOLVED | Plan updated: added dual kiddo 4.x/5.x code paths as fallback. |
| F097 | 07-04: nearest.item return type from kiddo unverified | Medium | RESOLVED | Plan updated: verify with cargo doc at execution time. |
| F098 | 07-04: Font1 recolor tables are approximate, not exact C++ values | Medium | RESOLVED | Plan updated: replaced with exact C++ recolor byte sequences. |
| F099 | 07-04: ShapeVectorGlyphSelector lifetime/borrow ordering complex | Medium | RESOLVED | Plan updated: added explicit borrow sequence in pipeline system. |
| F100 | 07-04: six-samples.json location verification needed | Low | RESOLVED | Plan updated: verify file exists at execution time. |
| F101 | 07-05: Snow glyphs notation ambiguous (trailing comma looks like separator) | Medium | RESOLVED | Plan updated: explicit CP437 codes [0x2A, 0x2B, 0x2E, 0x2C]. |
| F102 | 07-05: Rain glyphs invented (not from C++) | Medium | RESOLVED | Plan updated: documented rain as new feature, not C++ port. |
| F103 | 07-05: weather_composite_system schedule ordering ambiguous | Critical | RESOLVED | Plan updated: added STOP-AND-CHECK step to verify schedule before registration. |
| F104 | 07-05: noise crate version may conflict with Phase 6 | Low | RESOLVED | Already handled by P7-115 FIX check. |
| F105 | 07-05: No rain-specific glyph test | Low | RESOLVED | Plan updated: added test_rain_uses_rain_glyphs. |
| F106 | All P7: Bevy 0.18 APIs unverified systemically across 5 plans | High | RESOLVED | Plan updated: added "Day 0" verification step at Phase 7 start. |
| F107 | All P7: Cargo.toml sequential chain fragile (07-01→03→04→05) | Medium | RESOLVED | Plan updated: cargo check between plans. |
| F108 | All P7: R25 (combined performance budget) not addressed by any plan | Low | RESOLVED | Plan updated: added Phase 7 integration performance test note. |
| F109 | All P7: R26 (per-subsystem profiling) not addressed | Low | RESOLVED | Plan updated: added per-subsystem timing note. |

**Cross-Phase Contracts (4 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F110 | 06-03 vs 07-02: GamePlugin ownership conflict — 07-02 may overwrite 06-03's GamePlugin | High | RESOLVED | Plan 07-02 updated: explicitly extends existing GamePlugin, preserves Phase 6 systems. |
| F111 | 05-03 vs 06-02: Camera input system no-op fragility — should be removed/gated not silenced | Medium | RESOLVED | Plan 06-02 updated: gates with run_if(not(any_with_component::<Character>)). |
| F112 | 07-01: AsciickerAudioPlugin ordering ambiguity (P7-121 vs P7-055 contradiction) | Medium | RESOLVED | Plan 07-01 updated: unconditionally drain events, downgraded P7-121 to post-hoc. |
| F113 | 05-05→06-03→07-02→07-04: render_pipeline_system accumulated signature not documented | Medium | RESOLVED | Plan 07-05 updated: added final accumulated signature comment. |

**Completed Phases vs Code (4 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F114 | Codebase: AsciiCellGrid::new(w,h) constructor missing — Phase 5 tests need it | Critical | RESOLVED | Same as F031. Plan 05-04 updated. |
| F115 | 06-03: AnsiCell.flags field doesn't exist — correct field is spare | Critical | RESOLVED | Plan 06-03 updated: all references changed from flags to spare. |
| F116 | Codebase: test_pattern_system overwrites pipeline output (no feature gate) | Medium | RESOLVED | Tracked — Phase 5 Plan 05 adds #[cfg(feature = "test_pattern")] gate. |
| F117 | 05-05: Debug deps (bevy-inspector-egui, bevy_mod_debugdump) already in Cargo.toml | Medium | RESOLVED | Same as F043. Plan 05-05 Task 3 updated to verify not re-add. |

### Round 14 Findings (2026-02-21)

**Standard Scope — Phase 7 (3 actionable)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F118 | 07-05: R13-035 pipeline signature missing `sprite_queue: Res<SpriteQueue>` and `time: Res<Time>` params | High | RESOLVED | Pipeline signature updated with both params in 07-05-PLAN.md |
| F119 | 07-02: main.rs plugin ordering needs verification step (AsciiOutputPlugin before CpuRasterizerPlugin) | Medium | RESOLVED | Verification step 9 added to 07-02-PLAN.md |
| F120 | 07-04: Font1 recolor trailing `0,0` is C++ terminator sentinel, not a 4th color pair — needs clarification | Medium | RESOLVED | R14-M03 FIX added to 07-04-PLAN.md |

**Standard Scope — Phase 6 (1 actionable)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F121 | 06-03: Benchmark uses `engine_port::` crate path but Cargo.toml name is `asciicker-engine` (= `asciicker_engine`) | Low | RESOLVED | R14-L01 FIX added to 06-03-PLAN.md |

**Standard Scope — Cross-Phase (2 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F122 | 07-05: R13-035 accumulated pipeline signature omits `ResMut<SpriteQueue>` (required by sprite blit stage) | Low | RESOLVED | Fixed with F118 — SpriteQueue added to pipeline sig |
| F123 | 07-05: R13-035 includes `Res<WaterLevel>` but pipeline reads `Res<WaterConfig>` — WaterLevel is game-domain | Low | RESOLVED | WaterLevel removed from sig, comment added explaining WaterConfig path |

**Implementation Sequence (17 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F124 | 05-05: No plan explicitly owns main.rs plugin reorder (AsciiOutputPlugin before CpuRasterizerPlugin) | Critical | RESOLVED | R14-F124 FIX added as Step 0 in 05-05-PLAN.md |
| F125 | 07-02: depends_on:[] is misleading — pipeline.rs 4-way ownership requires 06-03 to complete first | Critical | RESOLVED | Changed to depends_on: ["06-03"] in 07-02-PLAN.md |
| F126 | 06-03: Update→PostUpdate schedule migration lacks verification step for stale references | Critical | RESOLVED | R14-F126 FIX grep verification added to 06-03-PLAN.md |
| F127 | Phase 7 wave 1: depends_on:[] with comment-only dependencies confuses automated executors | High | RESOLVED | R14-F127 FIX phase prerequisite comments added to Phase 7 plans |
| F128 | 05-01/05-02: Stubs must register resources via init_resource before assembly consumes them | Medium | MONITORING | Informational — plans already specify init_resource pattern |
| F129 | 06-02: Disabling Phase 5 camera Q/E handling may break Phase 5 tests | High | MONITORING | Informational — Phase 5 tests should be camera-independent |
| F130 | 05-04/05-05: MaterialTable Resource ownership gap — both plans claim creation | High | RESOLVED | R14-F130 FIX clarification added to both plans |
| F131 | Phase 5 Wave 1: No integration check before Wave 2 consumes Wave 1 APIs | Medium | OPEN | Add cargo build verification at wave boundaries |
| F132 | 07-02: GameState testing requires functional render pipeline (bootstrap concern) | Medium | OPEN | Already addressed by Option (a)/(b) dual path |
| F133 | Phase 7 wave labels imply parallelism that doesn't exist (sequential required) | Medium | OPEN | Informational — wave labels are misleading |
| F134 | 05-05: test_pattern_system conflicts with render_pipeline_system (both write AsciiCellGrid) | Medium | OPEN | Already tracked (F116). Feature gate resolves this. |
| F135 | Critical path is 10 sequential steps (informational) | Low | OPEN | Informational — reasonable for scope |
| F136 | Phase 6 depends on Phase 5 APIs that don't exist yet (bootstrap managed by depends_on) | High | OPEN | Informational — correctly handled by depends_on |
| F137 | Multi-file plans have no explicit rollback strategy | Medium | OPEN | Informational — git checkout . is implicit |
| F138 | Phase 7 plans reference external GSD workflow files | Low | OPEN | Informational — execution framework concern |
| F139 | SpriteQueue lifecycle split across Phase 5 and 6 (ownership transition) | Medium | OPEN | Correctly documented, no fix needed |
| F140 | 06-02: game/mod.rs 3-way ownership (06-03→07-02→07-05) | High | OPEN | Correctly sequenced by depends_on |

**UX and Complexity (grouped, 8 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F141 | Plans are ~55% FIX-note audit noise (6,319 lines total, ~3,490 are annotations) | Critical | DEFERRED | Restructuring into SPEC+APPENDIX format deferred to post-execution |
| F142 | 9 contradictions: old wrong text coexists with corrections (struck-through but readable) | High | DEFERRED | Would require major plan restructuring — deferred |
| F143 | 7 avoidable ambiguities dumped onto executor (AnimState model, geometry source design, etc.) | High | RESOLVED | 4 key decisions resolved: AnimState Model B, free functions, patch scale, channel tests |
| F144 | 8 critical code sections in prose not Rust (frustum planes, collision sweep, water reflection) | High | DEFERRED | Prose-to-code conversion deferred to execution time |
| F145 | GlyphSelector trait in Phase 5 is premature (YAGNI — only needed in Phase 7) | Medium | DEFERRED | Simplification deferred — trait exists for Phase 7 extensibility |
| F146 | PhysicsGeometrySource trait vs free functions — plan admits free functions would be simpler | Medium | DEFERRED | Noted — implementer may use free functions per P6-012 |
| F147 | SpawnNetworkedPlayer Command struct is over-engineering for single call site | Medium | DEFERRED | Noted — simplify during Phase 7 execution if appropriate |
| F148 | ResolveBuffer pre-allocation (R62) added without profiling evidence | Medium | DEFERRED | Low-risk optimization — keep but profile during execution |

**Process Gaps (12 findings)**

| ID | Issue | Severity | Status | Resolution |
|----|-------|----------|--------|------------|
| F149 | PG-01: No plan includes ROADMAP.md update step after completion | High | RESOLVED | R14-PG01 FIX added to all 14 plan files |
| F150 | PG-06: No full regression test (cargo test --all) between phases | High | RESOLVED | R14-PG06 FIX added to phase boundary plans (05-06, 06-03, 07-05) |
| F151 | PG-10: No visual verification checkpoint after Phase 5 (SC requires visual recognition) | Critical | RESOLVED | R14-PG10 FIX visual checkpoint added to 05-06-PLAN.md |
| F152 | PG-13: No .a3d world file in engine-port/assets/ — Phase 5 can't render anything | Critical | RESOLVED | R14-PG13 FIX asset prerequisite added to 05-05-PLAN.md |
| F153 | PG-16: Golden-file C++ reference data doesn't exist — synthetic baselines were accepted | Critical | BLOCKER | R14-SYNTH-BAN: Synthetic baselines BANNED. Real C++ reference data is a hard blocker. Must extract from C++ engine before golden-file tests can pass. Previous R14-PG16 FIX (accept synthetic) SUPERSEDED. |
| F154 | PG-17: No CI/CD pipeline exists (.github/workflows missing) | High | RESOLVED | CI workflow created at .github/workflows/ci.yml |
| F155 | PG-15: No .xp sprite files for Phase 6 character/equipment rendering | High | RESOLVED | R14-PG15 FIX asset prerequisite added to 06-02-PLAN.md |
| F156 | PG-11/12: No visual verification or playtest checkpoint after Phase 6 or Phase 7 | High | RESOLVED | R14-PG11/PG12 FIX playtest checklists added to 06-03 and 07-05 |
| F157 | PG-05: cargo fmt/clippy not mandated project-wide at wave boundaries | Medium | RESOLVED | R14-PG05 FIX added to wave-final plans |
| F158 | PG-22: Plugin stub conversion not explicitly tracked in plans | Medium | OPEN | Add stub cleanup verification step |
| F159 | PG-23: render_pipeline_system in Update during Phase 6 wave 1 (before 06-03 migrates to PostUpdate) | Medium | OPEN | Accept intermediate state — documented behavior |
| F160 | PG-14/21: Sound files not available; bevy_replicon fallback not fully documented | Medium | OPEN | Add asset setup and dependency fallback notes |
| F161 | R15: 07-05 R13-035 accumulated pipeline signature includes removed ResolveBuffer Resource | High | RESOLVED | Replaced `ResMut<ResolveBuffer>` with comment re: local alloc per R14-F148. |
| F162 | R15: 06-03 SPEC uses `resolve_buf.0` syntax assuming removed ResolveBuffer newtype | Medium | RESOLVED | Changed `&mut resolve_buf.0` to `&mut resolve_buf` in 06-03 apply_water_ripple_pass call. |
| F163 | R15: 06-03 precondition text lists ResolveBuffer as Phase 5 abstraction (removed by R14-F148) | Low | RESOLVED | Removed ResolveBuffer from 06-03 precondition text. |
| F164 | R15: 06-02 depends_on missing "06-01" — PhysicsIO type dependency causes compile failure if parallel | High | RESOLVED | Added `"06-01"` to 06-02 depends_on. |
| F165 | R15: 07-03 calls spawn_character_world(&mut commands) but fn takes &mut World — should call spawn_character | Medium | RESOLVED | Changed to spawn_character(&mut commands) throughout 07-03. |
| F166 | R15: ROADMAP.md Phase 6 plan 06-01 description says "TerrainGeometrySource + WorldGeometrySource" (stale trait names) | Low | RESOLVED | Updated to reference free functions per R14-F146. |
| F167 | R15: 06-03 render_water_reflections body is comment-only pseudocode — no actual Rust code | High | RESOLVED | Replaced comment body with actual Z-flip matrix code and spare-bit OR pattern. |
| F168 | R15: 05-01 has struck-through P5-H07 text inline (no APPENDIX to move it to) | Medium | RESOLVED | Deleted struck-through line. |
| F169 | R15: 07-02 has struck-through legacy text inside R7-HIGH-003 FIX block | Low | RESOLVED | Removed struck-through text. |
| F170 | R15: 07-03 has 5 struck-through SUPERSEDED fixes (ECS-FIX-03, C1, C2, R13-025, R13-027) | Low | RESOLVED | Removed all five lines. |
| F171 | R15: 07-04 P7-025/P7-057/P7-104 supersession chain is 18 lines of tangled commentary | Medium | RESOLVED | Condensed to 3-line blockquote with P7-057 AUTHORITATIVE label. |
| F172 | R15: 05-05 Task 2.2 uses placeholder `/* GlyphSelector impl */` instead of explicit AutoMatGlyphSelector | Low | RESOLVED | Replaced with `AutoMatGlyphSelector::new()`. |
| F173 | R15: 05-06 says cells_count() is "not a confirmed method" but it exists at ascii_cell_grid.rs:45 | Low | RESOLVED | Updated comment to acknowledge cells_count() exists. |
| F174 | R15: 07-03 test description references stale SpawnNetworkedPlayer in ECS integration test comment | Low | RESOLVED | Updated to reference direct spawn_character() call. |

---

## RESOLVED ISSUES

| ID | Issue | Severity | Resolution | Date |
|----|-------|----------|------------|------|
| F001 | Initial assumption: OOP codebase | Medium | Corrected: DOD (Data-Oriented Design) | 2026-02-19 |
| F002 | Initial assumption: Perspective not needed | High | Corrected: Must implement perspective for Q/E rotation | 2026-02-19 |
| F003 | .xp terrain format doesn't exist | Medium | Corrected: Terrain uses .a3d format | 2026-02-20 |
| F004 | Terrain.cpp bugs identified | High | Planned: Document in Rust, fix in port | 2026-02-19 |
| F005 | MSAA sample count mismatch | High | Set Msaa::Off on Camera2d (uncommitted, pending 03-03 Task 2) | 2026-02-20 |
| F006 | STATE.md drift: Phase 4 vs Phase 3 status | Medium | Corrected via maintainer-reliability audit | 2026-02-20 |
| F007 | ROADMAP doc drift: Phase 3 status | Medium | Fixed ROADMAP.md (03-03 -> [x], 3/3 Complete) | 2026-02-20 |

---

## KNOWN BUGS IN C++ (To Document in Rust)

| Bug ID | File | Line | Description | Fix Strategy |
|--------|------|------|-------------|-------------|
| TERRAIN-001 | terrain.cpp | 613 | `if(x)` twice | Document + Rust validation |
| TERRAIN-002 | terrain.cpp | 805 | `u < y` wrong scope | Document + Rust validation |
| TERRAIN-003 | terrain.cpp | 1671 | Same as TERRAIN-002 | Document + Rust validation |
| TERRAIN-004 | terrain.cpp | 480,492 | `>` vs `>=` inconsistency | Document + choose consistent |
| AUDIO-001 | audio.cpp | 704 | No sample unload (memory leak) | Rust Drop trait |
| AUDIO-002 | audio.cpp | 553 | Silent failure on decode | Add logging |

---

## ASSUMPTION FAILURES

| Assumption | Expected | Actual | Impact | Resolution |
|------------|----------|--------|--------|------------|
| OOP architecture | OOP | DOD | Low | Different port approach |
| Perspective optional | Isometric OK | Required | High | Must implement |
| Mage-core complete | Full engine | Rendering lib only | Medium | Use Bevy instead |
| 89 unknown unknowns | All real | Some gaps | Low | Categorized properly |

---

## BLOCKERS REMOVED

| Blocker | Removed Date | Notes |
|---------|--------------|-------|
| Missing perspective math | 2026-02-19 | Documented in audit-unknown-perspective-matrix.md (note: exact matrix values still unresolved — RE-AUDIT R-002) |
| Missing animation timing | 2026-02-20 | Documented in codedoc-animation-timing.md |
| Missing physics constants | 2026-02-20 | Constants documented in ASSUMPTION_MASTER_CHECKLIST.md; codedoc-physics-constants.md exists at project root |
| Missing A3D key codes | 2026-02-20 | Documented in codedoc-a3d-keycodes.md |
| Missing .a3d format | 2026-02-20 | Documented in audit-unknown-a3d-format.md |

---

## CURRENT READINESS

| Area | Status | Notes |
|------|--------|-------|
| Research | In Progress | 47% (42/89) unknowns resolved per RE-AUDIT-MASTER.md |
| Assumptions | Complete | All verified |
| Gaps | Complete | All planned |
| Implementation | In Progress | Phases 1-4 complete, Phase 3 (03-03 Task 2 visual checkpoint pending). Phase 3.1 planned. Phase 5-6 replanned. |
| Risk Assessment | Complete | 62 risks identified across all phases. R01-R19 (Phases 1-4 audit), R20-R34 (Phase 7), R35-R43 (Phase 4 execution), R44-R62 (cross-phase architecture audit). |

---

## NOTES

- Phase 3 complete (visual verification passed, 03-03 done)
- Phase 4 performance benchmark needs release-mode execution (04-VERIFICATION.md)
- Phases 5-6 replanned: all 11 audit gaps resolved (AUDIT #7, #10, #11 plus original critical/high gaps)
- Phase 3.1 (Audit Remediation) inserted: 1 plan addressing AUDIT-01 through AUDIT-05 (code-level fixes)
- Phase 3.1 gap: AUDIT-01 must also cover resize path in gpu_plugin.rs (03-03 Task 1 already committed)
- Recommended execution order: Phase 3.1 → 03-03 Task 2 (visual checkpoint) → Phase 5 → Phase 6
- Phase 4 execution audit: 11 gaps found (3 critical: golden-file, quantization, auto_mat), see R35-R43
- Cross-phase architecture audit: 19 new risks (R44-R62), including 1 critical (SampleBuffer resize), 9 high
- Full risk assessment: .planning/RISK-ASSESSMENT.md (62 risks across all phases)
- Phase 7 recommend sequential plan execution (02->01->03->04->05) to avoid merge conflicts

---

*Failure log last updated: 2026-02-21 (R15 findings F161-F174 added)*
