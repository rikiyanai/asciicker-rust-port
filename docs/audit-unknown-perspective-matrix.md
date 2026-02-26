# Audit: Perspective Projection Implementation in render.cpp

## Summary

This document details the perspective projection implementation found in `/Users/r/Downloads/asciicker-Y9-2/render.cpp`. The original codebase uses a custom perspective projection that does not use a standard 4x4 perspective matrix.

## 1. Focal Length Default Value

**Location:** Line 3023

```cpp
r->focal = (float)fmax(dw,dh) * 2.0f; //500;
```

- **Default value:** `2.0 * max(dw, dh)` where `dw` and `dh` are the screen dimensions (in sample buffer pixels)
- **Commented fallback:** The original code had a hardcoded value of `500` (now commented out)
- The focal length scales with screen resolution, ensuring consistent perspective across different display sizes

## 2. Hardcoded Perspective Values

### View Direction (Lines 3024-3026)

```cpp
r->view_dir[0] = (float)( - sinyaw * 1); // cos30;
r->view_dir[1] = (float)(cosyaw * 1); // cos30;
r->view_dir[2] = 0.0f; // -sin30;
```

- Uses yaw angle (horizontal rotation only - no pitch)
- Note: The 30-degree isometric angles (`cos30`, `sin30`) are commented out in favor of "architectural" perspective
- The `1` multiplier was previously `cos30` (approximately 0.866)

### View Position (Lines 3028-3030)

```cpp
r->view_pos[0] = HEIGHT_CELLS * pos[0] - r->view_dir[0] * r->focal;
r->view_pos[1] = HEIGHT_CELLS * pos[1] - r->view_dir[1] * r->focal;
r->view_pos[2] = pos[2];
```

The view position is offset from the player position along the negative view direction, scaled by the focal length.

### View Direction Normalization (Lines 3031-3032)

```cpp
r->view_dir[0] /= r->focal;
r->view_dir[1] /= r->focal;
```

After initial setup, view_dir is normalized by dividing by focal length.

### View Offset (Lines 3033-3034)

```cpp
r->view_ofs[0] = (float)(dw/2 + scene_shift[0]*2);
r->view_ofs[1] = (float)(dh/2 + scene_shift[1]*2);
```

- `view_ofs` represents the screen center plus any scene shift offset
- Used as the vanishing point for perspective projection

### Clip Planes (Lines 3045-3048)

```cpp
double clip_left[4]   = {  1,  0,  0, 1 };
double clip_right[4]  = { -1,  0,  0, 1 };
double clip_bottom[4] = {  0,  1,  0, 1 };
double clip_top[4]    = {  0, -1,  0, 1 }; // +1 for perspective
```

These define the view frustum in normalized device coordinates. The `w=1` values create a standard frustum.

## 3. FOV Calculation

**There is no explicit FOV calculation in the code.**

Instead, the perspective is defined implicitly through:
- The focal length (derived from screen dimensions)
- The view offset (screen center)

The "FOV" effect is achieved through the perspective divide operation.

## 4. How to Derive Perspective from Existing Code

### The Core Perspective Formula

The perspective projection is applied during vertex transformation (see `RenderFace` around lines 968-1108):

```cpp
// Calculate vector from eye to vertex
float eye_to_vtx[3] =
{
    ws[0] * HEIGHT_CELLS - r->view_pos[0],
    ws[1] * HEIGHT_CELLS - r->view_pos[1],
    ws[2] - r->view_pos[2],
};

// Calculate viewer distance (dot product with view direction)
viewer_dist = DotProduct(eye_to_vtx, r->view_dir);
if (viewer_dist > 0)
{
    viewer_dist = 1.0f/viewer_dist;  // Reciprocal for perspective divide
    
    // Apply perspective correction to screen coordinates
    fx = (fx - r->view_ofs[0]) * viewer_dist + r->view_ofs[0];
    fy = (fy - r->view_ofs[1]) * viewer_dist + r->view_ofs[1];
}
```

### Perspective Derivation Steps

1. **View Direction (normalized):**
   ```
   view_dir = normalize([-sin(yaw), cos(yaw), 0])
   ```

2. **View Position:**
   ```
   view_pos = player_pos * HEIGHT_CELLS - view_dir * focal
   ```

3. **Perspective Divide:**
   For any 3D point P transformed to screen coordinates (sx, sy):
   ```
   viewer_dist = dot(P - view_pos, view_dir)
   if viewer_dist > 0:
       screen_x = (sx - view_ofs[0]) / viewer_dist + view_ofs[0]
       screen_y = (sy - view_ofs[1]) / viewer_dist + view_ofs[1]
   ```

### Relationship to Standard Perspective Matrix

This implementation is equivalent to a perspective projection where:

- **Focal length** = camera distance from projection plane
- **view_ofs** = principal point (vanishing point on screen)
- The perspective divide uses `1/viewer_dist` instead of `z/focal`

To convert to a standard perspective matrix:
- The effective "focal length" in pixel terms is `focal * scale_factor`
- The near/far planes are handled implicitly through the clip planes

### Unprojection (Inverse Perspective)

The inverse operation is implemented in `UnprojectCoords3D` (lines 4519-4566), which solves a system of linear equations to recover world coordinates from screen coordinates and depth.

Key variables used:
- `tm0-tm13`: Elements from the view transform matrix (`r->mul[]` and `r->add[]`)
- `view_dir`, `view_pos`, `view_ofs`: The perspective parameters
- `xyz[0], xyz[1], xyz[2]`: Screen x, y and depth

## Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `HEIGHT_CELLS` | 4 | terrain.h:60 | Vertices per patch axis minus 1 (5×5 grid) |
| `HEIGHT_SCALE` | Varies | Vertical scaling factor |
| `VISUAL_CELLS` | 8 | Texture/visual cell count |
| `focal` | `2.0 * max(dw, dh)` | Line 3023 |

## Rust Port Recommendations

1. **Focal Length:** Use `2.0 * max(screen_width, screen_height)` as default
2. **View Direction:** Compute from yaw: `[-sin(yaw), cos(yaw), 0]` normalized
3. **View Offset:** Screen center: `[screen_width/2, screen_height/2]`
4. **Perspective Divide:** Apply `1/distance` scaling around the view offset point

The key insight is that this is NOT a standard pinhole camera model - it's an "architectural" perspective that keeps vertical lines parallel on screen, which is why there's no pitch component in the view direction and no traditional FOV parameter.
