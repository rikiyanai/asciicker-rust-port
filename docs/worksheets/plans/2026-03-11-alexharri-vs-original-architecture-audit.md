# 2026-03-11 Alex Harri vs Original Architecture Audit

## Question

Does Alex Harri's shape-vector renderer fit the original Asciicker render architecture cleanly, or is the current Rust integration applying it at the wrong layer?

## Short Answer

There is a real architectural conflict.

Alex Harri's method is an image-space character picker. It assumes glyph choice happens after sampling the visual content of a cell and that the main goal is sharper contour readability.

The original Asciicker renderer does not treat glyph choice that way. In `render.cpp`, glyphs are already part of the scene-semantic resolve path:

- terrain materials select glyphs
- mixed 2x2 cells switch into `auto_mat`
- half-block glyphs (`0xDE`, `0xDF`) encode split surfaces
- silhouette overlays force `_` or `-`
- line overlays force punctuation glyphs
- water ripple mainly modulates colors, not universal glyph replacement

So the current Rust approach of running shape-vector selection as a broad final-stage override is not a neutral “visual upgrade.” It can overwrite glyphs that already mean something in the original renderer.

## Sources

### Original C++ renderer

- `'(ORIGINAL GAME)asciicker-Y9-2-main/render.cpp'`
  - mixed reflection / non-reflection `use_auto_mat`
  - auto-mat split glyphs
  - silhouette / line overlays
  - water ripple branch

### Current Rust integration

- `engine-port/src/render/pipeline.rs`
- `engine-port/src/render/shape_vector.rs`
- `engine-port/src/render/resolve.rs`
- `engine-port/src/render/material.rs`

### Alex Harri article

- Alex Harri, “ASCII characters are not pixels: a deep dive into ASCII rendering”
  - https://alexharri.com/blog/ascii-rendering

Relevant points from the article:

- it is explicitly an image-to-ASCII renderer
- the core goal is sharper edges and better contour following
- more samples alone are not enough if each cell is still treated like a pixel
- the algorithm selects glyphs from 6D shape vectors by nearest-neighbor search
- contrast enhancement is applied to the sampling vector
- external samples push the internal vector to strengthen boundaries

### Prior local research

- `docs/worksheets/research/alexharri-asciicker-integration.md`

## What the Original Engine Actually Does

The original engine uses glyphs as part of render semantics, not just final presentation.

### 1. Resolve already decides structure

`render.cpp` resolves each output cell from a 2x2 sample block. That resolve step already decides:

- whether the cell is terrain or mesh driven
- whether to use material lookup or `auto_mat`
- whether the cell should be a half-block split
- whether it should receive a silhouette glyph
- whether it should receive a wireframe / linecase glyph

In other words, the original engine does not produce a neutral “image” and then choose ASCII afterwards. It produces ASCII as part of the scene logic itself.

### 2. Water is mostly color-domain at the end

In the original water branch, the fully-underwater condition applies Perlin-driven color modulation to the resolved cell. It is not a general postprocess that re-picks glyphs for all water-adjacent content.

### 3. Some glyphs are intentionally non-naturalistic

Half-blocks, linecase punctuation, and silhouette glyphs exist because they communicate discrete terrain / boundary structure, not because they are the nearest visual match to a sampled image patch.

## What Alex Harri's System Assumes

Harri's article assumes a different pipeline:

1. Start from an already-rendered image or grayscale/lightness field.
2. Sample each cell into a vector.
3. Pick the character whose shape best matches that sampled cell.
4. Apply contrast enhancement to improve edge readability.

This is a strong fit for:

- pure image-to-ASCII rendering
- postprocessed 3D scenes
- scenes where glyph choice has no independent semantic contract

It is a weaker fit for a renderer where glyphs already encode material and topology decisions.

## Where the Current Rust Port Conflicts

### 1. Shape-vector runs too late and too broadly

In `engine-port/src/render/pipeline.rs`, the current flow is:

1. resolve to `resolve_buf`
2. apply water ripple
3. run shape-vector glyph selection
4. write final grid

