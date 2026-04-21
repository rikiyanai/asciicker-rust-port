# terrain.cpp Part 2: Functions (lines 1711-3310)

Analysis of core terrain query and raycasting functions.

### `UpdateTerrainDark (with PatchIndex*)` (terrain.cpp:1754-1759)

**Signature:**
```cpp
void UpdateTerrainDark(Terrain* t, PatchIndex* pi, World* w, float lightpos[3], bool editor)
```

**Purpose:**
Updates a single terrain patch's darkness/shadow mask based on light direction and world occlusion. Computes which cells are shadowed by objects in the world.

**Called by:**
- asciiid.cpp:7511 — editor lighting update

**Calls:**
- QueryTerrainSample (callback traversal)
- HitWorld (world mesh intersection test)

**Globals read:**
- updater.w (World pointer)
- updater.lightdir (normalized light direction, scaled by HEIGHT_SCALE)

**Globals mutated:**
- p->dark (per-cell shadow mask, updated via DarkUpdater callback)

**Side effects:**
Updates terrain patch visual appearance by modifying shadow mask; affects rendering output.

**Notes:**
- Overload 1 of 2: Takes PatchIndex to process single patch at known location
- DarkUpdater struct carries context (editor flag, terrain, world, light direction)
- Uses HEIGHT_SCALE/4 threshold for vertical occlusion checks
- Updates dark mask via bitwise OR (accumulates shadows)

---

### `UpdateTerrainDark (Terrain*, World*)` (terrain.cpp:1760-1765)

**Signature:**
```cpp
void UpdateTerrainDark(Terrain* t, World* w, float lightpos[3], bool editor)
```

**Purpose:**
Updates darkness/shadow masks for all patches in terrain. Wrapper that queries entire quadtree and invokes single-patch update for each.

**Called by:**
- asciiid.cpp:7511 — full terrain lighting update
- render.cpp (lighting pass)

**Calls:**
- QueryTerrainSample (full quadtree traversal)
- DarkUpdater callback (per patch)

**Globals read:**
- t->root (quadtree root)
- t->x, t->y, t->level (tree bounds)
- VISUAL_CELLS (const 8)

**Globals mutated:**
- All patch->dark fields via DarkUpdater

**Side effects:**
Updates all terrain visual appearance simultaneously; full lighting recalculation.

**Notes:**
- Overload 2 of 2: Processes all patches in terrain
- Negates light position to create view-relative light direction
- Uses VISUAL_CELLS << t->level for root range calculation

---

### `QueryTerrain (inline, no frustum)` (terrain.cpp:1772-1798)

**Signature:**
```cpp
static inline void QueryTerrain(QuadItem* q, int x, int y, int range, int view_flags, 
                                void(*cb)(Patch* p, int x, int y, int view_flags, void* cookie), 
                                void* cookie)
```

**Purpose:**
Recursively traverses quadtree without frustum culling. Visits all patches in subtree, invoking callback for each leaf patch. Used when no spatial filtering needed.

**Called by:**
- Other QueryTerrain overloads (dispatch point)
- Indirectly from render paths

**Calls:**
- Recursively: QueryTerrain on 4 children
- cb (user callback)

**Globals read:**
- QuadItem.flags (neighbor indicators)
- VISUAL_CELLS (const 8)

**Globals mutated:**
- None (read-only traversal)

**Side effects:**
Invokes callback for each patch; callback determines actual side effects.

**Notes:**
- Marked `static inline` for optimization
- view_flags & ~q->flags: filters edge rendering based on neighbor presence
- No early rejection — visits entire subtree
- Base case: range == VISUAL_CELLS triggers callback
- Recursively visits all 4 quadrants

---

### `QueryTerrain (inline, with frustum planes)` (terrain.cpp:1803-1904)

**Signature:**
```cpp
static void inline QueryTerrain(QuadItem* q, int x, int y, int range, int planes, double* plane[], 
                                int view_flags, 
                                void(*cb)(Patch* p, int x, int y, int view_flags, void* cookie), 
                                void* cookie)
```

