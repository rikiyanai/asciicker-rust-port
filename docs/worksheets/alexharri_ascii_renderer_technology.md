# Alex Harri's ASCII Rendering Technology

Source article: https://alexharri.com/blog/ascii-rendering

## What he built
Alex Harri built a high-quality, real-time image/video-to-ASCII renderer that avoids the common "ASCII looks blurry" problem by matching **character shape** to local image structure, instead of treating characters like square pixels.

## Core idea
Most ASCII renderers map brightness to character density. That works, but edges look soft because character geometry is ignored.

His approach models each character as a **shape vector** and does nearest-neighbor matching in vector space:

1. Precompute a shape vector for each character (how much ink appears in multiple regions of a cell).
2. For each output cell, sample the source image to build a corresponding sampling vector.
3. Pick the character whose shape vector is closest (Euclidean distance).

## Rendering pipeline (high level)
1. Build ASCII grid over the source image/canvas.
2. Compute per-cell sampling vectors (first 2D, then 6D for better feature capture).
3. Normalize character vectors by component-wise maxima so dimensions are comparable.
4. Do nearest-neighbor character lookup.
5. Apply two contrast effects to improve boundary readability:
- Global contrast enhancement.
- Directional contrast enhancement using external samples from neighboring regions.
6. Render characters to canvas/DOM.

## Why 6D shape vectors
A 2D upper/lower representation is too coarse for many glyphs. Moving to a 6D layout (staggered sampling circles) captures directional structure better (e.g., stems and corners), which significantly improves character selection along contours.

## Contrast enhancement strategy
### Global contrast
Normalize by max component, apply exponent, denormalize. This exaggerates dominant components while preserving vector scale.

### Directional contrast
Use an **external sampling vector** (samples outside the cell) to detect nearby boundaries and darken affected components.

### Widened directional effect
Map each internal component to multiple external components (`AFFECTING_EXTERNAL_INDICES`) so edge influence spreads spatially and reduces staircasing artifacts.

## Performance strategy
He reports early versions were too slow on mobile, then optimized with:

1. **k-d tree** for 6D nearest-neighbor queries.
2. **Quantized cache keys** for sampling vectors (bit-packed) to avoid repeated searches.
3. **GPU acceleration (WebGL shader passes)** for sampling collection and contrast passes.

GPU passes include collecting internal/external vectors and applying directional/global crunch steps in textures, which moved heavy per-frame work off CPU and enabled smooth rendering.

## Key tradeoff
The system deliberately sacrifices strict per-cell fidelity to improve overall readability and edge clarity. The output is more legible and visually coherent, especially for animated scenes.
