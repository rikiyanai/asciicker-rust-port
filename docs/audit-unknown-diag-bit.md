# Audit: Terrain Diagonal Bit (diag) Mapping

## Overview

This document details the `diag` bit field in the terrain patch system, which controls triangle diagonal orientation for terrain rendering and collision detection.

## Location

**Source file**: `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp`

**Patch struct** (line 193-208):
```cpp
struct Patch : QuadItem
{
#ifdef DARK_TERRAIN
    uint64_t dark; // (8x8)
#endif
    uint16_t visual[VISUAL_CELLS][VISUAL_CELLS];
    uint16_t height[HEIGHT_CELLS + 1][HEIGHT_CELLS + 1];
    uint16_t diag; // (4x4)
#ifdef TEXHEAP
    TexAlloc* ta;
#endif
};
```

## Bit to Cell Mapping

The `diag` field is a 16-bit unsigned integer where each bit controls the diagonal orientation of one height cell in the 4x4 grid.

**Constants** (from `terrain.h`):
- `HEIGHT_CELLS = 4` - Number of cells along each axis
- `VISUAL_CELLS = 8` - Visual resolution (not directly related to diag)

### Bit Position Formula

```cpp
// x = column (0-3), y = row (0-3)
bit_index = x + (y * HEIGHT_CELLS)
mask = 1 << bit_index
```

### Complete Bit Mapping Table

| Bit | Mask (hex) | Cell (x, y) |
|-----|------------|-------------|
| 0   | 0x0001     | (0, 0)      |
| 1   | 0x0002     | (1, 0)      |
| 2   | 0x0004     | (2, 0)      |
| 3   | 0x0008     | (3, 0)      |
| 4   | 0x0010     | (0, 1)      |
| 5   | 0x0020     | (1, 1)      |
| 6   | 0x0040     | (2, 1)      |
| 7   | 0x0080     | (3, 1)      |
| 8   | 0x0100     | (0, 2)      |
| 9   | 0x0200     | (1, 2)      |
| 10  | 0x0400     | (2, 2)      |
| 11  | 0x0800     | (3, 2)      |
| 12  | 0x1000     | (0, 3)      |
| 13  | 0x2000     | (1, 3)      |
| 14  | 0x4000     | (2, 3)      |
| 15  | 0x8000     | (3, 3)      |

### Visual Representation

```
Patch layout (4x4 cells, 5x5 vertices):

    y=0   y=1   y=2   y=3
     |     |     |     |
x=0--[ 0 ]-[ 4 ]-[ 8 ]-[12]--
     |     |     |     |
x=1--[ 1 ]-[ 5 ]-[ 9 ]-[13]--
     |     |     |     |
x=2--[ 2 ]-[ 6 ]-[10]-[14]--
     |     |     |     |
x=3--[ 3 ]-[ 7 ]-[11]-[15]--

[0-15] = bit indices for each cell
```

## Diagonal Orientation

Each terrain cell (quad) is split into two triangles. The `diag` bit determines which diagonal is used:

### When bit is SET (1)
- **Diagonal**: NW-SE (from vertex at (x, y) to vertex at (x+1, y+1))
- **Triangles**: 
  - Lower: v[2], v[0], v[1] 
  - Upper: v[2], v[1], v[3]

### When bit is CLEAR (0)  
- **Diagonal**: NE-SW (from vertex at (x, y+1) to vertex at (x+1, y))
- **Triangles**:
  - Lower: v[0], v[3], v[2]
  - Upper: v[0], v[1], v[3]

### Vertex Numbering

```
Cell at (hx, hy) with vertices:
  v[0] = (hx,   hy)   - top-left
  v[1] = (hx+1, hy)   - top-right
  v[2] = (hx,   hy+1) - bottom-left  
  v[3] = (hx+1, hy+1) - bottom-right

When diag=1 (NW-SE diagonal):
  /|
 /_|
    ' '
  Triangles: (v2,v0,v1) and (v2,v1,v3)

When diag=0 (NE-SW diagonal):
   .-
   |\
   |_\
  '  '
  Triangles: (v0,v3,v2) and (v0,v1,v3)
```

## SetDiag Function

Located in the `Tap3x3` class (lines 429-468 in terrain.cpp).

### Signature
```cpp
void SetDiag(int x, int y, bool d)
```

### Purpose
Sets the diagonal flag for a cell, handling cross-patch boundaries automatically. The `Tap3x3` class wraps a 3x3 grid of patches (the center patch plus its 8 neighbors).

### Implementation Details

