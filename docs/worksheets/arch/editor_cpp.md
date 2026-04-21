# Asciicker Editor System (asciiid.cpp + urdo.cpp)

The Asciicker editor (asciiid) is a comprehensive map editing application built on top of the game engine. It provides real-time 3D editing of terrain, materials, meshes, sprites, items, and enemy spawners through an ImGui-based UI overlaying an OpenGL 3.3+/4.5 renderer.

---

## 1. Editor Modes and Tools

The editor operates in eight distinct modes controlled by the `edit_mode` variable (asciiid.cpp:1767), each accessible via tabbed interface in the "Brush" panel.

### Mode 0: SCULPT — Terrain Height Editing

The SCULPT mode modifies the terrain height map using various brush types and operations.

**Brush Shapes** (asciiid.cpp:8091):
- **Gaussian**: Smooth falloff from center (default)
- **Square**: Hard-edged brush
- **Noise**: Randomized falloff pattern

**Operations** (asciiid.cpp:8068-8085):
- **Ascent**: Raises terrain when `br_alpha > 0`
- **Descent**: Lowers terrain when `br_alpha < 0`
- **Blur**: Smooths height transitions (Shift key)
- **Sharpens**: Increases height contrast (Shift + negative alpha)
- **Height Probe**: Samples terrain height at cursor (Ctrl+Shift)
- **Diagonal Flip**: Flips terrain triangle diagonals (Ctrl key)
- **Multi-tile**: Creates/deletes patches in radius (Alt key)

**Brush Parameters**:
- `br_radius`: Brush radius in world units (5.0 - 100.0)
- `br_alpha`: Strength/direction (-0.5 to +0.5)
- `br_tile_radius`: Patch creation radius (0.5 - 20.0)
- `br_limit`: Enable height-based filtering

---

### Mode 1: MAT-id — Material ID Painting

Paints material IDs (0-255) onto terrain visual cells.

**Operations**:
- **Paint**: Applies selected material ID to terrain cells
- **Paint Above**: Only paint cells above height threshold
- **Paint Below**: Only paint cells below height threshold (Shift key)
- **Material Probe**: Samples material ID at cursor (Ctrl key)
- **Height Probe**: Samples terrain height (Ctrl+Shift)

**Auto-Material Functions**:
- `ApplyAutoMatElev()`: Assigns elevation ramps based on slope/height
- `ApplyAutoTexture()`: Applies materials based on slope and height ranges
- `ClearMatElev()`: Resets elevation ramp assignments

---

### Mode 3: MAT-elev — Material Elevation Ramps

Controls which of the 4 elevation ramps is used for each terrain cell. Each material has 4 ramps for different vertical slopes.

**Elevation Values**:
- 0/1 (top): Steep upward slope
- 1/1 (upper): Moderate upward slope
- 1/0 (lower): Moderate downward slope
- 0/0 (bottom): Flat or slight downward slope

---

### Mode 2: MESH — 3D Mesh Instance Placement

Places 3D mesh instances from the mesh library (.akm files).

**Mesh Preferences** (MeshPrefs struct, asciiid.cpp:472-496):

**Scale Controls**:
- `scale_val[3]`: Base scale [X, Y, Z]
- `scale_rnd[3]`: Random scale variation [X, Y, Z]

**Rotation Controls**:
- `rotate_locZ_val`: Rotation around Z axis (degrees)
- `rotate_locZ_rnd`: Random Z rotation variation
- `rotate_XY_val[2]`: Rotation around X/Y axes
- `rotate_XY_rnd[2]`: Random X/Y rotation variation
- `rotate_align`: Terrain normal alignment (0-1)

**Height Offset**:
- `height`: Vertical offset above terrain

**Operations**:
- **Insert Mesh**: Click to place mesh instance
- **Delete Mesh**: Ctrl+click to remove instance
- **Add/Remove Tiles**: Alt+click to create/delete patches

**Bake Functions** (asciiid.cpp:8402-8423):
- `BakeMeshesToTerrain()`: Transfers mesh geometry to terrain
  - Bake height: Imprints mesh vertex heights onto terrain
  - Bake material: Maps mesh vertex colors to terrain materials
  - Bake vertex colors: Creates new materials from mesh colors
  - Ray top: Maximum height for raycasting (1000-120000)