**Purpose:**
Frustum-culled quadtree traversal. Tests AABB (axis-aligned bounding box) of each node against frustum planes, skipping subtrees entirely outside view volume.

**Called by:**
- QueryTerrain(Terrain*, int planes, ...) dispatch

**Calls:**
- PositiveProduct (plane-point tests)
- Recursively: QueryTerrain (with/without planes depending on culling)
- cb (user callback)

**Globals read:**
- q->lo, q->hi (height bounds)
- plane[i] (frustum plane equations)

**Globals mutated:**
- None (modifies local plane array copy only)

**Side effects:**
Invokes callback for visible patches only; reduces callback invocations via culling.

**Notes:**
- Tests all 8 corners of bounding box against each frustum plane
- Early out if all 8 corners on negative side of any plane (completely outside)
- Plane removal optimization: if all 8 corners on positive side, removes plane from further checks
- Switches to faster non-culled query once all planes eliminated
- Per-node plane array management: swaps eliminated planes, decrements count

---

### `QueryTerrain (Terrain*, planes)` (terrain.cpp:1906-1918)

**Signature:**
```cpp
void QueryTerrain(Terrain* t, int planes, double plane[][4], int view_flags, 
                  void(*cb)(Patch* p, int x, int y, int view_flags, void* cookie), 
                  void* cookie)
```

**Purpose:**
Public entry point for frustum-culled terrain query. Validates terrain existence, sets up plane array, dispatches to recursive QueryTerrain variant.

**Called by:**
- render.cpp:3180 — main rendering pipeline
- asciiid.cpp:10575 — editor rendering

**Calls:**
- QueryTerrain (recursive variant with planes)

**Globals read:**
- t->root (quadtree root)
- t->x, t->y, t->level (tree bounds)
- planes (count, 0-6 typically)

**Globals mutated:**
- None

**Side effects:**
Invokes callback for visible patches; controls rendering output.

**Notes:**
- Entry point validates t and t->root non-null
- view_flags & 0xAA masks to CCW directions (0xAA = 0b10101010)
- Copies plane pointers to local array for modification during traversal
- Short-circuit: if planes <= 0, calls non-culled variant instead

---

### `QueryTerrain (QuadItem*, radius)` (terrain.cpp:1921-1989)

**Signature:**
```cpp
void QueryTerrain(QuadItem* q, int x, int y, int range, const double xyr[3], int view_flags, 
                  void(*cb)(Patch* p, int x, int y, int view_flags, void* cookie), 
                  void* cookie)
```

**Purpose:**
Radius-culled quadtree traversal. Tests if AABB overlaps sphere (circle in 2D) defined by center (xyr[0], xyr[1]) and radius xyr[2].

**Called by:**
- QueryTerrain(Terrain*, double x, double y, double r, ...) dispatch

**Calls:**
- Recursively: QueryTerrain (radius variant or no-filter variant)
- cb (user callback)

**Globals read:**
- None (pure geometry)

**Globals mutated:**
- None

**Side effects:**
Invokes callback for patches within radius sphere only.

**Notes:**
- Computes squared distances from circle center to 4 corners of rect
- If hit == 4 (all 4 corners inside circle), switches to fast non-culled recursion for children
- If hit < 4, performs per-child radius test
- Includes axis-aligned strip tests: if fit_x or fit_y, checks strip overlap
- 2D circle-AABB collision: corners + axis-aligned edge checks

---

### `QueryTerrain (Terrain*, radius)` (terrain.cpp:1991-2002)

**Signature:**
```cpp
void QueryTerrain(Terrain* t, double x, double y, double r, int view_flags, 
                  void(*cb)(Patch* p, int x, int y, int view_flags, void* cookie), 
                  void* cookie)
```

**Purpose:**
Public entry point for radius-culled terrain query. Validates terrain, packs center/radius into array, dispatches to recursive QueryTerrain(QuadItem*, radius).

**Called by:**
- game.cpp:10730 — material stamping
- asciiid.cpp (multiple brush/tool operations)

