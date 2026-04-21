# Asciicker Rust Port

## What This Is

A Rust/Bevy reimplementation of the Asciicker C++ game engine (~82K lines across 48 files). Asciicker is a multiplayer ASCII-art game featuring a custom CPU software rasterizer that renders 3D worlds using CP437 glyphs with per-cell foreground/background colors. The port preserves the original's unique aesthetic while modernizing the architecture with Bevy ECS and GPU-accelerated ASCII output via Mage Core's 4-texture approach. The final Alex Harri-quality resolve path is still in progress; the current renderer uses an intermediate shape-vector implementation.

## Core Value

The product target is an interactive ASCII rendering workbench that exposes
scene selection plus live resolution, scale, preset, inversion, and culling
controls while using the original C++ engine as reference evidence for render
behavior, not as the sole release gate.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- L3-AUDIT-FIX: Formal requirement IDs (AUD-01, AUD-02, NET-01, NET-02, GAME-01, GAME-02,
     GAME-03, VIS-01, VIS-03, etc.) are defined in ROADMAP.md Phase Details sections.
     See ROADMAP.md as the formal requirement ID registry for cross-referencing with plan files. -->

- [ ] Load and parse .xp sprite files (gzip compressed, CP437 glyphs, 3+ layers: colorkey/height/visual)
- [ ] Load and parse .a3d world files (header "AS3D" 0x44335341 LE, mesh library + terrain patches + instances + BSP)
- [ ] Implement 6-stage rendering pipeline: CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE
- [ ] Implement SampleBuffer with 2x supersampled depth/color buffer
- [ ] Port Bresenham line and barycentric triangle rasterization
- [ ] Port RGB555 -> xterm-256 color quantization
- [ ] Port auto_mat shade/glyph lookup tables
- [ ] Implement terrain system with quadtree heightmaps (HEIGHT_CELLS=4, 5x5 vertex grid per patch)
- [ ] Implement BSP tree world loading with SAH construction
- [ ] Implement sphere-based physics collision (TOI sweep, face/edge/vertex tests)
- [ ] Port character state machine (idle, walk, run, attack, block, etc.)
- [ ] Port 5D equipment sprite lookup
- [ ] Implement deferred sprite blit (after RESOLVE stage)
- [ ] Implement GPU-accelerated ASCII output as Bevy render plugin (Mage Core 4-texture approach: char index, fg, bg, font atlas)
- [ ] Integrate Alex Harri 6D shape-vector k-d tree matching at RESOLVE stage (replaces auto_mat glyph selection; auto_mat still used for fg/bg color)
- [ ] Perspective camera with Q/E rotation toggle (D004-D005: perspective REQUIRED)
- [ ] Ship the canonical render workbench with model/source selection, center ASCII canvas, and right-panel sliders/toggles from `docs/CANONICAL_SPEC.md`
- [ ] Basic multiplayer networking (client-server model)
- [ ] Audio system via bevy_kira_audio (16-track mixer)
- [ ] Water rendering with reflective surface
- [ ] Weather effects (rain, snow)
- [ ] Main menu and game state management
- [ ] NPC AI and autonomous combat behavior (NPC-01 to NPC-08)
- [ ] Grid-based inventory and world item interaction (ITEM-01 to ITEM-04)
- [ ] HUD status bars, minimap, and chat UI (HUD-01 to HUD-04)
- [ ] Hierarchical menu system and settings persistence (FMENU-01 to FMENU-04)
- [ ] Full multiplayer networking with prediction and reconciliation (FNET-01 to FNET-05)
- [ ] Embedded NPC scripting with Lua hot-reloading (SCRIPT-01 to SCRIPT-03)

### Out of Scope

- Editor/URDO system — complex, separate tool; defer to post-v1
- Mobile/web platform targets — desktop-first (Windows/Linux/macOS)
- Custom engine from scratch — Bevy provides ECS, input, audio, windowing (Decision D001)
- GPU rasterization — CPU rasterizer matches C++ fidelity (Decision D003); GPU only for final ASCII output
- Full Alex Harri 6D vectors from day one — start with auto_mat, upgrade to 6D after performance validation (Decision D010)
- Pixel-perfect original-engine parity as the primary ship gate — use it as reference evidence and regression tooling, not the sole product definition

## Context

