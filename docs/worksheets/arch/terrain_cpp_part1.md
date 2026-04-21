# terrain.cpp Function Analysis (Lines 1-1710)

Complete schema of all public and significant internal functions in the terrain quadtree system.

---

## Helper Functions

### `my_abs` (terrain.cpp:130-135)

**Signature:** `inline int my_abs(int i)`

**Purpose:** Compute absolute value of a signed integer for terrain geometry calculations

**Called by:** `Tap3x3::Update` (terrain.cpp:549)

**Calls:** None (inline, math only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (pure function)

**Notes:** Simple branch-based implementation instead of `abs()` or `std::abs()`. Used by diagonal orientation calculation to compare height gradient magnitudes.

---

### `GetTerrainBase` (terrain.cpp:223-227)

**Signature:** `void GetTerrainBase(Terrain* t, int b[2])`

**Purpose:** Retrieve world-space origin coordinates of the terrain quadtree root

**Called by:** asciiid.cpp (merge operations for copying terrain base offset)

**Calls:** None (direct struct access)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to caller's output array `b[]` (x at [0], y at [1])

**Notes:** WHY this function: Terrain::x and Terrain::y track the world offset of the quadtree root. This getter abstracts access for client code doing terrain copying or repositioning (e.g., Merge::CommitPatch).

---

### `SetTerrainBase` (terrain.cpp:229-233)

**Signature:** `void SetTerrainBase(Terrain* t, const int b[2])`

**Purpose:** Update world-space origin coordinates of the terrain quadtree root

**Called by:** asciiid.cpp (merge operations restoring terrain base after tile operations)

**Calls:** None (direct struct assignment)

**Globals read:** None

**Globals mutated:** Terrain::x, Terrain::y

**Side effects:** Changes spatial offset of entire quadtree (all patches reinterpreted relative to new base)

**Notes:** WHY this function: Used by Merge for offsetting terrain during copy/paste operations. Setting base does NOT move patch data, only the coordinate frame.

---

## Creation and Destruction

### `CreateTerrain` (terrain.cpp:235-302)

**Signature:** `Terrain* CreateTerrain(int z)`

**Purpose:** Allocate and initialize a terrain structure, optionally with a single uniform-height patch

**Called by:** asciiid.cpp (multiple terrain initialization contexts), game_svr.cpp, game_app.cpp, mainmenu.cpp, urdo.cpp

**Calls:** 
- `malloc()` — allocate Terrain struct
- `TexHeap::Create()` (when TEXHEAP defined)
- `TexHeap::Alloc()` (when TEXHEAP defined)

**Globals read:** None (TEXHEAP ifdef constant)

**Globals mutated:** None (returns new allocation)

**Side effects:** 
- Heap allocation of Terrain (+ TexHeap if TEXHEAP)
- If z >= 0: allocates and initializes one Patch at level 0
- Initializes height map to uniform value z, visual to 0x01 per cell, diag bitfield to 0

**Notes:** 
- **Level semantics:** z >= 0 → level 0 (single patch), z < 0 → level -1 (empty tree)
- **HEIGHT_CELLS:** 8 (vertices are 9×9 per patch)
- **VISUAL_CELLS:** 8 (material cells are 8×8 per patch)
- **TexHeap:** GPU allocation only when EDITOR defined; allocates 2 texture slots (height + visual)
- Initialization pattern matches AddTerrainPatch's single-patch bootstrap

---

### `DeleteTerrain(Node* n, int lev)` — Static Recursive Helper (terrain.cpp:304-326)

**Signature:** `static void DeleteTerrain(Node* n, int lev)`

**Purpose:** Recursively deallocate internal tree nodes and leaf patches

**Called by:** DeleteTerrain(Terrain*) only (lines 356)

**Calls:** 
- Recursive `DeleteTerrain(Node*, int)` (self-call)
- `free()` on children

**Globals read:** None

**Globals mutated:** Heap (freeing allocations)

**Side effects:** Traverses quadtree, deallocates all Nodes and Patches bottom-up

**Notes:** 
- **Termination:** When lev==1, children are Patches; when lev>1, children are Nodes
- Modeled after standard DFS tree deletion
- TexHeap cleanup is caller's responsibility (DeleteTerrain(Terrain*))

---

### `DeleteTerrain(Terrain* t)` (terrain.cpp:328-411)

**Signature:** `void DeleteTerrain(Terrain* t)`

**Purpose:** Deallocate entire terrain structure and all quadtree data

**Called by:** asciiid.cpp (terrain rebuild, cleanup), mainmenu.cpp, game_svr.cpp, game_app.cpp, game_web.cpp, physics.cpp

**Calls:** 
- `TexHeap::Destroy()` (when TEXHEAP defined, line 334)
- `free()` on root Patch (if level==0)
- `DeleteTerrain(Node*, int)` (recursive tree deletion)

**Globals read:** None

**Globals mutated:** None (caller responsible for `terrain` pointer)

**Side effects:** 
- Deallocates all Patch and Node allocations
- Destroys GPU texture heap (if TEXHEAP)
- Frees Terrain struct itself
- Caller must NULL the terrain pointer (no automatic invalidation)

**Notes:** 
- **NULL guard:** Returns early if t == NULL
- **Level-0 fast path:** Single patch is freed directly without recursion
- **Recursive path:** Delegates to DeleteTerrain(Node*, int) for multi-level trees

---

## Quadtree Navigation (Core Query Functions)

### `GetTerrainPatch(Terrain* t, int x, int y)` (terrain.cpp:557-587)

**Signature:** `Patch* GetTerrainPatch(Terrain* t, int x, int y)`

**Purpose:** Retrieve patch at world coordinates (x, y), or NULL if out of bounds

**Called by:** asciiid.cpp (editor brush operations, patch access), urdo.cpp, DelTerrainPatch, AddTerrainPatch (for neighbor queries), CalcTerrainGhost, render.cpp, physics.cpp

**Calls:** None (pure tree navigation)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (read-only query)

**Notes:** 
- **Coordinate frame:** (x, y) are world coordinates relative to Terrain::x, Terrain::y offset
- **Bounds check:** Returns NULL if outside current quadtree range [0, 1<<level)
- **Level-0 case:** Single patch terrain returns itself (or NULL if no root)
- **Tree descent:** Binary search via bit extraction at each level (i = ((x >> lev) & 1) | (((y >> lev) & 1) << 1))

---

### `GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y)` — Overload (terrain.cpp:589-615)

**Signature:** `void GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y)`

**Purpose:** Inverse query — given a Patch, return its world coordinates (x, y)

**Called by:** No callers found via grep

**Calls:** None (tree traversal only)

**Globals read:** None

**Globals mutated:** Output arrays *x, *y (if non-NULL)

**Side effects:** Modifies caller's int pointers if provided

**Notes:** 
- **Algorithm:** Ascends from patch to root, extracting quadrant bits at each level to reconstruct coordinates
- **BUG:** Line 613 checks `if (x)` twice; second should be `if (y)` (logic error in output assignment)
- **Coordinate transformation:** Returns world-relative-to-base coordinates (subtracts t->x, t->y)

---

### `UpdateNodes(Patch* p)` — Static Helper (terrain.cpp:617-644)

**Signature:** `static void UpdateNodes(Patch* p)`

**Purpose:** Propagate patch height bounds and neighbor flags up the quadtree

**Called by:** AddTerrainPatch, DelTerrainPatch (when patch flags change), UpdateTerrainHeightMap

**Calls:** None (direct struct updates)

**Globals read:** None

**Globals mutated:** Node::lo, Node::hi, Node::flags for all ancestors

**Side effects:** Walks parent pointers from patch to root, updating each ancestor's bounds and neighbor flags

**Notes:** 
- **WHY:** After creating/deleting patches or changing flags, parent nodes must be re-computed. lo/hi bounds are used for frustum culling and ray-casting acceleration.
- **Neighbor flags:** Bitwise AND of all children's flags (bit set only if ALL children have that neighbor)
- **Performance:** O(log_4 patches) — tree depth bounded by roughly log

---

## Patch Modification

### `DelTerrainPatch(Terrain* t, int x, int y)` (terrain.cpp:646-764)

**Signature:** `bool DelTerrainPatch(Terrain* t, int x, int y)`

**Purpose:** Remove patch at (x, y) from quadtree, auto-shrink tree if empty, update neighbors

**Called by:** urdo.cpp, physics.cpp (implicit via URDO_Delete)

**Calls:** 
- `GetTerrainPatch()` — find patch to delete
- `TexAlloc::Free()` (when TEXHEAP defined) — deallocate GPU texture
- `UpdateTerrainVisualMap()`, `UpdateTerrainHeightMap()` — update displaced patch's GPU data
- `free()` — deallocate patch and unused ancestor nodes
- `UpdateNodes()` — propagate bounds after neighbor changes

**Globals read:** None

**Globals mutated:** 
- Terrain::root, level, patches, nodes
- Patch parent pointers
- Neighbor flags (flags field) of surrounding patches

**Side effects:** 
- Deallocates Patch and may deallocate ancestor Nodes if they become empty
- Auto-shrinks tree root if only one child remains (decrements level, adjusts origin)
- Updates all 8 neighboring patches' flags
- TexHeap may relocate GPU data (Free() returns last texalloc which triggers update)

**Notes:** 
- **Leaf trim:** Ascends and deallocates empty ancestor Nodes
- **Root trim:** If root has only one non-NULL child after shrinking, promote that child and decrement level
- **Neighbor updates:** Sets/clears the appropriate neighbor flag bits (CCW 8-bit layout per header)
- **Return:** true if patch was found and deleted; false if patch doesn't exist

---

### `AddTerrainPatch(Terrain* t, int x, int y, int z)` (terrain.cpp:771-1303)

**Signature:** `Patch* AddTerrainPatch(Terrain* t, int x, int y, int z)`

**Purpose:** Add or retrieve a patch at (x, y) with initial height z, auto-expanding quadtree if needed

**Called by:** asciiid.cpp (editor painting, terrain generation), game_svr.cpp, urdo.cpp (undo/redo create)

**Calls:** 
- `getenv()`, `fprintf()`, `fflush()` (debug output when ASCIICKER_TERRAIN_DEBUG set)
- `malloc()` — allocate new Nodes and Patch
- `GetTerrainPatch()` — query neighboring patches (8 neighbors for height/flag sync)
- `UpdateNodes()` — propagate bounds after neighbor flag changes
- `TexHeap::Alloc()` — GPU allocation if TEXHEAP defined

**Globals read:** ASCIICKER_TERRAIN_DEBUG environment variable

**Globals mutated:** 
- Terrain::root, level, x, y, patches, nodes
- Patch parent pointers, height maps, visual maps, flags

**Side effects:** 
- Allocates new Nodes/Patches, modifies height and neighbor data
- Updates all 8 neighboring patches' neighbor flags
- Initializes height via 4 strategies: (1) copy from neighbors, (2) linear interpolation (edges), (3) inverse-distance interpolation (interior), (4) uniform fallback
- Initializes diag bitfield via Tap3x3::Update() (stores diagonal triangle orientation for each HEIGHT_CELLS² cell)
- Initializes visual to 0x00 (all cells transparent)

**Notes:** 
- **Empty terrain bootstrap:** If t->root is NULL, creates single Patch at level 0 with offset (x, y) and returns immediately
- **WHY auto-expand:** Four expand loops handle out-of-bounds coordinates:
  - x < 0: expand left (quad 0 or 2 depending on y)
  - y < 0: expand up (quad 0 or 1 depending on x)
  - x >= range: expand right (quad 2 or 3 depending on y)
  - y >= range: expand down (quad 1 or 3 depending on x)
- **WHY descend and create:** Once bounds-checked, tree descent allocates missing Nodes and finally creates the Patch
- **Height interpolation:** 
  - **Corners:** Copied from diagonal neighbors if they exist; else fallback to z
  - **Edges:** Linear interpolation between corner heights (line 1196, 1206, 1216, 1226)
  - **Interior:** Weighted average of edge samples using inverse-distance weights (lines 1238-1261)
- **Neighbor flag sync:** After patch creation, all 8 neighbors' flags are updated (mutual bidirectional marking)
- **Diag calculation:** Tap3x3 helper computes per-cell diagonal orientation for triangle mesh (diagonal faces direction with larger height gradient)

---

### `GetTerrainNeighbor(Patch* p, int dx, int dy)` (terrain.cpp:1308-1379)

**Signature:** `Patch* GetTerrainNeighbor(Patch* p, int dx, int dy)`

**Purpose:** Find neighboring patch relative to p by offset (dx, dy), using two-phase ascent-then-descend

**Called by:** Tap3x3 constructor (lines 418-426), UpdateTerrainHeightMap (indirectly via Tap3x3)

**Calls:** None (pure tree navigation)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (read-only query)

**Notes:** 
- **Phase 1 (Ascent):** Start with r=1 (patch size). Walk up tree, accumulating offset into (dx, dy). When offset falls within ancestor's spatial domain, found common ancestor.
- **Phase 2 (Descent):** From common ancestor, descend by computing quadrant indices from (dx, dy) at each level. When hr==1 (leaf level), return target patch or NULL.
- **WHY two-phase:** More efficient than recomputing absolute coordinates and searching from root
- **Return:** NULL if neighbor doesn't exist (tree boundary, no wrap-around)
- **Quadrant math:** i = (dx >= hr ? 1 : 0) | (dy >= hr ? 2 : 0) where hr is half-range at current level

---

## Query and Utility Functions

### `GetTerrainPatches(Terrain* t)` (terrain.cpp:1382-1385)

**Signature:** `int GetTerrainPatches(Terrain* t)`

**Purpose:** Return count of patches in the quadtree

**Called by:** mainmenu.cpp (UI display), asciiid.cpp

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple accessor for Terrain::patches counter. Incremented by AddTerrainPatch, decremented by DelTerrainPatch.

---

### `GetTerrainBytes(Terrain* t)` (terrain.cpp:1387-1390)

**Signature:** `size_t GetTerrainBytes(Terrain* t)`

**Purpose:** Calculate total heap memory used by terrain (Patch + Node allocations)

**Called by:** asciiid.cpp (UI memory usage display)

**Calls:** None

**Globals read:** sizeof(Patch), sizeof(Node)

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns `t->patches * sizeof(Patch) + t->nodes * sizeof(Node)`. Does NOT include TexHeap GPU memory or malloc overhead.

---

### `GetTerrainHeightMap(Patch* p)` (terrain.cpp:1393-1396)

**Signature:** `uint16_t* GetTerrainHeightMap(Patch* p)`

**Purpose:** Get pointer to height array for direct read/write access

**Called by:** asciiid.cpp (editor height manipulation), urdo.cpp, render.cpp

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Returns raw pointer (caller must call UpdateTerrainHeightMap() after modifications)

**Notes:** 
- Array dimensions: [HEIGHT_CELLS+1][HEIGHT_CELLS+1] = [9][9]
- Caller must notify terrain system via UpdateTerrainHeightMap() to propagate changes (recalc bounds, diag, update GPU)

---

### `GetTerrainVisualMap(Patch* p)` (terrain.cpp:1398-1401)

**Signature:** `uint16_t* GetTerrainVisualMap(Patch* p)`

**Purpose:** Get pointer to visual (material) array for direct read/write access

**Called by:** asciiid.cpp (editor material painting), urdo.cpp, render.cpp

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Returns raw pointer (caller must call UpdateTerrainVisualMap() after modifications)

**Notes:** 
- Array dimensions: [VISUAL_CELLS][VISUAL_CELLS] = [8][8]
- Each cell stores uint16_t with: bit0 = elevation flag, bits 1-6 = material ID
- Caller must notify via UpdateTerrainVisualMap() to upload to GPU

---

### `CalcTerrainGhost(Terrain* t, int x, int y, int z, uint16_t ghost[4*HEIGHT_CELLS])` (terrain.cpp:1403-1544)

**Signature:** `Patch* CalcTerrainGhost(Terrain* t, int x, int y, int z, uint16_t ghost[4*HEIGHT_CELLS])`

**Purpose:** Generate ghost patch boundary (4 edges) for coordinate (x, y) when patch doesn't exist, interpolating from neighbors

**Called by:** asciiid.cpp (preview visualization for non-existent patches)

**Calls:** 
- `GetTerrainPatch()` — check if patch exists (fast path)
- `GetTerrainPatch()` — query all 8 neighbors

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to caller's ghost array (4 * HEIGHT_CELLS = 32 uint16_t values)

**Notes:** 
- **Return:** Returns existing patch if it exists; returns NULL if generated ghost
- **Output format:** ghost[] stores perimeter heights in CCW order:
  - [0..HEIGHT_CELLS): bottom edge (y=0)
  - [HEIGHT_CELLS..2*HEIGHT_CELLS): right edge (x=HEIGHT_CELLS)
  - [2*HEIGHT_CELLS..3*HEIGHT_CELLS): top edge reversed (y=HEIGHT_CELLS)
  - [3*HEIGHT_CELLS..4*HEIGHT_CELLS): left edge reversed (x=0)
- **Interpolation strategy:** Mirror of AddTerrainPatch:
  - Copy corners from diagonal neighbors if present
  - Linear interpolation for edges
  - Inverse-distance weighted interior
  - Fallback to uniform z if no neighbors

---

### `GetTerrainLimits(Patch* p, uint16_t* lo, uint16_t* hi)` (terrain.cpp:1547-1553)

**Signature:** `void GetTerrainLimits(Patch* p, uint16_t* lo, uint16_t* hi)`

**Purpose:** Retrieve min/max height bounds for a patch

**Called by:** asciiid.cpp

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to output pointers if non-NULL

**Notes:** Simple accessor for Patch::lo and Patch::hi. These are maintained by UpdateTerrainHeightMap().

---

## Update Functions (GPU and Bounds)

### `UpdateTerrainHeightMap(Patch* p)` (terrain.cpp:1556-1579)

**Signature:** `void UpdateTerrainHeightMap(Patch* p)`

**Purpose:** Recalculate height bounds and diagonal orientation, upload to GPU

**Called by:** asciiid.cpp (after height edits), urdo.cpp, DelTerrainPatch

**Calls:** 
- Loop recalculating min/max heights
- `Tap3x3::Update()` — recalculate diagonal bitfield
- `TexAlloc::Update()` — GPU upload (when TEXHEAP defined)
- `UpdateNodes()` — propagate bounds to ancestors

**Globals read:** None

**Globals mutated:** Patch::lo, Patch::hi, Patch::diag, GPU texture (via TexHeap)

**Side effects:** 
- Scans all HEIGHT_CELLS² vertices to update bounds
- Recalculates diagonal orientation for all cells
- Uploads height data to GPU (TexHeap slot 0)
- Updates ancestor bounds

**Notes:** 
- **MUST call after:** GetTerrainHeightMap() direct modifications
- **Diag recalculation:** Tap3x3::Update() uses neighbor height gradients to determine triangle diagonal orientation per cell

---

### `UpdateTerrainVisualMap(Patch* p)` (terrain.cpp:1581-1587)

**Signature:** `void UpdateTerrainVisualMap(Patch* p)`

**Purpose:** Upload visual (material) data to GPU

**Called by:** asciiid.cpp (after material edits), urdo.cpp, DelTerrainPatch

**Calls:** 
- `TexAlloc::Update()` — GPU upload (when TEXHEAP defined)

**Globals read:** None

**Globals mutated:** GPU texture (via TexHeap)

**Side effects:** Uploads visual data to GPU (TexHeap slot 1 only; does NOT recalc bounds or update ancestors)

**Notes:** 
- **MUST call after:** GetTerrainVisualMap() direct modifications
- **GPU-only operation:** Unlike UpdateTerrainHeightMap, this does NOT recalculate any CPU-side data
- **Note in header:** Line 1575 comment emphasizes "ONLY HEIGHT !!!" for height update; line 1585 "ONLY VISUAL !!!'"

---

### `GetTerrainHi(Patch* p, uint16_t* lo)` (terrain.cpp:1601-1606)

**Signature:** `uint16_t GetTerrainHi(Patch* p, uint16_t* lo)`

**Purpose:** Get max height and optionally return min height

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to *lo if non-NULL

**Notes:** Combined getter for bounds; unusual parameter order (lo returned via pointer, hi via return value).

---

### `GetTerrainDiag(Patch* p)` (terrain.cpp:1608-1611)

**Signature:** `uint16_t GetTerrainDiag(Patch* p)`

**Purpose:** Retrieve diagonal bitfield (triangle orientation per cell)

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Bitfield: bit i set means cell i has diagonal 0→1; clear means diagonal 0→2.

---

### `SetTerrainDiag(Patch* p, uint16_t diag)` (terrain.cpp:1613-1616)

**Signature:** `void SetTerrainDiag(Patch* p, uint16_t diag)`

**Purpose:** Directly set diagonal bitfield

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** Patch::diag

**Side effects:** Overwrites diagonal data (does NOT trigger GPU update; caller responsible)

**Notes:** Caller should call UpdateTerrainHeightMap() afterward to upload to GPU.

---

### `GetTerrainDark(Patch* p)` — DARK_TERRAIN only (terrain.cpp:1619-1622)

**Signature:** `uint64_t GetTerrainDark(Patch* p)`

**Purpose:** Retrieve shadow/occlusion bitfield (when DARK_TERRAIN compiled)

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** 64 bits for 8×8 visual cells (1 bit per cell) storing shadow/occlusion state.

---

### `SetTerrainDark(Patch* p, uint64_t dark)` — DARK_TERRAIN only (terrain.cpp:1624-1627)

**Signature:** `void SetTerrainDark(Patch* p, uint64_t dark)`

**Purpose:** Set shadow/occlusion bitfield (when DARK_TERRAIN compiled)

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** Patch::dark

**Side effects:** Overwrites occlusion data

**Notes:** Paired with SetTerrainDiag for full patch restoration during undo/redo.

---

## Additional Accessors (Texture Heap, TexHeap only)

### `GetTerrainTexHeap(Terrain* t)` — TEXHEAP only (terrain.cpp:1590-1593)

**Signature:** `TexHeap* GetTerrainTexHeap(Terrain* t)`

**Purpose:** Return TexHeap pointer for GPU texture management

**Called by:** No callers found via grep (used by render system for batch GPU uploads)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** TexHeap handles GPU memory allocation/deallocation for height and visual textures.

---

### `GetTerrainTexAlloc(Patch* p)` — TEXHEAP only (terrain.cpp:1595-1598)

**Signature:** `TexAlloc* GetTerrainTexAlloc(Patch* p)`

**Purpose:** Get TexAlloc handle for individual patch's GPU texture

**Called by:** No callers found via grep (render system uses for GPU queries)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Each patch has one TexAlloc managing two texture slots (height + visual).

---

## Internal Helper: Tap3x3 (terrain.cpp:413-555)

Helper class for height map analysis (boundary handling, diagonal orientation calculation).

### `Tap3x3(Patch* c)` — Constructor (terrain.cpp:415-427)

**Signature:** `Tap3x3(Patch* c)`

**Purpose:** Initialize 3×3 tap of neighboring patches around central patch c

**Called by:** AddTerrainPatch (line 1282), UpdateTerrainHeightMap (line 1570)

**Calls:** `GetTerrainNeighbor()` 8 times (one per neighbor)

**Globals read:** None

**Globals mutated:** p[3][3] member array

**Side effects:** Loads 9-patch grid (center + 8 neighbors) from quadtree

**Notes:** 
- Central patch at p[1][1]
- Neighbors arranged CCW starting from NW: p[0][0] (NW), p[0][1] (N), p[0][2] (NE), etc.
- Neighbors is NULL (boundary patches)

---

### `Tap3x3::SetDiag(int x, int y, bool d)` (terrain.cpp:429-468)

**Signature:** `void SetDiag(int x, int y, bool d)`

**Purpose:** Set diagonal flag for cell (x, y) in center or neighbor patches

**Called by:** Tap3x3::Update (line 549)

**Calls:** None (direct struct update)

**Globals read:** None

**Globals mutated:** Patch::diag of affected patch

**Side effects:** Modifies diag bitfield of appropriate patch (is center or neighbor)

**Notes:** 
- **Boundary handling:** If (x, y) falls outside center patch, translates to neighbor patch coordinate space
- **Graceful out-of-bounds:** If neighbor doesn't exist, silently skips (lines 465-467 have empty else with int a=0 debug marker)

---

### `Tap3x3::Sample(int x, int y)` (terrain.cpp:470-517)

**Signature:** `int Sample(int x, int y)`

**Purpose:** Query height at (x, y) with boundary handling (may read from neighbors or use clamped values)

**Called by:** Tap3x3::Update (lines 526-547)

**Calls:** None (direct height access)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** 
- **Boundary clamping:** If coordinate falls outside [0, HEIGHT_CELLS], adjusts to neighbor patch and re-projects
- **Known TODO (line 480, 492):** Boundary condition uses `>` instead of `>=` (comment says "assuming '>' is fresher" — needs verification)
- **NULL safety:** If neighbor doesn't exist, clamps to edge (line 500-514)

---

### `Tap3x3::Update()` (terrain.cpp:519-552)

**Signature:** `void Update()`

**Purpose:** Recalculate diagonal bitfield for all cells using height gradient comparison

**Called by:** AddTerrainPatch (line 1283), UpdateTerrainHeightMap (line 1571)

**Calls:** 
- `Sample()` — query 10 heights per cell
- `SetDiag()` — store diagonal flag
- `my_abs()` — compare gradient magnitudes

**Globals read:** None

**Globals mutated:** Diag bitfields of center and potentially neighboring patches

**Side effects:** Updates diagonal orientation for cells spanning boundary (may write to neighbor patches)

**Notes:** 
- **Algorithm:** For each cell, compute two gradient magnitudes (diagonal 0 vs. diagonal 1)
- **Formula (line 525-547):** 
  - c0: weighted sum of height differences (diagonal 0-3 orientation)
  - c1: weighted sum of height differences (diagonal 1-2 orientation)
  - SetDiag(x, y, |c0| > |c1|) — higher magnitude wins
- **Purpose:** Ensures mesh normals align with height field principal curvature
- **Loop range:** -1 to HEIGHT_CELLS in both x and y (includes boundary)

---

### `QueryTerrainSample(Patch*, x, y, cb)` (terrain.cpp:1630-1691)

**Signature:** `void QueryTerrainSample(Patch* p, int x, int y, void(*cb)(Patch* p, int u, int v, double coords[3], void* cookie), void* cookie)`

**Purpose:** Sample height at center of each visual cell (8×8 grid), invoking callback with interpolated 3D coordinates for shadow/occlusion calculations

**Called by:**
- QueryTerrainSample(QuadItem*, ...) overload (terrain.cpp:1709) — leaf dispatch
- UpdateTerrainDark (terrain.cpp:1757, 1764) — per-patch shadow computation via DarkUpdater callback

**Calls:** None (pure computation, invokes user callback)

**Globals read:**
- VISUAL_CELLS (const 8)
- HEIGHT_CELLS (const 5)

**Globals mutated:** None (callback determines side effects)

**Side effects:** Invokes callback 64 times (once per visual cell) with interpolated height coordinates

**Notes:**
- **Grid sampling:** Centers of visual cells mapped to height field via bilinear interpolation
- **Height interpolation:** Uses p->diag bitfield to determine triangle split per cell
- **Two triangle cases:**
  - rot (diag bit set): diagonal from (hx, hy) to (hx+1, hy+1)
  - !rot (diag bit clear): diagonal from (hx, hy+1) to (hx+1, hy)
- **Callback signature:** cb(p, u, v, coords, cookie) where coords = {x + u + 0.5, y + v + 0.5, interpolated_height}
- **Purpose:** Enables per-cell shadow raycasting by providing sample points across patch surface
- Line 1671 contains condition `u < y` where `y` parameter is out of scope in that context

---

### `QueryTerrainSample(QuadItem*, x, y, range, cb)` (terrain.cpp:1693-1710)

**Signature:** `void QueryTerrainSample(QuadItem* q, int x, int y, int range, void(*cb)(Patch* p, int u, int v, double coords[3], void* cookie), void* cookie)`

**Purpose:** Recursively traverse quadtree, invoking per-patch QueryTerrainSample for all leaf patches in tree

**Called by:**
- Recursively: self (lines 1700, 1702, 1704, 1706)
- UpdateTerrainDark (terrain.cpp:1764) — full-terrain shadow update dispatch

**Calls:**
- Recursively: QueryTerrainSample(QuadItem*, ...) on 4 children
- QueryTerrainSample(Patch*, ...) at leaf level (line 1709)

**Globals read:** VISUAL_CELLS (const 8)

**Globals mutated:** None (callback determines side effects)

**Side effects:** Invokes callback for every visual cell in every patch in subtree

**Notes:**
- **Base case:** range == VISUAL_CELLS → cast to Patch* and call patch-level overload
- **Recursive case:** range > VISUAL_CELLS → descend into 4 children
- **Quadrant order:** [0] at (x, y), [1] at (x+range, y), [2] at (x, y+range), [3] at (x+range, y+range)
- **Purpose:** Provides full-terrain sample iteration for shadow/lighting calculations
- **Total callback invocations:** 64 × num_patches (VISUAL_CELLS² per patch)

---

### `UpdateNodes` (terrain.cpp:617-644)

**Signature:** `static void UpdateNodes(Patch* p)`

**Purpose:** Propagate height bounds and flags up the quadtree after patch modification

**Called by:**
- AddTerrainPatch (line 1128)
- ModifyTerrain (line 1285)
- MergeTerrain (line 1578)
- terrain.cpp:3052

**Calls:** None (pure traversal logic)

**Globals read:** None

**Globals mutated:**
- Parent nodes' lo/hi bounds (propagated from children)
- Parent nodes' flags (bitwise AND of children flags)

**Side effects:** Updates all ancestor nodes' cached bounds and flags from patch to root

**Notes:** Maintains quadtree spatial invariant: each node's bounds are the union of its children's bounds. Flags are combined via bitwise AND (all children must have flag set for parent to inherit it).

---

### `GetTerrainTexHeap` (terrain.cpp:1590-1593)

**Signature:** `TexHeap* GetTerrainTexHeap(Terrain* t)`

**Purpose:** Retrieve pointer to the terrain's texture heap for atlas management

**Called by:** asciiid.cpp (terrain texture allocation operations)

**Calls:** None (direct struct access)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (returns pointer to existing heap)

**Notes:** Simple accessor wrapping t->th. Compiled only when DARK_TERRAIN is defined.

---

### `GetTerrainTexAlloc` (terrain.cpp:1595-1598)

**Signature:** `TexAlloc* GetTerrainTexAlloc(Patch* p)`

**Purpose:** Retrieve pointer to the patch's texture allocation descriptor

**Called by:** asciiid.cpp (patch texture queries)

**Calls:** None (direct struct access)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (returns pointer to existing allocator)

**Notes:** Simple accessor wrapping p->ta. Compiled only when DARK_TERRAIN is defined.

---

### `GetTerrainDark` (terrain.cpp:1619-1622)

**Signature:** `uint64_t GetTerrainDark(Patch* p)`

**Purpose:** Retrieve the 64-bit darkness/shadow bitmask for the patch (DARK_TERRAIN feature)

**Called by:** asciiid.cpp (lighting visualization, shadow editing)

**Calls:** None (direct struct access)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (read-only accessor)

**Notes:** Only compiled when DARK_TERRAIN is defined. Returns bitmask where each bit represents shadow state for a cell.

---

### `SetTerrainDark` (terrain.cpp:1624-1627)

**Signature:** `void SetTerrainDark(Patch* p, uint64_t dark)`

**Purpose:** Update the 64-bit darkness/shadow bitmask for the patch (DARK_TERRAIN feature)

**Called by:** asciiid.cpp (lighting updates, shadow painting)

**Calls:** None (direct struct mutation)

**Globals read:** None

**Globals mutated:** p->dark (patch shadow bitmask)

**Side effects:** Modifies patch's shadow state; requires terrain re-render to visualize changes

**Notes:** Only compiled when DARK_TERRAIN is defined. Each bit in bitmask corresponds to a cell's shadow state (1=shadowed, 0=lit).

---

---

