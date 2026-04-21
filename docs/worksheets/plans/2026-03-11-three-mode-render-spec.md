# Three-Mode Render Specification
## 2026-03-11 Deep Audit — Original / Harri / Combined

---

## Part 1 — Original Game Deep Audit (C++ render.cpp)

### 1.1 The Six Resolve Branches

The RESOLVE stage (render.cpp:3655–4070) iterates every output cell. For each 2×2 sample block it applies one of four primary paths, then three overlay passes. The exact decision tree is:

```
for each output cell (cx, cy):
  examine 2×2 block (sx, sy) = (2+2cx, 2+2cy)

  ┌── ALL CLEAR? → sky color (blue), glyph=' '
  │
  ├── TERRAIN ONLY (no mesh, no reflection):
  │   mat = visual & 0xFF
  │   elevation = bit15 pattern in rows above/below
  │   shade = (dif[0]+dif[1]+dif[2]+dif[3] + 34) / 68   (0..15)
  │   → matlib[mat].shade[elevation][shade].{fg, bg, glyph}
  │
  ├── REFLECTION ONLY (all samples spare & 0x03 == 0x03, no mesh):
  │   Same as TERRAIN but dim: fg/bg *= 255/400
  │
  ├── MESH or MIXED (any sample has spare & 0x08):
  │   For each of 4 samples: bg_color = auto_mat_bg (rgb2pal of sample's lit color)
  │   top    = avg(samples[0], samples[1])   ← upper row
  │   bottom = avg(samples[2], samples[3])   ← lower row
  │   left   = avg(samples[0], samples[2])   ← left col
  │   right  = avg(samples[1], samples[3])   ← right col
  │   err_h  = sum_sq_error(top) + sum_sq_error(bottom)
  │   err_v  = sum_sq_error(left) + sum_sq_error(right)
  │   if err_h*1000 < err_v*999:  gl=0xDF (▀), bk=auto_mat(top),   fg=auto_mat(bottom)
  │   if err_v*1000 < err_h*999:  gl=0xDE (▐), bk=auto_mat(left),  fg=auto_mat(right)
  │   else:
  │     avg_rgb = average of all 4 bg_colors
  │     (bg_pal, fg_pal, dither_gl) = auto_mat_table[rgb555(avg)]
  │     if bg_pal == fg_pal:                          ← bk==fg FALLBACK (MISSING in Rust)
  │       gl = dither_gl from " ..::%"
  │     else:
  │       gl = 0xDF (default half-block)
  │
  └── POST-OVERLAYS (applied in this order, each can override the above):
      1. GRID LINECASE (elv==3, spare & 0x04):
         linecase = 4-bit bitmask of 0x04 in 2×2 block samples
         gl = LINECASE_GLYPHS[linecase] = {0,',',',',',','`',';',';',';',...}

      2. SILHOUETTE (elv==1||3, !reflection):
         z_lo = height of row sy    (2 samples, summed)
         z_hi = height of row sy+1  (2 samples, summed)
         z_pr = height of row sy-1  (2 samples, summed)
         minus = z_lo - z_hi
         under = z_pr - z_lo
         if minus > under and minus > HEIGHT_SCALE:  gl=0xC4 ('-'), fg=darken(bk)
         if under > minus and under > HEIGHT_SCALE:  gl=0x5F ('_'), fg=darken(bk)

      3. WIREFRAME LINECASE (spare & 0x40):
         linecase = 4-bit bitmask of 0x40 in 2×2 block samples (shifted)
         gl = same LINECASE_GLYPHS table
         fg = 16 (palette index 16 = dark color)

      4. WATER RIPPLE (fully underwater cells, Perlin noise):
         inv_tm[4×4] used to compute Perlin UV from sample coordinates
         noise modulates bg_pal (lighten/darken via PerlinNoise2D)
         applied to the resolved fg/bg palette indices
