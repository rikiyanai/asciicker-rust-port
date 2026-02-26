# world.cpp Part 2: Raycast Dispatch Functions (Lines 2147-4291)

## Overview
This section documents eight static raycast dispatch functions (`HitWorld3`-`HitWorld7` and related query functions) that implement orientation-specific bounding box clipping for the Plucker-based ray-instance intersection system. These functions are dispatched from `HitWorld()` based on the sign of the ray direction vector components.

---

### `HitWorld3` (world.cpp:2218-2363)

**Signature:**
```cpp
static Inst* HitWorld3(BSP* q, double ray[10], double ret[3], double nrm[3],
                        bool positive_only, bool editor, bool solid_only,
                        bool sprites_too, uint8_t* out_color)
```

**Purpose:**
Recursively traverses BSP tree and flat instance list performing ray-box intersection tests. Dispatched when ray direction satisfies `v[1]>=0` and `v[2]>=0` (sign_case bit pattern matching). Uses sign-specific plane inequalities to reject bbox nodes early.

**Called by:**
- `HitWorld()` (line 3016, via function pointer array `func_vect[3]`)
- Recursive self-calls (lines 2277-2278 for BSP_NODE children; lines 2287-2288 for BSP_NodeShare children)

**Calls:**
- `HitWorld3()` (recursive, lines 2277-2278, 2287-2288)
- `MeshInst::HitFace()` (line 2249)
- `SpriteInst::Hit()` (line 2257)
- `ItemInst::Hit()` (line 2265)

**Globals read:**
- None directly (BSP tree traversal via pointer parameter)

**Globals mutated:**
- None directly (output written to `ret[]`, `nrm[]`, `out_color`)

**Side effects:**
- Updates `ret[3]` with closest intersection t-distance (ray parameter)
- Updates `nrm[3]` with surface normal at intersection
- Updates `out_color` with material ID if `out_color` pointer non-null
- Returns first non-null instance hit (closest by traversal order, not guaranteed closest t)

**Notes:**
- Plane equations hardcoded (lines 2232-2237) based on empirically determined ray direction octant
- Identical structure to `HitWorld0`, `HitWorld1`, `HitWorld2`, `HitWorld4`-`HitWorld7` with different plane inequalities per octant
- Editor vs. game filtering via `INST_VOLATILE` flag (lines 2243-2245)
- `positive_only` comment (lines 2227-2230) indicates future optimization for rays starting above geometry (reflection raycasts)

---

### `HitWorld4` (world.cpp:2365-2510)

**Signature:**
```cpp
static Inst* HitWorld4(BSP* q, double ray[10], double ret[3], double nrm[3],
                        bool positive_only, bool editor, bool solid_only,
                        bool sprites_too, uint8_t* out_color)
```

**Purpose:**
Ray-BSP intersection for octant 4 (sign_case=4): `v[0]>=0`, `v[1]<0`, `v[2]>=0`. Plane inequalities account for this direction sign combination to reject non-intersecting bboxes efficiently.

**Called by:**
- `HitWorld()` (line 3016, via function pointer array `func_vect[4]`)
- Recursive self-calls (lines 2424-2425 for BSP_NODE; lines 2434-2435 for BSP_NodeShare)

**Calls:**
- `HitWorld4()` (recursive, lines 2424-2425, 2434-2435)
- `MeshInst::HitFace()` (line 2396)
- `SpriteInst::Hit()` (line 2404)
- `ItemInst::Hit()` (line 2412)

**Globals read:**
- None directly

**Globals mutated:**
- None directly

**Side effects:**
- Same as `HitWorld3`: updates `ret[]`, `nrm[]`, `out_color` with closest hit

**Notes:**
- Plane equations (lines 2379-2384) differ from octant 3, derived from octant-specific geometry
- Identical traversal and filtering logic to all other `HitWorldN` variants

---

### `HitWorld5` (world.cpp:2512-2657)

**Signature:**
```cpp
static Inst* HitWorld5(BSP* q, double ray[10], double ret[3], double nrm[3],
                        bool positive_only, bool editor, bool solid_only,
                        bool sprites_too, uint8_t* out_color)
```

**Purpose:**
Ray-BSP intersection for octant 5 (sign_case=5): `v[0]<0`, `v[1]>=0`, `v[2]>=0`.

**Called by:**
- `HitWorld()` (line 3016, via function pointer array `func_vect[5]`)
- Recursive self-calls (lines 2571-2572 for BSP_NODE; lines 2581-2582 for BSP_NodeShare)

**Calls:**
- `HitWorld5()` (recursive, lines 2571-2572, 2581-2582)
- `MeshInst::HitFace()` (line 2543)
- `SpriteInst::Hit()` (line 2551)
- `ItemInst::Hit()` (line 2559)

**Globals read:**
- None directly

**Globals mutated:**
- None directly

**Side effects:**
- Updates intersection result (`ret[]`, `nrm[]`, `out_color`)

**Notes:**
- Plane equations (lines 2526-2531) for octant 5 ray direction
- Structurally identical to other octant functions; variation is plane inequality coefficients only

---

### `HitWorld6` (world.cpp:2659-2804)

**Signature:**
```cpp
static Inst* HitWorld6(BSP* q, double ray[10], double ret[3], double nrm[3],
                        bool positive_only, bool editor, bool solid_only,
                        bool sprites_too, uint8_t* out_color)
```

**Purpose:**
Ray-BSP intersection for octant 6 (sign_case=6): `v[0]>=0`, `v[1]<0`, `v[2]<0`.

**Called by:**
- `HitWorld()` (line 3016, via function pointer array `func_vect[6]`)
- Recursive self-calls (lines 2718-2719 for BSP_NODE; lines 2728-2729 for BSP_NodeShare)

**Calls:**
- `HitWorld6()` (recursive, lines 2718-2719, 2728-2729)
- `MeshInst::HitFace()` (line 2690)
- `SpriteInst::Hit()` (line 2698)
- `ItemInst::Hit()` (line 2706)

**Globals read:**
- None directly

**Globals mutated:**
- None directly

**Side effects:**
- Updates intersection result

**Notes:**
- Plane equations (lines 2673-2678) for octant 6
- Continues pattern of identical structure with octant-specific plane inequalities

---

### `HitWorld7` (world.cpp:2806-2951)

**Signature:**
```cpp
static Inst* HitWorld7(BSP* q, double ray[10], double ret[3], double nrm[3],
                        bool positive_only, bool editor, bool solid_only,
                        bool sprites_too, uint8_t* out_color)
```

**Purpose:**
Ray-BSP intersection for octant 7 (sign_case=7): `v[0]<0`, `v[1]<0`, `v[2]<0`.

**Called by:**
- `HitWorld()` (line 3016, via function pointer array `func_vect[7]`)
- Recursive self-calls (lines 2865-2866 for BSP_NODE; lines 2875-2876 for BSP_NodeShare)

**Calls:**
- `HitWorld7()` (recursive, lines 2865-2866, 2875-2876)
- `MeshInst::HitFace()` (line 2837)
- `SpriteInst::Hit()` (line 2845)
- `ItemInst::Hit()` (line 2853)

**Globals read:**
- None directly

**Globals mutated:**
- None directly

**Side effects:**
- Updates intersection result

**Notes:**
- Plane equations (lines 2820-2825) for octant 7 (all ray directions negative)
- Final octant function in dispatch set

---

### `HitWorld` (world.cpp:2955-3018)

**Signature:**
```cpp
Inst* HitWorld(double p[3], double v[3], double ret[3], double nrm[3],
               bool positive_only, bool editor, bool solid_only,
               bool sprites_too, uint8_t* out_color = 0)
```

**Purpose:**
Public raycast entry point. Converts ray origin and direction to Plucker coordinates, determines octant via ray direction sign bits, and dispatches to appropriate `HitWorldN()` function. Returns first instance hit by ray.

**Called by:**
- Public raycast entry; dispatcher for octant-specific functions

**Calls:**
- `HitWorld0()` through `HitWorld7()` (line 3016, via function pointer array indexed by `sign_case`)

**Globals read:**
- `root` (BSP tree root pointer, line 2957)