**Calls:**
- QueryTerrain (recursive radius variant)

**Globals read:**
- t->root, t->x, t->y, t->level

**Globals mutated:**
- None

**Side effects:**
Invokes callback for patches within radius.

**Notes:**
- Early exit if r <= 0 (invalid radius)
- Transforms terrain-relative coordinates to world coordinates
- view_flags & 0xAA (CCW direction mask)

---

### `HitPatch` (terrain.cpp:2007-2092)

**Signature:**
```cpp
bool HitPatch(Patch* p, int x, int y, double ray[10], double ret[3], double nrm[3], bool positive_only)
```

**Purpose:**
Tests ray against all 64 terrain cells (8x8 grid) in patch. Each cell contains 2 triangles split by diagonal. Returns true if any hit, updates ray[9] with closest distance and optionally normal.

**Called by:**
- HitTerrain0-7 (per-octant ray tracers)

**Calls:**
- RayIntersectsTriangle (ray-triangle test)
- CrossProduct (normal computation)

**Globals read:**
- triangle_intersections (counter, incremented per test)
- hit_patch_tests (counter, incremented at entry)

**Globals mutated:**
- triangle_intersections (count of triangles tested)
- ret[3], nrm[3] (intersection point and optional normal)

**Side effects:**
Updates ray casting state; computes intersection geometry.

**Notes:**
- Iterates 8x8 cells in patch's local grid
- Per cell: constructs 4 vertices from height[hy][hx], height[hy][hx+1], height[hy+1][hx], height[hy+1][hx+1]
- diag bitfield determines triangle split direction (NW-SE or NE-SW per cell)
- Scales cell positions by sxy = VISUAL_CELLS / HEIGHT_CELLS ratio
- Computes normal via cross product if nrm != null
- Does NOT filter by distance — relies on HitTerrainN caller to manage ray[9]

---

### `HitTerrain0` (terrain.cpp:2096-2161)

**Signature:**
```cpp
Patch* HitTerrain0(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (+X, +Y, +Z) octant. Optimized by encoding direction signs, avoiding per-step branching in inner loop.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 0)

**Calls:**
- HitPatch (leaf level)
- Recursively: HitTerrain0 on 4 children

**Globals read:**
- q->lo, q->hi (height bounds)

**Globals mutated:**
- ret[3] (ray intersection point)
- ray[9] (closest hit distance, updated by HitPatch)

**Side effects:**
Finds closest terrain intersection along ray.

**Notes:**
- 6 hardcoded plane-box tests encode (+X, +Y, +Z) ray direction assumptions
- Early rejection if AABB entirely outside ray
- Front-to-back traversal: quadrants visited in order [0,1,2,3]
- ray[9] used for distance comparison (HitPatch updates with min distance)
- Returns first (closest) patch hit; later hits ignored if further away

---

### `HitTerrain1` (terrain.cpp:2164-2216)

**Signature:**
```cpp
Patch* HitTerrain1(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (-X, +Y, +Z) octant.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 1)

**Calls:**
- HitPatch (leaf level)
- Recursively: HitTerrain1 on 4 children

**Globals read:**
- q->lo, q->hi (height bounds)

**Globals mutated:**
- ret[3], ray[9] (intersection point and distance)

**Side effects:**
Finds closest terrain intersection for (-X, +Y, +Z) rays.

**Notes:**
- Direction signs: bit0=1 (negative X), bit1=0 (positive Y), bit2=0 (positive Z)
- 6 plane-box tests encode this octant's constraints

---

### `HitTerrain2` (terrain.cpp:2219-2271)

**Signature:**
```cpp
Patch* HitTerrain2(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (+X, -Y, +Z) octant.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 2)

**Calls:**
- HitPatch, recursively HitTerrain2

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (+X, -Y, +Z) rays.

**Notes:**
- Direction signs: bit0=0 (positive X), bit1=1 (negative Y), bit2=0 (positive Z)

---

### `HitTerrain3` (terrain.cpp:2274-2326)

**Signature:**
```cpp
Patch* HitTerrain3(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (-X, -Y, +Z) octant.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 3)

