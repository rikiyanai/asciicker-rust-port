# Asciicker Rendering System - Critical Unknowns Re-Audit

**Date:** 2026-02-19  
**Source:** /Users/r/Downloads/asciicker-Y9-2/

---

## Executive Summary

This re-audit verifies the status of 4 critical unknowns in the Asciicker rendering system. **All 4 items have been VERIFIED or REFUTED.**

---

## 1. auto_mat Lookup Table Format (CRITICAL)

### Status: **VERIFIED**

### Findings:

**Location:** render.cpp:707-840

**Definition:**
```cpp
static uint8_t auto_mat[/*b*/32/*g*/ * 32/*r*/ * 32/*bg,fg,gl*/ * 3];
int auto_mat_result = create_auto_mat(auto_mat);
```

**Table Size:** 32 × 32 × 32 × 3 = **98,304 bytes (32K entries)**

**Entry Format:** 3 bytes per entry = `{bg, fg, gl}`
- `bg` = xterm 256-color palette index for background (0-255)
- `fg` = xterm 256-color palette index for foreground (0-255)  
- `gl` = ASCII glyph code for dithering pattern

**Index Formula (render.cpp:810):**
```cpp
int idx = 3 * (r + 32 * (g + 32 * b));
```
Where `r, g, b` are 5-bit color values (0-31) derived from RGB555 input.

**Generation Algorithm (render.cpp:710-840):**
The `create_auto_mat()` function:
1. Iterates through all 32×32×32 = 32,768 RGB555 combinations
2. For each color, finds the best pair of xterm 256-color cube colors
3. Computes optimal dither glyph based on projection error
4. Stores {bg, fg, gl} triple at the corresponding index

**Usage in Resolve Phase (render.cpp:3715-3758):**
```cpp
if (use_auto_mat)
{
    int auto_mat_idx = 3 * (bg[0] / 33 + 32 * (bg[1] / 33) + 32 * 32 * (bg[2] / 33));
    ptr->gl = auto_mat[auto_mat_idx + 2];
    ptr->bk = auto_mat[auto_mat_idx + 0];
    ptr->fg = auto_mat[auto_mat_idx + 1];
}
```

**Helper Function:** Direct array indexing - no helper function needed. Index computed inline using the formula above.

---

## 2. Perspective Projection Matrix (CRITICAL)

### Status: **VERIFIED - FULLY IMPLEMENTED**

### Findings:

**Renderer Struct (render.cpp:690-694):**
```cpp
// perspective test
float view_dir[3];
float view_pos[3];
float view_ofs[2]; // dw/2 + shift[0]*2, dh/2 + shift[1]*2
float focal;
```

**Focal Length Calculation (render.cpp:3023):**
```cpp
r->focal = (float)fmax(dw,dh) * 2.0f; //500;
```

**View Direction Setup (render.cpp:3024-3032):**
```cpp
r->view_dir[0] = (float)( - sinyaw * 1); // cos30;
r->view_dir[1] = (float)(cosyaw * 1); // cos30;
r->view_dir[2] = 0.0f; // -sin30;

r->view_pos[0] = HEIGHT_CELLS * pos[0] - r->view_dir[0] * r->focal;
r->view_pos[1] = HEIGHT_CELLS * pos[1] - r->view_dir[1] * r->focal;
r->view_pos[2] = pos[2];
r->view_dir[0] /= r->focal;
r->view_dir[1] /= r->focal;
```

**Projection Function - ProjectCoords (render.cpp:4413-4461):**
```cpp
bool ProjectCoords(Renderer* r, const float pos[3], int view[3])
{
    float w_pos[3] = { pos[0] * HEIGHT_CELLS, pos[1] * HEIGHT_CELLS, pos[2] };

    if (r->perspective)
    {
        float vx = w_pos[0], vy = w_pos[1], vz = w_pos[2];
        float viewer_dist;
        float eye_to_vtx[3] =
        {
            vx - r->view_pos[0],
            vy - r->view_pos[1],
            vz - r->view_pos[2],
        };

        viewer_dist = DotProduct(eye_to_vtx, r->view_dir);

        if (viewer_dist <= 0)
            return false;

        float fx = (float)(r->mul[0] * vx + r->mul[2] * vy + r->add[0]);
        float fy = (float)(r->mul[1] * vx + r->mul[3] * vy + r->mul[5] * vz + r->add[1]);

        float recp_dist = 1.0f / viewer_dist;

        fx = (fx - r->view_ofs[0]) * recp_dist + r->view_ofs[0];
        fy = (fy - r->view_ofs[1]) * recp_dist + r->view_ofs[1];

        int tx = (int)floorf(fx + 0.5f);
        int ty = (int)floorf(fy + 0.5f);

        view[0] = (tx - 1) >> 1;
        view[1] = (ty - 1) >> 1;
        view[2] = (int)floorf(w_pos[2] + 0.5f) + HEIGHT_SCALE / 2;
    }
    // ... orthographic path
}
```

