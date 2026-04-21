# Audit: .a3d Save File Format

**Status:** AUDIT COMPLETE  
**Source:** `/Users/rikihernandez/Downloads/Aciicker-Y9-2/`  
**Date:** 2026-02-20

---

## Executive Summary

The .a3d format is a binary file format for Asciicker game world persistence. It stores terrain patches, materials, mesh/sprite/item instances, and enemy generators. The format has a version system using a negative `format_version` field to distinguish legacy files from modern ones.

---

## 1. File Structure Overview

The .a3d file is composed of several sequential sections:

| Section | Size | Description |
|---------|------|-------------|
| Terrain Header | 16 bytes | Magic signature + patch count |
| Terrain Patches | 188 bytes each | Height map + visual/material map |
| Materials | 131,072 bytes (256 x 512) | Material palette with shade ramps |
| World Header | 8 bytes | format_version + instance count |
| Instances | Variable | Mesh/sprite/item instances |
| Enemy Generators | 44 bytes each | Enemy spawn points |

---

## 2. Magic Bytes & Version

### Magic Signature (Terrain Section)
- **Location:** Bytes 0-3 of file
- **Value:** `"AS3D"` (0x41, 0x53, 0x33, 0x44 in little-endian)
- **Defined in:** `terrain.cpp` line 3148: `*(uint32_t*)"AS3D"`

### Format Version (World Section)
- **Location:** First int32 after materials
- **Current Version:** `-1` (negative = versioned format)
- **Legacy Behavior:** If first int32 >= 0, it's directly the instance count (no version field)

**Version Detection Logic** (`world.cpp` lines 5019-5038):
```cpp
int num_of_instances = 0;
if (1 != fread(&num_of_instances, 4, 1, f)) { /* error */ }

int format_version = 0; // all till y4

if (num_of_instances < 0)
{
    format_version = -num_of_instances;  // Version is negative of first value
    if (1 != fread(&num_of_instances, 4, 1, f)) { /* error */ }
}
```

**Version History:**
| Version | Value | Description |
|---------|-------|-------------|
| Legacy | N/A | No version field; first int32 is directly `num_of_instances` |
| Current | -1 | Added `format_version` field before instance count; added `story_id` per instance |

---

## 3. Version Conditional Logic

### In SaveWorld (world.cpp:4971-5001)
```cpp
void SaveWorld(World* w, FILE* f)
{
    int format_version = -1;  // Always writes version -1
    
    // Header: format_version (int32, 4 bytes, value -1)
    fwrite(&format_version, 1, 4, f);
    
    // Header: num_of_instances (int32, 4 bytes)
    int num_of_instances = w->insts - w->temp_insts;
    fwrite(&num_of_instances, 1, 4, f);
    
    // ... serialize instances
}
```

### In LoadWorld (world.cpp:5008-5238)
```cpp
// format_version > 0 check for story_id field:
if (format_version > 0)
{
    if (1 != fread(&story_id, 4, 1, f)) { /* error */ }
}
```

The loader conditionally reads `story_id` only when `format_version > 0`. Currently, all saved files use version -1, so this is always read.

---

## 4. Detailed Binary Layout

### 4.1 Terrain Header (16 bytes)
```
Offset  Size  Type     Description
------  ----  ------   -----------
0       4     uint32   Magic: "AS3D" (0x44335341)
4       4     uint32   header_size (always 16)
8       4     uint32   num_patches
12      4     uint32   reserved (0)
```

### 4.2 Terrain Patch (188 bytes each)
```
Offset  Size      Type          Description
------  ----      -----------   -----------
0       4        int32         x (world coordinate)
4       4        int32         y (world coordinate)
8       128      uint16[8][8]  visual map (material IDs)
136     50       uint16[5][5]  height map (vertices)
186     2        uint16         diag (triangle split bitfield)
```

### 4.3 Material Palette (131,072 bytes)
- 256 materials x 512 bytes each
- Each material: 4 elevation ramps x 16 shade steps x 8 bytes per MatCell
- MatCell layout:
  - Bytes 0-2: Foreground RGB
  - Byte 3: Glyph (ASCII code)
  - Bytes 4-6: Background RGB
  - Byte 7: Flags

### 4.4 World Section
```
Offset  Size   Type     Description
------  ----   ------   -----------
0       4      int32    format_version (<0 = versioned, >=0 = legacy count)
4       4      int32    num_of_instances
8+      varies  varies   Instance records
```

### 4.5 Instance Variants (Discriminated by mesh_id_len)

**Mesh Instance (mesh_id_len >= 0):**
```
mesh_id_len    (4 bytes)    - Length of mesh name
mesh_id        (n bytes)    - Mesh name string (no null terminator)
inst_name_len  (4 bytes)    - Length of instance name
inst_name      (m bytes)    - Instance name string
transform      (128 bytes)  - 4x4 matrix (16 doubles, column-major)
flags          (4 bytes)    - INST_VISIBLE | INST_USE_TREE | etc.
story_id       (4 bytes)    - Narrative ID (-1 = none)
```

