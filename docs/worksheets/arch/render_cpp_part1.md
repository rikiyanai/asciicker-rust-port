# render.cpp Part 1: Functions (lines 1-2289)

## Overview

This document analyzes all function definitions in `render.cpp` spanning lines 1-2289. The file implements a CPU software rasterizer with a 6-stage rendering pipeline that transforms 3D voxel/polygon worlds into terminal ASCII grids.

---

### `Bresenham` (render.cpp:111-184)

**Signature:**
```cpp
template <typename Sample>
inline void Bresenham(Sample* buf, int w, int h, int from[3], int to[3], int _or)
```

**Purpose:**
Rasterizes a line segment in sample-buffer space using Bresenham's algorithm. Writes a bit flag (e.g., 0x04 for grid lines) into `Sample::spare` without changing color or depth, allowing the resolve pass to choose overlay glyphs at intersections.

**Called by:**
- `RenderPatch` (line 2041-2042, for grid line rendering)
- `RenderFace` (line 1060, for wireframe edges when visual flag has bit 31 set)

**Calls:**
- `Sample::DepthTest_RO()` (depth testing at each rasterized position)

**Globals read:**
None

**Globals mutated:**
- `buf` (sample buffer array, writes to `spare` field)

**Side effects:**
Modifies sample buffer depth and spare bit flags during line drawing.

**Notes:**
- Uses step-by-2 in horizontal mode due to 2x supersampling.
- Edge cases: exits early if from and to are identical.
- Performs half-coordinate rounding at start to avoid out-of-domain samples.

---

### `PerspectiveCorrectCellLine` (render.cpp:192-286)

**Signature:**
```cpp
template <typename Sample>
inline void PerspectiveCorrectCellLine(Sample* smp, AnsiCell* buf, int w, int h, int from[3], int to[3], float d_from, float d_to, int gl, int fg)
```

**Purpose:**
Draws a line at AnsiCell resolution (not sample resolution) with perspective-correct depth testing against the 2x supersampled SampleBuffer. Used for sprites and projectiles where 1/w interpolation corrects for perspective foreshortening.

**Called by:**
No callers found via grep

**Calls:**
- `Sample::DepthTest_RO()` (depth testing)
- `AverageGlyph()` (color averaging)
- `LightenColor()` (color brightening, called twice)

**Globals read:**
None

**Globals mutated:**
- `buf` (AnsiCell output buffer, modifies bk, fg, gl fields)

**Side effects:**
Modifies AnsiCell buffer with averaged colors and glyph along the projected line.

**Notes:**
- `ka` value implements perspective correction: `ka = ka / ((1-t)*d_from + t*d_to)`
- Separates horizontal and vertical domain processing for efficiency.
- Uses 1/w interpolation to avoid visual stretching on angled surfaces.

---

### `CellLine` (render.cpp:291-379)

**Signature:**
```cpp
template <typename Sample>
inline void CellLine(Sample* smp, AnsiCell* buf, int w, int h, int from[3], int to[3], int gl, int fg)
```

**Purpose:**
Non-perspective variant of line drawing at AnsiCell resolution. Used for orthographic projection where linear depth interpolation in screen space is sufficient.

**Called by:**
No callers found via grep

**Calls:**
- `Sample::DepthTest_RO()` (depth testing)
- `AverageGlyph()` (color averaging)
- `LightenColor()` (color brightening, called twice)

**Globals read:**
None

**Globals mutated:**
- `buf` (AnsiCell output buffer, modifies bk, fg, gl fields)

**Side effects:**
Modifies AnsiCell buffer with averaged colors and glyph along the line.

**Notes:**
- Simpler than `PerspectiveCorrectCellLine` due to no 1/w correction.
- Reuses same Bresenham stepping logic as perspective variant, minus correction.
- Horizontal/vertical domain separation matches `PerspectiveCorrectCellLine` structure.

---

### `Rasterize` (render.cpp:404-557)

**Signature:**
```cpp
template <typename Sample, typename Shader>
inline void Rasterize(Sample* buf, int w, int h, Shader* s, const int* v[3], bool dblsided)
```

**Purpose:**
Barycentric triangle rasterizer using compile-time duck typing. For each pixel inside a triangle's bounding box, computes 3 edge functions for inside/outside testing. Normalizes to [0,1] barycentric weights and calls `Shader::Blend()` with interpolated depth and attributes.