```

### 1.2 auto_mat_bg — What It Actually Does

`auto_mat_bg` (used in the MESH/MIXED path) extracts the background color from a sample:

- For mesh samples (`spare & 0x08`): decode RGB555 from `sample.visual`, scale by `sample.diffuse/255`
- For terrain samples: look up `matlib[mat_idx].shade[elevation][shade_idx].bg`
- Quantize RGB888 → xterm-256 palette index via `rgb2pal()`: `(channel + 25) / 51` → 6×6×6 cube index

The key point: `auto_mat` resolves a palette index, not an RGB value. The VH-split compares palette indices from auto_mat to decide the split direction. This means the split decision is made in xterm-256 palette space, not in RGB888 space.

The Rust port does this in RGB888 space (`average_partition` / `partition_error` in resolve.rs), then converts to palette at the end. This introduces quantization differences vs C++.

### 1.3 darken_palette_index — C++ Exact Formula

C++ render.cpp silhouette path (around line 3730):
```cpp
fg = darken(bk);
// darken operates on xterm-256 cube index:
// r_idx = (pal - 16) / 36
// g_idx = ((pal - 16) % 36) / 6
// b_idx = (pal - 16) % 6
// r_new = max(0, r_idx - 1)
// g_new = max(0, g_idx - 1)
// b_new = max(0, b_idx - 1)
// return 16 + r_new*36 + g_new*6 + b_new
```

The Rust port's `darken_palette_index()` (resolve.rs) needs verification against this. The C++ formula decrements the cube coordinate by 1, not a continuous dim. It only works correctly for palette indices 16–231 (the 6×6×6 cube). Grayscale ramp indices (232–255) need separate handling.

### 1.4 Terrain Grid Lines — COMPLETELY MISSING FROM RUST PORT

The terrain grid system (render.cpp:2239–2255) writes a center cross through each terrain patch after the triangle fill completes. Requirements:

```
after RenderPatch() completes for patch (ph, pv):
  mid = (HEIGHT_CELLS + 1) / 2      // center cell = 2 for HEIGHT_CELLS=4
  // raise heights to sit above surface: height += HEIGHT_SCALE/2
  Bresenham(horizontal line through row ph*2 + mid*2 + 1, v0..v1, flag=0x04)
  Bresenham(vertical line from v0 to v1 through col ph*2 + mid*2 + 1, flag=0x04)
  // only when !global_refl_mode
```

`Bresenham()` (render.cpp:313–400) writes the spare bit ONLY — it does NOT change color or height. It uses `DepthTest_RO()` (read-only depth test): the bit is written only if the sample at that position has the terrain height. Grid lines placed behind other geometry are silently dropped.

Rust status: **ZERO implementation**. `terrain_shader.rs` has no Bresenham call and no post-patch cross. Because the 0x04 bit is never written, the GRID linecase in `apply_post_overlays` (resolve.rs:522–535) can NEVER trigger even though the code path exists.

### 1.5 Wireframe Edges — COMPLETELY MISSING FROM RUST PORT

The wireframe system (render.cpp:1270–1273) activates when a mesh face has bit 31 of its visual field set. In that case:
```cpp
Bresenham(v0→v1, flag=0x40)
Bresenham(v1→v2, flag=0x40)
Bresenham(v2→v0, flag=0x40)
return;  // skip fill entirely
```

Rust status: `mesh_shader.rs:27–35` sets ONLY `MESH_FLAG (0x08)`, never `WIREFRAME (0x40)`. The wireframe linecase in `apply_post_overlays` (resolve.rs:576–591) can NEVER trigger. There is also no Bresenham implementation in the Rust rasterizer.

### 1.6 Mesh Parity Bits — PARTIALLY MISSING

C++ RenderFace::Shader::Blend() (render.cpp:1055–1110):
```cpp
// reflection mode:
sample.spare = (sample.spare & ~0x44) | 0x8 | 0x3;  // MESH + REFLECTION parity
// normal mode:
sample.spare = (sample.spare & ~0x44) | 0x8 | 0x1;  // MESH + NORMAL parity
```

Rust mesh_shader.rs:27–35: sets `spare = spare_bits::MESH_FLAG (0x08)` only. Parity bits 0x1 (normal) and 0x3 (reflection) are never written.

Impact: `resolve.rs` checks parity bits to detect reflection terrain. Mesh samples always appear as parity=0 (which falls into neither normal nor reflection path), causing mesh cells to be treated as `use_auto_mat = true` regardless. This is partially correct (mesh uses auto-mat) but the missing parity bits mean reflection pass mesh geometry is indistinguishable from normal pass mesh.

### 1.7 Water Mesh Clamping — MISSING

C++ RenderFace::Shader::Blend() (render.cpp:1063–1110) has water threshold gates:
```cpp
// reflection pass (global_refl_mode):
if (pos[2] < r->water + HEIGHT_SCALE/8) return;   // skip faces below water in reflection