That makes shape-vector a broad final authority on glyph choice for most visible cells.

### 2. The sampled input is not Harri's intended source image

The shape-vector selector samples `SampleBuffer` lightness from materials / mesh colors. That is not the same as Harri's “source image” assumption.

The sampled field has already been shaped by:

- material tables
- terrain elevation buckets
- reflection tagging
- special spare-bit overlays
- auto-mat approximations

So shape-vector is not analyzing a clean raw image. It is analyzing an intermediate game-specific buffer with embedded semantics.

### 3. It can overwrite semantically meaningful glyphs

Today the Rust path can replace or suppress glyphs that the original resolve chose for reasons other than local shape matching:

- material glyphs
- split half-block glyphs
- silhouette overlays
- linecase overlays

This is the strongest reason the result can feel both sharper in places and chaotic / wrong in others.

### 4. The conflict is most obvious at boundaries

The user-visible symptoms match this:

- chaotic edges
- too many blank or near-blank cells
- limited obvious improvement despite heavy shape-vector work

That is exactly where a postprocess shape matcher will fight a renderer whose boundary glyphs are already purposeful.

## What Does Not Look Like the Main Conflict

### Mage Core

Mage Core is not the source of this issue.

This repo uses Mage Core ideas in the output layer:

- 4-texture GPU composition
- font / fg / bg / glyph index packing
- fullscreen WGSL compositing

That is downstream from glyph selection. It affects presentation, not the semantic choice of glyphs in resolve.

### ECS

Bevy ECS is also not the main source of the conflict.

The current problem is not “ECS vs OOP.” It is “where in the pipeline shape-based glyph choice belongs.”

ECS does matter for maintainability and debug visibility, but not for the core mismatch described here.

## Why the Current Visual Gain Can Be Hard To See

The user’s observation that the image still does not obviously look better is consistent with this audit.

Reasons:

- the original renderer already had strong glyph structure in some cases
- shape-vector can improve contour matching while simultaneously erasing original semantic glyph choices
- threshold rejection and fallback-to-space reduce visible ink
- low-confidence structural swaps can create motion/noise without improving readability

So the likely outcome of a broad “replace auto_mat glyphs with Harri glyphs” strategy is exactly what we are seeing:

- some local wins
- weak global readability gain
- boundary instability

## Architectural Recommendation

Do not treat shape-vector as a universal final glyph replacement stage.

Instead, constrain it.

### Recommended policy

Shape-vector should be allowed only on cells that are visually eligible and semantically weak.

Good candidates:

- ordinary terrain/material cells
- non-overlay cells
- non-water cells
- non-reflection mixed-edge cells
- non-auto-mat split cells
- cells where resolve would otherwise land on weak / blank structure

Bad candidates:

- silhouette overlay cells
- linecase / wireframe cells
- half-block split cells
- explicit auto-mat split cells
- strong water/reflection boundary cells
- UI / text / sprite cells

### Better integration model

The best fit is likely:

- original resolve remains authoritative
- shape-vector acts as a constrained structural refinement layer
- it is used only where the original glyph is weak, blank, or non-informative

That is much closer to “copy the technique” than “replace the renderer’s glyph semantics.”

## Recommended Next Steps

1. Add explicit shape-vector eligibility buckets in the debug metadata.
2. Prevent shape-vector from running on:
   - silhouette cells
   - linecase cells
   - split half-block cells
   - mixed reflection / auto-mat cells
3. Compare the orbit baseline again after bucket gating.
4. Re-evaluate whether Harri should be:
   - a selective fallback for weak cells, or
   - a separate render mode, not the default renderer path

## Conclusion

The current limited visual improvement is not strong evidence that Harri's method is bad.

It is stronger evidence that the current integration point is wrong.

Harri's method is built to choose glyphs for sampled image cells.
The original Asciicker renderer already uses glyphs to encode scene logic.

Those two approaches can coexist, but only if shape-vector is constrained to the parts of the frame where it is actually additive rather than destructive.
