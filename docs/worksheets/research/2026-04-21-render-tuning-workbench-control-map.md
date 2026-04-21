# 2026-04-21 Render Tuning Workbench Control Map

Status: active worksheet  
Canonical reference: `docs/CANONICAL_SPEC.md`  
Plan reference: `docs/worksheets/plans/2026-04-21-render-tuning-workbench-replan.md`  
Related failure: `F250` in `docs/FAILURE_LOG.md`

## Purpose

This worksheet is the code-backed control map requested by the Render Tuning
Workbench replan before any more UI work.

It answers one question only: which current runtime state already exists in
code, what it actually affects, and whether it belongs in the first trustworthy
workbench pass.

## Audit Notes

- Current branch/worktree state for this audit: `planning-audit-normalization`
  at `59863c6` plus local uncommitted doc updates.
- Conductor status is still blocked in this checkout because
  `scripts/conductor_tools.py` does not exist.
- This is a worksheet inventory, not a canonical status claim.
- `Phase 1` below means "include in the first real workbench control surface,"
  not "already implemented correctly."

## Phase 1 Control Inventory

These are real runtime-backed controls or readouts that should define the first
trustworthy workbench surface.

| State / resource | Current source file(s) | Current default | Visual / render effect | Desired control type | Phase 1? | Notes |
|---|---|---:|---|---|---|---|
| `RenderWorkbenchState.visible` | `engine-port/src/render/workbench.rs` | `true` | Controls whether the current egui overlay is shown | Explicit enter/exit action, not a free-floating overlay toggle | Yes | Keep the concept, but replace the current backquote overlay model with an explicit workbench mode/state |
| `RenderWorkbenchState.resolution_scale` | `engine-port/src/render/workbench.rs`; `engine-port/src/output/mod.rs`; `engine-port/src/render/pipeline.rs` | `1.0` | Scales the window-derived ASCII grid before `RenderConfig` and `SampleBuffer` are synchronized, directly changing output density and frame cost | Slider with numeric readout | Yes | This is the current code path for "resolution scale or exact grid size" |
| `RenderConfig.ascii_width` | `engine-port/src/render/config.rs`; `engine-port/src/render/pipeline.rs` | `240` | Sets ASCII grid width; sample buffer width becomes `2 * width + 4` | Numeric readout, optional stepper | Yes | In practice the active value is kept in sync from `AsciiCellGrid` at render time |
| `RenderConfig.ascii_height` | `engine-port/src/render/config.rs`; `engine-port/src/render/pipeline.rs` | `135` | Sets ASCII grid height; sample buffer height becomes `2 * height + 4` | Numeric readout, optional stepper | Yes | Pair with width as one `View` group control/readout |
| `GameCamera.zoom` | `engine-port/src/render/camera.rs`; `engine-port/src/render/workbench.rs` | `1.0` | Changes camera scale/focal behavior and visibly changes scene scale in the ASCII output | Slider with numeric readout | Yes | Already wired in the current overlay and belongs in canonical `View` |
| `GameCamera.yaw` | `engine-port/src/render/camera.rs` | `45.0` | Rotates the camera and changes both the visible scene and frustum orientation | Readout first; optional stepper / orbit actions later | Yes | Important visible state even if not a primary slider on day one |
| `GameCamera.pos` | `engine-port/src/render/camera.rs` | `[0.0, 15.0, 0.0]` | Changes which world region is rendered and what enters culling queries | Readout | Yes | Belongs in diagnostics/readouts, not as a freeform tuning slider |
| `RenderWorkbenchState.show_terrain` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Gates terrain rasterization entirely | Toggle | Yes | Canonical `Visibility` group |
| `RenderWorkbenchState.show_meshes` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Gates mesh instance rasterization in world stage | Toggle | Yes | Canonical `Visibility` group |
| `RenderWorkbenchState.show_sprites` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Gates sprite/item queueing and final sprite content | Toggle | Yes | Canonical `Visibility` group |
| `RenderWorkbenchState.enable_shadows` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Gates the player shadow pass | Toggle | Yes | Canonical `Visibility` group |
| `RenderWorkbenchState.enable_reflections` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Gates the water reflection pass | Toggle | Yes | Canonical `Visibility` group |
| `RenderWorkbenchState.terrain_culling` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Switches terrain rendering between frustum query and full patch iteration | Toggle | Yes | Canonical `Culling` group |
| `RenderWorkbenchState.world_culling` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `true` | Switches world rendering between BSP visibility query and brute-force visible instance iteration | Toggle | Yes | Canonical `Culling` group |
| `RenderWorkbenchState.invert_colors` | `engine-port/src/render/workbench.rs`; `engine-port/src/render/pipeline.rs` | `false` | Swaps foreground/background colors after sprite blit across the whole ASCII grid | Toggle | Yes | Runtime-backed; keep only if the user still finds it useful for inspection |
| `ShapeVectorConfig.mode` | `engine-port/src/render/shape_vector.rs` | `Combined` | Changes the glyph matching behavior used in the shape-vector override path | Segmented control | Yes | Canonical `Glyph Matching` group |
| `ShapeVectorConfig.alphabet` | `engine-port/src/render/shape_vector.rs` | `Default` | Changes the glyph alphabet used by the matcher | Segmented control / dropdown | Yes | `Default` and `Minimal` are real runtime values |
| `ShapeVectorConfig.distance_threshold` | `engine-port/src/render/shape_vector.rs`; `engine-port/src/render/pipeline.rs` | `0.08` | Rejects candidate glyphs beyond threshold and pushes cells toward fallback behavior | Slider with numeric readout | Yes | Core glyph resolve control |
| `ShapeVectorConfig.enable_contrast_adaptive_threshold` | `engine-port/src/render/shape_vector.rs` | `false` | Enables contrast-based expansion of the effective distance threshold | Toggle | Yes | Pair with adaptive boost control |
| `ShapeVectorConfig.contrast_adaptive_threshold_boost` | `engine-port/src/render/shape_vector.rs` | `0.25` | Increases effective threshold based on contrast when adaptive threshold is enabled | Slider with numeric readout | Yes | Only meaningful when adaptive threshold is enabled |
| `ShapeVectorConfig.enable_structural_fallback` | `engine-port/src/render/shape_vector.rs`; `engine-port/src/render/pipeline.rs` | `true` | Allows structural fallback glyph preservation after threshold rejection | Toggle | Yes | Canonical `Glyph Matching` group |
| `ShapeVectorConfig.structural_fallback_distance_threshold` | `engine-port/src/render/shape_vector.rs`; `engine-port/src/render/pipeline.rs` | `0.22` | Sets the maximum distance for structural fallback to preserve a non-space glyph | Slider with numeric readout | Yes | Canonical fallback threshold |
| `ShapeVectorConfig.enable_global_crunch` | `engine-port/src/render/shape_vector.rs` | `true` | Enables global lightness shaping before glyph matching | Toggle | Yes | Pair with global crunch exponent |
| `ShapeVectorConfig.global_crunch_exponent` | `engine-port/src/render/shape_vector.rs` | `2.5` | Changes how aggressively global crunch redistributes sampled values | Slider with numeric readout | Yes | Canonical crunch tuning |
| `ShapeVectorConfig.enable_directional_crunch` | `engine-port/src/render/shape_vector.rs` | `true` | Enables directional feature shaping before glyph matching | Toggle | Yes | Pair with directional exponent |
| `ShapeVectorConfig.directional_crunch_exponent` | `engine-port/src/render/shape_vector.rs` | `6.0` | Changes how strongly directional context reshapes the sampled vector | Slider with numeric readout | Yes | Canonical crunch tuning |
| `ShapeVectorConfig.sampling_quality` | `engine-port/src/render/shape_vector.rs` | `8` | Changes sampling density used to build feature vectors for glyph matching | Slider or stepper | Yes | Canonical sampling-quality control |
| `RenderWorkbenchState.reset(...)` | `engine-port/src/render/workbench.rs` | n/a | Resets workbench state, `GameCamera`, and `ShapeVectorConfig` back to defaults | Button | Yes | Canonical `Reset to defaults` action |
| `PipelineTiming.total_us` / `resolve_us` / pass timings | `engine-port/src/render/pipeline.rs` | `0` until frames run | Read-only frame cost diagnostics for total, terrain, world, shadow, reflection, resolve, and sprite stages | Metric readouts | Yes | Keep visible at rest; no hover-only diagnostics |
| `ShapeVectorFrameStats` | `engine-port/src/render/shape_vector.rs` | `0` until a frame runs | Read-only glyph resolve diagnostics such as override rate, fallback rate, threshold skips, and colored blanks | Metric readouts | Yes | Keep the most decision-relevant counters visible, not the full struct dump |