---

### Mode 4: SPRITE — 2D Sprite Placement

Places animated 2D billboard sprites from the sprite library (.xp files).

**Sprite Preferences** (SpritePrefs struct, asciiid.cpp:354-370):

- `yaw`: Rotation angle (0-360 degrees)
- `anim`: Animation index
- `frame`: Specific frame (when t[] all zero)
- `t[4]`: Animation timing [rep_first, rep_fwd, rep_last, rep_back]
- `height`: Vertical offset above terrain
- `rand_anim`: Randomize animation on placement
- `rand_frame`: Randomize starting frame
- `rand_yaw`: Randomize rotation

**Operations**:
- **Insert Sprite**: Click to place sprite instance
- **Delete Sprite**: Ctrl+click to remove instance

---

### Mode 5: ITEM — Inventory Item Placement

Places inventory items from the item prototype library.

**Item Types**:
- Weapon (W)
- Shield (S)
- Helmet (H)
- Armor (A)
- Potion (P)
- Food (F)
- Door (D)

**Operations**:
- **Insert Item**: Click to place item
- **Delete Item**: Ctrl+click to remove
- **RESET items**: Rebuilds world item instances from prototypes

---

### Mode 6: ENEMYGEN — Enemy Spawner Placement

Places enemy spawn point generators that spawn NPCs during gameplay.

**Spawner Parameters**:
- `eg_alive_max`: Maximum simultaneous enemies (1-7)
- `eg_revive_min`: Minimum revive time exponent (2^n seconds, 0-10)
- `eg_revive_max`: Maximum revive time exponent
- `eg_armor`: Enemy armor level (0-10)
- `eg_helmet`: Enemy helmet level (0-10)
- `eg_shield`: Enemy shield level (0-10)
- `eg_sword`: Sword probability (0-10, inverse of crossbow)
- `eg_crossbow`: Crossbow probability (0-10, inverse of sword)

**Operations**:
- **Delete All Generators**: Removes all enemy spawners

---

### Mode 7: STORY — Story Element Tracking

Associates story IDs with placed objects for game scripting integration.

**Operations**:
- Click on any object to assign current story_id
- Tracks: meshes, sprites, items, enemy generators

---

## 2. Undo/Redo System (URDO)

The URDO system (urdo.cpp, 898 lines) provides comprehensive undo/redo for all editor operations through a doubly-linked list with cursor architecture.

### Architecture

```
[op1] <-> [op2] <-> [op3] <-> [op4] <-> [op5]
                      ^undo     ^redo
```

- **undo pointer**: Last executed operation (can be undone)
- **redo pointer**: Next undone operation (can be redone)
- **New operations**: Appended after undo pointer, redo chain is purged

### Group Nesting (Stack-Based, 64 Levels)

- `URDO_Open()`: Pushes new group onto stack, starts collecting operations
- `URDO_Close()`: Pops group, seals as single undo unit

Example: A merge operation creates a group containing:
```
[GROUP] -> [PATCH_CREATE] -> [PATCH_UPDATE_HEIGHT] -> [PATCH_UPDATE_VISUAL]
```

### Operation Types (Six Types)

| Type | Description | Data Stored |
|------|-------------|-------------|
| `CMD_GROUP` | Nested group (atomically undone/redone) | group_head, group_tail pointers |
| `CMD_PATCH_CREATE` | Create/delete terrain patch | terrain, patch, cx, cy, attached |
| `CMD_PATCH_UPDATE_HEIGHT` | Height map snapshot (SWAP) | patch, height[5][5], diag |
| `CMD_PATCH_UPDATE_VISUAL` | Visual map snapshot (SWAP) | patch, visual[8][8] |
| `CMD_PATCH_DIAG` | Diagonal flag snapshot (SWAP) | patch, diag |
| `CMD_INST_CREATE` | Create/delete mesh/sprite instance | inst, attached |

### The SWAP Pattern

For height and visual updates, URDO uses a **swap pattern** rather than copy:

```cpp
// URDO_PatchUpdateHeight::Do() - O(1) swap
void URDO_PatchUpdateHeight::Do(bool un)
{
    uint16_t* t = GetTerrainHeightMap(patch);
    uint16_t* u = (uint16_t*)height;
    for (int i = 0; i < (HEIGHT_CELLS + 1)*(HEIGHT_CELLS + 1); i++)
    {
        uint16_t s = t[i];
        t[i] = u[i];
        u[i] = s;
    }
    // ... same for diagonal
}
```

