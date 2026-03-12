# 2026-03-10 Architecture Alignment Handoff

Purpose: measure how aligned the current Rust port is with the intended implementation targets:
- original C++ engine architecture
- Alex Harri shape-matching renderer
- Mage Core output techniques
- Bevy ECS migration

This is an evidence-backed handoff, not a completion claim.

## Alignment Snapshot

| Target | Current Alignment | Evidence | Main Remaining Gaps |
|---|---:|---|---|
| Original C++ engine architecture | 68% | 6-stage CPU pipeline, BSP visibility query, terrain + mesh raster, shadow stage, physics/world/game state all exist | sprite XP blit still placeholder, world reflections still incomplete, original visual parity still not proven end-to-end |
| Alex Harri system | 82% | k-d tree matcher, bounded cache, real `default.json` metadata, external-point directional crunch, circular sampling, runtime-font-derived vectors at resolve | no GPU sampling path, no alphabet switching, threshold/perf tuning still open |
| Mage Core | 30% overall | 4-texture GPU ASCII output pattern is implemented in Bevy | Mage Core engine loop/app/input/present architecture is not adopted |
| Mage Core output layer only | 90% | extract/prepare/render plugin, fg/bg/text/font textures, fullscreen compositing shader, font atlas config | implementation is adapted to Bevy, not direct engine reuse |
| ECS migration | 72% | Bevy plugins/resources/system sets/states are established across gameplay, render, physics, output, network | many subsystems still center on large singleton resources instead of deeper componentization |

## Area Notes

### 1. Original Engine Architecture

Aligned:
- `engine-port/src/render/pipeline.rs` implements the staged CPU render flow.
- `engine-port/src/world/mod.rs` and `engine-port/src/world/bsp.rs` provide BSP-backed visibility.
- `engine-port/src/physics/*` provides fixed-step collision/forces.
- `engine-port/src/game/*` provides game state, loading, weather, and cross-plugin sync.

Still not aligned:
- `engine-port/src/render/sprite_blit.rs` is still the placeholder sprite path, not full XP sprite compositing parity.
- `engine-port/src/render/water.rs` still has outstanding parity work and should remain last per current direction.
- Full original-vs-port deterministic trace comparison is still incomplete because the original runtime is not buildable locally yet.

### 2. Alex Harri System

Aligned:
- `engine-port/src/render/shape_vector.rs` now uses the upstream `default.json` sampling metadata.
- External-point directional crunch is implemented with affects-mapping.
- Circular multi-sampling replaced the old single-point shortcut.
- Runtime glyph vectors are generated from `assets/fonts/cp437_10x16.png`, which closes the earlier font mismatch.

Still not aligned:
- This remains a CPU resolve-stage integration, not the optional GPU sampling pipeline from the Alex Harri reference stack.
- Alphabet selection/runtime switching is not implemented.
- Performance tuning and threshold calibration remain empirical.

### 3. Mage Core

Aligned:
- `engine-port/src/output/gpu_plugin.rs`
- `engine-port/src/output/gpu_types.rs`
- `engine-port/src/output/shader.wgsl`

These clearly implement the Mage Core 4-texture ASCII output pattern inside Bevy:
- font atlas texture
- glyph index texture
- foreground color texture
- background color texture
- fullscreen shader composition

Not aligned:
- The codebase does not use Mage Core's `App` trait, `run()` loop, input model, or present/tick API.
- This is an adaptation of Mage Core rendering techniques, not a general Mage Core engine port.

### 4. ECS

Aligned:
- Bevy state machine and system scheduling are real, not superficial.
- Physics, render, world, terrain, output, network, and game logic all run as plugins/resources/systems.

Still not aligned:
- `RuntimeTerrain`, `RuntimeWorld`, `SampleBuffer`, and several cross-domain resources still centralize large amounts of state.
- Some logic remains subsystem-oriented rather than fully entity/component-driven.

## Why Many Cells Have No Glyphs

This is not just “missing art.” It is also how the current renderer works.

Contributing reasons:
- `engine-port/src/render/shape_vector.rs`: `ShapeVectorGlyphSelector::select_glyph()` intentionally returns `None` for clear cells, underwater cells, and weak matches.
- `engine-port/src/render/pipeline.rs`: when shape-vector returns `None`, the pipeline keeps the resolved glyph already present in `AnsiCell`.
- `engine-port/src/output/shader.wgsl`: the background color still renders even when the glyph is effectively blank/space.

So many visually non-empty cells are color-driven cells with little or no glyph ink, not just failed writes.

## Recommended Next Investigation

1. Quantify glyph occupancy by frame:
   - count `space` / non-space cells
   - count cells where bg differs but glyph is `space`
   - count shape-vector overrides vs fallback cells
2. Compare that against the locked `3a621b8` baseline.
3. Only after that, resume water-specific investigation.

## Handoff Snapshot
- Branch: `planning-audit-normalization`
- HEAD: `f7897e5`
- Completed:
  - Alex Harri subsystem moved to upstream metadata + full external crunch + runtime font vectors (dirty working tree)
  - Stage 4 player shadow ported into render pipeline (dirty working tree)
  - Geometry bbox collision proxy removed; real AKM triangles only (dirty working tree)
  - Font1 centered runtime text path cleaned up for menu/loading (dirty working tree)
- Deferred:
  - sprite XP blit parity (reason: intentionally left until after render/core parity)
  - water parity work (reason: explicitly deprioritized until upstream gaps are closed)
  - original runtime trace comparison (reason: original build still blocked locally)
- Open Risks:
  - current working tree is heavily dirty and mixes unrelated prior work with this pass
  - visual parity is improved but not signed off
  - glyph sparsity is still not numerically characterized
- Resume:
  - `cd /Users/rikihernandez/Downloads/asciicker-rust-port/engine-port && cargo run --release`
