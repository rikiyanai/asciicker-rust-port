# Asciicker Rendering System - HIGH Priority Re-Audit

**Audit Date:** 2026-02-19  
**Source:** /Users/r/Downloads/asciicker-Y9-2/render.cpp

---

## 1. Visual Averaging Method (2x2 Sample Blocks)

**Status:** VERIFIED

**Finding:** Integer division averaging

### Code Snippet (render.cpp:3493)

```cpp
int shd = (dif[0] + dif[1] + dif[2] + dif[3] + 17 * 2) / (17 * 4); // 17: FF->F, 4: avr
```

**Analysis:**
- The resolve phase averages diffuse lighting across 2x2 sample blocks
- Uses integer division (`/`), not floating point
- The `17 * 2` term adds bias before division to round correctly
- Division by `(17 * 4)` = 68 normalizes the sum
- This is a fixed-point approximation of floating-point averaging

### Additional RGB Averaging (render.cpp:3528-3530)

```cpp
int r = ((vis[i] & 0x1F) * 527 + 23) >> 6;
int g = (((vis[i] >> 5) & 0x1F) * 527 + 23) >> 6;
int b = (((vis[i] >> 10) & 0x1F) * 527 + 23) >> 6;
```

- RGB555 to 888 conversion using integer math: `(x * 527 + 23) >> 6` approximates `x * 255 / 31`
- This is also integer arithmetic (shift-based division)

---

## 2. Depth Test Semantics

**Status:** VERIFIED

**Finding:** `<=` (less-than-or-equal) comparison

### Code Snippet (render.cpp:585-588)

```cpp
inline bool DepthTest_RO(float z)
{
    return height <= z + HEIGHT_SCALE/2;
}
```

**Analysis:**
- Comparison operator is `<=` (less than or equal)
- The `+ HEIGHT_SCALE/2` adds a half-unit bias to resolve ties
- This means objects at the SAME depth will both pass the test (closer wins)
- Standard depth test: "near things are bigger than far things" - closer samples overwrite

---

## 3. Shadow Projection - Inverse Transform

**Status:** VERIFIED

**Finding:** Screen-space to world-space via inverse matrix transform

### Code Snippet (render.cpp:3184-3217)

```cpp
// [FLOW:RENDER] Stage 4: Shadow — player blob shadow on terrain
// WHY: The player shadow is projected onto the SampleBuffer by inverse-
// transforming each nearby sample back to world space, computing distance
// to the player position, and attenuating diffuse within a radius of ~2
// world units.

// Invert view/projection matrix
Invert(tm, r->inv_tm);
double* inv_tm = r->inv_tm;

// For each sample in shadow radius:
for (int y = 0; y < dh; y++)
{
    for (int x = left; x <= right; x++)
    {
        Sample* s = r->sample_buffer.ptr + x + y * dw;
        if (abs(s->height - pos[2]) <= 64)
        {
            // Transform screen space (x, y, height) to world space
            double screen_space[] = { (double)x, (double)y, s->height, 1.0 };
            double world_space[4];
            
            Product(inv_tm, screen_space, world_space);
            
            // Compute distance from player position
            double dx = world_space[0]/HEIGHT_CELLS - pos[0];
            double dy = world_space[1]/HEIGHT_CELLS - pos[1];
            double sq_xy = dx*dx + dy*dy;
            
            // Attenuate shadow based on distance
            if (sq_xy <= 2.00)
            {
                int dz = (int)(2*(pos[2] - s->height) + 2*sq_xy);
                // ... shadow intensity calculation
            }
        }
    }
}
```

**Algorithm Summary:**
1. Compute inverse view/projection matrix: `Invert(tm, r->inv_tm)`
2. For each sample near player (within bounds check):
   - Transform screen coordinates to world space using `Product(inv_tm, screen_space, world_space)`
   - Calculate Euclidean distance from player position (dx, dy)
   - Apply radial attenuation: closer samples get darker shadows
3. Modify sample's diffuse lighting to darken

---

## 4. RGB555 Color Packing

**Status:** VERIFIED (REFUTED original claim)

**Finding:** bits[14:10] = **BLUE**, not Red

### Code Snippet (render.cpp:3528-3530 + 3252)

**Extraction (reading from RGB555):**
```cpp
int r = ((vis[i] & 0x1F) * 527 + 23) >> 6;       // bits[4:0]  -> R
int g = (((vis[i] >> 5) & 0x1F) * 527 + 23) >> 6; // bits[9:5]  -> G
int b = (((vis[i] >> 10) & 0x1F) * 527 + 23) >> 6; // bits[14:10] -> B
```

**Packing (writing to RGB555):**
```cpp
// From shadow projection (render.cpp:3252)
s->visual = r | (g << 5) | (b << 10);
```

### Bit Layout

| Bits     | Field     |
|----------|-----------|
| [4:0]    | R (Red)   |
| [9:5]    | G (Green) |
| [14:10]  | B (Blue)  |
| [15]     | Edge flag |

**Confirmation:** The original audit claim that "bits[14:10]=B not R" is **CORRECT**. Blue is in bits[14:10].

---

## Summary Table

| Item | Status | Key Finding |
|------|--------|-------------|
| Visual Averaging | VERIFIED | Integer division `(sum + bias) / divisor` |
| Depth Test | VERIFIED | `<=` comparison with half-unit bias |
| Shadow Inverse Transform | VERIFIED | Inverse matrix multiply + distance calculation |
| RGB555 Packing | VERIFIED | bits[14:10] = B (Blue) |

---

## Remaining Gaps

None identified. All HIGH priority items have been verified with exact code citations.
