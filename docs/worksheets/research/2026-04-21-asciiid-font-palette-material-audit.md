# 2026-04-21 ASCIIID Font, Palette, Material, and View Audit

Status: active worksheet  
Canonical reference: `docs/CANONICAL_SPEC.md`  
Related failure: `F251` in `docs/FAILURE_LOG.md`  
Primary source: `reference/original-game/asciiid.cpp`

## Scope

This audit answers the workbench parity questions raised after the live
Render Tuning Workbench launch:

- What do the ASCIIID `FONT` and `SKIN`/palette windows do?
- What are `MAT-id` and `MAT-elev`?
- What do ASCIIID view/config parameters translate to on the final render
  screen?
- Which of those concepts should influence the Rust workbench requirements?

This is source research, not an implementation claim.

## Source Map

Relevant ASCIIID source anchors:

- View controls: `asciiid.cpp:7486`, `asciiid.cpp:7726`, `asciiid.cpp:7728`
- Spin behavior: `asciiid.cpp:1748`, `asciiid.cpp:9355`
- Font loading and atlas editing: `asciiid.cpp:1331`, `asciiid.cpp:1565`,
  `asciiid.cpp:8779`
- Palette loading/editing: `asciiid.cpp:1238`, `asciiid.cpp:1263`,
  `asciiid.cpp:8913`
- Material ramp editing: `asciiid.cpp:8969`, `asciiid.cpp:9005`,
  `asciiid.cpp:9020`, `asciiid.cpp:9035`
- MAT-id painting: `asciiid.cpp:8154`, `asciiid.cpp:8189`,
  `asciiid.cpp:3876`
- MAT-elev painting: `asciiid.cpp:8278`, `asciiid.cpp:8311`,
  `asciiid.cpp:3904`
- Auto MAT-elev / auto texture: `asciiid.cpp:8216`, `asciiid.cpp:4055`,
  `asciiid.cpp:4154`
- Terrain final shader translation: `asciiid.cpp:2613`,
  `asciiid.cpp:2676`, `asciiid.cpp:2697`, `asciiid.cpp:2701`,
  `asciiid.cpp:2711`, `asciiid.cpp:2790`

## Font Window

The ASCIIID `FONT` window is not just a display selector. It is an editor for
the active CP437 glyph atlas.

Behavior:

- ASCIIID scans the `fonts/` directory and loads image files as 16x16 glyph
  atlases. Each atlas has 256 glyph slots.
- The active font is selected with left/right arrows; changing it calls
  `TermResizeAll()` so dependent terminal windows resize.
- The UI shows a 16x16 grid of glyph buttons. Clicking a glyph sets
  `active_glyph`.
- The `Character` section shows the selected glyph at pixel resolution.
  Clicking a pixel toggles that font texel alpha by XORing `0xFF`.
- When loaded, ASCIIID exports the font image to `.bdf` and `.psf`, mapping
  each CP437 slot through the `cp437` Unicode table.

Render translation:

- The terrain shader samples the active font texture at the material cell's
  glyph code. The alpha at that glyph pixel blends between material background
  RGB and foreground RGB.
- Therefore, changing the font atlas changes the shape of every final glyph
  that uses that CP437 code without changing material IDs or colors.

Workbench implication:

- A serious glyph workbench cannot stop at `Default`/`Minimal` alphabet
  toggles. It needs user-selectable glyph sets or a custom glyph picker for
  shape-vector candidates.
- Full font pixel editing is editor-scope and should not block the render
  workbench, but active glyph selection and candidate-set editing are directly
  relevant.

## Palette Window

ASCIIID's palette system is split between palette asset editing and final
palettization.

Behavior:

- `MyPalette::Init()` creates 256 palette slots, each with 256 RGB entries.
- Palette assets are loaded by scanning `palettes/`; each source image is
  sampled on a 16x16 grid to populate one 256-color palette.
- The `SKIN` window `Palettes` section selects `active_palette` and exposes a
  16x16 grid of `ColorEdit3` swatches. Editing a swatch mutates the active
  palette RGB value.
- The `VIEW` window has a `PALETTIZE` / `DEPALETTIZE` button. `PALETTIZE`
  snaps colors through the active palette; `DEPALETTIZE` passes `0` to undo or
  bypass that palette mapping path.

Render translation:

- The terrain shader computes material RGB first, then performs
  `color.rgb = texture(p_tex, color.xyz).rgb`. In ASCIIID, `p_tex` is the GPU
  palette lookup texture. This makes palette selection a final color
  quantization/tinting stage after material glyph/color composition.
- Palette changes therefore affect final screen colors without changing
  terrain material IDs, material ramps, or glyph codes.

Workbench implication:

- The Rust workbench currently exposes neither active palette selection nor
  palette quantization diagnostics. If palette parity is in scope, it needs a
  color/palette section distinct from glyph matching.
- Palette controls should be labeled as color quantization or palette mapping,
  not as glyph presets.

## Material Window

ASCIIID's `SKIN` window `Materials` section is the material-ramp editor.

Data model:

- There are 256 material IDs.
- Each material has `shade[4][16]`: 4 elevation/ramp rows and 16 diffuse/shade
  columns.
- Each `MatCell` stores foreground RGB, background RGB, glyph code, and flags.

Behavior:

- The active material is selected by arrows and displayed as
  `0xNN (decimal) Elevation ramps`.
- The UI draws a 4x16 grid. Each cell previews the material glyph using the
  active font, foreground color, and background color.
- Left-clicking a material cell applies the active glyph and/or paint
  foreground/background colors depending on the enabled `Glyph`, `Foreground`,
  and `Background` checkboxes.
- Right-clicking probes the cell, copying its glyph and/or colors into the
  active paint state.
- Drag/drop across cells interpolates foreground/background colors and chooses
  endpoint glyphs across the selected range.
- Per-row buttons copy/paste or rotate a ramp row.

Render translation:

- A terrain visual cell stores a material ID in its low 8 bits.
- Lighting computes a diffuse index from 0 to 15.
- Elevation/ramp selection chooses the material row.
- The final terrain glyph and foreground/background colors are fetched from
  `mat[matid].shade[elev][diffuse]`.

Workbench implication:

- `Glyph Matching` should not be the only glyph concept. There is a separate
  original-engine material glyph path.
- User-selectable custom glyph sets for shape-vector matching should be
  visually tied to CP437 glyph IDs and should make it clear whether the user is
  changing candidate glyphs, material glyphs, or both.

## MAT-id

`MAT-id` is terrain material painting, not a display-only debug value.

Behavior:

- `active_material` is a 0-255 material ID.
- The `MAT-id` tab lets the user paint that ID into terrain visual cells.
- Ctrl probes the current material ID under the cursor.
- Ctrl+Shift probes height.
- Optional brush height limit restricts painting above or below `probe_z`.
- The write path preserves upper visual bits and replaces only the low material
  byte: `(visual & ~0x00FF) | active_material`.

Render translation:

- MAT-id decides which material row set is used for a terrain visual cell.
- Changing MAT-id can alter the final glyph, foreground color, and background
  color for the same height/light condition because it switches the entire
  material definition.

Workbench implication:

- Culling/visibility controls are not enough to inspect terrain semantics. A
  future advanced workbench needs at least a read-only material probe showing
  material ID, elevation bit, diffuse/shade index, resolved glyph, and colors
  for the hovered cell.
- Editing material IDs is probably editor scope, but material-ID readouts are
  render-debug scope.

## MAT-elev

`MAT-elev` is a 1-bit terrain visual flag used to select material ramp behavior.

Behavior:

- The `MAT-elev` tab exposes an `ELEVATED` checkbox and paint/probe controls.
- Painting writes bit `0x8000` in the terrain visual cell:
  `(visual & ~0x8000) | (active_elev << 15)`.
- `Auto MAT-elev` can set the bit by slope or height threshold across all
  terrain.

Render translation:

- The shader reads `elev = (visual >> 15) & 0x1`.
- In the editor shader excerpt audited here, `elevated` also controls grid
  overlay color: elevated cells use cyan grid coloring, non-elevated cells use
  blue.
- The comments describe 4 material ramps. The exact audited shader snippet
  normalizes the sampled `elev` to row `1`, while other code/comments and the
  Rust runtime use elevation/ramp concepts more broadly. This is a known source
  of confusion and must be treated carefully when porting UI labels.

Workbench implication:

- The workbench should explain `MAT-elev` as a terrain visual flag/ramp selector
  and should not call it generic "elevation" without showing how it affects the
  final material lookup.
- A material probe should show whether bit `0x8000` is set and what final ramp
  row the Rust renderer actually uses.