**Calls:**
- HitPatch, recursively HitTerrain3

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (-X, -Y, +Z) rays.

**Notes:**
- Direction signs: bit0=1 (negative X), bit1=1 (negative Y), bit2=0 (positive Z)

---

### `HitTerrain4` (terrain.cpp:2329-2382)

**Signature:**
```cpp
Patch* HitTerrain4(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (+X, +Y, -Z) octant (downward rays).

**Called by:**
- HitTerrain (via function pointer dispatch, octant 4)

**Calls:**
- HitPatch, recursively HitTerrain4

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (+X, +Y, -Z) rays (e.g., ground traces).

**Notes:**
- Direction signs: bit0=0 (positive X), bit1=0 (positive Y), bit2=1 (negative Z)

---

### `HitTerrain5` (terrain.cpp:2385-2437)

**Signature:**
```cpp
Patch* HitTerrain5(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (-X, +Y, -Z) octant.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 5)

**Calls:**
- HitPatch, recursively HitTerrain5

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (-X, +Y, -Z) rays.

**Notes:**
- Direction signs: bit0=1 (negative X), bit1=0 (positive Y), bit2=1 (negative Z)

---

### `HitTerrain6` (terrain.cpp:2440-2492)

**Signature:**
```cpp
Patch* HitTerrain6(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (+X, -Y, -Z) octant.

**Called by:**
- HitTerrain (via function pointer dispatch, octant 6)

**Calls:**
- HitPatch, recursively HitTerrain6

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (+X, -Y, -Z) rays.

**Notes:**
- Direction signs: bit0=0 (positive X), bit1=1 (negative Y), bit2=1 (negative Z)

---

### `HitTerrain7` (terrain.cpp:2495-2547)

**Signature:**
```cpp
Patch* HitTerrain7(QuadItem* q, int x, int y, int range, double ray[10], double ret[3], 
                   double nrm[3], bool positive_only)
