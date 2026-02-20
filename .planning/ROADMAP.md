# Roadmap: Asciicker Rust Port

## Overview

This roadmap takes the Asciicker C++ game engine (82K lines, custom CPU software rasterizer rendering 3D worlds as ASCII art) and rebuilds it in Rust/Bevy across 7 phases. The journey moves from a compiling skeleton through isolated subsystems (asset parsers, GPU output, CPU rasterizer) to full pipeline integration, then layers physics, character gameplay, and finally game systems like audio/networking/weather. Phases 3 and 4 are independent and can execute in parallel -- the GPU output plugin uses synthetic test data while the CPU rasterizer is pure algorithm work. Phase 5 is the critical convergence where all prior work connects to render a real Asciicker world file.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Foundation** - Compiling Bevy 0.18 skeleton with plugin-per-subsystem architecture and ECS conventions
- [ ] **Phase 2: Asset Parsers** - XP sprite and A3D world/terrain binary file loaders with golden-file tests
- [ ] **Phase 3: GPU Output** - Bevy render plugin displaying ASCII glyphs via Mage Core 4-texture WGSL shader
- [ ] **Phase 4: CPU Rasterizer Core** - SampleBuffer, triangle/line rasterization, materials, color quantization, and RESOLVE stage
- [ ] **Phase 5: Pipeline Integration** - Full 6-stage rendering pipeline producing real scene output from .a3d world files
- [ ] **Phase 6: Physics and Character** - Sphere collision, character state machine, player input, water, and effects
- [ ] **Phase 7: Game Systems** - Audio, networking, weather, menus, and visual quality polish

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
- [ ] 01-01-PLAN.md -- Project setup, Z-up coordinates, 8 stub plugins
- [ ] 01-02-PLAN.md -- ECS resources (SampleBuffer, AsciiCellGrid) with TDD

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
- [ ] 02-02-PLAN.md -- A3D terrain and material table parsers (TDD, wave 1)
- [ ] 02-03-PLAN.md -- A3D world and AKM mesh parsers (TDD, wave 1)
- [ ] 02-04-PLAN.md -- Bevy AssetLoader integration and golden-file tests (wave 2)

### Phase 3: GPU Output
**Goal**: A Bevy render plugin displays an AsciiCellGrid as colored CP437 glyphs in a window using the Mage Core 4-texture WGSL shader approach, independent of the CPU rasterizer
**Depends on**: Phase 1
**Requirements**: GPU-01, GPU-02, GPU-03, GPU-04, GPU-05
**Success Criteria** (what must be TRUE):
  1. A synthetic test pattern (checkerboard of glyphs with varying fg/bg colors) renders correctly in a Bevy window using the fullscreen WGSL shader
  2. The font atlas (CP437 16x16 glyph grid) loads as a Bevy PNG asset and every glyph renders with correct proportions
  3. The render plugin uses Bevy's Extract/Prepare/Render pipeline with unconditional extraction of AsciiCellGrid from Main World to Render World every frame (no stale data)
  4. Resizing the window updates the AsciiCellGrid dimensions and the display adjusts without artifacts or crashes
**Plans**: TBD

Plans:
- [ ] 03-01: TBD
- [ ] 03-02: TBD

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
**Plans**: TBD

Plans:
- [ ] 04-01: TBD
- [ ] 04-02: TBD
- [ ] 04-03: TBD

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
**Plans**: TBD

Plans:
- [ ] 05-01: TBD
- [ ] 05-02: TBD
- [ ] 05-03: TBD

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
**Plans**: TBD

Plans:
- [ ] 06-01: TBD
- [ ] 06-02: TBD
- [ ] 06-03: TBD

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
**Plans**: TBD

Plans:
- [ ] 07-01: TBD
- [ ] 07-02: TBD
- [ ] 07-03: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 (parallel with 4) -> 4 (parallel with 3) -> 5 -> 6 -> 7
Note: Phases 3 and 4 are independent (both depend only on Phase 1) and can execute in parallel.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/? | Not started | - |
| 2. Asset Parsers | 1/4 | In progress | - |
| 3. GPU Output | 0/? | Not started | - |
| 4. CPU Rasterizer Core | 0/? | Not started | - |
| 5. Pipeline Integration | 0/? | Not started | - |
| 6. Physics and Character | 0/? | Not started | - |
| 7. Game Systems | 0/? | Not started | - |
