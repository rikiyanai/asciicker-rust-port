# 2026-04-21 Render Tuning Workbench Replan

Status: implementation complete, pending live feedback
Canonical reference: `docs/CANONICAL_SPEC.md`  
Related failure: `F250` in `docs/FAILURE_LOG.md`
Current monitored failures: `F251` and `F252` in `docs/FAILURE_LOG.md`
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

## Comprehensive Implementation Plan

This plan replaces the rejected overlay/preset direction with a render
inspection workbench that exposes every important final-render-path layer:
camera/view, source/material, resolve/glyph, color/palette, pass/visibility,
and comparison evidence.

### Phase 0: Naming and removal guardrail

Goal:

- Prevent another mockup-shaped UI pass before coding new controls.

Tasks:

- Keep `Render Tuning Workbench` as the only product name in new code and docs.
- Remove in-product Figma credit/link UI and any copied Figma fixture labels
  unless they map to real scene/source switching.
- Remove or quarantine `fixture` and undocumented `glyph_preset` vocabulary
  from the next implementation pass.
- Treat `59863c6` and the rejected `93f4b37` behavior as failure evidence, not
  as a baseline to polish.

Acceptance:

- No visible control appears unless it maps to a runtime setting, runtime
  readout, or explicit action.

### Phase 1: Mode shell and round-trip navigation

Goal:

- Make workbench entry, exit, and return reliable before adding more sliders.

Required UX:

- Top-level actions:
  - `Resume Scene`
  - `Return to Workbench` / `Workbench` from gameplay, pause, or menu state
  - `Reset Render Defaults`
  - `Capture Frame`
  - disabled or deferred `Compare Capture` if comparison is not ready
- Keyboard access remains secondary, but the same state must be visible in UI.
- `Resume Scene` must preserve a visible and keyboard-accessible return path.

Likely code areas:

- `engine-port/src/game/state.rs`
- `engine-port/src/game/menu.rs`
- `engine-port/src/render/workbench.rs`
- `engine-port/src/render/mod.rs`

Acceptance:

- Starting in workbench, resuming the scene, and returning to the workbench can
  be completed without restarting.
- UI input and gameplay input do not fight for pointer focus.

### Phase 2: Stable canvas and right-panel layout

Goal:

- Fix the live usability issues before adding new diagnostic complexity.

Required UX:

- Center ASCII canvas remains primary and unobscured.
- Right control stack is independently scrollable.
- Numeric values use stable widths and remain visible at rest.
- Long labels and values do not clip on desktop or narrow windows.

Required controls/readouts:

- grid width and height
- sample buffer width and height
- frame timing summary
- current camera position, yaw, zoom, and spin state

Acceptance:

- No right-side numbers are hidden or clipped.
- Dragging a slider or toggling a control does not resize or shift unrelated
  controls.

### Phase 3: View and viewport controls

Goal:

- Expose the ASCIIID-derived inspection controls needed to inspect the scene
  while it moves.

Required widgets:

- resolution scale slider with numeric readout
- exact grid width/height readouts, with steppers only if they are safe against
  window-derived grid synchronization
- zoom slider
- yaw slider or stepper plus current yaw readout
- pitch slider only if the current camera path is actually runtime-backed
- `Spin` toggle
- spin speed slider
- grid overlay alpha slider if runtime-backed
- center/reset-view button
- optional orbit/capture trigger if backed by existing capture infrastructure

Acceptance:

- Enabling `Spin` visibly changes yaw over time.
- Disabling `Spin` stops yaw drift immediately.
- Resolution and zoom controls produce immediate visible canvas changes and
  show current values without relying on logs.

### Phase 4: Visibility, weather, shadow, reflection, and culling proof

Goal:

- Make every pass toggle auditable so the user can tell whether it affected
  the current frame.

Required widgets:

- toggles: terrain, world/meshes, sprites, shadows, reflections, weather,
  terrain culling, world/BSP culling, invert colors if retained
- weather selector or weather state toggle with intensity only if the runtime
  supports it
- pass-proof readouts beside the toggles

Required readouts:

- terrain patches considered / drawn / culled
- world instances considered / drawn / culled
- sprite count
- shadow affected samples/cells
- reflection affected samples/cells
- weather active particles and visible particles/cells
- shape-vector accepted / rejected / fallback / override counts

Implementation requirement:

- A toggle that changes no cells in the current scene must report `0 affected`
  or an equivalent explicit reason. It must not look inert.
- Culling must expose enough data to compare culled and unculled traversal for
  the current frame or last sampled frame.

Acceptance:

- The user can answer "did Shadows, Rain, Terrain Culling, or World Culling do
  anything?" from the workbench without reading logs.

### Phase 5: Glyph matching and custom candidate sets

Goal:

- Replace raw, confusing glyph controls with user-friendly matching presets
  backed by real glyph-candidate and resolve settings.

Required widgets:

- preset cards inspired by the Figma page pattern, but backed by transparent
  Asciicker renderer settings
- each preset card must show a short purpose, sample preview, active glyph
  family, matching mode, and changed thresholds
- clone/edit/save actions for presets
- shape-vector mode segmented control
- alphabet dropdown or segmented control
- CP437 16x16 glyph grid
- active candidate glyph chips/list
- add/remove glyph actions
- clear set action
- restore default set action
- optional named glyph sets only if they persist explicit candidate lists
- distance threshold slider
- adaptive threshold toggle and boost slider
- structural fallback toggle and threshold slider
- sampling quality slider or stepper
- global crunch toggle and exponent slider
- directional crunch toggle and exponent slider

Required preset families:

- `Original Material`: disables shape-vector replacement and shows original
  material glyph behavior.