**Why SWAP**:
- O(1) operation (no allocation)
- Minimal memory (single snapshot stored)
- Undo and redo are identical (both swap)

### Memory Tracking

- `bytes` counter: Total memory used by undo/redo history
- Per-operation size tracking in `Free()`
- Detached patches/instances freed when operation freed

### Public API (urdo.h)

```cpp
bool URDO_CanUndo();
bool URDO_CanRedo();
size_t URDO_Bytes();
void URDO_Purge();
void URDO_Undo(int max_depth);  // 0=one leaf, 64=all
void URDO_Redo(int max_depth);
void URDO_Open();
void URDO_Close();

// Patch operations
Patch* URDO_Create(Terrain* t, int x, int y, int z);
void URDO_Delete(Terrain* t, Patch* p);
void URDO_Patch(Patch* p, bool visual = false);
void URDO_Diag(Patch* p);

// Instance operations
Inst* URDO_Create(World* w, Sprite* s, ...);
Inst* URDO_Create(Mesh* m, ...);
void URDO_Delete(Inst* i);
```

---

## 3. World Editing Functionality

### Terrain System (terrain.h)

The terrain uses a **quadtree** structure with discrete patches:

- **HEIGHT_CELLS = 4**: 5x5 vertex grid per patch
- **VISUAL_CELLS = 8**: 8x8 material grid per patch
- **HEIGHT_SCALE = 16**: Z-steps per visual cell

**Terrain API**:
```cpp
Terrain* CreateTerrain(int z = -1);
void DeleteTerrain(Terrain* t);

Patch* AddTerrainPatch(Terrain* t, int x, int y, int z);
bool DelTerrainPatch(Terrain* t, int x, int y);

uint16_t* GetTerrainHeightMap(Patch* p);
uint16_t* GetTerrainVisualMap(Patch* p);

void UpdateTerrainHeightMap(Patch* p);
void UpdateTerrainVisualMap(Patch* p);

uint16_t GetTerrainDiag(Patch* p);
void SetTerrainDiag(Patch* p, uint16_t diag);

void QueryTerrain(Terrain* t, ...);  // Frustum culled iteration
Patch* HitTerrain(Terrain* t, ...);   // Raycasting
```

### World System (world.h)

The world manages mesh, sprite, and item instances in a **BSP tree** for efficient spatial queries.

**Instance Types**:
- **MeshInst**: Static 3D mesh with transform matrix
- **SpriteInst**: Animated 2D billboard
- **ItemInst**: Inventory item instance

**World API**:
```cpp
World* CreateWorld();
void DeleteWorld(World* w);
void RebuildWorld(World* w, bool boxes = false);

// Mesh resources
Mesh* LoadMesh(World* w, const char* path, const char* name = 0);
bool UpdateMesh(Mesh* m, const char* path);

// Instances
Inst* CreateInst(World* w, Sprite* s, ...);
Inst* CreateInst(Mesh* m, ...);
Inst* CreateInst(World* w, Item* item, ...);
void DeleteInst(Inst* i);

void SoftInstAdd(Inst* i);   // Add to BSP (undoable)
void SoftInstDel(Inst* i);   // Remove from BSP (undoable)
void HardInstDel(Inst* i);   // Permanent delete

// Queries
void QueryWorld(World* w, ...);  // Frustum culled
Inst* HitWorld(World* w, ...);   // Raycasting
```

### Instance Flags

```cpp
enum INST_FLAGS {
    INST_VISIBLE   = 0x1,  // Render instance
    INST_USE_TREE = 0x2,  // BSP tree inclusion
    INST_VOLATILE = 0x4,   // Temporary (NPCs, projectiles)
    INST_SELECTED = 0x8    // Editor selection
};
```

---

## 4. Asset Editing

### Material System (MyMaterial, asciiid.cpp:835-1217)

256 material slots, each with:

- **4 elevation ramps**: For different vertical slopes
- **16 shade levels**: Per ramp for lighting variations
- **Each cell**: Background color, foreground color, ASCII glyph

