# Stack Research

**Domain:** Custom ASCII game engine (Rust/Bevy port of 82K-line C++ engine)
**Researched:** 2026-02-20
**Confidence:** MEDIUM-HIGH (Bevy 0.18 confirmed via official release; some library versions verified via crates.io/docs.rs; custom rendering architecture pattern verified via Mage Core reference code + Bevy examples)

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended | Confidence |
|------------|---------|---------|-----------------|------------|
| **Bevy** | 0.18.0 | ECS, input, windowing, game loop, asset loading | Released 2026-01-13. Provides ECS-first architecture matching the port's needs. The `2d_api` feature collection (new in 0.18) lets us use Bevy's ECS/input/windowing WITHOUT its default renderer -- exactly what a custom CPU rasterizer + GPU ASCII output needs. 174 contributors, 659 PRs in this release. | HIGH |
| **wgpu** | 27.x (bundled with Bevy 0.18) | GPU abstraction for ASCII output shader | Bevy 0.18 bundles wgpu 27. Do NOT add a separate wgpu dependency -- access it through `bevy_render`'s re-exports to avoid version conflicts. The 4-texture ASCII shader runs through Bevy's render graph, not standalone wgpu. | HIGH |
| **Rust** | 2021 edition, stable | Language | Bevy 0.18 targets stable Rust. No nightly features needed. | HIGH |

### Bevy Feature Configuration

Use `default-features = false` to exclude Bevy's built-in 2D/3D renderers. The ASCII engine provides its own rendering.

```toml
[dependencies]
bevy = { version = "0.18", default-features = false, features = [
    "2d_api",           # ECS + input + windowing + sprites WITHOUT built-in renderer
    "bevy_render",      # Core render backend (wgpu access, render graph, render nodes)
    "bevy_core_pipeline", # Core pipeline infrastructure (camera, clear color)
    "bevy_shader",      # WGSL shader compilation
    "scene",            # Scene serialization for world loading
    "png",              # PNG font atlas loading
] }
```

**Why this configuration:** The `2d_api` feature (new in 0.18) provides `common_api` + `bevy_sprite` without `2d_bevy_render`. This gives us ECS, input, camera, color, text, windowing -- but NOT Bevy's sprite renderer, which we replace with our CPU rasterizer + GPU ASCII output. We add `bevy_render` explicitly because we need wgpu access for our custom render node.

**What NOT to include:** `2d_bevy_render`, `3d`, `bevy_pbr`, `bevy_gltf` -- these are Bevy's built-in renderers that conflict with our custom pipeline.

### ASCII GPU Output (Render Plugin)

| Component | Approach | Why | Confidence |
|-----------|----------|-----|------------|
| **4-texture shader** | Custom WGSL fragment shader (Mage Core pattern) | Proven pattern: char index texture + fg color texture + bg color texture + font atlas texture. The fragment shader looks up the glyph in the font atlas and outputs fg or bg color. Reference implementation in Mage Core (v0.2.0, ~500 LOC) is MIT licensed and uses this exact approach with wgpu 22.1. We adapt the shader to run as a Bevy ViewNode. | HIGH |
| **Bevy integration** | ViewNode + ViewNodeRunner in custom render graph node | Bevy 0.18's ViewNode trait (from `bevy_render::render_graph`) is the correct abstraction for a fullscreen post-process pass. Our node reads the SampleBuffer output from the CPU rasterizer, uploads to 3 GPU textures (char/fg/bg), and runs the fullscreen WGSL shader. See Bevy's `custom-post-processing` example. | HIGH |
| **FullscreenMaterial** | Consider for simpler initial implementation | New in Bevy 0.18: `FullscreenMaterial` trait reduces boilerplate for fullscreen shaders. Evaluate whether it provides enough control for our 4-texture bind group. If not, fall back to raw ViewNode. | MEDIUM |
| **Font atlas** | CP437 16x16 glyph grid, PNG loaded as Bevy asset | Standard approach from Mage Core. 256 glyphs arranged in 16x16 grid. Loaded once at startup. | HIGH |

### CPU Rasterizer