**Globals mutated:**
- None directly (output via reference parameters)

**Side effects:**
- Computes Plucker ray representation (lines 2972-2980) for all octant functions
- Determines ray direction octant (lines 2982-2989)
- Dispatches to octant-specific plane inequality handler

**Notes:**
- Plucker representation: `ray[0..2]` = cross product `p × v`, `ray[3..5]` = direction `v`, `ray[6..8]` = origin `p`, `ray[9]` = max t-distance (initialized to `FLT_MAX`)
- `positive_only` optimization commented out (lines 2960-2969); intended for reflection raycasts where ray starts above geometry
- Function pointer array `func_vect[]` (lines 2993-3004) dispatches based on 3-bit sign case (8 octants)
- Null check on `root` handles empty worlds (line 2957-2958)

---

### `DeleteItemInsts` (world.cpp:3351-3420)

**Signature:** `static void DeleteItemInsts(BSP* bsp, bool all)`

**Purpose:** Recursively traverses BSP tree and collects ItemInst items into `delete_item_list` linked list for deletion. Filters by item purpose unless `all` is true.

**Called by:** `DeleteWorld()` (lines 4461, 4465), recursive self-calls on BSP children

**Calls:** Recursive `DeleteItemInsts()` for BSP child nodes

**Globals read:** `delete_item_list` (global linked list head, line 3363)

**Globals mutated:** `delete_item_list` (prepends items to list via linked list manipulation: `item->proto = (ItemProto*)delete_item_list; delete_item_list = item;`)

**Side effects:** Builds linked list of items to delete using `item->proto` field as next pointer (abusing proto for list linking during deletion).

**Notes:** Collects items instead of deleting directly to avoid corruption during BSP traversal. `delete_item_list` global reused as temporary linked list storage (cleared before use by caller). Purpose filtering: only `Item::WORLD` items collected unless `all=true`. Traverses all 4 BSP node types.

---

### `DeleteSpriteInsts` (world.cpp:3422-3479)

**Signature:** `static void DeleteSpriteInsts(BSP* bsp)`

**Purpose:** Recursively traverses BSP tree and collects SpriteInst instances into `delete_sprite_list` linked list for deletion.

**Called by:** `DeleteWorld()` (lines 4476, 4478), recursive self-calls on BSP children

**Calls:** Recursive `DeleteSpriteInsts()` for BSP child nodes

**Globals read:** `delete_sprite_list` (global linked list head, line 3432)

**Globals mutated:** `delete_sprite_list` (prepends SpriteInsts using `si->sprite` field as next pointer)

**Side effects:** Builds linked list of sprite instances to delete using `SpriteInst::sprite` field as next pointer.

**Notes:** Similar pattern to `DeleteItemInsts` but for sprite instances. `si->sprite` reused as next pointer during deletion (cleared before use). No instance filtering; collects all sprite instances regardless of flags.

---

### `CloneItemInsts` (world.cpp:3484-3561)

**Signature:** `static void CloneItemInsts(World* w, BSP* bsp)`

**Purpose:** Recursively traverses BSP tree and clones all `Item::EDIT` purpose items into new `Item::WORLD` purpose items in target world. Used to copy editor items to runtime world.

**Called by:** `ResetItemInsts()` (lines 3583-3585, 3597-3599)

**Calls:** `CreateItem()`, `memcpy()`, `CreateInst()`, recursive `CloneItemInsts()`

**Globals read:** `delete_item_list` (global, read by caller context)

**Globals mutated:** None directly (modifies target world `w`)

**Side effects:** Allocates new Item structs via `CreateItem`, copies instance data, creates new instances in target world via `CreateInst()`. New items have `purpose = Item::WORLD`.

**Notes:** Pattern: Copy `Item` struct → set `purpose=WORLD` → `inst=0` → call `CreateInst()` to create instance and link to world. Used when transitioning from editor mode to game mode. Recursive for all 4 BSP node types. Does not handle flat list (handled by caller).

---

### `ResetItemInsts` (world.cpp:3564-3609)

**Signature:** `void ResetItemInsts(World* w)`

**Purpose:** Destroys all existing `Item::WORLD` items in world, then clones all `Item::EDIT` items from BSP tree and flat list to restore world state. Used to reset world items to editor template.

**Called by:** No callers found via grep in repository.

**Calls:** `DeleteItemInsts()`, `CloneItemInsts()`, `DestroyItem()`

**Globals read:** `delete_item_list` (global)

**Globals mutated:** `delete_item_list` (used as temporary list, cleared after use)

**Side effects:** Removes all world items from BSP tree and flat list, deallocates them, then recreates fresh instances from EDIT items.

**Notes:** Two-phase process: (1) Delete all WORLD items via `DeleteItemInsts(w->root, false)` → iterate `delete_item_list` → `DestroyItem()` each; (2) Clone EDIT items: flat list walk → BSP tree recursion via `CloneItemInsts()`. Global `delete_item_list` reused in both phases (cleared before second phase at line 3566). Used to reset level state without reloading entire world file.

---

### `Mesh::Update` (world.cpp:3619-4389)

**Signature:** `bool Mesh::Update(const char* path)`

**Purpose:** Reloads mesh geometry from .akm file (PLY format), deallocating previous vertices/faces/lines.

**Called by:** `UpdateMesh()` wrapper function (line 4505)

**Calls:** File I/O (`fopen`, `fread`), PLY parsing, vertex/face allocation, linked list operations

**Globals read:** `mesh->world` pointer (self-check), mesh's own geometry lists

**Globals mutated:** `mesh` struct (verts, faces, lines, bbox)

**Side effects:** Deallocates previous geometry; reads .akm file; rebuilds vertex/face/line linked lists; recomputes bbox.

**Notes:** Called when editor reloads mesh after external modification. Full geometry replacement, not incremental. Parses PLY format with custom .akm extensions (vertex colors, materials). Returns false on file read errors. Updates `mesh->verts`, `mesh->lines`, `mesh->faces` linked lists. Handles both ASCII and binary PLY formats. Sets `mesh->bbox` from vertex extents.

---

### `World::LoadMesh` (world.cpp:4392-4403)

**Signature:** `Mesh* World::LoadMesh(const char* path, const char* name)`

**Purpose:** Loads mesh from .akm file, creating new mesh node and linking to world's mesh library.

**Called by:** `LoadMesh()` wrapper (line 4498)

**Calls:** `AddMesh()`, `Mesh::Update()`, `SetMeshCookie()`

**Globals read:** None (uses `this` pointer)

**Globals mutated:** None (delegates to AddMesh)

**Side effects:** Allocates new Mesh struct via `AddMesh()`, loads geometry via `Mesh::Update()`, sets name via `SetMeshCookie()`.

**Notes:** Convenience wrapper combining `AddMesh()` + `Mesh::Update()`. Returns null on failure. Name stored as cookie for external reference. Mesh automatically linked to world's `head_mesh`/`tail_mesh` doubly-linked list.

---

### `CreateWorld` (world.cpp:4406-4420)

**Signature:** `World* CreateWorld()`

**Purpose:** Allocates and initializes a new empty World structure with all counters and pointers set to zero.

**Called by:** No callers found via grep in repository.

**Calls:** `calloc()` (line 4408)

**Globals read:** None

**Globals mutated:** None (allocates new world)

**Side effects:** Allocates World struct with zeroed state; returns null on allocation failure. Initialization: `head_mesh`, `tail_mesh`, `insts`, `temp_insts`, `head_inst`, `tail_inst`, `editable`, `root` all set to 0.

**Notes:** World must be populated via `AddMesh()`, `AddInst()`, and `Rebuild()` before use for spatial queries. No automatic mesh loading; caller must load meshes explicitly.

---

### `DeleteWorld` (world.cpp:4424-4495)

**Signature:** `void DeleteWorld(World* w)`

**Purpose:** Frees all memory associated with a World, including meshes, instances, and BSP trees. Handles both flat list and BSP tree cleanup.

**Called by:** Application shutdown, level unload

**Calls:** `DeleteItemInsts()`, `DeleteSpriteInsts()`, `DeleteBSP()`, `DelMesh()`, `DestroyItem()`, `DeleteInst()`, `free()`

