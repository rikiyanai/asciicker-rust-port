# HEIGHT_SCALE Audit

## 1. Definition Location

**File:** `/Users/r/Downloads/asciicker-Y9-2/terrain.h`  
**Line:** 54

```c
// WHY HEIGHT_SCALE 16: Each visual character cell spans 16 discrete z-steps in
// the heightmap. This ratio controls the vertical resolution of terrain geometry
// relative to the horizontal grid. Changing this value breaks existing .a3d files.
// [DATA-CONTRACT:A3D]
#define HEIGHT_SCALE 16 // how may z-steps produces 1 visual cell
```

## 2. Exact Numeric Value

**Value:** `16`

This constant is a compile-time macro defining how many discrete heightmap z-steps correspond to one visual character cell in the ASCII rendering.

## 3. Usage in Depth Calculations

### 3.1 World.cpp - Sprite Depth Calculation

**Location:** `/Users/r/Downloads/asciicker-Y9-2/world.cpp`, lines 467-471, 496

```cpp
// Isometric projection depth computation
float dlz = zoom * -f->ref[1] * 0.5f / cos30 * HEIGHT_SCALE;
float dhz = zoom * (f->height - f->ref[1] * 0.5f) / cos30 * HEIGHT_SCALE;

float ds = 2.0 * (/*zoom*/ 1.0 * /*scale*/ 3.0) / VISUAL_CELLS * 0.5 /*we're not dbl_wh*/;
float dz_dy = HEIGHT_SCALE / (cos30 * HEIGHT_CELLS * ds);

// Actual depth value computation (line 496):
float h = (float)(HEIGHT_SCALE / 4 + pos[2] + (2.0*ac->spare + f->ref[2]) * 0.5 * dz_dy);
```

**Purpose:** Computes the Z-depth for each sprite cell in isometric projection, using HEIGHT_SCALE to convert between heightmap units and visual cell units.

### 3.2 asciiid.cpp - Fragment Depth Output

**Location:** `/Users/r/Downloads/asciicker-Y9-2/asciiid.cpp`, lines 2158-2159

```cpp
float ds = 2.0 * (/*zoom*/ 1.0 * /*scale*/ 3.0) / 8/*VISUAL_CELLS*/ * 0.5 /*we're not dbl_wh*/;
float dz_dy = 16/*HEIGHT_SCALE*/ / (cos(30 * 3.141592/*M_PI*/ / 180) * 4/*HEIGHT_CELLS*/ * ds);
gl_FragDepth = (16/*HEIGHT_SCALE*/ / 4 + ansi_depth_ofs.x + (2.0*cell.w*255.0 + ansi_depth_ofs.y) * 0.5 * dz_dy) / 0xFFFF;
```

**Note:** The value 16 is hardcoded here instead of using the HEIGHT_SCALE macro (note the `/*HEIGHT_SCALE*/` comments). This writes the depth to the GPU depth buffer for proper occlusion handling.

### 3.3 render.cpp - Terrain Depth Operations

Multiple usages in render.cpp for:
- Water plane clipping (tolerance: `HEIGHT_SCALE / 8`)
- Grid line elevation (`HEIGHT_SCALE / 2`)
- Z-fighting prevention fudge factor (`HEIGHT_SCALE / 2`)
- Sprite positioning relative to terrain

Key examples:
```cpp
// Water plane clipping with tolerance
if (z < water + HEIGHT_SCALE / 8)
if (z >= water - HEIGHT_SCALE / 8)

// Grid line visibility
xyzf[lin][mid][2] += HEIGHT_SCALE / 2;

// Sprite vertical offset
buf->s_pos[2] = (int)(2*r->water) - ((int)floorf(w_pos[2] + 0.5f) + HEIGHT_SCALE / 2);
```

## Summary

HEIGHT_SCALE = 16 is the fundamental conversion factor between:
- **Heightmap units** (discrete z-steps stored in terrain data)
- **Visual cell units** (character cells in the ASCII render)

It is used extensively in depth calculations for:
1. Converting sprite/content height to proper Z coordinates in isometric projection
2. Computing `dz_dy` - the change in Z per unit change in Y for depth buffer writes
3. Establishing tolerances for water plane clipping and z-fighting prevention
4. All terrain height operations in the rendering pipeline

The value is marked as part of the `.a3d` file format contract - changing it would invalidate existing save files.
