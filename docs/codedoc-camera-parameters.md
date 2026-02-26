# Camera Parameters Documentation

Extracted from C++ source in `/Users/r/Downloads/asciicker-Y9-2/`

## Overview

This document details camera-related parameters found in the Asciicker C++ codebase, specifically in `render.cpp` and `game.cpp`.

---

## 1. Camera Offset Values

### scene_shift

**Type:** `int` (in `Game` struct)  
**Location:** `game.h:350`, `game.cpp:521`, `game.cpp:6650-6661`

`scene_shift` is a horizontal pan offset used primarily for the inventory slide-in animation.

```cpp
// game.h
int scene_shift;  // Line 350

// Inventory sliding logic in game.cpp (lines 6650-6661)
int inventory_width = 39;
if (show_inventory && scene_shift < inventory_width) 
{
    scene_shift += f120;  // Slide in
    if (scene_shift > inventory_width)
        scene_shift = inventory_width;
}
if (!show_inventory && scene_shift > 0)
{
    scene_shift -= f120;  // Slide out
    if (scene_shift < 0)
        scene_shift = 0;
}
```

**Usage in rendering** (`render.cpp:3033-3034`):
```cpp
r->view_ofs[0] = (float)(dw/2 + scene_shift[0]*2);
r->view_ofs[1] = (float)(dh/2 + scene_shift[1]*2);
```

> NOTE: `scene_shift` exists in TWO forms:
> - In game.h (line 350): `int scene_shift` — a scalar
> - In render.h / render.cpp Render() parameter: `const int scene_shift[2]` — a 2-element array
> The game creates `int ss[2] = { scene_shift/2, 0 }` and passes that to Render(). The Rust port must accept `[i32; 2]` for the render call.

### cam_shift

**Type:** `int` (in `Game` struct)  
**Location:** `game.h:351`, `game.cpp:522`, `game.cpp:5772-5774`

`cam_shift` is a vertical camera pan controlled by player input.

```cpp
// game.h - Line 351
int cam_shift; // vertical camera pan

// game.cpp - Vertical panning (lines 5771-5774)
if (input.IsKeyDown(A3D_I) || input.IsKeyDown(A3D_2))
    cam_shift -= f120;
if (input.IsKeyDown(A3D_X))
    cam_shift += f120;
```

**Initialization** (`game.cpp:3697`):
```cpp
g->cam_shift = 0;
```

---

## 2. Zoom Levels

**Type:** `float` (in `Game` struct)  
**Location:** `game.h:335`, `game.cpp:4160`, `game.cpp:8397-8404`

### Default Zoom

```cpp
// game.cpp:4160
g->zoom = 1.0f;  // match legacy web build default zoom
```

### Zoom Limits

```cpp
// game.cpp:8397-8404 (keyboard zoom controls)
if (key == '+' || key == '=')
{
    zoom *= 1.1f;
    if (zoom > 5.0f) zoom = 5.0f;
}
if (key == '-' || key == '_')
{
    zoom /= 1.1f;
    if (zoom < 0.2f) zoom = 0.2f;
}
```

**Zoom Range:** `0.2f` to `5.0f`  
**Zoom Step:** Multiplier of `1.1f` per keypress

### Zoom Usage in Rendering

Zoom is passed to the `Render()` function and affects the view transform:
- Passed as parameter: `float zoom` to `Render()` (`render.h:113`)
- Used for adjusting view calculations

---

## 3. View Distance / Far Plane

The renderer uses a "focal length" based view system rather than a traditional far plane. The effective view distance is determined by the focal length.

### Focal Length

**Type:** `float` (in `Renderer` struct)  
**Location:** `render.cpp:694`, `render.cpp:3023`

```cpp
// render.cpp:3023 - Focal length calculation
r->focal = (float)fmax(dw,dh) * 2.0f; //500;
```

Where `dw` and `dh` are the render target dimensions (width and height).

**Formula:**
```
focal = max(width, height) * 2.0
```

For a typical 112x63 terminal:
- `focal = max(112, 63) * 2.0 = 224`

### View Direction and Position

**Type:** `float[3]` (in `Renderer` struct)  
**Location:** `render.cpp:691-692`, `render.cpp:3024-3032`

```cpp
// render.cpp:3024-3032
r->view_dir[0] = (float)( - sinyaw * 1); // cos30;
r->view_dir[1] = (float)(cosyaw * 1); // cos30;
r->view_dir[2] = 0.0f; // -sin30;

r->view_pos[0] = HEIGHT_CELLS * pos[0] - r->view_dir[0] * r->focal;
r->view_pos[1] = HEIGHT_CELLS * pos[1] - r->view_dir[1] * r->focal;
r->view_pos[2] = pos[2];
r->view_dir[0] /= r->focal;
r->view_dir[1] /= r->focal;
```

**View Position Formula:**
```
view_pos.x = HEIGHT_CELLS * player_x - view_dir_x * focal
view_pos.y = HEIGHT_CELLS * player_y - view_dir_y * focal  
view_pos.z = player_z
```