**Globals read:** `delete_item_list`, `delete_sprite_list` (globals used as temporary storage)

**Globals mutated:** `delete_item_list`, `delete_sprite_list` (initialized, used, then deleted)

**Side effects:** Destroys all instances (mesh, sprite, item), deallocates BSP tree, deallocates all meshes, frees World struct.

**Notes:** Three-phase cleanup: (1) Collect and delete items from flat list → BSP tree; (2) Collect and delete sprite instances from flat list → BSP tree; (3) Delete BSP tree and all meshes. Uses global `delete_item_list` and `delete_sprite_list` as temporary linked lists to avoid corruption during recursive deletion. Mesh deletion cascades to all sharing instances via `DelMesh()`.

---

### `LoadMesh` (world.cpp:4498-4502)

**Signature:** `Mesh* LoadMesh(World* w, const char* path, const char* name)`

**Purpose:** Convenience wrapper that loads mesh from file into world.

**Called by:** External code via world.h API

**Calls:** `World::LoadMesh()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Delegates to `World::LoadMesh()` method.

**Notes:** Simple indirection wrapper; validates null world before calling method.

---

### `UpdateMesh` (world.cpp:4505-4507)

**Signature:** `bool UpdateMesh(Mesh* m, const char* path)`

**Purpose:** Reloads mesh geometry from file; used for live-reloading edited meshes in editor.

**Called by:** Editor reload commands

**Calls:** `Mesh::Update()`

**Globals read:** None

**Globals mutated:** `m->verts`, `m->lines`, `m->faces`, `m->bbox`

**Side effects:** Replaces mesh geometry; updates bbox.

**Notes:** Wrapper for `Mesh::Update()` method. Returns false on failure (file not found or parse error). Does not delete shared instances; they continue referencing updated mesh.

---

### `DeleteMesh` (world.cpp:4510-4515)

**Signature:** `void DeleteMesh(Mesh* m)`

**Purpose:** Removes mesh from its world, deallocating geometry and all sharing instances.

**Called by:** Editor delete mesh commands, world cleanup

**Calls:** `World::DelMesh()`

**Globals read:** None

**Globals mutated:** World's mesh list (via `DelMesh()`)

**Side effects:** Cascading deletion: removes all instances sharing this mesh, frees mesh geometry, unlinks from world's mesh list.

**Notes:** Wrapper for `World::DelMesh()` method. Invalidates any pointers to sharing instances. Mesh ownership verified by `m->world` check inside `DelMesh()`.

---

### `CreateInst(Mesh*)` (world.cpp:4518-4523)

**Signature:** `Inst* CreateInst(Mesh* m, int flags, const double tm[16], const char* name, int story_id)`

**Purpose:** Creates mesh instance from shared mesh with transform matrix.

**Called by:** External code via world.h API

**Calls:** `m->world->AddInst(m, flags, tm, name, story_id)`

**Globals read:** `m->world`

**Globals mutated:** World's instance lists (via `AddInst()`)

**Side effects:** Allocates MeshInst, links to world, adds to mesh's share list.

**Notes:** Validates mesh ownership; if `tm` is null or mesh has no world, can't create instance. Instance linked to both world's flat list and mesh's share list for cascading delete.

---

### `CreateInst(Sprite*)` (world.cpp:4526-4531)

**Signature:** `Inst* CreateInst(World* w, Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], const char* name, int story_id)`

**Purpose:** Creates sprite billboard instance with animation state.

**Called by:** External code via world.h API

**Calls:** `w->AddInst(s, flags, pos, yaw, anim, frame, reps, name, story_id)`

**Globals read:** None

**Globals mutated:** World's instance lists (via `AddInst()`)

**Side effects:** Allocates SpriteInst, computes bbox from sprite projection, links to world's flat list.

**Notes:** Position, yaw, animation state stored directly. `reps[4]` array stores animation repetition counts for playback control. Instance NOT in BSP tree initially (must call `AttachInst()` or `Rebuild()`).

---

### `CreateInst(Item*)` (world.cpp:4534-4538)

**Signature:** `Inst* CreateInst(World* w, Item* item, int flags, float pos[3], float yaw, int story_id)`

**Purpose:** Creates item instance (world item, NPC, pickup) from Item prototype.

**Called by:** External code via world.h API

**Calls:** `w->AddInst(item, flags, pos, yaw, story_id)`

**Globals read:** None

**Globals mutated:** World's instance lists (via `AddInst()`)

**Side effects:** Allocates ItemInst from free pool, links to world's flat list.

**Notes:** Item instances use pooled allocator (`AllocItemInst()`). `INST_VOLATILE` flag distinguishes editor items (temporary) from runtime items (persistent). Instance rendered as sprite via `item->proto->sprite_3d`.

---

### `DeleteInst` (world.cpp:4541-4552)

**Signature:** `void DeleteInst(Inst* i)`

**Purpose:** Removes instance from world, unlinks from BSP tree/flat list, deallocates memory or returns to pool.

**Called by:** External code via world.h API

**Calls:** `I->DelInst()` (polymorphic dispatch to `World::DelInst()` method)

**Globals read:** `I->w` (world pointer from instance)

**Globals mutated:** Removes instance from world's lists/bounds

**Side effects:** Type-specific cleanup: mesh instances unlinked from share list, sprite instances freed, item instances returned to pool.

**Notes:** Wrapper that extracts `i->w` (instance's world) and calls appropriate `DeleteInst` overload. Invalidates instance pointer; caller must not use after call.

---

### `QueryWorld` (world.cpp:4555-4563)

**Signature:** `void QueryWorld(World* w, int planes, double plane[][4], QueryWorldCB* cb, void* cookie)`

**Purpose:** Public API for BSP tree traversal with frustum culling; calls callback for each visible instance.

**Called by:** Renderer frustum culling, physics queries

**Calls:** `World::Query(planes, plane, cb, cookie)`

**Globals read:** None

**Globals mutated:** BSP query counters (`bsp_tests`, `bsp_insts`, `bsp_nodes`) reset before query

**Side effects:** Invokes mesh/sprite callbacks for all instances passing frustum tests.

**Notes:** Wrapper for `World::Query()` method. Callback struct `QueryWorldCB` has `mesh_cb` and `sprite_cb` function pointers. Used by renderer to collect visible geometry.

---

### `AppendInstUnique` (world.cpp:4566-4587)

**Signature:** `static void AppendInstUnique(Inst*** arr, int* count, int* cap, Inst* inst)`

**Purpose:** Adds instance to dynamic array if not already present. Expands array as needed.

**Called by:** `CollectMeshInsts()` (line 4647), recursive self-calls

**Calls:** None (array manipulation only)

**Globals read:** None

**Globals mutated:** `arr`, `count`, `cap` (via pointer parameters)

**Side effects:** Doubles array capacity if needed (lines 4570-4575); appends instance pointer; increments count.

**Notes:** Linear search for uniqueness check (lines 4578-4582). Array stored as pointer-to-pointer (`Inst***`) to handle reallocation transparently. Used by `CollectMeshInsts` to avoid duplicate instances from BSP+flat list.

---

### `CollectInstsFromBSP` (world.cpp:4590-4631)

**Signature:** `static void CollectInstsFromBSP(BSP* bsp, Inst*** arr, int* count, int* cap)`

**Purpose:** Recursively collects all instances from BSP tree into dynamic array with deduplication.

**Called by:** `CollectMeshInsts()` (line 4649), recursive self-calls

**Calls:** `AppendInstUnique()`, recursive `CollectInstsFromBSP()`

**Globals read:** None

**Globals mutated:** `arr`, `count`, `cap` (via AppendInstUnique)

**Side effects:** Adds all instances from BSP tree to array; handles all 4 BSP node types (NODE, NODE_SHARE, LEAF, INST).

**Notes:** For LEAF: iterate doubly-linked instance list and append each. For NODE_SHARE: iterate share list and recurse into children. For NODE: recurse into both children. For INST: append single instance. Deduplication handled by `AppendInstUnique`.

---

### `CollectMeshInsts` (world.cpp:4634-4665)

**Signature:** `int CollectMeshInsts(World* w, Inst*** out)`

**Purpose:** Collects all mesh instances from world (flat list + BSP tree) into dynamically allocated array. Returns count.

**Called by:** No callers found via grep in repository.

**Calls:** `AppendInstUnique()`, `CollectInstsFromBSP()`, allocation/free

**Globals read:** None

**Globals mutated:** Allocates dynamic array via `*out` parameter

**Side effects:** Traverses flat instance list → traverses BSP tree → filters for MESH type only → frees if empty.

**Notes:** Returns 0 if world is null or no mesh instances found. Caller responsible for freeing returned array after use (`free(arr)`). Useful for exporting or iterating all mesh instances regardless of spatial organization.

---

### `QueryWorldBSP` (world.cpp:4668-4672)

**Signature:** `void QueryWorldBSP(World* w, int planes, double plane[][4], void (*cb)(int level, const float bbox[6], void* cookie), void* cookie)`

**Purpose:** Queries BSP tree structure, invoking callback for each node's bbox (used for debugging or visualization).

**Called by:** No callers found via grep

**Calls:** `World::QueryBSP(level=1, root, planes, plane, cb, cookie)`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Invokes user callback for each BSP node with its level and bbox.

**Notes:** Direct wrapper to `World::QueryBSP()`. Level starts at 1 for root. Used for diagnostics or BSP visualization tools. Plane culling passed through to underlying implementation (currently disabled per comment in `QueryBSP`).

---

### `GetFirstMesh` (world.cpp:4676-4680)

**Signature:** `Mesh* GetFirstMesh(World* w)`

**Purpose:** Returns first mesh in world's mesh linked list.

**Called by:** Editor mesh iteration, export loops

**Calls:** None (direct field access)

**Globals read:** `w->head_mesh`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if world is null or empty_mesh. Use with `GetNextMesh()` for full iteration.

---

### `GetLastMesh` (world.cpp:4683-4687)

**Signature:** `Mesh* GetLastMesh(World* w)`

**Purpose:** Returns last mesh in world's mesh linked list (tail).

**Called by:** No callers found via grep in repository.

**Calls:** None

**Globals read:** `w->tail_mesh`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if world is null or empty_mesh. Use with `GetPrevMesh()` for reverse iteration.

---

### `GetPrevMesh` (world.cpp:4690-4694)

**Signature:** `Mesh* GetPrevMesh(Mesh* m)`

**Purpose:** Returns previous mesh in doubly-linked list.

**Called by:** Editor navigation, reverse iteration

**Calls:** None

**Globals read:** `m->prev`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if mesh is null or at head of list.

---

### `GetNextMesh` (world.cpp:4697-4701)

**Signature:** `Mesh* GetNextMesh(Mesh* m)`

**Purpose:** Returns next mesh in doubly-linked list.

**Called by:** Editor navigation, forward iteration

**Calls:** None

**Globals read:** `m->next`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if mesh is null or at tail of list.

---

### `GetMeshWorld` (world.cpp:4704-4707)

**Signature:** `World* GetMeshWorld(Mesh* m)`

**Purpose:** Returns world that owns this mesh.

**Called by:** Ownership validation, instance creation

**Calls:** None

**Globals read:** `m->world`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if mesh is null or not owned by any world.

---

### `GetMeshName` (world.cpp:4709-4723)

**Signature:** `int GetMeshName(Mesh* m, char* buf, int size)`

**Purpose:** Copies mesh name string into user buffer; returns length of name (0 if null/empty).

**Called by:** Editor UI, export tools

**Calls:** `strncpy()`, `strlen()`

**Globals read:** `m->name`

**Globals mutated:** Writes to user buffer `buf`

**Side effects:** Copies up to `size-1` characters; ensures null termination.

**Notes:** Returns name length regardless of `size` limitation. Caller should check return value vs. `size` to detect truncation. Returns 0 for null mesh or null name.

---

### `GetMeshBBox` (world.cpp:4726-4736)

**Signature:** `void GetMeshBBox(Mesh* m, float bbox[6])`

**Purpose:** Copies mesh's bounding box {xmin, xmax, ymin, ymax, zmin, zmax} to user buffer.

**Called by:** Editor bounding box display, culling

**Calls:** `memcpy()`

**Globals read:** `m->bbox`

**Globals mutated:** Writes to `bbox[6]`

**Side effects:** None

**Notes:** BBox computed during mesh load from vertex extents. Untransformed mesh space coordinates.

---

### `QueryMesh` (world.cpp:4739-4801)

**Signature:** `void QueryMesh(Mesh* m, void (*cb)(float coords[9], uint8_t colors[12], uint32_t visual, void* cookie), void* cookie)`

**Purpose:** Iterates mesh geometry (vertices and lines), invoking callback for each primitive. Used for exporting or custom rendering.

**Called by:** Export tools, debug visualization

**Calls:** `cb()` callback function for each vertex/line

**Globals read:** `m->verts`, `m->lines` (linked lists)

**Globals mutated:** None

**Side effects:** Invokes user callback for each vertex (3 coords + RGBA) and line (2 vertices + RGBA).

**Notes:** Vertex callback format: `coords[9]` = {x,y,z, x,y,z, x,y,z} (3 vertices for triangles), `colors[12]` = {R,G,B,A × 3}, `visual` = material ID. Line callback: 2 vertices. Iterates linked lists linearly.

---

### `GetMeshCookie` (world.cpp:4804-4808)

**Signature:** `void* GetMeshCookie(Mesh* m)`

**Purpose:** Returns user-defined cookie pointer stored in mesh.

**Called by:** External metadata lookup

**Calls:** None

**Globals read:** `m->cookie`

**Globals mutated:** None

**Side effects:** None

**Notes:** Cookie is opaque pointer set via `SetMeshCookie()`. Typically used for filename or asset metadata. Returns null if mesh is null.

---

### `SetMeshCookie` (world.cpp:4811-4814)

**Signature:** `void SetMeshCookie(Mesh* m, void* cookie)`

**Purpose:** Sets user-defined cookie pointer in mesh.

**Called by:** Mesh loading, asset management

**Calls:** None

**Globals read:** None

**Globals mutated:** `m->cookie`

**Side effects:** Stores cookie pointer in mesh struct.

**Notes:** Opaque pointer owned by caller; not managed by world system. Can be string, struct, or any data.

---

### `RebuildWorld` (world.cpp:4817-4825)

**Signature:** `void RebuildWorld(World* w, bool boxes)`

**Purpose:** Public wrapper to rebuild BSP tree; optionally recomputes mesh bounding boxes.

**Called by:** Editor rebuild command, after batch add/delete operations

**Calls:** `w->Rebuild(boxes)`

**Globals read:** None

**Globals mutated:** World's BSP tree structure

**Side effects:** Reconstructs BSP tree from all INST_USE_TREE instances; optionally updates mesh bboxes.

**Notes:** Use after adding many instances or changing transforms to restore spatial indexing. `boxes=true` forces bbox recomputation (e.g., after mesh reload).

---

### `SaveInst` (world.cpp:4828-4916)

**Signature:** `static void SaveInst(Inst* inst, FILE* f)`

**Purpose:** Serializes instance to .a3d file format using [DATA-CONTRACT:A3D] schema.

**Called by:** `SaveQueryBSP()` (recursive), `SaveWorld()` (flat list)

**Calls:** `fwrite()`, `strlen()`, sizeof computations

**Globals read:** `item_proto_lib` (for item proto index calculation, line 4904)

**Globals mutated:** None

**Side effects:** Writes binary data to file; skips INST_VOLATILE instances (line 4830).

**Notes:** Three formats based on instance type:
- MESH: mesh_id_len (4) + mesh_id (var) + inst_name_len (4) + inst_name (var) + transform[16×double] + flags + story_id
- SPRITE: mesh_id_len=-1 (4) + sprite_name_len (4) + sprite_name (var) + pos[3×float] + yaw + anim + frame + reps[4] + flags + story_id
- ITEM: mesh_id_len=-2 (4) + item_proto_index (4) + count + pos[3×float] + yaw + flags + story_id

Item insts only saved if they are EDIT instances (purpose filtering handled by caller).

---

### `SaveQueryBSP` (world.cpp:4919-4969)

**Signature:** `static void SaveQueryBSP(BSP* bsp, FILE* f)`

**Purpose:** Recursively traverses BSP tree and serializes all non-volatile instances to file.

**Called by:** `SaveWorld()` (line 4993), recursive self-calls

**Calls:** `SaveInst()`, recursive `SaveQueryBSP()`

**Globals read:** None

**Globals mutated:** Writes to FILE* `f`

**Side effects:** Serializes instances from BSP tree to .a3d format.

**Notes:** Traverses all 4 BSP node types: NODE (recurse children), NODE_SHARE (recurse children + save instances), LEAF (save instances), INST (save single instance). Skips INST_VOLATILE instances in `SaveInst()`.

---

### `SaveWorld` (world.cpp:4971-5005)

**Signature:** `void SaveWorld(World* w, FILE* f)`

**Purpose:** Serializes world to .a3d file: header, mesh count, mesh list, instance count, instances.

**Called by:** Editor save command

**Calls:** `fwrite()`, mesh iteration, `SaveInst()`, `SaveQueryBSP()`

**Globals read:** `w->meshes` (mesh count), mesh lists, instance lists

**Globals mutated:** Writes to FILE* `f`

**Side effects:** Writes binary .a3d world file; skips INST_VOLATILE instances.

**Notes:** .a3d format schema:
- Header: magic ('A3D\x1A'), version (4), 32-bit flags (4), 32-bit world ID (4) = 16 bytes
- Mesh count: int32 (4 bytes)
- For each mesh: name_len + name + num_verts + verts + num_faces + faces + num_lines + lines
- Instance count: int32 (4 bytes)
- For each instance: flat list insts → BSP tree insts (via `SaveQueryBSP()`)

Item insts filtered by `item->purpose == Item::EDIT` (editor-only items saved).

---

### `LoadWorld` (world.cpp:5008-5238)

**Signature:** `World* LoadWorld(FILE* f, bool editor)`

**Purpose:** Deserializes world from .a3d file; creates world, loads meshes, creates instances, rebuilds BSP tree.

**Called by:** Editor open command, game level load

**Calls:** `fread()`, `CreateWorld()`, `CreateMesh()`, `CreateInst()`, `RebuildWorld()`

**Globals read:** Reads from FILE* `f`

**Globals mutated:** None (allocates and returns new World*)

**Side effects:** Allocates World struct; loads all meshes from file; creates instances; constructs BSP tree via `RebuildWorld()`.

**Notes:** .a3d format parsing:
- Read and verify header (magic, version)
- Read mesh count, loop to load each mesh (PLY data embedded)
- Read instance count, loop to create instances (mesh/sprite/item types by mesh_id_len: positive=MESH, -1=SPRITE, -2=ITEM)
- Item instances: proto index resolved to `item_proto_lib[proto_index]`, count set, sprite_3d from proto
- If `editor=true`: item purpose set to `Item::EDIT`, `INST_VOLATILE` flag set
- Final: rebuild BSP tree

Returns null on version mismatch or read error.

---

### `HitWorld` (world.cpp:5242-5244)

**Signature:** `Inst* HitWorld(World* w, double p[3], double v[3], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, bool sprites_too, uint8_t* out_color)`

**Purpose:** Public wrapper for world raycasting; delegates to internal `HitWorld()` function.

**Called by:** Game raycasting, editor selection

**Calls:** `HitWorld(p, v, ret, nrm, positive_only, editor, solid_only, sprites_too, out_color)` (static function, not method)

**Globals read:** `root` (inside static `HitWorld`)

**Globals mutated:** Returns hit point in `ret[3]`, normal in `nrm[3]`, color in `out_color`

**Side effects:** Raycast queries BSP tree + flat list.

**Notes:** Identical signature to static `HitWorld()` at line 2955; appears to be wrapper for non-OO API. May be redundant or for header compatibility.

---

### `GetInstMesh` (world.cpp:5247-5251)

**Signature:** `Mesh* GetInstMesh(Inst* i)`

**Purpose:** Returns mesh if instance is mesh type; otherwise null.

**Called by:** Editor UI, mesh queries

**Calls:** None (type check and cast)

**Globals read:** `i->inst_type`, `i->inst_union.mesh`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null for non-mesh instances (sprite, item). Use `GetInstWorld()` to get instance's world.

---

### `GetInstFlags` (world.cpp:5254-5257)

**Signature:** `int GetInstFlags(Inst* i)`

**Purpose:** Returns instance flags (INST_VISIBLE, INST_USE_TREE, INST_VOLATILE, etc.).

**Called by:** Editor UI, filtering

**Calls:** None

**Globals read:** `i->flags`

**Globals mutated:** None

**Side effects:** None

**Notes:** Flags are bitmask: test with `i->flags & INST_VISIBLE`, etc.

---

### `SetInstFlags` (world.cpp:5260-5262)

**Signature:** `void SetInstFlags(Inst* i, int flags)`

**Purpose:** Sets instance flags.

**Called by:** Editor property editing

**Calls:** None

**Globals read:** None

**Globals mutated:** `i->flags`

**Side effects:** Updates flag bitmask; affects visibility, BSP participation, etc.

**Notes:** Replaces all flags; use `i->flags |= FLAG` to add specific flag without clearing others.

---

### `GetInstStoryID` (world.cpp:5265-5267)

**Signature:** `int GetInstStoryID(Inst* i)`

**Purpose:** Returns instance's story/game narrative identifier.

**Called by:** Scripting, game logic

**Calls:** None

**Globals read:** `i->story_id`

**Globals mutated:** None

**Side effects:** None

**Notes:** Used to associate instances with gameplay triggers, cutscenes, or narrative events.

---

### `GetInstName` (world.cpp:5270-5273)

**Signature:** `const char* GetInstName(Inst* i)`

**Purpose:** Returns instance's name string; null if unnamed.

**Called by:** Editor UI, scripting

**Calls:** None

**Globals read:** `i->inst_type`, `MeshInst::name` (for mesh instances)

**Globals mutated:** None

**Side effects:** None

**Notes:** Only mesh instances have names (sprite and item instances have null names). Returns null for non-mesh or unnamed instances.

---

### `SetInstStoryID` (world.cpp:5276-5279)

**Signature:** `void SetInstStoryID(Inst* i, int id)`

**Purpose:** Sets instance's story ID.

**Called by:** Editor scripting, level design

**Calls:** None

**Globals read:** None

**Globals mutated:** `i->story_id`

**Side effects:** Updates story identifier for gameplay logic.

**Notes:** Used by game scripts to trigger events based on instance IDs.

---

### `GetInstTM` (world.cpp:5282-5290)

**Signature:** `bool GetInstTM(Inst* i, double tm[16])`

**Purpose:** Copies instance's 4×4 transform matrix to user buffer; returns false if not mesh instance.

**Called by:** Editor transform editing, export

**Calls:** `memcpy()`

**Globals read:** `i->inst_type`, `MeshInst::tm`

**Globals mutated:** Copies to `tm[16]`

**Side effects:** None

**Notes:** Sprite and item instances use position+yaw not full matrix; returns false for those types. Matrix is column-major OpenGL format.

---

### `SetInstTM` (world.cpp:5293-5300)

**Signature:** `void SetInstTM(Inst* i, const double tm[16])`

**Purpose:** Sets mesh instance's transform matrix; updates bbox.

**Called by:** Editor transform editing

**Calls:** `UpdateBox()`, `memcpy()`

**Globals read:** `i->inst_type`

**Globals mutated:** `MeshInst::tm`, `MeshInst::bbox` (via UpdateBox)

**Side effects:** Updates transform and recomputes world bbox.

**Notes:** Only works for mesh instances. BBox update required for spatial query accuracy. If instance is in BSP tree, bbox changes may affect tree structure (caller may need to `Rebuild()` or `AttachInst()`/`DetachInst()`).

---

### `GetMeshFaces` (world.cpp:5303-5305)

**Signature:** `int GetMeshFaces(Mesh* m)`

**Purpose:** Returns face count for mesh.

**Called by:** Editor stats, export

**Calls:** Linked list iteration to count faces

**Globals read:** `m->faces` linked list

**Globals mutated:** None

**Side effects:** None

**Notes:** Iterates face linked list linearly; O(n) where n=face count. Returns 0 if mesh is null or empty.

---

### `GetInstBBox` (world.cpp:5308-5315)

**Signature:** `void GetInstBBox(Inst* i, double bbox[6])`

**Purpose:** Copies instance's world-space bounding box to user buffer {xmin, xmax, ymin, ymax, zmin, zmax}.

**Called by:** Editor bounds display, culling

**Calls:** `memcpy()`

**Globals read:** `i->bbox`

**Globals mutated:** Copies to `bbox[6]`

**Side effects:** None

**Notes:** BBox is precomputed world-space bounding box. For meshes, includes transform. For sprites/items, based on projection bbox + position.

---

### `GetInstWorld` (world.cpp:5318-5328)

**Signature:** `World* GetInstWorld(Inst* i)`

**Purpose:** Returns world that owns this instance.

**Called by:** Ownership validation, queries

**Calls:** Type-specific dispatch

**Globals read:** Depends on instance type:
- MeshInst: `i->mesh->world`
- SpriteInst: `si->w`
- ItemInst: `ii->w`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns null if instance is null or not owned by any world. Mesh instances get world indirectly through mesh reference; sprite/item instances store world directly.

---

### `GetInstSprite` (world.cpp:5331-5356)

**Signature:** `Sprite* GetInstSprite(Inst* i, float pos[3], float* yaw, int* anim, int* frame, int reps[4])`

**Purpose:** Returns sprite pointer for sprite/item instances; optionally outputs position, yaw, animation state.

**Called by:** Editor UI, animation queries

**Calls:** Type check and pointer extraction

**Globals read:** `i->inst_type`, SpriteInst fields, ItemInst fields

**Globals mutated:** Writes to optional output parameters if non-null

**Side effects:** None

**Notes:** For sprite instances: returns `si->sprite`; for item instances: returns `ii->item->proto->sprite_3d`. For meshes (inst_type != SPRITE/ITEM), returns null. Output parameters (pos, yaw, anim, frame, reps) only filled if non-null pointers passed.

---

### `GetInstSpriteData` (world.cpp:5359-5364)

**Signature:** `void* GetInstSpriteData(Inst* i)`

**Purpose:** Returns opaque user data pointer associated with sprite/item instance.

**Called by:** Game logic, entity data access

**Calls:** Type check and field access

**Globals read:** `i->inst_type`, `SpriteInst::data`, `ItemInst::data` (if present)

**Globals mutated:** None

**Side effects:** None

**Notes:** SpriteInst has explicit `data` field; ItemInst may have data via `item->data` or similar. Used to store player/creature pointers or other game entity state.

---

### `SetInstSpriteData` (world.cpp:5367-5373)

**Signature:** `bool SetInstSpriteData(Inst* i, void* data)`

**Purpose:** Sets opaque user data pointer for sprite/item instance.

**Called by:** Game entity initialization

**Calls:** Type check

**Globals read:** `i->inst_type`

**Globals mutated:** `SpriteInst::data` or `ItemInst::data`

**Side effects:** Stores user data pointer in instance.

**Notes:** Returns false if instance is not sprite/item type. Data pointer ownership not managed by world; caller responsible for lifetime.

---

### `GetInstItem` (world.cpp:5376-5391)

**Signature:** `Item* GetInstItem(Inst* i, float pos[3], float* yaw)`

**Purpose:** Returns Item pointer for item instances; optionally outputs position and yaw.

**Called by:** Inventory systems, item pickup logic

**Calls:** Type check

**Globals read:** `i->inst_type`, `ItemInst::item`, `ItemInst::pos`, `ItemInst::yaw`

**Globals mutated:** Writes to optional output parameters

**Side effects:** None

**Notes:** Returns null if instance is not item type. Item struct contains proto (definition), count, purpose, and other gameplay data. Position/yaw output only if non-null pointers passed.

---

### `BSP::InsertInst` (world.cpp:5394-5525)

**Signature:** `bool BSP::InsertInst(World* w, Inst* i)`

**Purpose:** Inserts instance into BSP tree at appropriate node based on bounding box overlap.

**Called by:** `AttachInst()` (line 5639), recursive self-calls for navigation

**Calls:** `BSP::InsertInst()` (recursive)

**Globals read:** None (uses this pointer)

**Globals mutated:** BSP tree structure (adds instance to LEAF lists or creates new NODE)

**Side effects:** Traverses tree; inserts instance into leaf with minimal overlap or splits node at instance bbox plane.

**Notes:** Algorithm:
- Start at node (initially root); compute overlap with instance bbox
- If NODE: check both children overlap; recurse into overlapping children; create NODE_SHARE if both children overlap
- If NODE_SHARE: check children overlap; iterate existing share list for duplicates; add instance to share list
- If LEAF: add instance to leaf's instance list
- If empty tree or insufficient instances: just add to list without splitting

Returns false if insertion fails. Uses box-box overlap test for child selection.

---

### `DetachInst` (world.cpp:5529-5596)

**Signature:** `bool DetachInst(World* w, Inst* inst)`

**Purpose:** Removes instance from BSP tree, placing it in flat list; unlinks from parent node.

**Called by:** Editor detach command, BSP restructuring

**Calls:** Linked list manipulation

**Globals read:** `w->root`, `inst->bsp_parent`

**Globals mutated:** BSP tree structure, instance's bsp_parent and prev/next

**Side effects:** Unlinks instance from BSP node (LEAF list or NODE_SHARE share list or NODE child pointer); updates parent node pointers.

**Notes:** Cases:
- If no bsp_parent: already detached, return false
- If instance is root: clear `w->root`
- If parent is NODE: clear child pointer (bsp_child[0] or bsp_child[1])
- If parent is NODE_SHARE: clear child pointer or remove from share list (linked list remove)
- If parent is LEAF: remove from doubly-linked instance list (prev/next)

Sets `inst->bsp_parent = 0`. Does NOT add back to flat list (caller responsibility).

---

### `AttachInst` (world.cpp:5599-5639)

**Signature:** `bool AttachInst(World* w, Inst* inst)`

**Purpose:** Adds instance to BSP tree; updates bbox if needed; handles mesh/sprite/item types.

**Called by:** Editor attach command, dynamic BSP insertion

**Calls:** `UpdateBox()` (for mesh), `BSP::InsertInst()`

**Globals read:** `inst->bsp_parent`, `w->root`

**Globals mutated:** instance's bbox, BSP tree structure

**Side effects:** Updates sprite/item bbox from sprite projection; recomputes mesh bbox via UpdateBox; inserts into BSP tree.

**Notes:** BBox update logic:
- SpriteInst: bbox = sprite->proj_bbox + position
- ItemInst: bbox = item->proto->sprite_3d->proj_bbox + position
- MeshInst: call `UpdateBox()` to transform mesh bbox

Returns false if instance already in tree or no root exists. Does NOT remove from flat list (should already be out of tree or caller handles that).

---

### `ShowInst` (world.cpp:5642-5645)

**Signature:** `void ShowInst(Inst* i)`

**Purpose:** Sets INST_VISIBLE flag; makes instance visible to queries and rendering.

**Called by:** Editor UI, script visibility toggles

**Calls:** None

**Globals read:** None

**Globals mutated:** `i->flags`

**Side effects:** Adds INST_VISIBLE flag; instance will be rendered and included in `Query` callbacks.

**Notes:** Complement to `HideInst()`. Flag persisted across AttachInst/DetachInst operations.

---

### `HideInst` (world.cpp:5647-5649)

**Signature:** `void HideInst(Inst* i)`

**Purpose:** Clears INST_VISIBLE flag; hides instance from rendering and queries.

**Called by:** Editor UI, script visibility toggles

**Calls:** None

**Globals read:** None

**Globals mutated:** `i->flags`

**Side effects:** Removes INST_VISIBLE flag; instance skipped in `Query` callbacks (line 3096 check).

**Notes:** Complement to `ShowInst()`. Flag persists; hidden instances still exist in BSP tree but ignored during traversal.

---

### `UpdateSpriteInst` (world.cpp:5652-5678)

**Signature:** `void UpdateSpriteInst(World* world, Inst* i, Sprite* sprite, const float pos[3], float yaw, int anim, int frame, const int reps[4])`

**Purpose:** Updates sprite instance state: sprite pointer, position, yaw, animation, repetition counts.

**Called by:** Editor property editing, runtime animation state changes

**Calls:** None (direct field assignment)

**Globals read:** `world` (unused)

**Globals mutated:** SpriteInst fields: sprite, pos, yaw, anim, frame, reps

**Side effects:** Updates instance state; bbox must be updated separately (not done here).

**Notes:** Does not update bbox; caller should call `AttachInst()` or manually update `i->bbox`. `world` parameter unused (function takes it but never accesses). Only works for sprite instances (type check not performed; caller must ensure correct type).

---

### `SoftInstAdd` (world.cpp:5681-5699)

**Signature:** `void SoftInstAdd(Inst* i)`

**Purpose:** Adds instance to world's flat list without affecting BSP tree.

**Called by:** Editor operations, temporary instance handling

**Calls:** Linked list manipulation

**Globals read:** None (uses `i->w` extracted as I)

**Globals mutated:** `I->head_inst`, `I->tail_inst`, instance's prev/next

**Side effects:** Links instance to doubly-linked flat instance list at end.

**Notes:** Increments `I->insts` counter; does NOT increment `temp_insts`. "Soft" means flat list only, no BSP tree modification. Used for instances that shouldn't be spatially indexed (e.g., editor tool objects).

---

### `SoftInstDel` (world.cpp:5702-5726)

**Signature:** `void SoftInstDel(Inst* i)`

**Purpose:** Removes instance from world's flat list without affecting BSP tree.

**Called by:** Editor operations, cleanup

**Calls:** Linked list manipulation

**Globals read:** Instance's `next` and `prev` pointers

**Globals mutated:** `head_inst`, `tail_inst`, prev/next pointers

**Side effects:** Unlinks instance from flat list; does NOT touch BSP tree.

**Notes:** Handles all cases: head, tail, middle. Null check on `i->next` to detect tail. Returns if not in flat list (`prev` is null but not head? Not clear from code logic). "Soft" removal preserves BSP tree membership.

---

### `HardInstDel` (world.cpp:5729-5755)

**Signature:** `void HardInstDel(Inst* i)`

**Purpose:** Removes instance from both BSP tree and flat list; hard deletion.

**Called by:** Editor delete operations

**Calls:** `DetachInst()` (line 5735), linked list removal

**Globals read:** Instance's next/prev, world pointer extracted as `I`

**Globals mutated:** BSP tree structure, flat list structure

**Side effects:** Unlinks from BSP tree AND flat list; decrements `I->insts` or `I->temp_insts` based on INST_VOLATILE flag.

**Notes:** Combines `DetachInst()` + flat list removal. Counter decrement: if flags & INST_VOLATILE, decrement `temp_insts`, else decrement `insts`. Does NOT free instance memory or return to pool; caller must call `DeleteInst(i)` for full cleanup.

---

### `AnimateSpriteInst` (world.cpp:5759-5785)

**Signature:** `int AnimateSpriteInst(Inst* i, uint64_t stamp)`

**Purpose:** Computes current frame for sprite instance based on animation timing.

**Called by:** Renderer, animation loop

**Calls:** None

**Globals read:** Instance's anim, frame, reps, sprite animation data

**Globals mutated:** None

**Side effects:** Returns computed frame index; does NOT update instance state.

**Notes:** Timing calculation:
- `len = reps[0] + reps[1]*anim_length + reps[2] + reps[3]*anim_length`
- `time = (stamp>>14) % len` (61.035 FPS timing base)
- Frame selection based on `time` ranges:
  - time < reps[0]: frame 0
  - time < reps[0] + reps[1]*len: interpolate (time - reps[0]) / reps[1]
  - etc.

Returns -1 if instance is not sprite type. Result can be assigned back to `si->frame`.

---

### `IsMaterialUsedInWorld` (world.cpp:5798-5831)

**Signature:** `bool IsMaterialUsedInWorld(World* w, int mat_id)`

**Purpose:** Checks if any mesh in world uses specified material ID.

**Called by:** Editor material cleanup, export filtering

**Calls:** Mesh face iteration linked list traversal

**Globals read:** `w->head_mesh`, mesh faces linked lists

**Globals mutated:** None

**Side effects:** None

**Notes:** Iterates all meshes in world (`head_mesh` → `tail_mesh` via `next`), then iterates all faces in each mesh, checking face's `visual` field for material ID match. Returns true on first match; linear search O(N_faces). Returns false if world is null or no match found.

---

**Signature:** `World* LoadWorld(FILE* f, bool editor)`

**Purpose:** Deserializes world from .a3d file; creates world, loads meshes, creates instances, rebuilds BSP tree.

**Called by:** Editor open command, game level load

**Calls:** `fread()`, `CreateWorld()`, `CreateMesh()`, `CreateInst()`, `RebuildWorld()`

**Globals read:** Reads from FILE* `f`

**Globals mutated:** None (allocates and returns new World*)

**Side effects:** Allocates World struct; loads all meshes from file; creates instances; constructs BSP tree via `RebuildWorld()`.

**Notes:** .a3d format parsing:
- Read and verify header (magic, version)
- Read mesh count, loop to load each mesh (PLY data embedded)
- Read instance count, loop to create instances (mesh/sprite/item types by mesh_id_len: positive=MESH, -1=SPRITE, -2=ITEM)
- Item instances: proto index resolved to `item_proto_lib[proto_index]`, count set, sprite_3d from proto
- If `editor=true`: item purpose set to `Item::EDIT`, `INST_VOLATILE` flag set
- Final: rebuild BSP tree

Returns null on version mismatch or read error.

---

**Signature:** `void ResetItemInsts(World* w)`

**Purpose:** Destroys all existing `Item::WORLD` items in world, then clones all `Item::EDIT` items from BSP tree and flat list to restore world state. Used to reset world items to editor template.

**Called by:** No callers found via grep in repository.

**Calls:** `DeleteItemInsts()`, `CloneItemInsts()`, `DestroyItem()`

**Globals read:** `delete_item_list` (global)

**Globals mutated:** `delete_item_list` (used as temporary list, cleared after use)

**Side effects:** Removes all world items from BSP tree and flat list, deallocates them, then recreates fresh instances from EDIT items.

**Notes:** Two-phase process: (1) Delete all WORLD items via `DeleteItemInsts(w->root, false)` → iterate `delete_item_list` → `DestroyItem()` each; (2) Clone EDIT items: flat list walk → BSP tree recursion via `CloneItemInsts()`. Global `delete_item_list` reused in both phases (cleared before second phase at line 3566). Used to reset level state without reloading entire world file.

---

**Signature:**
```cpp
Inst* HitWorld(double p[3], double v[3], double ret[3], double nrm[3],
               bool positive_only, bool editor, bool solid_only,
               bool sprites_too, uint8_t* out_color = 0)
