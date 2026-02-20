# Phase 1: Foundation - Context

**Gathered:** 2026-02-20
**Status:** Ready for planning

<domain>
## Phase Boundary

A compiling Bevy 0.18 project with the correct plugin architecture, coordinate conventions, and ECS resource/entity mapping so that all subsequent phases build on a solid base. This phase delivers project scaffolding — no game logic, no rendering output, no asset loading.

</domain>

<decisions>
## Implementation Decisions

### Project layout
- Start fresh in `engine-port/` directory — do NOT salvage `asciicker-rust/` skeleton (structural problems: wrong crate type, full Bevy features, missing modules, stale build artifacts)
- Single crate (one Cargo.toml, modules under src/) — not a Cargo workspace. Split later if needed.
- Follow Bevy community best practices for code organization:
  - Plugin-per-subsystem: each plugin registers its own systems, components, and resources
  - First-level modules own their types — avoid cross-module component reuse
  - Events for inter-module communication — don't couple plugins directly
  - Resources for shared data singletons (SampleBuffer, AsciiCellGrid)

### Plugin architecture
- Register all 7 plugins as stubs in Phase 1: AssetLoaderPlugin, WorldPlugin, TerrainPlugin, CpuRasterizerPlugin, AsciiOutputPlugin, PhysicsPlugin, CharacterPlugin, GamePlugin
- Terrain is a SEPARATE plugin from World (separate data owners that couple only during rendering)
- Plugin communication: Resources + explicit system ordering (.before/.after) — not events for data flow
- Each plugin is a Bevy Plugin struct implementing the Plugin trait

### Coordinate enforcement
- Claude's discretion on enforcement strictness — pick whichever approach causes fewer bugs given the C++ porting context
- Z-is-UP convention throughout — Claude decides whether to use Z-up everywhere with conversion at Bevy boundary, or Bevy's Y-up with conversion during loading
- Use glam types directly (Vec3, Mat4, Quat) — no thin wrappers over glam
- Success criteria require `const UP: Vec3 = Vec3::Z` and a compile-time type alias at minimum

### ECS resource design
- SampleBuffer dimensions configurable via a RenderConfig resource (not hardcoded)
- Default resolution: 240x135 ASCII (SampleBuffer = 480x270 at 2x supersampling)
- SampleBuffer: flat Vec<Sample> with index methods (sample_at/sample_at_mut) — match C++ layout for performance
- AsciiCellGrid: GPU-ready from start — structure data for Mage Core's 4-texture approach (separate char_index, fg, bg arrays) to avoid restructuring in Phase 3
- Testing: both unit tests (Bevy World directly, fast CI) AND integration tests (full Bevy App, end-to-end resource flow)

### Claude's Discretion
- Shared types location (likely src/core/ or similar)
- Coordinate enforcement approach specifics (newtype vs type alias vs conversion layer — whatever prevents the most bugs)
- Z-up vs Y-up internal convention choice
- System ordering details within and between plugins
- Exact module file organization within each plugin directory

</decisions>

<specifics>
## Specific Ideas

- Follow Bevy community patterns from:
  - [Bevy Cheat Book - Plugins](https://bevy-cheatbook.github.io/programming/plugins.html)
  - [Bevy Resources](https://taintedcoders.com/bevy/resources)
  - [Bevy Best Practices](https://github.com/tbillington/bevy_best_practices)
  - [Bevy Code Organization](https://taintedcoders.com/bevy/code-organization)
- The rendering pipeline data flow is: Terrain/World systems write to SampleBuffer -> RESOLVE reads SampleBuffer, picks glyph + colors, writes to AsciiCellGrid -> Mage Core GPU shader reads AsciiCellGrid + font atlas, renders to screen
- Plugin split (Terrain vs World) has zero effect on Mage Core or Alex Harri RESOLVE — they only see SampleBuffer and AsciiCellGrid

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-02-20*
