# world.cpp Architecture — Part 1 (Lines 1-2146)

## File Overview
- **Total lines:** 5832
- **Scope of this analysis:** Lines 1-2146 (infrastructure, instance management, BSP construction, raycasting stubs)
- **Purpose:** Manages dynamic instances (mesh, sprite, item) in game world via BSP tree for spatial queries; handles .a3d serialization

---

## Functions (Lines 1-2146)

### `WorldDebugEnabled` (world.cpp:158-167)

**Signature:** `static bool WorldDebugEnabled()`

**Purpose:** Returns true if ASCIICKER_WORLD_DEBUG environment variable is set.

**Called by:** No callers found via grep in codebase.

**Calls:** `getenv()`

**Globals read:** `cached` (static local variable)

**Globals mutated:** `cached` (static local variable)

**Side effects:** Caches environment variable check on first call; subsequent calls return cached value.

**Notes:** Guard pattern used to avoid repeated `getenv()` calls. Returns early cached value if already checked. Used internally by `UpdateBox()` (328), `AddInst()` (795), `DelInst()` (1741), `Rebuild()` (1616).

---

### `AllocItemInst` (world.cpp:567-574)

**Signature:** `ItemInst* AllocItemInst()`

**Purpose:** Allocates ItemInst from free pool or allocates new memory if pool empty.

**Called by:** `World::AddInst(Item*, ...)` (706), verified by grep in world.cpp.

**Calls:** `malloc()` (conditional)

**Globals read:** `item_inst_cache`

**Globals mutated:** `item_inst_cache`

**Side effects:** Removes item from cache linked list or allocates heap memory.

**Notes:** Pool-based allocator pattern. If cache empty, malloc creates new. Otherwise, pops from head of cache. Caller responsible for initializing returned ItemInst.

---

### `FreeItemInst` (world.cpp:576-580)

**Signature:** `void FreeItemInst(ItemInst* ii)`

**Purpose:** Returns ItemInst to free pool for reuse.

**Called by:** `World::DelInst(ItemInst*)` (1224), verified by grep in world.cpp.

**Calls:** None

**Globals read:** `item_inst_cache`

**Globals mutated:** `item_inst_cache` (prepends ii to cache list)

**Side effects:** Reuses ItemInst memory without deallocating; adds to head of cache linked list.

**Notes:** Complementary to AllocItemInst. Pushes ii to front of cache list. No bounds checking on cache size.

---

### `PurgeItemInstCache` (world.cpp:582-592)

**Signature:** `void PurgeItemInstCache()`

**Purpose:** Deallocates all cached ItemInst objects, clearing the free pool.

**Called by:** `mainmenu.cpp:main()`, `asciiid.cpp:ImGui_Impl()`, `game_app.cpp:Game::Shutdown()`, verified by grep.

**Calls:** `free()`, iteration over linked list

**Globals read:** `item_inst_cache`

**Globals mutated:** `item_inst_cache` (set to 0)

**Side effects:** Frees all pooled ItemInst memory; invalidates any cached pointers.

**Notes:** Called during shutdown/cleanup phases. Safe to call multiple times (second call loops over 0, does nothing).

---

### `Mesh::Update` (world.cpp:262 decl, definition at line 3619+)

**Signature:** `bool Mesh::Update(const char* path)`

**Purpose:** Reloads mesh geometry from .akm file (PLY format).

**Called by:** `UpdateMesh()` wrapper function (line 4505), verified by grep.

**Calls:** File I/O, PLY parsing, vertex/face allocation

**Globals read:** `mesh->world` pointer (self-check)

**Globals mutated:** `mesh` struct (verts, faces, bbox)

**Side effects:** Deallocates previous geometry; reads .akm file; rebuilds vertex/face linked lists.

**Notes:** Called when editor reloads mesh after external modification. Full geometry replacement, not incremental.

---

### `World::AddMesh` (world.cpp:606-642)

**Signature:** `Mesh* World::AddMesh(const char* name = 0, void* cookie = 0)`

**Purpose:** Creates and links a new empty Mesh node to world's mesh library.

