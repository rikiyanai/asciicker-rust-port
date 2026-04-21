# 2026-04-21 Figma 3D ASCII Model Viewer UI/UX Research

Status: worksheet  
Authority: non-canonical research note  
Related canon: `docs/CANONICAL_SPEC.md`

## Purpose

Capture the exact rendered UI and interaction surface of the live Figma Make
`3D ASCII Model Viewer` page that was open in local Chrome during this review,
then preserve the page shell used to bootstrap the site.

This worksheet exists so future spec and implementation work can refer to a
local, stable snapshot instead of relying on the live Figma page remaining
reachable.

## Sources

1. Live rendered page in local Chrome:
   - tab title: `3D ASCII Model Viewer`
   - runtime URL: `https://onyx-trace-71142944.figma.site/`
2. Figma community source file reference:
   - `https://www.figma.com/community/file/1530223431472150953/3d-ascii-model-viewer`
3. Static page shell copied from the user-provided source:
   - [2026-04-21-figma-3d-ascii-model-viewer-page-shell.html](2026-04-21-figma-3d-ascii-model-viewer-page-shell.html)

## Capture Method

The rendered UI was inspected by executing JavaScript in the live Chrome tab.
This produced:

- page title
- `document.body.innerText`
- visible control labels
- control bounding rectangles
- canvas presence and canvas size
- basic computed-style data

The Figma community page itself was not directly scrapeable through the web
tooling in this environment. The rendered `figma.site` page and the provided
HTML shell were therefore treated as the durable evidence.

## Exact Rendered UI

### Viewport and Surface

- Viewport at time of inspection: `837 x 805`
- Background color: `rgb(255, 255, 255)`
- Main render surface: exactly one `CANVAS`
- Canvas bounds: `x=0`, `y=0`, `w=837`, `h=805`

Interpretation:

- The UI is not a boxed app card inside a page.
- The canvas fills the viewport and acts as the page itself.
- Controls sit directly on top of the canvas as overlays.

### Visible Copy

Visible text extracted from the rendered page:

```text
Logo
Computer
Plant
Shiba
Crystal
Presets
.:-=+*#%@.-+*#
Resolution
0.220
Scale
1.00
Invert colors
Reset
Credits
```

The body text also included the large ASCII-rendered output from the canvas
content itself.

## Control Inventory

Structured control extraction from the rendered DOM produced these visible
interactive labels:

```json
[
  {"tag":"BUTTON","text":"Logo"},
  {"tag":"BUTTON","text":"Computer"},
  {"tag":"BUTTON","text":"Plant"},
  {"tag":"BUTTON","text":"Shiba"},
  {"tag":"BUTTON","text":"Crystal"},
  {"tag":"LABEL","text":"Presets"},
  {"tag":"BUTTON","text":".:-=+*#%@"},
  {"tag":"BUTTON","text":".-+*#"},
  {"tag":"LABEL","text":"Resolution"},
  {"tag":"LABEL","text":"Scale"},
  {"tag":"BUTTON","text":"on"},
  {"tag":"LABEL","text":"Invert colors"},
  {"tag":"BUTTON","text":"Reset"},
  {"tag":"BUTTON","text":"Credits"}
]
```

Notes:

- `Resolution` and `Scale` were visible with numeric values (`0.220` and
  `1.00`) in page text, but no native HTML `input` elements were exposed in the
  DOM query. They may be custom-rendered or canvas-driven controls.
- The invert toggle exposed a visible state button with text `on`.
- `Credits` appears as a lightweight secondary action rather than a primary
  control.

## Layout Geometry

Bounding boxes captured from the rendered DOM:

