# Roadmap: Asciicker Rust Port

## Overview

This roadmap takes the Asciicker C++ game engine (82K lines, custom CPU software rasterizer rendering 3D worlds as ASCII art) and rebuilds it in Rust/Bevy across 15 phases. The journey moves from a compiling skeleton through isolated subsystems (asset parsers, GPU output, CPU rasterizer) to full pipeline integration, then layers physics, character gameplay, and finally game systems like audio/networking/weather. Phases 3 and 4 are independent and can execute in parallel -- the GPU output plugin uses synthetic test data while the CPU rasterizer is pure algorithm work. Phase 5 is the critical convergence where all prior work connects to render a real Asciicker world file.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Compiling Bevy 0.18 skeleton with plugin-per-subsystem architecture and ECS conventions
- [x] **Phase 2: Asset Parsers** - XP sprite and A3D world/terrain binary file loaders with golden-file tests
- [x] **Phase 3: GPU Output** - Bevy render plugin displaying ASCII glyphs via Mage Core 4-texture WGSL shader
- [x] **Phase 3.1: Audit Remediation** - Fix Critical/High severity code-level risks from Phases 1-3 audit before Phase 5 integration (INSERTED)
- [x] **Phase 4: CPU Rasterizer Core** - SampleBuffer, triangle/line rasterization, materials, color quantization, and RESOLVE stage
- [~] **Phase 5: Pipeline Integration** - Partial: real world rendering exists, but resolve/compositing still diverges from C++ and Stage 4/visibility remain incomplete
- [~] **Phase 6: Physics and Character** - Partial: runtime is playable, but renderer parity and water-edge correctness remain incomplete
- [~] **Phase 7: Game Systems** - Partial: audio/network/weather/menu paths exist, but visual occupancy/contrast tuning and final sign-off remain incomplete
- [ ] **Phase 7.1: Physics & Character Polish** - Critical fixes for camera sync, rotation, and actions (INSERTED)
- [ ] **Phase 8: NPC AI and Combat** - Enemy spawning, target selection, stuck detection, and melee combat
- [ ] **Phase 9: Inventory and Items** - Item catalog, grid-based inventory UI, pickup/drop interaction, and equipment lifecycle
- [ ] **Phase 10: UI/HUD and Interaction** - Status bars, chat UI, minimap, and screen-to-world unprojection
- [ ] **Phase 11: Full Menu System** - Hierarchical menu navigation, dither transitions, level selection, and settings persistence
- [ ] **Phase 12: Full Networking** - Authoritative entity replication, combat protocol, item sync, movement prediction, and lag compensation
- [ ] **Phase 13: NPC Scripting** - Embedded Lua runtime, script API, and hot-reloading behavior

## Phase Details

### Phase 1: Foundation
**Goal**: A compiling Bevy 0.18 project with the correct plugin architecture, coordinate conventions, and ECS resource/entity mapping so that all subsequent phases build on a solid base
**Depends on**: Nothing (first phase)
**Requirements**: FOUND-01, FOUND-02, FOUND-03, FOUND-04
**Success Criteria** (what must be TRUE):
  1. `cargo build` succeeds with Bevy 0.18.0 pinned, `default-features = false`, and the custom feature set (2d_api, bevy_render, bevy_core_pipeline, bevy_shader)
  2. Running the binary opens a Bevy window and each plugin (AssetLoader, World, CpuRasterizer, AsciiOutput, Physics, Character, Game) registers without error
  3. Coordinate system convention (Z is UP) is enforced via a documented constant and compile-time type alias, not just comments
  4. SampleBuffer and AsciiCellGrid exist as Bevy Resources; a test system can write to SampleBuffer and read from AsciiCellGrid within the same frame
**Plans**: 2 plans

Plans:
- [x] 01-01-PLAN.md -- Project setup, Z-up coordinates, 8 stub plugins
- [x] 01-02-PLAN.md -- ECS resources (SampleBuffer, AsciiCellGrid) with TDD