**Called by:**
- `RenderFace` (line 1158, 1166, for mesh triangle rasterization)
- `RenderPatch` (line 1940, 1948, 1963, 1971, 1988, 1996, 2012, 2020, for terrain patch triangles)

**Calls:**
- `Shader::Blend()` (per-pixel shader callback with barycentric weights)
- `BC_A()` (macro, computes signed area)
- `BC_P()` (macro, computes edge function at pixel center)

**Globals read:**
- `FLT_EPSILON` (C standard library constant)

**Globals mutated:**
- `buf` (sample buffer array)

**Side effects:**
Rasterizes triangle geometry into sample buffer, updating depth and visual/diffuse/spare fields.

**Notes:**
- Edge function tie-breaking: when bc[i]==0 (pixel on edge), x-coordinate comparison ensures exactly one triangle owns the boundary.
- Supports back-face culling via `dblsided` flag (if false, skips CCW/CW check).
- Area threshold of 0x10000 triggers early rejection for degenerate triangles.

---

### `create_auto_mat` (render.cpp:710-840)

**Signature:**
```cpp
static int create_auto_mat(uint8_t mat[])
```

**Purpose:**
Precomputes a 32K-entry lookup table mapping RGB555 colors to xterm 256-color palette {bg, fg, glyph} triples. Called once at static init time to prepare `auto_mat` for mesh color quantization in the resolve pass.

**Called by:**
- Global static initializer (line 709, `int auto_mat_result = create_auto_mat(auto_mat)`)

**Calls:**
- `std::min()` (standard library function)
- `floorf()` (math library, line 811)
- `sqrtf()` (math library, line 798)

**Globals read:**
- `MCV` (macro, defined as 5)
- `glyph` (static local array, dither characters)

**Globals mutated:**
- `mat` (output parameter, 32K-byte lookup table)

**Side effects:**
Populates the `mat` array with quantized color mappings.

**Notes:**
- Algorithm: for each RGB555 cell, finds best pair of xterm cube vertices and computes projection distance.
- Dither glyphs " ..::%": progressive from empty to full block to represent shading.
- Clamped shading value (0-11) selects dither glyph; split between bg/fg based on threshold (shd < 6).
- `MCV = 5` defines cube subdivisions per RGB channel.

---

### `Renderer::RenderFace` (render.cpp:847-1168)

**Signature:**
```cpp
void Renderer::RenderFace(float coords[9], uint8_t colors[12], uint32_t visual, void* cookie)
```

**Purpose:**
Callback for `QueryMesh` that receives a single triangle face. Transforms 3 vertices from model space through view*instance matrix, computes per-face diffuse lighting from surface normal, then rasterizes via `Rasterize<>` with inline shader writing RGB555+diffuse to SampleBuffer (flagged 0x8 in spare for mesh).

**Called by:**
- `RenderMesh` (line 1519, via `QueryMesh()`)

**Calls:**
- `Product()` (matrix operations, lines 966, 1013, 1066, 1128)
- `DotProduct()` (vector operations, line 980, 1027, 1080)
- `Rasterize()` (template rasterizer, line 1158, 1166)
- `Bresenham()` (for wireframe edges, line 1060)
- `floorf()` (math library, multiple lines)

**Globals read:**
- `global_refl_mode` (reflection rendering mode)
- `water` (water plane height, accessed via `r->water`)

**Globals mutated:**
- `r->sample_buffer.ptr` (sample buffer array)

**Side effects:**
Writes RGB555 mesh geometry and diffuse lighting into SampleBuffer; marks with spare|0x8 for auto-material quantization.

**Notes:**
- Normal is computed per-face via edge cross product (lines 1121-1125).
- Perspective projection supported: uses `viewer_dist` and focal correction if `r->perspective=true`.
- Water plane clipping: clamped to water height with HEIGHT_SCALE/8 tolerance.
- Reflection mode inverts Z in view matrix and flips vertex order (lines 1150-1152).
- Wireframe flag (visual & (1<<31)) draws only edge outline (line 1060).

---

### `Renderer::RenderSprite` (render.cpp:1175-1503)

**Signature:**
```cpp
void Renderer::RenderSprite(Inst* inst, Sprite* s, float pos[3], float yaw, int anim, int frame, int reps[4], void* cookie)
```