**Called by:** `LoadMesh()` (4392), verified by grep.

**Calls:** `malloc()`, `strdup()`, memset

**Globals read:** None (uses this pointer)

**Globals mutated:** `head_mesh`, `tail_mesh`, `meshes` (instance counter)

**Side effects:** Allocates Mesh struct; links to world's doubly-linked list; initializes all geometry lists to empty.

**Notes:** Creates empty shell; actual geometry loaded separately via Mesh::Update or PLY parsing. name/cookie copied into mesh for metadata.

---

### `World::DelMesh` (world.cpp:644-695)

**Signature:** `bool World::DelMesh(Mesh* m)`

**Purpose:** Removes mesh from world, deallocates geometry, deletes sharing instances.

**Called by:** `DeleteMesh()` wrapper (4510), verified by grep.

**Calls:** `DelInst()`, `free()` (faces, lines, verts)

**Globals read:** `m->world` (self-check)

**Globals mutated:** `head_mesh`, `tail_mesh`, `meshes` (counter)

**Side effects:** Cascading deletion: removes all instances sharing this mesh; deallocates all faces, lines, verts; unlinks mesh from world list.

**Notes:** Returns false if mesh not owned by world. Face/line/vert lists traversed and freed individually. Name string also freed if present.

---

### `World::AddInst(Item*, ...)` (world.cpp:704-746)

**Signature:** `Inst* World::AddInst(Item* item, int flags, float pos[3], float yaw, int story_id)`

**Purpose:** Creates item instance (world item, NPC, etc.) from Item prototype; adds to flat list.

**Called by:** `CreateInst(World*, Item*, ...)` wrapper (4534), verified by grep in urdo.cpp.

**Calls:** `AllocItemInst()`, `strdup()` (conditional)

**Globals read:** `item->proto->sprite_3d` (for bbox)

**Globals mutated:** `insts`, `temp_insts` (if INST_VOLATILE), `head_inst`, `tail_inst`

**Side effects:** Allocates ItemInst from cache; initializes position, yaw, sprite bbox; links to flat list; increments instance counter.

**Notes:** Instance NOT inserted into BSP tree (no INST_USE_TREE flag set). Flat list holds non-tree instances. INST_VOLATILE flag increments temp_insts counter (world items vs. editor items).

---

### `World::AddInst(Sprite*, ...)` (world.cpp:748-807)

**Signature:** `Inst* World::AddInst(Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], const char* name, int story_id)`

**Purpose:** Creates sprite instance (billboard, character, NPC animation) from Sprite atlas.

**Called by:** `CreateInst(World*, Sprite*, ...)` wrapper (4526), verified by grep in urdo.cpp.

**Calls:** `malloc()`, `strdup()` (conditional)

**Globals read:** `s->proj_bbox` (sprite projection bbox), `WorldDebugEnabled()`

**Globals mutated:** `insts`, `temp_insts` (if INST_VOLATILE), `head_inst`, `tail_inst`

**Side effects:** Allocates SpriteInst (malloc, not cached); computes world bbox from sprite proj_bbox + position; links to flat list; logs debug info if enabled.

**Notes:** Sets type = BSP_TYPE_INST but NOT inserted into tree initially (bsp_parent = 0). Stores animation state (anim, frame, reps). Warns if INST_USE_TREE missing (debug).

---

### `World::AddInst(Mesh*, ...)` (world.cpp:809-866)

**Signature:** `Inst* World::AddInst(Mesh* m, int flags, const double tm[16], const char* name, int story_id)`

**Purpose:** Creates mesh instance from shared mesh; optionally initializes transform.

**Called by:** `CreateInst(Mesh*, ...)` wrapper (4518), verified by grep in urdo.cpp, asciiid.cpp.

**Calls:** `malloc()`, `strdup()` (conditional), `memcpy()`, `UpdateBox()`

**Globals read:** `m->world` (ownership check), `m->share_list`

**Globals mutated:** `insts`, `temp_insts` (if INST_VOLATILE), `head_inst`, `tail_inst`, `m->share_list`