### Phase 2: Asset Parsers
**Goal**: All original Asciicker binary asset formats (.xp sprites, .a3d terrain, .a3d world, .akm meshes) load correctly through Bevy's async asset system, validated by golden-file tests against known C++ reference output
**Depends on**: Phase 1
**Requirements**: ASSET-01, ASSET-02, ASSET-03, ASSET-04, ASSET-05, ASSET-06, ASSET-07
**Success Criteria** (what must be TRUE):
  1. An .xp sprite file from the C++ asset directory loads and produces the correct CP437 glyphs, fg/bg colors, and layer structure (colorkey, height, visual, swoosh merge) verified by golden-file snapshot
  2. An .a3d terrain file loads and produces the correct 188-byte FilePatch array with HEIGHT_SCALE=16, verified by golden-file snapshot of patch heights
  3. An .a3d world file loads and produces the correct BSP tree structure and instance list (3 variant types, correct format version detection), verified by golden-file snapshot
  4. All loaders integrate with Bevy AssetServer -- assets load via `Handle<XpSprite>`, `Handle<A3dTerrain>`, `Handle<A3dWorld>` with async loading and typed access
  5. `cargo test` passes all golden-file comparisons with zero diff against C++ reference data
**Plans**: 4 plans

Plans:
- [x] 02-01-PLAN.md -- XP sprite parser with shared error/constants (TDD, wave 1)
- [x] 02-02-PLAN.md -- A3D terrain and material table parsers (TDD, wave 1)
- [x] 02-03-PLAN.md -- A3D world and AKM mesh parsers (TDD, wave 1)
- [x] 02-04-PLAN.md -- Bevy AssetLoader integration and golden-file tests (wave 2)

### Phase 3: GPU Output
**Goal**: A Bevy render plugin displays an AsciiCellGrid as colored CP437 glyphs in a window using the Mage Core 4-texture WGSL shader approach, independent of the CPU rasterizer
**Depends on**: Phase 1
**Requirements**: GPU-01, GPU-02, GPU-03, GPU-04, GPU-05
**Success Criteria** (what must be TRUE):
  1. A synthetic test pattern (checkerboard of glyphs with varying fg/bg colors) renders correctly in a Bevy window using the fullscreen WGSL shader
  2. The font atlas (CP437 16x16 glyph grid) loads as a Bevy PNG asset and every glyph renders with correct proportions
  3. The render plugin uses Bevy's Extract/Prepare/Render pipeline with unconditional extraction of AsciiCellGrid from Main World to Render World every frame (no stale data)
  4. Resizing the window updates the AsciiCellGrid dimensions and the display adjusts without artifacts or crashes
**Plans**: 3 plans

Plans:
- [x] 03-01-PLAN.md -- Font atlas, WGSL shader, GPU types, test pattern system (wave 1)
- [x] 03-02-PLAN.md -- Bevy ViewNode render pipeline with Extract/Prepare/Render (wave 2)
- [x] 03-03-PLAN.md -- Window resize handling + visual verification checkpoint (wave 3)

### Phase 3.1: Audit Remediation (INSERTED)
**Goal**: Fix code-level risks from the Phases 1-4 audit that would cause failures or undefined behavior during Phase 5 pipeline integration — TextureView lifetime, coordinate safety, parser robustness, GPU hardening, plus Phase 4 execution gap closures (dead unsafe code, exhaustive quantization tests, LUT consistency, reflection path, boundary tests)
**Depends on**: Phase 3, Phase 4
**Requirements**: AUDIT-01, AUDIT-02, AUDIT-03, AUDIT-04, AUDIT-05, GAP-02, GAP-03, GAP-06, GAP-10, GAP-11
**Success Criteria** (what must be TRUE):
  1. TextureView objects in GPU BindGroup have correct lifetime management — no use-after-free under high-frequency texture updates
  2. GameVec3 is a newtype wrapper that prevents silent mixing with Bevy Vec3 at compile time
  3. Asset parsers validate sprite dimensions (checked_mul) and transform matrices (is_finite) without panicking on malformed input
  4. Font atlas loading failure produces a visible warn! log instead of a silent black screen
  5. An integration test verifies all plugins initialize in correct order without resource-missing panics
  6. RGB555 rgb2pal() returns valid xterm-256 index for all 32768 input values
  7. auto_mat LUT passes full-table consistency check (valid fg/bg indices, non-zero glyphs)
  8. Dead unsafe SampleBuffer accessors removed or justified with safety comments
  9. SampleBuffer boundary tests pass (zero-size, border pixels, last valid index)
  10. Reflection palette path produces correctly darkened output vs non-reflection path
**Plans**: 1 plan

