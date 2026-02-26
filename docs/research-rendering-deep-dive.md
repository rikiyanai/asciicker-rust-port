> **STATUS: ACTIVE REFERENCE WITH CORRECTIONS** — C++ rendering analysis. CORRECTIONS: HEIGHT_CELLS=4 not 8 (terrain.h:60), vertex grid is 5×5 not 9×9, Z is UP not Y (physics.h:41).

# Asciicker Rendering System - Deep Dive Research

This document contains comprehensive technical analysis of the core rendering subsystems for porting Asciicker to Rust.

---

## Table of Contents
1. [Rendering Pipeline](#1-rendering-pipeline)
2. [Sprite System](#2-sprite-system)
3. [World/BSP System](#3-worldbsp-system)
4. [Terrain Quadtree](#4-terrain-quadtree)

---

## 1. Rendering Pipeline

### Data Structures

#### Sample Structure (Core Rendering Element)
```cpp
struct Sample {
    float height;      // Depth value - Z-buffer depth (signed, negative = closer)
    uint16_t visual;   // RGB555 color (5 bits per channel: RRRR RGGGG GBBBB)
    uint8_t diffuse;   // Lighting value (0-255, represents illumination intensity)
    uint8_t spare;     // Flags/overlay data
};
```

**Memory Layout:** 8 bytes per sample

**Field Details:**
| Field | Type | Bits/Range | Purpose |
|-------|------|------------|---------|
| `height` | float | 32-bit IEEE 754 | Depth value. Negative = closer to viewer |
| `visual` | uint16_t | 15 bits (RGB555) | Packed color: bits[14:10]=R, bits[9:5]=G, bits[4:0]=B |
| `diffuse` | uint8_t | 0-255 | Per-vertex/per-face diffuse lighting |
| `spare` | uint8_t | 8 bits | Flags: bit 2 (0x04) = grid line, bit 3 (0x08) = mesh |

#### AnsiCell Structure (Output Buffer Element)
```cpp
struct AnsiCell {
    uint8_t fg;    // Foreground color: 256-color xterm palette index (0-255)
    uint8_t bk;    // Background color: 256-color xterm palette index (0-255)
    uint8_t gl;    // Glyph: CP437 character code (0-255)
};
```

**Memory Layout:** 3 bytes (tightly packed)

#### SampleBuffer Structure
```cpp
struct SampleBuffer {
    Sample* ptr;    // Pointer to sample array (2× supersampled: dw × dh)
    int w;          // Width in samples (2× output cell width)
    int h;          // Height in samples (2× output cell height)
};
```

**Key Assumption:** The SampleBuffer is 2× supersampled relative to output AnsiCell resolution.

### Rendering Pipeline (6 Stages)

#### Stage 1: CLEAR
- Clear sample_buffer to background color
- Set all Sample::height to -1000000.0f (far)
- Set all Sample::visual, diffuse, spare to 0

#### Stage 2: TERRAIN
```
Render() 
  → QueryTerrain(t, patch_x, patch_y, RenderPatch, r)
    → RenderPatch(Patch*, x, y, view_flags, cookie)
      → GetTerrainHeightMap(p)
      → GetTerrainVisualMap(p)
      → For each cell in patch:
        → GetTerrainDiag(p, cell_x, cell_y)
        → Transform 4 vertices to screen coords
        → Compute diffuse lighting
        → Rasterize triangles
```

#### Stage 3: WORLD
```
Render()
  → QueryWorld(w, inst, RenderMesh, RenderSprite_v1, r)
    → For each MeshInst: RenderMesh → QueryMesh → RenderFace → Rasterize
    → For each SpriteInst: RenderSprite_v1 → ProjectCoords → Append to queue
```

#### Stage 4: SHADOW
- Compute player blob shadow projection
- Inverse transform screen samples to world
- Test against terrain height

#### Stage 5: REFLECTION (conditional)
- Set global_refl_mode = true
- QueryTerrain/QueryWorld with flipped Z
- Vertex order flipped for correct face winding

#### Stage 6: RESOLVE
- For each output cell (x, y):
  - Read 2×2 sample block from SampleBuffer
  - Compute average visual (RGB555)
  - Apply auto_mat lookup for mesh or terrain color mapping
  - Write AnsiCell to output buffer

#### Stage 7: SPRITES
- Sort sprites far-to-near
- For each sprite: RenderSprite_v2 with depth test against SampleBuffer

### Color Quantization (RGB555 → xterm256)

#### create_auto_mat Function
- Input: RGB555 color (15 bits, 32,768 combinations)
- Output: 3 bytes per entry: {fg_color, bg_color, glyph}

**Algorithm:**
1. Extract r, g, b from RGB555
2. Find best pair of xterm cube vertices (6×6×6 = 216 colors)
3. Compute shading value based on diffuse lighting (0-11 levels)
4. Map shading to glyph: " ..::%%"

**Xterm 256-color cube:** Indices 16-231 form 6×6×6 cube
- Index formula: `16 + r*36 + g*6 + b` where r,g,b ∈ [0,5]

### 1/W Perspective Correction Math

**Mathematical Foundation:**
```cpp
// For parameter t in [0, 1] along the line:
// Perspective-correct:
ka = ka / ((1-t)*d_from + t*d_to)
```

Where `((1-t)*d_from + t*d_to)` is the harmonic mean of depths, correctly accounting for foreshortening.

### Magic Numbers and Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| HEIGHT_SCALE/2 | varies | Z-fighting tolerance |
| HEIGHT_SCALE/8 | varies | Water plane tolerance |
| 0x10000 | 65536 | Triangle area rejection (16.16 fixed-point) |
| 0x04 | 4 | Grid line flag |
| 0x08 | 8 | Mesh flag |
| -1000000.0f | background | Default cleared depth |

---

## 2. Sprite System

### XP File Format

#### Binary Layout
```
+------------------+
|   GZIP Header    |  10+ bytes (RFC 1952)
+------------------+
|   Deflate Data   |  Variable length
+------------------+
```

#### GZIP Header (RFC 1952)
```cpp
struct GZ {
    uint8_t  id1;    // 0x1f (31)
    uint8_t  id2;    // 0x9b (139)
    uint8_t  cm;     // 8 = deflate
    uint8_t  flg;    // Flags
    uint8_t  mtime[4]; // Modification time
    uint8_t  xfl;    // Extra flags
    uint8_t  os;     // Operating system
};
```

#### REXPaint/XP Header (After Decompression)
| Field | Type | Size |
|-------|------|------|
| version | uint32_t | 4 bytes |
| num_layers | uint32_t | 4 bytes |
| width | uint32_t | 4 bytes |
| height | uint32_t | 4 bytes |

**Total Header:** 16 bytes

#### XPCell Binary Layout (10 bytes packed)
```cpp
struct XPCell {
    uint32_t glyph;    // 4 bytes: CP437 code point
    uint8_t  fg[3];    // 3 bytes: RGB888 foreground
    uint8_t  bk[3];    // 3 bytes: RGB888 background
};
```

**CRITICAL:** Column-major ordering (not row-major!)

### Layer Semantics

| Layer | Index | Purpose |
|-------|-------|---------|
| Layer 0 | 0 | Color key / Grid layout, animation metadata |
| Layer 1 | 1 | Height encoding ('0'-'9' = 0-9, 'A'-'Z' = 10-35) |
| Layer 2 | 2 | Visual data - primary sprite content |
| Layers 3+ | 3+ | Swoosh overlays |

### BlitSprite Algorithm

**Clipping and Compositing:**
1. Source background is transparent (255): Average glyph with destination
2. Source foreground is transparent: Average glyph with destination  
3. Both opaque: Direct cell copy

### DitherSprite Algorithm

**4×4 Dither Matrix:**
```cpp
static const int sprite_dither_matrix[4][4] = {
    { 1,  9,  3, 11},
    {13,  5, 15,  7},
    { 4, 12,  2, 10},
    {16,  8, 14,  6}
};
```

Cells where `matrix[y&3][x&3] <= dither_level` are skipped.

### Color Quantization (RGB888 → xterm256)

```cpp
int r_level = (rgb[0] + 25) / 51;  // 0-255 → 0-5
int g_level = (rgb[1] + 25) / 51;
int b_level = (rgb[2] + 25) / 51;
int palette_index = 16 + 36 * r_level + 6 * g_level + b_level;
```

### Reference Counting

```cpp
void FreeSprite(Sprite* spr) {
    spr->refs--;
    if (spr->refs == 0) {
        // Free frame cells, atlas, anim arrays, name, struct
        free(spr);
    }
}
```

---

## 3. World/BSP System

### BSP Tree Types
```cpp
enum BSP_TYPE {
    BSP_TYPE_NODE = 0,        // Interior node with 2 children
    BSP_TYPE_NODE_SHARE = 1,  // Interior node + straddling instance list
    BSP_TYPE_LEAF = 2,        // Leaf node with instance list
    BSP_TYPE_INST = 3,        // Single instance promoted directly
};
```

### Instance Types

#### MeshInst
```cpp
struct MeshInst {
    Mesh* mesh;              // Pointer to shared mesh geometry
    double tm[16];           // Column-major 4×4 transform matrix
    char* name;              // Optional name
    float bbox[6];           // World-space bounding box
};
```

#### SpriteInst
```cpp
struct SpriteInst {
    Sprite* sprite;          // Sprite atlas
    float pos[3];            // World position {x, y, z}
    float yaw;               // Rotation around Y axis
    int anim, frame;        // Animation state
    int reps[4];            // Repetition counts
};
```

#### ItemInst
```cpp
struct ItemInst {
    Item* item;              // Item prototype
    float pos[3];            // World position
    float yaw;               // Y rotation
};
```

### Surface Area Heuristic (SAH) Algorithm

**Steps:**
1. Base case: If only 1 item, promote to leaf
2. For each axis (X, Y, Z):
   - Sort items by centroid
   - Compute cumulative bboxes
   - Calculate SAH cost: `SA(left) * N(left) + SA(right) * N(right)`
3. Choose best axis and split position
4. If cost exceeds threshold, create LEAF instead

### Plucker Coordinates for Raycasting

**Ray Array Format (double ray[10]):**
```cpp
// ray[0-2]: p × v (moment vector)
ray[0] = p[1]*v[2] - p[2]*v[1];
ray[1] = p[2]*v[0] - p[0]*v[2];
ray[2] = p[0]*v[1] - p[1]*v[0];

// ray[3-5]: Direction vector v
// ray[6-8]: Origin point p
// ray[9]: Maximum t-distance
```

### 8 Octant Variants

| sign_case | v[0] | v[1] | v[2] | Function |
|-----------|-------|-------|------|---------|
| 0 | <0 | <0 | <0 | HitWorld0 |
| 1 | >=0 | <0 | <0 | HitWorld1 |
| 2 | <0 | >=0 | <0 | HitWorld2 |
| 3 | >=0 | >=0 | <0 | HitWorld3 |
| 4 | <0 | <0 | >=0 | HitWorld4 |
| 5 | >=0 | <0 | >=0 | HitWorld5 |
| 6 | <0 | >=0 | >=0 | HitWorld6 |
| 7 | >=0 | >=0 | >=0 | HitWorld7 |

### .a3d Serialization Format

**File Header:**
```cpp
char magic[4] = {'A', '3', 'D', 0x1A};
uint32_t version;
uint32_t flags;
uint32_t world_id;
```

**Instance Sentinels:**
- `mesh_id_len > 0`: MESH instance
- `mesh_id_len = -1`: SPRITE instance  
- `mesh_id_len = -2`: ITEM instance

---

## 4. Terrain Quadtree

### Core Structures

```cpp
struct Terrain {
    QuadItem* root;      // Root node/patch
    int x, y;           // World offset of root
    int level;          // Tree depth (patches = 2^level)
    int patches;         // Total patch count
    int nodes;          // Internal node count
};

struct QuadItem {
    uint16_t lo;        // Min height in subtree
    uint16_t hi;        // Max height in subtree
    uint8_t flags;      // Neighbor presence bits (8 bits)
    QuadItem* child[4]; // 4 quadrants
};

struct Patch {
    QuadItem base;              // Embedded QuadItem
    Node* parent;              // Parent node pointer
    uint16_t height[5][5];     // 5×5 vertex heights
    uint16_t visual[8][8];     // 8×8 cell materials
    uint16_t diag;             // Diagonal orientation bitfield
    uint64_t dark;             // Shadow state
};
```

### Constants
```cpp
const int HEIGHT_CELLS = 4;     // Vertices: 5×5
const int VISUAL_CELLS = 8;      // Cells: 8×8
```

### Neighbor Flags (8-bit CCW)

```
Bit:  0   1   2   3   4   5   6   7
     N  NE   E  SE   S  SW   W  NW
```

### Height Interpolation

**For cell with diag bit set (NW-SE diagonal):**
- Triangle 1: (hx,hy) → (hx+1,hy) → (hx,hy+1)
- Triangle 2: (hx+1,hy) → (hx+1,hy+1) → (hx,hy+1)

**For cell with diag bit clear (NE-SW diagonal):**
- Triangle 1: (hx,hy) → (hx+1,hy+1) → (hx,hy+1)
- Triangle 2: (hx,hy) → (hx+1,hy) → (hx+1,hy+1)

### 8 Octant Raycast Variants

| Variant | Direction | Description |
|---------|-----------|-------------|
| HitTerrain0 | (+X, +Y, +Z) | Heading diagonally up-right |
| HitTerrain1 | (-X, +Y, +Z) | Heading diagonally up-left |
| HitTerrain2 | (+X, -Y, +Z) | Heading diagonally up-back |
| HitTerrain3 | (-X, -Y, +Z) | Heading diagonally up-forward |
| HitTerrain4 | (+X, +Y, -Z) | Heading down-right |
| HitTerrain5 | (-X, +Y, -Z) | Heading down-left |
| HitTerrain6 | (+X, -Y, -Z) | Heading down-back |
| HitTerrain7 | (-X, -Y, -Z) | Heading straight down |

### Coordinate System

- **Z is UP**: Terrain patches are in the XY plane, height is the Z axis. Confirmed from physics.h:41 (z_force = vertical force). (Corrected: was incorrectly stated as Y is UP)
- **Origin at corner**: Terrain root at (Terrain::x, Terrain::y)
- **Integer coordinates**: Patch positions are integer grid coordinates

---

## Rust Port Summary

### Key Type Mappings

| C++ Type | Rust Equivalent |
|----------|-----------------|
| `Sample` | `#[repr(C)] Sample` |
| `AnsiCell` | `#[repr(C)] AnsiCell` |
| `SampleBuffer` | `struct SampleBuffer { ptr: Box<[Sample]>, w: i32, h: i32 }` |
| `BSP` | `enum BSPNode { Node(Box<BSP>), Leaf(...), Inst(...) }` |
| `PluckerRay` | `#[repr(C)] PluckerRay` |
| `Terrain` | `struct Terrain { root: QuadItem, ... }` |
| `Patch` | `struct Patch { height: [[u16; 9]; 9], ... }` |

### Key Crates Needed

| Purpose | Crate |
|---------|-------|
| GZIP decompression | `flate2` |
| Binary parsing | `byteorder` |
| Memory mapping | `memmap2` |
| Math | `glam`, `nalgebra` |
| Image | `image` |

### Critical Assumptions

1. **Column-major XPCell ordering** - Must verify in Rust port
2. **Y-up coordinate system** - Standard game engine convention
3. **Double precision for transforms** - Matrix tm[16] uses double
4. **Single-threaded rendering** - No locks or atomics
5. **IEEE 754 floats** - FLT_EPSILON comparisons
