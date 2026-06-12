# Asciicker .a3d World File Format

This document details the binary .a3d world file format extracted from the C++ source code in `/Users/r/Downloads/asciicker-Y9-2/`.

## Table of Contents

1. [World Loader Function](#world-loader-function)
2. [Header Layout](#header-layout)
3. [Instance Structures](#instance-structures)
4. [EnemyGen Format](#enemygen-format)
5. [Story ID Handling](#story-id-handling)
6. [Binary Format Summary](#binary-format-summary)

---

## World Loader Function

### Location
- **Source file**: `world.cpp`
- **Load function**: Line 5008-5239
- **Save function**: Line 4971-5001

### Function Signatures

```cpp
// [DATA-CONTRACT:A3D] Deserializes world from .a3d binary.
// WHY editor param: Controls item instance cloning behavior.
//   editor==true  clones items for test-players
//   editor==false changes items purpose directly for player(s)
World* LoadWorld(FILE* f, bool editor);

// [DATA-CONTRACT:A3D] Serializes world state (mesh references + instance data) to binary.
void SaveWorld(World* w, FILE* f);
```

### Save Flow (from world.cpp:4971-5001)

```cpp
void SaveWorld(World* w, FILE* f)
{
    int format_version = -1;
    // Write header: format_version (int32, value -1)
    fwrite(&format_version, 1, 4, f);

    // Write header: num_of_instances (int32, 4 bytes)
    // Excludes temp_insts (volatile runtime objects not persisted to disk)
    int num_of_instances = w->insts - w->temp_insts;
    fwrite(&num_of_instances,1,4,f);

    // Save non-BSP instances first (flat list)
    Inst* i = w->head_inst;
    while (i)
    {
        SaveInst(i,f);
        i=i->next;
    }

    // Then save BSP tree instances
    if (w->root)
        SaveQueryBSP(w->root,f);
}
```

---

## Header Layout

### Magic Number and Version Detection

The format uses a clever versioning scheme where negative first int32 indicates versioned format:

```cpp
// [DATA-CONTRACT:A3D] Read header: first int32 is either format_version (<0) or
// num_of_instances (>=0, legacy format). WHY check sign: backward compatibility.

int num_of_instances = 0;
fread(&num_of_instances, 4, 1, f);

int format_version = 0; // all till y4

if (num_of_instances < 0)
{
    format_version = -num_of_instances;  // e.g., -1 becomes version 1
    fread(&num_of_instances, 4, 1, f);   // then read actual instance count
}
```

### Header Structure

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `format_version` | int32 | 4 bytes | **Negative** value indicating format version. Current: `-1`. If positive, legacy format (no version field). |
| `num_of_instances` | int32 | 4 bytes | Count of instances to follow (excludes volatile temp objects) |

**Version History** (from comments at line 4976-4978):
- **Version -1**: Adds `format_version` and per-instance `story_id`

---

## Instance Structures

### Base Inst Structure

From `world.cpp` lines 301-319:

```cpp
struct Inst : BSP
{
    enum INST_TYPE
    {
        MESH = 1,
        SPRITE = 2,
        ITEM = 3
    };

    INST_TYPE inst_type;
    char* name;
    int story_id;  // Narrative/gameplay identifier

    // in BSP_Leaf::inst / BSP_NodeShare::inst
    Inst* next;
    Inst* prev;    

    int /*FLAGS*/ flags; 
};
```

### Instance Flags

From `world.h` lines 145-151:

```cpp
enum INST_FLAGS
{
    INST_VISIBLE = 0x1,    // Instance is rendered
    INST_USE_TREE = 0x2,   // Instance participates in BSP tree
    INST_VOLATILE = 0x4,   // Temporary runtime objects (NPCs, projectiles)
    INST_SELECTED = 0x8    // Editor selection highlight
};
```

---

### MeshInst (Static Geometry)

From `world.cpp` lines 321-421:

```cpp
struct MeshInst : Inst
{
    Mesh* mesh;
    double tm[16];  // 4x4 transform matrix, column-major, ABSOLUTE world coords

    MeshInst* share_next;  // Next instance sharing same mesh

    void UpdateBox();      // Compute world-space bounding box
    bool HitFace(...);     // Ray intersection testing
};
```

**Binary Format** (from SaveInst, lines 4833-4862):

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `mesh_id_len` | int32 | 4 | Length of mesh name string (>=0 for mesh) |
| `mesh_id` | char[] | mesh_id_len | Mesh name/path string |
| `inst_name_len` | int32 | 4 | Length of instance name |
| `inst_name` | char[] | inst_name_len | Instance name string |
| `tm` | double[16] | 128 | 4x4 transform matrix (column-major) |
| `flags` | int32 | 4 | Instance flags |
| `story_id` | int32 | 4 | Story ID (-1 if none) |

---

### SpriteInst (Billboard Sprites)

From `world.cpp` lines 513-530:

```cpp
struct SpriteInst : Inst
{
    World* w;
    Sprite* sprite;
    void* data;        // player(human) or creature or null
    int anim;          // Current animation index
    int frame;         // Current frame within animation
    int reps[4];      // Palette remapping indices
    float yaw;         // Y rotation angle (degrees)
    float pos[3];     // World position XYZ

    bool Hit(double ray[10], double ret[3], bool positive_only);
};
```

**Binary Format** (from SaveInst, lines 4864-4887):

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `mesh_id_len` | int32 | 4 | **-1** (identifies sprite instance) |
| `inst_name_len` | int32 | 4 | Length of sprite name |
| `inst_name` | char[] | inst_name_len | Sprite name string |
| `pos` | float[3] | 12 | World position XYZ |
| `yaw` | float | 4 | Y rotation angle |
| `anim` | int32 | 4 | Animation index |
| `frame` | int32 | 4 | Frame within animation |
| `reps` | int32[4] | 16 | Palette remapping |
| `flags` | int32 | 4 | Instance flags |
| `story_id` | int32 | 4 | Story ID |

---

### ItemInst (Inventory Items)

From `world.cpp` lines 549-563:

```cpp
struct ItemInst : Inst
{
    World* w;
    Item* item;
    float yaw;       // Y rotation angle
    float pos[3];   // World position XYZ

    bool Hit(double ray[10], double ret[3], bool positive_only);
};
```

**Binary Format** (from SaveInst, lines 4889-4916):

| Field | Type | Size | Description |
|-------|------|------|-------------|
| `mesh_id_len` | int32 | 4 | **-2** (identifies item instance) |
| `item_proto_index` | int32 | 4 | Index into `item_proto_lib` array |
| `count` | int32 | 4 | Item stack count |
| `pos` | float[3] | 12 | World position XYZ |
| `yaw` | float | 4 | Y rotation angle |
| `flags` | int32 | 4 | Instance flags |
| `story_id` | int32 | 4 | Story ID |

---

## EnemyGen Format

### Structure Definition

From `enemygen.h` lines 11-53:

```cpp
struct EnemyGen
{
    EnemyGen* next;   // Next spawn point in global linked list
    EnemyGen* prev;   // Previous spawn point

    float pos[3];     // World position XYZ where NPCs spawn

    // Population Parameters
    int alive_max;    // Max simultaneous NPCs (1-7)

    // Revive Timer (exponential backoff: 2^revive_min to 2^revive_max seconds)
    int revive_min;   // Min revive exponent (0-10)
    int revive_max;   // Max revive exponent (0-10)

    // Equipment Probabilities (0-10 scale: fast_rand() % 11 < value)
    int armor;        // Armor probability
    int helmet;       // Helmet probability  
    int shield;       // Shield probability

    // Weapon Weights (weighted random: fast_rand() % (sword + crossbow + 1) < sword)
    int sword;        // Sword weight (0-10)
    int crossbow;     // Crossbow weight (0-10)
};
```

### Binary Format

From `enemygen.cpp` lines 201-211:

| Offset | Field | Type | Size | Description |
|--------|-------|------|------|-------------|
| 0 | `pos` | float[3] | 12 | World position XYZ |
| 12 | `alive_max` | int32 | 4 | Max simultaneous NPCs (1-7) |
| 16 | `revive_min` | int32 | 4 | Min revive exponent (2^n seconds) |
| 20 | `revive_max` | int32 | 4 | Max revive exponent |
| 24 | `armor` | int32 | 4 | Armor probability (0-10) |
| 28 | `helmet` | int32 | 4 | Helmet probability (0-10) |
| 32 | `shield` | int32 | 4 | Shield probability (0-10) |
| 36 | `sword` | int32 | 4 | Sword weight for weapon choice |
| 40 | `crossbow` | int32 | 4 | Crossbow weight for weapon choice |

**Total: 44 bytes per EnemyGen spawn point**

### Load/Save Functions

From `enemygen.cpp` lines 219-292:

```cpp
// Load from .a3d binary (line 219)
void LoadEnemyGens(FILE* f)
{
    FreeEnemyGens();
    
    int num = 0;
    fread(&num, 4, 1, f);
    
    for (int i = 0; i < num; i++)
    {
        EnemyGen* eg = (EnemyGen*)malloc(sizeof(EnemyGen));
        
        fread(eg->pos, sizeof(float), 3, f);
        fread(&eg->alive_max, sizeof(int), 1, f);
        fread(&eg->revive_min, sizeof(int), 1, f);
        fread(&eg->revive_max, sizeof(int), 1, f);
        fread(&eg->armor, sizeof(int), 1, f);
        fread(&eg->helmet, sizeof(int), 1, f);
        fread(&eg->shield, sizeof(int), 1, f);
        fread(&eg->sword, sizeof(int), 1, f);
        fread(&eg->crossbow, sizeof(int), 1, f);
        
        // Insert at head of linked list (O(1))
        eg->prev = 0;
        eg->next = enemygen_head;
        if (enemygen_head)
            enemygen_head->prev = eg;
        enemygen_head = eg;
    }
}

// Save to .a3d binary (line 263)
void SaveEnemyGens(FILE* f)
{
    int num = 0;
    EnemyGen* eg = enemygen_head;
    while (eg) { num++; eg = eg->next; }
    
    fwrite(&num, 4, 1, f);
    
    eg = enemygen_head;
    while (eg)
    {
        fwrite(eg->pos, sizeof(float), 3, f);
        fwrite(&eg->alive_max, sizeof(int), 1, f);
        fwrite(&eg->revive_min, sizeof(int), 1, f);
        fwrite(&eg->revive_max, sizeof(int), 1, f);
        fwrite(&eg->armor, sizeof(int), 1, f);
        fwrite(&eg->helmet, sizeof(int), 1, f);
        fwrite(&eg->shield, sizeof(int), 1, f);
        fwrite(&eg->sword, sizeof(int), 1, f);
        fwrite(&eg->crossbow, sizeof(int), 1, f);
        eg = eg->next;
    }
}
```

---

## Story ID Handling

### Inst Story ID Field

From `world.cpp` line 312:

```cpp
struct Inst : BSP
{
    // ...
    int story_id;  // Narrative/gameplay identifier
    // ...
};
```

### API Functions

From `world.cpp` lines 5265-5280:

```cpp
// Get story ID for an instance
int GetInstStoryID(Inst* i)
{
    return i->story_id;
}

// Set story ID for an instance (for GAMEPLOT/EDITOR)
void SetInstStoryID(Inst* i, int id) 
{
    if (i)
        i->story_id = id;
}
```

### Story ID in Binary Format

- **Version -1**: Each instance has a 4-byte `story_id` at the end
- **Legacy format**: No story_id field (format_version == 0)

The story_id is serialized/deserialized in SaveInst/LoadInst for all instance types:
- MeshInst: line 4861 (`fwrite(&i->story_id, 1, 4, f);`)
- SpriteInst: line 4886
- ItemInst: line 4914

**Purpose**: Links instances to the game plot/scripting system. Each placed object can be referenced by script via its story_id for quest triggers, dialogue, and item interactions.

---

## Binary Format Summary

### Complete .a3d File Layout

```
┌─────────────────────────────────────────────────────────────┐
│ HEADER                                                       │
│   format_version: int32   (currently -1, negative = newer) │
│   num_of_instances: int32                                     │
├─────────────────────────────────────────────────────────────┤
│ INSTANCES (repeat num_of_instances times)                   │
│                                                             │
│ Mesh Instance (mesh_id_len >= 0):                          │
│   mesh_id_len: int32                                        │
│   mesh_id: char[mesh_id_len]                                │
│   inst_name_len: int32                                      │
│   inst_name: char[inst_name_len]                            │
│   tm: double[16] (128 bytes, column-major 4x4)             │
│   flags: int32                                              │
│   story_id: int32 (version -1 only)                        │
│                                                             │
│ Sprite Instance (mesh_id_len == -1):                        │
│   mesh_id_len: int32 (= -1)                                 │
│   inst_name_len: int32                                      │
│   inst_name: char[inst_name_len]                            │
│   pos: float[3] (12 bytes)                                  │
│   yaw: float                                                │
│   anim: int32                                               │
│   frame: int32                                              │
│   reps: int32[4]                                            │
│   flags: int32                                              │
│   story_id: int32 (version -1 only)                        │
│                                                             │
│ Item Instance (mesh_id_len == -2):                          │
│   mesh_id_len: int32 (= -2)                                 │
│   item_proto_index: int32                                   │
│   count: int32                                              │
│   pos: float[3]                                             │
│   yaw: float                                                │
│   flags: int32                                              │
│   story_id: int32 (version -1 only)                        │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ ENEMY SPAWN POINTS (EnemyGen)                               │
│   count: int32                                              │
│   [repeat count times:]                                      │
│     pos: float[3] (12 bytes)                                │
│     alive_max: int32                                        │
│     revive_min: int32                                        │
│     revive_max: int32                                        │
│     armor: int32                                            │
│     helmet: int32                                            │
│     shield: int32                                           │
│     sword: int32                                             │
│     crossbow: int32                                          │
│   Total: 4 + (count * 44) bytes                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Source Files

| File | Purpose |
|------|---------|
| `world.cpp` | World/Instance management, LoadWorld/SaveWorld |
| `world.h` | Public API declarations |
| `enemygen.h` | EnemyGen spawn point structure |
| `enemygen.cpp` | EnemyGen load/save functions |

---

## Notes

1. **Double precision transforms**: Mesh transforms use `double tm[16]` to preserve accuracy for large worlds (terrain extends thousands of units) and avoid cumulative float errors in raycasting.

2. **Volatile instances**: Objects marked `INST_VOLATILE` (NPCs, projectiles) are NOT saved to .a3d files - they're runtime-only.

3. **Legacy format**: Files with positive first int32 are legacy format (no version, no story_id).

4. **Mesh name extension**: On load, `.ply` is automatically converted to `.akm` (lines 5062-5063):
   ```cpp
   if (mesh_id_len>=4 && strcmp(mesh_id+mesh_id_len-4,".ply")==0)
       strcpy(mesh_id+mesh_id_len-4,".akm");
   ```

5. **Item cloning**: In editor mode (`editor==true`), items are cloned: one stays as EDIT purpose, one becomes WORLD purpose for test-playing.
