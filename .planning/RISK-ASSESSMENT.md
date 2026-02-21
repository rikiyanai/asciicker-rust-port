# Risk Assessment: Phases 1-4 + Phase 7

**Audit date:** 2026-02-20
**Audited by:** Parallel agents (Phase 1, 2, 3, 7 independent audits) + Phase 4 UAT
**Scope:** Architectural gaps, integration risks, and performance concerns across all phases

## Risk Summary

| ID | Phase | Risk | Severity | Status |
|----|-------|------|----------|--------|
| R01 | 1 | System ordering framework undefined (no SystemSet) | CRITICAL | MONITORING - Phase 5 monolithic pipeline mitigates (stages are function calls, not separate systems) |
| R02 | 1 | Coordinate conversion (game_to_bevy) is dead code | HIGH | MONITORING - Not needed: CPU rasterizer uses game-space Z-up via custom view_tm. Phase 3.1 AUDIT-02 adds GameVec3 newtype |
| R03 | 3 | ROADMAP doc drift (Phase 3 marked incomplete) | MEDIUM | FIXED - ROADMAP.md updated |
| R04 | 3 | TextureView use-after-free in BindGroup | MEDIUM | PLANNED - Phase 3.1 AUDIT-01 (must also fix resize path from 03-03 Task 1 commit `0dfe33d`) |
| R05 | 2 | Parse-time structs != Runtime structs (no converter) | HIGH | PLANNED - Phase 5 Plans 01 (RuntimeTerrain) + 02 (RuntimeWorld) define converters |
| R06 | 3 | 1-frame render latency undocumented | MEDIUM | Open - Phase 5 must document |
| R07 | 3 | No integration test for test_pattern -> rasterizer switch | MEDIUM | Open - Phase 5 must add |
| R08 | 1 | GameVec3 type alias provides no type safety | MEDIUM | PLANNED - Phase 3.1 AUDIT-02 replaces with newtype |
| R09 | 1 | Plugin order implicit (AsciiCellGrid depends on RenderConfig) | MEDIUM | PLANNED - Phase 3.1 AUDIT-05 adds integration test |
| R10 | 2 | Integer overflow in sprite dimension multiplication | MEDIUM | PLANNED - Phase 3.1 AUDIT-03 adds checked_mul |
| R11 | 2 | Transform matrix NaN/Inf not validated | MEDIUM | PLANNED - Phase 3.1 AUDIT-03 adds is_finite check |
| R12 | 2 | Sprite swoosh quadrant masking simplified | LOW | Open - verify in Phase 5 |
| R13 | 3 | Font atlas error: silent black screen on missing asset | MEDIUM | PLANNED - Phase 3.1 AUDIT-04 adds warn! logging |
| R14 | 3 | Physical vs logical pixel contract fragile | MEDIUM | Open - add documentation |
| R15 | 3 | GPU uniform buffer recreated every frame | LOW | Open - optimize later |
| R16 | 3 | Glyph index u16->u8 truncation no validation | LOW | PLANNED - Phase 3.1 AUDIT-04 adds debug_assert |
| R17 | 4 | BSP traversal order: no near-child-first ordering | MEDIUM | PLANNED - Phase 5 Plan 02 (replan) adds camera_pos + near-child-first with test |
| R18 | 4 | Frame budget: no profiling, no degradation strategy | MEDIUM-HIGH | PLANNED - Phase 5 Plans 05+06 (replan) add PipelineTiming + budget assertion |
| R19 | 1 | Cross-plugin communication pattern not established | MEDIUM | Open - Phase 5 sets pattern |
| R35 | 4 | No golden-file snapshot tests vs C++ reference | CRITICAL | PARTIAL - Phase 5 Plan 06 builds infra; C++ dump utility needed for full coverage |
| R36 | 4 | RGB555 quantization: 5 spot checks of 32768 values | CRITICAL | PLANNED - Phase 3.1 Task 3 adds exhaustive range validation |
| R37 | 4 | auto_mat LUT not validated against C++ | CRITICAL | PLANNED - Phase 3.1 Task 3 adds consistency check; C++ comparison deferred |
| R38 | 4 | Performance benchmark never run (release mode) | HIGH | PLANNED - Phase 5 Plan 06 budget assertion test |
| R39 | 4 | "<1% cell difference" metric never measured | HIGH | PLANNED - Phase 5 Plan 06 golden-file threshold test |
| R40 | 4 | Dead unsafe unchecked SampleBuffer accessors | MEDIUM | PLANNED - Phase 3.1 Task 3 |
| R41 | 4 | Reflection palette path untested | MEDIUM | PLANNED - Phase 3.1 Task 3 |
| R42 | 4 | Elevation thresholds approximate | MEDIUM | SELF-RESOLVES - Phase 5 tunes with real terrain data |
| R43 | 4 | No SampleBuffer boundary/edge-case tests | LOW | PLANNED - Phase 3.1 Task 3 |

