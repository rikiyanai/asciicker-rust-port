# Asciicker Physics System Documentation

This document provides comprehensive documentation of the Asciicker physics system as implemented in `physics.cpp` and `physics.h`.

## Table of Contents

1. [Physics Model Overview](#physics-model-overview)
2. [Character Movement and Collision](#character-movement-and-collision)
3. [Gravity and Jumping](#gravity-and-jumping)
4. [Raycasting for Line-of-Sight](#raycasting-for-line-of-sight)
5. [Physics I/O Interface](#physics-io-interface)
6. [Spatial Queries](#spatial-queries)

---

## Physics Model Overview

### Dimensionality

The Asciicker physics system implements a **3D physics model** with the following characteristics:

- **Coordinate System**: World space uses global XYZ coordinates where +Z is up (gravity acts in -Z direction)
- **Character Representation**: All characters (players, NPCs, mounts) are modeled as **1.0 unit radius spheres**
- **Height Offset**: Sphere center is positioned at character position + 0.5*height offset

### Collision Detection Method

The system uses **Continuous Collision Detection (CCD)** via **Time-of-Impact (TOI) sweep**:

- Prevents tunneling through thin geometry by sweeping the sphere along its velocity vector
- Finds the earliest collision along the movement trajectory
- Returns TOI value in range [0, 1] representing fraction of timestep until collision

### Geometry Sources

Collision detection operates against two geometry sources:

1. **Terrain Heightfield**: Quadtree patches with 2 triangles per height cell
2. **World Meshes**: BSP tree instances containing triangle soup

### Collision Algorithm

The `CheckCollision` method (physics.cpp:465-821) performs sphere-triangle intersection using three sequential tests:

#### 1. Face Collision (Plane Intersection + Barycentric Containment)

```
TOI calculation:
  t = (1 - nrm[3] - dot(nrm, sphere_pos)) / dot(nrm, sphere_vel)

Where:
  - nrm = triangle plane normal (nx, ny, nz, d)
  - sphere_pos = current sphere center position
  - sphere_vel = sphere velocity vector
```

- Calculate TOI when sphere surface hits triangle plane
- Project contact point onto plane along velocity vector
- Test if contact is inside triangle using barycentric coordinates
- Return TOI if valid face collision

#### 2. Edge Collision (Sphere-vs-Line-Segment)

If face test fails, test each of 3 triangle edges:

- Treat edge as infinite cylinder (radius=1.0) around edge line
- Find closest point on edge to sphere center
- Calculate TOI using quadratic equation derivation:
  ```
  A = V·V         (perpendicular velocity magnitude squared)
  B = 2*(U·V)     (twice the dot product)
  C = U·U - r²*vc⁴ (perpendicular distance squared at t=0, scaled)
  ```

#### 3. Vertex Collision (Sphere-vs-Sphere)

If edge test fails, test each of 3 triangle vertices:

- Treat vertex as zero-radius stationary sphere
- Standard moving sphere-vs-sphere collision (quadratic equation):
  ```
  A*t² + B*t + C = 0
  A = sphere_vel·sphere_vel
  B = 2*dot(sphere_pos - V, sphere_vel)
  C = |sphere_pos - V|² - 1.0
  ```

### Collision Response

Velocity reflection along contact normal:

```cpp
// Remove perpendicular component (inelastic collision, restitution=0)
float project = DotProduct(sphere_vel, slide_normal);
sphere_vel[0] -= slide_normal[0] * project;
sphere_vel[1] -= slide_normal[1] * project;
sphere_vel[2] -= slide_normal[2] * project;
```

---

## Character Movement and Collision

### Input Forces

Movement is driven by input forces from gamepad/keyboard:

```cpp
float x_force;  // Horizontal force X (right=positive, left=negative), normalized [-1, 1]
float y_force;  // Horizontal force Y (forward=positive, backward=negative), normalized [-1, 1]
```

### Velocity Integration

The physics system uses **Euler integration** (physics.cpp:1352-1486):

```cpp
phys->vel[0] += (float)(dt * (dx * cos(yaw) - dy * sin(yaw)));
phys->vel[1] += (float)(dt * (dx * sin(yaw) + dy * cos(yaw)));
```

### Friction

Friction is applied differently based on state:

- **Grounded**: Velocity damping of `pow(0.9f, dt)` per substep
- **Airborne**: No friction
- **In Water**: Water resistance with depth-based factors:
  - XY resistance: `pow(1.0f - 0.5f * res, dt)`
  - Z resistance: `pow(1.0f - 0.1f * res, dt)`

### Speed Limits

- **Air/Ground**: 27 units/sec maximum
- **In Water**: 10 units/sec maximum (interpolated based on depth)

### Player Direction

Character facing direction for animation is separate from camera yaw:

- Quantized to 8 directions: 0, 45, 90, 135, 180, 225, 270, 315 degrees
- Interpolates smoothly toward input direction using 0.1 factor per frame
- Used to select correct sprite frame from 8-directional sprite sheets

### Animation Step Counter

```cpp
int player_stp;  // -1 = idle, >=0 = walking (frame = player_stp / 1024)
```

Incremented by velocity magnitude:
- Normal mounts: `player_stp += (int)(64 * xy_vel)`
- Flying mounts: `player_stp += (int)(24 * xy_vel)`

### Character Radius by Type

```cpp
float radius_cells = req->mount ? 3.0f : 2.0f;  // Mounts: 3 cells, Players: 2 cells
float world_radius = radius_cells / patch_cells * world_patch;
```

---

## Gravity and Jumping

### Gravity

Constant downward acceleration in world Z:

- **Base gravity**: ~9.8 world units/sec² (acts in -Z direction)
- Applied via Euler integration: `vel[2] += dt * acc`

### Water Buoyancy

When character is submerged, **Archimedes principle** applies:

```cpp
float cnt = 0.78f + ampl * sinf(wave);  // Center of mass (fraction of height)
float acc = (water_z - character_center_z) / (2 * cnt * height);

if (acc < -cnt) acc = -cnt;   // Clamp downward acceleration
if (acc > 1 - cnt) acc = 1 - cnt;  // Clamp upward acceleration
```

Buoyancy calculation factors:
- Wave animation: `2 * (stamp >> 10) & 0x7FF` with amplitude 0.05-0.1
- Depth factor: `(water - pos[2]) / world_height` in range [0, 1]

### Jump Mechanics

**Jump Impulse** (physics.cpp:2066-2085):

```cpp
if (phys->accum_contact >= 1.0 || req->mount > 1)  // Grounded or flying mount
{
    if (io->jump)
    {
        if (phys->vel[2] < 0)
            phys->vel[2] = 10;  // Jump velocity (10 units/sec upward)
        else
            phys->vel[2] += 10;  // Add to existing upward velocity
        
        io->jump = false;  // Consume jump input
    }
}
```

**Jump Conditions**:
- Must be grounded (`accum_contact >= 1.0`) OR flying mount
- Jump input flag must be set (consumed on use)
- Maximum fly height check for mounts (prevents infinite flight)

### Auto-Jump (Step Climbing)

When character runs into low obstacles (physics.cpp:1799-1805):

```cpp
if (!io->jump && !io->fly && collision_time < 0.2f && slide_normal[2] < 0.8f)
{
    io->jump = true;  // Trigger automatic step climb
}
```

Conditions:
- `collision_time < 0.2`: Hit obstacle very early in trajectory
- `slide_normal[2] < 0.8`: Wall is steep (not a gentle floor)

### Fly Mode

For flying mounts, gravity is disabled and z_force controls vertical movement:

```cpp
if (io->fly)
{
    phys->vel[2] += dt * io->z_force;  // Direct vertical force
    phys->vel[2] *= powf(0.9f, dt);   // Drag/damping
}
```

---

## Raycasting for Line-of-Sight

The physics system uses raycasting functions for line-of-sight queries:

### HitWorld - World Mesh Raycast

Located in `world.h/cpp`, used for intersection with BSP tree meshes:

```cpp
Inst* HitWorld(World* w, double p[3], double v[3], double ret[3], 
               double nrm[3], bool positive_only = false, 
               bool editor = false, bool solid_only = false, 
               bool sprites_too = true);
```

**Parameters**:
- `p[3]`: Ray origin (XYZ)
- `v[3]`: Ray direction (XYZ)
- `ret[3]`: Return point (XYZ + t)
- `nrm[3]`: Return normal at hit point
- `positive_only`: Only hit positive-facing triangles (backface culling)

### HitTerrain - Terrain Heightfield Raycast

Located in `terrain.h/cpp`, used for intersection with quadtree terrain:

```cpp
Patch* HitTerrain(Terrain* t, double p[3], double v[3], double ret[4], 
                  double nrm[3] = 0, bool positive_only = false);
```

**Implementation**:
- Uses octant-based recursive traversal (HitTerrain0-7)
- Each octant has specialized tracer function
- Queries quadtree patches near ray trajectory

### Usage in Physics System

**Safe Spawn Positioning** (physics.cpp:2276-2285):

```cpp
// Prevent spawning inside terrain
double p[3] = { phys->pos[0], phys->pos[1], -1 };  // Ray origin (spawn XY, very low Z)
double v[3] = { 0, 0, -1 };  // Ray direction (downward)
double r[4];  // Hit result
double n[3];  // Hit normal

Patch* patch = HitTerrain(phys->terrain, p, v, r, n);

if (patch)
    phys->pos[2] = (float)r[2] + 200;  // Spawn 200 units above terrain
```

### Line-of-Sight Queries

For line-of-sight detection (not currently in physics.cpp but available):

- Use `HitWorld` with ray from character eye position in look direction
- Returns first mesh hit (can use `positive_only=true` for visible surfaces)
- Distance check can determine if line-of-sight is blocked

---

## Physics I/O Interface

The `PhysicsIO` struct (physics.h:26-111) provides the **input/output contract** between game logic and physics:

### Design Pattern

Uses **opaque pointer pattern** - game.cpp does not access Physics internals directly:

1. Game fills INPUT fields based on player input or AI
2. Game calls `Animate(physics, timestamp, &io, sprite_req, is_player)`
3. Physics updates OUTPUT fields
4. Game reads OUTPUT fields to update rendering

### Input Fields (game.cpp → physics.cpp)

| Field | Type | Description |
|-------|------|-------------|
| `x_force` | float | Horizontal force X, normalized [-1, 1] |
| `y_force` | float | Horizontal force Y, normalized [-1, 1] |
| `z_force` | float | Vertical force (fly mode only) |
| `torque` | float | Yaw rotation force (or absolute yaw if >= 1000000) |
| `water` | float | Water surface Z coordinate for buoyancy |
| `jump` | bool | Jump requested (consumed by physics if grounded) |
| `fly` | bool | Fly mode enabled (disables gravity) |

### IO Fields (both directions)

| Field | Type | Description |
|-------|------|-------------|
| `x_impulse` | float | Accumulated horizontal impulse X (combat knockback) |
| `y_impulse` | float | Accumulated horizontal impulse Y (combat knockback) |

Impulse handling:
- Game adds impulses (e.g., hit by enemy)
- Physics applies and drains: `impulse *= 0.5` per frame

### Output Fields (physics.cpp → game.cpp)

| Field | Type | Description |
|-------|------|-------------|
| `pos[3]` | float | Updated world position (X, Y, Z) |
| `yaw` | float | Camera/character yaw angle in degrees |
| `player_dir` | float | Character facing direction (for animation), degrees |
| `player_stp` | int | Animation step counter (-1=idle, >=0=walking) |
| `dt` | int | Physics timestep duration in microseconds |
| `grounded` | bool | True if character has ground contact |

### Animate Function

```cpp
int Animate(Physics* phys, uint64_t stamp, PhysicsIO* io, 
            const SpriteReq* req, bool me);
```

**Returns**: Number of physics substeps executed (for animation frame sync)

**Parameters**:
- `phys`: Opaque physics state handle
- `stamp`: Current timestamp in microseconds
- `io`: Input/output structure
- `req`: Sprite requirements (for audio properties)
- `me`: True if this is the player (for audio events)

---

## Spatial Queries

### Geometry Collection Pipeline

The physics system collects collision geometry via callbacks (physics.cpp:1602-1633):

#### QueryWorld - BSP Tree Meshes

```cpp
QueryWorldCB cb = { Physics::MeshCollect, Physics::SpriteCollect };
QueryWorld(phys->world, 4, clip_world, &cb, phys);
```

- Traverses BSP tree to find meshes near character
- Invokes `MeshCollect` callback for each mesh instance
- Transforms mesh triangles to sphere space

#### QueryTerrain - Quadtree Heightfield

```cpp
QueryTerrain(phys->terrain, 4, clip_world, 0xAA, Physics::PatchCollect, phys);
```

- Traverses terrain quadtree
- Invokes `PatchCollect` callback for each terrain patch
- Generates triangles from heightfield cells

### Triangle Soup

Collected triangles stored in `SoupItem` array:

```cpp
struct SoupItem
{
    float tri[3][3];    // Triangle vertices in sphere space
    int material;       // Surface material ID (for audio)
    float nrm[4];      // Plane equation (nx, ny, nz, d)
};
```

**Precomputed data per triangle**:
- Vertices transformed to sphere space
- Plane equation for fast collision tests
- Material ID derived from triangle color

### Sphere Space Transform

World coordinates transformed to sphere space for collision math:

```cpp
// Horizontal scaling (radius-based)
phys->collect_mul_xy = 1.0f / world_radius;

// Vertical scaling (ellipsoid)
phys->collect_mul_z = 2.0f / world_height;

float sphere_pos[3] =
{
    phys->pos[0] * phys->collect_mul_xy,
    phys->pos[1] * phys->collect_mul_xy,
    (phys->pos[2] + world_height * 0.5f) * phys->collect_mul_z,
};
```

### AABB Clipping

Query uses clipping planes to cull distant geometry:

```cpp
double clip_world[4][4] =
{
    { 1,  0, 0, qx - cx },  // +X plane
    {-1,  0, 0, qx + cx },  // -X plane
    { 0,  1, 0, qy - cy },  // +Y plane
    { 0,-1, 0, qy + cy },   // -Y plane
};
```

Only triangles within bounding box around character trajectory are collected.

### Material Detection

Triangle material determined from color (physics.cpp:872-938):

| Material | Detection Criteria |
|----------|-------------------|
| Rock (0) | Low saturation (grayscale) |
| Wood (1) | Red or blue dominant |
| Dirt (2) | Explicit terrain material |
| Grass (3) | Green dominant + low elevation |
| Hi-Grass (4) | Green dominant + high elevation |
| Blood (5) | Explicit terrain material |
| Water (6) | Character legs submerged (voting override) |

---

## Physics Constants Summary

| Constant | Value | Description |
|----------|-------|-------------|
| Timestep | 15000 μs (15ms) | Fixed physics step (~66 Hz) |
| Gravity | ~9.8 units/sec² | Downward acceleration |
| Jump Velocity | 10 units/sec | Initial upward velocity |
| Max Air Speed | 27 units/sec | Horizontal speed limit in air |
| Max Water Speed | 10 units/sec | Horizontal speed limit in water |
| Friction (ground) | 0.9^dt | Velocity damping per substep |
| Water XY Resistance | 1.0 - 0.5*depth | Horizontal water drag |
| Water Z Resistance | 1.0 - 0.1*depth | Vertical water drag |
| Sphere Radius | 1.0 | Character collision radius |
| Substep Limit | 10 | Maximum iterations per timestep |

---

## Key Functions Reference

| Function | Purpose |
|----------|---------|
| `Animate()` | Main physics update - integrate forces, sweep collisions |
| `CheckCollision()` | Sphere-triangle collision test returning TOI |
| `CreatePhysics()` | Allocate and initialize physics state |
| `DeletePhysics()` | Free physics state and soup buffer |
| `SetPhysicsPos()` | Teleport character (bypasses collision) |
| `SetPhysicsYaw()` | Set yaw and angular velocity directly |
| `SetPhysicsDir()` | Set player facing direction |

---

## File Dependencies

- **physics.h**: Public API and PhysicsIO structure
- **physics.cpp**: Implementation (this file)
- **terrain.h/cpp**: Heightfield geometry queries
- **world.h/cpp**: BSP tree mesh geometry queries
- **matrix.h**: Matrix transformations (Product, DotProduct, CrossProduct)
- **audio.h/cpp**: Footstep and landing sound events
- **game.cpp**: Calls Animate() each frame