**Side effects:** Allocates MeshInst; copies transform (or sets identity); computes world bbox via UpdateBox(); links to world's flat list AND mesh's share_list.

**Notes:** If tm=null, uses identity transform and mesh's untransformed bbox. Mesh ownership verified. Instance linked to BOTH flat list (for non-tree) AND mesh's share list (for mesh deletion cascades).

---

### `World::DelInst(Inst*)` (world.cpp:868-879)

**Signature:** `bool World::DelInst(Inst* i)`

**Purpose:** Type dispatcher; delegates to specialized delete for MESH/SPRITE/ITEM.

**Called by:** `DeleteInst()` wrapper (4541), verified by grep; cascading via `DelMesh()` (652).

**Calls:** `DelInst(MeshInst*)`, `DelInst(SpriteInst*)`, `DelInst(ItemInst*)`

**Globals read:** `i->inst_type`

**Globals mutated:** None (delegates)

**Side effects:** None directly; delegates all state changes.

**Notes:** Polymorphic dispatcher pattern. Returns false if type unknown or instance is null.

---

### `World::DelInst(MeshInst*)` (world.cpp:881-995)

**Signature:** `bool World::DelInst(MeshInst* i)`

**Purpose:** Removes mesh instance from world; unlinks from mesh share_list and BSP tree/flat list.

**Called by:** `World::DelInst(Inst*)` dispatcher (873), verified by grep.

**Calls:** `free()` (name string)

**Globals read:** `i->mesh->world` (ownership check), `editable`, `root` (globals)

**Globals mutated:** `editable`, `root`, `head_inst`, `tail_inst`, `insts`, `temp_insts`, `i->mesh->share_list`

**Side effects:** Unlinks from mesh share list (walk to remove); unlinks from BSP tree or flat list; clears editable/root if they point to this instance; decrements counters.

**Notes:** Handles both tree and flat-list instances. Tree deletion is STUBBED for ancestor cleanup (lines 922-971 marked "do ancestors cleanup // ..."). Returns false if invalid.

---

### `World::DelInst(SpriteInst*)` (world.cpp:998-1104)

**Signature:** `bool World::DelInst(SpriteInst* i)`

**Purpose:** Removes sprite instance from world; unlinks from BSP tree or flat list.

**Called by:** `World::DelInst(Inst*)` dispatcher (875), verified by grep.

**Calls:** `free()` (name string)

**Globals read:** `editable`, `root`

**Globals mutated:** `editable`, `root`, `head_inst`, `tail_inst`, `insts`, `temp_insts`

**Side effects:** Unlinks from flat list or BSP tree; clears editable/root if they point to this; decrements counters; deallocates name string.

**Notes:** Similar to MeshInst deletion but no mesh share_list to unlink from. Ancestor cleanup STUBBED (lines 1031-1081). Comment at 1140-1145 explains WHY cleanup is needed.

---

### `World::DelInst(ItemInst*)` (world.cpp:1107-1227)

**Signature:** `bool World::DelInst(ItemInst* i)`

**Purpose:** Removes item instance from world; returns ItemInst to free pool.

**Called by:** `World::DelInst(Inst*)` dispatcher (877), verified by grep.

**Calls:** `FreeItemInst()`

**Globals read:** `editable`, `root`, `i->flags`

**Globals mutated:** `editable`, `root`, `head_inst`, `tail_inst`, `insts`, `temp_insts`, `item_inst_cache`

**Side effects:** Unlinks from flat list or BSP tree; clears editable/root; decrements counters; returns ItemInst to cache pool (not malloc freed).

**Notes:** Item instances are pooled (FreeItemInst) not freed. Ancestor cleanup STUBBED (lines 1146-1197). TODO at 1140-1145.

---

### `World::DeleteBSP` (world.cpp:1236-1327)

**Signature:** `void World::DeleteBSP(BSP* bsp)`

**Purpose:** Recursively deallocates BSP tree; returns instances to flat list.

**Called by:** `Rebuild()` (1609), verified by grep.

**Calls:** Recursive `DeleteBSP()`, `free()`, linked-list manipulation