## Critical Path for Phase 5

These MUST be resolved before or during Phase 5 execution:

### R01: System Ordering Framework (CRITICAL)
**Phase 1 gap.** No `SystemSet` enum defined for pipeline stage ordering. When Phase 5 adds 6+ systems (Clear, Terrain, World, Shadow, Reflection, Resolve), they will execute in **arbitrary order** without explicit `.before()` / `.after()` constraints.

**Fix:** Define `RenderSystemSet` enum deriving `SystemSet` in `render/mod.rs`. Register all pipeline systems with ordering constraints. The `PipelineStage` enum already exists but isn't wired to Bevy's scheduling.

**Note:** Phase 5 Plan 04 designs the pipeline as a SINGLE monolithic system (`render_pipeline_system`) which avoids this entirely — the stages are function calls within one system, not separate systems. If this design holds, R01 is mitigated by architecture. But if future phases split stages into separate systems, R01 re-emerges.

### R02: Coordinate Conversion Dead Code (HIGH)
**Phase 1 gap.** `core/coords.rs` defines `game_to_bevy()` and `bevy_to_game()` with 7 passing tests, but **no code in the entire codebase calls these functions**. `GameVec3` type alias is never used.

**Risk:** Phase 5 loads .a3d positions (Z-up C++ space) and must convert to Bevy (Y-up). If developers forget `game_to_bevy()`, all geometry renders rotated 90 degrees.

**Fix:** Phase 5 asset converter module must call `game_to_bevy()` for all loaded positions. Add integration test loading known .a3d position and verifying Bevy-space output.

### R05: Parse-to-Runtime Type Conversion (HIGH)
**Phase 2 gap.** Parsers produce `XpSprite`, `A3dTerrain`, `A3dWorld`, `AkmMesh`. Phase 5 needs `RuntimeTerrain`, `RuntimeWorld`, `RuntimeMesh` with GPU-ready or algorithm-ready layouts.

**Fix:** Phase 5 plans already define `RuntimeTerrain` (Plan 01) and `RuntimeWorld` (Plan 02) with `build_from_parsed()` methods. This gap is **acknowledged and planned**.

### R04: TextureView Use-After-Free (MEDIUM)
**Phase 3 bug.** In `gpu_plugin.rs` Prepare system, `TextureView` objects are created as local variables, stored in a `BindGroup`, then dropped. The BindGroup persists across frames holding references to dropped views.

**Risk:** Currently works because views are recreated every frame and GPU driver doesn't validate. Under Phase 5's high-frequency texture updates, undefined behavior or crash possible.

**Fix:** Store `TextureView` in `AsciiGpuTextures` struct, or refactor to create views with the same lifetime as the BindGroup.

## Medium Risks (Address in Phase 5-6)

### R06: Frame Latency
CPU rasterizer writes AsciiCellGrid in Update. Extract copies to Render World. GPU renders. This is standard Bevy architecture (1-frame latency). Document it. If input lag is noticeable, consider FixedUpdate timing.

### R07: test_pattern Switchover
Phase 5 replaces `test_pattern_system` with rasterizer output. Add integration test verifying rasterizer output renders correctly through GPU pipeline. Keep test_pattern behind feature flag for debugging.

### R10-R11: Parser Robustness
Add `checked_mul` for sprite dimensions and `is_finite()` for transform matrices. Low effort, prevents crashes on corrupted assets.

