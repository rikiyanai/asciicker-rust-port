# Audit: Unknown Mesh Reference Resolution

**Date:** 2026-02-20  
**Source:** `/Users/r/Downloads/asciicker-Y9-2/`  
**Context:** Rust port research - understanding mesh reference loading mechanism

---

## 1. Overview

This document audits how Asciicker resolves mesh references when loading `.a3d` world files. Mesh references are stored as string identifiers (mesh names) in the binary format, with actual geometry loaded separately from `.akm` mesh files.

---

## 2. Mesh Reference in .a3d Files

### Binary Layout (from `world.cpp` lines 5000-5200)

Each mesh instance in an `.a3d` file stores:

```
int32  mesh_id_len     (4 bytes)   -- length of mesh name string
char   mesh_id[mesh_id_len]        -- mesh name/ID (no null terminator)
int32  inst_name_len  (4 bytes)   -- instance name length
char   inst_name[...]              -- instance name
double tm[16]        (128 bytes)  -- 4x4 column-major transform
int32  flags          (4 bytes)   -- instance flags
int32  story_id       (4 bytes)   -- story/quest ID
```

**Type Discriminant:**
- `mesh_id_len >= 0`: MESH instance (references a mesh)
- `mesh_id_len == -1`: SPRITE instance
- `mesh_id_len == -2`: ITEM instance

### Instance Type Detection (world.cpp:5049)

```cpp
if (mesh_id_len >= 0)
{
    // MESH: read mesh_id string, create mesh reference
}
else if (mesh_id_len == -1)
{
    // SPRITE: read sprite name
}
else if (mesh_id_len == -2)
{
    // ITEM: read item proto index
}
```

---

## 3. Mesh Reference Loading in World.cpp

### 3.1 LoadWorld() Function (lines 5008-5239)

The `LoadWorld()` function performs a **two-phase loading**:

**Phase 1: Create Empty Mesh Stubs**
- Reads mesh name from `.a3d` file
- Creates `Mesh*` with name only (no geometry)
- Looks up existing mesh by name: `strcmp(m->name, mesh_id)`
- If not found, adds new empty mesh: `w->AddMesh(mesh_id)`

```cpp
// mesh id lookup (world.cpp:5115-5121)
Mesh* m = w->head_mesh;
while (m && strcmp(m->name, mesh_id))
    m = m->next;

if (!m)
    m = w->AddMesh(mesh_id);  // Creates empty stub
```

**Phase 2: Caller Reloads Geometry**
- After `LoadWorld()` returns, caller must reload mesh geometry from `.akm` files
- This is a deliberate design: `.a3d` stores references, not geometry

### 3.2 Mesh Data Structure (world.cpp:227-263)

```cpp
struct Mesh
{
    World* world;
    char* name;        // mesh_id from .a3d file
    void* cookie;      // user data (MeshPrefs in editor)
    
    Mesh* next;
    Mesh* prev;
    
    TYPE type;         // MESH_TYPE_2D or MESH_TYPE_3D
    
    int faces;
    Face* head_face;
    Face* tail_face;
    
    int lines;
    int verts;
    // ... vertex/face/line lists ...
    
    float bbox[6];     // untransformed bounding box
    
    MeshInst* share_list;  // instances sharing this mesh
};
```

### 3.3 MeshInst Structure (world.cpp:321-327)

```cpp
struct MeshInst : Inst
{
    Mesh* mesh;        // POINTER to shared Mesh
    double tm[16];     // world transform matrix
    
    MeshInst* share_next;  // next instance sharing same mesh
};
```

**Key Point:** `MeshInst::mesh` is a direct pointer. If the mesh is NULL or invalid, rendering/collision will fail.

---

## 4. Geometry Reloading (The "Missing Mesh" Problem)

### 4.1 When Geometry Is Reloaded

The caller reloads mesh geometry AFTER `LoadWorld()`. This happens in multiple places:

**Editor (asciiid.cpp:662-683):**
```cpp
// reload meshes too
Mesh* m = GetFirstMesh(merge._world);
while (m)
{
    char obj_path[4096];
    sprintf(obj_path, "%smeshes/%s", base_path, mesh_name);
    
    if (!UpdateMesh(m, obj_path))
    {
        // what now?
        // missing mesh file!
    }
    m = GetNextMesh(m);
}
```

### 4.2 Known Fallback Behavior

The code has several hardcoded fallbacks for legacy/corrupted mesh references:

**Legacy Extension Transform (world.cpp:5062-5063):**
```cpp
if (mesh_id_len>=4 && strcmp(mesh_id+mesh_id_len-4,".ply")==0)
    strcpy(mesh_id+mesh_id_len-4,".akm");  // .ply -> .akm
```