**Globals read:** `head_inst`, `tail_inst`

**Globals mutated:** `head_inst`, `tail_inst`, BSP node's `bsp_parent` (set to 0)

**Side effects:** Deallocates all BSP nodes; moves instances from tree back to flat list; clears parent pointers.

**Notes:** Handles all 4 BSP node types: NODE, NODE_SHARE, LEAF, INST. NODE_SHARE and LEAF transfer instance lists back to flat_list. INST type promotes instance back to flat list without further recursion.

---

### `World::SplitBSP` (world.cpp:1340-1603)

**Signature:** `BSP* World::SplitBSP(BSP_Item* arr, int num)`

**Purpose:** Recursively constructs axis-aligned BSP tree from instance array using Surface Area Heuristic (SAH) for splitting.

**Called by:** `Rebuild()` (1757), recursive self-call (1588, 1591), verified by grep.

**Calls:** `qsort()`, `malloc()` (BSP_Node/BSP_NodeShare/BSP_Leaf), fmin/fmax, recursive `SplitBSP()`

**Globals read:** `bsp_tests`, `bsp_insts`, `bsp_nodes` (globals, write-only)

**Globals mutated:** bsp stats globals (unused in this section)

**Side effects:** Allocates BSP tree; transforms instance list into hierarchical nodes; sets bsp_parent, prev/next pointers; returns root of subtree.

**Notes:** Base case (num==1): promote single instance to BSP_TYPE_INST directly. Computes SAH by testing all 3 axes (X/Y/Z), sorting instances by centroid, computing cumulative bbox surface areas. If best_cost too high, creates leaf instead of splitting. Leaf contains doubly-linked instance list. Internal node allocates BSP_NodeShare size for future upgrades.

---

### `World::Rebuild` (world.cpp:1605-1778)

**Signature:** `void World::Rebuild(bool boxes)`

**Purpose:** Reconstructs BSP tree from all instances marked INST_USE_TREE; optionally updates mesh bboxes.

**Called by:** `RebuildWorld()` wrapper (4817), verified by grep in mainmenu.cpp, asciiid.cpp, game_web.cpp.

**Calls:** `DeleteBSP()`, `SplitBSP()`, `UpdateBox()` (conditional), debug fprintf

**Globals read:** `root`, `head_inst`, `insts`, `WorldDebugEnabled()`

**Globals mutated:** `root`, `head_inst`, `tail_inst`, instance `bsp_parent` pointers

**Side effects:** Deletes old BSP tree; extracts INST_USE_TREE instances; optionally recomputes mesh bboxes; rebuilds tree via SplitBSP; returns instances to flat list if split failed.

**Notes:** Non-tree instances remain in flat list throughout. Extraction from list during collection (1730-1737). Allocation check at 1723-1726 catches array overflow (defensive). Debug logging at 1616, 1752 if enabled.

---

### `HitWorld0` (world.cpp:1780-1924)

**Signature:** `static Inst* HitWorld0(BSP* q, double ray[10], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, bool sprites_too, uint8_t* out_color)`

**Purpose:** Raycasts BSP tree using axis-aligned plane inequalities (SIMD-like optimization for specific ray direction).

**Called by:** No callers found via grep in lines 1-2146.

**Calls:** `HitWorld0()` (recursive), `MeshInst::HitFace()`, `SpriteInst::Hit()`, `ItemInst::Hit()`

**Globals read:** None

**Globals mutated:** `ret[3]`, `nrm[3]`, `out_color`, `ray[9]` (distance accumulation)

**Side effects:** Updates ret (intersection point), nrm (normal vector), out_color (vertex color), ray[9] (closest distance).

**Notes:** Three variants (HitWorld0, HitWorld1, HitWorld2) for different ray projection planes (hardcoded inequalities). Filters instances by editor flag and INST_VOLATILE. Returns first (closest) hit. bbox[6] = {xmin, xmax, ymin, ymax, zmin, zmax}. Assumes ray is normalized.

---

### `HitWorld1` (world.cpp:1926-2070)

