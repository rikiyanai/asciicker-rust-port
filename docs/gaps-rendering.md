> **STATUS: ACTIVE GAP ANALYSIS** — Generated 2026-02-20. Plan generated: plan-rendering-gaps.md. CORRECTION at line ~265: triangle area rejection condition appears inverted.

# Asciicker Rendering System - GAP ANALYSIS

This document identifies areas NOT covered in the existing research documents when comparing against the actual C++ source code in `/Users/r/Downloads/asciicker-Y9-2/render.cpp`.

---

## 1. RENDERING STAGES MISSED

### 1.1 Clear Stage - Cached Buffer Optimization
**NOT DOCUMENTED:** The clear stage uses a cached "clean" buffer optimization where the upper half of the SampleBuffer allocation stores a pre-rendered background that gets memcpy'd instead of per-element memset.

**Source (render.cpp:591-595):**
```cpp
// Upper half of the allocation is a cached "clean" state
// that gets memcpy'd each frame instead of a slower per-element memset.
```

### 1.2 Debug Breakpoint Feature
**NOT DOCUMENTED:** There is a debug breakpoint system via `render_break_point[2]` global that allows coordinate-based breakpointing in the resolve pass.

**Source (render.cpp:3425-3429):**
```cpp
if (x == render_break_point[0] && y == render_break_point[1])
{
    render_break_point[0] = -1;
    render_break_point[1] = -1;
}
```

---

## 2. SHADER/RASTERIZATION FEATURES NOT DOCUMENTED

### 2.1 Edge Function Mathematical Derivation
**NOT DOCUMENTED:** The comments in Rasterize contain a detailed mathematical derivation of the edge function algorithm that was not captured.

**Source (render.cpp:414-421):**
```cpp
// EDGE FUNCTION MATH DERIVATION:
// The edge function for edge (a->b) evaluated at point c is:
//   e(a,b,c) = (b.x - a.x)*(c.y - a.y) - (b.y - a.y)*(c.x - a.x)
// This computes the signed area of the parallelogram formed by vectors
// (a->b) and (a->c). The sign tells us which side of edge (a->b) point c
// lies on.
```

### 2.2 BC_P Pixel Center Sampling
**NOT DOCUMENTED:** The BC_P macro samples at the CENTER of cells (2*c+1 terms) rather than corners to avoid sampling bias.

**Source (render.cpp:426-431):**
```cpp
// BC_P evaluates the edge function at the CENTER of cell c (hence the
// 2*c+1 terms) rather than at its corner. This avoids sampling bias that
// would cause triangles sharing an edge to either double-draw or miss
// the boundary pixel.
```

### 2.3 Edge Pairing for Shared Triangle Boundaries
**NOT DOCUMENTED:** When bc[i]==0 (pixel lies exactly ON an edge), an x-coordinate comparison ensures exactly one triangle owns the boundary.

**Source (render.cpp:474-483):**
```cpp
// WHY edge pairing: when bc[i]==0, the pixel lies exactly ON an
// edge. Without this tie-breaking rule, adjacent triangles sharing
// that edge would both claim the pixel (double-draw). The x-coord
// comparison ensures exactly one triangle "owns" each shared edge.
if (bc[0] == 0 && v[1][0] <= v[2][0] ||
    bc[1] == 0 && v[2][0] <= v[0][0] ||
    bc[2] == 0 && v[0][0] <= v[1][0])
{
    continue;
}
```

### 2.4 Double-Sided Rendering Logic
**NOT DOCUMENTED:** The Rasterize function handles both CCW (area > 0) and CW (area < 0) triangles when dblsided is true, with different edge function sign tests.

**Source (render.cpp:501-554):**
```cpp
else
if (area < 0 && dblsided)
{
    // Same algorithm but with inverted sign tests: bc[0] > 0 || bc[1] > 0 || bc[2] > 0
}
```

### 2.5 Wireframe Rendering via Visual Flag
**NOT DOCUMENTED:** Meshes can request wireframe-only rendering by setting bit 31 in the visual flag.

