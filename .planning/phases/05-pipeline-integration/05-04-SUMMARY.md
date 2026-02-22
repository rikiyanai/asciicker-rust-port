---
phase: 05-pipeline-integration
plan: 04
subsystem: render
tags: [terrain-shader, mesh-shader, resolve-bridge, raster-shader, xterm-palette, glyph-selector]

# Dependency graph
requires:
  - phase: 04-cpu-rasterizer-core
    provides: RasterShader trait, rasterize(), resolve(), SampleBuffer, Sample, AnsiCell, Material, auto_mat_lookup, rgb2pal, rgb8_to_rgb5
  - plan: 05-01
    provides: RuntimePatch with height/visual/diag/dark fields
  - plan: 05-02
    provides: RuntimeWorld with AkmMesh instances
  - plan: 05-03
    provides: GameCamera with view_tm
provides:
  - TerrainShader implementing RasterShader (writes material indices, spare=0)
  - MeshShader implementing RasterShader (writes RGB555, spare=MESH_FLAG)
  - render_patch function (terrain patch triangulation to rasterizer)
  - render_mesh function (mesh instance rendering to rasterizer)
  - resolve_to_grid function (AnsiCell xterm-256 to AsciiCellGrid RGBA bridge)
  - GlyphSelector trait and AutoMatGlyphSelector default
  - transform_vertex shared utility (render/math.rs)
  - XTERM_256_PALETTE constant (full 256-color RGB lookup)
  - AsciiCellGrid::new() standalone constructor
affects: [05-05-PLAN, 05-06-PLAN, 06-03-PLAN, 07-04-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns: [RasterShader concrete implementations, xterm-256 palette bridge, pluggable glyph selection trait]

key-files:
  created:
    - engine-port/src/render/math.rs
    - engine-port/src/render/terrain_shader.rs
    - engine-port/src/render/mesh_shader.rs
    - engine-port/src/render/resolve_bridge.rs
  modified:
    - engine-port/src/render/mod.rs
    - engine-port/src/output/ascii_cell_grid.rs

key-decisions:
  - "TerrainShader uses inline depth test (sample.height > z || CLEAR_HEIGHT), not depth_test_ro() which has semantic inversion"
  - "TerrainShader writes spare=0 (no MESH_FLAG) so resolve takes material path"
  - "MeshShader writes spare=MESH_FLAG so resolve takes auto_mat path"
  - "render_patch subdivides each HEIGHT_CELLS quad into vis_per_height x vis_per_height visual sub-quads"
  - "resolve_to_grid uses generic GlyphSelector<G> (not dyn) for monomorphization (P5-306)"
  - "XTERM_256_PALETTE uses evenly-spaced levels [0,51,102,153,204,255] to match rgb2pal round-tripping"
  - "transform_vertex extracted to render/math.rs as shared utility (F038/F039 decision)"

patterns-established:
  - "Terrain spare=0 vs mesh spare=MESH_FLAG resolve path selection"
  - "GlyphSelector trait with &mut self for Phase 7 stateful implementations"
  - "resolve_buf parameter pattern: caller provides reusable Vec<AnsiCell> to avoid per-frame allocation"

requirements-completed: [REND-08]

# Metrics
duration: 11min
completed: 2026-02-22
---

# Phase 5 Plan 04: Shader Implementations and Resolve Bridge Summary

**TerrainShader and MeshShader as concrete RasterShader implementations calling Phase 4 rasterize(), plus xterm-256-to-RGBA resolve bridge with pluggable GlyphSelector trait**

## Performance

- **Duration:** 11 min
- **Started:** 2026-02-22T21:19:28Z
- **Completed:** 2026-02-22T21:30:22Z
- **Tasks:** 2
- **Files created:** 4
- **Files modified:** 2
- **Tests added:** 31 (6 math + 6 terrain_shader + 8 resolve_bridge palette + 4 resolve_bridge functional + 5 terrain_shader functional + 2 mesh_shader)

## Accomplishments
- TerrainShader rasterizes terrain patch triangles with material indices and shadow modulation
- MeshShader rasterizes mesh instance faces with averaged vertex colors as RGB555
- resolve_to_grid converts AnsiCell xterm-256 palette indices to AsciiCellGrid RGBA colors
- GlyphSelector trait enables Phase 7 shape-vector extensibility
- Full xterm-256 palette constant with 6x6x6 cube and grayscale ramp
- Shared transform_vertex utility for world-to-screen projection
- AsciiCellGrid::new() standalone constructor for unit tests
- CRITICAL gap #1 (placeholder rendering) and CRITICAL gap #3 (format mismatch) resolved

## Task Commits

Each task was committed atomically:

1. **Task 1: TerrainShader, render_patch, and transform_vertex** - `537eb82` (feat)
2. **Task 2: MeshShader, resolve_to_grid bridge, and module wiring** - `6a53ab0` (feat)

## Files Created/Modified
- `engine-port/src/render/math.rs` - transform_vertex shared utility, 6 tests
- `engine-port/src/render/terrain_shader.rs` - TerrainShader, render_patch, 6 tests
- `engine-port/src/render/mesh_shader.rs` - MeshShader, render_mesh, 2 tests
- `engine-port/src/render/resolve_bridge.rs` - GlyphSelector trait, resolve_to_grid, XTERM_256_PALETTE, 11 tests
- `engine-port/src/render/mod.rs` - Added pub mod math, terrain_shader, mesh_shader, resolve_bridge
- `engine-port/src/output/ascii_cell_grid.rs` - Added AsciiCellGrid::new() constructor

## Decisions Made
- TerrainShader uses inline depth test pattern (not depth_test_ro which has semantic inversion)
- Shadow modulation halves diffuse for shadowed cells via patch.dark bitmask
- render_patch subdivides each height-cell quad into 2x2 visual sub-quads for correct material mapping
- MeshShader averages per-vertex colors for face color, converts to RGB555
- resolve_to_grid uses Vec::resize() (not reserve) to match resolve()'s debug_assert_eq on output.len()
- Generic GlyphSelector (not dyn) for zero-cost monomorphization
- XTERM_256_PALETTE levels match rgb2pal's (c+25)/51 formula for correct round-tripping

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- TerrainShader and MeshShader ready for pipeline orchestrator (05-05) to call render_patch/render_mesh
- resolve_to_grid ready for pipeline system to bridge Phase 4 output to Phase 3 GPU input
- GlyphSelector trait ready for Phase 7 shape-vector implementation
- All 219 tests passing (31 new + 188 existing)

## Self-Check: PASSED

- [x] engine-port/src/render/math.rs - FOUND
- [x] engine-port/src/render/terrain_shader.rs - FOUND
- [x] engine-port/src/render/mesh_shader.rs - FOUND
- [x] engine-port/src/render/resolve_bridge.rs - FOUND
- [x] Commit 537eb82 (Task 1) - FOUND
- [x] Commit 6a53ab0 (Task 2) - FOUND
- [x] cargo test --lib: 219 passed, 0 failed
- [x] cargo clippy -- -D warnings: clean
- [x] Phase 4 files (rasterizer.rs, resolve.rs, sample_buffer.rs) NOT modified

---
*Phase: 05-pipeline-integration*
*Completed: 2026-02-22*