**Signature:** `static Inst* HitWorld1(BSP* q, double ray[10], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, bool sprites_too, uint8_t* out_color)`

**Purpose:** Raycasts BSP tree with alternate plane inequalities (variant 1 of 3).

**Called by:** No callers found via grep in lines 1-2146.

**Calls:** `HitWorld1()` (recursive), `MeshInst::HitFace()`, `SpriteInst::Hit()`, `ItemInst::Hit()`

**Globals read:** None

**Globals mutated:** `ret`, `nrm`, `out_color`, `ray[9]`

**Side effects:** Same as HitWorld0.

**Notes:** Identical logic to HitWorld0/HitWorld2 except plane inequalities. Suggests three ray projection directions are optimized separately (e.g., for camera pointing left/right/down).

---

### `HitWorld2` (world.cpp:2072-2200+)

**Signature:** `static Inst* HitWorld2(BSP* q, double ray[10], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, bool sprites_too, uint8_t* out_color)`

**Purpose:** Raycasts BSP tree with alternate plane inequalities (variant 2 of 3).

**Called by:** No callers found via grep in lines 1-2146.

**Calls:** `HitWorld2()` (recursive), mesh/sprite/item hit tests

**Globals read:** None

**Globals mutated:** `ret`, `nrm`, `out_color`, `ray[9]`

**Side effects:** Same as HitWorld0/HitWorld1.

**Notes:** Third of three variants. Line 2200+ indicates file continues beyond scope of this analysis.

---

### `HitSprite(sprite, anim, frame, pos, yaw, ray, ret, positive_only)` (world.cpp:423-511)

**Signature:** `inline bool HitSprite(Sprite* sprite, int anim, int frame, float pos[3], float yaw, double ray[10], double ret[3], bool positive_only)`

**Purpose:** Ray-sprite billboard intersection test using ASCII cell-level rasterization.

**Called by:** `SpriteInst::Hit()` (line 527), `ItemInst::Hit()` (line 559), non-inline `HitSprite()` wrapper (line 545)

**Calls:** `DotProduct()` (line 428), `atan2()` (line 451), math operations

**Globals read:** None

**Globals mutated:** `ray[9]` (closest distance, updated on intersection), `ret[0..2]` (intersection point)

**Side effects:** Computes intersection with sprite billboard cells; updates intersection point and ray distance; filters transparent cells.

**Notes:** Inline helper for sprite raycasting. Calculates billboard rotation from ray yaw, selects correct sprite frame based on rotation angle, maps ray intersection to sprite cell coordinates, checks cell transparency. Transparent cells: `ac->bk == 255 && ac->gl == 219` OR `ac->fg == 255 && (ac->gl == 0 || ac->gl == 32)`. Returns true only for non-transparent cells with valid height check.

---

### `HitSprite(sprite, anim, frame, pos, yaw, p, v, ret, positive_only)` (world.cpp:533-546)

**Signature:** `bool HitSprite(Sprite* sprite, int anim, int frame, float pos[3], float yaw, double p[3], double v[3], double ret[3], bool positive_only)`

**Purpose:** Non-inline wrapper that converts ray origin/direction to Plucker coordinates and delegates to inline `HitSprite()`.

**Called by:** No callers found via grep (function only active when `#ifdef EDITOR` defined).

**Calls:** Plucker coordinate computation (lines 537-542), inline `HitSprite()` (line 545)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Converts ray to Plucker representation; delegates intersection test.

**Notes:** Compiler-disabled when `EDITOR` not defined. Wrapper for editor-specific raycasting interface that takes separate origin/direction instead of pre-computed Plucker ray.

---

## Global Variables (Lines 1-2146)

**Signature:** `inline bool HitSprite(Sprite* sprite, int anim, int frame, float pos[3], float yaw, double ray[10], double ret[3], bool positive_only)`

**Purpose:** Ray-sprite billboard intersection test using ASCII cell-level rasterization.

**Called by:** `SpriteInst::Hit()` (line 527), `ItemInst::Hit()` (line 559), non-inline `HitSprite()` wrapper (line 545)