**Source (render.cpp:1060, 220):**
```cpp
// In RenderFace: if (visual & (1<<31)) draw only edge outline
// Part 1 doc mentions this but no details on implementation
```

---

## 3. COLOR SPACE AND FORMAT DETAILS MISSING

### 3.1 Sample Spare Bit Flags - Complete Set
**NOT FULLY DOCUMENTED:** The existing docs mention bits 2 (grid line) and 3 (mesh), but miss bit 6 (wireframe).

**Source (render.cpp:565-566):**
```cpp
// bit 0-1: parity (0=empty, 1=odd, 2=even, 3=reflection),
// bit 2: grid line hit, bit 3: mesh/auto-material flag, bit 6: wireframe hit.
```

### 3.2 RGB555 to RGB888 Conversion Formula
**NOT DOCUMENTED:** The exact formula for expanding RGB555 to RGB888.

**Source (render.cpp:3528-3530, 865-869):**
```cpp
// For visual (RGB555):
int r = ((vis[i] & 0x1F) * 527 + 23) >> 6;
int g = (((vis[i] >> 5) & 0x1F) * 527 + 23) >> 6;
int b = (((vis[i] >> 10) & 0x1F) * 527 + 23) >> 6;

// Alternative formula in shader:
int r5 = (r8 * 249 + 1014) >> 11;  // 888 to 555
```

### 3.3 Glyph Coverage for Alpha Blending
**NOT DOCUMENTED:** How glyph_coverage[256] provides 4-quadrant coverage for half-transparent sprite compositing.

**Source (sprite.cpp:1944-1948):**
```cpp
// Each CP437 glyph has a precomputed 4-quadrant coverage in glyph_coverage[256]:
//   bits 0-3: bottom-left, bits 4-7: bottom-right, bits 8-11: top-left, bits 12-15: top-right
// The mask selects which quadrants to consider (bit 1=BL, 2=BR, 4=TL, 8=TR).
```

### 3.4 LightenColor and DarkenGlyph Functions
**NOT DOCUMENTED:** These color manipulation functions used in sprite compositing.

**Source (sprite.cpp:1903-1942):**
```cpp
// LightenColor: increments each RGB component by 1 (max 5)
int LightenColor(int c)

// DarkenGlyph: subtracts 2 from each RGB component  
int DarkenGlyph(const AnsiCell* ptr)
```

### 3.5 AverageGlyphTransp Function
**NOT DOCUMENTED:** A variant of AverageGlyph that preserves transparency instead of falling back.

**Source (sprite.cpp:1991-1999):**
```cpp
// Same as AverageGlyph but does NOT fall back to the other color when
// the result is transparent (255). Used during swoosh merging where
// transparency needs to be preserved.
```

---

## 4. PERFORMANCE OPTIMIZATIONS NOT CAPTURED

### 4.1 DBL Compile Flag
**NOT DOCUMENTED:** This flag controls whether the sample buffer uses 2x supersampling. When disabled, sprites will have incorrect depth.

**Source (render.cpp:88, 2067):**
```cpp
#define DBL

// TODO: or the DBL define is removed, these divisions produce wrong offsets.
// ref[0]/2 and ref[1]/2 assume 2x supersampling
```

### 4.2 DARK_TERRAIN Compile Flag
**NOT DOCUMENTED:** Optional compile flag for terrain darkness support.

**Source (render.cpp:1608, 1694, 1890):**
```cpp
#ifdef DARK_TERRAIN
    // terrain darkness handling
#endif
```

### 4.3 Lazy Allocation Strategy
**NOT DOCUMENTED:** The sample_buffer and sprites_alloc are allocated on-demand in Render() rather than at Renderer creation, allowing viewport resize support.

**Source (render.cpp:35, 39):**
```cpp
// Allocated sample_buffer and sprites_alloc on first call or resize (lazy init).
// The Renderer doesn't know the viewport dimensions until the first Render() call.
```

### 4.4 Integer Alignment for DBL Mode
**NOT DOCUMENTED:** When DBL is enabled and int_flag is set, coordinates are rounded to even numbers.