**Untitled Mesh Fallback (world.cpp:5107-5111):**
```cpp
if (strstr(mesh_id,"untitled"))
{
    strcpy(mesh_id,"tree-3.akm");  // Hardcoded fallback!
}
```

**NOTE:** This fallback is currently commented out in the code but documented here for completeness.

---

## 5. Load Order Dependencies

### 5.1 Required Load Sequence

1. **Create World:** `CreateWorld()`
2. **Load Terrain:** `LoadTerrain(f)` - reads patches first
3. **Load Instances:** `LoadWorld(f, editor)` - creates empty mesh stubs
4. **Reload Mesh Geometry:** `UpdateMesh(m, path)` - loads .akm files
5. **Rebuild BSP:** `RebuildWorld(w, true)` - rebuilds spatial index

### 5.2 Mesh Path Resolution

Mesh files are resolved relative to `base_path`:

```
base_path + "meshes/" + mesh_name
```

Where `mesh_name` is the string from `.a3d` (e.g., `"tree-1.akm"`).

---

## 6. Reference Counting / Sharing

Multiple instances can share the same mesh (world.cpp:845-851):

```cpp
if (m)
{
    i->share_next = m->share_list;
    m->share_list = i;  // Add to share list
}
```

When a mesh is deleted, all sharing instances are deleted (world.cpp:649-652):
```cpp
// kill sharing insts
Inst* i = m->share_list;
while (m->share_list)
    DelInst(m->share_list);
```

---

## 7. Godot Importer Handling

The Godot importer (`godot_project/addons/asciicker_importer/import_a3d.gd unknown`) handles mesh references differently:

**Instance Mapping (lines 299-304):**
```gdscript
var instance_map = {
    "tree": "res://assets/props/tree_1.tscn",
    "rock": "res://assets/props/rock_1.tscn",
    "Tree": "res://assets/props/tree_1.tscn",
    "Rock": "res://assets/props/rock_1.tscn"
}
```

**Unknown Mesh Fallback (lines 320-334):**
```gdscript
if node_3d == null:
    # Unmapped instances get a RED SEMI-TRANSPARENT BOX
    var mi = MeshInstance3D.new()
    var box = BoxMesh.new()
    box.size = Vector3(1, 1, 1)
    mi.mesh = box
    
    var red_mat = StandardMaterial3D.new()
    red_mat.albedo_color = Color(1, 0, 0, 0.5)
    red_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
    mi.material_override = red_mat
```

**Metadata Preservation (lines 343-345):**
```gdscript
node_3d.set_meta("mesh_source", mesh_name)
node_3d.set_meta("story_id", story_id)
node_3d.set_meta("flags", flags)
```

---

## 8. Rust Port Considerations

### 8.1 Key Insights for Implementation

1. **Two-Phase Loading:** Mesh references are strings in `.a3d`, geometry comes from separate `.akm` files
2. **Lazy Loading:** Geometry is loaded after instance creation, not during
3. **Name-Based Lookup:** `strcmp()` on mesh name strings
4. **Pointer Stability:** `MeshInst::mesh` is a direct pointer - mesh must outlive instance
5. **Share List:** Multiple instances can share one mesh via linked list

### 8.2 Potential Issues

| Issue | Location | Impact |
|-------|----------|--------|
| Missing .akm file | `UpdateMesh()` returns false | Empty mesh, no rendering |
| Null mesh pointer | `MeshInst::mesh == NULL` | Crash on render |
| Invalid transform | `tm[16]` corruption | Incorrect positioning |
| Case sensitivity | `"Tree"` vs `"tree"` | Different instances in Godot |

### 8.3 Recommended Approach

For Rust port, consider:

1. **Deferred Mesh Loading:** Store mesh names, load geometry on demand
2. **Handle Missing Files:** Return `Option<Mesh>` or `Result`, don't panic
3. **Metadata Storage:** Store original `mesh_source` string for debugging
4. **Fallback Strategy:** Log warnings for unknown meshes, don't silently fail

---

## 9. References

- **Source:** `/Users/r/Downloads/asciicker-Y9-2/world.cpp` (lines 5000-5300)
- **Format Spec:** `/Users/r/Downloads/asciicker-Y9-2/io_asciicker/scene/a3d_format.py`
- **Editor Loading:** `/Users/r/Downloads/asciicker-Y9-2/asciiid.cpp` (lines 631-697)
- **Godot Importer:** `/Users/r/Downloads/asciicker-Y9-2/godot_project/addons/asciicker_importer/import_a3d.gd`

---

*End of audit.*