// normal pass:
if (pos[2] >= r->water - HEIGHT_SCALE/8)           // clamp to water surface
    height = min(height, r->water);
```

Rust mesh_shader.rs: no water check. Mesh faces below the water plane are rendered when they shouldn't be (in reflection) or aren't clamped to the surface (in normal).

### 1.8 bk==fg Dither Fallback — MISSING

In the MESH/MIXED path, after the VH-split fails (err_h ≈ err_v), the C++ code does:
```cpp
auto_mat_table[rgb555] → (bg_pal, fg_pal, dither_gl)
if (bg_pal == fg_pal): use dither_gl (from " ..::%")
else: use 0xDF (default)
```

Rust resolve.rs:313–324: always returns `dither_glyph` from auto_mat_lookup without checking `bg == fg`. The dither glyph from auto_mat_lookup is the one to use when there's no split — but the `0xDF` fallback from C++ is only for when `bg != fg`. Rust may be using the dither glyph when it should use 0xDF or vice versa.

---

## Part 2 — Alex Harri Method Deep Audit

### 2.1 Algorithm — Exact Steps

Alex Harri's method (from article + code audit):

**Offline (once per alphabet load):**
1. Render each CP437 glyph to a pixel buffer (font → image)
2. For each glyph, at 6 circle positions: sample average luma using Vogel spiral (golden angle, N subsamples)
3. Build a 6D vector per glyph → store in `default.json`
4. Build k-d tree over all 6D vectors

**Runtime (per cell, per frame):**
1. Compute 6D shape vector from the SOURCE IMAGE (rendered scene) at positions matching the 6 circles
   - Use same circle positions and radius as the glyph vectors
   - Vogel spiral sampling with golden angle: `radial = sqrt((i+0.5)/N) * radius`, `angle = golden_angle * i`
   - Luma formula: `0.2126*r + 0.7152*g + 0.0722*b` (Rec. 709)
   - Circle radius = 0.28125 × min(cell_width, cell_height) at runtime
2. Apply DIRECTIONAL CRUNCH (exponent=7 default):
   - Compute 10 external sample values (same method, positions outside cell boundary)
   - For each internal dim `i`: `context = max(external_values[affects_mapping[i]])`
   - If `context > value`: `value = (value/context)^7 * context`
   - Effect: darkens internal dims that have bright neighboring context → sharpens edges
3. Apply GLOBAL CRUNCH (exponent=3 default):
   - `max_val = max(all 6 dims)`
   - For each dim: `value = (value/max_val)^3 * max_val`
   - Effect: boosts dominant dims, suppresses weak ones → increases within-cell contrast
4. k-d tree nearest-neighbor search (Euclidean distance in 6D) → glyph
5. Quantized LRU cache with 5-bit quantization per dim → 30-bit key

**What Alex Harri does NOT specify:**
- Fg/bg color optimization — his method selects only glyphs, not colors
- He operates on a grayscale/luma source image and already has colors from the source
- In Asciicker context, "colors" must come from somewhere else

### 2.2 What the Rust Port Adds (Not from Alex Harri)

The Rust port adds `optimize_glyph_colors()` (shape_vector.rs:882–939) which is **original work not from Alex Harri**:

Given a selected glyph G and the 2×2 cell region of the SampleBuffer:
1. Load the glyph's pixel mask from the runtime CP437 font PNG (10×16 pixels)
2. For each font pixel (gx, gy): compute `ink` (luma of pixel), `bg = 1 - ink`
3. Map font pixel position to sample buffer position via bilinear interpolation
4. Build linear system: `fg * ink + bk * bg ≈ source_rgb[channel]` for each font pixel
5. Solve 2×2 normal equations per channel (least squares): find fg, bk that minimizes total squared error
6. Quantize to xterm-256 palette

This is a correct and sophisticated color optimization. It is the right approach for the Combined mode.

### 2.3 Critical Bug in Sampling Consistency

There is an inconsistency between:
- **Glyph selection sampling** (`select_glyph_with_debug` → `sample_vector_with_points` → `bilinear_sample_lightness` → `sample_to_lightness`):
  - For terrain: returns luma of `mat_cell.fg` (foreground text color)
  - For mesh: returns luma of `diffuse_scale * rgb555`

- **Color optimization sampling** (`optimize_glyph_colors` → `bilinear_sample_rgb` → `sample_to_rgb`):
  - For terrain: returns `mat_cell.bg` (background color)
  - For mesh: returns `diffuse_scale * rgb555`

The glyph is selected based on the **fg** color's luma, but the color optimization minimizes error against the **bg** color. This is incoherent. The glyph shape should be selected to match what the eye sees, which for terrain is primarily the **bg** (background fills the whole cell) not the fg (only fills the glyph ink).

**Fix:** `sample_to_lightness` for terrain should use `mat_cell.bg` not `mat_cell.fg`.

### 2.4 HarriPriority Mode Does Not Optimize Colors

`choose_final_glyph` with `ShapeVectorMode::HarriPriority` (pipeline.rs:227–229):
```rust
if config.mode == ShapeVectorMode::HarriPriority {
    return (decision.glyph.unwrap_or(resolve_glyph), false, false);
}
```
Returns `(glyph, preserved_resolve=false, semantic_gate=false)`.

Then `choose_final_colors` (pipeline.rs:267–269):
```rust
if preserved_resolve || semantic_gate {
    return (resolve_fg_rgb, resolve_bg_rgb);
}
```
Not gated, so it proceeds to line 271:
```rust
let Some(selected_glyph) = decision.glyph else {
    return (resolve_fg_rgb, resolve_bg_rgb);
```
If decision.glyph is Some, it reaches line 274:
```rust
if selected_glyph != final_glyph || final_glyph == resolve_glyph {
    return (resolve_fg_rgb, resolve_bg_rgb);
}
```
In HarriPriority, `final_glyph = decision.glyph` and `resolve_glyph` may differ, so `final_glyph != resolve_glyph`. The condition `selected_glyph != final_glyph` is false (both are decision.glyph). The condition `final_glyph == resolve_glyph` may be false if shape-vector selected differently. So this DOES fall through to `optimize_glyph_colors`.

Wait — actually this is correct. The color optimization IS called in HarriPriority mode when the glyph differs. But only when `final_glyph != resolve_glyph` AND `selected_glyph == final_glyph`. In HarriPriority mode, `final_glyph = decision.glyph.unwrap_or(resolve_glyph)`, so if `decision.glyph` is None (threshold rejected), `final_glyph = resolve_glyph`, and the condition `final_glyph == resolve_glyph` is true → returns resolve colors (correct).

If `decision.glyph` is Some(g) where g ≠ resolve_glyph → `final_glyph = g`, `selected_glyph = g`, condition `selected_glyph != final_glyph` is FALSE, condition `final_glyph == resolve_glyph` is FALSE → falls through to `optimize_glyph_colors`. Color optimization DOES run.

**So `HarriPriority` mode IS calling optimize_glyph_colors.** The problem must be elsewhere.

### 2.5 The Real Problem With HarriPriority Mode

In HarriPriority mode, ALL semantic gates are skipped. This means:
- Silhouette cells get their overlay glyphs (`-`, `_`) replaced by shape-vector glyphs
- Linecase cells get their punctuation replaced
- Half-block split cells (0xDE, 0xDF) get replaced
- Mixed reflection/terrain cells get replaced

These replaced cells often have HIGH contrast between fg and bg (they're boundary cells). The color optimization works on the sample geometry, not on the glyph's semantic meaning. So a linecase `,` glyph might get replaced by `M` with optimized colors — visually the edge structure is destroyed.

Additionally, the sampling using `fg` luma (the bug from 2.3) means the shape-vector is making decisions based on wrong input.

### 2.6 The 6D Vector Sampling and Cell Coverage

The 6 sampling circles in default.json are positioned at:
- (0.25, 0.25): upper-left quadrant
- (0.75, 0.25): upper-right quadrant
- (0.25, 0.5): middle-left
- (0.75, 0.5): middle-right
- (0.25, 0.75): lower-left
- (0.75, 0.75): lower-right

Radius = 0.28125 × cell_width (normalized).

In the Rust sample buffer, each ASCII cell maps to 2×2 samples. A "cell width" of 2 sample units means circle radius = 0.28125 × 2 = 0.5625 sample units. The circle diameter is ≈ 1.125 sample units — barely more than a single sample. With 8 subsamples (golden angle Vogel), this is approximating the circle from a 1-sample-wide region.

This means the 6D vector captures 6 distinct single-sample-resolution points. With only 2×2 samples per cell (after border compensation), this is coarse sampling and adjacent circles are overlapping. Compare to Alex Harri's original use case where each cell spans many pixels.

**Implication:** The shape-vector approach works better with a higher resolution sample buffer (4× or 8× supersampling) or a lower visual character density. At 2×2 samples per cell, directional crunch is almost meaningless (external points only differ by 2–3 samples from internal points).

---

## Part 3 — Mode Specifications

### Mode 1: ORIGINAL MODE — Full Faithful C++ Port

**Goal:** Pixel-identical to the C++ game's visual output given the same world and camera.

**Requirements (all MUST be satisfied):**

#### R-O-01: Terrain Grid Bresenham
In `terrain_shader.rs` (or a post-terrain pass):
- After each patch is rasterized, compute `mid = (HEIGHT_CELLS + 1) / 2` (= 2)
- Identify the patch center column and row in sample buffer coordinates
- Run a Bresenham scan writing `spare |= 0x04` only (no color/height change)
- Depth-test: only write if `sample.height >= (patch_height - epsilon)` (approximately)
- Only in non-reflection mode
- The heights used for the Bresenham pass must be raised by `HEIGHT_SCALE/2` relative to patch geometry

#### R-O-02: Wireframe Bresenham
In `mesh_shader.rs`:
- When face visual bit 31 is set: run Bresenham on all 3 edges writing `spare |= 0x40`, skip fill
- The Bresenham algorithm traverses in 2D screen coordinates
- Only writes spare bit, no color/height change
- Depth-test: only write if new height >= existing height

#### R-O-03: Mesh Parity Bits
In `mesh_shader.rs`:
- Normal pass (non-reflection): `spare = (spare & !0x44) | 0x08 | 0x01`
- Reflection pass (`global_refl_mode`): `spare = (spare & !0x44) | 0x08 | 0x03`

#### R-O-04: Mesh Water Clamping
In `mesh_shader.rs`:
- Reflection pass: skip face if face position < water_z + HEIGHT_SCALE/8
- Normal pass: clamp height to water_z if face position >= water_z - HEIGHT_SCALE/8

#### R-O-05: bk==fg Dither Fallback
In `resolve.rs` → `resolve_auto_mat_cell()`:
- In the VH-split fallback path: after auto_mat_lookup returns `(bg, fg, dither_gl)`
- If `bg_pal == fg_pal`: use `dither_gl` (the density glyph from " ..::%")
- If `bg_pal != fg_pal`: use `0xDF` as default half-block

#### R-O-06: darken_palette_index Exact Formula
In `resolve.rs`:
```
fn darken_palette_index(pal: u8) -> u8 {
    if pal < 16 || pal > 231 { return pal; }  // skip system colors and grayscale
    let idx = pal as usize - 16;
    let r = idx / 36;
    let g = (idx % 36) / 6;
    let b = idx % 6;
    let r_new = r.saturating_sub(1);
    let g_new = g.saturating_sub(1);
    let b_new = b.saturating_sub(1);
    (16 + r_new * 36 + g_new * 6 + b_new) as u8
}
```

#### R-O-07: auto_mat in RGB555 Space
The VH-split error comparison should use `rgb2pal` → palette index comparison (as C++ does) not RGB888 partition error. This requires converting the 4 sample BG colors to palette indices FIRST, then averaging/comparing palette-indexed colors. (Or accept the approximation as "close enough" if the palette quantization is reversible.)

#### R-O-08: Water Ripple
After resolve, for cells where all 4 samples have `height < water_z`:
- Compute Perlin UV from cell screen position via inverse transform matrix
- Sample Perlin2D noise at that UV
- Modulate bg_pal: if noise > threshold → apply lighten_palette_index; else darken

Note: The exact C++ Perlin formula (render.cpp:~3850–3900) must be read and ported exactly. The noise output drives a color shift, not a glyph change.

#### R-O-09: resolve() Uses Original Order
The post-overlay order MUST be: grid linecase (elv==3 only) → silhouette → wireframe linecase. This is already correct in `apply_post_overlays`.

---

### Mode 2: HARRI MODE — Pure Shape-Vector Output

**Goal:** Maximum shape-matching visual quality. No C++ semantic glyphs. Looks like a high-quality image-to-ASCII conversion of the rendered scene.

**Requirements:**

#### R-H-01: Sampling Uses Background Color
`sample_to_lightness` must use `mat_cell.bg` for terrain (not `fg`):
```rust
// terrain path:
let mat_cell = material.lookup(0, sample.diffuse);
let r = mat_cell.bg[0] as f32;  // WAS: mat_cell.fg
let g = mat_cell.bg[1] as f32;  // WAS: mat_cell.fg
let b = mat_cell.bg[2] as f32;  // WAS: mat_cell.fg
```

#### R-H-02: Full Crunch Pipeline Active
Both global crunch (exponent=3) and directional crunch (exponent=7) must run by default in Harri mode. Current defaults of 2.5 and 6.0 deviate from the reference values.

#### R-H-03: No Semantic Gates
In Harri mode, `should_gate_shape_vector_by_cell_class` is NOT called. Shape-vector runs on every non-clear, non-underwater cell. This is the defining characteristic of Harri mode.

#### R-H-04: Color Optimization Always Runs
When shape-vector selects a glyph different from the sky/clear fallback, `optimize_glyph_colors` must be called. Current code path does this correctly (see Part 2 analysis), but only if `selected_glyph != final_glyph` AND `final_glyph != resolve_glyph`. Ensure this condition is always satisfied in Harri mode by always calling optimization when `decision.glyph.is_some()`.

#### R-H-05: Distance Threshold Tuning
The default `distance_threshold: 0.08` is empirical. For Harri mode, a higher threshold (0.12–0.15) reduces the fallback-to-space rate and produces more ink-dense output. Consider separate defaults for each mode.

#### R-H-06: Temporal Coherence Note
Harri mode has inherent temporal flicker because k-d tree nearest-neighbor is not continuous — small changes in lightness can snap to a very different glyph. The quantized LRU cache provides implicit smoothing but only for cells that quantize to the same key. Consider frame-blended character selection as a future enhancement.

---

### Mode 3: COMBINED MODE — Correct Integration

**Goal:** Use original C++ semantics for structurally meaningful cells; use Alex Harri shape-vector for ordinary surface cells. The sum should look better than either alone.

**Requirements:**

#### R-C-01: The Eligibility Partition

Every cell falls into exactly one bucket:

**SEMANTIC bucket (original resolve, no shape-vector):**
- Any cell where `apply_post_overlays` set a silhouette overlay
- Any cell where `apply_post_overlays` set a grid linecase
- Any cell where `apply_post_overlays` set a wireframe linecase
- Any cell where resolve chose a half-block split glyph (0xDE or 0xDF)
- Any cell in a mixed reflection/normal terrain boundary (HAS_REFLECTION && HAS_NORMAL_TERRAIN)
- Any cell that is fully underwater

**SHAPE-VECTOR bucket (shape-vector with color optimization):**
- Ordinary terrain cells (material path, no overlays, no half-block)
- Pure mesh cells (MESH_FLAG set, no reflection mixing)
- Clear cells (already handled: sky color, space glyph)

#### R-C-02: Signal the Bucket via AnsiCell.spare

`resolve()` writes `AnsiCell.spare = 0` to signal eligible, `AnsiCell.spare = 0x01` to signal semantic. `resolve_to_grid` and the pipeline loop read this flag before calling the glyph selector.

Currently `resolve.rs:213` writes `spare: 0xFF` for all cells. This must be changed to encode the overlay result.

Minimal implementation: the `OverlayResult` struct already captures which overlays fired. Thread it back via:
```rust
output[out_idx].spare = if overlay.grid || overlay.silhouette || overlay.linecase {
    0x01  // semantic — skip shape-vector
} else if matches!(cell.gl, 0xDE | 0xDF) {
    0x01  // half-block split — skip shape-vector
} else if mixed_reflection_terrain {
    0x01  // reflection edge — skip shape-vector
} else {
    0x00  // eligible for shape-vector
};
```

#### R-C-03: Edge Cells Keep Original Colors

When a cell is in the SEMANTIC bucket, it uses the exact fg/bk from `resolve()`. No shape-vector, no color optimization. The colors are already correct for the semantic glyph.

#### R-C-04: Shape-Vector Uses Background Sampling (same as R-H-01)

`sample_to_lightness` uses `bg` for terrain in the Combined mode for the same reason as Harri mode.

#### R-C-05: Color Optimization for Shape-Vector Cells

When shape-vector selects a glyph for an eligible cell:
- If distance ≤ threshold AND glyph ≠ resolve_glyph: run `optimize_glyph_colors`, use optimized fg/bk
- If distance > threshold: fall back to resolve_glyph with resolve colors
- If glyph == resolve_glyph: use resolve colors (optimization unnecessary — same visual result)

This is the current behavior of `choose_final_glyph` + `choose_final_colors` in `OriginalEdges` mode, with one correction: eligibility check MUST happen before calling `select_glyph`, not inside the gate function that checks debug flags.

#### R-C-06: Remove Dependency on Debug Flags for Eligibility

The current `should_gate_shape_vector_by_cell_class` reads `debug_cell.flags` which are written alongside the resolve output. This is a correct implementation IF `resolve_with_debug` always runs. But:
- It creates a coupling between the debug system and the render correctness path
- If `resolve_with_debug` is not called (e.g., in release mode), the gate silently breaks

The eligibility bucket MUST be encoded in `AnsiCell.spare` directly from `resolve()` (R-C-02). The debug flag check in `should_gate_shape_vector_by_cell_class` should be removed in favor of reading `AnsiCell.spare`.

#### R-C-07: Structural Fallback Is a Combined-Mode Concept

The `should_preserve_resolve_glyph` logic (structural fallback) is only meaningful in Combined mode:
- If shape-vector selects a different glyph but with non-trivial distance → keep resolve glyph
- This prevents weak shape-vector matches from replacing good resolve glyphs

In Harri mode: disable this. In Original mode: not applicable. In Combined mode: keep with tuned thresholds.

#### R-C-08: Example Cell Classification Table

| Cell type | Resolve glyph | Shape-vector? | Color opt? |
|-----------|--------------|---------------|------------|
| Sky (all clear) | ` ` | No | No |
| Flat terrain, no overlays | material glyph | YES | YES if changed |
| Terrain at elevation edge | `-` or `_` (silhouette) | NO | No |
| Terrain patch center | `,` or `;` (grid linecase) | NO | No |
| Pure mesh face | VH-split glyph | YES | YES if changed |
| Mixed mesh+terrain | VH-split glyph | NO | No |
| Mesh wireframe face | `,` or `;` (wireframe linecase) | NO | No |
| Underwater (below water_z) | ripple-modulated cell | NO | No |
| Half-block split (0xDE/0xDF) | 0xDE or 0xDF | NO | No |

---

## Part 4 — Implementation Priority

### Blocker issues (affect all modes):

1. **R-O-01 Terrain grid Bresenham** — without this, the grid linecase in resolve NEVER fires
2. **R-O-03 Mesh parity bits** — without this, reflection mesh is indistinguishable from normal mesh
3. **R-H-01 / R-C-04 Sampling uses bg** — sampling using fg luma is wrong for terrain, affects all shape-vector quality
4. **R-C-02 AnsiCell.spare encodes eligibility** — without this, Combined mode eligibility depends on debug flags

### Medium-priority issues (improve fidelity/quality):

5. **R-O-02 Wireframe Bresenham** — mesh wireframe visual never appears
6. **R-O-04 Mesh water clamping** — underwater mesh incorrectly visible in reflection
7. **R-O-05 bk==fg dither fallback** — subtle difference in mixed cells
8. **R-O-06 darken_palette_index exact formula** — silhouette fg color slightly wrong
9. **R-H-02 Reference crunch exponents** — current 2.5/6.0 vs reference 3.0/7.0

### Low-priority (visual polish):

10. **R-O-08 Water ripple** — no color modulation for underwater cells
11. **R-H-05 Per-mode threshold defaults** — tuning
12. **R-H-06 Temporal coherence** — flicker reduction

---

## Part 5 — What "Combined Doesn't Look Good" Actually Is

Based on this audit, the visual problems in the combined method come from:

1. **Terrain sampling uses fg luma, not bg luma** (R-H-01 / R-C-04): Shape-vector is classifying terrain cells based on the text glyph color, not the terrain background color. The shape selection is systematically wrong for terrain, which is the majority of the visible frame.

2. **HarriPriority mode was probably being tested**: This mode skips semantic gates, replacing silhouette and linecase glyphs with shape-matched glyphs. The edges and terrain boundaries — which carry all the structural information — are destroyed.

3. **Grid linecase never fires** (R-O-01 missing): Even in OriginalEdges mode with correct gates, the grid lines are absent from the output because Bresenham never writes 0x04. The terrain appears as smooth bands with no grid character, unlike the original.

4. **Half-block split (0xDE/0xDF) is gated correctly** but the underlying auto-mat error comparison in RGB888 space vs palette space means mesh surfaces produce different splits than C++, creating subtly different patterns that the shape-vector then tries to "fix" in eligible cells.

5. **Missing bk==fg fallback**: In flat mesh areas, C++ would use a dither glyph (`,`, `:`, `%`). Rust uses 0xDF unconditionally. Shape-vector then sees a half-block when there should be a dither character, and tries to replace it, creating inconsistency at flat mesh surfaces.

The path to "looks good combined":
1. Fix R-H-01 (sample bg not fg) — immediate quality gain on all terrain
2. Implement R-C-02 (spare bit eligibility) — correct semantic gating without debug coupling
3. Implement R-O-01 (terrain grid Bresenham) — grid lines appear correctly
4. Keep OriginalEdges mode as default — HarriPriority is for experimentation only

---

*Audit source files: `(ORIGINAL GAME)asciicker-Y9-2-main/render.cpp` (C++), `engine-port/src/render/{shape_vector,resolve,resolve_bridge,pipeline,mesh_shader,terrain_shader}.rs` (Rust), `docs/worksheets/audit-reaudit-alexharri.md`, `docs/worksheets/research/alexharri-asciicker-integration.md`, `docs/worksheets/plans/2026-03-11-alexharri-vs-original-architecture-audit.md`*
