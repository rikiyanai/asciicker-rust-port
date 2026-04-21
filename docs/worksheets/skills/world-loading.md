# Skill Pack: World & Terrain Loading Subsystem

BSP-tree world instances + quadtree terrain patches. Binary .a3d format for both.
Global pointers: `extern Terrain* terrain; extern World* world;`

**Key files:** `world.cpp` (~5000 lines), `world.h`, `terrain.cpp` (~3300 lines), `terrain.h`, `world_patch.cpp`

**Cross-references:** [Engine Architecture](../../ENGINE_ARCHITECTURE.md), [A3D World Format](../codedoc-xp-terrain-format.md)

---

## 1. Entrypoints

### Terrain Lifecycle

```cpp
Terrain* CreateTerrain(int z=-1);       // z=-1 = empty, z>=0 = one patch at origin
void DeleteTerrain(Terrain* t);
bool SaveTerrain(const Terrain* t, FILE* f);   // [DATA-CONTRACT:A3D]
Terrain* LoadTerrain(FILE* f, PatchIndex** idx = 0);  // idx for editor patch selection
```

### Terrain Patch Operations

```cpp
Patch* GetTerrainPatch(Terrain* t, int x, int y);         // lookup by coords
void GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y); // reverse lookup
Patch* AddTerrainPatch(Terrain* t, int x, int y, int z);  // create new patch
bool DelTerrainPatch(Terrain* t, int x, int y);

// Raw data access (caller MUST respect array bounds)
uint16_t* GetTerrainHeightMap(Patch* p);   // [(HEIGHT_CELLS+1)^2] = 25 elements
uint16_t* GetTerrainVisualMap(Patch* p);   // [VISUAL_CELLS^2] = 64 elements

// MUST call after modifying raw data (propagates to GPU + quadtree bounds)
void UpdateTerrainHeightMap(Patch* p);
void UpdateTerrainVisualMap(Patch* p);
```

### Terrain Queries

```cpp
// Frustum culling: calls cb per visible patch
void QueryTerrain(Terrain* t, int planes, double plane[][4], int view_flags,
                  void(*cb)(Patch*, int x, int y, int view_flags, void*), void*);

// Radius query: circular area around point
void QueryTerrain(Terrain* t, double x, double y, double r, int view_flags,
                  void(*cb)(Patch*, int x, int y, int view_flags, void*), void*);

// Raycasting: returns hit patch + intersection point (ret[4] = xyz + t parameter)
Patch* HitTerrain(Terrain* t, double p[3], double v[3], double ret[4],
                  double nrm[3]=0, bool positive_only=false);
```

### Terrain Undo (dedicated, do not use for general patch ops)

```cpp
size_t TerrainDetach(Terrain* t, Patch* p, int* x, int* y);  // remove without free
size_t TerrainAttach(Terrain* t, Patch* p, int x, int y);    // re-insert
size_t TerrainDispose(Patch* p);                              // free detached
```

### Terrain Shadows

```cpp
void UpdateTerrainDark(Terrain* t, World* w, float lightpos[3], bool editor);
uint64_t GetTerrainDark(Patch* p);   // 64-bit = 8x8 shadow flags
void SetTerrainDark(Patch* p, uint64_t dark);
```

### World Lifecycle

```cpp
World* CreateWorld();
void DeleteWorld(World* w);
void SaveWorld(World* w, FILE* f);                // [DATA-CONTRACT:A3D]
World* LoadWorld(FILE* f, bool editor);           // editor=true clones items
void RebuildWorld(World* w, bool boxes = false);  // Reconstruct BSP tree
```

### Mesh Resources

```cpp
Mesh* LoadMesh(World* w, const char* path, const char* name = 0);  // [DATA-CONTRACT:AKM]
bool UpdateMesh(Mesh* m, const char* path);  // Reload geometry, keep identity
void DeleteMesh(Mesh* m);

// Linked list traversal
Mesh* GetFirstMesh(World* w);
Mesh* GetNextMesh(Mesh* m);

void QueryMesh(Mesh* m, void(*cb)(float coords[9], uint8_t colors[12],
               uint32_t visual, void*), void*);
```

### Instance Management (3 types)

