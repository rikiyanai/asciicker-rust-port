# Audit: Unknown Terrain Quadtree Expansion Logic

**Source File**: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/terrain.cpp`  
**Date**: 2026-02-20  
**Status**: COMPLETE

---

## Summary

This document captures the findings from research into the terrain quadtree expansion logic in the original asciicker C++ codebase. The analysis covers patch creation/expansion, any thresholds or limits, and the "grow upward" strategy mentioned in audit notes.

---

## 1. Terrain Patch Creation/Expansion

### Primary Function: `AddTerrainPatch()`

**Location**: `terrain.cpp` lines 771-1303

The `AddTerrainPatch()` function is the main entry point for creating terrain patches:

```cpp
Patch* AddTerrainPatch(Terrain* t, int x, int y, int z)
```

**Parameters**:
- `t`: Pointer to the Terrain structure
- `x, y`: World coordinates for the patch
- `z`: Initial height value for the patch

### Patch Creation Flow

1. **Empty Terrain (First Patch)**: If `t->root` is NULL (empty terrain), the first patch becomes the root at level 0 (lines 777-820)

2. **Coordinate Translation**: The input coordinates are adjusted by the terrain's base offset (lines 829-830):
   ```cpp
   x += t->x;
   y += t->y;
   ```

3. **Tree Expansion**: If coordinates are outside current bounds, the tree automatically expands

4. **Tree Descent**: Once in bounds, traverse from root to leaf, creating intermediate Nodes as needed (lines 1037-1299)

5. **Patch Initialization**: Create the Patch, initialize height map, set neighbor flags, interpolate edges

---

## 2. Thresholds and Limits on Patch Creation

### Explicit Limits

**No hard-coded limit exists** for the number of patches that can be created. The system is designed to expand infinitely.

### Implicit/Cractical Limits

1. **Integer Overflow (Line 838)**: 
   ```cpp
   int range = 1 << t->level;
   ```
   - With 32-bit signed integers, maximum tree level is 31
   - Maximum world size: `2^31` patches in each dimension
   - This is a theoretical limit that would never be reached in practice

2. **Memory**: Actual limit is available system memory (RAM)

3. **TEXHEAP Capacity** (when `TEXHEAP` defined):
   - `TERRAIN_TEXHEAP_CAPACITY` = 1024 / max(VISUAL_CELLS, HEIGHT_CELLS+1)
   - With default values (VISUAL_CELLS=8, HEIGHT_CELLS=4): 1024/8 = 128 patches per texture page
   - This is a GPU texture allocation limit, not a patch count limit

### Constants (from terrain.h)

| Constant | Value | Description |
|----------|-------|-------------|
| HEIGHT_CELLS | 4 | Vertices per axis - 1 (5x5 vertex grid) |
| VISUAL_CELLS | 8 | Material cells per axis (8x8 grid) |
| HEIGHT_SCALE | 16 | Z-steps per visual cell |

---

## 3. "Grow Upward" Logic

### Description

The "grow upward" strategy is explicitly documented in the codebase at multiple locations:

**Lines 768-770** (function comment):
```cpp
// The quadtree automatically expands upward if the patch is outside current bounds. This "grow upward"
// strategy avoids the need for a fixed maximum world size - the tree can expand infinitely in any direction.
```

**Lines 832-836** (expansion logic comment):
```cpp
// [FLOW:WORLD] Tree insertion -- parent split, quadrant calculation, auto-expand if out of bounds
//
// WHY auto-expand upward: When a new patch is added outside current quadtree bounds,
// we create new parent nodes above the current root. This allows infinite world expansion
// without pre-allocating a fixed maximum size. The root "grows upward" rather than failing.
```

### Expansion Algorithm

The expansion happens via **four while loops** (lines 842-1035), each handling a boundary condition:

#### 1. Expand Left (x < 0) - Lines 842-894
```cpp
while (x < 0)
{
    // Create new parent node
    // Place old root in appropriate quadrant based on y position
    // Double the range
}
```

#### 2. Expand Down (y < 0) - Lines 896-945
```cpp
while (y < 0)
{
    // Create new parent node
    // Place old root in appropriate quadrant based on x position
    // Double the range
}
```

#### 3. Expand Right (x >= range) - Lines 947-990
```cpp
while (x >= range)
{
    // Create new parent node
    // Place old root based on y position
    // Double the range
}
```

#### 4. Expand Up (y >= range) - Lines 992-1035
```cpp
while (y >= range)
{
    // Create new parent node
    // Place old root based on x position
    // Double the range
}
```

### Key Expansion Details

1. **Each iteration adds one tree level** (`t->level++`)

2. **Each iteration doubles spatial coverage** (`range *= 2`)

3. **Quadrant Selection**: The old root is placed in quadrant[1] or quadrant[3] (for y expansion) or quadrant[0] or quadrant[2] (for x expansion) based on whether its coordinates are in the lower or upper half of the new parent's domain

4. **Offset Adjustment**: The terrain base offset (`t->x`, `t->y`) is adjusted to maintain coordinate consistency

5. **Parent Linking**: The old root's parent pointer is updated, and it becomes a child of the new root

### Debug Output

The expansion logic includes debug output when `ASCIICKER_TERRAIN_DEBUG` environment variable is set:
```cpp
fprintf(stderr, "[AddTerrainPatch] expand x<0 range=%d x=%d y=%d\n", range, x, y);
```

---

## 4. Key Data Structures

### Terrain (lines 210-221)
```cpp
struct Terrain
{
    int x, y;          // worldspace origin from tree origin
    int level;         // 0 -> root is patch, -1 -> empty
    QuadItem* root;    // Node or Patch or NULL
    int nodes;         // count of internal nodes
    int patches;       // count of leaf patches
};
```

### Node (lines 188-191)
```cpp
struct Node : QuadItem
{
    QuadItem* quad[4]; // all 4 are same type (Nodes or Patches)
};
```

### Patch (lines 193-208)
```cpp
struct Patch : QuadItem
{
    uint16_t visual[VISUAL_CELLS][VISUAL_CELLS];  // 8x8 material grid
    uint16_t height[HEIGHT_CELLS + 1][HEIGHT_CELLS + 1]; // 5x5 vertex grid
    uint16_t diag; // triangle diagonal orientation (4x4)
};
```

---

## 5. Implications for Rust Port

### Recommendations

1. **Implement Infinite Expansion**: The Rust port should replicate the "grow upward" behavior for infinite world support

2. **Consider Memory Limits**: While there's no hard limit, the Rust implementation should consider:
   - Adding configurable maximum world size for safety
   - Implementing chunk/unload strategies for very large worlds

3. **Maintain Coordinate System**: The offset-based coordinate translation is critical - ensure Rust port maintains the same semantics

4. **Debug Infrastructure**: Consider adding similar debug environment variable controls for development

5. **Integer Type Considerations**: Using `i64` instead of `i32` would extend the theoretical maximum from 2^31 to 2^63 patches

---

## References

- `terrain.cpp`: Full implementation (lines 771-1303)
- `terrain.h`: Header with constants and public API
- Existing audit: `docs/worksheets/audit-reaudit-terrain.md`