**Sprite Instance (mesh_id_len == -1):**
```
mesh_id_len    = -1         (4 bytes, discriminant)
inst_name_len  (4 bytes)    - Length of sprite name
inst_name      (m bytes)    - Sprite name
pos[3]         (12 bytes)  - Position (float[3])
yaw            (4 bytes)    - Rotation (float)
anim           (4 bytes)    - Animation index
frame          (4 bytes)    - Frame index
reps[4]        (16 bytes)   - Repetitions
flags          (4 bytes)    - Instance flags
story_id       (4 bytes)    - Narrative ID
```

**Item Instance (mesh_id_len == -2):**
```
mesh_id_len    = -2         (4 bytes, discriminant)
item_proto_idx (4 bytes)    - Item prototype index
count          (4 bytes)    - Item count
pos[3]         (12 bytes)  - Position
yaw            (4 bytes)    - Rotation
flags          (4 bytes)    - Instance flags
story_id       (4 bytes)    - Narrative ID
```

### 4.6 Enemy Generator (44 bytes each)
```
Offset  Size   Type     Description
------  ----   ------   -----------
0       12     float    pos[3] (XYZ)
12      4      int32    alive_max
16      4      int32    revive_min
20      4      int32    revive_max
24      4      int32    armor
28      4      int32    helmet
32      4      int32    shield
36      4      int32    sword
40      4      int32    crossbow
```

---

## 5. Key Constants

| Constant | Value | Location | Notes |
|----------|-------|----------|-------|
| `VISUAL_CELLS` | 8 | terrain.h:66 | 8x8 material grid per patch |
| `HEIGHT_CELLS` | 4 | terrain.h:60 | 5x5 vertices per patch |
| `HEIGHT_SCALE` | 16 | terrain.h:54 | Z-units per visual cell |

> NOTE: WATER_LEVEL and BASE_TERRAIN_HEIGHT were previously listed here but do NOT exist as named constants in terrain.h. The values 0x8000 and 0xA000 appear as inline literals in other files.

**CRITICAL:** Changing `HEIGHT_SCALE` breaks all existing .a3d files (see TRAP-W06 in world-loading.md).

---

## 6. Reference Implementation Files

### C++ Engine
- **Terrain save/load:** `terrain.cpp` lines 3083-3280
  - `SaveTerrain()` at line 3141
  - `LoadTerrain()` at line 3165
- **World save/load:** `world.cpp` lines 4828-5239
  - `SaveWorld()` at line 4971
  - `LoadWorld()` at line 5008
  - `SaveInst()` at line 4828
  - `SaveQueryBSP()` at line 4919

### Python (Blender Addon)
- **Format definitions:** `io_asciicker/scene/a3d_format.py`
  - `A3DHeader` class
  - `A3DPatch` class
  - `A3DMaterial` class
  - `A3DInstance` class
  - `A3DEnemyGen` class
- **Exporter:** `io_asciicker/scene/export_a3d.py`
  - `save_a3d()` function

---

## 7. Known Format Issues (Traps)

From `docs/worksheets/skills/world-loading.md`:

| Trap | Description |
|------|-------------|
| TRAP-W02 | Format version ambiguity: first int32 sign determines interpretation |
| TRAP-W06 | HEIGHT_SCALE=16 is baked into .a3d; changing breaks all files |
| TRAP-W12 | Little-endian assumed; no endianness conversion exists |

---

## 8. Existing .a3d Files

Sample files in `/Users/rikihernandez/Downloads/Aciicker-Y9-2/a3d/`:
- `game_map_y7.a3d` - Original game map (legacy format?)
- `game_map_y8.a3d` - Current game map
- `game_map_y8_original_game_map.a3d` - Backup of original
- `test_map.a3d` - Test map
- `minimal_1x1.a3d`, `minimal_2x2.a3d` - Minimal test cases

---

## 9. Rust Port Recommendations

### For Rust Implementation:

1. **Endianness:** Only little-endian is supported (matching x86/ARM)

2. **Version Handling:**
   - Read first int32
   - If < 0: it's `format_version`, read next int32 for instance count
   - If >= 0: it's directly instance count (legacy format)

3. **Instance Discrimination:**
   - `mesh_id_len >= 0`: Mesh instance
   - `mesh_id_len == -1`: Sprite instance
   - `mesh_id_len == -2`: Item instance

4. **Material Palette:**
   - Always 256 materials x 512 bytes
   - Must read even if not used (determines instance offset)

5. **HEIGHT_SCALE:** Do NOT change this value (16) or existing files will corrupt

6. **Enemy Generator Offset:** After instances, read int32 count, then that many 44-byte records

---

## 10. Sources

- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/terrain.cpp` (lines 3083-3280)
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/world.cpp` (lines 4828-5239)
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/io_asciicker/scene/a3d_format.py`
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/io_asciicker/scene/export_a3d.py`
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/docs/worksheets/skills/world-loading.md`