Plans:
- [x] 031-01-PLAN.md -- Fix 5 audit items + 5 Phase 4 execution gaps: TextureView lifetime, GameVec3 newtype, parser robustness, GPU hardening, plugin ordering test, RGB555 validation, auto_mat consistency, dead unsafe cleanup, boundary tests, reflection path test (wave 1)

### Phase 4: CPU Rasterizer Core
**Goal**: The CPU rasterizer produces correct AnsiCell output from hard-coded geometry, matching C++ reference output within the 1% cell difference threshold, at 60fps or better at 240x135 ASCII resolution
**Depends on**: Phase 1
**Requirements**: REND-01, REND-02, REND-03, REND-04, REND-05, REND-06, REND-07, REND-10
**Success Criteria** (what must be TRUE):
  1. SampleBuffer allocates with 2x supersampling and clears via double-allocation memcpy in under 0.5ms at 240x135 resolution
  2. Bresenham line rasterization and barycentric triangle rasterization produce output matching C++ reference for canonical test geometry (verified by golden-file snapshot)
  3. The material system (auto_mat LUT, 32KB shade table) produces correct glyph and fg/bg color selection for known input samples
  4. RGB555 to xterm-256 color quantization matches C++ output for all 32768 RGB555 values
  5. RESOLVE stage (2x2 downsample, per-cell glyph/color selection) produces AnsiCell grid matching C++ reference within <1% cell difference on canonical test scenes, at 60fps+ sustained
**Plans**: 4 plans

Plans:
- [x] 04-01-PLAN.md -- Sample struct, SampleBuffer double-allocation, AnsiCell, color quantization (wave 1)
- [x] 04-02-PLAN.md -- MatCell/Material structs, auto_mat LUT generation (wave 2)
- [x] 04-03-PLAN.md -- Bresenham line and barycentric triangle rasterization (wave 2)
- [x] 04-04-PLAN.md -- RESOLVE stage, pipeline skeleton, performance benchmark (wave 3)

### Phase 5: Pipeline Integration
**Goal**: The full 6-stage rendering pipeline connects asset parsers, CPU rasterizer, and GPU output to render a real Asciicker .a3d world file in a window with perspective camera navigation
**Depends on**: Phase 2, Phase 3, Phase 4
**Requirements**: TERR-01, TERR-02, TERR-03, TERR-04, WRLD-01, WRLD-02, WRLD-03, WRLD-04, REND-08, REND-09, CAM-01, CAM-02, CAM-03, VIS-02
**Success Criteria** (what must be TRUE):
  1. Loading an original Asciicker .a3d world file renders terrain, mesh instances, and sprites in a Bevy window that is visually recognizable as the same scene rendered by the C++ engine
  2. The perspective camera responds to Q/E rotation toggle and scene shift (multiplied by 2 per TRAP-R06) with smooth navigation through the world
  3. Terrain quadtree with HEIGHT_CELLS=4 and VISUAL_CELLS=8 renders with frustum culling, and terrain shadows cast correctly via 64-bit bitmask per patch
  4. BSP tree traversal renders world geometry with frustum culling, all 4 node types functional (NODE, NODE_SHARE, LEAF, INST), and instance flags respected
  5. Golden-file CI comparison of full-scene AnsiCell output against C++ reference shows <1% cell difference
**Current Status**: PARTIAL
**Reality Check**:
  1. Real terrain/world rendering is present, but `engine-port/src/render/resolve.rs` still contains an explicitly simplified resolve path relative to original `render.cpp`.
  2. `engine-port/src/render/pipeline.rs` still has an incomplete shadow/visibility story, so the advertised full 6-stage parity is not yet true.
  3. Deterministic replay comparisons against the locked `3a621b8` baseline still show large render-side divergence (see `docs/FAILURE_LOG.md` entries `F244`-`F246`).
**Plans**: 8 plans

Plans:
- [x] 05-01-PLAN.md -- Terrain quadtree runtime (RuntimePatch, QuadNode, frustum query) (wave 1)
- [x] 05-02-PLAN.md -- BSP tree runtime with SAH construction, near-child-first frustum query (wave 1)
- [x] 05-03-PLAN.md -- Perspective camera with view matrix, frustum planes, Q/E rotation (wave 1)
- [x] 05-04-PLAN.md -- TerrainShader, MeshShader, resolve_to_grid bridge (wave 2)
- [x] 05-05-PLAN.md -- A3D assembly system, pipeline orchestrator with per-stage timing (wave 3)
- [x] 05-06-PLAN.md -- Terrain shadows, golden-file CI comparison, budget assertion (wave 4)
- [x] 05-07-PLAN.md -- GAP CLOSURE: Deploy game_map_y8.a3d asset, wire render_mesh() in pipeline (wave 5)
- [x] 05-08-PLAN.md -- GAP CLOSURE: VIS-02 status correction and C++ reference unblock documentation (wave 5)