| Element | X | Y | W | H |
|---|---:|---:|---:|---:|
| Canvas | 0 | 0 | 837 | 805 |
| `Logo` | 49 | 290 | 72 | 23 |
| `Computer` | 49 | 341 | 72 | 23 |
| `Plant` | 49 | 391 | 72 | 23 |
| `Shiba` | 49 | 442 | 72 | 23 |
| `Crystal` | 49 | 492 | 72 | 23 |
| `Presets` | 623 | 188 | 172 | 23 |
| `.:-=+*#%@` | 623 | 221 | 172 | 28 |
| `.-+*#` | 623 | 256 | 172 | 28 |
| `Resolution` | 623 | 312 | 172 | 23 |
| `Scale` | 623 | 405 | 172 | 23 |
| `Invert colors` | 644 | 498 | 117 | 23 |
| `Reset` | 623 | 549 | 172 | 32 |
| `Credits` | 623 | 594 | 63 | 23 |

Implications:

- The left-side fixture selector is a narrow vertical stack.
- The right-side controls form a compact floating stack rather than a full
  sidebar.
- The canvas remains the dominant visual object.
- There is no visible top nav, hero, intro copy, or boxed settings inspector.

## Visual Language

Observed from computed styles:

- Body background is white.
- The canvas itself is transparent at the DOM layer and visually provides the
  actual scene.
- Control text uses a light/white foreground against the rendered field.
- Preset buttons and `Reset` use soft rounded rectangles:
  - border radius approximately `6.75px`
  - faint translucent white fills and borders
- `Credits` is visually lighter-weight than the main buttons.
- The left fixture picker buttons are plain text buttons without heavy chrome.

This produces a minimal tool-like interface:

- sparse labels
- low panel chrome
- maximum surface area for the actual ASCII render

## Exact UX Pattern

### 1. Source selection is immediate

The available fixture choices are directly visible at rest:

- `Logo`
- `Computer`
- `Plant`
- `Shiba`
- `Crystal`

There is no hidden dropdown or modal chooser. Switching source appears to be a
single-click action from the left rail.

### 2. The workspace is primary

The entire viewport behaves like the renderer surface first and the UI second.
This is not a documentation page describing a viewer. It is the viewer.

### 3. Controls are compact and low-friction

The right side contains only:

- presets
- resolution
- scale
- invert toggle
- reset
- credits

That is a deliberately small control set. No explanatory copy is present.

### 4. Numeric state is visible

The current values for resolution and scale are visible in the UI without user
interaction:

- `Resolution: 0.220`
- `Scale: 1.00`

This matters because the current state is inspectable at a glance.

### 5. Presets are encoded as the value itself

The preset buttons display the actual glyph-set strings, not friendly labels
like "Dense" or "Sparse". The character string itself is the control label.

That is important product behavior:

- the user sees the output alphabet directly
- the control doubles as documentation of what will render

### 6. Secondary actions remain secondary

`Credits` is present but visually subordinate. It does not compete with the
main tuning controls.

## Canon-Safe Conclusions

These are the high-confidence conclusions suitable for canon/spec alignment:

1. The target should be called a `Render Workbench`, not a `demo`.
2. The viewport should be a full-bleed canvas with overlay controls.
3. Source selection belongs on the left as a compact vertical rail.
4. Render controls belong on the right as a compact floating stack.
5. Resolution and scale must show live numeric state.
6. Preset buttons should expose the glyph set itself, not only abstract names.
7. The interface should avoid onboarding prose and heavy panel chrome.

## Limits

- This worksheet captures the rendered UI surface and visible DOM only.
- It does not recover the underlying bundled Figma Make component code.
- It does not prove hidden interactions that were not surfaced by the DOM
  queries.
- It captures one viewport size (`837 x 805`) rather than a full responsive
  matrix.

## Local Artifact

The static page shell used to bootstrap the Figma site is preserved here:

- [2026-04-21-figma-3d-ascii-model-viewer-page-shell.html](2026-04-21-figma-3d-ascii-model-viewer-page-shell.html)

That file is the exact HTML shell supplied during this review and should be
treated as a supporting artifact, not as proof of the full runtime DOM.