```cpp
// Mesh instance (stores 4x4 double transform matrix)
Inst* CreateInst(Mesh* m, int flags, const double tm[16], const char* name, int story_id);

// Sprite instance (billboard with animation state)
Inst* CreateInst(World* w, Sprite* s, int flags, float pos[3], float yaw,
                 int anim, int frame, int reps[4], const char* name, int story_id);

// Item instance (inventory object in world)
Inst* CreateInst(World* w, Item* item, int flags, float pos[3], float yaw, int story_id);

void DeleteInst(Inst* i);

bool GetInstTM(Inst* i, double tm[16]);
void SetInstTM(Inst* i, const double tm[16]);
void GetInstBBox(Inst* i, double bbox[6]);

bool AttachInst(World* w, Inst* i);  // move from flat list into BSP
```

### World Queries

```cpp
struct QueryWorldCB {
    void(*mesh_cb)(Inst*, Mesh*, double tm[16], void*);
    void(*sprite_cb)(Inst*, Sprite*, float pos[3], float yaw, int anim, int frame, int reps[4], void*);
};

void QueryWorld(World* w, int planes, double plane[][4], QueryWorldCB* cb, void* cookie);

Inst* HitWorld(World* w, double p[3], double v[3], double ret[3], double nrm[3],
               bool positive_only=false, bool editor=false, bool solid_only=false,
               bool sprites_too=true, uint8_t* out_color=0);
```

### Undo/Redo Instance Operations

```cpp
void SoftInstAdd(Inst* i);   // restore BSP linkage (reversible)
void SoftInstDel(Inst* i);   // remove BSP linkage (reversible)
void HardInstDel(Inst* i);   // permanently free (MUST call after SoftInstDel)
```

---

## 2. Invariants & Data Contracts

### .a3d Terrain Binary Format

```
FileHeader (16 bytes):
  uint32_t file_sign = "AS3D"      // magic (0x44333341 little-endian)
  uint32_t header_size = 16        // version check
  uint32_t num_patches             // count of FilePatch records
  uint32_t reserved = 0

FilePatch (188 bytes each):
  int32_t x, y                     // patch world coordinates (8 bytes)
  uint16_t visual[8][8]            // material grid (128 bytes)
  uint16_t height[5][5]            // vertex heightmap (50 bytes)
  uint16_t diag                    // triangle orientation bitfield (2 bytes)
```

**Byte order:** native CPU (little-endian on x86). No endianness field.

### .a3d World Binary Format

```
Header:
  int32_t format_version           // <0 = versioned (current: -1), >=0 = legacy (num_insts directly)
  int32_t num_of_instances         // (only if format_version < 0)

Per-Instance (3 variants based on mesh_id_len):
  mesh_id_len >= 0:  MeshInst  (string mesh_id + double tm[16] + flags + story_id)
  mesh_id_len == -1: SpriteInst (string sprite_name + float pos[3] + yaw + anim + frame + reps[4])
  mesh_id_len == -2: ItemInst  (int item_proto + int count + float pos[3] + yaw)
```

### Key Dimension Constants

| Constant | Value | Meaning |
|----------|-------|---------|
| `VISUAL_CELLS` | 8 | Material grid per patch (8x8 = 64 cells) |
| `HEIGHT_CELLS` | 4 | Height quads per patch (4x4 = 16 quads, 5x5 = 25 vertices) |
| `HEIGHT_SCALE` | 16 | Z-units per visual cell. Changing breaks all .a3d files |
| `DARK_TERRAIN` | defined | Enables 64-bit shadow bitmask per patch (8x8 = 1 bit per visual cell) |

### Instance Flags

```cpp
enum INST_FLAGS {
    INST_VISIBLE   = 0x1,  // rendered (hidden instances skip query callbacks)
    INST_USE_TREE  = 0x2,  // participates in BSP (vs. flat list)
    INST_VOLATILE  = 0x4,  // temporary (NPCs, projectiles) — excluded from save
    INST_SELECTED  = 0x8   // editor selection highlight
};
```

### BSP Tree Node Types (internal to world.cpp)

```
BSP_TYPE_NODE       — Interior: 2 children, split plane
BSP_TYPE_NODE_SHARE — Interior + linked list of straddling instances
BSP_TYPE_LEAF       — Leaf: linked list of instances
BSP_TYPE_INST       — Promoted single instance (no subdivision)
```

### Quadtree Structure (internal to terrain.cpp)