```

**Purpose:**
Specialized ray-terrain intersection for rays heading in (-X, -Y, -Z) octant (straight down/back).

**Called by:**
- HitTerrain (via function pointer dispatch, octant 7)

**Calls:**
- HitPatch, recursively HitTerrain7

**Globals read:**
- q->lo, q->hi

**Globals mutated:**
- ret[3], ray[9]

**Side effects:**
Finds closest terrain intersection for (-X, -Y, -Z) rays.

**Notes:**
- Direction signs: bit0=1 (negative X), bit1=1 (negative Y), bit2=1 (negative Z)

---

### `HitTerrain (double, u,v)` (terrain.cpp:2549-2624)

**Signature:**
```cpp
double HitTerrain(Patch* p, double u, double v)
```

**Purpose:**
Interpolates terrain height at normalized (u,v) coordinates within a patch. Used for per-pixel height lookup after determining which patch contains a point.

**Called by:**
- asciiid.cpp:3860, 3866, 3888, 3894 — mesh-terrain intersection tests

**Calls:**
- None (pure computation)

**Globals read:**
- HEIGHT_CELLS (const 5)

**Globals mutated:**
- None

**Side effects:**
None (read-only query).

**Notes:**
- u,v expected in [0,1] range; clamped at boundaries (u0/v0 adjusted to HEIGHT_CELLS-1 if == HEIGHT_CELLS)
- Returns -1 if u,v out of [0,1] bounds
- Two interpolation cases based on p->diag bit at (u0, v0):
  - diag set: diagonal from (u0,v0) to (u1,v1) — chooses triangle based on u+v < 1
  - diag unset: diagonal from (u0,v1) to (u1,v0) — chooses triangle based on u-v > 0
- Bilinear interpolation within chosen triangle using height grid values

---

### `HitTerrain (Terrain*, ray)` (terrain.cpp:2650-2695)

**Signature:**
```cpp
Patch* HitTerrain(Terrain* t, double p[3], double v[3], double ret[3], double nrm[3], bool positive_only)
```

**Purpose:**
Public ray-terrain intersection entry point. Casts ray from origin p in direction v against entire terrain, returns closest hit patch and intersection point.

**Called by:**
- game.cpp:4793, 6799, 6918 — gameplay raycasting (ground checks, physics)
- asciiid.cpp:9675 — editor terrain picking

**Calls:**
- HitTerrainN variants (via function pointer dispatch)

**Globals read:**
- t->root, t->x, t->y, t->level (terrain structure)
- triangle_intersections (counter, reset at entry)
- hit_patch_tests (counter, reset at entry)

**Globals mutated:**
- triangle_intersections, hit_patch_tests (performance counters)
- ret[3], nrm[3] (ray intersection point and optional normal)

**Side effects:**
Finds terrain-ray intersections; updates performance diagnostic counters.

**Notes:**
- Constructs 10-element ray array: [0-2] = p × v (plane equation), [3-5] = v (direction), [6-8] = p (origin), [9] = FLT_MAX (distance threshold)
- Computes sign_case from v direction signs (3-bit octant index):
  - bit0: v[0] >= 0 ? 1 : 0 (X sign)
  - bit1: v[1] >= 0 ? 2 : 0 (Y sign)
  - bit2: v[2] >= 0 ? 4 : 0 (Z sign)
- Dispatches to HitTerrain0-7 via static function pointer table indexed by sign_case
- Alternative octant encoding interpretation:
  - sign_case = 0: all positive (ray heading +X, +Y, +Z) → HitTerrain0
  - sign_case = 1: X negative → HitTerrain1
  - etc.

---

### `TerrainDetach` (terrain.cpp:2697-2812)

**Signature:**
```cpp
size_t TerrainDetach(Terrain* t, Patch* p, int* px, int* py)
```

**Purpose:**
Removes patch from terrain quadtree structure. Performs tree cleanup: trims empty interior nodes (leaf trim) and collapses root if it has only 1 child (root trim). Updates neighbor flags on remaining patches.

**Called by:**
- Terrain patch destruction workflows (editor deletion, world unload)

**Calls:**
- GetTerrainPatch (lookup neighbor patches)
- free() (deallocate nodes)
- UpdateNodes (height bound recomputation)

**Globals read:**
- p->parent (node structure)
- p->flags (neighbor presence bits)

**Globals mutated:**
- t->patches-- (decrement patch count)
- t->nodes-- (decrement for each freed node)
- t->root, t->level, t->x, t->y (tree structure if root collapsed)
- neighbor patches' flags (8 adjacent patches updated)
- p->parent = 0 (detached patch marked)

**Side effects:**
Modifies terrain tree structure; deallocates nodes; updates neighbor connectivity.

**Notes:**
- Leaf trim: walks up from detached patch, freeing parent nodes that have no remaining children
- Root trim: collapses root if only 1 child remains, updates t->level and (t->x, t->y) origin
- Neighbor flag update: for each of 8 adjacent neighbors, clears reciprocal direction flag
- Returns sizeof(Patch) (memory freed by this operation, not including recursive node deallocation)
- Assumes patch is not freed by caller (only marked p->parent = 0)

---

### `TerrainAttach` (terrain.cpp:2814-3060)

**Signature:**
```cpp
size_t TerrainAttach(Terrain* t, Patch* p, int x, int y)
```

**Purpose:**
Inserts patch into terrain quadtree at specified (x, y) coordinates. Expands tree boundaries as needed in all 4 directions. Creates intermediate nodes to enclose target coordinates. Updates height bounds via UpdateNodes.

**Called by:**
- Terrain patch creation workflows (editor brush, world load)

**Calls:**
- malloc (allocate intermediate nodes)
- UpdateNodes (recompute height bounds)
- GetTerrainPatch (neighbor lookup)

**Globals read:**
- t->root, t->x, t->y, t->level (initial tree state)

**Globals mutated:**
- t->patches++ (increment count if new)
- t->nodes++ (increment for each created node)
- t->root, t->level, t->x, t->y (tree bounds expansion)
- p->parent (assigned to containing node)

**Side effects:**
Modifies terrain tree structure; allocates memory; updates height hierarchy.

**Notes:**
- Early exit if (x, y) already contains patch (returns 0)
- Tree expansion phase: 4 while loops expand root in X- (negative), Y- (negative), X+ (positive), Y+ (positive) directions
- For each boundary expansion, creates new root parent with old root as one of 4 children
- Tree descent phase: walks from new root down to leaf level, creating intermediate nodes as needed
- Each level creates node if quadrant [i] doesn't exist
- At leaf level (lev == 0), attaches patch to quadrant and calls UpdateNodes
- Patch parent initially set during tree descent
- Returns sizeof(Patch) if new patch added, 0 if already existed

---

### `TerrainDispose` (terrain.cpp:3062-3079)

**Signature:**
```cpp
size_t TerrainDispose(Patch* p)
```

**Purpose:**
Deallocates patch memory and associated texture allocations. Low-level cleanup; assumes patch already detached from terrain tree.

**Called by:**
- Patch destruction workflows

**Calls:**
- TexAlloc::Free() (if TEXHEAP defined)
- UpdateTerrainVisualMap, UpdateTerrainHeightMap (if texture freed)
- free(p) (deallocate patch struct)

**Globals read:**
- p->ta (texture allocator, if TEXHEAP enabled)

**Globals mutated:**
- p freed (deallocated)

**Side effects:**
Deallocates patch memory; updates visual/height maps if texture freed.

**Notes:**
- Conditional compilation: TEXHEAP guards texture cleanup
- If last texture allocation freed, updates visual and height maps for affected patch (last->user)
- Returns sizeof(Patch) (memory freed)

---

### `SaveTree` (terrain.cpp:3100-3132)

**Signature:**
```cpp
void SaveTree(FILE* f, int x, int y, int lev, const QuadItem* item)
```

**Purpose:**
Recursively writes terrain quadtree to file in binary format. Converts quadtree structure to linear sequence of FilePatch records in traversal order.

**Called by:**
- SaveTerrain (initial call)

**Calls:**
- fwrite (binary file output)
- Recursively: SaveTree on 4 children

**Globals read:**
- None (pure I/O)

**Globals mutated:**
- FILE* f (file position advanced)

**Side effects:**
Writes binary patch data to file.

**Notes:**
- Base case (lev == 0): writes single FilePatch record (188 bytes) with coordinates and data
- Recursive case (lev > 0): processes 4 children in order (recursive SaveTree calls)
- Quadrant order: [0] at (x, y), [1] at (x+r, y), [2] at (x, y+r), [3] at (x+r, y+r)
- In-order traversal: children visited front-to-back
- FilePatch format: 8 bytes (x,y) + 128 bytes (visual[8][8]) + 50 bytes (height[5][5]) + 2 bytes (diag) = 188 total

---

### `SaveTerrain` (terrain.cpp:3141-3160)

**Signature:**
```cpp
bool SaveTerrain(const Terrain* t, FILE* f)
```

**Purpose:**
Serializes entire terrain to .a3d file. Writes FileHeader followed by all patches via SaveTree traversal.

**Called by:**
- World save operations (game.cpp, asciiid.cpp)

**Calls:**
- fwrite (header)
- SaveTree (recursive patch serialization)

**Globals read:**
- t->patches (patch count)
- t->root, t->x, t->y, t->level (tree structure)

**Globals mutated:**
- f (file position)

**Side effects:**
Writes complete terrain data to file.

**Notes:**
- Early exit if t or f null (returns false)
- FileHeader magic: "AS3D" (0x33534341 little-endian)
- Header size stored for version compatibility
- Patch count used for load-time array allocation
- Reserved field set to 0

---

### `LoadTerrain` (terrain.cpp:3165-3266)

**Signature:**
```cpp
Terrain* LoadTerrain(FILE* f, PatchIndex** idx)
```

**Purpose:**
Deserializes terrain from .a3d file. Reconstructs quadtree structure, optionally builds PatchIndex for O(1) patch lookup by serial number.

**Called by:**
- World load operations (game.cpp, asciiid.cpp)

**Calls:**
- fread (binary file input)
- CreateTerrain (allocate empty terrain)
- AddTerrainPatch (insert patch into tree)
- UpdateTerrainVisualMap, UpdateTerrainHeightMap (rebuild per-patch lookup tables)
- memcpy (copy patch data)
- malloc (allocate PatchIndex array)
- DeleteTerrain (error cleanup)

**Globals read:**
- ASCIICKER_TERRAIN_DEBUG (env var, optional logging)

**Globals mutated:**
- f (file position advanced)
- *idx (output: allocated PatchIndex array)

**Side effects:**
Reconstructs terrain from file; optionally builds index; outputs debug traces if env var set.

**Notes:**
- Validates file signature "AS3D" and header size
- Early exit (returns null) if header invalid or read fails
- Creates PatchIndex array if idx != null (one entry per patch, allocated up-front with num_patches size)
- Iterates num_patches times: reads FilePatch, calls AddTerrainPatch, stores in index if allocated
- Populates PatchIndex with (patch pointer, x, y) triplets for fast lookup
- Debug env var enables stderr tracing of patch loading progress and coordinate bounds
- Returns initialized Terrain* or null on failure

---

### `FreePatchIndex` (terrain.cpp:3268-3272)

**Signature:**
```cpp
void FreePatchIndex(PatchIndex* idx)
```

**Purpose:**
Deallocates PatchIndex array allocated by LoadTerrain. Simple wrapper around free().

**Called by:**
- Terrain unload / cleanup workflows

**Calls:**
- free (deallocate)

**Globals read:**
- None

**Globals mutated:**
- idx freed (deallocated)

**Side effects:**
Frees memory.

**Notes:**
- Null-safe: checks idx != null before freeing
- Assumes idx was allocated by malloc in LoadTerrain

---

### `CollectPatchesRecursive` (terrain.cpp:3275-3289)

**Signature:**
```cpp
static void CollectPatchesRecursive(Node* n, int level, Patch*** out, int* count, int* cap)
```

**Purpose:**
Static helper for GetAllTerrainPatches. Recursively traverses quadtree, collecting all leaf patches into flat array with dynamic growth.

**Called by:**
- GetAllTerrainPatches (initial call)
- Recursively: CollectPatchesRecursive on 4 children

**Calls:**
- realloc (grow array if needed)
- Recursively: CollectPatchesRecursive

**Globals read:**
- None

**Globals mutated:**
- *out (patch array pointer, is reallocated)
- *count (number of patches collected)
- *cap (current capacity)

**Side effects:**
Builds flat patch array with dynamic reallocation.

**Notes:**
- Base case (level == 0): appends node to output array (it's a patch)
- Recursive case (level > 0): visits all 4 children
- Growth strategy: starts at cap=16, doubles each reallocation (cap = cap * 2)
- Out-of-bounds check: if count >= cap, reallocates before appending
- Assumes n is valid (null-check in caller)

---

### `GetAllTerrainPatches` (terrain.cpp:3291-3310)

**Signature:**
```cpp
void GetAllTerrainPatches(Terrain* t, Patch*** out_patches, int* out_count)
```

**Purpose:**
Returns flat array of all terrain patches. Provides convenient bulk-operation access to entire patch set without quadtree traversal by caller.

**Called by:**
- Bulk terrain operations (rendering, physics, diagnostics)

**Calls:**
- malloc (allocate initial array)
- CollectPatchesRecursive (if level > 0)

**Globals read:**
- t->root, t->level

**Globals mutated:**
- *out_patches (output: allocated patch array)
- *out_count (output: patch count)

**Side effects:**
Allocates memory; populates output parameters.

**Notes:**
- Null-safe: handles t == null or t->root == null (returns empty array: out_patches=null, out_count=0)
- Special case (level == 0): root is single patch; directly assigns and sets count=1
- Otherwise: recursively collects patches via CollectPatchesRecursive
- Caller responsible for freeing *out_patches via free()

---