### R17: BSP Near-Child-First
Add camera-position-based child ordering in BSP traversal. ~5 lines of code. Reduces overdraw, improves frame budget.

### R18: Frame Budget Infrastructure
Add per-stage timing (`Instant::now()` around each pipeline stage). Log to Bevy diagnostics. Set budget target: full pipeline < 12ms at 240x135 (leaving 4ms for Bevy overhead).

## Low Risks (Track, fix when convenient)

- R08: GameVec3 newtype (nice-to-have, prevents coordinate space bugs)
- R09: Plugin order test (add to integration tests)
- R12: Swoosh quadrant masking (verify against C++ in Phase 5)
- R13-R16: GPU pipeline hardening (logging, validation, optimization)
- R15: Uniform buffer optimization (defer to Phase 7)
- R19: Cross-plugin communication pattern (Phase 5 establishes by example)

## Phase 4 Execution Gaps

Post-UAT audit of Phase 4 code against success criteria. UAT passed (8/8) but success criteria claims exceed test evidence.

| ID | Phase | Gap | Severity | Status |
|----|-------|-----|----------|--------|
| R35 | 4 | No golden-file snapshot tests — SC-2/SC-5 claim C++ reference matching but no reference data in repo | CRITICAL | Open - needs C++ reference extraction |
| R36 | 4 | RGB555 quantization only 5 spot checks of 32768 — SC-4 claims full coverage | CRITICAL | Open - needs exhaustive or sampling test |
| R37 | 4 | auto_mat LUT not validated against C++ reference — internal consistency only | CRITICAL | Open - needs C++ LUT dump comparison |
| R38 | 4 | Performance benchmark #[ignore], never run in release mode — SC-5 "60fps+" untested | HIGH | Open - run in Phase 5 with real data |
| R39 | 4 | "<1% cell difference" metric defined but never measured | HIGH | Open - needs golden-file infrastructure |
| R40 | 4 | Dead unsafe unchecked accessors in SampleBuffer | MEDIUM | Open - remove or justify |
| R41 | 4 | Reflection palette path (divisor 400 vs 255) untested | MEDIUM | Open - add unit test |
| R42 | 4 | Elevation thresholds approximate (0.5/2.0/5.0) | MEDIUM | Open - tune in Phase 5 |
| R43 | 4 | No boundary tests for SampleBuffer edge cases | LOW | Open - add robustness tests |

### R35-R37: C++ Reference Validation (CRITICAL)

Phase 4 success criteria promise output "matching C++ reference" but no C++ reference data was extracted or committed. The tests verify internal consistency (e.g., rgb2pal returns indices in valid range) but never compare against actual C++ output.

**Fix:** Before Phase 5 integration:
1. Run C++ engine on canonical test scenes, capture SampleBuffer and AnsiCell output
2. Commit as golden-file fixtures in `engine-port/tests/fixtures/`
3. Add snapshot comparison tests for: RGB555 full-range quantization, auto_mat LUT dump, resolve() output on canonical geometry

### R38-R39: Performance and Accuracy Claims (HIGH)

The `#[ignore]` performance benchmark was never executed in release mode. The "<1% cell difference" metric from SC-5 has no test measuring it. Both become testable once Phase 5 provides real scene data.

**Fix:** Phase 5 Plan 05 or 06 should include golden-file CI comparison with cell-difference measurement.

## Phase-Specific Recommendations

### Before Phase 5 Starts
1. Fix R03 ROADMAP doc drift [DONE]
2. Plan R05 converter module in Phase 5 plans [ALREADY PLANNED]
3. Add R17 near-child-first to Phase 5 Plan 02
4. Add R18 frame timing to Phase 5 Plan 04

### During Phase 5 Execution
5. Fix R04 TextureView lifetime (before wiring rasterizer to GPU)
6. Enforce R02 coordinate conversion in all asset converters
7. Add R07 integration test (rasterizer -> GPU pipeline)
8. Document R06 frame latency in rendering architecture

### Phase 6 Cleanup
9. Fix R10-R11 parser robustness
10. Address R13 font atlas error logging
11. Profile and tune R18 frame budget

---

## Phase 7 Risks

### Risk Summary (Phase 7)

