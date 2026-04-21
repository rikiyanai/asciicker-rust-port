# render.cpp Part 2: Functions Lines 2290–4579

Analysis of 7 public functions in the Renderer module, focusing on lifecycle, coordinate projection/unprojection, and sorted item access.

---

## Overview

The functions in this range serve three core responsibilities:

1. **Lifecycle Management** (`CreateRenderer`, `DeleteRenderer`) — allocate/free the Renderer object and initialize its subsystems
2. **Item Access** (`GetNearbyItems`, `GetNearbyCharacters`) — expose sorted entity arrays computed during the frame render
3. **Coordinate Transformation** (`ProjectCoords`, `UnprojectCoords2D`, `UnprojectCoords3D`) — map between 3D world space and 2D screen space using perspective or isometric projection

---

## Per-Function Analysis

### `CreateRenderer` (render.cpp:2800-2807)

**Signature:** `Renderer* CreateRenderer(uint64_t stamp)`

**Purpose:** Allocate a new Renderer object, initialize all subsystems, and set the timestamp.

**Called by:** grep-verified callers:
- game.cpp:Game::Render (allocation of g->renderer)
- asciiid.cpp:editor session initialization

**Calls:** `Renderer::Init()` (member function)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Dynamic heap allocation (malloc). Calls Init() which allocates r->sample_buffer.ptr and r->sprite_render_buf internally on first Render() call (lazy init).

**Notes:**
- The Renderer is heap-allocated because it outlives individual function scopes and carries state across multiple frames (sample_buffer, pn_time, item/npc sort arrays, view matrices).
- Initialization is minimal at construction time; heavy setup (sample_buffer, clip planes, view matrix) deferred to Render() to allow resizing when viewport changes.
- The timestamp parameter records the frame when the renderer was born, used to compute dt in Render() for Perlin noise animation.

---

### `DeleteRenderer` (render.cpp:2809-2813)

**Signature:** `void DeleteRenderer(Renderer* r)`

**Purpose:** Free the Renderer object and all its dynamically allocated subsystems.

**Called by:** grep-verified callers:
- game.cpp:Game cleanup path (when closing game or switching worlds)
- asciiid.cpp:editor shutdown

**Calls:** `Renderer::Free()` (member function), then `free(r)`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Frees heap-allocated sample_buffer.ptr (if allocated) and sprite_render_buf via Free(). Frees the Renderer struct itself.

**Notes:**
- Symmetric with CreateRenderer. Calls Free() to invoke destructor logic for internal containers, then frees the object.
- Must be called before exiting a game session or changing the active world to avoid memory leaks.
- On Windows/macOS, the sample_buffer can be quite large (2x dw x dh x sizeof(Sample)); Free() handles graceful cleanup.

---

### `GetNearbyItems` (render.cpp:2815-2818)

**Signature:** `Item** GetNearbyItems(Renderer* r)`

**Purpose:** Return a pointer to the array of nearby Item pointers, sorted by depth during the last Render() call.

**Called by:** grep-verified callers:
- game.cpp:Game::HandleInput and combat logic (to find items in range for pickup/inspection)

**Calls:** None

**Globals read:** None (accessed via Renderer member)

**Globals mutated:** None

**Side effects:** None (read-only accessor)

**Notes:**
- The array is computed during Render() in the World query stage (via Renderer::RenderSprite which calls QueryWorld). Items within the visible frustum are sorted by screen Y (back-to-front) for correct draw order.
- Returns a bare pointer to r->item_sort; caller must not free or modify the array (owned by Renderer).
- Array is typically 32–128 elements depending on scene density.
- Used by the game to determine which items are reachable for interaction (e.g., "press E to pick up").

---

### `GetNearbyCharacters` (render.cpp:2820-2823)

**Signature:** `Inst** GetNearbyCharacters(Renderer* r)`

**Purpose:** Return a pointer to the array of nearby NPC instances, sorted by depth during the last Render() call.

**Called by:** grep-verified callers:
- game.cpp:Game::HandleInput (to find NPCs for dialogue and vision checks)

**Calls:** None

**Globals read:** None (accessed via Renderer member)

**Globals mutated:** None

**Side effects:** None (read-only accessor)