**Source (render.cpp:2997-3007):**
```cpp
// if yaw didn't change, make it INTEGRAL (and EVEN in case of DBL)
#ifdef DBL
    x &= ~1;
    y &= ~1;
#endif
```

### 4.5 Precomputed Perspective Unprojection Coefficients
**NOT DOCUMENTED:** Perspective unprojection uses precomputed coefficients (wx_*, wy_*, ww_*) that encode frustum geometry.

**Source (render.cpp:3399-3404):**
```cpp
// The XY coefficients (wx_*, wy_*) encode the perspective frustum geometry;
// any change to the projection (zoom, focal length, view distance) requires recalculation.
```

---

## 5. EDGE CASES NOT DOCUMENTED

### 5.1 TODO: Missing Animation Frame Validation
**NOT DOCUMENTED:** No validation of anim/frame/angle bounds - out-of-range reads possible if .xp asset is mis-exported.

**Source (render.cpp:2055-2057):**
```cpp
// TODO(PIPELINE-FIX): No validation that anim/frame/angle are within
// bounds. Out-of-range reads possible if .xp asset is mis-exported.
```

### 5.2 TODO: Sprite Depth Assumes 2x Supersampling
**NOT DOCUMENTED:** Sprite depth calculation assumes DBL mode.

**Source (render.cpp:2065-2067):**
```cpp
// TODO(PIPELINE-FIX): ref[0]/2 and ref[1]/2 assume .xp sprite cells map
// to 2x supersampled buffer. If DBL define removed, sprite depth is wrong.
```

### 5.3 TODO: Sprite Sort Assumes Correct Distance
**NOT DOCUMENTED:** Far-to-near sort assumes all sprites have correct distance values.

**Source (render.cpp:4075-4076):**
```cpp
// TODO(PIPELINE-FIX): Far-to-near sort assumes all sprites have correct
// dist values. Invalid distances could cause incorrect draw order.
```

### 5.4 Water Plane Tolerance Values
**NOT DOCUMENTED:** Specific HEIGHT_SCALE tolerances for water plane handling.

**Source (render.cpp:857, 878, 1553, 1581):**
```cpp
// Water plane clipping: HEIGHT_SCALE/8 tolerance
if (z < water + HEIGHT_SCALE / 8)
if (z >= water - HEIGHT_SCALE / 8)
```

### 5.5 Z-Fighting Tolerance
**NOT DOCUMENTED:** HEIGHT_SCALE/2 is used to avoid Z-fighting on coplanar surfaces.

**Source (render.cpp:587, 418):**
```cpp
// DepthTest_RO adds HEIGHT_SCALE/2 tolerance to avoid Z-fighting
return height <= z + HEIGHT_SCALE/2;
```

### 5.6 Triangle Area Rejection Threshold
**NOT DOCUMENTED:** Triangles smaller than 0x10000 (16.16 fixed-point) in area are rejected.

**Source (render.cpp:443, 504):**
```cpp
if (area >= 0x10000)
    return; // degenerate triangle rejection
```

> **CORRECTION:** This condition as written rejects LARGE triangles and accepts degenerate ones.
> The actual C++ behavior needs verification from source. Degenerate triangles have area near zero,
> so the rejection threshold should reject area BELOW a minimum, not above.

### 5.7 Reflection/Regular Terrain Mixing Triggers Auto-Mat
**NOT DOCUMENTED:** When a cell contains both reflected and non-reflected terrain samples, auto_mat is forced.

**Source (render.cpp:3512-3518):**
```cpp
// if cell contains both refl and non-refl terrain enable auto-mat
bool has_refl = (spr[0] & 3) == 3 || ...;
bool has_norm = (spr[0] & 3) == 1 || ...;
if (has_refl && has_norm)
{
    use_auto_mat = true;
}
```

### 5.8 Silhouette Edge Detection
**NOT DOCUMENTED:** The resolve pass detects silhouette edges based on depth differences between sample rows.

**Source (render.cpp:3783-3817):**
```cpp
// silhouette repetitoire:  _-/\| 
// Detected by comparing z_hi (bottom row), z_lo (current row), z_pr (previous row)
float minus = z_lo - z_hi;
float under = z_pr - z_lo;
```