Where `HEIGHT_CELLS = 4` (from `terrain.h:60`)

---

## 4. Focal Length Formula

### Primary Formula

```cpp
// render.cpp:3023
focal = max(display_width, display_height) * 2.0
```

### View Direction Calculation

```cpp
// render.cpp:3024-3026
view_dir_x = -sin(yaw)
view_dir_y = cos(yaw)
view_dir_z = 0  // (isometric, no vertical tilt in base view)
```

### Focus Node (vanishing point)

```cpp
// render.cpp:3057-3062
double focus_node[3] = 
{
    pos[0] + sinyaw * r->focal / HEIGHT_CELLS,
    pos[1] - cosyaw * r->focal / HEIGHT_CELLS,
    pos[2] + sin30 * r->focal / HEIGHT_CELLS * HEIGHT_SCALE
};
```

Note: `sin30 = 0.5`, `HEIGHT_SCALE` is terrain height scaling factor.

---

## 5. Camera Smoothing / Interpolation

### Yaw (Rotation) Smoothing

**Type:** `float` for yaw velocity  
**Location:** `game.h:371`, `game.cpp:5689`, `game.cpp:5695`, `physics.cpp:1333-1348`

```cpp
// game.h:371
float yaw_vel;

// Physics integration (physics.cpp:1333-1348)
phys->yaw_vel += dt * io->torque;
if (phys->yaw_vel > 10)
    phys->yaw_vel = 10;
if (phys->yaw_vel < -10)
    phys->yaw_vel = -10;
phys->yaw += dt * 0.5f * phys->yaw_vel;
phys->yaw_vel *= vel_damp;  // Velocity damping
```

**Yaw Interpolation** (`game.cpp:5689-5695`):
```cpp
double dt = (_stamp - stamp) * 0.000001 * 20;
yaw_vel = (yaw - prev_yaw);
if (dt < 0) dt = 0;
else if (dt > 1) dt = 1;
yaw = (float)(prev_yaw + yaw_vel*dt);
```

### Inventory Scene Shift Smoothing

```cpp
// game.cpp:6650-6661 - Linear slide animation
int inventory_width = 39;
if (show_inventory && scene_shift < inventory_width) 
{
    scene_shift += f120;      // ~8 pixels per frame at 60fps
    if (scene_shift > inventory_width)
        scene_shift = inventory_width;
}
if (!show_inventory && scene_shift > 0)
{
    scene_shift -= f120;
    if (scene_shift < 0)
        scene_shift = 0;
}
```

The `f120` appears to be a frame-based increment (approximately 8 units per frame).

### Menu Scroll Smoothing

From `mainmenu.cpp:446-451`:
```cpp
// Linear interpolation for smooth scrolling (±1 per frame at 60 FPS)
if (menu_smooth_scroll < menu_scroll)
    menu_smooth_scroll++;    // gradually approach target
if (menu_smooth_scroll > menu_scroll)
    menu_smooth_scroll--;    // gradually approach target
```

---

## 6. Default Camera Position

### Default Initialization

**Location:** `game_app.cpp:2059`, `game.cpp:4160`

```cpp
// game_app.cpp:2055-2060
float water = 55;
float dir = 0;
float yaw = 45;
float pos[3] = {0,15,0};    // Default position: x=0, y=15, z=0
float lt[4] = {1,0,1,.5};
```

**Default Position:** `{0.0f, 15.0f, 0.0f}` (x, y, z)

### Default Yaw

```cpp
float yaw = 45;  // 45 degrees
```

### Default Lighting

```cpp
float lt[4] = {1,0,1,.5};  // {light_x, light_y, light_z, ambient}
```

---

## Summary Table

| Parameter | Type | Default | Min | Max | Location |
|-----------|------|---------|-----|-----|----------|
| `zoom` | float | 1.0 | 0.2 | 5.0 | game.cpp:4160 |
| `scene_shift` | int | 0 | 0 | 39 | game.cpp:6650 |
| `cam_shift` | int | 0 | - | - | game.cpp:3697 |
| `yaw` | float | 45.0 | - | - | game_app.cpp:2058 |
| `pos[3]` | float[3] | {0,15,0} | - | - | game_app.cpp:2059 |
| `focal` | float | max(w,h)*2 | - | - | render.cpp:3023 |

---

## Key Constants

- `HEIGHT_CELLS = 4` (terrain.h:60) - Patch grid size
- `f120` - Frame increment for animations (~8 units/frame)
- `inventory_width = 39` (game.cpp:6648) - Inventory panel width

---

## Render Function Signature

```cpp
void Render(Renderer* r, uint64_t stamp, Terrain* t, World* w, float water,
    float zoom, float yaw, const float pos[3], const float lt[4],
    int width, int height, AnsiCell* ptr, 
    Inst* player,
    const int scene_shift[2],
    bool perspective);
```

Camera parameters are passed as: `zoom`, `yaw`, `pos[3]`, `scene_shift[2]`