## View, Spin, Zoom, Grid, and Light Controls

ASCIIID `VIEW` controls:

- `VIEW PITCH`: camera vertical angle.
- `VIEW YAW`: camera horizontal rotation.
- `Spin`: toggles automatic yaw rotation.
- `ZOOM`: changes `font_size`, which is effectively pixels-per-visual-cell /
  output density. The UI shows the resulting screen cell dimensions.
- `GRID`: changes grid overlay alpha.
- `NOON PITCH`, `NOON YAW`, `LIGHT TIME`, `AMBIENCE`: lighting controls.

Render translation:

- Mouse wheel changes `font_size`, which changes both visual scale and computed
  screen cell count.
- Right-mouse drag changes `rot_yaw` and `rot_pitch`.
- If `spin_anim` is true, ASCIIID increments `rot_yaw` by `0.1` each frame and
  wraps it at 180 degrees.
- The view matrix uses yaw, pitch, font size, position, and `HEIGHT_SCALE` to
  project terrain to screen.
- `GRID` modulates the final terrain color toward grid colors after material
  composition and palettization.

Workbench implication:

- The Rust workbench needs a Spin toggle because it is a direct ASCIIID view
  control and useful for repeatable visual inspection.
- Spin must be explicit and pauseable, with visible yaw and speed.
- Current Rust `resolution_scale` and `zoom` should be described separately:
  resolution scale changes ASCII grid density; zoom changes camera projection.

## Final Render Translation Summary

For terrain, ASCIIID's audited path can be summarized as:

1. Terrain visual cell stores material ID in low 8 bits and MAT-elev in bit 15.
2. Terrain geometry / normals and light controls compute a diffuse value.
3. Material lookup uses material ID, ramp/elevation selection, and diffuse
   shade index to fetch foreground RGB, background RGB, and glyph code.
4. Active font atlas samples the glyph alpha for that glyph code.
5. The shader blends background and foreground RGB by glyph alpha.
6. Palette lookup maps the resulting RGB through the active palette texture.
7. Probe overlays, grid alpha, brush previews, and front/back-face treatment
   can modify the final displayed color.

For workbench design, this means each visible tuning control should identify
which layer it affects:

- Camera/view layer: yaw, pitch, spin, zoom, position, grid density.
- Source/material layer: material ID, MAT-elev bit, material ramp, material
  glyph, foreground/background colors.
- Resolve/glyph layer: shape-vector mode, candidate glyph set, thresholding,
  fallback policy.
- Color/palette layer: palette selection, palettization, inversion/debug color
  transforms.
- Pass/visibility layer: terrain/world/sprite visibility, shadows, reflections,
  weather, culling.

## Current Rust Workbench Gaps From This Audit

- No route back to the workbench after `Resume Scene`.
- No ASCIIID-style Spin toggle or spin speed.
- No pitch control; Rust currently exposes yaw and zoom only.
- No grid-alpha control.
- No material probe showing MAT-id, MAT-elev, diffuse/ramp, glyph, and colors.
- No palette/palettization controls or diagnostics.
- No user-selectable glyph candidate set for shape-vector matching.
- No clear proof that shadows, rain, or culling toggles changed the final frame.
- Numeric readouts can be hidden in the right-side panel; values must remain
  visible and should not be right-aligned into clipped space.

## Recommended Workbench Changes

1. Add round-trip navigation: `Workbench -> Playing` must have a visible and
   keyboard-backed `Playing -> Workbench` return path.
2. Add Spin as a first-class View control: toggle plus speed slider/readout.
3. Add pass-effect diagnostics: when culling/shadows/weather toggles are
   changed, show counts/deltas that prove whether the pass affected the frame.
4. Add custom glyph candidate selection: a CP437 glyph grid, selected-glyph
   chips, and named sets. These sets must feed shape-vector matching, not just
   UI state.
5. Add a material probe panel before material editing: hovered cell material ID,
   MAT-elev bit, chosen diffuse shade, final glyph, foreground/background RGB,
   and palette/index outcome.
6. Fix numeric layout: values must remain visible at rest and inside the
   scrollable panel width on desktop and narrow windows.
7. Treat palette/material editing as advanced/editor-adjacent scope; keep the
   initial workbench focused on readouts and candidate-set tuning unless the
   user explicitly asks for map editing.
