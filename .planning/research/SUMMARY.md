# Project Research Summary

**Project:** Asciicker Rust Port (ASCII game engine — C++ to Rust/Bevy)
**Domain:** Custom 3D CPU software rasterizer with GPU ASCII terminal output
**Researched:** 2026-02-20
**Confidence:** MEDIUM-HIGH

## Executive Summary

Asciicker is a 3D game engine that renders its world entirely through ASCII characters, implemented in 82,000 lines of C++. Unlike roguelike ASCII engines (which are 2D grid renderers), Asciicker runs a full 6-stage CPU rasterization pipeline — CLEAR, TERRAIN, WORLD, SHADOW, REFLECTION, RESOLVE — that produces a grid of AnsiCells (character + fg/bg color), then uploads those cells to the GPU for display. The Rust port targets Bevy 0.18 as its ECS/windowing/input backbone while replacing Bevy's built-in renderer entirely with a custom CPU rasterizer feeding a Mage Core-style 4-texture WGSL shader. No existing ASCII engine framework attempts 3D rendering at this level; the port has no direct competitor except the C++ original.

The recommended approach is a strict layered build order: foundation data structures and asset parsers first (pure logic, testable in isolation), then CPU rasterizer core, then GPU output in parallel, then integration of the full pipeline, then physics and character systems, and finally polish and networking. The GPU output plugin can be developed with synthetic test data immediately, independently of the CPU rasterizer — this parallelism is the single biggest schedule accelerant. Visual fidelity against the C++ engine is the primary correctness criterion; the project should use golden-file snapshot tests of AnsiCell output from day one to detect regressions automatically.

The two most consequential risks are: (1) Bevy's dual-world extraction architecture being implemented incorrectly, causing silent render desync that is hard to debug once the pipeline is complex; and (2) performance death in the CPU rasterizer inner loop from accumulated small inefficiencies (bounds checks, allocations, cache misses) that are catastrophic at 60fps but invisible in isolated unit tests. Both risks are addressed by architectural decisions made in Phase 1: establish the Bevy render plugin skeleton with unconditional extraction, and pre-allocate all rasterizer buffers at startup. The existing skeleton in the repository has structural misalignments (wrong crate type, wrong audio version, missing plugin architecture) and should be restructured rather than salvaged.

## Key Findings

### Recommended Stack

Bevy 0.18 is used with `default-features = false` and a custom feature set (`2d_api`, `bevy_render`, `bevy_core_pipeline`, `bevy_shader`) that provides ECS, input, windowing, and wgpu access while excluding Bevy's built-in 2D/3D renderers. The wgpu GPU backend (v27, bundled with Bevy) is accessed exclusively through Bevy's render world — never as a direct dependency. The CPU rasterizer and all spatial data structures are hand-rolled to match C++ behavior exactly; no generic rasterizer crate (euc, rust-softrender) is appropriate because visual fidelity requires matching the C++ engine's specific edge-case behaviors.

Two critical fixes to the existing skeleton are required before any other work: `bevy_kira_audio` must be updated from `"0.24"` (Bevy 0.17 compatible) to `"0.25"` (Bevy 0.18 compatible), and the `crate-type = ["cdylib", "rlib"]` must be replaced with a `[[bin]]` target.

**Core technologies:**
- **Bevy 0.18** (ECS, input, windowing, wgpu access) — `default-features = false` with `2d_api` feature; provides everything except Bevy's built-in renderer, which the ASCII engine replaces
- **wgpu 27** (GPU abstraction) — accessed through `bevy_render`, never as a direct dependency; powers the 4-texture WGSL shader for ASCII output
- **nom 8.0** (binary parsing) — combinator-based parser for the complex .a3d format (variable-length sections, BSP tree, version headers); manual parsing acceptable only for .xp
- **flate2 1.1** (gzip decompression) — .xp sprite files are gzip-wrapped; pure Rust via miniz_oxide backend
- **bytemuck 1.24** (zero-copy type casting) — GPU texture uploads; `cast_slice()` for writing u32 RGBA to texture storage
- **kiddo 5.2** (k-d tree) — `ImmutableKdTree` built once at startup from 256 CP437 glyph shape vectors; used for Alex Harri nearest-neighbor glyph matching in RESOLVE stage
- **rayon 1.11** (data parallelism) — terrain patch rasterization is embarrassingly parallel; `par_iter()` for the outer terrain loop only
- **bevy_kira_audio 0.25** (audio) — 16-track mixer matching the C++ engine's audio architecture; must disable Bevy's built-in `bevy_audio`
- **lightyear 0.24.x** (networking, Phase 6+) — entity replication, client-side prediction, rollback; MEDIUM confidence on Bevy 0.18 compatibility, verify before adding
- **insta 1.39 + cargo-insta** (snapshot testing) — snapshot SampleBuffer and AsciiCell output for visual regression CI
- **thiserror 2.0 / anyhow 1.0** (error handling) — thiserror for library error types; anyhow for application-level propagation