**Material Initialization**:
```cpp
struct MyMaterial : Material {
    static void Init();     // Load defaults or generate
    void Update();          // Upload to GPU
    static GLuint tex;      // 128x256 texture atlas
};
```

**Default Materials** (asciiid.cpp:872-1147):
- Material 0: Water
- Material 1: Grass
- Material 2: Dirt
- Material 3: Stone
- Material 4: Sand
- Material 5: Snow
- Material 6: Mud
- Material 7: Cobblestone
- Material 8: Gravel
- Materials 9-255: Available for custom use

### Mesh Assets (.akm files)

Exported from Blender via `io_mesh_akm` addon. Format includes:
- Vertex positions and colors
- Face definitions
- Per-vertex material IDs

**Loading**:
```cpp
Mesh* LoadMesh(World* w, "meshes/Tree.akm");
// Or reload after Blender export:
bool UpdateMesh(m, "meshes/Tree.akm");
```

### Sprite Assets (.xp files)

2D billboard sprites with animation support:
- Multiple animations per sprite
- Frame-based animation with timing
- Palette-based coloring

### Item System

Items are defined in the item prototype library and placed as instances. The editor maintains two copies:
- **Editor items**: Placed in editor world
- **World items**: Cloned for gameplay

**Reset Function**:
```cpp
void ResetItemInsts(World* w);  // Rebuild from prototypes
```

---

## 5. File I/O and Map Formats

### .a3d Format (AS3D Binary)

The editor saves maps in a custom binary format:

```
Header (16 bytes):
  - "AS3D" magic (4 bytes)
  - Header size (4 bytes)
  - Number of patches (4 bytes)
  - Reserved (4 bytes)

Per Patch (188 bytes):
  - Position (x, y, z): 12 bytes
  - Visual map: 8*8*2 = 128 bytes
  - Height map: 5*5*2 = 50 bytes
  - Diagonal: 2 bytes
  - Reserved: 4 bytes

Materials (after terrain):
  - 256 materials * 4 ramps * 16 shades * sizeof(MatCell)

World (after materials):
  - Mesh references and instance transforms
  - Sprite instances
  - Item instances
  - Enemy generators
```

### Map Merging (asciiid.cpp:503-732)

The `Merge` struct enables importing terrain and meshes from another .a3d file:

```cpp
void MergeOpen(const char* path);  // Load external map
void MergeCommit();                 // Apply merge (single undo unit)
void MergeCancel();                 // Cancel merge
```

**Merge Strategy**:
- Terrain: Max-height merge (tallest from both maps wins)
- Meshes: Name-matched with offset translation

---

## 6. Rendering Pipeline

The editor uses OpenGL 3.3+/4.5 with embedded GLSL shaders (RenderContext, asciiid.cpp:1506-2700):

1. **Terrain rendering**: Height map + visual material grid
2. **Mesh rendering**: Per-vertex colored 3D geometry
3. **Sprite rendering**: Billboard quads with animation
4. **Shadow pass**: Updates terrain dark mask from mesh occlusion

---

## 7. Integration Points

### External Dependencies

- **Dear ImGui** (v1.69): UI framework
- **OpenGL 3.3+/4.5**: 3D rendering
- **Blender + io_mesh_akm**: Mesh asset creation

### Key Internal Dependencies

- **terrain.h**: Quadtree terrain management
- **world.h**: BSP spatial indexing
- **sprite.h**: Sprite loading/rendering
- **urdo.h**: Undo/redo integration
- **render.h**: AnsiCell format, Material struct

---

## Summary

The Asciicker editor is a monolithic 10,655-line application that provides comprehensive world-building capabilities:

| Component | Files | Purpose |
|-----------|-------|---------|
| Editor UI | asciiid.cpp | 8 editing modes, ImGui panels |
| Undo/Redo | urdo.cpp | 6 operation types, group nesting, swap pattern |
| Terrain | terrain.h/cpp | Quadtree patches, height/visual maps |
| World | world.h/cpp | BSP tree, mesh/sprite/item instances |
| Materials | asciiid.cpp:835-1217 | 256 slots, 4 ramps x 16 shades |
| Rendering | RenderContext | OpenGL 3.3+/4.5 + embedded GLSL |

All terrain and instance modifications flow through the URDO system, ensuring full undo/redo capability for every edit operation.
