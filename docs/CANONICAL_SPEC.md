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

Do not infer visual parity with C++ from this wiring. `docs/FAILURE_LOG.md` still carries open/partial render risks, including missing C++ golden baselines.

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

## Rendering Demo Mode

Status: planned, not implemented on `main`.

The rendering demo mode is the canonical target for interactive visual inspection of ASCII rendering behavior. It should be a tool-like workbench, not a landing page or explanatory demo screen.

Core layout:

- **Model list / source selection:** A distinct area for choosing what is being rendered. This is the "what" surface: model, scene, or fixture selection. It must not be mixed into render-setting controls.
- **Canvas center:** The primary workspace where the selected 3D model or scene is rendered as ASCII characters. The canvas must update live as parameters change.
- **Right control panel:** The "how" surface for rendering settings. It contains presets, resolution, scale, and functional toggles.

Canvas requirements:

- Render the model or scene using ASCII characters.
- React in real time to all right-panel settings.
- Support a rough 30-frame camera-rotation capture path for visual comparisons.
- Support side-by-side or repeatable comparison of multiple rendering modes when mode comparison is active.
- Provide enough visual stability that culling, density, scale, and glyph-set changes can be inspected without guessing from a single frame.

Right panel requirements:

- **Presets:** Quick-toggle character sets. Examples:
  - dense: `@%#*+=-:. `
  - medium: `#*+=-:. `
  - sparse: `.: `
- **Resolution slider:** Adjusts ASCII pixel density and updates the canvas immediately.
- **Scale slider:** Adjusts model zoom/size and updates the canvas immediately.
- **Invert colors toggle:** Swaps light/dark mapping.
- **Reset button:** Restores default preset, resolution, scale, color inversion, camera, and culling settings.
- **Culling/debug controls:** Expose culling behavior clearly enough to reproduce and inspect render differences. Keyboard shortcuts may exist, but every critical state must also be visible in the UI.

UX requirements:

- Sliders must provide immediate feedback; the user should see legibility and density changes as they drag.
- Character-set presets are part of the product value, not decoration. They should make density and ASCII-art style differences obvious.
- The interface must separate "what is being rendered" from "how it is rendered" to keep the workbench usable as controls grow.
- The first viewport should be the usable workbench itself.
- The UI must avoid in-app instructional prose. Labels and state readouts are acceptable; tutorial copy is not.
- Controls and text must remain legible and non-overlapping on desktop and mobile-width viewports.

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