| ID | Category | Risk | Severity | Status |
|----|----------|------|----------|--------|
| R20 | 7-Deps | bevy_kira_audio 0.25 version unverified for Bevy 0.18 | HIGH | Open - verify Day 1 |
| R21 | 7-Deps | bevy_replicon 0.38 + bevy_replicon_renet 0.14 compat unknown | MEDIUM | Open - verify Day 1 |
| R22 | 7-Integration | Phase 6 dependency: Character entities don't exist yet | CRITICAL | Open - Plan 03 blocked |
| R23 | 7-Integration | RESOLVE stage API change for shape-vector breaks golden-files | MEDIUM | Open - regression test needed |
| R24 | 7-Integration | Game state machine vs Phase 6 pause logic conflict | MEDIUM | Open - define scope |
| R25 | 7-Perf | Combined load: audio + network + weather + shape-vector | HIGH | Open - budget target needed |
| R26 | 7-Perf | No per-subsystem profiling infrastructure | MEDIUM | Open - add instrumentation |
| R27 | 7-GameState | Loading FSM stage countdown has no asset loading hooks | MEDIUM | Open - wire decrement logic |
| R28 | 7-Audio | 16-track DynamicAudioChannels concurrent behavior untested | MEDIUM | Open - add concurrent test |
| R29 | 7-Weather | Particle compositing order vs sprite blit ordering undefined | MEDIUM | Open - document order |
| R30 | 7-ShapeVec | 6D cache thrashing in complex scenes (hit rate unknown) | MEDIUM | Open - add telemetry |
| R31 | 7-ShapeVec | six-samples.json alphabet mismatch with CP437 font-1.xp | MEDIUM | Open - generate custom vectors |
| R32 | 7-Font | Font1 recolor table byte ordering (Pitfall 6) | MEDIUM | Open - add golden test |
| R33 | 7-Scope | 5 concurrent Wave 1 plans risk merge conflicts | MEDIUM | Open - recommend sequential |
| R34 | 7-Network | In-process integration test may not be runnable single-threaded | MEDIUM | Open - choose workaround |

### Critical (Phase 7)

#### R22: Phase 6 Dependency — Character Entities Not Yet Ported
Plan 03 (Networking) requires Character entities with Transform and PoseUpdate components from Phase 6. If Phase 6 hasn't shipped, networking cannot test entity replication. **Blocker for Plan 03 Task 2.**

**Fix:** Either ensure Phase 6 ships first, or define stub Character component in Phase 7 for testing. Document as explicit dependency.

#### R20: bevy_kira_audio 0.25 Version Mismatch
Plan 01 pins bevy_kira_audio 0.25 for Bevy 0.18 compatibility. Version not verified at runtime. Legacy skeleton has 0.24 (Bevy 0.17 only).

**Fix:** Query crates.io on Day 1. Fallback: bevy_seedling if 0.25 doesn't exist.

#### R25: Combined Performance Budget
Phase 7 adds ~6.5ms of new CPU work (audio 1ms + network 0.5ms + weather 2ms + shape-vector 3ms) to the existing pipeline. Combined with Phase 5 rendering, this may exceed the 16.7ms frame budget.

**Fix:** Add per-subsystem timing. Set budget: each subsystem < 2ms. Full pipeline < 12ms at 240x135. Feature flags to disable subsystems at runtime if budget exceeded.

### High (Phase 7)

#### R23: RESOLVE Stage API Change
Plan 04 adds optional `ShapeVectorMatcher` parameter to resolve(). All existing calls must be updated. Phase 4/5 golden-file tests must pass with `shape_matcher=None`.

**Fix:** Grep all resolve() call sites. Update to pass None. Run full Phase 4/5 test suite before merging.

#### R27: Loading FSM Missing Wiring
Plan 02 defines Loading stages (3..0 countdown) but doesn't wire asset loading to stage decrement. Game hangs in Loading state.

**Fix:** Wire asset_server.is_loaded() checks to stage decrement. Add unit test verifying transition to Playing.

### Medium (Phase 7)