**Purpose:**
Queues a sprite for deferred rendering. Checks if sprite is item/character, updates "nearby items" lists, projects world position to screen space, and appends to `sprites_alloc` for later sorting and blitting. Handles both gameplay logic (item pickup proximity) and rendering setup.

**Called by:**
- `QueryWorld` callback (called during world query phase)

**Calls:**
- `GetInstSpriteData()` (character retrieval)
- `AnimateSpriteInst()` (animation frame lookup, line 1491)
- `DotProduct()` (distance calculation)
- `Product()` (matrix transform, line 971)
- `realloc()` (sprite buffer expansion, line 1321)
- `floorf()` (coordinate rounding)
- `floor()` (coordinate rounding)

**Globals read:**
- `global_refl_mode` (reflection mode)
- `player_sprite`, `attack_sprite`, `inventory_sprite` (sprite references)

**Globals mutated:**
- `r->sprites_alloc` (sprite render queue)
- `r->sprites` (sprite count)
- `r->items`, `r->item_sort[]`, `r->item_dist[]` (nearby items list)
- `r->npcs`, `r->npc_sort[]`, `r->npc_dist[]` (nearby NPCs list)

**Side effects:**
Expands sprite render queue; updates item/NPC proximity lists for gameplay (e.g., item pickup UI).

**Notes:**
- Deferred approach: sprites are queued, not rasterized immediately; sorted far-to-near for correct painter's algorithm compositing.
- Item handling: items flagged with `anim < 0`; extracted from `reps` parameter as `Item*`.
- Dead character loot: dead NPCs populate item list from their inventory.
- Perspective fade: sprites scaled/faded when viewer distance approaches bounds (lines 1346-1361).
- Reflection exclusion: sprites with `projs == 1` (non-reflected) skip rendering when `global_refl_mode=true` (line 1314-1315).

---

### `Renderer::RenderMesh` (render.cpp:1505-1535)

**Signature:**
```cpp
void Renderer::RenderMesh(Inst* inst, Mesh* m, double* tm, void* cookie)
```

**Purpose:**
Callback for mesh rendering. Constructs combined view*instance transform matrix, then invokes `QueryMesh()` to iterate over mesh faces and rasterize via `RenderFace` callback.

**Called by:**
- `QueryWorld` callback during world query phase

**Calls:**
- `MatProduct()` (matrix multiplication, line 1518)
- `QueryMesh()` (face iteration callback, line 1519)

**Globals read:**
- `global_refl_mode` (affects Z-flip in view matrix)

**Globals mutated:**
- `r->inst_tm` (instance transform reference)
- `r->viewinst_tm` (combined view*instance matrix)

**Side effects:**
Queues mesh faces for rasterization via `RenderFace` callback.

**Notes:**
- Reflection mode flips Z component: `-1.0` when `global_refl_mode=true`, `+1.0` otherwise (line 1513).
- View matrix incorporates 2x sample-buffer scaling (HEIGHT_CELLS multiplier).
- Comment notes that RGB/diffuse mapping happens in post-pass, not per-face (lines 1524-1534).

---

### `Renderer::RenderPatch` (render.cpp:1543-2045)

**Signature:**
```cpp
void Renderer::RenderPatch(Patch* p, int x, int y, int view_flags, void* cookie)
```

**Purpose:**
Callback for terrain patch rendering. Transforms patch's (HEIGHT_CELLS+1)^2 vertex grid to screen coords, splits each cell into 2 triangles (diagonal chosen by `GetTerrainDiag`), computes per-triangle diffuse lighting, then rasterizes via `Rasterize<>` with inline shader sampling patch visual map.

**Called by:**
- `QueryTerrain` callback during terrain query phase

**Calls:**
- `GetTerrainHeightMap()` (heightmap retrieval)
- `GetTerrainDiag()` (diagonal pattern lookup)
- `GetTerrainVisualMap()` (texture map retrieval)
- `GetTerrainDark()` (optional, darkness mask, line 1891)
- `DotProduct()` (perspective distance)
- `Rasterize()` (template rasterizer, 8 calls for 4 cells × 2 triangles)
- `Bresenham()` (grid line drawing, line 2041-2042)

**Globals read:**
- `global_refl_mode` (reflection mode)
- `water` (water plane height)
- `DARK_TERRAIN` (optional compile flag)

**Globals mutated:**
- `r->sample_buffer.ptr` (sample buffer array)