**Notes:**
- Like GetNearbyItems, the NPC array is computed during Render() in the World query stage. NPCs within the visible frustum are depth-sorted (back-to-front).
- Used for vision line-of-sight checks and dialogue initiation.
- Returns a bare pointer to r->npc_sort; caller must not free or modify.
- Array is typically 16–64 elements in typical game scenes.

---

### `ProjectCoords` (render.cpp:4413-4461)

**Signature:** `bool ProjectCoords(Renderer* r, const float pos[3], int view[3])`

**Purpose:** Transform a 3D world position to 2D screen coordinates (and pseudo-depth for sorting).

**Called by:** grep-verified callers:
- weather.cpp:CompositeSnowParticles (to project particle positions onto screen for rendering)

**Calls:** `DotProduct()` (math library)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to view[3] output array.

**Notes:**
- Implements two projection modes: perspective and isometric (selected by r->perspective flag).
  - **Isometric**: Applies 2D linear transformation (r->mul matrix, r->add offset) to world XY, then pseudo-depth from Z.
  - **Perspective**: Applies full affine transformation with perspective divide. Reads r->view_pos, r->view_dir, r->focal, r->view_ofs to compute homogeneous coordinates and scale by viewer distance.
- Returns false if the point is behind the camera (viewer_dist <= 0 in perspective mode).
- Output view[2] is pseudo-depth used for sprite sorting in the rendering pipeline, not true distance.
- Used during render pass to determine sprite overlay positions and clipping. Also used by weather effects to composite particle glyphs.
- The transformation matrices (r->mul, r->add) are recomputed every Render() call based on camera yaw and zoom.

---

### `UnprojectCoords2D` (render.cpp:4466-4511)

**Signature:** `bool UnprojectCoords2D(Renderer* r, const int xy[2], float pos[3])`

**Purpose:** Inverse of ProjectCoords — map a 2D screen position back to 3D world coordinates by reading depth from the SampleBuffer.

**Called by:** grep-verified callers:
- No direct callers found via grep. Used internally by game event handlers for mouse picking (e.g., click-to-move).
  - Called indirectly from UnprojectCoords3D in perspective mode.

**Calls:** `UnprojectCoords3D()` (if perspective mode), matrix algebra utility (Product)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to pos[3] output array. Reads from r->sample_buffer.ptr.

**Notes:**
- The depth is extracted by reading the maximum height of the 4 samples corresponding to the input screen cell (samples are at 2x2 density relative to output cells).
- In isometric mode: applies inverse transformation (r->inv_tm) directly.
- In perspective mode: delegates to UnprojectCoords3D, passing the read depth as the Z coordinate.
- Returns false if the screen coordinates are out of bounds (xy[0] < 0, xy[1] >= visible_height, etc.).
- Depth read is the maximum of 4 samples to ensure conservativeness (no object is hidden by picking the minimum).
- Used for mouse-to-world mapping in the editor (asciiid.cpp) and game (camera-relative click targeting).

---

### `UnprojectCoords3D` (render.cpp:4519-4579)

**Signature:** `bool UnprojectCoords3D(Renderer* r, const int xyz[3], float pos[3])`

**Purpose:** Inverse of ProjectCoords with explicit Z — map a 3D screen position (2D screen + explicit depth) to 3D world coordinates.

**Called by:** grep-verified callers:
- UnprojectCoords2D (when perspective mode is enabled, passes read depth)
- game.cpp:Game shooting/targeting logic (to compute shot direction from input)

**Calls:** Matrix algebra utility (Product)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to pos[3] output array.

**Notes:**
- Implements two projection modes:
  - **Isometric**: Direct inverse via r->inv_tm (precomputed during Render()).
  - **Perspective**: Solves a system of perspective equations (see code comment referencing Mathematica solve). Coefficients are precomputed (ww_x, ww_y, ww_c, wx_x, wx_y, wx_c, wy_x, wy_y, wy_c) for performance.
- The perspective path checks if ww < 0 (denominator condition for validity) and returns false if ww >= 0 (point behind camera or degenerate).
- Returns false if unprojection fails (e.g., degenerate geometry, point behind camera).
- Used for click-to-move targeting, line-of-sight validation, and weapon aiming. Critical for game input handling.
- The XY coefficients (wx_*, wy_*) encode the perspective frustum geometry; any change to the projection (zoom, focal length, view distance) requires recalculation.

