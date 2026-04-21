# Asciicker Rust Port Canonical Spec

**Status:** canonical  
**Last updated:** 2026-04-21  
**Canonical docs:** this file and `docs/FAILURE_LOG.md`

## Authority Model

This repository has two durable documentation surfaces:

1. `docs/CANONICAL_SPEC.md` defines intended architecture, durable decisions, and current code/doc truth.
2. `docs/FAILURE_LOG.md` tracks failures, blockers, regressions, and proof state.

All other docs are worksheets. They may be useful as historical evidence, source notes, or temporary planning material, but they are not authoritative until their durable facts are absorbed into this spec or the failure log. At session end, temporary worksheets should either be absorbed or removed so they do not become parallel truth.

Completion claims must separate:

- **Code state:** what is present in the checked-out source.
- **Proof state:** what was actually verified, with commands or artifacts.
- **Canon state:** what this spec and the failure log say.

Do not call work complete, done, fixed, resolved, verified, or proven unless code state, proof state, and canon state all support that claim.

## Repository State

Snapshot used for this spec:

- Branch: `main`
- HEAD: `07d8bd0` (`wip: sync Projects repo state for cross-device work`)
- Local `origin/main` ref: even with `HEAD` at the time of this audit (`git rev-list --left-right --count HEAD...origin/main` returned `0 0`)
- Conductor: `scripts/conductor_tools.py` is absent in this checkout; this is an audit limitation, not proof of conductor health.
- Existing non-canonical worktree: `.worktrees/render-bugfixes` on `fix/render-bugfixes-f236-f239-f241-water` at `49a7e26`
- Dirty state during this spec creation: pre-existing `CLAUDE.md` modification in the main worktree, not touched by this spec.

No build or test command was run for this spec.

## Source Material

Original-source references are split deliberately:

- Primary local original checkout in this environment:
  `/Users/rikihernandez/Downloads/Aciicker-Y9-2/`
- Stable in-repo editor reference:
  `reference/original-game/asciiid.cpp`

Use the vendored `asciiid.cpp` for durable editor line references in canon and
planning docs. Use the local original checkout for broader engine files that
have not been vendored into this repo.

## Runtime Architecture

The Rust port is a Bevy 0.18 application with a custom CPU rasterizer and a GPU-backed ASCII output stage.

Durable subsystem layout:

- `engine-port/src/render/`: CPU render pipeline, camera, rasterizer, terrain shader, mesh shader, resolve, water, shape-vector glyph selection.
- `engine-port/src/output/`: Bevy GPU output path that displays the resolved `AsciiCellGrid`.
- `engine-port/src/terrain/`: runtime terrain quadtree and patch data.
- `engine-port/src/world/`: runtime world instances and BSP structures.
- `engine-port/src/game/`: game state, camera/player sync, water sync, menu, weather.
- `engine-port/src/physics/`: movement, collision, forces, physics I/O.
- `engine-port/src/network/`: optional multiplayer transport.

The CPU rasterizer writes a supersampled `SampleBuffer`. Resolve turns that into an `AsciiCellGrid`. The GPU output shader displays the grid; Bevy's 3D renderer is not the gameplay renderer.

## Render Pipeline

The C++ reference render pipeline is documented as a 6-stage pipeline:

1. Clear sample buffer.
2. Render terrain through terrain quadtree query and patch rasterization.
3. Render world geometry through world/BSP query; meshes rasterize immediately, sprites queue for later blit.
4. Render player blob shadow.
5. Render optional water reflection pass.
6. Resolve `SampleBuffer` to ANSI/CP437 cells, then sort and blit sprites.

The Rust `render_pipeline_system` follows this general shape, but main branch truth is mixed:

- Terrain is rasterized with frustum culling via `RuntimeTerrain::query_visible`.
- World/BSP frustum culling is intentionally disabled in main because of a coordinate-system mismatch; world instances are iterated directly.
- Stage 4 shadow is still a timing stub on `main`.
- Stage 5 water reflection is wired.
- Resolve includes sky color, reflection dimming, water ripple ordering, and optional shape-vector glyph selection.