**Calls:** `DotProduct()` (line 428), `atan2()` (line 451), math operations

**Globals read:** None

**Globals mutated:** `ray[9]` (closest distance, updated on intersection), `ret[0..2]` (intersection point)

**Side effects:** Computes intersection with sprite billboard cells; updates intersection point and ray distance; filters transparent cells.

**Notes:** Inline helper for sprite raycasting. Calculates billboard rotation from ray yaw, selects correct sprite frame based on rotation angle, maps ray intersection to sprite cell coordinates, checks cell transparency. Transparent cells: `ac->bk == 255 && ac->gl == 219` OR `ac->fg == 255 && (ac->gl == 0 || ac->gl == 32)`. Returns true only for non-transparent cells with valid height check.

---

### `HitSprite(sprite, anim, frame, pos, yaw, p, v, ret, positive_only)` (world.cpp:533-546)

**Signature:** `bool HitSprite(Sprite* sprite, int anim, int frame, float pos[3], float yaw, double p[3], double v[3], double ret[3], bool positive_only)`

**Purpose:** Non-inline wrapper that converts ray origin/direction to Plucker coordinates and delegates to inline `HitSprite()`.

**Called by:** No callers found via grep (function only active when `#ifdef EDITOR` defined).

**Calls:** Plucker coordinate computation (lines 537-542), inline `HitSprite()` (line 545)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Converts ray to Plucker representation; delegates intersection test.

**Notes:** Compiler-disabled when `EDITOR` not defined. Wrapper for editor-specific raycasting interface that takes separate origin/direction instead of pre-computed Plucker ray.

---

## Global Variables (Lines 1-2146)

| Name | Type | Scope | Purpose |
|------|------|-------|---------|
| `cached` | static int | WorldDebugEnabled() | Caches env var check (-1=unchecked, 0=false, 1=true) |
| `item_inst_cache` | ItemInst* | file scope | Head of free pool linked list |
| `bsp_tests` | int | file scope | Unused stat counter |
| `bsp_insts` | int | file scope | Unused stat counter |
| `bsp_nodes` | int | file scope | Unused stat counter |

---

## Key Architectural Patterns

### Instance Lifecycle
```
AddInst() → flat list [or BSP insert via Rebuild()]
   ↓
Rebuild() → BSP tree construction via SplitBSP()
   ↓
Query/Hit → traverse BSP tree with raycasting
   ↓
DelInst() → remove from BSP tree/flat list or FreeItemInst()
```

### BSP Tree Types
- **BSP_TYPE_NODE**: Interior node, 2 children, no instances
- **BSP_TYPE_NODE_SHARE**: Interior node + straddling instance list (stubs only)
- **BSP_TYPE_LEAF**: Leaf node with doubly-linked instance list
- **BSP_TYPE_INST**: Single instance promoted directly to BSP node

### Known Limitations (TODOs)

1. **Ancestor cleanup STUBBED** (lines 922, 953, 969, 1031, 1062, 1078, 1146-1197): When BSP leaf becomes empty after instance deletion, walk up tree to collapse empty parent nodes. Algorithm missing → memory accumulates over many deletions.

2. **HitWorld0/1/2 unused**: Three variants exist for different ray projections but no callsites found in lines 1-2146.  called from game.cpp (outside this analysis scope).

3. **NODE_SHARE instances**: Comment at 1578 mentions future capability to detect straddling instances; currently not implemented.

---

## Cross-File Dependencies

| Dependency | Location | Purpose |
|------------|----------|---------|
| `sprite.h` | #include | Sprite, Sprite::Frame definitions |
| `world.h` | #include | Public API declarations |
| `matrix.h` | #include | Product, CrossProduct, DotProduct |
| `terrain.h` | #include | Terrain data structures |
| `inventory.h` | #include | Item, Item::proto definitions |
| `getenv()` | stdlib | Environment variable lookup |
| `malloc/free` | stdlib | Heap allocation |
| `qsort` | stdlib | SAH sorting in SplitBSP |
| `fmin/fmax` | math | Bbox min/max computation |
| `fprintf` | stdio | Debug logging |