**Side effects:**
Writes terrain geometry and lighting into SampleBuffer; marks parity bits for grid lines.

**Notes:**
- Vertex transform: inline logic (not delegated) to avoid per-face overhead; perspective correction applied if enabled.
- Reflection handling: water level clipping with HEIGHT_SCALE/8 tolerance; Z sign flipped in reflected mode.
- Grid lines: drawn in non-reflection mode only; raised by HEIGHT_SCALE/2 for visibility.
- Parity bits (0-3): 0=empty, 1=odd patch, 2=even patch, 3=underwater (bits 0-1 of spare).
- Terrain darkness (optional): dimness mask per cell indexed into dark bitmap (line 1609-1615).

---

### `Renderer::RenderSprite` (render.cpp:2053-2289+)

**Signature:**
```cpp
void Renderer::RenderSprite(AnsiCell* ptr, int width, int height, Sprite* s, bool refl, int anim, int frame, int angle, int pos[3])
```

**Purpose:**
Rasterizes a single sprite frame into output AnsiCell buffer. Performs Z-buffer depth testing against SampleBuffer, handles "swoosh" transparency (smoke/magic effects), and transforms sprite height/depth to world space for correct occlusion.

**Called by:**
-  called from resolve phase to blit queued sprites (inferred from `sprites_alloc` array)

**Calls:**
- `LightenColor()` (multiple calls for color brightening)
- `AverageGlyph()` (color blending based on mask)
- `std::max()` (bounds clamping)
- `std::min()` (bounds clamping)
- `floorf()` (coordinate rounding)

**Globals read:**
- `sample_buffer` (SampleBuffer for depth testing)
- `water` (water plane height)
- Sprite constants: `SPRITE_TRANSPARENT_INDEX`, `SPRITE_SWOOSH_INDEX`, `SPRITE_MASK_FULL`, `SPRITE_ZOOM`, `SPRITE_SCALE` (sprite_constants.h)

**Globals mutated:**
- `ptr` (output AnsiCell buffer)
- `sample_buffer.ptr[].height` (depth buffer, updated during writes)

**Side effects:**
Composites sprite pixels into output buffer with proper depth ordering and transparency handling.

**Notes:**
- TODO(PIPELINE-FIX): No validation of anim/frame/angle bounds; out-of-range reads possible if .xp asset is mis-exported.
- TODO(PIPELINE-FIX): ref[0]/2, ref[1]/2 assume 2x supersampling; if DBL define removed or cell size changes, sprite depth is wrong.
- Swoosh handling: two paths depending on which channel (fg or bk) is SPRITE_SWOOSH_INDEX.
  - Swoosh marker (fg): non-depth-writing transparency for overlays; reads 4-sample depth mask.
  - Swoosh background (bk): depth-writing transparency; updates sample heights for further occlusion.
- Dither glyphs (220-223): 220=lower, 223=upper, 221=left, 222=right (partial block fill for anti-aliasing).
- Perspective constants (ds, dz_dy): must match Render() function values; dynamic zoom would require sprite depth recalc.
- Per-sample depth testing: 2x2 sample grid (s00, s01, s10, s11) per output cell for sub-pixel accuracy.

---

### `Sample::DepthTest_RO` (render.cpp:585-588)

**Signature:**
```cpp
inline bool DepthTest_RO(float z)
```

**Purpose:**
Read-only depth test for determining if a candidate depth passes the current sample's height threshold. Returns true if the candidate depth is greater than or equal to the sample height (minus a small fudge factor).

**Called by:**
- `Bresenham` (lines 148, 151, 179, during line rasterization)
- `PerspectiveCorrectCellLine` (lines 235, 275, during perspective-correct line drawing)
- `CellLine` (lines 331, 368, during orthographic line drawing)
- `RenderSprite` (lines 4132-4133, for 4-sample depth testing)

**Calls:**
None (inline comparison only)

**Globals read:**
- `HEIGHT_SCALE/2` (tolerance constant)

**Globals mutated:**
None (read-only test)

**Side effects:**
None (pure read-only test, no state modification)

**Notes:**
- Read-only variant: does not modify sample state, only checks depth pass condition.
- Fudge factor: adds `HEIGHT_SCALE/2` tolerance to avoid Z-fighting artifacts on nearly coplanar surfaces.
- The "RO" suffix indicates this is for read-only depth queries, unlike the commented-out `DepthTest_RW` which would update the depth buffer.