**What to avoid:** `log`/`env_logger` (Bevy uses `tracing`), direct `wgpu` or `winit` dependencies, generic CPU rasterizer crates, `tokio` (use `bevy::tasks`), `serde_json` (not needed for binary formats).

### Expected Features

The port's correctness bar is binary compatibility with existing .xp sprite and .a3d world files. Every table-stakes feature in the list below must be implemented for the port to render an existing Asciicker scene correctly.

**Must have — v1 "First Render" (P1):**
- XP sprite loading (gzip decompression, CP437 charset, column-major layout, 3+ layer semantics, swoosh merge on last layer)
- A3D terrain loading (AS3D magic `0x44335341`, 188-byte FilePatch, HEIGHT_SCALE=16, quadtree construction)
- A3D world loading (format version detection, 3 instance variants keyed by mesh_id_len, LoadWorld/UpdateMesh/RebuildWorld call order)
- SampleBuffer with 2x supersampling and double-allocation for fast memcpy clear
- Bresenham line + barycentric triangle rasterization (template-style duck-typed shaders)
- 6-stage CPU rendering pipeline (CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE) in exact order
- Material system with auto_mat LUT (32KB RGB555 -> glyph + fg/bg)
- RGB555 -> xterm-256 color quantization
- RESOLVE stage (2x2 downsample, per-cell glyph and color selection, silhouette detection)
- GPU ASCII output via Bevy render plugin (Mage Core 4-texture approach: char index + fg + bg + font atlas)
- Perspective camera with basic controls
- Font atlas (CP437 16x16 glyph grid loaded as Bevy PNG asset)

