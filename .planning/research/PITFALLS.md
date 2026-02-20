# Pitfalls Research

**Domain:** C++-to-Rust game engine port (custom CPU software rasterizer to Bevy ECS)
**Researched:** 2026-02-20
**Confidence:** MEDIUM-HIGH (domain-specific risks verified against project docs, Bevy docs, and community post-mortems; some items based on training data alone flagged as LOW)

---

## Critical Pitfalls

Mistakes that cause rewrites, multi-week delays, or fundamental architecture failures.

### Pitfall 1: Bevy Main World / Render World Desync

**What goes wrong:**
Bevy uses a dual-world architecture: a Main App World for simulation and a separate Render World for GPU work. Data must be explicitly "extracted" from Main to Render each frame. In the retained render world (Bevy 0.15+), render entities persist across frames instead of being wiped. If extraction is conditional (e.g., only extracting when a value is non-zero), components that were previously extracted are never removed from the render world when conditions change. This causes the render world to hold stale data, producing visual artifacts, crashes (as with the Bloom HDR-off crash in Bevy issue #15871), or silent rendering errors.

For this project, the custom ASCII render plugin must extract the CPU-rasterized SampleBuffer, the resolved AnsiCell grid, and camera state from the main world into the render world every frame. Any conditional extraction logic risks desync.

**Why it happens:**
Developers coming from single-world engines (or from the C++ codebase where rendering directly reads simulation state) assume the render world has the same data as the main world. The extraction boilerplate feels redundant, so developers add conditionals to "optimize" it, not realizing the retained render world keeps old data.

**How to avoid:**
- Always extract unconditionally. If a component is removed from the main world, the extraction system must also remove it from the render world.
- Use `ExtractComponentPlugin` with the `ExtractComponent` derive macro for straightforward component mirroring.
- For the ASCII render plugin: extract the full `AsciiOutputBuffer` resource every frame, never conditionally. The buffer is small (240x135 x 3 bytes = ~97KB) -- copying is cheap.
- Store frame-persistent render data (font atlas texture handle, pipeline cache) as Render World Resources, not as extracted components.
- During the Extract stage, nothing else runs in parallel. Keep extraction minimal: copy data, do not compute.

**Warning signs:**
- Render output flickers or shows stale frames when game state changes
- Crash on component removal (e.g., disabling a post-processing effect)
- Render world entity count growing unbounded across frames
- Visual state "lags" one frame behind simulation

**Phase to address:**
Phase 1 (Foundation) -- establish the Bevy render plugin skeleton with correct extraction from day one. This is architectural; fixing it later requires rewriting the render plugin.

---

### Pitfall 2: 1:1 C++ Translation Instead of Idiomatic Rust/ECS

**What goes wrong:**
Developers port C++ code line-by-line into Rust, preserving the original's global mutable state, pointer-heavy data structures, and imperative control flow. The resulting code fights the borrow checker at every turn, uses excessive `unsafe`, `Rc<RefCell<>>`, or `Arc<Mutex<>>` to work around ownership, and fails to leverage ECS parallelism. The Asciicker C++ codebase is DOD (Data-Oriented Design) with global pointers (`terrain`, `world`, `renderer`) -- porting these as global `Mutex<>` resources defeats ECS benefits.

**Why it happens:**
The C++ codebase is 82K lines across 48 files. The temptation to "just make it compile" by mechanically translating is strong, especially when the C++ code already uses DOD patterns. But C++ DOD with raw pointers is fundamentally different from Rust ownership + ECS queries.

**How to avoid:**
- Map C++ global state to Bevy Resources (`#[derive(Resource)]`), not to static mutables or `lazy_static`
- Map C++ struct arrays (e.g., entity lists, terrain patches) to ECS entities with components
- Map C++ `update()` methods to Bevy systems that query components
- Keep the BSP tree and quadtree as Resources (they are spatial indices, not per-entity data) -- this is the correct hybrid approach documented in the ECS conversion research
- Resist the urge to put `pub` on everything; use Bevy's query system for data access
- Budget time for "rethinking" each module, not just "rewriting" it. The rendering pipeline (CLEAR/TERRAIN/WORLD/SHADOW/REFLECTION/RESOLVE) should map to system ordering in Bevy's Update schedule, not to a single monolithic function

**Warning signs:**
- More than 5 `unsafe` blocks outside of FFI or SIMD code
- Any `static mut` usage
- Systems that take more than 4 mutable query parameters (indicates over-centralized logic)
- `Arc<Mutex<>>` on hot-path data structures
- A single system file exceeding 800 lines

**Phase to address:**
Phase 1 (Foundation) -- define ECS architecture before writing implementation code. Create component/system mapping document as Phase 1 deliverable.

---

### Pitfall 3: Binary Format Parsing with Unsafe Transmute

**What goes wrong:**
The C++ codebase reads `.xp` and `.a3d` files by casting raw byte buffers to struct pointers (`*(XPCell*)ptr`). Porting this directly to Rust using `std::mem::transmute` or `std::ptr::read_unaligned` introduces undefined behavior if:
- Struct padding differs between C++ and Rust (even with `#[repr(C)]`)
- Endianness assumptions are violated (A3D magic is `0x44335341` little-endian)
- The gzip-compressed XP data contains extra header bytes not accounted for
- Alignment requirements differ (XPCell is 10 bytes -- not naturally aligned)

The XP format has column-major ordering (not row-major), and the A3D format has variable-length sections (mesh library, terrain patches, instances, BSP). A single off-by-one in offset calculation corrupts all subsequent parsing.

**Why it happens:**
C++ makes raw struct reads trivial with pointer casting. The Rust equivalent requires explicit handling of endianness, alignment, and padding. Developers use `unsafe` transmute to "keep it simple" and introduce subtle bugs that only manifest on specific files or platforms.

**How to avoid:**
- Use `nom` for the A3D parser (complex variable-length format with sections, version headers, and BSP trees). Nom provides composable parsers with proper error handling.
- Use `zerocopy` with `#[derive(FromBytes, IntoBytes)]` for fixed-size structures like XPCell (10 bytes: u32 glyph + 3 bytes fg + 3 bytes bg). Zerocopy's `Unalign<T>` handles the non-aligned XPCell.
- Use `flate2` for gzip decompression of XP files (verified: XP format is gzip-wrapped per RFC 1952)
- Explicitly specify `LittleEndian` for all multi-byte reads (A3D is LE, XP header fields are LE)
- Write golden file tests: parse a known C++ output file, compare byte-for-byte with Rust output
- Validate magic bytes and version numbers before parsing: A3D magic = `0x44335341`, XP version in header

**Warning signs:**
- Parsing works on test files but crashes on real game assets
- Values appear byte-swapped (e.g., width/height reversed)
- Glyph indices > 255 (CP437 is 0-255; XPCell stores glyph as u32 but only low byte matters)
- Off-by-one errors in layer iteration (XP layers are column-major)

**Phase to address:**
Phase 2 (Asset Loading) -- binary parsers must be correct and tested before any rendering work begins. Incorrect parsing poisons everything downstream.

---

### Pitfall 4: CPU Rasterizer Performance Death by a Thousand Cuts

**What goes wrong:**
The CPU software rasterizer must process every pixel at 2x supersampled resolution (e.g., 480x270 samples for 240x135 output) at 60fps. That is 129,600 samples per frame, with triangle rasterization, depth testing, and color computation per sample. Performance fails not from one bottleneck but from accumulated small inefficiencies: heap allocations in the inner loop, poor cache locality from AoS (Array of Structs) layout, branch mispredictions in the edge function, bounds checking on every array access, and f32-to-integer conversions in the color pipeline.

At 1080p (960x540 supersampled = 518,400 samples), the budget is 32ns per sample at 60fps. There is zero room for per-sample allocations.

**Why it happens:**
Rust's safety guarantees (bounds checking, no raw pointer arithmetic) add overhead in tight loops. Developers write correct-but-slow code first, then discover the inner rasterization loop is 3-5x slower than C++ because of:
1. Bounds checking on `SampleBuffer[index]` in the inner loop
2. `Vec` allocations for intermediate triangle lists
3. AoS layout for `Sample` struct (8 bytes: height + visual + diffuse + spare) causes cache line waste when iterating only heights
4. No SIMD for the edge function / barycentric coordinate computation

**How to avoid:**
- Use `unsafe { buffer.get_unchecked(index) }` in the inner rasterization loop ONLY after proving index bounds with an outer check (the triangle bounding box is clipped to buffer dimensions). Document the safety invariant.
- Pre-allocate all buffers at startup. The SampleBuffer is fixed-size per frame. Use `Vec::with_capacity` and never reallocate during rendering.
- Keep the `Sample` struct as a compact 8-byte AoS (matching C++) for now -- the access pattern (read/write all fields per sample) favors AoS over SoA for this specific case.
- Use integer arithmetic for edge functions and barycentric coordinates, matching the C++ implementation's fixed-point approach (the `0x10000` area threshold suggests 16.16 fixed-point).
- Profile with `cargo flamegraph` after Phase 3 (first render). Optimize the hot path, not everything.
- Target: rasterization < 8ms per frame at 1080p, leaving 8ms for RESOLVE + GPU upload.

**Warning signs:**
- Frame time > 16.67ms with a simple scene (flat terrain, no meshes)
- `perf` / Instruments showing > 5% time in bounds checking (`core::panicking::panic_bounds_check`)
- Memory allocator appearing in flame graph during rendering
- Cache miss rate > 10% in L1 data cache during rasterization (use `perf stat` or Instruments)

**Phase to address:**
Phase 3 (CPU Rasterizer) -- implement with performance-aware patterns from day one. Performance testing in Phase 4 (Integration), with optimization pass if needed in Phase 5.

---

### Pitfall 5: Floating-Point Divergence Between C++ and Rust Output

**What goes wrong:**
The C++ rasterizer's visual output depends on specific floating-point behavior: edge function sign tests, perspective-correct interpolation with 1/W division, depth comparisons, and the RGB555 color quantization pipeline. Rust and C++ produce different floating-point results for the same operations due to:
1. FMA (Fused Multiply-Add): C++ compilers (especially Clang) may auto-fuse `a*b+c` into a single FMA instruction, which produces a different (more accurate) result than separate multiply and add. Rust does NOT auto-fuse; you must explicitly call `f32::mul_add()`.
2. Transcendental functions (`sin`, `cos`, `atan2`): Rust documents these as non-deterministic -- results vary by platform, Rust version, and even within the same execution.
3. Intermediate precision: C++ on x87 FPU may use 80-bit extended precision for intermediate results. Rust targets SSE2 (32-bit/64-bit) by default on x86_64.
4. Expression ordering: `(a + b) + c` vs `a + (b + c)` produces different results with floats. C++ compilers with `-ffast-math` may reorder; Rust never reorders (strict IEEE 754 by default).

The consequence: "pixel-for-pixel identical output" (PROJECT.md constraint) may be impossible for edge cases. Triangles sharing edges may be claimed by different triangles in Rust vs C++, producing visible seam artifacts.

**Why it happens:**
The project requires visual fidelity matching. Developers assume that porting the same algorithm means the same output. But floating-point arithmetic is not associative, and compiler optimizations change results.

**How to avoid:**
- Accept "perceptually identical" rather than "bit-identical" as the fidelity target. Define a tolerance: e.g., < 1% of cells differ, and differing cells are adjacent to triangle edges.
- Use integer arithmetic for the edge function and triangle rasterization (matching C++ fixed-point patterns). Integer math IS deterministic and platform-independent.
- For the RGB555 color pipeline, use the exact C++ integer formula: `((value * 527) + 23) >> 6`. This is integer arithmetic -- deterministic.
- Use `f32::mul_add()` explicitly where the C++ code relies on FMA behavior. Profile whether this changes output vs separate multiply/add.
- Establish golden reference files: render known scenes in C++, capture the AnsiCell output, and compare with Rust output in CI. Track which cells differ and why.
- The BC_P center sampling (2*c+1) uses integer arithmetic -- preserve this exactly.

**Warning signs:**
- Z-fighting artifacts along triangle edges that don't appear in C++ output
- Color banding differences (adjacent cells choosing different xterm-256 palette entries)
- Depth test disagreements visible as triangle ordering flicker
- Golden file tests showing > 2% cell difference rate

**Phase to address:**
Phase 3 (CPU Rasterizer) -- establish golden file comparison framework. Phase 4 (Integration) -- tune tolerance thresholds with real game scenes.

---

### Pitfall 6: Bevy Version Churn Breaking the ASCII Render Plugin

**What goes wrong:**
Bevy releases breaking API changes approximately every 3 months (0.14, 0.15, 0.16, 0.17, 0.18...). The render pipeline APIs are among the most volatile: `RenderPhase`, `ExtractComponent`, `ViewNode`, pipeline descriptors, and bind group layouts change signatures between versions. A custom ASCII render plugin that deeply integrates with Bevy's render graph (as this project requires) will break on every Bevy upgrade. The project targets Bevy 0.18, but 0.19 or 0.20 may ship during development.

Notable breaking changes in recent versions:
- 0.14->0.15: `Handle` can no longer be used as `Component`; retained render world introduced
- 0.15->0.16: Built-in entity relationships; `Curve` refactored to extension traits
- 0.16->0.17: Further render pipeline restructuring
- 0.17->0.18: WGPU upgraded to v27; portal/mirror infrastructure added

**Why it happens:**
Bevy is pre-1.0 and explicitly prioritizes improvement over stability. The render pipeline is the most actively developed subsystem. Custom render plugins are "deep integration" -- they depend on internal APIs that are not covered by stability guarantees.

**How to avoid:**
- Pin Bevy to exact version `0.18.0` in `Cargo.toml` (not `^0.18` or `0.18.*`)
- Isolate all Bevy render API usage behind a thin abstraction layer (`AsciiRenderBackend` trait) so that Bevy version upgrades only require changing one module
- Do NOT depend on Bevy's built-in mesh rendering pipeline. This project renders via CPU rasterizer to a texture -- use a simple fullscreen quad with a custom material, not the full `MeshPipeline`
- Monitor `thisweekinbevy.com` and the Bevy migration guides for upcoming changes
- Budget 1-2 days per Bevy version upgrade in the project timeline
- Consider: is the Bevy upgrade necessary? If 0.18 works, stay on 0.18 until v1 is needed.

**Warning signs:**
- `cargo update` breaks compilation with cryptic render pipeline errors
- Bevy examples from blog posts don't compile (they target a different version)
- Community plugins (bevy_kira_audio, etc.) lag behind Bevy releases, creating version conflicts

**Phase to address:**
Phase 1 (Foundation) -- pin version and establish abstraction layer. Every phase -- resist upgrading Bevy mid-project unless a specific 0.19+ feature is required.

---

## Moderate Pitfalls

### Pitfall 7: Over-Componentizing the ECS Architecture

**What goes wrong:**
Developers split every C++ field into its own Bevy component (Position, Velocity, Health, Damage, Armor, Name, Sprite, Layer, ...), creating entities with 15+ components. This causes:
- Archetype fragmentation: each unique combination of components creates a new archetype, increasing memory overhead and reducing iteration speed
- Query complexity: systems need `Query<(&A, &B, &C, &D, &E, &F), (With<G>, Without<H>)>` -- hard to read, easy to forget a filter
- Frequent archetype moves: adding/removing one component moves the entire entity to a new archetype, which is expensive

The Asciicker C++ code has monolithic structs (Sample = 8 bytes, 4 fields). Splitting `Sample` into 4 components for the SampleBuffer would be catastrophic for cache performance.

**Prevention:**
- The SampleBuffer should be a Resource containing a flat `Vec<Sample>`, not 500K entities with components. It is a dense grid, not a sparse entity collection.
- Group related data into larger components: `TerrainPatch { heightmap, visual_map, position }` rather than separate `Heightmap`, `VisualMap`, `PatchPosition` components
- Only use ECS entities for things that benefit from composition: game characters, world objects, UI elements
- The BSP tree and quadtree should be Resources with `Entity` references, not entity hierarchies
- Rule of thumb: if you would iterate it as a contiguous array, make it a Resource. If you would query it by component combination, make it entities.

**Phase to address:**
Phase 1 (Foundation) -- define which data structures are Resources vs Entities in the architecture document.

---

### Pitfall 8: System Ordering Spaghetti

**What goes wrong:**
The C++ rendering pipeline has a strict order: CLEAR -> TERRAIN -> WORLD -> SHADOW -> REFLECTION -> RESOLVE -> SPRITES. In Bevy, systems run in parallel by default. If the render pipeline systems are not explicitly ordered, terrain may render after RESOLVE, shadows may compute before terrain is drawn, and sprites may blit before the SampleBuffer is populated.

**Prevention:**
- Use `.chain()` for the render pipeline systems to enforce sequential execution:
  ```rust
  app.add_systems(Update, (
      clear_system,
      terrain_system,
      world_system,
      shadow_system,
      reflection_system,
      resolve_system,
      sprite_blit_system,
  ).chain());
  ```
- Define `SystemSet` enums for pipeline stages to allow future systems to hook in at specific points
- Physics and input can run in parallel with rendering (different data) -- only chain systems that share the SampleBuffer
- Test ordering by adding debug prints that log system execution order during development

**Phase to address:**
Phase 3 (CPU Rasterizer) -- establish system ordering when implementing the pipeline stages.

---

### Pitfall 9: Coordinate System Confusion (Z-Up vs Y-Up)

**What goes wrong:**
The C++ Asciicker engine uses Z as the up axis (documented in physics.h:41). Bevy uses Y as the up axis by default. Mixing these conventions causes:
- Terrain rendering upside-down or rotated 90 degrees
- Physics collisions on wrong axis
- Camera looking at wrong plane
- BSP plane normals pointing in wrong direction

This is insidious because partial scenes may "look right" (a flat terrain rendered from above looks the same regardless of up axis) but break when 3D features like height, gravity, or camera rotation are added.

**Prevention:**
- Define a `Coordinate` conversion module early (Phase 1) that handles C++ Z-up to Bevy Y-up transforms
- Alternatively: keep the internal coordinate system as Z-up (matching C++ data) and only convert at the Bevy camera/transform boundary. This minimizes conversion points and preserves C++ algorithm correctness.
- Document the convention in a `COORDINATES.md` or module-level doc comment
- The perspective matrix and camera setup must account for the axis convention
- Add assertion: in debug mode, verify that all positions have Z (or Y) within expected range for the "up" axis

**Phase to address:**
Phase 1 (Foundation) -- decide and document convention before any spatial code is written.

---

### Pitfall 10: Asset Loading Lifecycle Mismanagement

**What goes wrong:**
Bevy's asset loading is asynchronous. Calling `asset_server.load("world.a3d")` returns a `Handle<A3dWorld>` immediately, but the asset isn't loaded yet. Systems that assume assets are available on the frame they're requested will crash or render nothing. Additionally:
- Weak handles don't prevent asset unloading: if no strong handle exists, the asset is dropped
- Asset dependencies (e.g., A3D referencing XP sprites) must be tracked manually
- The custom A3D and XP loaders need to implement `AssetLoader` trait correctly, including error propagation

**Prevention:**
- Implement custom `AssetLoader` for both `.xp` and `.a3d` formats
- Use `AssetServer::wait_for_asset` or event-based loading (`AssetEvent<T>`) to gate game state transitions on asset readiness
- Keep strong handles (`Handle<XpSprite>`, `Handle<A3dWorld>`) in a `LoadedAssets` resource
- Use Bevy's `States` system: `Loading` -> `Playing` transition only after all required assets are loaded
- For the A3D format (which contains mesh library + terrain + instances + BSP), the loader should produce a compound asset that holds sub-handles to referenced sprites

**Phase to address:**
Phase 2 (Asset Loading) -- implement correct async loading with state gating.

---

### Pitfall 11: Testing the Untestable -- Visual Regression for ASCII Output

**What goes wrong:**
The project's correctness criterion is visual: "CPU rasterizer output must match C++ engine." But there's no automated way to verify this without:
1. Golden reference files from the C++ engine
2. A comparison tool that accounts for acceptable differences
3. CI infrastructure to run visual comparisons on every commit

Without visual regression testing, bugs accumulate silently. A small change to edge function rounding changes 200 cells across 50 test scenes, but nobody notices until the game looks wrong.

**Prevention:**
- Generate golden reference files by running the C++ engine on known test scenes and capturing the AnsiCell output buffer (not screenshots -- the raw fg/bg/glyph data)
- Build a `diff_ascii_frames(expected: &[AnsiCell], actual: &[AnsiCell]) -> DiffReport` utility that reports: total cells, differing cells, percentage, and which cells differ
- Set CI threshold: fail if > 1% of cells differ from golden reference
- Use `insta` crate for snapshot testing of individual test scenes: `assert_snapshot!(render_scene("test_flat_terrain"))`
- For ratatui-style terminal rendering, consider `ratatui_testlib` for headless snapshot testing
- Create a small set of "canonical test scenes" that exercise each pipeline stage: flat terrain, terrain with height, single mesh, sprite overlay, shadow, reflection

**Phase to address:**
Phase 3 (CPU Rasterizer) -- establish golden file testing alongside the first render output. Phase 4 (Integration) -- expand test suite to cover all pipeline stages.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `unsafe` bounds-check bypass in rasterizer | 2-3x speedup in inner loop | Potential buffer overwrite if invariant broken | Only in the inner rasterization loop, with documented safety proof |
| Skipping the Alex Harri k-d tree (using auto_mat only) | Faster time-to-first-render | Must retrofit k-d tree later; auto_mat is less visually accurate | Acceptable for Phase 3; integrate k-d tree in Phase 5 per D010 |
| Hardcoding 240x135 output resolution | Simpler SampleBuffer allocation | Cannot support variable terminal sizes | Never -- parameterize from day one |
| Single-threaded RESOLVE phase | Simpler implementation | Cannot hit 60fps at 1080p | Until Phase 5 (Optimization); profile first |
| Copying entire SampleBuffer to render world | Correct extraction | ~97KB copy per frame at 240x135; ~2MB at 1080p | Acceptable at 240x135; at 1080p, consider double-buffering or shared memory |
| Using `HashMap` for glyph cache | Simple API | FxHashMap is 2-3x faster for integer keys | Never for hot-path code; use `FxHashMap` from `rustc-hash` |

---

## Integration Gotchas

Common mistakes when connecting Asciicker subsystems.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| SampleBuffer -> GPU texture upload | Uploading RGBA8 texture (4 bytes/cell) when only 3 bytes needed (fg+bg+glyph) | Use 3 separate R8 textures (Mage Core 4-texture approach): char index, fg palette index, bg palette index. Font atlas is loaded once. |
| CPU rasterizer -> Bevy scheduling | Running rasterizer as a Bevy system that blocks the main thread for 10ms+ | Use `par_iter` within the system, or run rasterizer on a background thread and double-buffer the SampleBuffer |
| XP sprite loading -> ECS | Creating one entity per XP cell (100x100 sprite = 10,000 entities) | Load XP as a Resource (`XpSprite` struct with `Vec<XpCell>`); only create entities for sprite instances in the world |
| A3D BSP tree -> ECS | Creating entity hierarchy mirroring BSP tree nodes (thousands of entities) | Store BSP tree as a single `Resource<BspTree>` with internal node/leaf vectors; reference world entities via Entity IDs |
| Audio (bevy_kira_audio) -> Bevy | Loading audio assets in the render thread | Load audio in main world systems; bevy_kira_audio handles playback threading internally |
| Physics collision -> Bevy transforms | Using Bevy's `Transform` for physics positions | Use custom `PhysicsPosition` component (Z-up, matching C++); sync to Bevy `Transform` (Y-up) only for rendering |

---

## Performance Traps

Patterns that work at small scale but fail at production resolution.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Per-cell heap allocation in RESOLVE | Frame time spikes, GC-like pauses | Pre-allocate AnsiCell output buffer; use stack arrays for 2x2 sample blocks | > 80x25 output resolution |
| `Vec<Triangle>` per terrain patch | Allocation visible in flame graph | Use fixed-size arrays: each patch has exactly `2 * VISUAL_CELLS * VISUAL_CELLS` triangles (128 for 8x8) | > 100 visible patches |
| Naive xterm-256 color lookup (linear search through 256 entries) | 30% of frame time in color quantization | Use the pre-computed `auto_mat` lookup table (32K entries, O(1) per RGB555 value) | Always -- this is the entire color pipeline |
| f32 -> u8 conversion via `as` in tight loops | Saturating cast adds branch per conversion | Use `unsafe { value.to_int_unchecked::<u8>() }` after clamping to 0..255 | > 100K samples per frame |
| Recalculating terrain normals every frame | 50% of terrain rendering time | Cache normals at load time; terrain heightmaps are static | > 50 visible terrain patches |
| String formatting in debug logging inside render loop | 10x slowdown in debug builds | Use `#[cfg(debug_assertions)]` guards on render loop logging; use tracing with compile-time level filtering | Always in debug builds |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **XP parser:** Often missing column-major order handling -- verify that glyph at (x=0, y=1) reads from offset `1 * width`, not offset `1`
- [ ] **A3D parser:** Often missing version validation -- verify magic bytes AND version number before parsing body
- [ ] **Triangle rasterizer:** Often missing double-sided rendering -- verify both winding orders render correctly (not just CCW)
- [ ] **SampleBuffer clear:** Often missing the correct initial depth value -- must be `-1000000.0f`, not `0.0` or `f32::MAX`
- [ ] **RGB555 packing:** Often swapping R and B channels -- verify bit layout is `[14:10]=R, [9:5]=G, [4:0]=B`
- [ ] **Terrain rendering:** Often missing the copy-paste bug fixes -- TERRAIN-001 through TERRAIN-004 must be corrected during port, not carried over
- [ ] **Camera perspective:** Often assuming orthographic -- the project requires perspective projection (D004-D005)
- [ ] **RESOLVE stage:** Often computing averages incorrectly -- the 2x2 supersample averaging must use min-depth (closest), not average-depth
- [ ] **Sprite rendering:** Often missing depth test against SampleBuffer -- sprites blit AFTER resolve but must depth-test against the pre-resolve SampleBuffer
- [ ] **Coordinate system:** Often forgetting Z-up conversion -- verify gravity acts on correct axis in physics

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Render world desync | MEDIUM | Audit all extraction systems; replace conditional extracts with unconditional; add frame-count assertions |
| 1:1 C++ translation | HIGH | Requires re-architecture of affected modules; prioritize hot-path modules (rasterizer, SampleBuffer) first |
| Binary parsing bugs | LOW | Fix parser, re-run golden file tests; parsers are isolated modules with clear inputs/outputs |
| Rasterizer too slow | MEDIUM | Profile with flamegraph; apply bounds-check elision and SIMD to identified hot spots; consider resolution scaling as interim fix |
| Floating-point divergence | LOW-MEDIUM | Adjust golden file tolerance thresholds; switch to integer arithmetic for the divergent operations; document accepted differences |
| Bevy version break | MEDIUM | Check migration guide; update abstraction layer; if severe, stay on previous version |
| Over-componentized ECS | HIGH | Requires merging components and rewriting queries; affects all systems that touch those components |
| System ordering bugs | LOW | Add `.chain()` or `.before()`/`.after()` constraints; Bevy's ambiguity detection helps identify the problem |
| Coordinate system mixup | MEDIUM | Add conversion layer; audit all spatial code for axis assumptions; fix is mechanical but touches many files |
| Asset loading crash | LOW | Add loading state gate; track handles in resource; straightforward fix once identified |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Render world desync | Phase 1 (Foundation) | Render plugin skeleton renders a test pattern from extracted data; no visual lag on state changes |
| 1:1 C++ translation | Phase 1 (Foundation) | Architecture document maps C++ modules to ECS components/systems/resources |
| Binary parsing bugs | Phase 2 (Asset Loading) | Golden file tests pass for all known test assets; parser handles malformed input gracefully |
| Rasterizer performance | Phase 3 (CPU Rasterizer) | Flat terrain renders at > 60fps at 240x135; flame graph shows no allocation in render loop |
| Floating-point divergence | Phase 3 (CPU Rasterizer) | Golden file comparison shows < 1% cell difference on canonical test scenes |
| Bevy version churn | Phase 1 (Foundation) | Bevy pinned to 0.18.0; render API isolated behind abstraction trait |
| Over-componentized ECS | Phase 1 (Foundation) | Architecture review: SampleBuffer, BSP, quadtree are Resources; game entities are ECS |
| System ordering | Phase 3 (CPU Rasterizer) | Render pipeline systems chained; debug logging confirms execution order |
| Coordinate system confusion | Phase 1 (Foundation) | Coordinate convention documented; conversion module exists with unit tests |
| Asset loading lifecycle | Phase 2 (Asset Loading) | Loading -> Playing state transition; no crashes on missing assets |
| Visual regression testing | Phase 3 (CPU Rasterizer) | CI runs golden file comparison; test failure blocks merge |

---

## Sources

- Bevy render world architecture: [Render Stages - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/gpu/stages.html)
- Bevy extraction desync: [Conditional extraction to render world can cause desync - Issue #15871](https://github.com/bevyengine/bevy/issues/15871)
- Bevy render architecture: [Render Architecture Overview - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/gpu/intro.html)
- Bevy version churn: [Confusion with Version Changes - Issue #16414](https://github.com/bevyengine/bevy/issues/16414)
- Bevy migration guides: [0.15 to 0.16](https://bevy.org/learn/migration-guides/0-15-to-0-16/), [0.16 to 0.17](https://bevy.org/learn/migration-guides/0-16-to-0-17/)
- Floating-point determinism: [Gaffer On Games - Floating Point Determinism](https://gafferongames.com/post/floating_point_determinism/)
- Rust FP differences: [Subtle floating-point differences between C library and Rust rewrite](https://users.rust-lang.org/t/subtle-floating-point-differences-between-c-library-and-its-rust-re-write/82355)
- FMA consistency: [Can Function Inlining Affect Floating Point Outputs?](https://siboehm.com/articles/23/Inlining-FMA-FP-consistency)
- Rust performance optimization: [The Rust Performance Book](https://nnethercote.github.io/perf-book/general-tips.html)
- Cache locality in Rust: [Optimizing for Cache Locality in Rust](https://softwarepatternslexicon.com/patterns-rust/23/9/)
- Binary parsing safe alternatives: [The Magic of zerocopy](https://swatinem.de/blog/magic-zerocopy/)
- Zerocopy alignment: [Unalign in zerocopy](https://docs.rs/zerocopy/latest/zerocopy/struct.Unalign.html)
- Bytemuck padding discussion: [Why can't you use this crate with types with padding bytes?](https://github.com/Lokathor/bytemuck/discussions/86)
- ECS design decisions: [Design decisions when building games using ECS](https://arielcoppes.dev/2023/07/13/design-decisions-when-building-games-using-ecs.html)
- OOP to ECS migration: [ECS: The opposite of OOP?](https://leetless.de/posts/ecs-the-opposite-of-oop/)
- Visual regression for TUI: [ratatui_testlib](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/)
- Project documentation: `PROJECT.md`, `RISK_REGISTER.md`, `FAILURE_LOG.md`, `GAPS_ANALYSIS_SUMMARY.md`, `plan-rendering-gaps.md`, `plan-SampleBuffer-bridge.md`, `research-bevy-render-pipeline.md`, `research-bevy-ecs-conversion.md`

---
*Pitfalls research for: Asciicker C++ to Rust/Bevy game engine port*
*Researched: 2026-02-20*