Do not infer visual parity with C++ from this wiring. `docs/FAILURE_LOG.md`
still carries open/partial render risks, including missing C++ golden
baselines. Those baselines are diagnostic reference material, not the primary
product target.

## Projection And Render Modes

The C++ renderer supports two projection modes:

- **Isometric:** direct inverse transform via `inv_tm`.
- **Perspective:** architectural perspective using focal/view parameters and perspective solve paths.

The Rust camera keeps the same conceptual split:

- `GameCamera.perspective` selects perspective or affine transform behavior.
- `view_tm` and `inv_tm` hold transform matrices.
- `view_dir`, `view_pos`, `view_ofs`, `mul`, and `add` support perspective projection.

Separate from projection modes, the old terminal layer has two rendering backends:

- OpenGL terminal emulator (`term.cpp`): AnsiCell buffer to GPU texture and quad.
- Pure terminal/PTTY (`terminal.cpp`): ANSI escape rendering, no OpenGL dependency.

These are not the "three rendering modes at once" demo. No tracked source, artifact, or reachable commit found during the 2026-04-21 audit implements a 30-frame camera-rotation comparison demo with three modes and culling keybindings.

The only tracked "three modes" code found in this checkout is networking mode selection:

- `Standalone`
- `Server`
- `Client`

That is unrelated to rendering.

## Render Tuning Workbench

Status: active, not yet user-approved. Current implementation includes the
`F251` return/pass-proof controls and the `F252` preset/palette/lane pass, and
is pending live user feedback.

Canonical product name: `Render Tuning Workbench`

Deprecated names:

- `inspector`
- `render demo`
- `render adjustable window`
- any literal Figma-derived label set that is not backed by engine state

The Render Tuning Workbench is the canonical target for interactive inspection
and tuning of ASCII rendering behavior. It is not a generic debug inspector,
not a landing page, and not a literal clone of the Figma reference. The Figma
artifact is useful as layout and tone inspiration only: full canvas first,
lightweight left/right chrome, minimal copy.

The original-engine baselines remain useful as reference evidence for culling,
legibility, and glyph behavior, but they are not the product gate. The product
gate is whether the Render Tuning Workbench is usable for live inspection,
comparison, and tuning with controls that correspond to real renderer state.

Purpose:

- Replace opaque hotkey-only tuning with visible, inspectable state.
- Make render variables discoverable without memorizing key chords or reading window-title text.
- Separate "what is being rendered" from "how it is rendered".
- Support repeatable visual debugging of culling, glyph selection, density, and pass behavior.
- Preserve the original ASCIIID editor concepts that matter to render
  inspection: spin/orbit, font/glyph selection, palette mapping, material ID,
  material elevation, material ramps, and final-render diagnostics.

Structural rules:

- The workbench must be a dedicated tuning surface or explicitly-entered mode. It must not be an always-on overlay that hijacks normal gameplay input.
- Entering the workbench must be reversible. If the user resumes gameplay,
  there must be a visible and keyboard-accessible path back to the workbench.
- The center canvas is primary. Controls support the canvas; they are not the main content.
- Mouse interaction must work for every primary control.
- Keyboard shortcuts may remain as secondary access paths, but the canonical state must be visible in UI.
- Any control that appears to toggle a render pass must also expose proof that
  the pass changed, or did not change, the current frame. Examples: visible
  affected-cell counts, visible rendered-object/patch counts, or before/after
  deltas.

Layout model:

- **Left navigation rail:** context and source selection only when those selections are real. Examples: scene, replay trace, comparison mode, or fixture only if the app truly supports switching among them.
- **Center canvas:** full-bleed ASCII render output with immediate visual response to tuning changes.
- **Right control stack:** compact groups for live renderer controls and diagnostics, not a generic property inspector.