- R21: bevy_replicon version coupling — verify on Day 1
- R24: Pause scope (rendering only? or physics too?) — coordinate with Phase 6
- R28: 16-track mixer concurrent test — add to Plan 01
- R29: Weather particle compositing order — document: Resolve -> Sprite -> Weather -> Font
- R30: Shape-vector cache telemetry — add hit/miss counters
- R31: Generate custom vectors from font-1.xp, not web font
- R32: Font recolor table — copy exact C++ byte sequence, add golden test
- R33: Execute plans sequentially (01->02->03->04->05) to avoid merge conflicts
- R34: Networking test — choose background thread + mpsc or mock transport

### Phase 7 Execution Order (Recommended)

**Sequential, not parallel:**
1. Plan 02 (Game State Machine) — foundation for Plans 01, 05
2. Plan 01 (Audio) — independent, verify bevy_kira_audio first
3. Plan 03 (Networking) — requires Phase 6 character entities
4. Plan 04 (Shape Vector + Font) — modifies RESOLVE, regression risk
5. Plan 05 (Weather) — depends on Plan 02, integrates with pipeline

### Phase 7 Success Criteria (Additions)

Before Phase 7 ships:
- All dependencies compile with Bevy 0.18
- 16-track audio mixer verified with concurrent playback test
- Networking in-process integration test passes
- Game state transitions Loading -> Playing -> Paused and back
- Font skins render correct colors (grey, gold, pink)
- Shape-vector cache hit rate > 50% in test scenes
- Weather particles visible after sprite blit
- Full pipeline sustains 60 FPS at 240x135 with all systems active
- Per-subsystem frame-time logging in place
- Phase 4/5 golden-file tests pass with resolve(shape_matcher=None)

---
## Cross-Phase Architecture Audit (R44-R62)

**Audit date:** 2026-02-20
**Audited by:** 4 parallel compound-engineering agents (architecture-strategist, spec-flow-analyzer x2, performance-oracle)
**Scope:** Phases 5, 6, 7 plan gaps + overall architecture review

### Summary Table

| ID | Phase | Risk | Severity | Status |
|----|-------|------|----------|--------|
| R44 | 5 | SampleBuffer has no resize(); AsciiCellGrid resizes on window change causing dimension desync | CRITICAL | PLANNED - Plan 05-05 adds ensure_buffer_size() |
| R45 | 5 | Circular dependency: Plan 05-05 calls update_terrain_dark from Plan 05-06, same wave | HIGH | PLANNED - Plan 05-05 defers shadow to 05-06's own system |
| R46 | 5 | AkmMesh loading and RuntimeInstance linkage underspecified - no MeshRegistry | HIGH | PLANNED - Plan 05-05 adds MeshRegistry resource |
| R47 | 5 | resolve() not extensible for Phase 7 shape-vector; needs GlyphSelector trait | HIGH | PLANNED - Plan 05-04 adds GlyphSelector trait |
| R48 | 5 | Phase 3.1 not listed as Phase 5 dependency (TextureView fix needed before Phase 5) | HIGH | RESOLVED - Phase 3.1 complete (executed 2026-02-20) |
| R49 | 5 | No end-to-end integration test loading real .a3d file through full pipeline | HIGH | PLANNED - Plan 05-06 adds test_load_a3d_full_pipeline |
| R50 | 5 | TerrainShader never reads patch.dark bitmask - shadows computed but never applied | MEDIUM | PLANNED - Plan 05-04 reads patch.dark to modulate diffuse |
| R51 | 5 | Frustum plane extraction algorithm underspecified in Plan 05-03 | MEDIUM | PLANNED - Plan 05-03 cites Gribb/Hartmann + deterministic tests |
| R52 | 5 | test_pattern_system not disabled when real pipeline activates (R07) | MEDIUM | PLANNED - Plan 05-05 gates behind cfg feature |
| R53 | 5 | Two distinct MatCell types in asset_loader vs render - no verified conversion | MEDIUM | PLANNED - Plan 05-05 adds test_matcell_layout_equivalence |
| R54 | 6 | Missing camera-follows-player system - camera stays at origin while character moves | HIGH | PLANNED - Plan 06-03 adds sync_camera_to_player |
| R55 | 6 | Frame-rate-dependent camera rotation (45 deg/frame not 45 deg/sec) | HIGH | PLANNED - Plan 06-03 multiplies by time.delta_secs() |
| R56 | 6 | No Block input key defined - state machine has Block but no way to enter it | HIGH | PLANNED - Plan 06-02 defines KeyF + shield guard |
| R57 | 6 | PhysicsIO.yaw never written - WASD forces not rotated by facing direction | HIGH | PLANNED - Plan 06-03 adds camera-relative WASD rotation |
| R58 | 6 | Water ripple cannot detect reflected cells post-resolve (spare bits lost) | MEDIUM | PLANNED - Plan 06-03 applies ripple during resolve (spare bits still available) |
| R59 | 6 | Attack/Stand animation-complete transitions undefined | MEDIUM | PLANNED - Plan 06-02 adds check_animation_complete() |
| R60 | 6 | Character spawns at origin, may be underground | MEDIUM | PLANNED - Plan 06-02 queries RuntimeTerrain height at spawn |
| R61 | 7 | Shape-vector cache unbounded HashMap will grow without limit | HIGH | PLANNED - Plan 07-04 uses LRU cache (8192 entries) |
| R62 | 5 | Per-frame Vec<AnsiCell> temp allocation in resolve_to_grid (129KB/frame) | MEDIUM | PLANNED - Plan 05-05 adds ResolveBuffer Resource |
| R63 | 5-7 | Silent query mismatches - systems matching wrong entity sets (AP-3/AP-7) | HIGH | PLANNED - Plans 05-05, 06-02, 07-03 add ECS integration tests that spawn entities and verify query matching |
| R64 | 5-7 | Compile time degradation - project growth increases iteration friction | MEDIUM | PLANNED - Fast linker config + dynamic_linking dev feature |
| R65 | 5-7 | Bevy version migration - 0.19 release during development may break plugin compatibility | HIGH | PLANNED - All versions pinned through Phase 7; no mid-project upgrades |