```

**Purpose:**
Public raycast entry point. Converts ray origin and direction to Plucker coordinates, determines octant via ray direction sign bits, and dispatches to appropriate `HitWorldN()` function. Returns first instance hit by ray.

**Called by:**
- Public raycast entry; dispatcher for octant-specific functions

**Calls:**
- `HitWorld0()` through `HitWorld7()` (line 3016, via function pointer array indexed by `sign_case`)

**Globals read:**
- `root` (BSP tree root pointer, line 2957)

**Globals mutated:**
- None directly (output via reference parameters)

**Side effects:**
- Computes Plucker ray representation (lines 2972-2980) for all octant functions
- Determines ray direction octant (lines 2982-2989)
- Dispatches to octant-specific plane inequality handler

**Notes:**
- Plucker representation: `ray[0..2]` = cross product `p × v`, `ray[3..5]` = direction `v`, `ray[6..8]` = origin `p`, `ray[9]` = max t-distance (initialized to `FLT_MAX`)
- `positive_only` optimization commented out (lines 2960-2969); intended for reflection raycasts where ray starts above geometry
- Function pointer array `func_vect[]` (lines 2993-3004) dispatches based on 3-bit sign case (8 octants)
- Null check on `root` handles empty worlds (line 2957-2958)

---

### `QueryBSP` (world.cpp:3020-3068)

**Signature:**
```cpp
static void QueryBSP(int level, BSP* bsp, int planes, double plane[][4],
                     void (*cb)(int level, const float bbox[6], void* cookie),
                     void* cookie)