### Phase 6: Physics and Character
**Goal**: A player-controlled character moves through the rendered world with sphere-based collision physics, state-machine animations, and water/effects, producing a playable single-player experience
**Depends on**: Phase 5
**Requirements**: PHYS-01, PHYS-02, PHYS-03, PHYS-04, CHAR-01, CHAR-02, CHAR-03, CHAR-04, FX-01, FX-02
**Success Criteria** (what must be TRUE):
  1. A character entity spawns in the world and responds to keyboard/mouse input for movement (walk, run) and actions (attack, block) with correct state transitions
  2. Sphere-based collision prevents the character from passing through terrain and world geometry, with correct grounded detection enabling walking on surfaces
  3. The character renders with the correct equipment sprite (5D lookup: action x weapon x shield x helmet x armor x mount) and frame animation timing
  4. Water surfaces render with reflections (reflection stage re-runs terrain+world below water plane) and Perlin Z-perturbation ripple effect
  5. Physics runs at 15ms fixed timestep via Bevy FixedUpdate with max 10 substeps, maintaining 60fps with character + full scene
**Current Status**: PARTIAL
**Reality Check**:
  1. Water reflections exist, but the current water edge / surface behavior is still visually wrong and remains an open regression (`F244`-`F246`).
  2. `engine-port/src/physics/geometry.rs` now uses AKM mesh triangles; Phase 6 remains partial because renderer/water parity is still open, not because of bbox proxy collision.
  3. The canonical renderer regression baseline remains commit `3a621b8` captured at `artifacts/baselines/backup-3a621b8-run2` until manual sign-off.
**Plans**: 3 plans

Plans:
- [x] 06-01-PLAN.md -- Physics core: collision, forces, PhysicsIO, FixedUpdate, collect_terrain_triangles + collect_world_triangles free functions (TDD, wave 1)
- [x] 06-02-PLAN.md -- Character: state machine (with Block), equipment, input (Q/E ownership), animation, sprite_query (TDD, wave 2)
- [x] 06-03-PLAN.md -- Water reflection (actual geometry re-query), Perlin ripple, GamePlugin (no sub-plugins), torque-to-camera, perf benchmark (wave 3)

### Phase 7: Game Systems
**Goal**: Audio, multiplayer networking, weather effects, menus, and visual quality upgrades complete the game for v1 release
**Depends on**: Phase 6
**Requirements**: AUD-01, AUD-02, NET-01, NET-02, GAME-01, GAME-02, GAME-03, VIS-01, VIS-03
**Success Criteria** (what must be TRUE):
  1. Sound effects play correctly via bevy_kira_audio with 16-track mixer support, and audio does not cause frame drops
  2. Two clients can connect to a server and see each other's characters move in the same world with position sync and entity replication
  3. Weather effects (rain, snow) render as particle systems that are visible in the ASCII output and respond to game state
  4. A main menu loads on startup with navigation to start game, and the game state machine transitions correctly between Loading, Playing, and Paused states
  5. Alex Harri 6D shape-vector glyph matching replaces auto_mat glyph selection at the RESOLVE stage (auto_mat still used for fg/bg color), and all 3 font skins (grey, gold, pink) are available
**Current Status**: PARTIAL
**Reality Check**:
  1. The shape-vector path now uses the full default alphabet, runtime alphabet switching, and live tuning controls, but occupancy/contrast tuning is still open (`F248`).
  2. Font1 is wired into menu/loading paths, but broader runtime/UI integration is still incomplete.
  3. Visual-quality completion claims are blocked until the renderer improves against the locked baselines and the user signs off on an actual improvement.
  4. A 2026-03-11 architecture audit found that global final-stage shape-vector override likely conflicts with original `render.cpp` glyph semantics (`auto_mat`, silhouette, linecase, half-block splits). A first constrained integration pass is now implemented, blocking shape-vector on those semantic cell classes, but replay evidence against the orbit baseline is still needed before the issue can be considered closed (`F249`).
