# Phase 1: Foundation - Research

**Researched:** 2026-02-20
**Domain:** Bevy 0.18 project setup, plugin architecture, ECS resources, coordinate conventions
**Confidence:** HIGH

## Summary

Phase 1 establishes a compiling Bevy 0.18 project in `engine-port/` with a plugin-per-subsystem architecture, Z-is-UP coordinate convention, and two core ECS resources (SampleBuffer and AsciiCellGrid). The existing `asciicker-rust/` skeleton is NOT salvageable (wrong crate type, full Bevy features, wrong audio version, missing modules).

Bevy 0.18 introduced cargo feature collections (`2d`, `3d`, `ui`) designed for `default-features = false`. For our custom CPU rasterizer, the correct feature set is `2d_api` (2D API without Bevy's renderer) + `bevy_render` (core GPU access for our output texture) + `bevy_core_pipeline` (cameras, basic pipeline) + `bevy_shader` (WGSL shader asset loading). This gives us ECS, windowing, input, and the render world infrastructure without pulling in Bevy's default sprite renderer.

Bevy uses Y-up internally (right-handed, +Y up, -Z forward). The C++ Asciicker engine uses Z-up. The recommended approach: use Z-up types in game code (matching C++ data), convert at the Bevy rendering boundary. A `const UP: Vec3 = Vec3::Z` constant plus a conversion module establishes this convention.

**Primary recommendation:** Create a fresh `engine-port/` crate with `default-features = false`, minimal feature set, 8 stub plugins, coordinate convention module, and two core resources -- verified by `cargo build` + `cargo test`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Start fresh in `engine-port/` directory -- do NOT salvage `asciicker-rust/` skeleton
- Single crate (one Cargo.toml, modules under src/) -- not a Cargo workspace
- Plugin-per-subsystem: each plugin registers its own systems, components, and resources
- First-level modules own their types -- avoid cross-module component reuse
- Events for inter-module communication -- don't couple plugins directly
- Resources for shared data singletons (SampleBuffer, AsciiCellGrid)
- Register all 8 plugins as stubs: AssetLoaderPlugin, WorldPlugin, TerrainPlugin, CpuRasterizerPlugin, AsciiOutputPlugin, PhysicsPlugin, CharacterPlugin, GamePlugin
- Terrain is a SEPARATE plugin from World
- Plugin communication: Resources + explicit system ordering (.before/.after)
- Each plugin is a Bevy Plugin struct implementing the Plugin trait
- Use glam types directly (Vec3, Mat4, Quat) -- no thin wrappers
- Success criteria require `const UP: Vec3 = Vec3::Z` and a compile-time type alias at minimum
- SampleBuffer dimensions configurable via a RenderConfig resource
- Default resolution: 240x135 ASCII (SampleBuffer = 480x270 at 2x supersampling)
- SampleBuffer: flat Vec<Sample> with index methods -- match C++ layout
- AsciiCellGrid: GPU-ready from start (separate char_index, fg, bg arrays for Mage Core 4-texture approach)
- Both unit tests AND integration tests required

### Claude's Discretion
- Shared types location (likely src/core/ or similar)
- Coordinate enforcement approach specifics (newtype vs type alias vs conversion layer)
- Z-up vs Y-up internal convention choice
- System ordering details within and between plugins
- Exact module file organization within each plugin directory

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FOUND-01 | Project compiles with Bevy 0.18 using `default-features = false` and custom feature set | Bevy 0.18 feature collections verified: `2d_api` + `bevy_render` + `bevy_core_pipeline` + `bevy_shader` + `default_app` + `default_platform` |
| FOUND-02 | Plugin-per-subsystem architecture established (8 plugins) | Bevy Plugin trait pattern verified via Context7: `impl Plugin for X { fn build(&self, app: &mut App) }` |
| FOUND-03 | Coordinate system convention documented and enforced (Z is UP) | Bevy is Y-up internally; C++ is Z-up; conversion at render boundary recommended |
| FOUND-04 | ECS resource/entity mapping defined (SampleBuffer and AsciiCellGrid as Resources) | `#[derive(Resource)]` pattern verified; `init_resource` / `insert_resource` APIs confirmed |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy | 0.18.0 | Game engine: ECS, windowing, input, render world | Decision D001; latest stable with feature collections |
| glam | (via bevy) | Linear algebra (Vec3, Mat4, Quat) | Bevy's native math; no separate dependency needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| thiserror | 2.0 | Error type derivation | Custom error types in parsers/loaders |
| anyhow | 1.0 | Error propagation in binary | main.rs and integration tests |

### Not Needed in Phase 1
| Library | Why Not |
|---------|---------|
| bevy_kira_audio | Phase 7 (audio) |
| serde / serde_json | No config files yet |
| flate2 | Phase 2 (XP decompression) |
| proptest / goldenfile / insta | Phase 2+ (golden-file tests) |

### Bevy Feature Configuration
```toml
[dependencies]
bevy = { version = "0.18.0", default-features = false, features = [
    "2d_api",            # 2D API without default renderer
    "bevy_render",       # Core GPU access for texture output
    "bevy_core_pipeline",# Cameras, basic render pipeline
    "bevy_shader",       # WGSL shader asset loading (Phase 3)
    "default_app",       # App framework, scheduling
    "default_platform",  # Platform support (winit, etc.)
] }
```

**Why these features:**
- `2d_api` gives us sprite types, transforms, and the 2D coordinate system without Bevy's sprite renderer (we use our own CPU rasterizer)
- `bevy_render` is required to create GPU textures for our ASCII output
- `bevy_core_pipeline` provides camera infrastructure
- `bevy_shader` enables loading WGSL shaders as assets (needed Phase 3)
- `default_app` and `default_platform` provide the application framework and windowing

## Architecture Patterns

### Recommended Project Structure
```
engine-port/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point: App::new() + plugin registration
│   ├── lib.rs               # Library root: pub mod declarations
│   ├── core/                # Shared types, coordinate conventions
│   │   ├── mod.rs
│   │   └── coords.rs        # Z-up convention, conversion utilities
│   ├── render/              # CpuRasterizerPlugin
│   │   └── mod.rs           # SampleBuffer resource, stub systems
│   ├── output/              # AsciiOutputPlugin
│   │   └── mod.rs           # AsciiCellGrid resource, stub systems
│   ├── asset_loader/        # AssetLoaderPlugin
│   │   └── mod.rs           # Stub
│   ├── world/               # WorldPlugin
│   │   └── mod.rs           # Stub
│   ├── terrain/             # TerrainPlugin
│   │   └── mod.rs           # Stub
│   ├── physics/             # PhysicsPlugin
│   │   └── mod.rs           # Stub
│   ├── character/           # CharacterPlugin
│   │   └── mod.rs           # Stub
│   └── game/                # GamePlugin
│       └── mod.rs           # Stub
└── tests/
    └── integration/
        └── resource_flow.rs # SampleBuffer -> AsciiCellGrid test
```

### Pattern 1: Plugin Stub
**What:** Each subsystem is a Bevy Plugin with its own module
**When to use:** Every subsystem in the engine
**Example:**
```rust
// Source: Context7 - Bevy Plugin trait
use bevy::prelude::*;

pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SampleBuffer>();
        // Systems added in later phases
    }
}
```

### Pattern 2: ECS Resource with Default
**What:** Global singleton data accessed by systems
**When to use:** SampleBuffer, AsciiCellGrid, RenderConfig
<!-- **P1-004 FIX:** The entire code example below is SUPERSEDED by Phase 4 implementation.
Actual formula is `2 * config.ascii_width + 4`. RenderConfig has no `supersample_factor` field.
See Phase 4 implementation for the authoritative API. -->
**Example:**
```rust
// Source: Context7 - Bevy Resource derive
use bevy::prelude::*;

#[derive(Resource)]
pub struct RenderConfig {
    pub ascii_width: u32,
    pub ascii_height: u32,
    pub supersample_factor: u32,  // NOTE: field does not exist in actual impl — see P1-004 FIX
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            ascii_width: 240,
            ascii_height: 135,
            supersample_factor: 2,
        }
    }
}

#[derive(Resource)]
pub struct SampleBuffer {
    pub width: u32,
    pub height: u32,
    pub samples: Vec<Sample>,
}

impl FromWorld for SampleBuffer {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<RenderConfig>();
        let w = config.ascii_width * config.supersample_factor;
        let h = config.ascii_height * config.supersample_factor;
        Self {
            width: w,
            height: h,
            samples: vec![Sample::default(); (w * h) as usize],
        }
    }
}
// **P1-004 FIX:** SUPERSEDED: Actual formula is `2 * config.ascii_width + 4`.
// RenderConfig has no `supersample_factor` field. See Phase 4 implementation.
// The code example above is for planning reference only and does not reflect
// the implemented API.
```

### Pattern 3: Coordinate Convention Module
**What:** Central place for Z-up convention and Bevy Y-up conversion
**When to use:** All spatial data in the engine
**Example:**
```rust
use bevy::math::Vec3;

/// Asciicker uses Z-up coordinate system.
/// Bevy uses Y-up. Convert at render boundary.
pub const UP: Vec3 = Vec3::Z;
pub const FORWARD: Vec3 = Vec3::Y;  // In Asciicker Z-up: forward is +Y
pub const RIGHT: Vec3 = Vec3::X;

/// Type alias for documentation: marks a Vec3 as being in game-space (Z-up)
pub type GameVec3 = Vec3;

/// Convert from game space (Z-up) to Bevy render space (Y-up)
#[inline]
pub fn game_to_bevy(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, -v.y)
}

/// Convert from Bevy render space (Y-up) to game space (Z-up)
#[inline]
pub fn bevy_to_game(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}
```

### Anti-Patterns to Avoid
- **Full Bevy features:** `bevy = "0.18.0"` pulls in 3D, PBR, GLTF, etc. (700+ crates). Use `default-features = false`.
- **cdylib crate type:** The skeleton uses `["cdylib", "rlib"]` which is for FFI libraries, not game binaries. Use default (binary + lib).
- **Cross-module resource ownership:** Don't let WorldPlugin own SampleBuffer. Each resource belongs to one plugin.
- **System coupling:** Don't call into another plugin's internals. Use resources as the data contract.
- **Y-up throughout:** Don't convert all C++ data to Y-up on load. Keep Z-up in game logic, convert only at Bevy camera/transform boundary.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Linear algebra | Custom Vec3/Mat4 | glam (via bevy::math) | Battle-tested, SIMD optimized, Bevy-native |
| Windowing | Custom window creation | Bevy + winit (via default_platform) | Cross-platform, event handling included |
| ECS | Custom entity system | Bevy ECS | Archetype storage, parallel systems, proven |
| App lifecycle | Custom game loop | Bevy App::run() | Schedule, plugin ordering, fixed timestep built-in |

## Common Pitfalls

### Pitfall 1: Feature Bloat
**What goes wrong:** Using `bevy = "0.18.0"` without `default-features = false` compiles 700+ crates including PBR, GLTF, 3D mesh rendering.
**Why it happens:** Bevy's defaults include everything for a full 3D game.
**How to avoid:** Always use `default-features = false` with explicit feature list.
**Warning signs:** >5 minute compile times, "bevy_pbr" in dependency tree.

### Pitfall 2: Coordinate System Confusion
**What goes wrong:** Z-up game data rendered with Y-up Bevy transforms produces rotated/inverted scenes.
**Why it happens:** Bevy is Y-up, C++ Asciicker is Z-up, easy to forget conversion.
**How to avoid:** Central `coords.rs` module with conversion functions. Use `GameVec3` type alias to mark game-space vectors. Convert at render boundary only.
**Warning signs:** Objects appearing sideways, camera looking at ground, terrain rotated 90 degrees.

### Pitfall 3: Resource Initialization Order
**What goes wrong:** `SampleBuffer::from_world` panics because `RenderConfig` doesn't exist yet.
**Why it happens:** Bevy's `init_resource` calls `FromWorld` immediately; if a resource depends on another, it must be initialized after.
**How to avoid:** Register `RenderConfig` first (in the plugin that owns it), then `SampleBuffer` (which depends on it).
**Warning signs:** Panics at startup: "Resource not found: RenderConfig".

### Pitfall 4: Plugin Registration Order
**What goes wrong:** Systems from one plugin can't find resources from another.
**Why it happens:** Bevy processes plugin builds in order, but system execution follows schedules.
**How to avoid:** Resources should be inserted during `build()`. System ordering uses `.before()` / `.after()` or system sets.
**Warning signs:** "Resource not found" panics during first frame.

### Pitfall 5: Wrong Crate Type
**What goes wrong:** `cdylib` crate type prevents `cargo run` from working as expected for a game binary.
**Why it happens:** The skeleton was set up for FFI export, not a standalone game.
**How to avoid:** Remove `[lib] crate-type` entirely, or use just `lib`. The binary comes from `src/main.rs`.
**Warning signs:** `cargo run` not finding main, or producing a .so/.dylib instead of executable.

## Code Examples

### Minimal Bevy 0.18 App with Custom Features
```rust
// Source: Bevy 0.18 release notes + Context7
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)  // Uses features from Cargo.toml
        .add_plugins((
            AssetLoaderPlugin,
            WorldPlugin,
            TerrainPlugin,
            CpuRasterizerPlugin,
            AsciiOutputPlugin,
            PhysicsPlugin,
            CharacterPlugin,
            GamePlugin,
        ))
        .run();
}
```

### Resource Access in Systems
```rust
// Source: Context7 - Bevy ECS Resource
fn debug_sample_buffer(buffer: Res<SampleBuffer>, grid: Res<AsciiCellGrid>) {
    info!(
        "SampleBuffer: {}x{} ({} samples), AsciiCellGrid: {}x{} ({} cells)",
        buffer.width, buffer.height, buffer.samples.len(),
        grid.width, grid.height, grid.cells_count(),
    );
}
```

### Integration Test: Resource Flow
```rust
// Source: Bevy test patterns
use bevy::prelude::*;

#[test]
fn sample_buffer_and_ascii_grid_coexist() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<RenderConfig>();
    app.init_resource::<SampleBuffer>();
    app.init_resource::<AsciiCellGrid>();

    app.update();

    let buffer = app.world().resource::<SampleBuffer>();
    let grid = app.world().resource::<AsciiCellGrid>();

    assert_eq!(buffer.width, 480);  // 240 * 2
    assert_eq!(buffer.height, 270); // 135 * 2
    assert_eq!(grid.width, 240);
    assert_eq!(grid.height, 135);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Full default features | Feature collections (2d, 3d, ui) | Bevy 0.18 (Jan 2026) | 2-5x faster compile with `default-features = false` |
| Single Default feature set | `2d_api` without renderer | Bevy 0.18 | Custom renderers can use 2D API without Bevy's sprite renderer |
| `PluginGroup` for grouping | Tuple syntax `(A, B, C)` | Bevy 0.14+ | Simpler plugin registration |

## Open Questions

1. **Exact `bevy_shader` necessity in Phase 1**
   - What we know: WGSL shaders are needed in Phase 3 (GPU output)
   - What's unclear: Whether `bevy_shader` is needed now or can be added later
   - Recommendation: Include it now -- it's lightweight and avoids feature-flag churn

2. **AsciiCellGrid GPU-readiness**
   - What we know: Phase 3 needs separate char_index, fg, bg arrays for 4-texture approach
   - What's unclear: Exact byte layout needed by the WGSL shader
   - Recommendation: Use separate `Vec<u16>` (char_index), `Vec<[u8; 4]>` (fg RGBA), `Vec<[u8; 4]>` (bg RGBA). This maps directly to GPU textures.

3. **DefaultPlugins vs MinimalPlugins + explicit additions**
   - What we know: With `default-features = false`, `DefaultPlugins` only adds what's available from features
   - What's unclear: Whether `DefaultPlugins` respects `2d_api` correctly or adds unnecessary plugins
   - Recommendation: Use `DefaultPlugins` with the constrained feature set. If it adds unwanted plugins, switch to `MinimalPlugins` + explicit additions.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_bevy` - Plugin trait, Resource derive, system patterns
- [Bevy 0.18 Release Notes](https://bevy.org/news/bevy-0-18/) - Feature collections, cargo features
- [Bevy 0.18 Cargo.toml](https://docs.rs/crate/bevy/latest/source/Cargo.toml) - Feature definitions
- [Bevy Feature Flags](https://lib.rs/crates/bevy/features) - Full feature list including `bevy_shader`

### Secondary (MEDIUM confidence)
- [Bevy Cheat Book - Coordinate System](https://bevy-cheatbook.github.io/fundamentals/coords.html) - Y-up confirmed
- [Bevy Discussions #1979](https://github.com/bevyengine/bevy/discussions/1979) - Coordinate system rationale

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Bevy 0.18 features verified via official Cargo.toml and release notes
- Architecture: HIGH - Plugin and Resource patterns verified via Context7 with code examples
- Pitfalls: HIGH - Feature bloat and coordinate issues well-documented in community

**Research date:** 2026-02-20
**Valid until:** 2026-03-20 (Bevy stable, 30-day validity)