**Must have — v1.x "Playable" (P2):**
- Sphere-based physics (TOI sweep, face/edge/vertex collision, 15ms fixed timestep, max 10 substeps)
- Character state machine (idle, walk, run, attack, block, dead; ~11,600 lines in C++)
- 5D equipment sprite lookup (ACTION x WEAPON x SHIELD x HELMET x ARMOR x MOUNT)
- Deferred sprite blit (post-RESOLVE, painter's algorithm far-to-near sort)
- Water rendering (reflection stage, Perlin Z-perturbation)
- Terrain shadow casting (64-bit bitmask per patch)
- Player input system (keyboard + mouse)
- 6D Alex Harri shape-vector glyph matching (phased: auto_mat first, then 2D, then 6D after profiling)

**Should have — v1.x+ "Complete Game" (P3):**
- Basic multiplayer networking (client-server, binary WebSocket protocol)
- Audio system (bevy_kira_audio 16-track mixer)
- Weather effects (rain, snow, blizzard particles)
- Enemy spawner system (depends on full character/combat system)
- Inventory system (3,100 lines in C++; needs UI)
- Main menu and game state management

**Defer to v2+:**
- Editor (asciiid, 11,500 lines, separate application; use C++ editor for content creation)
- Web/WASM export (GPU/WASM story still maturing; desktop-first)
- Mobile support (fundamentally different input/display model)
- Gamepad support (Bevy makes this easy to add; not critical path)
- Full 6D shape vectors if 2D proves sufficient (needs performance data)

**Key insight from feature analysis:** no existing ASCII engine framework attempts 3D rendering. Traditional ASCII engines (bevy_ascii_terminal, SadConsole) are 2D roguelike grids. The GPU output and character system are unique to Asciicker.

### Architecture Approach

The architecture follows a strict plugin-per-subsystem pattern in Bevy, with a clean separation between: (a) game-world data managed as ECS entities and Resources in the Main World, (b) the CPU rasterizer running as ordered Bevy systems within the Main World writing to a `SampleBuffer` Resource then a `AsciiCellGrid` Resource, and (c) a GPU output plugin living entirely in the Render World that extracts the `AsciiCellGrid` each frame and uploads 3 textures (char index, fg color, bg color) for a fullscreen WGSL shader pass. This Main World / Render World split is non-negotiable for correct Bevy integration. The Mage Core reference implementation's 4-texture WGSL shader (`shader.wgsl`) is directly portable to Bevy — only the texture binding mechanism changes.

**Major components:**
1. **AssetLoaderPlugin** — async `.xp` and `.a3d` binary format parsers using Bevy's `AssetLoader` trait; produces typed Rust assets, independent of ECS logic
2. **WorldPlugin** — BSP tree (Resource) + terrain quadtree (Resource) + mesh/sprite/item instance entities; the scene database that both physics and rendering query
3. **CPU Rasterizer Plugin** — 6 Bevy systems chained in order (CLEAR through RESOLVE) writing to `SampleBuffer` Resource then `AsciiCellGrid` Resource; largest module (~4,400 C++ lines)
4. **ASCII Output Plugin** — Bevy render world plugin: Extract copies `AsciiCellGrid`, Prepare uploads 3 GPU textures, Render executes fullscreen WGSL shader with font atlas
5. **PhysicsPlugin** — sphere sweep TOI collision querying BSP tree and terrain heightmap; writes entity positions; reads from WorldPlugin
6. **CharacterPlugin** — state machine (idle/walk/run/attack/block/dead), 5D equipment sprite lookup, animation; depends on PhysicsPlugin
7. **GamePlugin** — top-level state machine (Loading -> Playing -> Paused); orchestrates frame

**Recommended module structure:** `src/assets/`, `src/world/`, `src/physics/`, `src/character/`, `src/rendering/` (CPU rasterizer, largest module), `src/gpu_output/` (Bevy render plugin), `src/plugins/`. Each maps to a clear domain boundary.

**Critical architecture rule:** the existing skeleton's `Arc<Sprite>` pattern must be replaced with `Handle<XpSprite>` via Bevy's asset system. The skeleton's wildcard re-exports and lack of plugin structure are anti-patterns.

### Critical Pitfalls

1. **Bevy Render World desync** — if the `AsciiCellGrid` extraction from Main World to Render World is conditional or incomplete, the retained render world holds stale data, causing visual artifacts or crashes (Bevy issue #15871). Avoid by extracting the full buffer unconditionally every frame. The ~97KB copy at 240x135 resolution is negligible. Establish correct extraction in Phase 1 before building the rasterizer.

2. **1:1 C++ line-for-line translation** — the 82K-line C++ codebase uses global mutable pointers (`terrain`, `world`, `renderer`). Porting these as `static mut` or `Arc<Mutex<>>` defeats ECS parallelism and fights the borrow checker. Warning signs: more than 5 `unsafe` blocks outside SIMD/FFI, any `static mut`, systems exceeding 800 lines. Map C++ globals to Bevy Resources; map C++ struct arrays to ECS entities.

3. **Rasterizer performance death** — the inner rasterization loop at 60fps/1080p allows 32ns per sample. Bounds checking, per-sample heap allocations, and f32->u8 conversions accumulate to miss this budget. Pre-allocate all buffers at startup; use `unsafe get_unchecked()` in the inner loop only after proving bounds via the triangle bounding-box clip; profile with Tracy before optimizing.

4. **Binary format parsing with unsafe transmute** — the XP format has 10-byte non-aligned cells in column-major order; A3D has variable-length sections and version headers. Using `transmute` introduces UB on padding/endianness differences. Use `nom` for A3D, `zerocopy` with `Unalign<T>` for XP cells, and write golden-file tests against known C++ output immediately.

5. **Floating-point divergence from C++** — pixel-for-pixel identical output is likely impossible due to FMA differences, x87 vs SSE2 precision, and expression ordering. Accept "perceptually identical" (< 1% cells differ, differences confined to triangle edges). Use integer arithmetic for edge functions and the RGB555 pipeline; establish golden-file CI comparison in Phase 3.

6. **Bevy version churn** — Bevy is pre-1.0 and releases breaking render API changes every ~3 months. Pin to `bevy = "0.18.0"` (not `^0.18`). Isolate all Bevy render API calls behind an `AsciiRenderBackend` abstraction so that version upgrades only touch one module.

## Implications for Roadmap

Based on research, suggested phase structure (7 phases):

### Phase 1: Foundation (Skeleton + Bevy Plugin Architecture)
**Rationale:** Architecture decisions made here are the hardest to undo. The existing skeleton has structural problems (wrong crate type, wrong audio version, no plugin architecture, wildcard re-exports) that poison every subsequent phase if not fixed first. Bevy version must be pinned and render abstraction established before GPU work begins.
**Delivers:** Compiling project with correct Cargo.toml, plugin-per-subsystem skeleton, coordinate system convention documented, ECS resource/entity mapping document, Bevy 0.18.0 pinned, render world abstraction stub.
**Addresses:** XP/A3D asset handle pattern (vs raw pointers in skeleton)
**Avoids:** Pitfall 1 (render world desync), Pitfall 2 (1:1 translation), Pitfall 6 (Bevy version churn), Pitfall 7 (over-componentized ECS), Pitfall 9 (coordinate system confusion)
**Research flag:** Standard Bevy patterns — skip `/gsd:research-phase`, patterns well-documented.

### Phase 2: Asset Parsers (XP + A3D Binary Loaders)
**Rationale:** Asset loading is a pure input-output problem with no ECS or rendering dependencies. Parsers can be written and tested in complete isolation with unit tests and golden files before any rendering work begins. Incorrect parsers corrupt all downstream output invisibly.
**Delivers:** Working `.xp` loader (gzip, column-major, layer semantics, swoosh merge) and `.a3d` loader (terrain patches, world instances, BSP structure) integrated with Bevy's async `AssetLoader` trait. Golden-file tests pass for known C++ test assets.
**Uses:** nom 8.0, flate2 1.1, bytemuck 1.24 (zerocopy for XP cells)
**Implements:** AssetLoaderPlugin
**Avoids:** Pitfall 3 (binary parsing with unsafe transmute), Pitfall 10 (asset loading lifecycle)
**Research flag:** nom and Bevy `AssetLoader` API may need spot-checking against 0.18 docs; otherwise well-understood.

### Phase 3: GPU Output Plugin (Independent of CPU Rasterizer)
**Rationale:** The GPU output plugin only needs a synthetic `AsciiCellGrid` to test. It can be built in parallel with (or before) the CPU rasterizer because the interface between them is a simple flat array of AsciiCells. Getting pixels on screen early provides a critical feedback loop and validates the Bevy render world integration before the rasterizer adds complexity.
**Delivers:** Fullscreen WGSL shader (ported from Mage Core) displaying glyphs from a synthetic test pattern. Font atlas loaded as Bevy PNG asset. Correct Extract/Prepare/Render world pipeline. Window resize handling.
**Uses:** bevy_render (wgpu 27), bytemuck, WGSL shader (Mage Core port), png feature
**Implements:** ASCII Output Plugin (gpu_output/ module)
**Avoids:** Pitfall 1 (render world desync — established correctly here first), Pitfall 2 (bypassing render world)
**Research flag:** `FullscreenMaterial` vs raw `ViewNode` decision needs evaluation at implementation time; the Bevy 0.18 `custom-post-processing` example is the reference.

### Phase 4: CPU Rasterizer Core (SampleBuffer + Geometry)
**Rationale:** The rasterizer is the largest single subsystem (~4,400 C++ lines). Breaking it into an isolated phase ensures it is fully tested before integration. Performance baselines must be established here before integration hides bottlenecks.
**Delivers:** SampleBuffer data structure (2x supersampled, double-allocation clear), barycentric triangle rasterizer, Bresenham line rasterizer, material system (auto_mat LUT), RGB555/xterm-256 color quantization, RESOLVE stage (2x2 downsample, glyph/color selection). Renders hard-coded geometry at > 60fps at 240x135. Golden-file tests show < 1% cell difference from C++ output on canonical scenes.
**Uses:** rayon 1.11 (terrain parallelism), kiddo 5.2 (glyph k-d tree, phase-in after auto_mat works), insta (snapshot testing), proptest (rasterizer invariants)
**Implements:** rendering/ module (sample_buffer, rasterize, resolve, color, material)
**Avoids:** Pitfall 4 (performance death), Pitfall 5 (floating-point divergence), Pitfall 8 (system ordering)
**Research flag:** SIMD optimization paths (if profiling reveals bottleneck in inner loop); otherwise standard Rust performance patterns apply.

### Phase 5: Pipeline Integration (First Real Scene)
**Rationale:** This is the integration phase where Phase 2 (asset parsers), Phase 3 (GPU output), and Phase 4 (CPU rasterizer) connect. The terrain quadtree and BSP tree are wired to the rasterizer's CLEAR/TERRAIN/WORLD pipeline stages, and the full RESOLVE -> AsciiCellGrid -> GPU path produces real rendered output from an actual .a3d world file.
**Delivers:** Full 6-stage rendering pipeline rendering a real Asciicker world file in a window. Perspective camera with Q/E rotation. Scene matches C++ engine visually. Terrain, mesh instances, and sprites all render. Deferred sprite blit post-RESOLVE.
**Uses:** All of Phases 2, 3, 4 together; WorldPlugin (BSP + quadtree) wired to CPU rasterizer
**Implements:** WorldPlugin, TerrainSystem, complete CpuRasterizerPlugin with system ordering
**Avoids:** Pitfall 8 (system ordering — pipeline stages chained), Pitfall 5 (floating-point — golden files verified)
**Research flag:** BSP tree SAH construction and frustum-culled traversal may need research into Bevy 0.18 query patterns for large entity counts.

### Phase 6: Physics + Character System (Playable)
**Rationale:** Physics and the character state machine depend on all rendering and world-loading infrastructure being solid. Debugging physics bugs while the renderer is also uncertain produces unresolvable confusion. Implementing physics after the first real render gives a stable platform.
**Delivers:** Sphere-based TOI collision physics, gravity, character state machine (idle/walk/run/attack/block/dead), 5D equipment sprite lookup, player input system, water reflection, terrain shadows. Playable single-player loop.
**Implements:** PhysicsPlugin, CharacterPlugin, InputPlugin (complete)
**Avoids:** TRAP-P01 (TOI >= 2 means no collision), TRAP-P03 (sphere-space scaling), TRAP-G02 (mount changes physics size), coordinate system Z-up throughout
**Research flag:** Sphere TOI sweep implementation is non-trivial (~2,350 C++ lines); skill pack SKILL.md in physics-system/ is the primary reference. May need dedicated research phase.

### Phase 7: Systems + Polish (Complete Game)
**Rationale:** Audio, networking, weather, inventory, and the main menu are all independently addable after the core gameplay loop works. Each is isolated and can proceed in any order within this phase.
**Delivers:** bevy_kira_audio 16-track audio, basic client-server multiplayer (lightyear or tokio+tungstenite), weather particle effects, enemy spawner, inventory system, main menu.
**Uses:** bevy_kira_audio 0.25, lightyear 0.24.x (verify Bevy 0.18 compat before adding)
**Avoids:** Debugging networking before single-player works; audio blocking core rendering work
**Research flag:** lightyear 0.24.x Bevy 0.18 compatibility must be verified at implementation time (MEDIUM confidence). Multiplayer protocol port from C++ needs dedicated research.

### Phase Ordering Rationale

- Phases 1-2 are strictly sequential: skeleton must be correct before any parser or plugin is written, and parsers must exist before anything can load assets.
- Phases 3 and 4 can run in parallel once Phase 1 is complete — GPU output and CPU rasterizer are decoupled; this is the primary schedule accelerant.
- Phase 5 integration is blocked by all of Phases 2, 3, and 4; its gate is "all inputs ready."
- Phases 6 and 7 respect the dependency graph: physics requires the world data structures and renderer to be debuggable, and the complete-game systems require a working gameplay loop.
- This ordering concentrates the highest-risk architectural decisions (render world, ECS mapping, binary parsing) in the first two phases where rewrites are cheapest.

### Research Flags

Phases likely needing `/gsd:research-phase` during planning:
- **Phase 3 (GPU Output):** Evaluate `FullscreenMaterial` vs raw `ViewNode` in Bevy 0.18; verify `ExtractSchedule` API against actual 0.18 docs.
- **Phase 6 (Physics):** Sphere TOI sweep is 2,350 lines of C++ — a dedicated research pass on the Rust port strategy is warranted before implementation begins.
- **Phase 7 (Networking):** lightyear 0.24.x Bevy 0.18 compatibility must be verified; C++ multiplayer protocol is custom and needs mapping to lightyear's replication model.

Phases with well-documented patterns (skip research-phase):
- **Phase 1 (Foundation):** Standard Bevy plugin architecture, fully documented in Bevy cheat book and examples.
- **Phase 2 (Asset Parsers):** nom 8.0 binary parsing and Bevy `AssetLoader` are mature, well-documented patterns.
- **Phase 4 (CPU Rasterizer):** Pure algorithmic port from C++ with clear reference source; no framework integration uncertainty.
- **Phase 5 (Integration):** Wiring already-built components; complexity is coordination, not unknown APIs.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Bevy 0.18 confirmed via official release notes; wgpu 27 bundled version verified; bevy_kira_audio 0.25 compatibility confirmed via GitHub table; nom/kiddo/rayon/bytemuck all verified via docs.rs. lightyear is MEDIUM (check version before adding). |
| Features | HIGH | Based on direct C++ source analysis (82K lines), skill packs, and reference implementations. Feature list is complete and prioritized. MVP definition is clear. |
| Architecture | MEDIUM | Core ECS patterns and Mage Core integration approach are HIGH (verified from source). Bevy 0.18 render pipeline details are MEDIUM — some unofficial Bevy Cheat Book sources; verify Extract/Prepare/Render API signatures against actual 0.18 docs at implementation time. |
| Pitfalls | MEDIUM-HIGH | Domain-specific risks verified against project docs, Bevy community issues, and code review of C++ source. Floating-point determinism risk is well-documented in external references. Performance numbers are estimates pending actual profiling. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **lightyear 0.24.x + Bevy 0.18 compatibility:** Verify GitHub releases before adding to Cargo.toml. Do not add until Phase 7.
- **FullscreenMaterial vs ViewNode decision (Phase 3):** Evaluate at implementation time whether `FullscreenMaterial` (new in Bevy 0.18) provides sufficient bind group control for 4 textures, or whether raw `ViewNode` is required.
- **Floating-point fidelity tolerance:** "Perceptually identical" needs a concrete definition (< 1% cells differ, differences at triangle edges). Establish this as a CI threshold in Phase 4 by running canonical test scenes against C++ output. The exact achievable tolerance is unknown until first golden files are generated.
- **Alex Harri 6D vs 2D shape vectors performance decision (D040):** Cannot be resolved without profiling data. Phase 4 auto_mat baseline, then Phase 5 2D shape vector test, then decide whether 6D improvement justifies cost.
- **BSP ancestor cleanup (D041):** Stubbed in C++; fixing it is a known improvement but complexity is uncertain. Defer decision to Phase 5 when BSP is implemented and query performance is measurable.
- **SampleBuffer copy size at higher resolutions:** At 1080p, the extracted AsciiCellGrid is ~540KB per frame. At Phase 5, evaluate whether double-buffering or GPU-accessible shared memory is needed, or whether the copy is still acceptable.

## Sources

### Primary (HIGH confidence)
- Bevy 0.18 official release notes — feature collections, 2d_api, FullscreenMaterial, wgpu 27
- Bevy 0.18 docs.rs feature flags — complete feature list for custom feature configuration
- Bevy custom post-processing example — ViewNode + fullscreen shader pattern
- Mage Core source (local, MIT licensed) — 4-texture ASCII rendering in wgpu, shader.wgsl, render.rs
- Mage Core shader.wgsl — WGSL fullscreen quad with font atlas compositing (direct code review)
- C++ Asciicker source (~82K lines, (ORIGINAL GAME)asciicker-Y9-2-main/) — feature/architecture ground truth
- C++ skill packs (engine-render.md, world-loading.md, physics-system/SKILL.md, game-mechanics/SKILL.md) — trap catalogue
- HANDOFF_ENGINE_AUDIT.md — file inventory, 34 analysis outputs, architecture audit
- Alex Harri shape vectors (alexharri.com/blog/ascii-rendering, Jan 2026) — 6D glyph matching technique
- Alex Harri TypeScript implementation (local, direct code review) — shape vector computation reference
- bevy_kira_audio GitHub compatibility table — version 0.25 = Bevy 0.18
- kiddo 5.2 docs.rs — ImmutableKdTree, rkyv serialization
- nom 8.0 docs.rs — trait-based combinator API
- rayon 1.11 docs.rs — par_iter patterns
- PROJECT.md decisions (D001, D003, D004, D005, D010, D012, D040, D041)

### Secondary (MEDIUM confidence)
- Unofficial Bevy Cheat Book (bevy-cheatbook.github.io) — Main World/Render World, Extract, system sets, plugins; confirmed against multiple sources
- Bevy GitHub issue #15871 — conditional extraction desync (render world)
- Bevy GitHub issue #16414 — version churn documentation
- Bevy 0.17->0.18 migration guide — breaking changes
- lightyear GitHub — feature set, Bevy compatibility (version needs re-verification at implementation time)

### Tertiary (LOW confidence)
- DeepWiki Bevy render pipeline architecture — pipeline specialization details; not fully verified
- DeepWiki Bevy asset system — AssetLoader async details; not fully verified
- Bevy 0.17->0.18 migration guide — only partially read; verify specific API signatures before implementation

---
*Research completed: 2026-02-20*
*Ready for roadmap: yes*
