# 2026-03-11 Shape-Vector Occupancy Tuning

## Purpose

Use the user-approved 360 orbit capture as a stable renderer comparison run while reducing:

- colored blank cells
- accepted `space` overrides
- threshold-rejected high-contrast terrain cells

This is upstream renderer work. Water-specific tuning stays deferred until glyph occupancy and edge readability are under control.

## Comparison Artifacts

- Baseline working orbit capture:
  - `artifacts/baselines/orbit-2026-03-11-current`
- First post-fallback-policy replay:
  - `artifacts/baselines/orbit-2026-03-11-postfallback-debug`
- Second post-fallback-policy replay:
  - `artifacts/baselines/orbit-2026-03-11-postfallback2-debug`
- Experimental adaptive-threshold replay:
  - `artifacts/baselines/orbit-2026-03-11-adaptive-threshold-debug`
- Stabilized low-chaos replay:
  - `artifacts/baselines/orbit-2026-03-11-stabilized-debug`
- Locked canonical snapshot baseline remains:
  - `artifacts/baselines/backup-3a621b8-run2`

## Baseline Finding

In `artifacts/baselines/orbit-2026-03-11-current/frame_000001.json`:

- `threshold_skip_cells = 2704`
- `fallback_space_cells = 2504`
- `final_space_cells = 2611`
- `colored_space_cells = 2611`
- `selector_override_cells = 789`

The highest-signal colored blank samples showed that some cells already had valid resolve glyphs like `,`, `%`, or `:` but the shape-vector pass replaced them with `space`.

Representative evidence:

- `(70, 11)`:
  - `resolve_glyph = 44` (`,`)
  - `final_glyph = 32` (`space`)
  - labels: `material_path`, `has_normal_terrain`, `shape_vector_override`, `shape_colored_space`
- `(7, 0)`:
  - `resolve_glyph = 37` (`%`)
  - `final_glyph = 32` (`space`)
  - labels: `material_path`, `has_normal_terrain`, `shape_vector_override`, `shape_colored_space`

This proved one concrete sparse-glyph failure mode:

- the shape-vector matcher was sometimes accepting `space` as the nearest glyph and erasing existing structural resolve output

## Fix 1 Applied

Code change:

- `engine-port/src/render/pipeline.rs`

Behavior change:

- when the shape-vector pass selects `space`
- and the resolve stage already produced a non-space glyph
- and the cell is high-contrast
- keep the resolve glyph instead of allowing a `space` override

Debug metadata change:

- `engine-port/src/render/debug_cells.rs`
- `engine-port/src/output/capture.rs`

Added debug label:

- `shape_preserved_resolve`

## Fix 1 Result

Comparing `frame_000001.json` before vs after:

- `threshold_skip_cells`: `2704 -> 2703`
- `fallback_space_cells`: `2504 -> 2504`
- `final_space_cells`: `2611 -> 2496`
- `colored_space_cells`: `2611 -> 2496`
- `final_non_space_cells`: `3149 -> 3264`
- `selector_override_cells`: `789 -> 675`

Net effect:

- `115` fewer final colored blank cells on frame 1
- `114` fewer shape-vector overrides on frame 1

Aggregate over the full 120-frame replay:

- average `final_space_cells` delta: `-397.8` per frame
- average `colored_space_cells` delta: `-397.8` per frame
- average `selector_override_cells` delta: `-387.2` per frame
- average `fallback_space_cells` delta: `-12.6` per frame
- average `threshold_skip_cells` delta: `+10.2` per frame

Interpretation:

- the accepted-space override fix materially improved occupancy
- the remaining sparsity is now concentrated in threshold-rejected cells and true resolve-space fallbacks

## Fix 2 Applied

Code change:

- `engine-port/src/render/pipeline.rs`
- `engine-port/src/render/shape_vector.rs`

Behavior change:

- for high-contrast threshold-rejected cells where resolve had already collapsed to `space`
- inspect the underlying 2x2 sample block
- if it is solid terrain, recover a structural glyph from the dominant material sample instead of leaving the final glyph blank

Also adjusted stats/debug accounting so fallback-space vs fallback-structural reflects the actual final glyph, not only the pre-fallback resolve glyph.

## Fix 2 Result

Comparing `frame_000001.json` before vs after Fix 2:

- `threshold_skip_cells`: `2704 -> 2703`
- `fallback_space_cells`: `2504 -> 2259`
- `final_space_cells`: `2611 -> 2259`
- `colored_space_cells`: `2611 -> 2259`
- `final_non_space_cells`: `3149 -> 3501`
- `selector_override_cells`: `789 -> 675`

Aggregate over the full 120-frame replay:

- average `final_space_cells` delta: `-582.9` per frame
- average `colored_space_cells` delta: `-582.9` per frame
- average `fallback_space_cells` delta: `-210.8` per frame
- average `selector_override_cells` delta: `-387.2` per frame
- average `threshold_skip_cells` delta: `+10.2` per frame

Interpretation:

- occupancy is materially better than the original orbit capture
- the biggest remaining issue is not accepted-space erasure anymore
- it is the threshold policy itself, which still rejects too many cells even after structural rescue

## What Remains

The remaining top colored blank samples are no longer mostly accepted-space overrides. They are now dominated by:

- `shape_skip_threshold`
- `shape_fallback_space`
- `shape_colored_space`

Representative remaining cells in `artifacts/baselines/orbit-2026-03-11-postfallback-debug/frame_000001.json`:

- `(52, 34)`
- `(52, 35)`
- `(52, 0)`
- `(54, 0)`

The remaining worst cells still show:

- `resolve_glyph = 32`
- `final_glyph = 32`
- contrast still high
- very large `shape_distance`

So the next renderer step is narrower now:

1. tune threshold policy for high-contrast non-clear terrain cells
2. decide whether the threshold should become contrast-aware instead of purely distance-based
3. only after that, continue water-specific work

## Current Decision

Use `artifacts/baselines/orbit-2026-03-11-current` as the active working comparison run for renderer tuning, while still keeping `backup-3a621b8-run2` as the locked canonical regression baseline until manual user sign-off.

## Adaptive Threshold Experiment

An adaptive threshold path was implemented as a live tuning option:

- controls:
  - `7` / `8` adjust adaptive boost
  - `F11` toggles adaptive threshold

Result against `postfallback2`:

- threshold rejects improved strongly
- blank-cell improvement was tiny
- overrides increased too much to make it a good default

Full 120-frame comparison of `artifacts/baselines/orbit-2026-03-11-adaptive-threshold-debug` vs `artifacts/baselines/orbit-2026-03-11-postfallback2-debug`:

- average `threshold_skip_cells` delta: `-433.7` per frame
- average `final_space_cells` delta: `-2.8` per frame
- average `colored_space_cells` delta: `-2.8` per frame
- average `selector_override_cells` delta: `+430.9` per frame

Decision:

- keep adaptive threshold as an experimental live control
- leave it disabled by default
- continue tuning from the stronger `postfallback2` default path

## Edge Stability Pass

One more renderer pass tightened override policy:

- when the shape-vector pass wants to swap one non-space structural glyph for another
- and the match is only moderately confident
- preserve the resolve glyph instead of allowing a chaotic swap

Result of `artifacts/baselines/orbit-2026-03-11-stabilized-debug` vs `artifacts/baselines/orbit-2026-03-11-postfallback2-debug`:

- average `threshold_skip_cells` delta: `0.0` per frame
- average `final_space_cells` delta: `-0.008` per frame
- average `colored_space_cells` delta: `-0.008` per frame
- average `selector_override_cells` delta: `-296.2` per frame

Interpretation:

- occupancy stayed effectively unchanged
- low-confidence structural churn dropped materially
- this became the best default path before the architecture audit

## Semantic Gating Pass

The architecture audit then changed the integration policy.

Code change:

- `engine-port/src/render/pipeline.rs`
- `engine-port/src/render/debug_cells.rs`
- `engine-port/src/output/capture.rs`
- `engine-port/src/render/shape_vector.rs`
- `engine-port/src/main.rs`

Behavior change:

- shape-vector is now blocked from overriding the original resolve on:
  - silhouette cells
  - linecase/grid overlay cells
  - half-block split cells (`0xDE`, `0xDF`)
  - mixed auto-mat cells, including reflection-boundary auto-mat cells

Debug metadata change:

- new debug label: `shape_gated_semantic`
- new per-frame stat: `semantic_gate_cells`
- title bar now includes `gate %`
- capture JSON schema bumped to version `4`

Rationale:

- this is not a heuristic occupancy tweak
- it is an architecture correction so original semantic glyph paths remain authoritative where they should

Validation:

- `cargo test --lib --quiet`

## Elevation-Aware Structural Fallback

One remaining fallback-quality bug was outside the threshold math itself:

- `choose_material_structural_fallback()` in `engine-port/src/render/pipeline.rs`
  was still using `lookup(0, sample.diffuse)` for every recovered terrain glyph
  instead of the cell's real computed elevation

That meant fallback glyph rescue could pull the wrong terrain symbol on raised or
lowered edges even when the dominant material choice was otherwise correct.

Fix:

- compute the same 4-way elevation state from the local sample block
- use that elevation when querying the dominant material's glyph during
  structural fallback

Validation:

- added unit coverage for:
  - default low-flat fallback (`elevation = 3`)
  - raised-edge fallback (`elevation = 2`)
- `cargo test --lib --quiet`

Impact:

- fallback glyph rescue is now more faithful to the original terrain material
  shading model
- this is a correctness improvement for occupancy recovery, not a new
  shape-vector override heuristic
- `cargo build --quiet`

Next comparison step:

1. replay against `artifacts/baselines/orbit-2026-03-11-current`
2. measure whether semantic gating reduces override churn and chaotic edges without giving back the occupancy gains

## Semantic Gating Smoke Replay

A 30-frame deterministic replay was run against the locked orbit trace:

- output: `/tmp/asciicker-semantic-gate-smoke`

Frame 1 comparison against the previous best default replay `artifacts/baselines/orbit-2026-03-11-stabilized-debug/frame_000001.json`:

- `selector_override_cells`: `267 -> 222` (`-45`)
- `fallback_space_cells`: `2259 -> 2259`
- `final_space_cells`: `2259 -> 2259`
- `colored_space_cells`: `2259 -> 2259`
- `threshold_skip_cells`: `2703 -> 2703`
- new `semantic_gate_cells`: `405`

Interpretation:

- the gate did exactly what it was supposed to do on the first replay
- it reduced shape-vector override churn further
- it did not give back the occupancy gains
- the next needed evidence is a fuller replay / visual pass, not another speculative policy change

## Full Semantic-Gated Replay

A full 120-frame deterministic replay is now stored at:

- `artifacts/baselines/orbit-2026-03-11-semantic-gated-debug`

Average delta vs the previous best pre-gate replay `artifacts/baselines/orbit-2026-03-11-stabilized-debug`:

- `selector_override_cells`: `-42.2` per frame
- `fallback_space_cells`: `+0.02` per frame
- `final_space_cells`: `-3.7` per frame
- `colored_space_cells`: `-3.7` per frame
- `threshold_skip_cells`: `0.0` per frame
- `semantic_gate_cells`: `390.5` per frame on average

Interpretation:

- the semantic gate is a real net win
- it reduces override churn further than the stabilized pre-gate path
- it does not materially hurt occupancy
- it is now the best default replay artifact for the current shape-vector path

- occupancy stayed effectively the same
- blank-cell gains were preserved
- edge-chaos risk dropped materially because far fewer structural glyph swaps were allowed

Current best default path:

- `artifacts/baselines/orbit-2026-03-11-semantic-gated-debug`

## Harri Color Ownership

The broad glyph-only Harri integration was still structurally wrong even after
semantic gating: when Harri changed the glyph, the renderer kept resolve-time
`fg`/`bk` colors that were chosen for the original glyph. That made the output
look only marginally different in manual compare mode because the shape changed
without a matching color solve.

Fix implemented:

- Harri-selected cells now run a glyph-aware `fg`/`bk` optimization pass against
  the actual runtime CP437 atlas before the final xterm quantization step
- this only applies on cells Harri is allowed to own; semantic-gated cells still
  keep original resolve colors and glyphs

Additional audit-driven fixes now implemented:

- terrain lightness sampling for shape-vector uses `mat_cell.bg` instead of
  `mat_cell.fg`, so terrain glyph decisions finally sample the filled color the
  player actually sees
- semantic eligibility is now encoded directly into `AnsiCell.spare` during
  resolve, and the active pipeline skips shape-vector selection up front on
  semantic cells instead of depending on debug flags after the fact

Mode model update:

- `original_only`: original resolve only, shape-vector disabled
- `combined`: semantic-gated shape-vector integration
- `harri_priority`: broad Harri override mode

The replay harness now supports stitched variant captures across those modes on
the same trace, with a capture-only bottom panel showing the active mode and
the most relevant live tuning values.

Validation:

- `cargo test --lib --quiet`
- `cargo build --quiet`

## Original Edge Writer Progress

Two more original-engine edge-path writes are now ported:

- terrain patch center cross writing in `terrain_shader.rs`
- mesh water-plane clamp plus parity bits in `mesh_shader.rs`
- mesh wireframe `0x40` Bresenham writing in `mesh_shader.rs`

This closes two write-side gaps from the earlier audit:

- terrain grid linecase bits can now actually be produced by terrain rendering
- reflected mesh samples now clamp to the water plane and carry reflection
  parity, so resolve can apply the intended reflected-mesh dimming path
- mesh wireframe linecase glyphs can now trigger from actual mesh input rather
  than only existing as a dead resolve branch
