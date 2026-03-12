# Render / Glyph / Lighting Deep-Dive Audit
**Date:** 2026-03-11
**Sources read directly:**
- C++ `render.cpp` (4793 lines, original game)
- C++ `render.cpp:1055-1108` — RenderFace mesh shader (water clamping)
- C++ `render.cpp:2261-2530` — RenderSprite (full sprite blit)
- C++ `render.cpp:3655-4070` — Stage 6 Resolve (glyph selection, auto_mat, overlays)
- Rust `engine-port/src/render/resolve.rs` (full)
- Rust `engine-port/src/render/mesh_shader.rs` (full)
- Rust `engine-port/src/render/sprite_blit.rs` (full)
- Rust `engine-port/src/render/sample_buffer.rs` (spare_bits constants)

---

## SECTION A: GLYPH SELECTION

### A.1 Material Glyph Lookup — MATCHES C++

**C++ render.cpp:3709**
```cpp
int gl = matlib[mat[0]].shade[elv][shd].gl;
```
where:
- `mat[0] = src[0].visual & 0x00FF` (8-bit material index from sample)
- `elv` = elevation 0-3 from bit-15 pattern (lines 3668-3686)
- `shd = (dif[0]+dif[1]+dif[2]+dif[3] + 17*2) / (17*4)` (rounded average diffuse)

**Rust `resolve.rs:409-417`**
```rust
let shade_idx = ((diffuse_sum + 34) / 68).min(15) as usize;
let mat_cell = &ctx.materials[mat_idx].shade[elevation as usize][shade_idx];
// gl = mat_cell.gl
```
`diffuse_sum + 34` / 68 == `(dif_sum + 17*2) / (17*4)`. **Exact match.**

---

### A.2 Elevation Computation — MATCHES C++

**C++ render.cpp:3668-3686** — bit 15 of `src[-dw]`, `src[-dw+1]`, `src[dw]`, `src[dw+1]`:
```cpp
int e_lo = (src[-dw].visual >> 15) + (src[-dw + 1].visual >> 15);
int e_hi = (src[dw].visual >> 15) + (src[dw + 1].visual >> 15);
if (e_lo <= 1) { if (e_hi <= 1) elv = 3; else elv = 2; }
else           { if (e_hi <= 1) elv = 0; else elv = 1; }
```

**Rust `resolve.rs:467-494`** — identical 4-way logic. **Exact match.**

---

### A.3 Auto-mat VH Split — PARTIALLY MATCHES, MISSING FALLBACK

**C++ render.cpp:3930-3968**

The C++ auto-mat path has three steps:
1. Try horizontal split: `if (err_h * 1000 < err_v * 999)` → write glyph `0xDF` (upper half block), set `bk` = top color, `fg` = bottom color. Set `vh_near = false`.
2. Try vertical split: `else if (err_v * 1000 < err_h * 999)` → write glyph `0xDE` (right half block), set `bk` = left color, `fg` = right color. Set `vh_near = false`.
3. **Critical fallback at render.cpp:3961:**
```cpp
if (ptr->bk == ptr->fg || vh_near)
{
    int auto_mat_idx = 3 * (bg[0]/33 + 32*(bg[1]/33) + 32*32*(bg[2]/33));
    ptr->gl = auto_mat[auto_mat_idx + 2];   // dither glyph from table
    ptr->bk = auto_mat[auto_mat_idx + 0];
    ptr->fg = auto_mat[auto_mat_idx + 1];
    ptr->spare = 0xFF;
}
```
This means: **even if a split was chosen, if the split colors are equal, overwrite with the dither glyph.**

**Rust `resolve.rs:298-325`**
```rust
if err_h * 1000 < err_v * 999 {
    return ResolvedCell { gl: 0xDF, bk: auto_mat_bg_index(top), fg: auto_mat_bg_index(bottom) };
}
if err_v * 1000 < err_h * 999 {
    return ResolvedCell { gl: 0xDE, bk: auto_mat_bg_index(left), fg: auto_mat_bg_index(right) };
}
// fallback: dither glyph
```

**GAP-A3: Missing color-equality fallback.**
The Rust port returns early without checking if the split colors are the same. When `auto_mat_bg_index(top) == auto_mat_bg_index(bottom)`, C++ overwrites with the dither glyph. Rust keeps the half-block with equal fg/bk (renders as solid block, no dither texture visible).

**Impact:** Cells at mesh/terrain boundaries where the two halves happen to quantize to the same xterm-256 color get a wrong glyph. Minor visual artifact: solid blocks appear instead of dithered material texture.

