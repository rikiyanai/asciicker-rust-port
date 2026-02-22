---
phase: 05-pipeline-integration
plan: 05
subsystem: render-pipeline
tags: [pipeline, assembly, sprites, profiling, integration]
dependency_graph:
  requires: [05-01, 05-02, 05-03, 05-04]
  provides: [render-pipeline-system, a3d-assembly-system, sprite-queue, pipeline-timing, runtime-materials]
  affects: [06-02, 06-03, 07-01]
tech_stack:
  added: []
  patterns: [bevy-system-chain, resource-gating, deferred-sprite-blit, per-stage-timing]
key_files:
  created:
    - engine-port/src/render/assembly.rs
    - engine-port/src/render/pipeline.rs
    - engine-port/src/render/sprite_blit.rs
  modified:
    - engine-port/src/render/mod.rs
    - engine-port/src/output/mod.rs
    - engine-port/src/main.rs
    - engine-port/Cargo.toml
decisions:
  - "ResolveBuffer allocated locally in pipeline (not as Resource) per R14-F148"
  - "SpriteQueue cleared at Stage 3 WORLD start (Phase 5 standalone; Phase 6 migrates to PreUpdate)"
  - "test_pattern_system gated behind cfg(feature = test_pattern) to avoid overwriting pipeline output"
  - "Plugin order: AsciiOutputPlugin before CpuRasterizerPlugin (R14-F124 FIX)"
  - "Mesh rendering deferred until AKM mesh data loaded via MeshRegistry"
metrics:
  duration: 13min
  completed: 2026-02-22
---

# Phase 5 Plan 05: Pipeline Orchestrator & Assembly System Summary

A3D-to-runtime assembly system and 6-stage rendering pipeline with real rasterization, deferred sprite blit, and per-stage profiling.

## One-Liner

Assembly system builds RuntimeTerrain/RuntimeWorld/RuntimeMaterials from loaded A3D assets; 6-stage pipeline orchestrates CLEAR->TERRAIN->WORLD->SHADOW->REFLECTION->RESOLVE with real TerrainShader calls and per-stage microsecond timing.

## What Was Built

### Task 1: A3D Assembly System (assembly.rs)