## Real Runtime State, But Not Phase 1

These are real renderer-adjacent values, but they should not block the first
workbench delivery.

| State / resource | Current source file(s) | Current default | Visual / render effect | Desired control type | Phase 1? | Notes |
|---|---|---:|---|---|---|---|
| `ShapeVectorConfig.structural_fallback_contrast_threshold` | `engine-port/src/render/shape_vector.rs`; `engine-port/src/render/pipeline.rs` | `96` | Gates structural fallback on palette contrast | Slider or advanced numeric input | No | Real effect, but not part of the core first-pass control story; keep as readout or defer |
| `GameCamera.scene_shift` | `engine-port/src/render/camera.rs` | `[0, 0]` | Offsets the scene in sample/cell space | Advanced readout or stepper | No | Useful for shake/offset debugging, but not a primary workbench control |
| `GameCamera.light_dir` | `engine-port/src/render/camera.rs` | normalized `[1, 1, 1]` | Changes lighting direction inputs used by the renderer | Advanced control | No | Real render state, but outside the agreed immediate workbench scope |
| `GameCamera.light_ambient` | `engine-port/src/render/camera.rs` | `1.0` | Changes ambient light contribution | Slider | No | Real render state, but outside the agreed immediate workbench scope |

## Existing Comparison / Capture Infrastructure