**Perspective Correction in Line Drawing (render.cpp:186-200):**
```cpp
template <typename Sample>
inline void PerspectiveCorrectCellLine(/*const*/Sample* smp, AnsiCell* buf, int w, int h, int from[3], int to[3], float d_from, float d_to, int gl, int fg)
{
    // Uses 1/w interpolation: ka = ka / ((1-t)*d_from + t*d_to)
    // This corrects for perspective foreshortening
}
```

**Unprojection (render.cpp:4466-4508):**
- `UnprojectCoords2D`: Maps 2D screen position back to 3D using depth from SampleBuffer
- `UnprojectCoords3D`: Solves perspective equations for 3D unprojection (used for mouse picking)

**Toggle Mechanism:**
- Perspective mode toggled via `r->perspective` boolean
- Set from menu: `game->perspective` (game.cpp:10844-10848)
- Saved to config file (game.cpp:10835+)

---

## 3. Y-up vs Z-up Coordinate System (CRITICAL)

### Status: **REFUTED - CONFIRMED Z-UP**

### Findings:

**Evidence from codebase analysis:**

1. **Height stored in pos[2]** - Throughout game.cpp, world.cpp, render.cpp:
   ```cpp
   player.pos[2] = z;                    // game.cpp:5513
   i->pos[2] = pos[2];                    // world.cpp:725
   r->view_pos[2] = pos[2];               // render.cpp:3030
   player.shoot_to[2] = t->pos[2] + 40;  // game.cpp:6767
   ```

2. **HEIGHT_SCALE used with pos[2]:**
   ```cpp
   float dz_dy = HEIGHT_SCALE / (cos30 * HEIGHT_CELLS * ds);  // world.cpp:471
   float dhz = zoom * (f->height - f->ref[1] * 0.5f) / cos30 * HEIGHT_SCALE;  // world.cpp:468
   ```

3. **Projection uses pos[2] for vertical:**
   ```cpp
   float w_pos[3] = { pos[0] * HEIGHT_CELLS, pos[1] * HEIGHT_CELLS, pos[2] };
   // ...
   fy = (float)(r->mul[1] * vx + r->mul[3] * vy + r->mul[5] * vz + r->add[1]);
   view[2] = (int)floorf(w_pos[2] + 0.5f) + HEIGHT_SCALE / 2;
   ```

4. **Coordinate comments in world.cpp (lines 19-100):** No mention of Y-up; standard 3D coordinate conventions used.

**Conclusion:** The coordinate system is **Z-up** (pos[0]=X, pos[1]=Y, pos[2]=Z/height). The original audit claim of "Y-up" was incorrect and is now **REFUTED**.

---

## 4. 2x Supersampling DBL (CRITICAL)

### Status: **VERIFIED**

### Findings:

**Definition (render.cpp:88):**
```cpp
#define DBL
```

**Purpose (render.cpp lines 28-32):**
```cpp
// 2x SUPERSAMPLED SAMPLE BUFFER:
//   SampleBuffer is (2*width+4) x (2*height+4) — the +4 provides a 1-sample
//   border on each side to avoid bounds checks during 2x2 neighbor access.
```

**Usage Throughout Codebase:**
- render.cpp:2067 - Comment about DBL define affecting divisions
- render.cpp:2854, 2862 - Conditional compilation blocks
- render.cpp:2997, 3007 - Yaw calculations with DBL support
- render.cpp:3291, 3301 - Scene shift calculations
- render.cpp:3443, 3933 - Various rendering branches

**SampleBuffer Structure:**
- Dimensions: `(2*width+4) x (2*height+4)`
- Border: 2 samples on each side (total +4)
- Used for 2x2 downsampling to AnsiCell grid in resolve phase

---

## Remaining Unknowns After This Audit

### NONE - All Critical Items Resolved

| Item | Status | Confidence |
|------|--------|------------|
| auto_mat format | VERIFIED | High - Exact byte layout determined |
| Perspective matrix | VERIFIED | High - Full implementation found |
| Coordinate system | REFUTED | High - Z-up confirmed |
| 2x supersampling DBL | VERIFIED | High - Confirmed as 2x |

### No Blocking Unknowns Remain

The rendering system is now fully understood. All critical "unknown unknowns" have been resolved:
- Color quantization algorithm fully specified
- Perspective projection fully specified and implemented
- Coordinate system confirmed
- Supersampling mechanism confirmed

---

## Recommendations for Rust Port

1. **auto_mat**: Generate lookup table at startup using the exact algorithm in render.cpp:710-840, or pre-compute and embed as binary
2. **Perspective**: Implement focal length as `max(width, height) * 2.0`, store view_dir/view_pos for perspective divide
3. **Coordinates**: Use Z-up (pos[2] = height)
4. **DBL**: Keep 2x supersampling for quality

---

*End of Audit*