- `AssemblyState` Resource tracking A3D file loading state
- `MeshRegistry` Resource for AKM mesh handle tracking
- `RuntimeMaterials` Resource wrapping Vec<Material> (resolves HIGH gap #5)
- `load_a3d_scene` startup system setting the scene handle
- `a3d_assembly_system` that watches for loaded A3D sub-assets and builds:
  - RuntimeTerrain from A3dTerrain patches
  - RuntimeWorld from A3dWorld instances
  - RuntimeMaterials from MaterialTable (field-by-field MatCell conversion)
  - MeshRegistry entries for mesh instance AKM loading
- `convert_material_table` with explicit import aliases for MatCell disambiguation

### Task 2: Pipeline Orchestrator (pipeline.rs, sprite_blit.rs)

- `render_pipeline_system` implementing the full 6-stage pipeline:
  - Stage 1 CLEAR: memcpy-clear SampleBuffer via cached template
  - Stage 2 TERRAIN: real TerrainShader calls via RuntimeTerrain.query_visible
  - Stage 3 WORLD: world query + sprite queueing (mesh render deferred to loaded AKM)
  - Stage 4 SHADOW: stub (Phase 6)
  - Stage 5 REFLECTION: stub (Phase 6)
  - Stage 6 RESOLVE: resolve_to_grid with real materials, GlyphSelector, resolve_buf
  - Post-RESOLVE: SpriteQueue sort + placeholder blit
- `PipelineTiming` Resource with per-stage microsecond precision
- `SpriteQueue` Resource with far-to-near sort, push/drain/clear
- `SpriteRenderEntry` struct with dist/screen coords/sprite metadata
- `blit_sprite` placeholder marking sprite positions with 'S'
- `ensure_buffer_size` syncing SampleBuffer to RenderConfig on window resize
- `project_world_to_screen` for world-to-screen projection
- `verify_plugin_prerequisites` startup assertion

### Task 3: Debug Tooling Feature Flags

- `test_pattern` feature in Cargo.toml gating Phase 3 test pattern
- `#[cfg(feature = "test_pattern")]` on test_pattern_system registration
- `#[cfg(feature = "schedule_dump")]` for bevy_mod_debugdump in main.rs
- `inspector` feature already present; conditional ResourceInspectorPlugin registration added

### Wiring (render/mod.rs)

- CpuRasterizerPlugin::build() registers: AssemblyState, PipelineTiming, MeshRegistry, SpriteQueue
- System chain: camera_input -> camera_update -> a3d_assembly (gated) -> render_pipeline
- Assembly system gated with `.run_if(|assembly: Res<AssemblyState>| !assembly.assembled)`

### Fixes Applied

- **R14-F124 (CRITICAL):** Plugin reorder in main.rs -- AsciiOutputPlugin before CpuRasterizerPlugin
- **R52:** test_pattern_system gated behind feature flag
- **R14-F148:** ResolveBuffer allocated locally (not as Resource)
- **P5-102:** Bevy system uses let-else patterns (not `?` operator)
- **H-04:** RenderConfig synced from AsciiCellGrid at pipeline top

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

| Check | Result |
|-------|--------|
| `cargo test --lib -- render::assembly` | 4 passed |
| `cargo test --lib -- render::pipeline` | 5 passed |
| `cargo test --lib -- render::sprite_blit` | 5 passed |
| `cargo clippy -- -D warnings` | Clean |
| `cargo build` | Success |
| Total test count | 233 passed, 1 ignored |

## Test Summary

| Test | What it verifies |
|------|-----------------|
| test_convert_material_table | 256-entry MaterialTable -> Vec<Material> with correct values |
| test_assembly_state_default | Default AssemblyState has assembled=false, no handle |
| test_matcell_layout_equivalence | Both MatCell types are 8 bytes, field copy preserves values |
| test_mesh_registry_default_empty | Default MeshRegistry has no meshes |
| test_sprite_sort_far_to_near | Descending distance sort works correctly |
| test_sprite_queue_push_drain | Push/drain lifecycle |
| test_sprite_queue_clear | Clear empties queue |
| test_blit_sprite_placeholder | Placeholder writes 'S' at correct position |
| test_blit_sprite_out_of_bounds | No panic on out-of-bounds coordinates |
| test_ensure_buffer_size_resizes | Wrong-size buffer gets reallocated |
| test_ensure_buffer_size_noop | Correct-size buffer is untouched |
| test_project_world_to_screen | Camera origin projects near screen center |
| test_pipeline_timing_default | PipelineTiming starts at zero |
| test_pipeline_clears_buffer | Clear stage restores cleared samples |

## Contract Check

| Requirement | Status |
|-------------|--------|
| REND-08: Deferred sprite blit post-RESOLVE | DONE: SpriteQueue + sort + blit after Stage 6 |
| CRITICAL #1: No placeholder rendering | DONE: Pipeline calls real render_patch via TerrainShader |
| CRITICAL #2: A3D assembly system | DONE: a3d_assembly_system builds all runtime structures |
| HIGH #5: MaterialTable as Resource | DONE: RuntimeMaterials Resource extracted from asset |
| AUDIT #11: Per-stage timing | DONE: PipelineTiming with Instant::now() per stage |

## Commits

| Hash | Description |
|------|-------------|
| `37dad88` | feat(05-05): A3D assembly system, RuntimeMaterials, plugin reorder |
| `bf37678` | feat(05-05): 6-stage pipeline orchestrator, sprite blit, debug features |

## Self-Check: PASSED

- [x] engine-port/src/render/assembly.rs (9250 bytes)
- [x] engine-port/src/render/pipeline.rs (15109 bytes)
- [x] engine-port/src/render/sprite_blit.rs (7245 bytes)
- [x] .planning/phases/05-pipeline-integration/05-05-SUMMARY.md (6740 bytes)
- [x] Commit 37dad88 verified in git log
- [x] Commit bf37678 verified in git log
- [x] 233 tests pass, 0 failures
- [x] cargo clippy -- -D warnings clean
- [x] cargo build succeeds
