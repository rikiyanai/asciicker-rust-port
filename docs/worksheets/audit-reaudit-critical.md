# Asciicker render.cpp Audit - Critical Findings

## Source: /Users/rikihernandez/Downloads/Aciicker-Y9-2/render.cpp

---

## 1. auto_mat - Definition and Format

### Declaration (Line 708):
```cpp
static uint8_t auto_mat[/*b*/32/*g*/ * 32/*r*/ * 32/*bg,fg,gl*/ * 3];
```

### Static Initializer (Line 709):
```cpp
int auto_mat_result = create_auto_mat(auto_mat);
```

### Function Definition (Lines 710-840):
```cpp
static int create_auto_mat(uint8_t mat[])
{
    // Precomputes 32K-entry lookup table mapping RGB555 colors to xterm 256-color palette
    
    // Loop structure: b (0-31), g (0-31), r (0-31)
    for (int b=0; b<32; b++)
    {
        for (int g = 0; g < 32; g++)
        {
            for (int r = 0; r < 32; r++,i++)
            {
                // ... computation ...
                int idx = 3 * (r + 32 * (g + 32 * b));
                mat[idx + 0] = bg_color;   // xterm 256 palette index
                mat[idx + 1] = fg_color;   // xterm 256 palette index  
                mat[idx + 2] = glyph;       // dither glyph from " ..::%"
            }
        }
    }
}
```

**Format Summary:**
- Total size: 32 * 32 * 32 * 3 = 98,304 bytes (32K entries)
- Each entry: 3 bytes = {bg_color, fg_color, glyph}
- Index formula: `3 * (r + 32 * (g + 32 * b))` where r,g,b are 5-bit color components (0-31)
- Glyph set: `" ..::%"` (6 characters for dithering)

---

## 2. perspective / fov / focal - Projection Code

### Renderer Struct Members:

**Line 687:**
```cpp
bool perspective;
```

**Line 694:**
```cpp
float focal;
```

### Perspective Projection Logic (Example from Lines 968-1008):

```cpp
if (r->perspective) // #if PERSPECTIVE_TEST 
{
    float ws[4];
    Product(r->inst_tm, xyzw, ws);
    
    float viewer_dist; // {vx,vy,vz}  r->pos
    float eye_to_vtx[3] =
    {
        ws[0] * HEIGHT_CELLS - r->view_pos[0],
        ws[1] * HEIGHT_CELLS - r->view_pos[1],
        ws[2] - r->view_pos[2],
    };

    viewer_dist = DotProduct(eye_to_vtx, r->view_dir);
    if (viewer_dist > 0)
    {
        viewer_dist = 1.0f/viewer_dist;

        float fx = tmp0[0];
        float fy = tmp0[1];

        // Perspective divide with view offset
        fx = (fx - r->view_ofs[0]) * viewer_dist + r->view_ofs[0];
        fy = (fy - r->view_ofs[1]) * viewer_dist + r->view_ofs[1];

        int tx = (int)floorf(fx + 0.5f);
        int ty = (int)floorf(fy + 0.5f);

        v[0][0] = tx;
        v[0][1] = ty;
        v[0][2] = (int)floor(tmp0[2] + 0.5f);
    }
    else
        return; // Behind camera - culled
}
```

**Projection Parameters:**
- `r->perspective`: Boolean toggle for projection mode
- `r->focal`: Focal length for perspective (affects view_ofs)
- `r->view_pos[3]`: Camera position
- `r->view_dir[3]`: View direction vector
- `r->view_ofs[2]`: View offset (dw/2 + shift[0]*2, dh/2 + shift[1]*2)

---

## 3. DBL - Supersampling Factor

### Definition (Line 88):
```cpp
#define DBL
```

### Effect:
- Enables 2x supersampling (2x2 samples per character cell)
- SampleBuffer dimensions: `(2*width+4) x (2*height+4)`
- The +4 provides 1-sample border on each side

### Usage Throughout Code:
```cpp
// Line 2854: #ifdef DBL
// Line 2862: #ifdef DBL
// Lines 2997, 3007, 3291, 3301: DBL-aware coordinate calculations
// Lines 3443, 3933: #ifdef DBL blocks
```

**Supersampling Factor: 2x** (each character cell = 2x2 samples)

---

## 4. Depth Test Logic (<=, zbuf)

### Sample Struct with Depth (Lines 567-589):

```cpp
struct Sample
{
    uint16_t visual;
    uint8_t diffuse;
    uint8_t spare;   // refl, patch xy parity etc..., direct color bit
    float height;     // DEPTH BUFFER VALUE (z-coordinate)

    /*
    inline bool DepthTest_RW(float z)
    {
        if (height > z)
            return false;
        spare &= ~0x4; // clear lines
        height = z;
        return true;
    }
    */

    inline bool DepthTest_RO(float z)
    {
        return height <= z + HEIGHT_SCALE/2;
    }
};
```

### Depth Test Comparison:

**Line 585-588:**
```cpp
inline bool DepthTest_RO(float z)
{
    return height <= z + HEIGHT_SCALE/2;
}
```

**Depth Test Logic:**
- Compares: `sample.height <= incoming_z + HEIGHT_SCALE/2`
- HEIGHT_SCALE is a constant (typically 256 or similar - check terrain.h)
- The `+ HEIGHT_SCALE/2` provides a small bias to prevent z-fighting
- Read-only depth test (DepthTest_RO) - used for checking, not writing
- Commented-out DepthTest_RW would write new depth on pass

### Usage in Rasterization:

**Line 148, 151 (Bresenham line algorithm):**
```cpp
if (ptr->DepthTest_RO(z))
    ptr->spare |= _or;
```

**Line 235, 275 (PerspectiveCorrectCellLine):**
```cpp
if (test->DepthTest_RO(z))
{
    // write to AnsiCell
}
```

**Line 331, 368 (CellLine):**
```cpp
if (test->DepthTest_RO(z))
{
    // write to AnsiCell
}
```

**Line 853 (RenderFace Shader Blend):**
```cpp
if (s->height < z)
{
    // write new depth and color
}
```

**Line 1549 (RenderPatch Shader Blend):**
```cpp
if (s->height < z)
{
    // write new depth and color
}
```

---

## Summary Table

| Item | Line | Value/Code |
|------|------|------------|
| auto_mat array | 708 | `uint8_t auto_mat[32*32*32*3]` |
| auto_mat size | - | 98,304 bytes (32K entries) |
| auto_mat entry | - | 3 bytes: {bg, fg, glyph} |
| perspective flag | 687 | `bool perspective` |
| focal length | 694 | `float focal` |
| DBL macro | 88 | `#define DBL` |
| Supersample factor | - | 2x (2x2 samples/cell) |
| Depth field | 572 | `float height` |
| Depth test | 585-588 | `height <= z + HEIGHT_SCALE/2` |
| HEIGHT_SCALE | terrain.h:54 | 16 | Z-units per visual cell (CRITICAL: not ~256) |

---

## Rust Port Implications

1. **auto_mat**: Precompute at startup, store as `[[u8; 3]; 32768]` or generate dynamically
2. **perspective**: Requires view_pos, view_dir, focal, view_ofs vectors; perspective divide using 1/viewer_dist
3. **DBL**: Always enabled; SampleBuffer = (2*w+4) x (2*h+4)
4. **depth test**: `sample.height <= z + HEIGHT_SCALE/2` - straightforward translation

