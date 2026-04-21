# 2026-04-21 Render Tuning Workbench Replan

Status: active worksheet  
Canonical reference: `docs/CANONICAL_SPEC.md`  
Related failure: `F250` in `docs/FAILURE_LOG.md`
Current blocker: `F251` in `docs/FAILURE_LOG.md`
ASCIIID audit: `docs/worksheets/research/2026-04-21-asciiid-font-palette-material-audit.md`

## Name Decision

Canonical product name:

- `Render Tuning Workbench`

Acceptable shorthand:

- `render workbench`

Deprecated names:

- `inspector`
- `render demo`
- `render adjustable window`
- any copied Figma label set not backed by the engine

Reason:

- `Inspector` implies a generic property viewer.
- The target is a dedicated tuning surface for render behavior, not a resource browser.
- The previous implementation drifted because it treated the Figma as control vocabulary instead of layout inspiration.

## Problem Statement

The first implementation attempt shipped in commit `59863c6` and was rejected by the user. The failure was not cosmetic; it was conceptual.

Observed failure:

- It behaved like an overlay on top of the live game instead of an explicit tuning surface.
- The controls were not trustworthy or clearly clickable in practice.
- Several labels were copied from the Figma rather than derived from actual Asciicker renderer state.
- The UI did not answer the core question: which real renderer variables should be visible and adjustable?

Therefore, the next implementation must be control-inventory-first.

## 2026-04-21 Live Rejection Addendum

The explicit workbench implementation after `93f4b37` fixed the worst
overlay/click/scroll direction but was still rejected during live use. This is
tracked as `F251`.

New observed failures:

- `Resume Scene` is a one-way exit; the user cannot get back to the workbench.
- Rain/weather controls do not produce an evident visible rain result.
- Terrain/world culling toggles have unclear effect; the UI does not prove what
  changed.
- The workbench lacks the ASCIIID `Spin` view toggle.
- Right-side numeric readouts can be hidden or clipped.
- Glyph matching needs user-customizable glyph choices, not only fixed
  alphabet buttons.
- The Shadows button appears to do nothing.

Implication:

- The next implementation pass must be ASCIIID-audit-backed, not just
  hotkey-parity-backed. It must add pass-effect proof, round-trip navigation,
  spin/orbit behavior, custom glyph candidate selection, and material/palette
  diagnostics before it can be considered user-acceptable.

## Product Goal

Build a dedicated Render Tuning Workbench that:

- shows live ASCII output in a canvas-first layout
- exposes the real renderer variables needed for debugging and visual tuning
- keeps current state visible at rest
- makes the old hotkey-only tuning path optional instead of mandatory
- supports repeatable visual comparison without pretending the Figma mock is the product spec

## Control Inventory

The workbench must start from real engine state.

### Existing renderer-backed controls

These already exist in code and should drive the first UI pass:

- `RenderConfig`
  - `ascii_width`
  - `ascii_height`
  - source: `engine-port/src/render/config.rs`
- `GameCamera`
  - `zoom`
  - potentially yaw/orbit actions if exposed intentionally
  - source: `engine-port/src/render/camera.rs`
- `ShapeVectorConfig`
  - `mode`
  - `alphabet`
  - `distance_threshold`
  - `contrast_adaptive_threshold_boost`
  - `structural_fallback_distance_threshold`
  - `sampling_quality`
  - `enable_global_crunch`
  - `enable_directional_crunch`
  - `enable_contrast_adaptive_threshold`
  - `enable_structural_fallback`
  - `global_crunch_exponent`
  - `directional_crunch_exponent`
  - source: `engine-port/src/render/shape_vector.rs`
- existing hotkey tuning path
  - source: `shape_vector_tuning_input_system` in `engine-port/src/render/shape_vector.rs`

### Renderer controls that should become explicit workbench state

These are valid workbench controls, but they should be represented as deliberate workbench-owned state rather than fake UI labels:

- terrain visibility on/off
- mesh/world visibility on/off
- sprite visibility on/off
- shadow pass on/off
- reflection pass on/off
- terrain frustum culling on/off
- world/BSP culling on/off
- invert colors on/off
- capture / compare trigger
- reset to defaults
- workbench return action from gameplay or paused gameplay
- ASCIIID-style spin/orbit toggle plus spin speed
- visible pass-effect counters/deltas for culling, shadows, weather, and
  reflections

### Controls that are not canonical unless backed by real behavior

