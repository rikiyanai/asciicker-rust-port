# Feature Research

**Domain:** ASCII game engine port (C++ -> Rust/Bevy)
**Researched:** 2026-02-20
**Confidence:** HIGH (based on direct C++ source analysis, skill packs, and reference implementations)

## Feature Landscape

### Table Stakes (Must Match Original or Port is Incomplete)

These features are non-negotiable because the port's stated goal is visual fidelity with the C++ engine and binary compatibility with existing .xp/.a3d assets. Missing any of these means existing Asciicker worlds break.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **XP sprite loading** | Binary compat with all existing sprites; gzip + CP437 + 3+ layers | MEDIUM | 10 bytes/cell column-major. Layer semantics critical: L0=metadata, L1=height, L2=visual, L3+=swoosh. Min 3 layers or fail. See TRAP-R04 for swoosh merging (last layer only). |
| **A3D terrain loading** | Binary compat with existing world files; AS3D magic 0x44335341 | MEDIUM | 188 bytes per FilePatch. HEIGHT_SCALE=16 is baked into format (TRAP-W06). Little-endian assumed (TRAP-W12). |
| **A3D world loading** | Binary compat with mesh instances, sprite instances, item instances | HIGH | Format version ambiguity (TRAP-W02): first int32 negative=versioned, non-negative=legacy. Three instance variants keyed by mesh_id_len. Must LoadWorld then UpdateMesh then RebuildWorld in exact order (TRAP-W03). |
| **6-stage rendering pipeline** | Core visual identity: CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE | HIGH | ~4400 lines in C++ render.cpp. Each stage has specific shader types with duck-typed Blend/Fill/Diffuse methods. Stages must execute in exact order. |
| **SampleBuffer with 2x supersampling** | Defines visual quality; (2w+4)x(2h+4) samples, double-allocated for fast clear | MEDIUM | Sample.visual is overloaded: material index vs RGB555 (TRAP-R01, keyed on spare bit 3). Double-allocation for memcpy clear (TRAP-R02). |
| **Bresenham line + barycentric triangle rasterization** | All geometry rendering depends on these primitives | MEDIUM | Template rasterizer with duck-typed shaders (TRAP-R07). Depth test is read-only; shaders must write depth (TRAP-R05). |
| **RGB555 -> xterm-256 color quantization** | Correct color output; palette formula 16+36r+6g+b | LOW | Projection vs reflection use different quantization scales (TRAP-R03). auto_mat LUT is 32KB static (TRAP-R08). |
| **Material system (auto_mat)** | Terrain shading; shade[4][16] elevation/diffuse lookup | MEDIUM | 256 materials, each with shade table. MatCell has fg/bg RGB888, glyph, blend flags. Quantized at resolve stage. |
| **Terrain quadtree with heightmaps** | Spatial queries, frustum culling, heightfield collision | HIGH | HEIGHT_CELLS=4 (5x5 vertex grid), VISUAL_CELLS=8 (8x8 material cells). Quadtree propagates height bounds. Known bugs: TERRAIN-001 through TERRAIN-004. |
| **BSP tree world system** | Spatial partitioning for mesh/sprite instances; frustum-culled traversal | HIGH | SAH-style construction. 4 node types (NODE, NODE_SHARE, LEAF, INST). Ancestor cleanup is stubbed (TRAP-W11). Instance flags: VISIBLE, USE_TREE, VOLATILE, SELECTED. |
| **Sphere-based physics** | Character movement, collision, gravity, grounded detection | HIGH | ~2350 lines. TOI sweep with face/edge/vertex tests. 15ms fixed timestep (~66Hz). Max 10 substeps. Sphere-space scaling (TRAP-P03). TOI >= 2 means no collision (TRAP-P01). |
| **Character state machine** | Player/NPC behavior: idle, walk, run, attack, block, dead | HIGH | ~11600 lines in game.cpp. Character struct holds sprite, anim, frame, pos, dir, HP. Human extends with equipment, stats, nutrition, talk. |
| **5D equipment sprite lookup** | Visual character customization: player[color][armor][helmet][shield][weapon] | MEDIUM | Enum-bounded: ACTION(5), WEAPON(3), SHIELD(2), HELMET(2), ARMOR(2), MOUNT(3). Mount changes physics size (TRAP-G02). |
| **Deferred sprite blit** | Correct sprite-over-terrain compositing; sorted far-to-near after RESOLVE | MEDIUM | Painter's algorithm. Sprites queued during world query, qsorted, blitted post-resolve. Sprite ref[3] uses half-cell units for sub-cell precision. |
| **Perspective camera** | Required by D004-D005; Q/E rotation toggle | MEDIUM | Projection/unprojection APIs. Scene shift multiplied by 2 in sample-buffer space (TRAP-R06). |
| **Resolve stage (2x2 downsample)** | Final ASCII output generation; branches on material vs mesh | HIGH | Per output cell: read 2x2 sample block, average height/diffuse, branch on spare bit 3 for material vs auto_mat path, apply grid/wireframe, write AnsiCell{fg,bk,gl,spare}. |
| **AnsiCell output format** | Final rendering target; fg/bk (xterm-256), gl (CP437 0-255), spare flags | LOW | Width*height cells, row-major. Glyph 255 = transparent. Spare 0xFF = debug rendered cell. |
| **Water rendering** | Reflective surface with Perlin Z-perturbation | MEDIUM | Reflection stage re-runs terrain+world below water plane. Mesh Z-coordinates near water boundary get special-cased (TRAP-R11). Perlin ripple applied at resolve. |
| **Font system** | Text rendering with CP437 glyphs; 3 skins (grey, gold, pink) | LOW | Y-axis inverted (sprite atlas bottom-up, text top-down) (TRAP-R09). Font atlas loaded once. |
| **GPU ASCII output** | Display the AnsiCell grid on screen via Bevy render plugin | HIGH | Mage Core 4-texture approach: char index + fg color + bg color + font atlas. Fragment shader combines all four per cell. Requires WGPU/WGSL integration as Bevy render plugin. |