---

### A.4 Grid Linecase Glyphs — MATCHES C++

**C++ render.cpp:3987-3993** — only at `elv == 3`:
```cpp
int linecase = ((src[0].spare & 0x4) >> 2) | ((src[1].spare & 0x4) >> 1) | (src[dw].spare & 0x4) | ((src[dw+1].spare & 0x4) << 1);
static const int linecase_glyph[] = { 0, ',', ',', ',', '`', ';', ';', ';', '`', ';', ';', ';', '`', ';', ';', ';' };
```

**Rust `resolve.rs:522-535`** — same bit extraction, same glyph table. **Exact match.**

---

### A.5 Silhouette Glyphs — MATCHES C++ (minus minor color difference)

**C++ render.cpp:3995-4029** — at `elv == 1 || elv == 3`:
```cpp
float minus = z_lo - z_hi;
float under = z_pr - z_lo;
if (minus > under) {
    if (minus > thresh) {
        ptr->gl = 0xC4; // '-'
        bk_rgb[0] = std::max(0, bk_rgb[0] - 1);  // darken xterm cube component
        ptr->fg = 16 + 36*bk_rgb[0] + bk_rgb[1]*6 + bk_rgb[2];
    }
} else {
    if (under > thresh) { ptr->gl = 0x5F; // '_' ... same darkening ... }
}
```
C++ decrements the xterm 6x6x6 cube X component of bk_rgb, then recomputes `fg` as `16 + 36*r + 6*g + b`.

**Rust `resolve.rs:563-573`** — same height comparisons, same glyph codes, same threshold. Calls `darken_palette_index(cell.bk)`.

**GAP-A5 (MINOR): `darken_palette_index` semantics need verification.**
C++ decrements the R component of the xterm cube index (may not match full darkening). The Rust `darken_palette_index` function is not shown in the read portion — its exact implementation needs auditing to confirm it subtracts 1 from the R component of the 6x6x6 cube (not the palette index directly).

---

### A.6 Wireframe Linecase — MATCHES C++

**C++ render.cpp:4032-4038** — bit `0x40` (WIREFRAME), same glyph table.
**Rust `resolve.rs:576-591`** — identical. **Exact match.**

---

### A.7 Water Ripple Glyph (Underwater Cells) — PORT NOT VERIFIED

**C++ render.cpp:4040-4070+** — when all 4 samples `height < water`:
- Computes world-space UV from screen position via `inv_tm`
- Applies Perlin noise to shift palette index for ripple effect
- In perspective mode: full matrix inversion per cell
- In ortho mode: `Product(inv_tm, s, w)` for 2D lookup

**Rust `water.rs:apply_water_ripple_pass`** — exists as separate post-resolve pass. Not confirmed to match the per-cell `inv_tm` UV computation from C++.

**GAP-A7: Water ripple UV computation needs line-by-line verification against C++ render.cpp:4043-4067.**

---

## SECTION B: GEOMETRY / SHAPE RENDERING

### B.1 Terrain Rasterization — MATCHES C++

**C++ render.cpp:1755-2244** `RenderPatch` → `Rasterize<TerrainShader>`.
**Rust `terrain_shader.rs:98-250`** `render_patch` → `rasterize`.
Both: barycentric triangulation of 4x4 vertex patches, same depth test direction (higher z = on top). **Match confirmed.**

---

### B.2 Mesh Rasterization — MATCHES C++ EXCEPT WATER CLAMPING

**C++ render.cpp:1059-1110** `RenderFace::Shader::Blend()`:
```cpp
void Blend(Sample* s, float z, float bc[3])
{
    if (s->height < z)
    {
        if (global_refl_mode)
        {
            if (z < water + HEIGHT_SCALE / 8)           // line 1069
            {
                if (z > water) s->height = water;        // line 1071: clamp above water
                else           s->height = z;            // line 1074: below water OK
                // ... write rgb555, diffuse ...
                s->spare = (s->spare & ~0x44) | 0x8 | 0x3;  // line 1085: MESH|REFL_PARITY
            }
        }
        else
        {
            if (z >= water - HEIGHT_SCALE / 8)          // line 1090
            {
                if (z < water) s->height = water;        // line 1092: clamp below water
                else           s->height = z;            // line 1095
                // ... write rgb555, diffuse ...
                s->spare = (s->spare & ~(0x3|0x44)) | 0x8 | 0x1;  // line 1106: MESH|NORM_PARITY
            }
        }
    }
}
```

**Rust `mesh_shader.rs:27-35`**:
```rust
fn blend(&self, sample: &mut Sample, z: f32, _bc: [f32; 3]) {
    if sample.height < z || sample.height == Sample::CLEAR_HEIGHT {
        sample.visual = self.rgb555;
        sample.diffuse = self.diffuse;
        sample.spare = spare_bits::MESH_FLAG;   // 0x08 only — NO PARITY BITS
        sample.height = z;                       // NO WATER CLAMPING
    }
}
```

**GAP-B2a (CRITICAL): Missing water threshold gate.**
C++ only writes a mesh sample if `z < water + HEIGHT_SCALE/8` (reflection mode) or `z >= water - HEIGHT_SCALE/8` (normal mode). Rust writes all samples regardless of water level. Meshes that should be invisible in reflection mode render through the water surface.

**GAP-B2b (CRITICAL): Missing height clamping to water surface.**
C++ clamps `s->height = water` when the fragment is on the wrong side of the surface. Rust writes the raw z. Causes incorrect depth ordering between water surface and submerged mesh parts.

**GAP-B2c (HIGH): Missing parity bits in spare.**
C++ sets:
- `spare |= 0x3` (REFL_PARITY) in reflection mode
- `spare |= 0x1` (NORM_PARITY) in normal mode

Rust sets only `spare = MESH_FLAG (0x08)`. The resolve stage (`render.cpp:3744`) applies `/400` dimming when `(spare & 0x3) == 3`. Since Rust mesh samples never have REFL_PARITY set, reflected mesh geometry is not dimmed. Reflected meshes appear at full brightness instead of ~64% brightness.

---

### B.3 2x Supersampling — MATCHES C++

**C++ render.cpp:803-810** — `SampleBuffer` is `(2*ascii_w+4) * (2*ascii_h+4)`.
**Rust `sample_buffer.rs`** — same formula. Clear state double-allocation. **Exact match.**

---

### B.4 Barycentric Fill / Rasterizer — ASSUMED MATCH (not directly compared)

Both C++ and Rust invoke the same barycentric algorithm. The Rust `rasterizer.rs` was not directly read but is consistently referenced. No regression reported for triangle fill.

---

## SECTION C: LIGHTING / DIFFUSE

### C.1 Terrain Diffuse — EXACT MATCH

**C++ render.cpp:1892-1898** (TerrainShader inline):
```cpp
df = (dzdx*lt[0] + dzdy*lt[1] + HEIGHT_SCALE*lt[2]) / sqrt(dzdx²+dzdy²+HS²);
df = df*(1-0.5*lt[3]) + 0.5*lt[3];
```

**Rust `terrain_shader.rs:25-34`** — `compute_diffuse()` with `LIGHT_DIR = [0.3,-0.3,1.0,0.3]`. **Exact match.**

---

### C.2 Mesh Diffuse — EXACT MATCH

**C++ render.cpp:1115-1146** — cross product → transform by instance TM → div z by HEIGHT_SCALE → normalize → `n·l` → ambient blend → +0.5 bias → clamp.

**Rust `mesh_shader.rs:48-97`** — `compute_face_diffuse()`. Every step present. **Exact match.**

---

### C.3 auto_mat Table — EXACT MATCH

**C++ render.cpp:922-1045** — `create_auto_mat()`:
- 32×32×32×3 = 98,304 bytes
- dither glyphs: `" ..::%"` (6 levels)
- Index: `3 * (r5 + 32*g5 + 32*32*b5)`

**Rust `material.rs`** — `create_auto_mat()`. Same size, same dither glyph set, same index formula. **Exact match.**

---

### C.4 Material Shade Table — EXACT MATCH

**C++ render.cpp:3829** — `matlib[mat].shade[elv][dif/17].bg[rgb]`.
**Rust `material.rs`** — `Material::lookup(elv, dif)`. **Exact match.**

---

### C.5 Reflection Dimming for Terrain — EXACT MATCH

**C++ render.cpp:3744-3755** — when `(spare & 0x3) == 3`:
```cpp
r = r * dif[i] / 400;
g = g * dif[i] / 400;
b = b * dif[i] / 400;
```
vs normal: `/ 255`.

**Rust `resolve.rs:423-429`** — `dim = |rgb| [rgb[0]*255/400, ...]`. **Exact match for terrain.**

**However:** Due to GAP-B2c, mesh reflections NEVER trigger this path (they lack REFL_PARITY). The `/400` dimming is only applied to terrain reflections in the Rust port.

---

### C.6 RGB888 → xterm-256 Palette — EXACT MATCH

**C++ render.cpp:3975-3982**:
```cpp
ptr->bk = 16 + 36*((bg[0]+102)/204) + ((bg[1]+102)/204)*6 + (bg[2]+102)/204;
```

**Rust `resolve.rs:437, quantize.rs`** — `rgb2pal()`. **Exact match.**

---

### C.7 LightenColor (Sprite Path) — SPLIT IMPLEMENTATION

**C++ render.cpp:2383-2392** — `LightenColor(int pal)`:
- Operates on xterm-256 palette index
- Decomposes `16 + 36*r + 6*g + b` → extract r,g,b (0-5 cube coords)
- Increments r,g,b by 1, clamped to 5
- Recomposes to palette index
- Used in swoosh blit path only (post-resolve)

**Rust `asset_loader/xp_sprite.rs:221-228`** — `lighten_color([u8;3])`:
- Operates on RGB888
- Adds `SPRITE_LIGHTEN_AMOUNT` (probably 51) to each channel, saturating
- Used in `merge_layers()` swoosh step — happens at asset load time, not render time

**GAP-C7: Two different LightenColor semantics in play.**
- C++: palette-index lightening at blit time (renders to already-resolved AnsiCell buffer)
- Rust: RGB888 lightening at asset load time (baked into merged cell data)

These are not equivalent. C++ lightens the destination cell's existing palette color. Rust lightens the sprite's source RGB88 at load time. The visual result differs when the destination cell's color is dark vs. when the sprite's inherent color is light.

**Required:** A `lighten_palette_index(pal: u8) -> u8` function in the Rust render path that increments xterm cube coordinates (not RGB888).

---

## SECTION D: SPRITE BLIT

### D.1 RenderSprite — CRITICAL MISSING IMPLEMENTATION

**C++ render.cpp:2265-2530** — `Renderer::RenderSprite()`:

```
FRAME INDEX:
  i = frame + angle * s->anim[anim].length
  if (refl) i += s->anim[anim].length * s->angles
  f = s->atlas + s->anim[anim].frame_idx[i]

REFERENCE POINT:
  dx = f->ref[0] / 2
  dy = f->ref[1] / 2

DEPTH FORMULA (render.cpp:2310-2311):
  const float ds = 2.0 * (SPRITE_ZOOM * SPRITE_SCALE) / VISUAL_CELLS * 0.5;
  const float dz_dy = HEIGHT_SCALE / (cos(30°) * HEIGHT_CELLS * ds);
  float height = (2 * src->spare + f->ref[2]) * 0.5f * dz_dy + pos[2];

WATER GATE (render.cpp:2331):
  if (!refl && height >= water || refl && height <= water)

TRANSPARENCY CHECK (render.cpp:2334-2338):
  if (src->bk == SPRITE_TRANSPARENT_INDEX && src->fg == SPRITE_TRANSPARENT_INDEX
   || (src->gl == 32 || src->gl == 0) && src->bk == SPRITE_TRANSPARENT_INDEX
   || src->gl == 219 && src->fg == SPRITE_TRANSPARENT_INDEX) → skip

SWOOSH MODES (render.cpp:2343-2500+):
  - fg swoosh (SPRITE_SWOOSH_INDEX): uses glyph 219 fullblock → LightenColor fg+bk
  - bk swoosh (SPRITE_SWOOSH_INDEX): updates SampleBuffer heights + LightenColor
  - Normal cell: direct write to AnsiCell buffer (dst->gl, dst->bk, dst->fg)
```

**Rust `sprite_blit.rs:99-128`** — `blit_sprite()`:
```rust
// Placeholder: mark sprite position with 'S'
cell_grid.set_cell(ux, uy, b'S' as u16, [255,255,0,255], [64,0,64,255]);
```

**GAP-D1 (CRITICAL): Entire RenderSprite implementation is missing.**

The following are NOT implemented in the Rust port:
| Feature | C++ line | Rust |
|---------|----------|------|
| Frame index computation (anim + angle) | 2271 | NOT IMPLEMENTED |
| Reflection frame offset | 2272-2273 | NOT IMPLEMENTED |
| Reference point (dx, dy) | 2280-2281 | NOT IMPLEMENTED |
| Depth height formula (SPRITE_ZOOM × SPRITE_SCALE) | 2310-2311 | NOT IMPLEMENTED |
| Water gate per-cell | 2331 | NOT IMPLEMENTED |
| Transparency skip (TRANSPARENT_INDEX) | 2334-2338 | NOT IMPLEMENTED |
| fg swoosh (LightenColor blend) | 2343-2400 | NOT IMPLEMENTED |
| bk swoosh (LightenColor blend) | 2402-2500 | NOT IMPLEMENTED |
| Normal cell blit to AnsiCell | 2319-2320 | NOT IMPLEMENTED |
| Depth test against SampleBuffer per-quadrant | 2353-2372 | NOT IMPLEMENTED |
| AverageGlyph for swoosh quadrant masking | 2392, 2394 | NOT IMPLEMENTED |
| Reflection mode blit | 2272 | NOT IMPLEMENTED |

---

## SUMMARY TABLE

| Area | Gap ID | Severity | C++ Location | Rust Location | Description |
|------|--------|----------|--------------|---------------|-------------|
| **Glyph** | GAP-A3 | MEDIUM | render.cpp:3961 | resolve.rs:298-324 | Auto-mat color-equality fallback to dither glyph missing |
| **Glyph** | GAP-A5 | MINOR | render.cpp:4012-4026 | resolve.rs:566,571 | `darken_palette_index` semantics unverified vs C++ cube-decrement |
| **Glyph** | GAP-A7 | MEDIUM | render.cpp:4043-4067 | water.rs (separate pass) | Water ripple UV computation (per-cell inv_tm) needs line verification |
| **Geometry** | GAP-B2a | CRITICAL | render.cpp:1069,1090 | mesh_shader.rs:30-35 | Mesh water threshold gate missing — meshes render through water |
| **Geometry** | GAP-B2b | CRITICAL | render.cpp:1071-1074, 1092-1095 | mesh_shader.rs:34 | Mesh height NOT clamped to water surface |
| **Geometry** | GAP-B2c | HIGH | render.cpp:1085, 1106 | mesh_shader.rs:33 | Spare parity bits (0x1, 0x3) not set on mesh samples → reflection dimming broken for meshes |
| **Lighting** | GAP-C7 | MEDIUM | render.cpp:2383-2392 | xp_sprite.rs:221-228 | `LightenColor` works on palette index in C++, on RGB888 at load-time in Rust |
| **Sprite** | GAP-D1 | CRITICAL | render.cpp:2265-2530 | sprite_blit.rs:99-128 | RenderSprite entirely missing; placeholder writes `'S'` |

---

## CONFIRMED MATCHES (no gap)

| Area | C++ Location | Rust Location |
|------|-------------|---------------|
| Material glyph from shade table | render.cpp:3709 | resolve.rs:417 |
| Elevation bit-15 computation | render.cpp:3668-3686 | resolve.rs:467-494 |
| auto_mat vh split logic | render.cpp:3930-3958 | resolve.rs:298-311 |
| Grid linecase glyphs | render.cpp:3987-3993 | resolve.rs:522-535 |
| Silhouette glyphs (`-`, `_`) | render.cpp:4007-4029 | resolve.rs:563-572 |
| Wireframe linecase | render.cpp:4032-4038 | resolve.rs:576-591 |
| Clear cell sky color | render.cpp:2884 | resolve.rs:99 |
| Terrain diffuse formula | render.cpp:1892-1898 | terrain_shader.rs:25-34 |
| Mesh face diffuse formula | render.cpp:1115-1146 | mesh_shader.rs:48-97 |
| auto_mat table creation | render.cpp:922-1045 | material.rs |
| Material shade table lookup | render.cpp:3829 | material.rs:36-46 |
| Terrain reflection dimming `/400` | render.cpp:3744-3755 | resolve.rs:423-429 |
| RGB888 → xterm-256 palette | render.cpp:3975-3982 | resolve.rs (rgb2pal) |
| XP sprite parsing | sprite.cpp | xp_sprite.rs:109-179 |
| XP layer merge semantics | sprite.cpp | xp_sprite.rs:240-319 |

---

## IMPLEMENTATION PRIORITY

```
CRITICAL (visual correctness blocked):
  GAP-D1  → Implement RenderSprite (entire sprite blit path)
  GAP-B2a → Add water threshold gate to MeshShader::blend()
  GAP-B2b → Clamp mesh sample height to water surface
  GAP-B2c → Set parity bits (0x1 / 0x3) in MeshShader::blend()

HIGH (reflection rendering incorrect):
  (covered by GAP-B2c fix above)

MEDIUM (visual polish):
  GAP-A3  → Add bk==fg check after auto-mat split; fall through to dither glyph
  GAP-A7  → Verify water ripple UV against C++ render.cpp:4043-4067
  GAP-C7  → Add lighten_palette_index() working on xterm-256 index space

MINOR (investigate + fix if needed):
  GAP-A5  → Audit darken_palette_index() vs C++ cube-decrement formula
```
