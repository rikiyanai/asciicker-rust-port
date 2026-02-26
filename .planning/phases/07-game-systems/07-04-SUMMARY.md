---
phase: 07-game-systems
plan: 04
subsystem: render/shape-vector-font
tags: [shape-vector, font, glyph-selection, kd-tree, resolve]
dependency_graph:
  requires: [05-04, 07-01, 07-02, 07-03]
  provides: [ShapeVectorMatcher, ShapeVectorGlyphSelector, Font1]
  affects: [render/pipeline.rs, render/mod.rs]
tech_stack:
  added: [kiddo 5.x, lru 0.12]
  patterns: [k-d tree nearest-neighbor, LRU cache, GlyphSelector trait, recolor tables]
key_files:
  created:
    - engine-port/src/render/font.rs
  modified:
    - engine-port/Cargo.toml
    - engine-port/src/render/mod.rs
    - engine-port/src/render/pipeline.rs
    - engine-port/src/render/resolve.rs
    - engine-port/src/render/resolve_bridge.rs
    - engine-port/src/render/terrain_shader.rs
decisions:
  - Use kiddo KdTree<f32, 6> with SquaredEuclidean distance for 6D nearest-neighbor
  - LRU cache with 8192 capacity, storing (glyph, distance) pairs
  - Font1 recolor tables from C++ font1.cpp:243-250 exact values
  - ShapeVectorGlyphSelector constructed per-frame inside pipeline resolve block
  - distance_threshold = 0.05 squared Euclidean for auto_mat fallback
metrics:
  duration: ~12min
  completed: 2026-02-26
  tasks: 2/2
  tests_added: 29
  tests_total: 430
  files_modified: 8
---

# Phase 7 Plan 4: Shape-Vector Glyph Matching and Font1 Summary

6D shape-vector glyph matching via kiddo k-d tree with LRU cache, plus Font1 text rendering with 3 CP437 skins (Grey/Gold/Pink) from C++ font1.cpp recolor tables.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | ShapeVectorMatcher with k-d tree and cache | 1a81b70 | Cargo.toml, shape_vector.rs, mod.rs, resolve.rs, terrain_shader.rs, pipeline.rs |
| 2 | Font1 with 3 skins + pipeline ShapeVectorGlyphSelector wiring | 21bb105 | font.rs, mod.rs, pipeline.rs |

## Implementation Details

### Task 1: ShapeVectorMatcher

- Added `kiddo = "5"` and `lru = "0.12"` to Cargo.toml
- `shape_vector.rs` already existed from merge commit (68e63f9) with full implementation:
  - 95-character six-samples alphabet (printable ASCII 0x20-0x7E)
  - `ShapeVectorMatcher` with `KdTree<f32, 6>`, bounded `LruCache<u32, (u8, f32)>` (8192 cap)
  - `find_glyph_with_distance()` with quantized 30-bit cache key (5 bits/component)
  - `sample_cell_vector()` with bilinear interpolation at 6 sampling positions
  - `sample_to_lightness()` with dual-path mesh/terrain (MESH_FLAG branching)
  - `ShapeVectorGlyphSelector` implementing `GlyphSelector` trait
  - `crunch_vector()` for contrast exaggeration (exponent 1.5)
  - Sky cell guard (CLEAR_HEIGHT check, returns None)
  - Underwater cell guard (water_z comparison, returns None)
- Registered `ShapeVectorMatcher::new_default()` as Bevy Resource in `CpuRasterizerPlugin::build()`
- Also committed prerequisite fixes:
  - resolve.rs: sky palette color fix (RGB555->xterm), reflection terrain dimming (255/400)
  - terrain_shader.rs: per-pixel water clamping (F243 fix)
  - pipeline.rs: removed old F242 water tint code
  - resolve_bridge.rs: test correction for sky blue clear cells

### Task 2: Font1 + Pipeline Wiring

- Created `render/font.rs` (235 lines):
  - `FontSkin` enum: Grey=0, Gold=1, Pink=2
  - `Font1` Resource with `paint_char()`, `paint_string()`, `measure_string()`
  - Recolor tables from C++ font1.cpp:243-250:
    - Gold: [85,85,85]->[255,255,85], [170,170,170]->[255,204,0], [255,255,255]->[255,204,0]
    - Pink: [85,85,85]->[255,153,255], [170,170,170]->[255,0,255], [255,255,255]->[255,51,255]
  - Boundary safety (out-of-bounds coordinates silently ignored)
  - String clipping at grid edges
- Added `pub mod font;` to render/mod.rs
- Wired `ShapeVectorGlyphSelector` into pipeline.rs Step 3:
  - Added `Option<ResMut<ShapeVectorMatcher>>` system parameter
  - When matcher available: constructs `ShapeVectorGlyphSelector` with materials, water_z, distance_threshold
  - When matcher absent: falls back to `AutoMatGlyphSelector`
  - Uses macro_rules for shared resolve loop (avoids code duplication)

## Verification Results

1. `cargo build` -- PASS
2. `cargo test --lib` -- 430 tests pass (18 shape_vector + 11 font + 401 existing)
3. Clippy clean on new/modified files (pre-existing warnings in other files, out of scope)
4. `insert_resource(ShapeVectorMatcher::new_default())` in CpuRasterizerPlugin::build() -- confirmed
5. `ShapeVectorGlyphSelector` in pipeline.rs -- confirmed (3 occurrences)
6. in_state(Playing) gating on RenderSet::Pipeline -- confirmed (via game/mod.rs configure_sets)
7. resolve.rs NOT modified by Task 2 (XP-008 DO-NOT-MODIFY respected)

## Deviations from Plan

### Pre-existing Implementation

**[Observation] shape_vector.rs already existed**
- **Found during:** Task 1 precondition check
- **Issue:** The merge commit 68e63f9 already included the full shape_vector.rs implementation
- **Resolution:** Committed the remaining uncommitted dependencies (Cargo.toml kiddo/lru, mod.rs wiring) and bug fixes (resolve.rs, terrain_shader.rs, pipeline.rs) that were in the working tree
- **Impact:** Task 1 became primarily a "commit existing work" task rather than a full implementation task

### Bundled Bug Fixes

**[Rule 1 - Bug] Committed resolve/terrain/pipeline fixes alongside Task 1**
- **Found during:** Task 1 staging
- **Issue:** Working tree had uncommitted fixes for sky palette, reflection dimming, per-pixel water clamping, and F242 water tint removal
- **Fix:** Bundled into Task 1 commit as they are prerequisites for correct shape-vector rendering
- **Files:** resolve.rs, terrain_shader.rs, pipeline.rs, resolve_bridge.rs
- **Commit:** 1a81b70

## Decisions Made

1. **Used macro_rules for resolve loop** -- avoids duplicating the per-cell RGBA conversion loop for the two glyph selector paths (shape-vector vs auto_mat)
2. **Font1 default_fg = [170,170,170]** -- matches the Grey recolor source in C++ font1.cpp (standard VGA terminal silver/grey)
3. **distance_threshold = 0.05** -- squared Euclidean, tunable at runtime. Conservative value that preserves auto_mat in uniform regions
4. **Font1 is Resource-only, no systems** -- calling systems (HUD, chat) enforce ordering after render_pipeline_system

## Contract Verification

- [x] VIS-01 (Alex Harri 6D shape-vector): ShapeVectorMatcher + ShapeVectorGlyphSelector wired into pipeline
- [x] VIS-03 (Font system, 3 skins): Font1 with Grey/Gold/Pink skins and paint API