### Differentiators (Competitive Advantage Over C++ Original)

Features that improve on the C++ engine. These are not required for compatibility but represent the value proposition of the Rust port.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **6D shape-vector glyph matching (Alex Harri k-d tree)** | Dramatically sharper ASCII rendering vs auto_mat's fixed LUT. Matches glyph shapes to image regions using 6D sampling vectors + k-d tree nearest-neighbor. Published Jan 2026, state of the art. | HIGH | Phase in after auto_mat works (D010). 6 sampling points per cell, normalized lightness vectors, k-d tree for O(log n) lookup. Replaces auto_mat glyph selection at RESOLVE stage; auto_mat still used for fg/bg color. Decision D040 (2D vs 6D) needs perf data. |
| **Bevy ECS architecture** | Modern data-oriented design vs C++ god-object Game struct. Better parallelism, testability, modularity. Eliminates global state dependency graph that plagues the C++ engine. | MEDIUM | C++ Game struct is ~11600 lines with tightly coupled physics/rendering/networking. ECS decomposition into Position, Sprite, Character, Physics components enables clean system separation. |
| **GPU-accelerated final output** | 100% GPU rendering after texture upload vs C++ terminal/ncurses output. Eliminates terminal bottleneck. Enables high-resolution ASCII at 60+ FPS. | HIGH | Mage Core proves the approach works (~500 lines of render.rs). Four RGBA8 textures (fg, bg, chars, font) + WGSL fragment shader. CPU rasterizer fills textures, GPU composites. |
| **Rust memory safety** | Eliminates entire classes of C++ bugs: dangling pointers (TRAP-E04, TRAP-W01), buffer overflows (TRAP-W04, TRAP-W07), use-after-free. No null pointer crashes. | LOW | Inherent to language choice. Borrow checker prevents TRAP-W01 (dangling BSP pointers), TRAP-E01 (Load invalidates all pointers), TRAP-G04 (pointer/index confusion). |
| **Hot reloading (assets)** | Faster iteration: reload .xp sprites and .a3d worlds without restart. C++ requires full restart or fragile F5 reload (TRAP-E04). | MEDIUM | Bevy has asset hot-reloading infrastructure. XP loader and A3D loader can integrate with Bevy's AssetServer for watch-based reloading. |
| **Fixed timestep physics via Bevy** | Deterministic physics without manual substep management. C++ uses hand-rolled 15ms fixed step with max 10 substeps. | LOW | Bevy provides FixedUpdate schedule. Port physics into FixedUpdate system, remove manual timestep accumulation. Physics becomes deterministic and framerate-independent. |
| **Parallel system execution** | Bevy's schedule automatically parallelizes independent systems. C++ is entirely single-threaded (except networking). | LOW | Terrain rendering, world queries, physics, and AI can run in parallel when data dependencies allow. Bevy's scheduler handles this automatically. |
| **Type-safe asset formats** | Rust enums + serde for asset loading vs C++ raw pointer casts and manual byte counting. Eliminates format version ambiguity (TRAP-W02). | MEDIUM | Define A3D, XP, AKM format structs with nom or binrw for declarative binary parsing. Format errors caught at parse time, not as runtime corruption. |
| **Ancestor cleanup (BSP)** | Fix stubbed code (TRAP-W11): collapse empty BSP leaves after instance removal. C++ accumulates dead nodes, degrading query performance. | MEDIUM | Decision D041. Well-defined fix: after SoftInstDel, walk parent chain and collapse single-child nodes. Improves query perf for dynamic worlds. |
| **Known C++ bug fixes** | Fix TERRAIN-001 through TERRAIN-004 (coordinate typos, comparison operator intent). These are verified bugs in the C++ source. | LOW | TERRAIN-001: `if(x)` should be `if(y)` at terrain.cpp:613. TERRAIN-002/003: `u < y` should be `u < v`. TERRAIN-004: verify `>` vs `>=` intent. |