Control model:

- Every visible control must map to a real runtime setting or explicit comparison action.
- No decorative controls are allowed.
- No generic `preset` control is allowed unless it maps to a named, documented bundle of real renderer settings.
- No copied Figma labels such as `Logo`, `Computer`, `Plant`, `Shiba`, or `Crystal` are canonical unless the app actually exposes those render targets.

Canonical control groups:

- **View:** resolution scale or explicit grid dimensions, camera zoom/scale,
  camera yaw, camera pitch if runtime-backed, grid overlay alpha if
  runtime-backed, ASCIIID-style spin/orbit toggle and speed, and any
  capture/orbit trigger used for comparison.
- **Visibility and culling:** terrain visibility, world/mesh visibility, sprite visibility, shadow pass, reflection pass, terrain frustum culling, world/BSP culling, and back-face/double-sided controls only if they are truly runtime-backed.
- **Glyph matching and resolve tuning:** shape-vector mode, alphabet, custom
  user-selectable glyph candidate sets, distance threshold,
  adaptive-threshold toggle, adaptive boost, structural fallback toggle,
  fallback threshold, sampling quality, global crunch toggle/exponent, and
  directional crunch toggle/exponent.
- **Material and palette diagnostics:** hovered-cell material ID, MAT-elev bit,
  chosen material ramp/shade, resolved material glyph, foreground and
  background RGB, active font glyph, palette/quantization outcome, and final
  resolved cell output. Permanent editing of material IDs, material ramps, font
  pixels, or palette swatches is editor-adjacent. The workbench may provide
  non-destructive preview overrides for render inspection, but those overrides
  must be visibly labeled as preview state and must not write world/editor asset
  data unless that scope is explicitly added later.
- **Preset and theme controls:** user-facing presets are canonical when they
  are transparent bundles of real renderer settings. Presets must be editable,
  cloneable, and inspectable. Figma-like preset affordances are allowed, but the
  labels and bundles must describe Asciicker render intent: e.g. legibility,
  dense terrain, silhouette, rainy contrast, water/reflection stress, material
  ramp inspection, or elevation-lane inspection.
- **Weather and pass diagnostics:** weather state/type/intensity and visible
  particle or affected-cell counts; shadow/reflection/culling controls must
  report enough runtime data to distinguish "enabled but no effect in this
  scene" from "button is not wired".
- **Diagnostics and actions:** reset-to-defaults, capture/compare actions, and visible numeric/readout state for the currently active settings.

Required workbench UX/control plan:

1. **Mode and navigation shell**
   - Primary actions: `Resume Scene`, `Return to Workbench`, `Reset Render
     Defaults`, `Capture Frame`, and later `Compare Capture`.
   - `Resume Scene` must never be a one-way exit. A visible `Workbench` action
     must exist from gameplay/pause/menu state, and the keyboard shortcut must
     be shown in a compact shortcut/readout area.
   - The workbench state is distinct from normal gameplay state. UI input,
     gameplay input, and hotkey tuning must not compete for the same pointer
     focus.

2. **Viewport and canvas controls**
   - Use sliders for continuous values: resolution scale, zoom, yaw, pitch if
     supported, spin speed, grid alpha, and any future light/time controls.
   - Use steppers or numeric inputs for exact grid width/height only if the
     value can be applied safely without fighting window-derived grid sync.
   - Use toggles for binary behavior: spin, grid overlay, invert colors,
     terrain/world/sprite visibility, shadows, reflections, weather, terrain
     culling, world culling, adaptive threshold, structural fallback, global
     crunch, and directional crunch.
   - Spin must visibly change yaw over time, expose current yaw and speed, and
     pause cleanly.

