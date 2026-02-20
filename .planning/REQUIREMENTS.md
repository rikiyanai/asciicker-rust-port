# Requirements: Asciicker Rust Port

**Defined:** 2026-02-20
**Core Value:** The CPU rasterizer must produce visually identical output to the C++ engine — same glyphs, same colors, same depth ordering — so that existing Asciicker worlds render correctly in the Rust port.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Foundation

- [x] **FOUND-01**: Project compiles with Bevy 0.18 using `default-features = false` and custom feature set
- [x] **FOUND-02**: Plugin-per-subsystem architecture established (AssetLoader, World, CpuRasterizer, AsciiOutput, Physics, Character, Game)
- [x] **FOUND-03**: Coordinate system convention documented and enforced (Z is UP)
- [x] **FOUND-04**: ECS resource/entity mapping defined (SampleBuffer and AsciiCellGrid as Resources, instances as Entities)

### Asset Loading

- [x] **ASSET-01**: XP sprite files load correctly (gzip decompression, CP437 glyphs, column-major layout, 3+ layer semantics)
- [x] **ASSET-02**: XP layer semantics preserved (L0=colorkey/metadata, L1=height, L2+=visual, last layer swoosh merge)
- [x] **ASSET-03**: A3D terrain files load correctly (AS3D magic 0x44335341, 188-byte FilePatch, HEIGHT_SCALE=16)
- [x] **ASSET-04**: A3D world files load correctly (format version detection, 3 instance variants, LoadWorld/UpdateMesh/RebuildWorld order)
- [x] **ASSET-05**: AKM mesh files load correctly (Blender export format)
- [x] **ASSET-06**: Asset loaders integrate with Bevy AssetServer (async loading, Handle-based references)
- [x] **ASSET-07**: Golden-file tests validate parser output against known C++ reference data

### Rendering Pipeline