---

## Summary Statistics (Lines 1-2146)

---

### `MeshInst::UpdateBox` (world.cpp:328-370)

**Signature:** `void UpdateBox()`

**Purpose:** Recomputes world-space bounding box by transforming all mesh vertices with instance transform matrix.

**Called by:** `World::AddInst(Mesh*, ...)` (line 839), `World::Rebuild()` (line 1717), `AttachInst()` (line 5626)

**Calls:** `Product()` (matrix-vector multiplication), `fminf()`, `fmaxf()`, `WorldDebugEnabled()`, `fprintf()`

**Globals read:** None

**Globals mutated:** `this->bbox[6]` (instance bounding box)

**Side effects:** Updates instance bounding box; logs debug warnings if mesh has no vertices or bbox is invalid (min > max).

**Notes:** Transforms mesh vertices from local space to world space via `Product(tm, v->xyzw, w)`. Iterates all vertices to compute min/max extents. Debug logging controlled by `WorldDebugEnabled()` environment variable. Called whenever transform changes to maintain spatial query accuracy.

---

### `MeshInst::HitFace` (world.cpp:371-420)

**Signature:** `bool HitFace(double ray[10], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, uint8_t* out_color = 0)`

**Purpose:** Ray-triangle intersection test for all faces in mesh instance; returns true if any face hit.

**Called by:** `HitWorld0()` (line 1823), `HitWorld1()` (line 1969), `HitWorld2()` (line 2103), `HitWorld3()` (line 2249), `HitWorld4()` (line 2396), `HitWorld5()` (line 2543), `HitWorld6()` (line 2690), `HitWorld7()` (line 2837)

**Calls:** `Product()` (transforms vertices), `RayIntersectsTriangle()`, `CrossProduct()` (computes normal), barycentric interpolation

**Globals read:** `mesh->head_face`, `flags` (INST_VISIBLE check)

**Globals mutated:** `ret[3]` (intersection point), `nrm[3]` (surface normal), `ray[9]` (closest distance), `out_color[3]` (vertex color interpolation)

**Side effects:** Updates intersection results for closest hit; skips invisible instances (INST_VISIBLE flag); optionally filters opaque faces (solid_only).

**Notes:** Filters faces by alpha channel when `solid_only=true` (line 383: checks if any vertex has alpha >= 0x80). Transforms vertices from local to world space via instance transform matrix `tm`. Barycentric interpolation computes vertex colors (lines 407-410). Returns false if mesh is null or no visible faces hit.

---

### `SpriteInst::Hit` (world.cpp:524-529)

**Signature:** `bool Hit(double ray[10], double ret[3], bool positive_only)`

**Purpose:** Ray-billboard intersection test for sprite instance; delegates to `HitSprite()` helper.

**Called by:** `HitWorld0()` (line 1831), `HitWorld1()` (line 1977), `HitWorld2()` (line 2111), `HitWorld3()` (line 2257), `HitWorld4()` (line 2404), `HitWorld5()` (line 2551), `HitWorld6()` (line 2698), `HitWorld7()` (line 2845)

**Calls:** `HitSprite()` (inline helper, line 423)

**Globals read:** `flags` (INST_VISIBLE check), sprite animation state (sprite, anim, frame, pos, yaw)

**Globals mutated:** `ret[3]` (intersection point via HitSprite), `ray[9]` (closest distance via HitSprite)

**Side effects:** Returns false if instance not visible; delegates billboard intersection to HitSprite.

**Notes:** Visibility check prevents raycasting hidden sprites. Passes animation state to HitSprite for correct frame selection. Billboard always faces camera.

---

### `ItemInst::Hit` (world.cpp:556-561)

**Signature:** `bool Hit(double ray[10], double ret[3], bool positive_only)`

**Purpose:** Ray-billboard intersection test for item instance; renders item as sprite via proto sprite_3d.

**Called by:** `HitWorld0()` (line 1839), `HitWorld1()` (line 1985), `HitWorld2()` (line 2119), `HitWorld3()` (line 2265), `HitWorld4()` (line 2412), `HitWorld5()` (line 2559), `HitWorld6()` (line 2706), `HitWorld7()` (line 2853)