### Anti-Features (Explicitly NOT Building in v1)

Features that seem important but create outsized complexity, are tangential to the core port, or should be deferred until core gameplay works.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Editor (asciiid)** | Complete port means all tools. Editor is 11,500 lines with ImGui integration, 7 edit modes, undo/redo, MCP protocol. | Doubles the codebase scope. Tightly coupled to OpenGL rendering context (not Bevy). Undo system (urdo.cpp) uses SWAP pattern that's non-trivial to port. Editor is a separate application, not gameplay. | Defer to post-v1. Use existing C++ editor for content creation. Port editor as separate milestone after game engine is complete. |
| **Web/WASM export** | Play in browser like the original Asciicker demo. | WGPU/WASM support is still maturing. WebSocket networking needs different stack. Performance constraints differ significantly. Would compromise desktop-first GPU rendering decisions. | Defer to v2. Desktop-first (Windows/Linux/macOS) per PROJECT.md. Can revisit after Bevy's WASM story matures. |
| **Mobile platform support** | Wider audience. | Touch input model fundamentally different from keyboard+mouse. Screen size constraints for ASCII rendering. Performance budgets differ. | Defer to v2+. Desktop-first. Touch input requires redesigned UI. |
| **Full multiplayer networking (day 1)** | Original has multiplayer. | C++ networking is ~3700 lines of custom binary protocol with platform-specific threading (Win32/pthread). WebSocket framing, token-based protocol. Adding networking before single-player works risks debugging two things at once. | Implement basic client-server after single-player gameplay is solid. Port protocol design, use tokio + tungstenite. |
| **GPU rasterization of 3D geometry** | Performance; GPU is faster than CPU for rasterization. | Breaks visual fidelity contract. CPU rasterizer produces the specific per-sample data (material indices, RGB555, spare flags, diffuse values) that the resolve stage depends on. GPU rasterization would need entirely different resolve logic. | CPU rasterizer for geometry (matching C++), GPU only for final ASCII output (D003). This is the decided architecture. |
| **Audio system (full 16-track mixer)** | Original has audio. | Audio is cosmetic, not structural. bevy_kira_audio integration is straightforward but shouldn't block core rendering/physics work. | Defer to late phase. Add basic sound effects after gameplay loop works. 16-track mixer is a nice-to-have. |
| **Weather effects** | Original has snow/blizzard particles. | Weather is purely visual, depends on complete rendering pipeline. C++ weather.cpp is only 449 lines but requires particle system on top of the ASCII renderer. | Defer to post-core. Implement after RESOLVE stage works and basic gameplay loop is complete. |
| **Enemy spawner system (enemygen)** | Original has it for gameplay. | 1150 lines, depends on character system, physics, AI, combat. Cannot test without all those systems working. | Defer to gameplay phase. Implement after character state machine and physics are working. |
| **Inventory system** | Original has it. | 3100 lines, tightly coupled to item instances, UI rendering, equipment changes. | Defer to gameplay phase. Implement after character system and basic interaction. |
| **Main menu / character creation** | Original has 2844-line mainmenu.cpp. | UI-heavy, depends on font system, input system, game state management. Not needed for core engine validation. | Defer to polish phase. Use developer console/hardcoded state for testing until core works. |
| **Gamepad support** | Original has 2318-line gamepad.cpp. | Platform-specific (Linux/Windows/macOS different APIs). Bevy has gamepad abstraction but mappings need per-game tuning. | Defer to input polish phase. Keyboard+mouse first. Bevy's gamepad API makes this easy to add later. |
| **Full Alex Harri 6D from day one** | Best visual quality. | Decision D010: need performance data first. 6D k-d tree lookup per resolve cell is more expensive than auto_mat LUT. Start with auto_mat (known-working), then phase in 2D shape vectors, then upgrade to 6D after profiling. | Phased approach: auto_mat first -> 2D vectors -> 6D vectors. Each step validates before proceeding. |