---

### `SpriteRenderBuf::FarToNear` (render.cpp:620-630)

**Signature:**
```cpp
static int FarToNear(const void* a, const void* b)
```

**Purpose:**
Comparator function for `qsort()` that sorts sprites from far to near based on their `dist` field. Used as a qsort callback to implement correct painter's algorithm ordering for sprite compositing.

**Called by:**
- `Render` (line 4079, via `qsort(r->sprites_alloc, r->sprites, sizeof(SpriteRenderBuf), SpriteRenderBuf::FarToNear)`)

**Calls:**
None (pure comparison logic)

**Globals read:**
None

**Globals mutated:**
None (comparator only)

**Side effects:**
None (qsort callback, no direct side effects)

**Notes:**
- Sort order: returns -1 if `p->dist > q->dist`, 1 if `p->dist < q->dist`, 0 for equal distances.
- Ascending dist means descending visual depth (far sprites drawn first, near sprites drawn last).
- Static member function because it needs to work with C-style qsort API which requires a static function pointer.
- Used during sprite render phase: sprites are queued during World query, then sorted before blitting in the resolve pass.

---

### `Renderer::Init` (render.cpp:635-639)

**Signature:**
```cpp
void Init()
```

**Purpose:**
Initialize the Renderer object to zero state and reseed the Perlin noise generator for procedural effects (water animation, etc.).

**Called by:**
- `CreateRenderer` (line 2804, immediately after allocation)

**Calls:**
- `memset()` (to zero the structure)
- `pn.reseed()` (Perlin noise reseed)

**Globals read:**
- `std::default_random_engine::default_seed` (standard library constant)

**Globals mutated:**
- `this` (the entire Renderer struct, zero-initialized)

**Side effects:**
- Zeroes all Renderer member variables (sample_buffer, sprites_alloc, items, npcs, matrices, etc.).
- Resets Perlin noise generator state.

**Notes:**
- Minimal initialization: heavyweight allocation (sample_buffer, sprites_alloc) is deferred to first `Render()` call to support dynamic resizing.
- Perlin noise reseed ensures consistent procedural generation across renderer lifetimes.
- No memory allocation here: `sample_buffer.ptr` and `sprites_alloc` allocated on-demand in `Render()`.

---

### `Renderer::Free` (render.cpp:641-647)

**Signature:**
```cpp
void Free()
```

**Purpose:**
Release all dynamically allocated resources owned by the Renderer object. Symmetric with `Init()` for cleanup.

**Called by:**
- `DeleteRenderer` (line 2811, before freeing the Renderer struct itself)

**Calls:**
- `free()` (stdlib, lines 644, 646)

**Globals read:**
None

**Globals mutated:**
None (only frees memory, doesn't set pointers to NULL)

**Side effects:**
- Frees `sample_buffer.ptr` heap allocation (if non-NULL)
- Frees `sprites_alloc` heap allocation (if non-NULL)

**Notes:**
- Does NOT check for NULL before freeing (standard C free() handles NULL gracefully).
- Does NOT zero the pointers after freeing; relies on subsequent Init() or object destruction to clear state.
- Called by `DeleteRenderer` which then `free(r)` to release the Renderer struct itself.
- If Renderer is reused after Free(), must call Init() again before next use.

---

## Summary Table

| Function | Lines | Type | Purpose |
|----------|-------|------|---------|
| Bresenham | 111-184 | Template | Line rasterization with grid flag overlay |
| PerspectiveCorrectCellLine | 192-286 | Template | Perspective-correct line drawing |
| CellLine | 291-379 | Template | Orthographic line drawing |
| Rasterize | 404-557 | Template | Barycentric triangle rasterizer |
| create_auto_mat | 710-840 | Static | RGB555→xterm color lookup table generation |
| Renderer::RenderFace | 847-1168 | Member | Mesh triangle callback |
| Renderer::RenderSprite (v1) | 1175-1503 | Member | Sprite queuing and projection |
| Renderer::RenderMesh | 1505-1535 | Member | Mesh rendering orchestrator |
| Renderer::RenderPatch | 1543-2045 | Member | Terrain patch callback |
| Renderer::RenderSprite (v2) | 2053-2289+ | Member | Sprite rasterization and compositing |