```
Terrain {
    Node* root;     // quadtree root
    int level;      // tree depth (0 = single patch)
    int x, y;       // base offset (world-space origin)
    int patches;    // total patch count
}

Node {
    QuadItem* quad[4];   // NW, NE, SW, SE children
    int lo, hi;          // height bounds (propagated from children)
}

Patch : QuadItem {
    uint16_t height[(HEIGHT_CELLS+1)*(HEIGHT_CELLS+1)];  // 25 vertices
    uint16_t visual[VISUAL_CELLS*VISUAL_CELLS];           // 64 cells
    uint16_t diag;
    uint64_t dark;       // shadow bitmask (if DARK_TERRAIN defined)
}
```

### Pointer Ownership Rules

- **Terrain** owns all Patches (allocated via `AddTerrainPatch`, freed via quadtree walk)
- **World** owns all Mesh resources (doubly-linked list) and all Inst variants
- **BSP tree** owned by World, built by `RebuildWorld()`, not directly allocated by callers
- **Mesh geometry** (Vert/Face/Line) owned by Mesh, loaded from .akm
- Multiple `MeshInst` can share the same `Mesh` (reference semantics, share_list linked list)

---

## 3. Known Traps

### TRAP-W01: Soft/Hard Delete Pattern (CRITICAL)
`HardInstDel(i)` alone on a BSP-linked instance leaves **dangling parent pointers**. Correct sequence: `SoftInstDel(i)` then `HardInstDel(i)`. The ancestor cleanup after SoftInstDel is stubbed (empty leaves accumulate, degrading query performance over time).

### TRAP-W02: Format Version Ambiguity
First int32 in world section is ambiguous: negative = format_version (read second int for count), non-negative = legacy count directly. Failing to check the sign misaligns all subsequent reads.

### TRAP-W03: LoadWorld Creates Empty Mesh Stubs
LoadWorld reads mesh references by name but does NOT load geometry. Caller MUST:
1. `LoadWorld(f)` -> World with empty meshes
2. `UpdateMesh(m, "path/to/foo.akm")` per mesh -> load actual geometry
3. `RebuildWorld(world)` -> BSP with valid bounding boxes

**Premature RebuildWorld** (before UpdateMesh) produces incorrect bounding boxes (all zeros).

### TRAP-W04: .ply -> .akm Extension Conversion
LoadWorld silently converts `.ply` suffixes to `.akm` in mesh_id strings. This is a hardcoded `strcpy()` into a local buffer — corrupted file data could overflow.

### TRAP-W05: INST_VOLATILE Counter Mismatch
`temp_insts` is incremented at `CreateInst` time when INST_VOLATILE is set. Manually setting INST_VOLATILE after creation leaves `temp_insts` stale -> instance still appears in save file.

### TRAP-W06: HEIGHT_SCALE = 16 is Baked into .a3d
Changing HEIGHT_SCALE without migrating all .a3d files silently corrupts height data by the scale ratio. No version field in FilePatch to detect mismatch.

### TRAP-W07: Raw Patch Pointer — No Bounds Checking
`GetTerrainHeightMap(p)` and `GetTerrainVisualMap(p)` return raw pointers. Writing beyond `(HEIGHT_CELLS+1)^2` or `VISUAL_CELLS^2` causes silent memory corruption. Always call `UpdateTerrainHeightMap`/`UpdateTerrainVisualMap` after modifying.

### TRAP-W08: TerrainDetach/Attach Are Urdo-Only
These functions bypass normal add/delete and manipulate quadtree internals. Using them outside the urdo system corrupts node counts and height bounds propagation. Comment in terrain.h: "don't use, dedicated to urdo only!"

### TRAP-W09: HitWorld positive_only=false Can Hit Behind Ray
With `positive_only=false`, hits where ray parameter `t < 0` (behind origin) are returned. This produces unexpected results when the ray origin is inside geometry.

### TRAP-W10: Editor vs Game Instance Cloning
`LoadWorld(f, editor=true)` clones item instances (one EDIT, one WORLD purpose). `LoadWorld(f, editor=false)` marks items INST_VOLATILE. Items placed in editor -> play -> reload = items lost (they were volatile).

### TRAP-W11: Ancestor Cleanup is Stubbed
After removing an instance from a BSP leaf, the code that should collapse empty parent nodes is empty (lines 1143-1150 in world.cpp: `// do ancestors cleanup // ...`). Empty BSP_Leaf nodes accumulate, degrading query performance.

### TRAP-W12: Little-Endian Assumed
.a3d magic `*(uint32_t*)"AS3D"` assumes little-endian CPU. No endianness conversion exists anywhere in the load/save paths.