## Feature Dependencies

```
[XP Sprite Loading]
    +--requires--> [AnsiCell Format]
    +--requires--> [CP437 Glyph Set]

[A3D Terrain Loading]
    +--requires--> [Terrain Quadtree]
    +--requires--> [Material System]

[A3D World Loading]
    +--requires--> [BSP Tree]
    +--requires--> [Mesh Loading (.akm)]
    +--requires--> [XP Sprite Loading]

[6-Stage Rendering Pipeline]
    +--requires--> [SampleBuffer]
    +--requires--> [Terrain Quadtree] (Stage 2)
    +--requires--> [BSP Tree] (Stage 3)
    +--requires--> [Material System] (Stage 6: Resolve)
    +--requires--> [Rasterization Primitives]

[SampleBuffer]
    +--requires--> [AnsiCell Format]

[Resolve Stage]
    +--requires--> [SampleBuffer]
    +--requires--> [Material System (auto_mat)]
    +--requires--> [RGB555 -> xterm-256 Quantization]

[GPU ASCII Output]
    +--requires--> [Resolve Stage] (produces AnsiCell grid)
    +--requires--> [Bevy Render Plugin] (WGPU/WGSL)
    +--requires--> [Font Atlas (CP437)]

[6D Shape-Vector Matching]
    +--enhances--> [Resolve Stage] (replaces auto_mat glyph selection)
    +--requires--> [k-d Tree Implementation]
    +--requires--> [Font Atlas Sampling] (generate vectors per glyph)

[Sphere Physics]
    +--requires--> [Terrain Quadtree] (heightfield collision)
    +--requires--> [BSP Tree] (mesh collision)

[Character State Machine]
    +--requires--> [Sphere Physics]
    +--requires--> [XP Sprite Loading]
    +--requires--> [5D Equipment Lookup]

[Water Rendering]
    +--requires--> [Rendering Pipeline] (Reflection stage)
    +--requires--> [Terrain Quadtree] (water plane intersection)

[Deferred Sprite Blit]
    +--requires--> [Resolve Stage] (blits after resolve)
    +--requires--> [XP Sprite Loading]
    +--requires--> [BSP Tree] (world query queues sprites)

[Multiplayer Networking]
    +--requires--> [Character State Machine]
    +--requires--> [Physics System]

[Perspective Camera]
    +--requires--> [SampleBuffer] (projection math)
    +--requires--> [Rendering Pipeline]
```

### Dependency Notes

- **Rendering pipeline requires both spatial structures:** Terrain quadtree (stage 2) and BSP tree (stage 3) must both work before full pipeline can render.
- **Resolve stage is the critical integration point:** Everything flows through resolve. Material system, auto_mat, RGB555 quantization, and SampleBuffer all converge here. Get this right first.
- **GPU output is decoupled from CPU rasterization:** The GPU only needs the final AnsiCell grid. This means GPU output can be developed in parallel with CPU rasterizer using test data.
- **Physics depends on both spatial structures:** Sphere collision queries both terrain heightfield and BSP mesh geometry. Cannot test physics without both loading correctly.
- **6D shape vectors enhance but don't replace:** auto_mat still handles fg/bg color. Shape vectors only replace glyph selection. Both must coexist.
- **Character system depends on everything:** State machine needs physics (movement), sprites (visuals), equipment (5D lookup). This is necessarily a late-phase feature.