- `Legible Terrain`: favors stable terrain glyphs and conservative overrides.
- `Dense Detail`: allows richer glyph candidates for surface detail.
- `Silhouette Safe`: preserves semantic/silhouette/linecase glyph ownership.
- `Rain/Shadow Contrast`: biased toward glyphs/colors that make weather and
  shadows readable.
- `Water Stress`: emphasizes reflection/water boundary diagnosis.
- `Custom`: user-owned bundle with explicit glyph list and resolve settings.

Labeling requirement:

- The UI must distinguish:
  - font atlas glyph
  - material glyph
  - shape-vector candidate glyph
- Presets must not hide what they change. The advanced drawer for each preset
  must show candidate glyphs, mode, alphabet, thresholds, fallback, adaptive
  boost, sampling quality, and crunch toggles/exponents.

Acceptance:

- A non-expert can choose a useful glyph/matching preset without understanding
  shape-vector internals.
- Editing the active candidate set changes the actual shape-vector candidate
  list used by the renderer.
- Current candidate glyphs are visible at rest.
- Preset changes are reversible and cloneable.
- Generic `Dense`/`Sparse` style buttons are absent unless they are
  reintroduced as documented, editable bundles of real values.

### Phase 6: Material, font, and palette render-path probe

Goal:

- Let the user inspect and temporarily adjust final terrain render-path inputs
  without pretending to ship a full editor.

Required probe readouts:

- hovered screen cell
- sample coordinate and world coordinate when available
- MAT-id
- MAT-elev bit
- diffuse/shade index
- material ramp row
- material glyph code
- active font glyph source
- foreground RGB
- background RGB
- palette index or mapped RGB
- final `AnsiCell` glyph, foreground, and background

Optional non-destructive preview controls:

- active font selection
- active palette selection
- palettize/depalettize or palette mapping toggle
- default multi-color palette themes inspired by mature editor/Vim theme
  families, not monochrome filters
- material ID probe override
- MAT-elev probe override
- diffuse/ramp override
- material glyph override
- foreground/background preview override

Required palette themes:

- `Asciicker Original`: unmodified resolved material/palette output.
- `Solar Field`: warm/cool balanced palette for daylit terrain inspection.
- `Gruvbox Earth`: earthy high-contrast terrain palette with readable greens,
  yellows, browns, reds, and desaturated neutrals.
- `Nord Ice`: cool blue/cyan palette for water, sky, and shadow contrast.
- `Monokai Signal`: saturated contrast palette for silhouettes and material
  boundaries.
- `Dracula Night`: dark purple/blue base with bright accent ramps for
  nighttime/weather stress.
- `Accessibility High Contrast`: multi-color but high-separation ramp for
  foreground/background legibility.

Each palette theme must define at least:

- sky/clear color
- water/reflection ramp
- terrain vegetation ramp
- stone/neutral ramp
- shadow/dark ramp
- warning/accent ramp for overlays/probes
- glyph foreground/background contrast policy

Required material/elevation lane controls:

- 4 elevation/ramp lanes corresponding to material `shade[4][16]`
- 16 diffuse/shade stops per lane
- lane enable/solo controls for inspection
- lane weight/contrast sliders for preview
- MAT-elev preview toggle showing how bit `0x8000` changes lane selection
- diffuse-index scrubber showing movement across the 16 shade stops
- per-lane swatches for material glyph, foreground RGB, and background RGB
- read-only by default; any edits are preview-only unless editor persistence is
  explicitly scoped later

Boundary:

- These controls are inspection/preview state. They must not persist world,
  palette, material, or font edits unless a later editor-scope task explicitly
  adds persistence.

Acceptance:

- The workbench can explain a selected final cell from MAT-id and MAT-elev
  through material glyph/color, font alpha, palette mapping, and final output.
- Palette themes visibly differ as full color palettes, not grayscale/mono
  recolors.
- Elevation/material lanes make MAT-elev and diffuse/ramp behavior adjustable
  and understandable without opening the full editor.

### Phase 7: Capture and comparison workflow

Goal:

- Add repeatable evidence only after the base workbench is trustworthy.

Required UX:

- capture current frame
- capture orbit / spin sample if backed by existing replay infrastructure
- compare current settings against a captured/reference settings snapshot
- show current camera/culling/glyph settings in capture metadata

Likely reusable code:

- `VisualCaptureConfig`
- `ReplayHarnessConfig`
- `OrbitCaptureConfig`
- `VariantReplayConfig`

Acceptance:

- Captures include enough visible/readable settings to reproduce the frame.
- Compare mode never replaces live pass-proof diagnostics; it complements them.

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
- glyph/matching presets are user-friendly, editable, and disclose their real
  renderer settings instead of exposing only raw CP437 grid mechanics
- material/palette diagnostics explain the final rendered cell path from
  MAT-id and MAT-elev through glyph/color/palette output
- default palette themes are full multi-color palettes with named ramps, not
  monochrome recolors
- material/elevation lanes expose 4 elevation/ramp rows and 16 diffuse stops
  for previewing MAT-elev and material ramp behavior
- permanent material, palette, font, or world edits are not performed from the
  workbench unless explicitly scoped; render-path overrides are preview-only
  and visibly labeled as such
- every slider/toggle has a visible current value or proof readout at rest
- the right control stack remains usable when scrolled and does not block the
  center canvas

## Immediate Next Step

Open the workbench and collect live feedback against the `F252` controls now in
the working tree:

- named glyph/matching preset buttons for original, legible, dense,
  silhouette-safe, rain/shadow, water, and custom bundles
- real multi-color palette previews inspired by editor themes
- material/elevation lane controls for enable, solo, tint, weight, contrast, and
  diffuse-stop previewing
- advanced raw glyph grid and threshold sliders kept below the preset layer