---

## 4. Callgraph

### Game Load Sequence

```
game.cpp: load map
  +-- fopen("a3d/game_map_y8.a3d")
  |
  +-- LoadTerrain(f, &idx) ..................... [terrain.cpp]
  |     +-- Read FileHeader (AS3D magic check)
  |     +-- Per FilePatch (188 bytes each):
  |           +-- AddTerrainPatch(t, x, y, 0)
  |           +-- memcpy visual[], height[]
  |           +-- UpdateTerrainVisualMap(p) + UpdateTerrainHeightMap(p)
  |
  +-- fread(mat[256]) .......................... [material table]
  |
  +-- LoadWorld(f, editor=false) ............... [world.cpp]
  |     +-- CreateWorld() -> empty world
  |     +-- Read format_version (<0 check)
  |     +-- Per instance:
  |           +-- Read mesh_id_len -> dispatch variant
  |           +-- mesh_id >= 0:  CreateInst(Mesh*, flags, tm[16], name, story_id)
  |           +-- mesh_id == -1: CreateInst(World*, Sprite*, flags, pos, yaw, anim, ...)
  |           +-- mesh_id == -2: CreateInst(World*, Item*, flags, pos, yaw, story_id)
  |
  +-- Per Mesh m in world:
  |     +-- UpdateMesh(m, "meshes/foo.akm") ... [load .akm geometry]
  |
  +-- RebuildWorld(world, true) ................ [build BSP]
        +-- SplitBSP(root, instances, bounds)
              +-- Compute variance along X,Y,Z
              +-- Choose split axis, sort, partition
              +-- Recursively build left/right subtrees
              +-- Create BSP_Node / BSP_NodeShare / BSP_Leaf
```

### Frustum-Culled BSP Traversal (render time)

```
QueryWorld(world, planes, &callbacks, cookie)
  +-- RecurseWorldBSP(root, planes)
        +-- BSP_TYPE_NODE:
        |     +-- Test split plane vs frustum
        |     +-- Recurse left/right children
        +-- BSP_TYPE_NODE_SHARE:
        |     +-- Iterate straddling instance list
        |     +-- Recurse left/right children
        +-- BSP_TYPE_LEAF:
        |     +-- Iterate instance linked list
        +-- BSP_TYPE_INST:
              +-- Direct instance test
        Per visible instance:
          +-- mesh_cb(Inst*, Mesh*, tm[16], cookie)  [for mesh instances]
          +-- sprite_cb(Inst*, Sprite*, pos, ..., cookie)  [for sprite instances]
```

### Raycasting (editor picking / collision)

```
HitWorld(world, ray_origin, ray_dir, ret, nrm, ...)
  +-- Encode ray[10] = {cross_product, direction, origin, t_max}
  +-- Select HitWorld0-7 variant based on ray sign bits
  +-- RecurseHitWorld(root)
        +-- AABB-ray intersection test
        +-- BSP_TYPE_INST:
        |     +-- HitFace(MeshInst, ray)
        |     |     +-- Per Face in mesh:
        |     |           +-- Transform vertices via tm[16]
        |     |           +-- RayIntersectsTriangle()
        |     |           +-- Interpolate vertex color at hit point
        |     +-- HitSprite(SpriteInst, ray) [if sprites_too=true]
        +-- BSP_TYPE_LEAF/NODE:
              +-- Recurse, pick closest hit
  +-- Filter INST_VOLATILE if editor=true
  +-- Return closest Inst* or null
```

### Terrain Quadtree Query

```
QueryTerrain(terrain, planes, view_flags, cb, cookie)
  +-- QueryNode(root, level, x, y)
        +-- Compute node AABB from (x, y, level, lo, hi)
        +-- Frustum test (all planes):
        |     OUTSIDE: return (skip entire subtree)
        |     INSIDE:  visit all children without further tests
        |     PARTIAL: recurse children with per-child tests
        +-- If level == 0 (leaf = Patch):
              +-- cb(patch, x, y, view_flags, cookie)
```

### Instance Lifecycle

```
CreateInst(world, sprite, flags, pos, yaw, anim, frame, reps, name, story_id)
  +-- malloc(sizeof(SpriteInst))
  +-- Compute bbox from sprite->proj_bbox + pos
  +-- Set type=BSP_TYPE_INST, flags, bsp_parent=null
  +-- Insert into world->head_inst/tail_inst (flat list)
  +-- Return Inst*
  (Caller must RebuildWorld or AttachInst for BSP insertion)

DeleteInst(inst)
  +-- SoftInstDel(inst)    // unlink from BSP/flat list
  +-- HardInstDel(inst)    // free memory
```