| Component | Approach | Why | Confidence |
|-----------|----------|-----|------------|
| **SampleBuffer** | Hand-rolled Rust struct | This is the core data structure. Must match C++ SampleBuffer exactly: 2x supersampled depth/color buffer, per-cell glyph/fg/bg output. No crate provides this specific abstraction. ~200-400 lines. | HIGH |
| **Triangle rasterization** | Hand-rolled barycentric fill | Direct port of C++ `render.cpp` triangle rasterizer. No crate matches the specific edge-case behavior needed for visual fidelity. Standard barycentric coordinate approach. | HIGH |
| **Line rasterization** | Hand-rolled Bresenham | Direct port of C++ Bresenham implementation. Trivial (~30 lines) and must match C++ output exactly. | HIGH |
| **Parallelization** | rayon 1.11 for terrain patch rendering | Terrain patches are independent and embarrassingly parallel. rayon's `par_iter()` maps directly to the C++ OpenMP-style parallelism. Use for: terrain rendering (16+ patches), BSP traversal batching. Do NOT use for: individual triangle rasterization (overhead exceeds benefit for small triangles). | HIGH |

**Why NOT use a CPU rasterizer crate (euc, rust-softrender, etc.):** Visual fidelity is the #1 constraint. The C++ engine has specific edge-case behaviors in its rasterizer (depth tie-breaking, sub-pixel sampling positions, glyph selection thresholds) that must be matched exactly. Using a generic rasterizer would create subtle rendering differences. The rasterizer is also tightly coupled to the SampleBuffer format and RGB555 color space. Hand-rolling ~1500 lines of rasterization code is the correct choice for this project.

### Glyph Selection (RESOLVE Stage)

| Component | Approach | Why | Confidence |
|-----------|----------|-----|------------|
| **auto_mat (initial)** | Hand-rolled lookup tables | Direct port of C++ `auto_mat` shade/glyph arrays. These are static lookup tables (~500 entries). Must match C++ values exactly. Start here for first-render milestone. | HIGH |
| **Alex Harri 6D k-d tree (upgrade)** | kiddo 5.2 | Kiddo is the fastest k-d tree library in Rust. Supports arbitrary dimensions (we need 2D initially, then 6D). ImmutableKdTree variant is perfect for our use case: build once from 256 CP437 shape vectors, query per-cell during RESOLVE. Zero-copy serialization via rkyv for fast startup. | HIGH |
| **Shape vector computation** | Hand-rolled (port from TypeScript reference) | Alex Harri's shape-vector computation divides each glyph cell into regions and computes coverage. Port from TypeScript reference implementation at `../reference/alexharri-ascii`. ~300 lines. | MEDIUM |

### Binary Format Parsing

| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| **nom** | 8.0 | Binary format parser combinators | Parse .a3d world files (header, mesh library, terrain patches, instances, BSP tree). nom was designed for binary parsing from day one. v8 uses trait-based API: `combinator(arg).parse(input)`. Provides automatic error reporting and composable parsers. | HIGH |
| **flate2** | 1.1 | Gzip decompression | Decompress .xp sprite files. Uses miniz_oxide (pure Rust) backend by default. Streaming API via `GzDecoder` wrapping `Read`. | HIGH |
| **bytemuck** | 1.24 | Zero-copy type casting | Cast between `[u8]` slices and typed structs for GPU buffer uploads. Required for the 4-texture approach (write u32 RGBA values directly to texture storage). Already used by Mage Core reference. | HIGH |

**Why nom over manual parsing:** The .a3d format is complex (header + mesh library + terrain patches + instances + BSP tree, each with nested sub-structures). nom's combinator approach catches off-by-one errors at compile time and provides clear error messages during development. For the simpler .xp format (fixed 16-byte header + flat cell arrays), manual parsing with `bytemuck` and `std::io::Read` is acceptable -- nom is optional there.

**Why NOT binrw/binread:** binrw is attribute-macro-heavy and less transparent for debugging format issues. Since we are reverse-engineering a binary format from C++ source code (not a formal spec), we need maximum visibility into parse state. nom's explicit combinator chains are easier to debug than binrw's derive macros.

### Audio

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| **bevy_kira_audio** | **0.25** | Audio playback with advanced mixing | Bevy 0.18 compatible (0.24 is for Bevy 0.17 -- confirmed via compatibility table). Kira provides 16-track mixing, spatial audio, tween-based parameter control. The C++ engine uses a 16-track mixer, which maps directly to Kira's track system. Must disable `bevy_audio` default feature (incompatible). | HIGH |

**CRITICAL FIX from existing skeleton:** The existing `Cargo.toml` specifies `bevy_kira_audio = "0.24"` which is for Bevy 0.17. This MUST be updated to `"0.25"` for Bevy 0.18 compatibility.

### Networking

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| **lightyear** | 0.24.x | Server-authoritative multiplayer | Most complete Bevy networking solution. Provides: entity replication, client-side prediction with rollback, snapshot interpolation, input buffering with packet-loss protection. Supports WebTransport (QUIC), WebSocket, and Steam via aeronet. Actively maintained, follows Bevy release cycle. | MEDIUM |