3. **Pass-effect proof**
   - Every pass toggle must have an adjacent live proof readout. Required
     readouts include terrain patches considered/drawn/culled, world instances
     considered/drawn/culled, sprite count, shadow affected samples/cells,
     reflection affected samples/cells, weather active particles and visible
     cells, and shape-vector accepted/rejected/fallback/override counts.
   - If a toggle is enabled but produces no visible change, the workbench must
     say so through counts such as `0 affected cells`; it must not appear inert
     or unwired.
   - Culling controls must support an auditable comparison between culled and
     unculled traversal counts for the current frame or last sampled frame.

4. **Glyph candidate picker**
   - The user-facing design must start with task-oriented glyph/matching
     presets, not a raw CP437 grid alone. Presets must expose their included
     glyphs, matching mode, thresholds, fallback policy, and crunch settings.
   - The shape-vector candidate set must remain user-editable from the UI.
     Required advanced widgets: CP437 16x16 glyph grid, active glyph chips/list,
     add/remove actions, clear action, and restore-default action.
   - The UI must clearly separate three glyph layers: font atlas glyph,
     material glyph, and shape-vector candidate glyph.
   - Named glyph sets and matching presets are allowed only when they are
     persisted as explicit setting bundles. Generic presets that silently mutate
     thresholds or modes are not canonical.
   - A preset card must include a concise purpose, a preview swatch/sample, and
     an "advanced" disclosure that shows exactly which glyph and resolve
     controls it changes.

5. **Material, font, and palette probe**
   - Hover/probe readouts must show the final terrain render path for the
     selected cell: screen cell, sample/world coordinate if available, MAT-id,
     MAT-elev, diffuse/shade index, material ramp row, material glyph code,
     font glyph/alpha source, foreground RGB, background RGB, palette index or
     mapped RGB, and final `AnsiCell` foreground/background/glyph.
   - Render-path preview controls may include active font selection, active
     palette or palettize/depalettize mode, palette mapping toggle, material ID
     probe override, MAT-elev probe override, diffuse/ramp override, material
     glyph override, and foreground/background override. These are inspection
     controls unless explicitly promoted to editor persistence.
   - Palette controls must be labeled as color mapping/quantization controls,
     not as glyph or shape-vector controls. Default palette previews must be
     actual multi-color palettes, not monochrome filters. Vim/editor themes are
     acceptable inspiration: examples include solarized-style warm/cool ramps,
     gruvbox-style earthy high-contrast ramps, nord-style cool ramps,
     monokai-style saturated contrast, and dracula-style dark neon contrast.
   - Elevation/material lane controls are canonical for inspection. The
     workbench must expose 4 material/elevation lanes and 16 diffuse/shade
     stops as lanes, strips, or compact matrices. Users must be able to preview
     lane selection, MAT-elev behavior, and diffuse/ramp movement without
     persisting edits to world or material assets.

6. **Right-panel layout**
   - Numeric readouts must remain visible at rest, use stable widths, and never
     be right-aligned into clipped space.
   - The right control stack must be independently scrollable when it exceeds
     viewport height, while the center canvas remains usable.
   - Controls should be grouped by task: View, Visibility, Culling, Glyph
     Matching, Material/Palette Probe, Pass Proof, Capture/Compare.

UX requirements:

- Sliders must provide immediate feedback.
- Current values must remain visible at rest.
- Numeric readouts must not be clipped or hidden by right-aligned layout inside
  the control panel.
- Controls must be grouped by user task, not by resemblance to a mockup.
- The interface must avoid tutorial prose and generic debug noise.
- Controls and text must remain legible and clickable on desktop and narrow viewports.
- The workbench must not take over the normal game screen unless the user explicitly enters the tuning surface.

ASCIIID-derived render facts:

- The original editor's `FONT` window selects a CP437 font atlas and active
  glyph; it can edit glyph texels. For the Rust workbench, custom glyph
  candidate selection is canonical, while full font-pixel editing is
  editor-adjacent.
- The original editor's `SKIN` window edits both palettes and material ramps.
  Palettes affect final color mapping after material glyph/color composition.
  Material ramps are `shade[4][16]` tables containing foreground RGB,
  background RGB, and glyph code.
