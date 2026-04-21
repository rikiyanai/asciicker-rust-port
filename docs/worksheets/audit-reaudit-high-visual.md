# Visual Audit - High-Level Render Pipeline (Re-audit)

## Source: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/render.cpp`

---

## 1. Visual Averaging Method (2x2 Sample Downsampling)

**Location:** `render.cpp` lines 3412-3543 (Resolve phase)

The resolve phase performs 2x2 downsampling from `SampleBuffer` to `AnsiCell` grid.

### Key Code - Integer-based averaging:

**Line 3493** - Diffuse/shade averaging (INTEGER):
```cpp
int shd = (dif[0] + dif[1] + dif[2] + dif[3] + 17 * 2) / (17 * 4); // 17: FF->F, 4: avr
```

**Lines 3528-3542** - RGB component averaging (INTEGER with bit-shifting):
```cpp
int r = ((vis[i] & 0x1F) * 527 + 23) >> 6;
int g = (((vis[i] >> 5) & 0x1F) * 527 + 23) >> 6;
int b = (((vis[i] >> 10) & 0x1F) * 527 + 23) >> 6;

// Then multiplied by diffuse:
// r = r * dif[i] / 255;
```

**Finding:** The visual averaging uses **INTEGER math only** - no floating point in the resolve pass. This is a fixed-point approximation with bit-shift rounding.

---

## 2. Depth Test Comparison

**Location:** `render.cpp` line 587

### Exact Comparison:

```cpp
inline bool DepthTest_RO(float z)
{
    return height <= z + HEIGHT_SCALE/2;
}
```

**Finding:** The depth test uses **`<=` (less-than-or-equal)** comparison, NOT `>`.

The comparison is: `sample.height <= incoming_z + HEIGHT_SCALE/2`

This means samples pass the depth test if they are **at or below** the incoming depth (plus a half-height tolerance). This is a "less depth = closer" convention where smaller height values are "in front".

---

## 3. Shadow Projection Implementation

**Location:** `render.cpp` lines 3184-3263 (Stage 4: Shadow)

### Shadow Stage Overview:

The shadow pass projects a player blob shadow onto the SampleBuffer:

```cpp
// [FLOW:RENDER] Stage 4: Shadow — player blob shadow on terrain
// WHY: The player shadow is projected onto the SampleBuffer by inverse-
// transforming each nearby sample back to world space, computing distance
// to the player position, and attenuating diffuse within a radius of ~2
// world units. This runs AFTER terrain+world so the shadow falls on top.
```

### Key Implementation (lines 3197-3263):

```cpp
Invert(tm, r->inv_tm);
double* inv_tm = r->inv_tm;

// Loop over samples near player shadow position
for (int y = 0; y < dh; y++)
{
    int left = sh_x-5;
    int right = sh_x+5;
    // ... bounds checking ...
    
    for (int x = left; x <= right; x++)
    {
        Sample* s = r->sample_buffer.ptr + x + y * dw;
        
        // Only process samples within vertical range
        if (abs(s->height - pos[2]) <= 64)
        {
            // Inverse transform from screen to world space
            double screen_space[] = { (double)x,(double)y,s->height,1.0 };
            double world_space[4];
            Product(inv_tm, screen_space, world_space);
            
            // Calculate distance from player
            double dx = world_space[0]/HEIGHT_CELLS - pos[0];
            double dy = world_space[1]/HEIGHT_CELLS - pos[1];
            double sq_xy = dx*dx + dy*dy;
            
            // Apply shadow if within radius
            if (sq_xy <= 2.00)
            {
                int dz = (int)(2*(pos[2] - s->height) + 2*sq_xy);
                // Clamp shadow intensity: 180-255
                if (dz < 180) dz = 180;
                if (dz > 180) dz = 255;
                
                // Apply shadow darkening to sample
                if (s->spare & 0x8)
                {
                    // RGB mode: multiply diffuse
                    s->diffuse = s->diffuse * dz / 255;
                }
                else
                {
                    // Material mode: fetch shadow color from material library
                    int mat = s->visual & 0xFF;
                    int shd = (s->visual >> 8) & 0x7F;
                    
                    int r = (matlib[mat].shade[1][shd].bg[0] * 249 + 1014) >> 11;
                    int g = (matlib[mat].shade[1][shd].bg[1] * 249 + 1014) >> 11;
                    int b = (matlib[mat].shade[1][shd].bg[2] * 249 + 1014) >> 11;
                    
                    s->visual = r | (g << 5) | (b << 10);
                    s->spare |= 0x8;   // Mark as RGB mode
                    s->spare &= ~0x44; // Clear reflection bits
                    s->diffuse = dz;
                }
            }
        }
    }
}
```

### Shadow Algorithm Summary:
1. **Inverse transform** each nearby screen sample back to world coordinates
2. **Compute distance** (dx, dy) from player position in world space
3. **If within radius** (~2.0 world units): attenuate the sample's diffuse/color
4. **Two modes:** 
   - Material mode: replace visual with shadow color from material library
   - RGB mode: multiply existing diffuse by shadow intensity

---

## Summary Table

| Aspect | Finding | Line(s) |
|--------|---------|---------|
| Averaging method | **Integer** (fixed-point with bit-shift) | 3493, 3528-3542 |
| Depth test operator | **`<=`** (less-than-or-equal) | 587 |
| Shadow radius | ~2.0 world units | 3232 |
| Shadow intensity range | 180-255 | 3235-3238 |