## MVP Definition

### Launch With (v1.0 -- "First Render")

Minimum viable: load an existing Asciicker world and render it correctly in a window.

- [ ] XP sprite file loading (gzip, CP437, 3+ layers, swoosh merge) -- binary compat
- [ ] A3D terrain file loading (AS3D magic, FilePatch 188 bytes, quadtree construction) -- binary compat
- [ ] A3D world file loading (format version, 3 instance variants, mesh stub pattern) -- binary compat
- [ ] AKM mesh geometry loading -- required by world loading (TRAP-W03)
- [ ] SampleBuffer with 2x supersampling and double-allocation clear -- rendering foundation
- [ ] Bresenham line + barycentric triangle rasterization -- geometry rendering
- [ ] Material system with auto_mat LUT (32KB RGB555 -> glyph+fg+bg) -- terrain shading
- [ ] RGB555 -> xterm-256 palette quantization -- color correctness
- [ ] 6-stage rendering pipeline (CLEAR through RESOLVE) -- full frame
- [ ] GPU ASCII output via Bevy render plugin (4-texture Mage Core approach) -- display
- [ ] Perspective camera with basic controls -- viewport
- [ ] Font atlas (CP437 glyph set) -- text/glyph rendering

### Add After First Render (v1.x -- "Playable")

Features that make it interactive, triggered by successful first render.

- [ ] Sphere-based physics (TOI sweep, face/edge/vertex collision) -- movement
- [ ] Character state machine (idle, walk, run, attack, block, dead) -- gameplay
- [ ] 5D equipment sprite lookup -- character customization
- [ ] Deferred sprite blit (post-resolve, far-to-near sort) -- sprite compositing
- [ ] Water rendering with reflection stage -- visual completeness
- [ ] Terrain shadow casting (64-bit bitmask per patch) -- lighting
- [ ] Player input system (keyboard + mouse) -- interaction
- [ ] Basic game loop (spawn player, move, interact with world) -- playability
- [ ] 6D shape-vector glyph matching (phased: 2D first, then 6D) -- visual upgrade

### Add After Playable (v1.x+ -- "Complete Game")

Features to defer until core loop works.

- [ ] Basic multiplayer networking (client-server, binary protocol) -- defer: debug single-player first
- [ ] Audio system via bevy_kira_audio -- defer: cosmetic, straightforward later
- [ ] Weather effects (rain, snow, blizzard particles) -- defer: purely visual
- [ ] Enemy spawner system -- defer: depends on full character/combat system
- [ ] Inventory system -- defer: 3100 lines, needs UI
- [ ] Main menu and game state management -- defer: UI-heavy, use dev tools initially
- [ ] NPC AI and combat -- defer: depends on character + physics + pathfinding

### Future Consideration (v2+)

Features to explicitly skip until after v1 ships.

- [ ] Editor (asciiid) -- defer: 11,500 lines, separate application, use C++ editor
- [ ] Web/WASM export -- defer: platform story immature, desktop-first
- [ ] Mobile support -- defer: fundamentally different input/display model
- [ ] Gamepad support -- defer: easy to add via Bevy, not critical path
- [ ] Full 6D shape vectors if 2D proves sufficient -- defer: needs perf data (D040)

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| XP sprite loading | HIGH | MEDIUM | P1 |
| A3D terrain loading | HIGH | MEDIUM | P1 |
| A3D world loading | HIGH | HIGH | P1 |
| SampleBuffer | HIGH | MEDIUM | P1 |
| Rasterization primitives | HIGH | MEDIUM | P1 |
| Material system (auto_mat) | HIGH | MEDIUM | P1 |
| RGB555 quantization | HIGH | LOW | P1 |
| 6-stage render pipeline | HIGH | HIGH | P1 |
| Resolve stage | HIGH | HIGH | P1 |
| GPU ASCII output | HIGH | HIGH | P1 |
| Perspective camera | HIGH | MEDIUM | P1 |
| Sphere physics | HIGH | HIGH | P2 |
| Character state machine | HIGH | HIGH | P2 |
| 5D equipment lookup | MEDIUM | MEDIUM | P2 |
| Deferred sprite blit | MEDIUM | MEDIUM | P2 |
| Water rendering | MEDIUM | MEDIUM | P2 |
| 6D shape-vector matching | HIGH | HIGH | P2 |
| Player input | HIGH | LOW | P2 |
| Basic multiplayer | MEDIUM | HIGH | P3 |
| Audio | LOW | LOW | P3 |
| Weather effects | LOW | MEDIUM | P3 |
| Enemy spawner | MEDIUM | MEDIUM | P3 |
| Inventory | MEDIUM | HIGH | P3 |
| Main menu | LOW | MEDIUM | P3 |
| Editor | LOW | HIGH | P4 |
| Web export | LOW | HIGH | P4 |