**Why lightyear over renet/bevy_replicon:** lightyear is batteries-included -- prediction, rollback, interpolation, and replication are built in. renet is lower-level (just transport + encryption) and would require building replication from scratch. bevy_replicon provides replication but no transport layer. Since the C++ engine already has a client-server architecture with state synchronization, lightyear's feature set maps most directly.

**Why MEDIUM confidence:** lightyear's exact Bevy 0.18 compatible version needs verification at implementation time. The crate follows Bevy's release cadence but version 0.24.2 was the latest found on docs.rs. Check the GitHub releases page before adding to `Cargo.toml`.

**Networking is Phase 6+ work.** Do not add this dependency until the single-player engine renders correctly.

### Math & Linear Algebra

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| **glam** | 0.32 (bundled with Bevy) | Vectors, matrices, quaternions | Bevy re-exports glam types. Use `bevy::math::Vec3`, `Mat4`, etc. -- do NOT add a separate glam dependency. SIMD-accelerated on x86/x86_64. | HIGH |

### Error Handling

| Library | Version | Purpose | Why | Confidence |
|---------|---------|---------|-----|------------|
| **thiserror** | 2.0 | Library-level error types | Derive `Error` trait for domain-specific error types (ParseError, RenderError, AssetError). v2.0 is current stable. | HIGH |
| **anyhow** | 1.0 | Application-level error propagation | Use in main(), system startup, and one-off error paths. Do NOT use in library code -- prefer thiserror for typed errors. | HIGH |

### Supporting Libraries

| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| **rayon** | 1.11 | Data parallelism | CPU rasterizer terrain rendering, BSP traversal batching. Replace `.iter()` with `.par_iter()` for embarrassingly parallel work. | HIGH |
| **kiddo** | 5.2 | K-d tree for glyph matching | RESOLVE stage shape-vector nearest-neighbor lookup. Use `ImmutableKdTree` built once at startup from 256 glyph shape vectors. | HIGH |
| **serde** | 1.0 | Serialization | Config files, save state. Already in existing skeleton. Use `derive` feature. | HIGH |
| **tracing** | 0.1 | Structured logging | Bevy uses tracing internally. Use `tracing::info!`, `warn!`, `error!` instead of `log` crate. Enables Tracy/puffin integration. | HIGH |
| **tracing-subscriber** | 0.3 | Log output formatting | Dev-only. Configure with `EnvFilter` for per-module log levels. | HIGH |

**What to REMOVE from existing skeleton:**
- `log` and `env_logger` -- Bevy uses `tracing` internally. Using `log` creates a parallel logging system. Replace all `log::info!()` with `tracing::info!()`.
- `serde_json` -- Not needed unless we add JSON config files. .xp and .a3d are binary formats.
- `crate-type = ["cdylib", "rlib"]` -- cdylib is for FFI/WASM. This is a native application. Use `[[bin]]` target.

---

## Development Tools

| Tool | Purpose | Notes | Confidence |
|------|---------|-------|------------|
| **cargo-insta** (+ insta 1.39+) | Snapshot testing | Test CPU rasterizer output by snapshotting SampleBuffer contents. Use `assert_debug_snapshot!` for ASCII grid comparisons. Install CLI: `cargo install cargo-insta`. | HIGH |
| **proptest** | 1.9 | Property-based testing | Test rasterizer invariants: "every pixel in a filled triangle has depth <= face depth", "RGB555 round-trip preserves value", "glyph selection is deterministic for same input". | HIGH |
| **cargo-flamegraph** (flamegraph 0.6.11) | CPU profiling | Generates flamegraphs via DTrace on macOS. Use `cargo flamegraph --root -- --release` for rasterizer hot-path analysis. | HIGH |
| **samply** | Interactive profiling (macOS) | Firefox Profiler UI. Better than flamegraph for iterative profiling sessions. `cargo install samply && samply record -- target/release/asciicker`. | MEDIUM |
| **Tracy** (tracy_full 1.12) | Frame-level profiling | Bevy has built-in Tracy spans via `trace` cargo feature. Shows per-system timing, render node costs, frame budget. Enable: `bevy = { features = ["trace_tracy"] }`. | HIGH |
| **cargo-nextest** | Test runner | Faster parallel test execution than `cargo test`. Useful when test suite grows beyond 100 tests. | MEDIUM |

---

## Installation