---

## 5. Bevy Mapping

### BSP Tree as Resource (Not Entities)

The BSP tree **MUST** be stored as a Bevy `Resource` (`RuntimeWorld`), NOT as ECS entities. This is a firm architectural decision.

**Rationale:**
- BSP traversal requires recursive descent with strict parent-to-child ordering
- ECS iteration is **unordered** — entity queries cannot express BSP traversal order
- Cache locality: recursive traversal accesses nodes in spatial order, which arena-allocated `Vec<BspNode>` preserves but scattered entity storage destroys
- BSP nodes are not independently queryable game objects — they are internal spatial index structure
- The C++ uses pointer-linked tree nodes; the Rust port uses arena indices into a `Vec`, which is strictly better (no pointer chasing, bounds-checked)

### Terrain Quadtree as Resource

The terrain quadtree follows the same pattern as BSP — stored as a `Resource` (`RuntimeTerrain`), NOT as entities.

**Rationale:**
- Frustum culling requires recursive quadtree descent with early-out on OUTSIDE nodes
- Patch height bounds propagate bottom-up (child to parent) — ECS has no natural parent-child propagation
- Terrain patches are spatially indexed, not independently game-relevant entities
- The quadtree is a read-only spatial index during rendering; only the editor modifies it

### C++ to Bevy Mapping Table

| C++ Construct | Bevy Target | Rationale |
|---------------|-------------|-----------|
| `World*` (global) | `Resource` (`RuntimeWorld`) | Single world, BSP traversal order |
| `Terrain*` (global) | `Resource` (`RuntimeTerrain`) | Single terrain, quadtree traversal order |
| BSP node types (`BSP_TYPE_NODE`, `LEAF`, etc.) | Rust `enum BspNode` in `Vec<BspNode>` | Arena allocation, index-based children |
| `Patch` (quadtree leaf) | `RuntimePatch` in `Vec<RuntimePatch>` | Flat array, indexed by quadtree |
| `Mesh` resource | `Handle<MeshAsset>` or `Resource` | Shared across instances via Bevy asset system |
| `MeshInst` (world instance) | **ECS Entity** with `MeshInstance` + `Transform` components | Multiple instances, individually visible/hidden |
| `SpriteInst` (world instance) | **ECS Entity** with `SpriteInstance` + `Transform` components | Multiple instances, animation state per-entity |
| `ItemInst` (item in world) | **ECS Entity** with `WorldItem` + `Transform` components | Pickable, per-entity interaction |
| `QueryWorld()` callback | Method on `RuntimeWorld` Resource, returns iterator | Called from render system, not a separate system |
| `QueryTerrain()` callback | Method on `RuntimeTerrain` Resource, returns iterator | Called from render system, not a separate system |
| `HitWorld()` raycast | Method on `RuntimeWorld` Resource | Called on-demand (picking, combat), not per-frame |
| `RebuildWorld()` BSP construction | Startup system or on-change system | Runs once at load, not per-frame |
| Instance flags (`INST_VISIBLE`, etc.) | Component flags or marker components on entities | Per-entity, queryable via `With<Visible>` |

### Key Design Decision: World Instances ARE Entities, BSP Is NOT

The BSP tree is a spatial index (internal bookkeeping). World instances (meshes, sprites, items) are game objects with per-entity state (position, visibility, animation). This split is:

- **BSP/Quadtree** -> `Resource` (traversal order matters, no per-node game logic)
- **Instances** -> ECS Entities (per-entity components, queryable, spawn/despawn lifecycle)

The render system reads the `RuntimeWorld` Resource to traverse BSP, but the instances it finds are ECS entity IDs that it can look up via `Query<>`.

### TRAP: Do NOT Make BSP Nodes into Entities

A common ECS beginner mistake is to make every tree node an entity with `Parent`/`Children` components. For BSP this fails because:
1. Bevy's `Parent`/`Children` hierarchy is designed for transform propagation, not spatial indexing
2. Querying children in a specific order (near-side first for front-to-back) requires manual sorting that defeats ECS iteration
3. Adding/removing BSP nodes during rebuild would trigger archetype moves on hundreds of entities per frame