**Priority key:**
- P1: Must have for first render (prove the port works)
- P2: Must have for playable game (interactive, visually complete)
- P3: Should have for complete game (feature parity with C++)
- P4: Nice to have, future consideration (beyond v1 scope)

## Competitor Feature Analysis

| Feature | C++ Asciicker (Original) | bevy_ascii_terminal | Mage Core | SadConsole (.NET) | Our Approach |
|---------|--------------------------|--------------------|-----------|--------------------|-------------|
| 3D world rendering | CPU rasterizer, 6-stage pipeline | N/A (2D grid only) | N/A (2D grid only) | N/A (2D grid only) | Port CPU rasterizer, unique to Asciicker |
| Glyph selection | auto_mat LUT (RGB555 -> glyph) | Manual per-cell | Manual per-cell | Manual per-cell | auto_mat + 6D shape vectors (best of both) |
| GPU output | None (terminal) | Bevy sprite batching | 4-texture WGPU shader | OpenGL/MonoGame | 4-texture Bevy render plugin (Mage Core approach) |
| Color model | xterm-256 palette | RGB per cell | RGBA per cell | RGBA per cell | xterm-256 for fidelity, RGBA in GPU |
| Physics | Custom sphere-based TOI | None | None | None | Port sphere physics, unique to Asciicker |
| Spatial partitioning | BSP tree + quadtree | None | None | None | Port both, unique to Asciicker |
| Multiplayer | Custom binary WebSocket | None | None | None | Tokio + tungstenite, improved over C++ |
| Asset format | Custom .xp + .a3d + .akm | Built-in tile maps | Custom API | Built-in console format | Binary-compatible loaders |
| Architecture | God object (Game struct) | Bevy ECS | Custom app trait | Component-based | Bevy ECS (major improvement) |

**Key insight:** No existing ASCII game engine attempts what Asciicker does. Traditional ASCII engines (bevy_ascii_terminal, SadConsole, etc.) are 2D grid renderers for roguelikes. Asciicker is a 3D game engine that outputs to ASCII. The port has no direct competitors -- it competes with the C++ original.

## Sources

- C++ source analysis: `(ORIGINAL GAME)asciicker-Y9-2-main/` (~82K lines, 48 files) -- HIGH confidence
- Skill packs: engine-render.md, world-loading.md, physics-system/SKILL.md, game-mechanics/SKILL.md, networking/SKILL.md, editor-asciiid.md -- HIGH confidence
- Architecture audit: HANDOFF_ENGINE_AUDIT.md (file inventory, 34 analysis outputs) -- HIGH confidence
- Mage Core reference: `../reference/Mage-core/` (~2000 lines Rust) -- HIGH confidence (direct code review)
- Alex Harri shape vectors: https://alexharri.com/blog/ascii-rendering (Jan 2026) -- HIGH confidence (published technique)
- Alex Harri implementation: `../reference/alexharri-ascii/` (TypeScript) -- HIGH confidence (direct code review)
- bevy_ascii_terminal: https://github.com/sarkahn/bevy_ascii_terminal (v0.18.2, Feb 2026) -- MEDIUM confidence (web search, not code review)
- Mage Core crate: https://github.com/baad-c0de/mage-core -- MEDIUM confidence (web search + code review)
- PROJECT.md decisions: D001 (Bevy), D003 (CPU rasterizer), D004-D005 (perspective), D010 (auto_mat first), D012 (shape-match in RESOLVE), D040/D041 (pending) -- HIGH confidence

---
*Feature research for: Asciicker C++ -> Rust/Bevy ASCII game engine port*
*Researched: 2026-02-20*