These paths already exist in code and should be reused for later comparison
workflow work instead of inventing a fresh subsystem.

| State / resource | Current source file(s) | Current default | Visual / render effect | Desired control type | Phase 1? | Notes |
|---|---|---:|---|---|---|---|
| `VisualCaptureConfig.out_dir` / `delay_frames` / `exit_after_capture` | `engine-port/src/output/capture.rs` | `None`, `10`, `false` unless env says otherwise | Enables delayed one-shot visual capture and optional exit | Capture button plus capture settings popover | No | Current path is env-only |
| `ReplayHarnessConfig` | `engine-port/src/output/replay.rs` | `max_frames = 120`; `auto_start = true`; other fields env-backed | Drives baseline replay and capture workflow | Compare / replay panel | No | Current path is env-only |
| `OrbitCaptureConfig` | `engine-port/src/output/replay.rs` | Optional env-backed fields | Defines repeatable orbit capture behavior around locked camera/player/water state | Capture orbit action | No | Reuse later for comparison actions |
| Orbit trigger `F9` | `engine-port/src/output/replay.rs` | n/a | Arms orbit capture from the current pose | Button | No | Current path is hotkey-only |
| `VariantReplayConfig` | `engine-port/src/output/replay.rs` | `frames_per_mode = 120` when enabled by env | Produces stitched multi-mode compare captures | Compare mode action | No | Good foundation for later compare tooling |

## Non-Canonical Current Overlay State

These are present in the current `workbench.rs`, but they should not be treated
as the control vocabulary for the next implementation.

| State / resource | Current source file(s) | Current default | Visual / render effect | Desired control type | Phase 1? | Notes |
|---|---|---:|---|---|---|---|
| `RenderWorkbenchState.fixture` / `WorkbenchFixture` | `engine-port/src/render/workbench.rs` | `Scene` | Batch-applies sets of visibility/pass toggles using `Scene`, `Terrain`, `Meshes`, `Sprites`, `Water` labels | Remove, or replace only if real source/mode switching exists | No | This is the kind of fake fixture vocabulary the replan rejects |
| `RenderWorkbenchState.glyph_preset` / `GlyphPreset` | `engine-port/src/render/workbench.rs` | `Dense` | Applies undocumented bundles of `ShapeVectorConfig` values via `Dense` / `Sparse` buttons | Remove for now; reintroduce only as named, documented bundles of real settings | No | The concept is only acceptable if the preset bundle is explicit and trustworthy |
| Floating overlay toggle on backquote | `engine-port/src/render/workbench.rs`; `engine-port/src/render/mod.rs` | visible on startup, backquote toggles | Overlays the current workbench UI on the live game view | Replace with explicit mode entry | No | This is the rejected interaction model from `F250` |
| Figma credits link | `engine-port/src/render/workbench.rs` | present | No render effect; links to the Figma reference | Remove | No | The Figma is layout inspiration, not an in-product dependency |

## Implementation Readout Set

The first real workbench pass should keep the following values visible even when
the user is not actively dragging a control:

- current grid width and height
- current sample buffer width and height
- current camera zoom, yaw, and position
- current shape-vector mode and alphabet
- current threshold / fallback / crunch values
- current pass and culling toggles
- frame timing summary
- shape-vector override / fallback summary

## Recommended Build Order

1. Replace `RenderWorkbenchState.visible` with an explicit workbench mode entry
   path instead of the current overlay toggle.
2. Preserve and reuse the real runtime-backed controls already in code:
   resolution scale, zoom, pass visibility toggles, culling toggles, and the
   full `ShapeVectorConfig` surface selected above.
3. Remove non-canonical `fixture` and `glyph_preset` vocabulary from the next
   iteration unless they are rebuilt as documented bundles of real engine state.
4. Add comparison actions only after the base control surface is trustworthy,
   using the existing capture / replay infrastructure in `engine-port/src/output/`.