- generic `preset` buttons with no documented variable bundle
- copied Figma source labels like `Logo`, `Computer`, `Plant`, `Shiba`, `Crystal`
- any toggle that only changes UI state

If presets are added later, they must be named bundles of actual renderer values, for example:

- `Reference`
- `Dense Legibility`
- `Sparse Silhouette`
- `Threshold Stress`

The stronger replacement for generic presets is a user-editable glyph set:

- display a CP437 glyph grid
- allow adding/removing glyphs from the active shape-vector candidate set
- persist named glyph sets only if they map to actual candidate lists
- show which glyphs are active at rest

## Layout Replan

The Figma remains a spatial reference, not a control spec.

### Left rail

Purpose:

- mode/context/source navigation only when those choices are real

Allowed examples:

- live scene
- replay trace
- compare mode
- capture set

Not allowed:

- fake fixture buttons with no runtime source-switch backing

### Center canvas

Purpose:

- the main inspection surface
- must remain readable and primary

Requirements:

- immediate visual response to tuning changes
- no large blocking overlay behavior during normal interaction
- explicit entry into tuning mode if gameplay input would otherwise conflict

### Right control stack

Group by task, not by mockup resemblance.

Suggested groups:

- `View`
  - resolution scale or exact grid size
  - zoom
  - yaw
  - spin toggle and speed
- `Visibility`
  - terrain
  - meshes
  - sprites
  - shadows
  - reflections
- `Culling`
  - terrain culling
  - world culling
  - visible patch/instance counts for culled vs unculled paths
  - any future back-face control if runtime-backed
- `Glyph Matching`
  - mode
  - alphabet
  - custom active glyph set / glyph picker
  - threshold
  - adaptive threshold toggle/boost
  - structural fallback toggle/threshold
  - sampling quality
  - global crunch toggle/exponent
  - directional crunch toggle/exponent
- `Diagnostics`
  - current grid dimensions
  - current zoom
  - current matching mode/alphabet
  - capture/compare action
  - reset
  - material probe: MAT-id, MAT-elev, diffuse/ramp, resolved glyph,
    foreground/background colors, palette/quantization result
  - pass-effect proof: weather particles visible/active, shadow affected cells,
    terrain patches drawn, world instances drawn

## Interaction Rules

- The workbench must be explicitly entered.
- Leaving the workbench must not strand the user; a return path is mandatory.
- It must not silently take over the gameplay screen.
- Mouse input must work for every primary control.
- Keyboard hotkeys may remain, but only as secondary paths.
- Current state must remain visible without hovering or reading logs.
- Numeric values must not be clipped by right-aligned layout.

## Implementation Plan

### Phase 1: Remove ambiguity

- Deprecate `inspector` naming in canon/planning docs.
- Remove or replace any UI labels copied from the Figma without engine meaning.
- Treat commit `59863c6` as rejected reference, not as a baseline to polish.

### Phase 2: Establish workbench mode

- Add an explicit `RenderTuningWorkbench` mode/state instead of an always-on overlay.
- Define clear entry/exit behavior so gameplay input and tuning input do not fight.
- Keep the canvas full-bleed and the control chrome lightweight.

### Phase 3: Ship the real control surface

- Wire only controls backed by actual runtime state.
- Start with View, Visibility, Culling, and Glyph Matching groups.
- Keep current values visible.
- Ensure reset restores documented defaults.

### Phase 4: Add comparison workflow

- Add capture/replay/compare actions only after the base tuning controls are solid.
- Reuse existing replay and baseline artifacts as diagnostics, not as the only UI story.

## Acceptance Criteria

The Render Tuning Workbench is acceptable only if all are true:

- every visible control maps to a real renderer state or explicit action
- mouse interaction is reliable
- the user can tell what settings are active without reading logs
- the UI does not impersonate Figma labels that do not exist in the app
- the surface is explicitly entered and does not accidentally hijack normal gameplay
- the workbench is better than the hotkey path because it is visible, inspectable, and repeatable
- the user can resume and return to the workbench without restarting the app
- spin/orbit can be toggled from the UI and visibly changes yaw over time
- shadows/weather/culling controls expose enough live diagnostics to tell
  whether the current scene has no affected cells or the control is broken
- glyph matching supports a user-selected active glyph candidate set
- material/palette diagnostics explain the final rendered cell path from
  MAT-id and MAT-elev through glyph/color/palette output

## Immediate Next Step

Before implementing more UI, produce a code-backed control map listing:

- state/resource name
- current source file
- current default
- visual effect
- desired control type
- whether it belongs in Phase 1 of the workbench