- [x] **REND-01**: SampleBuffer implemented with 2x supersampling and double-allocation for fast clear
- [x] **REND-02**: Bresenham line rasterization matches C++ output
- [x] **REND-03**: Barycentric triangle rasterization with duck-typed shader support
- [ ] **REND-04**: 6-stage pipeline executes in order: CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE
- [x] **REND-05**: Material system with auto_mat LUT (32KB, shade[4][16] elevation/diffuse lookup)
- [x] **REND-06**: RGB555 -> xterm-256 color quantization with correct projection/reflection scales
- [ ] **REND-07**: RESOLVE stage produces correct AnsiCell output (2x2 downsample, per-cell glyph/color selection)
- [ ] **REND-08**: Deferred sprite blit post-RESOLVE (painter's algorithm, far-to-near sort)
- [ ] **REND-09**: Terrain shadow computation (64-bit bitmask per patch)
- [ ] **REND-10**: Rendering pipeline achieves 60fps at 240x135 ASCII resolution (1080p window)

### GPU Output

- [ ] **GPU-01**: Bevy render plugin displays AsciiCellGrid using Mage Core 4-texture approach (char index, fg, bg, font atlas)
- [x] **GPU-02**: WGSL fullscreen shader composites glyphs with correct fg/bg colors
- [x] **GPU-03**: Font atlas loaded as Bevy PNG asset (CP437 16x16 glyph grid)
- [ ] **GPU-04**: Correct Extract/Prepare/Render world pipeline with unconditional extraction
- [ ] **GPU-05**: Window resize handled correctly (AsciiCellGrid dimensions update)

### Terrain System

- [ ] **TERR-01**: Quadtree heightmap with HEIGHT_CELLS=4 (5x5 vertex grid per patch)
- [ ] **TERR-02**: VISUAL_CELLS=8 material grid (8x8 cells per patch)
- [ ] **TERR-03**: Quadtree propagates height bounds for frustum culling
- [ ] **TERR-04**: Known C++ bugs fixed during port (TERRAIN-001 through TERRAIN-004)

### World System

- [ ] **WRLD-01**: BSP tree with SAH-style construction
- [ ] **WRLD-02**: 4 BSP node types supported (NODE, NODE_SHARE, LEAF, INST)
- [ ] **WRLD-03**: Frustum-culled BSP traversal for rendering
- [ ] **WRLD-04**: Instance flags functional (VISIBLE, USE_TREE, VOLATILE, SELECTED)

### Physics

- [ ] **PHYS-01**: Sphere-based TOI sweep collision (face/edge/vertex tests)
- [ ] **PHYS-02**: 15ms fixed timestep via Bevy FixedUpdate (max 10 substeps)
- [ ] **PHYS-03**: Gravity, buoyancy, and impulse forces
- [ ] **PHYS-04**: Grounded detection for character state transitions

### Character System

- [ ] **CHAR-01**: Character state machine (idle, walk, run, attack, block, dead)
- [ ] **CHAR-02**: 5D equipment sprite lookup (action x weapon x shield x helmet x armor x mount)
- [ ] **CHAR-03**: Player input system (keyboard + mouse movement and actions)
- [ ] **CHAR-04**: Animation system with frame timing

### Camera

- [ ] **CAM-01**: Perspective camera with configurable FOV
- [ ] **CAM-02**: Q/E rotation toggle (required by D004-D005)
- [ ] **CAM-03**: Scene shift in sample-buffer space (multiplied by 2 per TRAP-R06)

### Effects

- [ ] **FX-01**: Water rendering with reflective surface (reflection stage re-runs terrain+world below water plane)
- [ ] **FX-02**: Perlin Z-perturbation for water ripple effect

### Audio

- [ ] **AUD-01**: bevy_kira_audio integration with basic sound effect playback
- [ ] **AUD-02**: 16-track audio mixer matching C++ engine architecture

### Networking

- [ ] **NET-01**: Basic client-server multiplayer (entity replication, position sync)
- [ ] **NET-02**: Binary protocol compatible with or inspired by C++ WebSocket protocol

### Game Systems

- [ ] **GAME-01**: Game state machine (Loading -> Playing -> Paused)
- [ ] **GAME-02**: Main menu with basic navigation
- [ ] **GAME-03**: Weather effects (rain, snow particle systems)

### Visual Quality

- [ ] **VIS-01**: Alex Harri 6D shape-vector glyph matching integrated at RESOLVE stage (phased: auto_mat first, then 2D, then 6D)
- [ ] **VIS-02**: Golden-file CI comparison of AnsiCell output against C++ reference (<1% cell difference threshold)
- [ ] **VIS-03**: Font system with CP437 glyphs (3 skins: grey, gold, pink)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Editor

- **EDIT-01**: Port asciiid editor (11,500 lines, 7 edit modes, undo/redo)
- **EDIT-02**: MCP protocol for editor integration

### Platform

- **PLAT-01**: Web/WASM export
- **PLAT-02**: Mobile platform support (touch input adaptation)
- **PLAT-03**: Gamepad support via Bevy gamepad API

### Advanced

- **ADV-01**: Full 6D shape vectors if 2D proves insufficient (needs D040 performance data)
- **ADV-02**: BSP ancestor cleanup (D041 — collapse empty leaves after instance removal)
- **ADV-03**: Enemy spawner system (1,150 lines, depends on full character/combat system)
- **ADV-04**: Inventory system (3,100 lines, needs UI)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| GPU rasterization of 3D geometry | Breaks visual fidelity contract; CPU rasterizer produces specific per-sample data the resolve stage depends on (D003) |
| Custom engine from scratch | Bevy provides ECS, input, audio, windowing (D001) |
| Direct wgpu dependency | Access GPU through bevy_render only; avoids version conflicts |
| 1:1 C++ code translation | ECS architecture requires structural redesign; static mut and global pointers are anti-patterns |
| Bit-identical floating-point output | FMA/precision differences between C++ and Rust make this impossible; target perceptually identical (<1% cell diff) |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUND-01 | Phase 1 | Complete |
| FOUND-02 | Phase 1 | Complete |
| FOUND-03 | Phase 1 | Complete |
| FOUND-04 | Phase 1 | Complete |
| ASSET-01 | Phase 2 | Complete |
| ASSET-02 | Phase 2 | Complete |
| ASSET-03 | Phase 2 | Complete |
| ASSET-04 | Phase 2 | Complete |
| ASSET-05 | Phase 2 | Complete |
| ASSET-06 | Phase 2 | Complete |
| ASSET-07 | Phase 2 | Complete |
| GPU-01 | Phase 3 | Pending |
| GPU-02 | Phase 3 | Complete |
| GPU-03 | Phase 3 | Complete |
| GPU-04 | Phase 3 | Pending |
| GPU-05 | Phase 3 | Pending |
| REND-01 | Phase 4 | Complete |
| REND-02 | Phase 4 | Complete |
| REND-03 | Phase 4 | Complete |
| REND-04 | Phase 4 | Pending |
| REND-05 | Phase 4 | Complete |
| REND-06 | Phase 4 | Complete |
| REND-07 | Phase 4 | Pending |
| REND-10 | Phase 4 | Pending |
| TERR-01 | Phase 5 | Pending |
| TERR-02 | Phase 5 | Pending |
| TERR-03 | Phase 5 | Pending |
| TERR-04 | Phase 5 | Pending |
| WRLD-01 | Phase 5 | Pending |
| WRLD-02 | Phase 5 | Pending |
| WRLD-03 | Phase 5 | Pending |
| WRLD-04 | Phase 5 | Pending |
| REND-08 | Phase 5 | Pending |
| REND-09 | Phase 5 | Pending |
| CAM-01 | Phase 5 | Pending |
| CAM-02 | Phase 5 | Pending |
| CAM-03 | Phase 5 | Pending |
| VIS-02 | Phase 5 | Pending |
| PHYS-01 | Phase 6 | Pending |
| PHYS-02 | Phase 6 | Pending |
| PHYS-03 | Phase 6 | Pending |
| PHYS-04 | Phase 6 | Pending |
| CHAR-01 | Phase 6 | Pending |
| CHAR-02 | Phase 6 | Pending |
| CHAR-03 | Phase 6 | Pending |
| CHAR-04 | Phase 6 | Pending |
| FX-01 | Phase 6 | Pending |
| FX-02 | Phase 6 | Pending |
| AUD-01 | Phase 7 | Pending |
| AUD-02 | Phase 7 | Pending |
| NET-01 | Phase 7 | Pending |
| NET-02 | Phase 7 | Pending |
| GAME-01 | Phase 7 | Pending |
| GAME-02 | Phase 7 | Pending |
| GAME-03 | Phase 7 | Pending |
| VIS-01 | Phase 7 | Pending |
| VIS-03 | Phase 7 | Pending |

**Coverage:**
- v1 requirements: 57 total
- Mapped to phases: 57
- Unmapped: 0

---
*Requirements defined: 2026-02-20*
*Last updated: 2026-02-20 after roadmap creation*