### CRITICAL: R44 - SampleBuffer/AsciiCellGrid Dimension Desync

SampleBuffer is created from RenderConfig at startup and never resized. AsciiCellGrid resizes with window via handle_window_resize. After a window resize, SampleBuffer dimensions disagree with AsciiCellGrid dimensions. resolve_to_grid() will either panic (debug) or produce corrupt output (release).

**Fix:** Add `SampleBuffer::resize(ascii_w, ascii_h)` method. Call it from pipeline system at frame start, or synchronize in handle_window_resize. **Must fix before Phase 5 Plan 05.**

### HIGH: R45 - Circular Dependency in Wave 3

Plan 05-05 Task 1 step 5e calls `update_terrain_dark()` which Plan 05-06 creates. Both are wave 3. If 05-05 compiles before 05-06, the call won't compile.

**Fix:** Either (a) move shadow trigger out of assembly into 05-06's own system, or (b) explicitly sequence 05-06 before 05-05's shadow-calling step, or (c) create shadow.rs stub in 05-01 (wave 1).

### HIGH: R54-R57 - Phase 6 Physics-Camera Integration Gaps

Four related gaps in how physics output connects to camera and input:
1. **R54**: No system updates GameCamera.pos from PhysicsIO.pos (camera stays at origin)
2. **R55**: Camera rotation is 45 deg per frame (not per second) - frame-rate dependent
3. **R56**: Block state exists but no key triggers it
4. **R57**: WASD forces not rotated by yaw - character moves in world-space not camera-space

All must be fixed in Phase 6 plans before execution.

### HIGH: R47 - resolve() API Not Extensible

Phase 7 Plan 04 adds ShapeVectorMatcher to resolve(). Current signature is fixed. Adding a `GlyphSelector` trait now (Phase 5 Plan 04) avoids a breaking API change later.

```rust
pub trait GlyphSelector {
    fn select_glyph(&self, samples: &[&Sample; 4], cx: i32, cy: i32) -> u8;
}
```

### HIGH: R61 - Shape-Vector Cache Unbounded

The HashMap<u32, u8> cache grows without limit. At 32,400 cells/frame with a 30-bit key space, memory usage grows indefinitely. Replace with bounded LRU cache of ~16K entries (~128KB).

### Performance Assessment