**Calls:** `HitSprite()` (inline helper, line 423)

**Globals read:** `flags` (INST_VISIBLE check), `item->proto->sprite_3d`, `pos`, `yaw`

**Globals mutated:** `ret[3]` (intersection point), `ray[9]` (closest distance)

**Side effects:** Returns false if instance not visible; treats item as stationary billboard (anim=0, frame=0).

**Notes:** Items are rendered as billboards using their prototype's sprite_3d atlas. No animation state (anim=0, frame=0). Visibility check consistent with sprite instances.

---

### `CentroidSorter::sortX` (world.cpp:1358-1369)

**Signature:** `static int sortX(const void* a, const void* b)`

**Purpose:** qsort comparator for BSP_Item array; sorts by X-axis centroid (bbox center).

**Called by:** `qsort()` via function pointer array (line 1398), dispatched from `SplitBSP()` (line 1456)

**Calls:** None (arithmetic only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (pure comparator)

**Notes:** Computes centroid as `(bbox[0] + bbox[1]) / 2` (line 1363). Returns -1/0/+1 for sorting. Used by Surface Area Heuristic (SAH) in BSP tree construction to find optimal split planes. Part of CentroidSorter struct (lines 1357-1394) containing 3 axis comparators.

---

### `CentroidSorter::sortY` (world.cpp:1370-1381)

**Signature:** `static int sortY(const void* a, const void* b)`

**Purpose:** qsort comparator for BSP_Item array; sorts by Y-axis centroid.

**Called by:** `qsort()` via function pointer array (line 1399), dispatched from `SplitBSP()` (line 1456)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Computes centroid as `(bbox[2] + bbox[3]) / 2` (line 1375). Identical structure to sortX but for Y axis. SAH tests all 3 axes to find best split.

---

### `CentroidSorter::sortZ` (world.cpp:1382-1393)

**Signature:** `static int sortZ(const void* a, const void* b)`

**Purpose:** qsort comparator for BSP_Item array; sorts by Z-axis centroid.

**Called by:** `qsort()` via function pointer array (line 1400), dispatched from `SplitBSP()` (line 1456)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Computes centroid as `(bbox[4] + bbox[5]) / 2` (line 1387). Third axis comparator. All three comparators referenced in function pointer array `sort[3]` (lines 1396-1401).

---

---

### `HitWorld2` (world.cpp:2072-2216)

**Signature:** `static Inst* HitWorld2(BSP* q, double ray[10], double ret[3], double nrm[3], bool positive_only, bool editor, bool solid_only, bool sprites_too, uint8_t* out_color)`

**Purpose:** Raycasts BSP tree with alternate plane inequalities (variant 2 of 8); third octant-specific implementation for ray-box intersection optimization.

**Called by:** `HitWorld()` (line 3016, via function pointer array `func_vect[2]`)

**Calls:** `HitWorld2()` (recursive, for BSP child nodes), `MeshInst::HitFace()`, `SpriteInst::Hit()`, `ItemInst::Hit()`

**Globals read:** None

**Globals mutated:** `ret[3]` (intersection point), `nrm[3]` (surface normal), `out_color` (material ID), `ray[9]` (closest distance)

**Side effects:** Updates intersection results; filters instances by editor flag (INST_VOLATILE).

**Notes:** Plane inequalities (lines 2086-2091) optimized for specific ray direction octant (sign_case=2). Identical structure to HitWorld0, HitWorld1, HitWorld3-7 but with different bbox rejection tests. Editor vs. game filtering at line 2097-2099: editor sees INST_VOLATILE instances, game sees non-volatile.

---

| Category | Count |
|----------|-------|
| Functions analyzed | 28 |
| Global variables | 4 |
| Instance types supported | 3 (MESH, SPRITE, ITEM) |
| BSP node types | 4 |
| Raycasting variants | 6 (HitWorld0/1/2, 2 HitSprite overloads, +HitWorld2 documented) |
| TODO/FIXME markers | 5+ |