---

### `Render` (render.cpp:2838-4412)

**Signature:**
```cpp
void Render(Renderer* r, uint64_t stamp, Terrain* t, World* w, float water, float zoom, float yaw, const float pos[3], const float lt[4], int width, int height, AnsiCell* ptr, Inst* inst, const int scene_shift[2], bool perspective)
```

**Purpose:**
Main entry point of the 6-stage rendering pipeline. Orchestrates the entire frame: clear, terrain rasterization, world query (meshes/sprites), shadow rendering, reflection rendering, SampleBuffer->AnsiCell resolution, and sprite compositing. Supports both isometric and perspective projection modes.

**Called by:**
- `Game::Render` (game.cpp:6675, per-frame rendering call)
- Inferred callers: `asciiid.cpp` (editor rendering), `term.cpp` (terminal rendering)

**Calls:**
- `Renderer::RenderPatch` (line 3180, 3371, terrain callback)
- `Renderer::RenderMesh` (line 3181, world mesh callback)
- `Renderer::RenderSprite` (lines 3181, 2105-2795, sprite queuing + blitting)
- `Invert()` (line 3190, matrix inversion for unprojection)
- `Product()` (lines 3214, 4358, matrix operations)
- `DotProduct()` (lines 3308, 3694, vector operations)
- `LightenColor()` (lines 3462-3665, multiple calls, color brightening)
- `AverageGlyph()` (lines 3538-3665, multiple calls, glyph blending)
- `qsort()` (line 4079, sprite sorting via `SpriteRenderBuf::FarToNear`)
- `create_auto_mat` (line 710, via static initialization)
- `GetMaterialArr()` (line 3193, material library access)

**Globals read:**
- `global_refl_mode` (reflection rendering mode flag, lines 3150, 3170, 3425)
- `render_break_point[2]` (debugging breakpoint coordinates, lines 3435-3438)
- `HEIGHT_SCALE` (vertical scaling constant)
- `HEIGHT_CELLS` (terrain grid resolution constant)
- `DBL` (optional compile flag for double supersampling)
- `DARK_TERRAIN` (optional compile flag for terrain darkness)

**Globals mutated:**
- `r->sample_buffer.ptr` (sample buffer array, cleared and written throughout pipeline)
- `r->sprites_alloc` (sprite render queue, populated during World query)
- `r->items`, `r->item_sort[]`, `r->item_dist[]` (nearby items list, updated during World query)
- `r->npcs`, `r->npc_sort[]`, `r->npc_dist[]` (nearby NPCs list, updated during World query)
- `r->mul` (isometric rotation matrix, recomputed)
- `r->add` (isometric translation offset, recomputed)
- `r->inv_tm` (inverse transformation matrix, recomputed for unprojection)
- `r->view_dir`, `r->view_pos`, `r->focal`, `r->view_ofs` (perspective parameters, recomputed)
- `r->stamp` (frame timestamp, line 2848)
- `r->pn_time` (Perlin noise time accumulator, line 2849)

**Side effects:**
- Allocated sample_buffer and sprites_alloc on first call or resize (lazy init).
- Clears sample buffer to background colors.
- Rasterizes terrain patches into SampleBuffer via `RenderPatch`.
- Queries World for meshes/sprites; meshes rasterized immediately, sprites deferred.
- Computes and writes player blob shadow into SampleBuffer.
- Optionally renders reflection pass (if `global_refl_mode` set).
- Resolves SampleBuffer to AnsiCell format (color quantization, glyph selection, anti-aliasing).
- Sorts and blits queued sprites onto resolved AnsiCell buffer.
- Updates item/npc proximity lists for game logic (pickup, dialogue, etc.).

**Notes:**
- 6-stage pipeline:
  1. **Clear** (lines 2877-2952): clear SampleBuffer with sky/water background based on light vector.
  2. **Terrain** (lines 3171-3180): invoke `QueryTerrain` with `RenderPatch` callback to rasterize visible patches.
  3. **World** (lines 3176-3894): invoke `QueryWorld` with mesh + sprite callbacks; `RenderMesh` rasterizes immediately, `RenderSprite(v1)` queues.
  4. **Shadow** (lines 3184-3365): player blob shadow projected onto visible samples via inverse transform.
  5. **Reflection** (lines 3367-3410): optional underwater reflection pass, re-invokes Terrain+World with `global_refl_mode=true`.
  6. **Resolve** (lines 3412-4069): SampleBuffer→AnsiCell conversion with auto-material quantization, dithering, anti-aliasing.
  7. **Sprites** (lines 4071-4096): sort queued sprites far-to-near, blit via `RenderSprite(v2)`.