**Plans**: 7 plans

Plans:
- [x] 07-01-PLAN.md -- Audio: bevy_kira_audio 0.25, 16-track DynamicAudioChannels mixer (wave 1)
- [x] 07-02-PLAN.md -- Game state machine (Loading/Playing/Paused) and main menu (wave 1)
- [x] 07-03-PLAN.md -- Networking: bevy_replicon 0.38, binary protocol, server/client (wave 2)
- [x] 07-04-PLAN.md -- Visual quality: Alex Harri 6D shape-vector + Font1 3 skins (TDD, wave 3)
- [x] 07-05-PLAN.md -- Weather: ring-buffer particle pool, Perlin wind, snow/rain (TDD, wave 4)
- [ ] 07-06-PLAN.md -- GAP CLOSURE: Weather debug keybind (F5 cycles WeatherState) (wave 5)
- [ ] 07-07-PLAN.md -- GAP CLOSURE: Network integration test (server+client renet transport) (wave 5)

### Phase 7.1: Physics & Character Polish
**Goal**: Fix critical integration gaps from Phase 6 identified during audit: camera sync, rotation, and Block action
**Depends on**: Phase 6
**Requirements**: PHYS-FIX-01, CHAR-FIX-01
**Success Criteria**:
  1. Camera position is synchronized to player position from physics output
  2. WASD forces are rotated by camera yaw before physics integration
  3. Block input key (KeyF) is functional and triggers state transition
**Plans**: 1 plan

### Phase 8: NPC AI and Combat
**Goal**: Enemies spawn in the world, have autonomous AI behavior, and the player can engage in melee combat with them
**Depends on**: Phase 7
**Requirements**: NPC-01, NPC-02, NPC-03, NPC-04, NPC-05, NPC-06, NPC-07, NPC-08
**Success Criteria**:
  1. NPCs spawn automatically at world load from .a3d EnemyGen data with randomized equipment
  2. The SpatialGrid provides efficient proximity queries for AI and combat targeting
  3. NPCs autonomously chase players, recover from being stuck, and return to spawn points
  4. Melee combat hits connect exactly on animation frame 21, applying damage and knockback
**Plans**: 4 plans

### Phase 9: Inventory and Items
**Goal**: A complete item system allowing pickup, management, and equipment of various item types
**Depends on**: Phase 8
**Requirements**: ITEM-01, ITEM-02, ITEM-03, ITEM-04
**Success Criteria**:
  1. An 8x20 grid-based inventory UI supports bitmask collision and directional navigation
  2. Players can pick up items from the world using the SpatialGrid and drop them back
  3. Consumables (food, potions) apply gameplay effects and are destroyed on use
  4. Equipping items modifies the character's 5D sprite lookup and visual appearance
**Plans**: 4 plans

### Phase 10: UI/HUD and Interaction
**Goal**: Enhanced player interface with status bars, chat, minimap, and world-space feedback
**Depends on**: Phase 9
**Requirements**: HUD-01, HUD-02, HUD-03, HUD-04
**Success Criteria**:
  1. HP and MP bars render correctly at the bottom-left using Font1 on the AsciiCellGrid
  2. A TalkBox system supports scrollable chat history and per-character talk bubbles
  3. A top-right minimap displays height-sampled terrain and nearby NPC dots
  4. Mouse clicks correctly target world entities via screen-to-world unprojection
**Plans**: 4 plans

### Phase 11: Full Menu System
**Goal**: Complete hierarchical menu hierarchy with settings persistence and dithered transitions
**Depends on**: Phase 7
**Requirements**: FMENU-01, FMENU-02, FMENU-03, FMENU-04
**Success Criteria**:
  1. A stack-based hierarchical menu allows navigation through Video, Audio, and Control settings
  2. Background scaling and dither-fade transitions match the C++ engine's aesthetic fidelity
  3. A Level Selection screen lists and loads available .a3d world files
  4. User settings are persisted to a JSON config file and applied at startup
**Plans**: 4 plans (1 optional)

### Phase 12: Full Networking
**Goal**: Authoritative multiplayer gameplay with entity replication and lag compensation
**Depends on**: Phase 10
**Requirements**: FNET-01, FNET-02, FNET-03, FNET-04, FNET-05
**Success Criteria**:
  1. Authoritative server replicates character positions, animations, and states to all clients
  2. Combat actions and item interactions are validated server-side to prevent cheating
  3. Client-side prediction and server reconciliation provide zero-latency movement feedback
  4. Remote entity interpolation and ping measurement handle network jitter and delay