### 5.9 Water Ripple Animation
**NOT DOCUMENTED:** Water ripples use Perlin noise with animated wave offsets.

**Source (render.cpp:2957, 3860-3867):**
```cpp
// Animate water with sine wave
water += HEIGHT_SCALE * 5 * sinf(frame*M_PI*0.01);

// Perlin noise for ripples
double d = r->pn.octaveNoise0_1(w[0] * 0.05, w[1] * 0.05, r->pn_time, 4);
```

### 5.10 Player Shadow Distance Attenuation
**NOT DOCUMENTED:** The player blob shadow uses distance-based attenuation within a 2.0 unit radius.

**Source (render.cpp:3232-3260):**
```cpp
if (sq_xy <= 2.00)  // 2.0 world unit radius
{
    int dz = (int)(2*(pos[2] - s->height) + 2*sq_xy);
    if (dz < 180) dz = 180;
    if (dz > 180) dz = 255;
    // attenuate diffuse
}
```

---

## 6. MATERIAL SYSTEM DETAILS MISSING

### 6.1 TODO: Animated Materials Feature
**NOT DOCUMENTED:** Comment indicates planned support for animated materials with time-based shade shifting.

**Source (render.cpp:3452-3454):**
```cpp
// TODO:
// every material must have 16x16 map and uses visual shade to select Y and lighting to select X
// animated materials additionally pre shifts and wraps visual shade by current time scaled by material's 'speed'
```

### 6.2 Material Library Shading Structure
**NOT DOCUMENTED:** The material library uses a shade[4][17] structure for elevation (0-3) and lighting (0-16) levels.

**Source (render.cpp:3497, 3493):**
```cpp
int elv;  // elevation level 0-3
int shd = (dif[0] + dif[1] + dif[2] + dif[3] + 17 * 2) / (17 * 4); // 0-16
int gl = matlib[mat[0]].shade[elv][shd].gl;
```

---

## 7. COORDINATE SYSTEM DETAILS

### 7.1 SampleBuffer Border
**NOT DOCUMENTED:** The SampleBuffer has a +4 border (1 sample on each side) to avoid bounds checks during 2x2 neighbor access.

**Source (render.cpp:28-30, 3420):**
```cpp
// SampleBuffer is (2*width+4) x (2*height+4)
// The +4 provides a 1-sample border on each side
Sample* src = r->sample_buffer.ptr + 2 + 2 * dw;
```

### 7.2 View Matrix Construction Details
**NOT DOCUMENTED:** The isometric view uses specific angles (30 degrees) and construction from yaw, position, and zoom.

**Source (render.cpp:2903-2970):**
```cpp
// cos30 = 0.866025 (cosine of 30 degrees)
// sin30 = 0.5 (sine of 30 degrees)
// ds = zoom / focal_length
```

---

## SUMMARY

| Category | Gaps Found |
|----------|-----------|
| Rendering Stages | 2 (cached buffer, debug breakpoint) |
| Shader/Rasterization | 5 (edge math, BC_P, edge pairing, double-sided, wireframe) |
| Color/Format | 5 (spare bits, RGB555 formula, glyph coverage, color functions) |
| Performance | 5 (DBL, DARK_TERRAIN, lazy alloc, alignment, precomputed coeffs) |
| Edge Cases | 10 (validation TODOs, tolerances, rejection, mixing, silhouettes, ripples, shadow) |
| Material System | 2 (animated materials, shade structure) |
| Coordinate System | 2 (border, view matrix) |

**Total Unique Gap Items: 31**

---

## RECOMMENDATIONS

1. **Update research-rendering-deep-dive.md** with the missing Sample spare bit flags and color conversion formulas
2. **Add new section** on compile-time flags (DBL, DARK_TERRAIN) and their effects
3. **Document the edge cases** from TODO comments as known limitations
4. **Create technical note** on the mathematical derivation of the rasterizer for future maintainers
5. **Add performance section** covering lazy allocation, cached buffers, and precomputation strategies