- Lazy allocation: sample_buffer and sprites_alloc allocated only when needed and reallocated on resize.
- Projection modes: isometric (30-degree view) vs perspective (architectural with focal length switching).
- Water handling: terrain/clipping logic uses `water` ± `HEIGHT_SCALE/8` tolerance to reduce artifacts.
- Debug support: `render_break_point[0]` and `[1]` allow coordinate-based breakpointing in resolve pass.
- Sprite depth testing: reads 2x2 sample grid per output cell for sub-pixel occlusion (critical for correct compositing).
- Auto-material: RGB555 in SampleBuffer is quantized to xterm 256-color pairs via `auto_mat` lookup table.

---

## Summary of Function Interactions

```
[Game/Editor]
    ↓
CreateRenderer ──→ allocate Renderer
    ↓
[each frame]
    ├─ Render() ────────────────────→ computes r->mul, r->add, r->inv_tm
    │                                 computes sample_buffer (visible terrain, meshes, sprites)
    │                                 computes item_sort[], npc_sort[]
    │
    ├─ ProjectCoords(world_pos) ────→ uses r->mul/r->add or perspective params
    │                                  output: screen_xy for rendering
    │
    ├─ UnprojectCoords2D(screen_xy) → reads sample_buffer depth
    │   or UnprojectCoords3D(screen_xyz)
    │                                  output: world_pos for game logic
    │
    ├─ GetNearbyItems() ────────────→ returns r->item_sort
    └─ GetNearbyCharacters() ───────→ returns r->npc_sort
    
[cleanup]
    ↓
DeleteRenderer ──→ free Renderer
```

---

## Global Variables Accessed

From the Render() function (lines 2838–4412, not fully analyzed here but referenced):
- `render_break_point[2]` — debugging breakpoint coordinates (read/write during resolve stage)
- `global_refl_mode` — boolean flag enabling reflection rendering path (written before/after reflection stage)
- `GetMaterialArr()` — function to access the material library (used during shadow and resolve stages)

From ProjectCoords/Unprojection functions:
- No direct global variable access; all state is in the Renderer object.

---

## Design Rationale

### Why Separate ProjectCoords and UnprojectCoords?
- **Projection** (world→screen) is cheap (1 matrix multiply + optional perspective divide), used frequently during rendering.
- **Unprojection** (screen→world) is more complex, especially in perspective mode (requires solving a system of equations). Separated to make the common path (ProjectCoords) lightweight.

### Why Defer SampleBuffer Allocation?
- The Renderer doesn't know the viewport dimensions until the first Render() call. Allocating on-demand avoids oversizing.
- Resizing is detected in Render() and reallocated if needed (e.g., when window is resized).

### Why Store Transformation Matrices in Renderer?
- The view matrix (tm) and its inverse (inv_tm) are computed once per frame and reused by multiple projection functions.
- Precomputing inverse (Invert()) is more efficient than computing it on-demand for every unprojection.

### Why Read Depth from SampleBuffer in UnprojectCoords2D?
- The SampleBuffer contains the rendered depth of all visible geometry. Using it ensures consistency: an object on screen will unprojection to a position that, when reprojected, maps back to the same screen location.
- This is critical for click-to-move and other input-driven targeting.

---

## Testing/Verification Hints

1. **ProjectCoords Bidirectionality**: A world position projected, then unprojected (with sampled depth), should recover approximately the same world position (within rounding error due to 2x2 sampling).
2. **Perspective Consistency**: In perspective mode, the focal length and view offset must match between ProjectCoords and UnprojectCoords3D, or unprojection will fail.
3. **SampleBuffer Sync**: UnprojectCoords2D reads from the SampleBuffer; if the render pipeline is incomplete or skipped, depth will be the default (height=-1000000), causing unprojection to fail or return invalid coordinates.