- `MAT-id` is the low 8 bits of a terrain visual cell and selects one of 256
  material definitions.
- `MAT-elev` is the `0x8000` terrain visual bit used by material ramp/elevation
  logic. Workbench labels must explain this as a render/material flag, not a
  generic world-height value.
- The final terrain screen path is material ID + MAT-elev/ramp + diffuse shade
  -> material glyph/foreground/background -> font alpha blend -> palette/color
  mapping -> grid/probe/pass overlays.

Non-canonical patterns:

- Copying mockup labels without runtime meaning.
- Showing UI state that does not affect the renderer.
- Shipping a Bevy inspector/resource viewer and calling it the workbench.
- Overlaying controls on gameplay in a way that blocks normal interaction without an explicit mode switch.
- Buttons whose effects cannot be observed or diagnosed from the workbench
  itself, such as a shadow/culling/weather toggle with no visible state delta.

## Culling

Terrain culling in main:

- `engine-port/src/terrain/quadtree.rs` implements quadtree frustum culling with plane elimination.
- Planes are `[a, b, c, d]` with `ax + by + cz + d >= 0` meaning inside.
- When a node AABB is fully inside a plane, that plane is removed for child traversal.
- When all planes are satisfied, descendants are visited without further plane tests.

Known terrain culling risk:

- Main branch `query_terrain_frustum` currently computes `x1 = x0 + size` and `y1 = y0 + size`.
- `docs/FAILURE_LOG.md` F239 says this is open on main: the terrain AABB and camera frustum spaces do not match correctly.
- The render-bugfix worktree has a candidate F239 fix in commit `a30b150`, but that worktree is not canonical main state.

World/BSP culling in main:

- `engine-port/src/world/` contains BSP structures and traversal.
- `engine-port/src/render/pipeline.rs` disables BSP frustum culling for rendering with a TODO because frustum planes and BSP bounding boxes are in incompatible spaces.
- Main currently iterates all visible world instances for rendering.

C++ reference culling:

- Terrain and world queries use plane/AABB testing against frustum planes.
- World query supports plane-based recursive BSP traversal and can early-out entire nodes.
- C++ notes include plane-array shortening during traversal: planes already satisfied by a node do not need to be checked against children.
- Mesh rasterization supports back-face culling through a `dblsided` flag, and reflection mode can flip vertex order.

## Open Render Truth

Main branch code and failure-log truth currently disagree with the render-bugfix branch in important places:

- Main `GameCamera::default` still uses `light_ambient = 1.0` and `[1,1,1]` normalized light direction.
- Main terrain culling still uses the smaller `x0 + size` / `y0 + size` AABB extent.
- Main shadow stage is still a stub.
- The render-bugfix worktree records F236/F239 resolved and F241 partial, but those changes are not present on `main`.

Therefore, canonical main status remains:

- F236: open on main unless commit `8245a38` or equivalent is integrated.
- F239: open on main unless commit `a30b150` or equivalent is integrated.
- F241: open/partial depending on branch; on main, shadow is still stubbed.
- F244: water ripple/reflection investigation remains unresolved in the render-bugfix worktree.

## Temporary Worksheet Lifecycle

Worksheets are allowed only as working memory. Examples include:

- Research notes.
- Phase plans.
- Handoff scratch files.
- Browser testing fixture notes.
- Audit dumps.
- Generated analysis.

Before ending a session:

1. Move durable facts into `docs/CANONICAL_SPEC.md`.
2. Move failures, regressions, blockers, and proof gaps into `docs/FAILURE_LOG.md`.
3. Delete or clearly discard temporary worksheets that are no longer needed.
4. Do not leave a worksheet that claims a status conflicting with this spec or the failure log.

If a worksheet must remain temporarily, it must be treated as non-canonical and must not be used to override this spec or the failure log.