**Frame budget at 240x135: ACHIEVABLE**
- Best case: ~3ms (no reflection, no shape-vector)
- Worst case: ~12.7ms (reflection + shape-vector + networking + weather)
- Budget: 16.67ms (60fps)
- Margin: ~4ms for Bevy overhead

**Resolution scaling: 480x270 is NOT achievable at 60fps** without fundamental changes (multi-threaded rasterizer, tiled rendering). Document as known limitation.

**Primary optimization target: rasterizer inner loop** - incremental edge function evaluation would save ~40-50% of rasterization time.

**Memory budget: ~6.1 MB engine-specific, ~30 MB total with Bevy** - well under 100MB.

### HIGH: R63 - Silent Query Mismatches (AP-3/AP-7)

ECS queries in Bevy have no compile-time safety for entity composition. If a system queries for `Query<(&SpriteReq, &AnimationState, &Transform)>` but an entity is spawned without `AnimationState`, the query silently skips it -- no error, no warning.

This is the #1 source of ECS bugs per Reddit community consensus. It compounds across phases:
- Phase 5: Pipeline reads Resources (low risk -- no entity queries)
- Phase 6: `query_character_sprites` queries character entities (MEDIUM risk -- first entity queries)
- Phase 7: `apply_remote_poses` queries networked entities (MEDIUM risk -- cross-phase composition)

**Impact:** A character system that expects `CharacterState` but an entity spawns without it -> character is invisible to the system. No error. Hours of debugging.

**Mitigation (3-pronged):**
1. **Integration tests** (PLANNED): Each phase adds tests that spawn entities and verify systems process them. Tests also spawn "decoy" entities without required components to verify exclusion.
   - Phase 5: `tests/pipeline_integration.rs` (1 test)
   - Phase 6: `tests/ecs_character_integration.rs` (3 tests)
   - Phase 7: `tests/ecs_network_integration.rs` (1 test)
2. **Character marker component** (PLANNED in 06-02): `#[derive(Component)] pub struct Character;` with `With<Character>` filter on all character queries.
3. **Required Components** (FUTURE): When Bevy 0.18's Required Components API is stable, add `#[require(Transform, AnimationState)]` on ActionState.

**Status:** PLANNED -- integration test requirements added to Plans 05-05, 06-02, 06-03, 07-03.

### MEDIUM: R64 - Compile Time Degradation

With 188 tests and growing (300+ projected by Phase 7), incremental compile times will increase as modules grow. Single-crate architecture means every change recompiles all dependent modules.

**Current mitigations (implemented):**
1. `.cargo/config.toml` with linker guidance for macOS (lld via Homebrew)
2. `dev` feature flag enabling `bevy/dynamic_linking` for fast iterative builds
3. Bevy features are selective (`default-features = false`, only 6 features enabled)
4. No custom proc macros (all derives are standard, cached well)

**Future mitigations (evaluate before Phase 7):**
5. Cargo workspace crate splitting: `asciicker-core` (types, parsers) + `asciicker-render` (rasterizer) + `asciicker-game` (binary)
6. `cargo-nextest` for parallel test execution
7. Profile-guided incremental build optimization

**Trigger for action:** If incremental compile time exceeds 10 seconds, evaluate crate splitting.

### HIGH: R65 - Bevy Version Migration

Bevy releases breaking changes every 3-6 months. Bevy 0.19 will likely ship mid-2026, potentially before Phase 7 completes. Third-party plugins (bevy_kira_audio, bevy_replicon, bevy_replicon_renet2) must all align with the same Bevy version. An unplanned upgrade mid-project would require touching every plugin, system, and test.

**Impact:** If a critical bug fix lands only in 0.19, or a third-party crate drops 0.18 support, the project faces a forced migration mid-development.

**Mitigation:**
1. Pin `bevy = "=0.18.0"` (exact, not caret) in Cargo.toml — already done.
2. Commit `Cargo.lock` to version control.
3. All work through Phase 7 targets Bevy 0.18.0. No version upgrades during active development.
4. Post-Phase-7 task: evaluate Bevy 0.19 migration when v1 is feature-complete.
5. If a 0.18-only bug is discovered, attempt workaround before considering upgrade.

---
*Last updated: 2026-02-21 after ECS/Bevy community audit — added R63-R65*