```

**Purpose:**
Recursively traverses BSP tree, invoking callback for each node. Currently disabled (comment at line 3022 notes plane culling not implemented). Used by frustum-culled instance enumeration.

**Called by:**
- No current callers found via grep (functionality appears dormant)

**Calls:**
- `cb()` (line 3023, callback invocation)
- `QueryBSP()` (recursive, lines 3030, 3032, 3040, 3042, 3047, 3059)

**Globals read:**
- None directly

**Globals mutated:**
- None directly (callback determines side effects via `cookie` parameter)

**Side effects:**
- Invokes user callback for each BSP node encountered

**Notes:**
- Plane culling disabled by design (comment line 3022): "temporarily don't check planes"
- Traverses all four BSP node types: `BSP_TYPE_NODE`, `BSP_TYPE_NODE_SHARE`, `BSP_TYPE_LEAF`, `BSP_TYPE_INST`
- Callback receives bbox and traversal level; used for diagnostics or visibility tests

---

### `Query()` (world.cpp:3073-3137)

**Signature:**
```cpp
static void Query(BSP* bsp, QueryWorldCB* cb, void* cookie)
```

**Purpose:**
Recursive BSP traversal without plane culling. Enumerates all instances in tree, dispatching to appropriate callback (mesh, sprite, or item) based on instance type. Updates global counters `bsp_nodes` and `bsp_insts`.

**Called by:**
- `World::Query()` (line 3319, for `root` traversal without planes)
- `World::Query()` (line 3341, for flat instance list `head_inst`)
- Recursive self-calls (lines 3081, 3112, 3114, 3122, 3124, 3128)

**Calls:**
- `Query()` (recursive, lines 3081, 3112, 3114, 3122, 3124, 3128)
- `cb->mesh_cb()` (line 3091, for MESH instances)
- `cb->sprite_cb()` (line 3097, for SPRITE instances with INST_VISIBLE flag)
- `cb->sprite_cb()` (line 3103, for ITEM instances via sprite_3d)

**Globals read:**
- None directly; counters updated are static BSP query state

**Globals mutated:**
- `bsp_nodes` (incremented per node, lines 3077, 3088, 3109, 3119)
- `bsp_insts` (incremented per instance, line 3088, 3193)

**Side effects:**
- Invokes mesh callback for mesh instances (line 3091)
- Invokes sprite callback for visible sprite and item instances (lines 3097, 3103)
- Accumulates traversal statistics for profiling

**Notes:**
- Item instances rendered as sprites via `item->proto->sprite_3d` (line 3103)
- Item purpose packed as integer (line 3103): `(int*)si->item` cast
- SPRITE instances filtered by `INST_VISIBLE` flag (line 3096); mesh instances rendered regardless

---

### `Query()` (world.cpp:3140-3293)

**Signature:**
```cpp
static void Query(BSP* bsp, int planes, double* plane[], QueryWorldCB* cb, void* cookie)
```

**Purpose:**
Recursive BSP traversal with plane-based frustum culling. Tests all 8 bbox corners against each culling plane, eliminating nodes entirely outside frustum. Falls back to plane-free `Query()` once all planes satisfied (lines 3224-3229, 3250-3263).

**Called by:**
- `World::Query()` (line 3315, for `root` with planes)
- `World::Query()` (line 3333, for flat instance list `head_inst` with planes)
- Recursive self-calls (lines 3219-3221, 3238-3241, 3239-3254, 3274-3276)

**Calls:**
- `PositiveProduct()` (lines 3150, 3153, 3156, 3159, 3162, 3165, 3168, 3171, plane-point dot product test)
- `Query()` (plane-free overload, lines 3226, 3228, 3253, 3255, 3260, 3284)
- `Query()` (recursive plane-culled overload, lines 3219, 3221, 3238, 3241, 3246, 3275)
- `cb->mesh_cb()` (line 3196, for instances)
- `cb->sprite_cb()` (lines 3202, 3208, for instances)

**Globals read:**
- `bsp_tests` (incremented per plane-bbox test, line 3144)
- `bsp_insts` (incremented per instance, line 3193)
- `bsp_nodes` (incremented per node, lines 3214, 3234, 3268)

**Globals mutated:**
- `bsp_tests`, `bsp_insts`, `bsp_nodes` (traversal statistics)

**Side effects:**
- Early rejection of entire subtrees outside frustum (lines 3175-3176 if all corners negative)
- Plane array modified in-place during culling (lines 3183-3186 swap rejected planes to end)
- Invokes callbacks for instances passing plane tests

**Notes:**
- Plane rejection logic (lines 3175-3188): iterates all 8 corners, counting positive/negative side tests
  - All 8 negative → node entirely outside plane → return early (line 3176)
  - All 8 positive → plane satisfied → remove from list (lines 3179-3186, swap to end, decrement count)
- Optimization: once all planes satisfied (planes==0), switches to plane-free `Query()` for efficiency
- Array `plane[]` destructively reordered (swap-and-pop on satisfied planes); caller must not rely on order preservation

---

### `World::Query` (world.cpp:3300-3345)

**Signature:**
```cpp
void Query(int planes, double plane[][4], QueryWorldCB* cb, void* cookie)
```

**Purpose:**
Public world query entry point. Resets BSP traversal counters, queries static BSP tree (`root`) and dynamic flat instance list (`head_inst`) using plane-based frustum culling if planes provided. Main interface for renderer frustum culling and physics raycasting.

**Called by:**
- Public method; main entry for spatial queries

**Calls:**
- `Query()` (plane-culled overload, lines 3315, 3333, for `root` and `head_inst` with planes)
- `Query()` (plane-free overload, lines 3319, 3341, for `root` and `head_inst` without planes)

**Globals read:**
- `root` (BSP tree root, line 3308)
- `head_inst` (flat instance list, line 3325)

**Globals mutated:**
- `bsp_tests` (reset to 0, line 3302)
- `bsp_insts` (reset to 0, line 3303)
- `bsp_nodes` (reset to 0, line 3304)

**Side effects:**
- Clears traversal statistics before query (lines 3302-3304)
- BSP tree queried before flat list (WHY comment line 3306-3307: spatial coherence → early rejection)
- Flat list always traversed if `head_inst` non-null (WHY comment lines 3323-3324: dynamic instances not in BSP)

**Notes:**
- Comment documents design rationale: static BSP tree for coherent spatial rejection, flat list for dynamic/volatile instances
- Conditionally allocates plane array pointers (lines 3312-3313, 3328-3329) only if planes > 0
- No plane culling when `planes <= 0` (lines 3317-3320 for `root`, 3337-3343 for `head_inst`)

---

## Summary of Range

**Lines 2147-4291 contain:**
- 8 ray dispatch functions (`HitWorld0` via `HitWorld7`, lines 1780-2951 including part 1, **HitWorld3-7 in this range**)
- 1 ray entry point (`HitWorld()`, lines 2955-3018)
- 1 BSP callback function (`QueryBSP()`, lines 3020-3068)
- 2 query overloads (plane-free and plane-culled, lines 3073-3293)
- 1 public world query interface (`World::Query()`, lines 3300-3345)
- Plus PLY mesh loader continuation (`Mesh::Update()` starts at line 3619, outside range end 4291)

**Key architectural patterns:**
- Plucker-based ray-box intersection with octant-specific plane inequalities (space-partitioned dispatch table)
- Dual-level spatial indexing: static BSP tree + dynamic flat instance list
- Frustum culling via plane-bbox corner testing with early rejection
- Callback-driven traversal decoupling queries from rendering/physics logic
- Global counters for profiling: `bsp_tests`, `bsp_insts`, `bsp_nodes`