**Plans**: 5 plans

### Phase 13: NPC Scripting
**Goal**: User-extensible NPC behavior via embedded Lua scripting with hot-reloading
**Depends on**: Phase 8
**Requirements**: SCRIPT-01, SCRIPT-02, SCRIPT-03
**Success Criteria**:
  1. A sandboxed Lua 5.4 runtime allows loading and executing NPC scripts from assets
  2. A script API exposes world data, entity stats, and movement/action commands to Lua
  3. Scripts can be hot-reloaded at runtime without restarting the game engine
**Plans**: 3 plans

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 (parallel with 4) -> 4 (parallel with 3) -> 5 -> 6 -> 7
Note: Phases 3 and 4 are independent (both depend only on Phase 1) and can execute in parallel.

## Immediate Next Steps

1. Continue threshold tuning against `artifacts/baselines/orbit-2026-03-11-current`, using `artifacts/baselines/orbit-2026-03-11-semantic-gated-debug` as the current best default replay.
2. Re-run deterministic replay after each renderer tweak and track `threshold_skip_cells`, `fallback_space_cells`, and `colored_space_cells`.
3. Keep comparing renderer changes against the locked `3a621b8` baseline in `artifacts/baselines/backup-3a621b8-run2` until manual user sign-off.
4. After occupancy and resolve behavior are trustworthy, resume deferred renderer correctness work:
   - remaining mixed-cell resolve parity
   - water-edge and water-surface tuning
   - remaining Phase 7 visual-path completion

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 2/2 | Complete | 2026-02-20 |
| 2. Asset Parsers | 4/4 | Complete | 2026-02-20 |
| 3. GPU Output | 3/3 | Complete | 2026-02-20 |
| 3.1 Audit Remediation | 1/1 | Complete    | 2026-02-20 |
| 4. CPU Rasterizer Core | 4/4 | Complete | 2026-02-20 |
| 5. Pipeline Integration | 8/8 | Partial | 2026-02-22 |
| 6. Physics and Character | 3/3 | Partial | 2026-02-24 |
| 7. Game Systems | 5/7 | Partial | 2026-02-26 |
| 7.1 Physics & Character Polish | 0/1 | Planned | - |
| 8. NPC AI and Combat | 0/4 | Planned | - |
| 9. Inventory and Items | 0/4 | Planned | - |
| 10. UI/HUD and Interaction | 0/4 | Planned | - |
| 11. Full Menu System | 0/4 | Planned | - |
| 12. Full Networking | 0/5 | Planned | - |
| 13. NPC Scripting | 0/3 | Planned | - |

## Developer Experience

### Iteration Speed Strategy

Fast compile times are critical for maintaining development velocity across Phases 5-7. The project uses a layered approach:

**Active now:**
- **Selective Bevy features:** `default-features = false` with only 6 features (avoids pulling in 3D rendering, UI, physics plugins that add compile time)
- **Dynamic linking feature:** `cargo run --features dev` enables `bevy/dynamic_linking` for ~10x faster incremental link times during development
- **Linker configuration:** `engine-port/.cargo/config.toml` documents fast linker setup for macOS (lld via Homebrew LLVM)
- **Zero proc macros:** No custom derive macros; all derives are standard Bevy/serde (cached by compiler)

**Planned (evaluate before Phase 7):**
- **Crate splitting:** If incremental compile time exceeds 10 seconds, split into workspace:
  - `asciicker-core`: types, constants, math, asset parsers (no Bevy dependency)
  - `asciicker-render`: CPU rasterizer, materials, quantization, resolve
  - `asciicker-game`: top-level binary, Bevy plugins, game systems
- **Parallel test runner:** `cargo-nextest` for faster test execution
- **Compile time tracking:** Monitor incremental build times at each phase boundary

**Risk:** R64 in RISK-ASSESSMENT.md tracks compile time degradation.

### Version Pinning

All work through Phase 7 targets **Bevy 0.18.0**. No Bevy version upgrades during active development. Third-party plugin versions (bevy_kira_audio, bevy_replicon, bevy_replicon_renet2) are verified at Day 1 of their respective phases.

**Risk:** R65 in RISK-ASSESSMENT.md tracks Bevy version migration risk.