```cpp
void SetDiag(int x, int y, bool d)
{
    int px = 1, py = 1;  // Default to center patch

    // Handle boundary wrapping for x coordinate
    if (x < 0) {
        x += HEIGHT_CELLS;  // Wrap to neighbor patch
        px = 0;              // Left neighbor
    } else if (x >= HEIGHT_CELLS) {
        x -= HEIGHT_CELLS;   // Wrap to neighbor patch
        px = 2;              // Right neighbor
    }

    // Handle boundary wrapping for y coordinate
    if (y < 0) {
        y += HEIGHT_CELLS;  // Wrap to neighbor patch
        py = 0;              // Bottom neighbor
    } else if (y >= HEIGHT_CELLS) {
        y -= HEIGHT_CELLS;   // Wrap to neighbor patch
        py = 2;              // Top neighbor
    }

    // Set or clear the bit in the appropriate patch
    if (p[py][px]) {
        if (d)
            p[py][px]->diag |= 1 << (x + y * HEIGHT_CELLS);
        else
            p[py][px]->diag &= ~(1 << (x + y * HEIGHT_CELLS));
    }
}
```

### Auto-Computation

The `Tap3x3::Update()` method automatically computes diagonal orientation based on height gradients:

```cpp
void Update()
{
    for (int y = -1; y <= HEIGHT_CELLS; y++)
    {
        for (int x = -1; x <= HEIGHT_CELLS; x++)
        {
            // Compute height derivatives (c0 = horizontal, c1 = vertical tendency)
            int c0 = ...; // height change along one axis
            int c1 = ...; // height change along other axis
            
            // Use whichever axis has stronger gradient
            SetDiag(x, y, my_abs(c0) > my_abs(c1));
        }
    }
}
```

## Public API

### GetDiag
```cpp
uint16_t GetTerrainDiag(Patch* p)  // terrain.cpp:1608-1611
```
Returns the current diag bitfield for a patch.

### SetDiag  
```cpp
void SetTerrainDiag(Patch* p, uint16_t diag)  // terrain.cpp:1613-1616
```
Sets the entire diag bitfield for a patch. Use XOR with individual bit masks to toggle specific cells.

### Editor Usage (asciiid.cpp)
```cpp
// Toggle diagonal for cell at (hx, hy)
uint16_t diag = GetTerrainDiag(p);
diag ^= 1 << (hx + hy * HEIGHT_CELLS);
SetTerrainDiag(p, diag);
```

## Data Persistence

The `diag` field is saved to and loaded from `.a3d` files:

- **Save**: `fwrite(&p->diag, 1, sizeof(int16_t), f);` (terrain.cpp:3115)
- **Load**: `p->diag = pch.diag;` (terrain.cpp:3229)

## Usage in Raycasting

The diagonal bit is used in `HitPatch()` (terrain.cpp:2007-2092) and `HitTerrain()` to determine which triangles to test for intersection:

```cpp
int rot = p->diag;
for (int hy = 0; hy < HEIGHT_CELLS; hy++) {
    for (int hx = 0; hx < HEIGHT_CELLS; hx++) {
        if (rot & 1) {
            // Test NW-SE diagonal triangles
            RayIntersectsTriangle(ray, v[2], v[0], v[1], ...);
            RayIntersectsTriangle(ray, v[2], v[1], v[3], ...);
        } else {
            // Test NE-SW diagonal triangles
            RayIntersectsTriangle(ray, v[0], v[3], v[2], ...);
            RayIntersectsTriangle(ray, v[0], v[1], v[3], ...);
        }
        rot >>= 1;
    }
}
```

## Usage in Rendering

The diagonal bit determines triangle orientation in `render.cpp` (around line 1882-2020):

```cpp
uint16_t diag = GetTerrainDiag(p);

for (int dy = 0; dy < HEIGHT_CELLS; dy++) {
    for (int dx = 0; dx < HEIGHT_CELLS; dx++, diag >>= 1) {
        if (diag & 1) {
            // Render with NW-SE diagonal
            ...
        } else {
            // Render with NE-SW diagonal  
            ...
        }
    }
}
```

## Summary

- **Field**: `uint16_t diag` in Patch struct
- **Size**: 16 bits (one per 4x4 height cell)
- **Bit formula**: `1 << (x + y * 4)` where x,y are cell coordinates
- **Set=1**: NW-SE diagonal (top-left to bottom-right)
- **Clear=0**: NE-SW diagonal (bottom-left to top-right)
- **Auto-compute**: `Tap3x3::Update()` calculates based on height gradients
- **Manual toggle**: XOR with `1 << (hx + hy * HEIGHT_CELLS)`
