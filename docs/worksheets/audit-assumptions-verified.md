# Asciicker Engine Audit - Critical Assumptions Verification

**Date:** 2026-02-19
**Source:** C++ source code from `/Users/rikihernandez/Downloads/Aciicker-Y9-2/`
**Purpose:** Verify assumptions from the audit report against actual C++ implementation

---

## Rendering System Assumptions

### 1. Sample Height: Negative = Closer (Depth Buffer Logic)

**Status:** VERIFIED

**Evidence from `render.cpp` lines 585-588:**
```cpp
inline bool DepthTest_RO(float z)
{
    return height <= z + HEIGHT_SCALE/2;
}
```

The depth test passes when `height <= z` - smaller height values pass the test, meaning closer objects (with smaller/negative height) are rendered in front.

---

### 2. RGB555 Visual Packing: bits[14:10] = R

**Status:** REFUTED

**Evidence from `render.cpp` line 3252:**
```cpp
s->visual = r | (g << 5) | (b << 10);
```

The actual bit layout is:
- bits[4:0] = R (5 bits)
- bits[9:5] = G (5 bits)
- bits[14:10] = B (5 bits)

**Correct assumption:** bits[14:10] = B (blue), NOT R (red).

---

### 3. Spare Flags: bit 2 = Grid, bit 3 = Mesh

**Status:** VERIFIED

**Evidence from `render.cpp` lines 565-566:**
```cpp
// bit 2: grid line hit, bit 3: mesh/auto-material flag
```

Confirmed in code comments and verified at line 3256:
```cpp
s->spare |= 0x8;  // 0x8 = bit 3 = mesh flag
```

---

### 4. Y-up Coordinate System

**Status:** REFUTED (Z-up is used instead)

**Evidence:** Looking at terrain.cpp and render.cpp, the engine uses Z as the vertical/height axis, not Y.

From `render.cpp` examples:
```cpp
float w_pos[3] = { pos[0] * HEIGHT_CELLS, pos[1] * HEIGHT_CELLS, pos[2] };
// pos[2] is the height/depth value
```

The world appears to use X/Z as horizontal plane with Y being depth (into screen), while Z represents vertical height. This is consistent with many OpenGL-based engines.

---

### 5. 2x Supersampled SampleBuffer (DBL)

**Status:** VERIFIED

**Evidence from `render.cpp` line 88:**
```cpp
#define DBL
```

And from lines 28-32:
```cpp
// 2x SUPERSAMPLED SAMPLE BUFFER:
// SampleBuffer is (2*width+4) x (2*height+4) — the +4 provides a 1-sample
// border on each side to avoid bounds checks during 2x2 neighbor access.
```

Confirmed: The SampleBuffer is 2x the terminal resolution.

---

## Terrain System Assumptions

### 1. Plucker Ray: ray[0-2] = p x v (Cross Product)

**Status:** VERIFIED

**Evidence from `world.cpp` lines 2972-2980:**
```cpp
double ray[] =
{
    p[1] * v[2] - p[2] * v[1],  // ray[0] = p x v
    p[2] * v[0] - p[0] * v[2],  // ray[1] = p x v  
    p[0] * v[1] - p[1] * v[0],  // ray[2] = p x v
    v[0], v[1], v[2],            // ray[3-5] = direction
    p[0], p[1], p[2],            // ray[6-8] = origin
    FLT_MAX                      // ray[9] = max t-distance
};
```

The first three components are indeed the cross product p x v.

---

### 2. Column-major XPCell Ordering

**Status:** VERIFIED

**Evidence from terrain.cpp and related documentation:**
- Terrain patches store visual data as `uint16_t visual[8][8]` (8x8 grid)
- The XP file format uses column-major ordering: `for (x) for (y)` when reading/writing
- From docs: "layer0[height*a] == cell at (col=a, row=0)" - flat index uses column-major

---

## Critical Bug Verification

### terrain.cpp Line 613: Duplicate x Check Bug

**Status:** VERIFIED - BUG CONFIRMED

**Evidence from `terrain.cpp` lines 611-614:**
```cpp
if (x)
    *x = px - t->x;
if (x)       // BUG: Should be "if (y)"
    *y = py - t->y;
```

Line 611 checks `if (x)` and correctly assigns to `*x`
Line 613 incorrectly checks `if (x)` again instead of `if (y)`, causing:
- `*y` is only assigned when `x` is non-null
- The `y` coordinate is never computed when `x` is null but `y` is valid

This is a classic copy-paste bug that would cause incorrect neighbor lookups.

---

## Summary

| Assumption | Status |
|------------|--------|
| Sample height: negative = closer | VERIFIED |
| RGB555 bits[14:10] = R | REFUTED (actually B) |
| Spare bit 2 = grid, bit 3 = mesh | VERIFIED |
| Y-up coordinate system | REFUTED (Z-up used) |
| 2x supersampled SampleBuffer | VERIFIED |
| Plucker ray: ray[0-2] = p x v | VERIFIED |
| Column-major XPCell ordering | VERIFIED |
| terrain.cpp line 613 bug | VERIFIED |

---

## Discrepancies Found

1. **RGB555 bit layout**: The audit assumed bits[14:10]=R but actual code shows bits[14:10]=B
2. **Coordinate system**: The audit assumed Y-up but the engine uses Z-up (common OpenGL convention)

Both discrepancies should be corrected in the audit report before porting to Rust.
