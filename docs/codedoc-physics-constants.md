# Asciicker Physics Constants

Source: C++ `physics.cpp` from `/Users/r/Downloads/asciicker-Y9-2/`

## 1. Gravity Constant

**Value:** ~9.8 world units/sec² downward (-Z direction)

```cpp
// From physics.cpp comments (line 127):
// Gravity: ~9.8 world units/sec² downward (modulated by water buoyancy)
```

**Actual implementation:** Gravity is implicitly handled through the buoyancy formula. When above water (not submerged), the buoyancy acceleration `acc` becomes negative, effectively applying gravity.

---

## 2. Jump Velocity

**Value:** 10 units/sec upward

```cpp
// From physics.cpp line 2079:
if (phys->vel[2] < 0)
    phys->vel[2] = 10; // Jump velocity (10 units/sec upward)
else
    phys->vel[2] += 10; // Additive for double-jump feel
```

**Notes:**
- Applied only when `io->jump == true` and `io->grounded == true`
- If already rising (`vel[2] > 0`), adds to existing velocity (double-jump behavior)
- Max vertical velocity clamped to 20 units/sec (line 2102-2103)

---

## 3. Speed Limits

### Air/Ground Speed

**Max velocity:** 27 units/sec

```cpp
// From physics.cpp line 1453:
float lim = 27;
lim *= xy_len * xy_len*xy_len;  // Cubic input scaling
```

### Water Speed

**Max velocity:** 10 units/sec (fully submerged)

### Transition Formula

```cpp
// From physics.cpp line 1451:
float xy_limit = 27 - 17 * (phys->water - phys->pos[2]) / world_height;

// Clamped bounds:
if (xy_limit < 10)
    xy_limit = 10;  // Minimum water speed
if (xy_limit > lim)
    xy_limit = lim;  // Apply input-scaled limit
```

**Explanation:** 
- Speed linearly interpolates between 27 (above water) and 10 (fully submerged)
- `world_height` = character height in world units (~7-9 cells scaled)

---

## 4. Friction Values

### Ground Friction

**Formula:** `powf(0.9f, dt)` - exponential velocity decay per substep

```cpp
// From physics.cpp lines 1483-1485:
float vel_damp = powf(0.9f, dt);
phys->vel[0] *= vel_damp;
phys->vel[1] *= vel_damp;
```

**Applied:** Only when grounded and moving (XY plane)

### Water Resistance

**XY Plane (horizontal):**
```cpp
// From physics.cpp line 1555:
float xy_res = powf(1.0f - 0.5f * res, dt);
// res = (water - pos_z) / world_height  (0 = above water, 1 = fully submerged)
```

**Z Axis (vertical):**
```cpp
// From physics.cpp line 1556:
float z_res = powf(1.0f - 0.1f * res, dt);
```

**Notes:**
- Water depth fraction `res` ranges from 0 (above water) to 1 (fully submerged)
- XY resistance is 5x stronger than Z resistance (0.5 vs 0.1 factor)
- This allows easier vertical movement in water

### Impulse Decay

```cpp
// From physics.cpp lines 1572-1573:
io->x_impulse *= 0.5;
io->y_impulse *= 0.5;
```

---

## 5. Water Buoyancy Formula

**Full implementation from physics.cpp lines 1506-1516:**

```cpp
// Wave animation (cosmetic)
float wave = 2 * (int)((phys->stamp >> 10) & 0x7FF) * (float)M_PI / 0x800;
float ampl = 0.05f;
if (ix || iy)
    ampl = 0.1f;  // Larger wave amplitude when moving

// Center of mass as fraction of character height
float cnt = 0.78f + ampl * sinf(wave);

// Buoyancy acceleration
float acc = (phys->water - (phys->pos[2] + cnt * world_height)) / (2 * cnt * world_height);

// Clamp to prevent extreme acceleration
if (acc < 0 - cnt)
    acc = 0 - cnt;  // Clamp downward (falling in air)
if (acc > 1 - cnt)
    acc = 1 - cnt;  // Clamp upward (deeply submerged)

// Apply to velocity
phys->vel[2] += dt * acc;
```

**Formula Breakdown:**

| Component | Value | Description |
|-----------|-------|-------------|
| `cnt` | 0.78 ± 0.1 | Center of mass as fraction of character height |
| `world_height` | ~7-9 cells | Character height in world units |
| `water` | float | Water surface Z coordinate |
| `pos[2]` | float | Character Z position |
| `acc` | float | Buoyancy acceleration (positive=up, negative=down) |

**Interpretation:**
- When `pos_z + cnt * world_height > water`: acceleration is negative (gravity dominates)
- When `pos_z + cnt * world_height < water`: acceleration is positive (buoyancy dominates)
- Clamping ensures stable behavior at extremes

---

## 6. Collision Radius

**Value:** 1.0 unit radius (sphere)

```cpp
// From physics.cpp comments (line 8):
// Sphere: 1.0 unit radius, centered at character position + 0.5*height offset
```

```cpp
// Used in collision detection (line 444):
float C = DotProduct(p_ps, p_ps) - 1.0f;  // radius^2 = 1.0
```

**Character Height Offset:**
- Sphere center is at `character_position + 0.5 * height`
- This means the character's "feet" are at position, sphere center is halfway up

---

## Summary Table

| Constant | Value | Location |
|----------|-------|----------|
| Gravity | ~9.8 units/sec² | buoyancy formula |
| Jump velocity | 10 units/sec | line 2079 |
| Max air/ground speed | 27 units/sec | line 1453 |
| Max water speed | 10 units/sec | line 1456 |
| Ground friction | 0.9^dt | line 1483 |
| Water XY resistance | (1 - 0.5*res)^dt | line 1555 |
| Water Z resistance | (1 - 0.1*res)^dt | line 1556 |
| Collision radius | 1.0 unit | line 8 comment |
| Physics timestep | 15ms (~66 Hz) | line 1277 |
| Max substeps | 10 per frame | line 1674 |

---

## Reference: World Height Calculation

```cpp
// From physics.cpp lines 1272-1275:
float height_cells = req->mount ? 9.0f : 7.0f;  // 7 for player, 9 for mounts
static const float world_height = height_cells * 2 / 3 / (float)cos(30 * M_PI / 180) * HEIGHT_SCALE;

// HEIGHT_SCALE = 16 (from terrain.h line 54)
// Results in approximately:
//   Player: ~7 * 2/3 / 0.866 * 16 ≈ 215 world units
//   Mount:  ~9 * 2/3 / 0.866 * 16 ≈ 277 world units
```