```toml
# Cargo.toml

[package]
name = "asciicker"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "asciicker"
path = "src/main.rs"

[dependencies]
# Core engine -- custom feature set, no default renderer
bevy = { version = "0.18", default-features = false, features = [
    "2d_api",
    "bevy_render",
    "bevy_core_pipeline",
    "bevy_shader",
    "scene",
    "png",
    "wayland",         # Linux Wayland support
    "x11",             # Linux X11 support
] }

# Audio
bevy_kira_audio = "0.25"

# Binary parsing
nom = "8.0"
flate2 = "1.1"
bytemuck = { version = "1.24", features = ["derive"] }

# Data structures
kiddo = "5.2"
rayon = "1.11"

# Serialization
serde = { version = "1.0", features = ["derive"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Logging (use tracing, NOT log crate)
tracing = "0.1"

[dev-dependencies]
# Snapshot testing
insta = { version = "1.39", features = ["yaml"] }

# Property-based testing
proptest = "1.9"

# Profiling integration
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
opt-level = 3
lto = true

[profile.dev]
opt-level = 1  # Slightly optimized dev builds for playable framerates
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|------------------------|
| **Bevy 0.18** (ECS + windowing) | miniquad / macroquad | If you need minimal dependency footprint and are fine hand-rolling ECS. Not recommended here -- the 82K-line engine benefits from a mature ECS. |
| **nom 8.0** (binary parsing) | binrw 0.14 | If parsing formats with an official spec and derive macros are preferred. Not ideal here because we are reverse-engineering from C++ source and need maximum parse-state visibility. |
| **nom 8.0** (binary parsing) | manual `std::io::Read` | Acceptable for the simple .xp format (fixed header + flat array). Use for .xp, nom for .a3d. |
| **kiddo 5.2** (k-d tree) | kd-tree 0.7 | If you need a simpler API and don't need SIMD or rkyv serialization. Kiddo is faster for our 6D use case. |
| **lightyear** (networking) | renet + bevy_renet | If you need lower-level control over the transport layer and want to build replication yourself. More work but fewer abstraction layers. |
| **bevy_kira_audio 0.25** | Bevy's built-in `bevy_audio` | If you only need basic playback. Not sufficient here -- C++ engine uses 16-track mixing, which Kira handles natively. |
| **rayon 1.11** (parallelism) | std::thread + crossbeam channels | If work items are heterogeneous and don't map to parallel iterators. Rayon is better for our uniform terrain-patch workload. |
| **insta** (snapshot testing) | goldenfile 0.5 | goldenfile writes to disk files, insta stores inline or in snapshot directories. insta has better ergonomics (cargo-insta review workflow) and is more actively maintained. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **log + env_logger** | Bevy uses `tracing` internally. `log` creates a parallel logging system that misses Bevy's structured spans and is invisible to Tracy/puffin. | `tracing` + `tracing-subscriber` |
| **wgpu as direct dependency** | Bevy 0.18 bundles wgpu 27. Adding a separate wgpu dep risks version conflicts and forces duplicate GPU state management. | Access wgpu through `bevy_render::renderer::RenderDevice` and `RenderQueue` |
| **winit as direct dependency** | Same as wgpu -- Bevy bundles winit. Direct use bypasses Bevy's input/windowing integration. | Use Bevy's `Window` component and `KeyInput` events |
| **euc / softrender (CPU rasterizer crates)** | Visual fidelity constraint requires matching C++ rasterizer behavior exactly. Generic rasterizers have different edge-case behaviors (depth tie-breaking, sub-pixel sampling). | Hand-roll ~1500 lines of rasterization code ported from C++ |
| **image crate (for font loading)** | Bevy's asset system loads PNGs natively. Adding `image` duplicates functionality and adds ~100KB binary size. | Use `bevy::asset::AssetServer` with PNG handle |
| **tokio (async runtime)** | Bevy has its own async task system (`bevy::tasks::AsyncComputeTaskPool`). tokio conflicts with Bevy's event loop and adds unnecessary overhead. Mage Core uses tokio because it doesn't have Bevy; we do. | `bevy::tasks` for async work |
| **chrono (time)** | Bevy provides `Time` resource with delta time, elapsed time, and fixed timestep. chrono is for calendar dates, not game time. | `bevy::time::Time` |
| **cdylib crate type** | This is a native application, not an FFI library or WASM module. cdylib adds linker overhead. | Standard `[[bin]]` target |

---

## Stack Patterns by Variant

**If starting from scratch (recommended):**
- Use the `Cargo.toml` above as-is
- Build the Bevy render plugin (ViewNode) first, verify ASCII output with a test pattern
- Then build the CPU rasterizer writing to SampleBuffer
- Bridge: SampleBuffer -> GPU texture upload -> ViewNode shader

**If salvaging existing skeleton:**
- Update `bevy` from `"0.18.0"` (already correct version) but add `default-features = false` + custom features
- Update `bevy_kira_audio` from `"0.24"` to `"0.25"` (BREAKING: 0.24 is Bevy 0.17)
- Remove `log`, `env_logger`, `serde_json`
- Remove `crate-type = ["cdylib", "rlib"]`
- Add `nom`, `bytemuck`, `kiddo`, `rayon`, `tracing`
- Keep `flate2`, `anyhow`, `thiserror`, `serde`

**If profiling reveals CPU rasterizer bottleneck:**
- Enable `rayon` parallelism for terrain patches first (biggest win)
- Profile with Tracy to identify per-system costs
- Consider SIMD intrinsics (`std::arch`) for inner rasterization loop as last resort
- Do NOT move rasterizer to GPU -- it defeats the visual-fidelity constraint

---

## Version Compatibility Matrix

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| bevy 0.18 | wgpu 27, glam 0.32, winit 0.30+ | All bundled -- do not add separate deps |
| bevy_kira_audio 0.25 | bevy 0.18 | MUST disable `bevy_audio` feature in Bevy |
| lightyear 0.24.x | bevy 0.18 (verify) | Check GitHub releases before adding; version may advance |
| nom 8.0 | Rust 1.65+ | No Bevy interaction -- standalone parsing |
| kiddo 5.2 | Rust 1.80+ | Standalone data structure, no framework deps |
| rayon 1.11 | Rust 1.80+ | Standalone, no Bevy interaction |
| proptest 1.9 | Rust 1.84+ | Dev-only, highest MSRV in the stack |

---

## Architecture Decision: Mage Core Adaptation Strategy

The Mage Core reference implementation (v0.2.0) uses raw wgpu + winit, NOT Bevy. The adaptation strategy:

1. **Shader:** Port `shader.wgsl` almost verbatim. The 4-texture bind group (fg, bg, chars, font) + uniform (font_width, font_height) is the correct pattern. Only change: adapt bind group indices to match Bevy's render graph expectations.

2. **Texture management:** Replace Mage Core's manual `wgpu::Texture` creation with Bevy's `RenderDevice::create_texture()`. The `bytemuck::cast_slice()` pattern for uploading u32 RGBA data to textures is identical.

3. **Render loop:** Replace Mage Core's manual `event_loop.run()` with a Bevy `ViewNode` that runs in the render graph. The node's `run()` method does what Mage Core's `render()` does: upload textures, create bind groups, issue draw call.

4. **App interface:** Replace Mage Core's `App` trait with Bevy ECS systems. Instead of `app.present(PresentInput { fore_image, back_image, text_image })`, a Bevy system writes to a `SampleBuffer` resource, and the ViewNode reads from it.

---

## Sources

- [Bevy 0.18 Release Notes](https://bevy.org/news/bevy-0-18/) -- Feature collections, FullscreenMaterial, 2d_api (HIGH confidence)
- [Bevy 0.18 docs.rs Feature Flags](https://docs.rs/crate/bevy/latest/features) -- Complete feature list (HIGH confidence)
- [Bevy Custom Post-Processing Example](https://bevy.org/examples/shaders/custom-post-processing/) -- ViewNode + fullscreen shader pattern (HIGH confidence)
- [Bevy Custom Render Phase Example](https://bevy.org/examples/shaders/custom-render-phase/) -- Custom RenderCommand pattern (HIGH confidence)
- [wgpu 28.0.0 docs.rs](https://docs.rs/crate/wgpu/latest) -- Latest wgpu; Bevy 0.18 uses wgpu 27 (HIGH confidence)
- [bevy_kira_audio GitHub](https://github.com/NiklasEi/bevy_kira_audio) -- Version compatibility table (HIGH confidence)
- [lightyear GitHub](https://github.com/cBournhonesque/lightyear) -- Multiplayer networking (MEDIUM confidence for Bevy 0.18 compat)
- [kiddo 5.2.2 docs.rs](https://docs.rs/crate/kiddo/latest) -- K-d tree for glyph matching (HIGH confidence)
- [nom 8.0 docs.rs](https://docs.rs/crate/nom/latest) -- Binary parser combinators (HIGH confidence)
- [rayon 1.11 docs.rs](https://docs.rs/crate/rayon/latest) -- Data parallelism (HIGH confidence)
- [Mage Core source](../ascii%20research/Mage-core/) -- Reference 4-texture ASCII rendering implementation, MIT licensed (HIGH confidence, local reference)
- [Bevy Profiling Guide](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md) -- Tracy + puffin integration (HIGH confidence)

---
*Stack research for: Asciicker Rust Port (ASCII game engine on Bevy 0.18)*
*Researched: 2026-02-20*