### C++ Reference
- Original codebase in this environment: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/` (~82K lines, 48 files)
- Stable vendored editor reference: `reference/original-game/asciiid.cpp`
- Comprehensive architecture documentation in `docs/worksheets/arch/` (30+ per-file analyses)
- 4 skill packs documenting C++ subsystem internals (engine-render, world-loading, physics-system, game-mechanics)

### Technology Stack
- **Engine**: Bevy 0.18+ (ECS, input, audio, windowing) — Decision D001 (2026-02-19)
- **ASCII Rendering**: Mage Core 4-texture GPU approach as Bevy render plugin (char index + fg + bg + font atlas via WGPU/WGSL)
- **Glyph Selection**: Intermediate shape-vector matching today; final Alex Harri-quality path remains in progress (per D010/D040)
- **CPU Rasterizer**: Custom (port of C++ render.cpp) outputs to SampleBuffer
- **Audio**: bevy_kira_audio for 16-track mixer

### Reference Implementations
- **Mage Core**: `../reference/Mage-core` (~2000 lines Rust, v0.2.0, GPU-accelerated ASCII rendering)
- **Alex Harri**: `../reference/alexharri-ascii` (TypeScript/WebGL2, 6D shape-vector matching with k-d tree)

### Existing Skeleton
- `asciicker-rust/` directory has ~385 LOC Bevy 0.18.0 skeleton (does NOT compile — 4 missing modules)
- Defines components: Position, Sprite/XpCell/SpriteLayer, Character/Equipment, TerrainPatch/QuadNode, Camera
- Has partial rendering stubs: SampleBuffer/Sample/RGB555, RenderPhase enum, triangle rasterization stub
- Zero tests, 541MB stale build artifacts, not tracked in git
- GSD Phase 1 will decide whether to salvage, restructure, or restart

### Research Corpus
- 135+ documentation files covering C++ architecture, rendering deep dives, ECS conversion strategies, Bevy integration, gap analyses, and implementation plans
- 89 tracked unknowns: 42 resolved (47%), 47 remaining (6 CRITICAL all resolved, 13 HIGH remaining)
- 124 identified gaps across rendering, game logic, terrain/world, systems, and integration
- 6 gap resolution plans (rendering, game logic, systems, integration, ancestor cleanup, SampleBuffer bridge)

### Known C++ Bugs (Pre-Port)
- TERRAIN-001: terrain.cpp:613 — `if(x)` should be `if(y)`
- TERRAIN-002: terrain.cpp:805 — `u < y` should be `u < v`
- TERRAIN-003: terrain.cpp:1671 — same as TERRAIN-002
- TERRAIN-004: terrain.cpp:480,492 — verify `>` vs `>=` intent

### Critical Constants (from C++ source)
- HEIGHT_SCALE = 16 (terrain.h:54)
- HEIGHT_CELLS = 4 (terrain.h:60, produces 5x5 vertex grid)
- VISUAL_CELLS = 8 (terrain.h:66, produces 8x8 material cells)
- Coordinate system: Z is UP (physics.h:41)
- XP format: 16-byte global header (version + layers + width + height)
- A3D magic: 0x44335341 ("AS3D" little-endian)

## Constraints

- **Tech Stack**: Bevy 0.18+ (Rust 2021 edition) — D001 final, no custom engine
- **Visual Fidelity**: Use the original C++ engine as render-behavior reference where useful, but optimize the shipped target around the render workbench UX in `docs/CANONICAL_SPEC.md`
- **Binary Compatibility**: Must load original .xp sprites and .a3d world files unchanged
- **Performance**: Target 60 FPS at 1080p with full scene (terrain + world + sprites + effects)
- **Architecture**: ECS where it adds value; plain Rust where it doesn't. Use ECS for spatial entities (characters, NPCs, projectiles, terrain patches). Use plain Rust for algorithms (pathfinding, collision math), data tables (equipment lookup, material tables), state machines (character FSM, game FSM).
- **Rendering Pipeline**: CPU rasterizer -> SampleBuffer -> RESOLVE (glyph/color selection) -> GPU output (Mage Core style)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| D001: Use Bevy Engine | Provides ECS, input, audio, UI — avoids building from scratch | Accepted |
| D003: CPU rasterizer first | Matches C++ fidelity; GPU only for final ASCII output | Accepted |
| D004-D005: Perspective required | Q/E rotation and toggle features depend on it | Accepted |
| D010: Keep auto_mat initially | Speed to first render; hybrid with k-d tree added later | Accepted |
| D012: Shape-match within RESOLVE | Alex Harri k-d tree replaces auto_mat glyph selection at RESOLVE stage | Accepted |
| D040: 2D vs 6D vectors | Pending — needs performance data after initial implementation | Pending (Phase 7) |
| D041: Ancestor cleanup | Pending — needs research on C++ behavior | Pending |

---
*Last updated: 2026-03-10 during renderer reality-check / baseline lock*
