---
name: physics-system
description: Use when working with the sphere-based collision detection and physics integration in Asciicker. Covers force accumulation, collision sweep, and grounded detection.
---

# Skill: Physics System

Sphere-based collision and movement integration for ASCII characters.

## Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `physics.cpp` | ~2350 | Collision detection, force integration |
| `physics.h` | ~120 | PhysicsIO interface, public API |

## Collision Model

- **Character shape:** 1.0 unit radius sphere
- **Geometry sources:** Terrain heightfield (quadtree) + World meshes (BSP tree)
- **Detection method:** Time-of-impact (TOI) sweep
- **Response:** Velocity reflection with friction/restitution

## PhysicsIO Pattern

Decouples game logic from physics internals:

```
Game fills INPUT fields → Animate() → Physics fills OUTPUT fields → Game reads
```

**Input fields:** `x_force`, `y_force`, `z_force`, `torque`, `jump`, `fly`, `water`

**Output fields:** `pos[3]`, `yaw`, `player_dir`, `player_stp`, `grounded`, `dt`

**IO fields:** `x_impulse`, `y_impulse` (accumulated and drained)

## Collision Algorithm (3 Tests)

1. **Face collision** - Plane intersection + barycentric containment
2. **Edge collision** - Sphere-vs-line-segment (if face fails)
3. **Vertex collision** - Sphere-vs-sphere (if edge fails)

## Physics Constants

| Constant | Value |
|----------|-------|
| Gravity | ~9.8 units/sec² |
| Timestep | 15ms fixed (~66 Hz) |
| Max velocity (air) | 27 units/sec |
| Max velocity (water) | 10 units/sec |
| Max substeps | 10 per frame |

## Known Traps

### TRAP-P01: TOI Return Convention
TOI >= 2 means "no collision" (not a valid time value). Code compares `toi < earliest_toi` where `earliest_toi` starts at 2.0.

### TRAP-P02: Grounded Detection Threshold
`accum_contact_z >= 1.0` determines grounded state. This accumulates upward normals across substeps, not a single collision.

### TRAP-P03: Sphere Space Scaling
All collision math happens in "sphere space" (scaled by 1/radius). Forgetting to transform vertices produces wrong collision results.

### TRAP-P04: Water Modifies Gravity
When `water > pos[2]`, gravity is reduced by buoyancy. Physics behavior changes dramatically at water boundary.

### TRAP-P05: Jump Flag is Consumed
Game sets `io.jump = true`, physics sets it to `false` when applied. Holding jump button requires game to re-set it each frame.

## Key Functions

```cpp
int Animate(Physics* phys, uint64_t stamp, PhysicsIO* io, const SpriteReq* req, bool me);
Physics* CreatePhysics(Terrain* t, World* w, float pos[3], float dir, float yaw, uint64_t stamp);
void DeletePhysics(Physics* phys);
void SetPhysicsPos(Physics* phys, float pos[3], float vel[3]);
```

## Port Considerations

- **Complexity:** ~2350 lines of collision math
- **Dependencies:** Terrain (quadtree) + World (BSP) queries
- **Rust approach:** Port as-is with traits for geometry queries

---

## Bevy Mapping

### PhysicsIO as Resource

`PhysicsIO` is a Bevy `Resource`, NOT a Component. The player has exactly one physics state. For NPCs, each NPC entity has its own `PhysicsIO` as a Component — but the physics LOGIC is still a plain Rust function, not a separate system per NPC.

### FixedUpdate Schedule

The physics `animate` function runs in Bevy's `FixedUpdate` schedule (default 64 Hz, matching C++ 15ms timestep of ~66 Hz). This ensures deterministic physics regardless of render framerate.

**Schedule placement:**
```
Update         -> Input accumulation (apply_torque_to_camera writes PhysicsIO.yaw)
FixedUpdate    -> animate() system (reads forces, runs collision, writes pos/vel/grounded)
PostUpdate     -> Sync positions to Transform components, then render
```

### Collision and Forces as Plain Rust Functions

The collision detection (`sphere_vs_face`, `sphere_vs_edge`, `sphere_vs_vertex`) and force accumulation (`accumulate_forces`) are **plain Rust functions**, NOT separate Bevy systems.

**Rationale:**
- If collision and forces were separate systems, they would fight over mutable access to velocity and position fields
- Bevy cannot parallelize them anyway (they read and write the same data)
- The C++ `Animate()` is one function that calls collision and force subfunctions — the Rust port mirrors this
- Splitting into systems would require `SystemParam` gymnastics to share mutable state, adding complexity for zero benefit

### C++ to Bevy Mapping Table

| C++ Construct | Bevy Target | Rationale |
|---------------|-------------|-----------|
| `Physics*` (opaque handle) | `Resource` (`PhysicsState`) for player | Single player physics |
| `PhysicsIO` struct | `Resource` (`PhysicsIO`) for player; `Component` for NPCs | Input/output decoupling pattern preserved |
| `Animate()` function | `animate_system` in `FixedUpdate` | One system, calls plain Rust subfunctions |
| `accumulate_forces()` | Plain Rust function (`fn accumulate_forces(&PhysicsIO) -> Vec3`) | Called from `animate_system`, reads forces immutably |
| `collision.cpp` sweep tests | Plain Rust functions in `collision.rs` | Called from `animate_system`, not separate systems |
| `SoupItem` triangle array | Plain Rust `Vec<SoupItem>` (local to animate call) | Temporary geometry collection, not a Resource |
| Terrain geometry query | `PhysicsGeometrySource` trait, impl `TerrainGeometrySource` | Reads `Res<RuntimeTerrain>` immutably |
| World geometry query | `PhysicsGeometrySource` trait, impl `WorldGeometrySource` | Reads `Res<RuntimeWorld>` immutably |
| `CreatePhysics()` / `DeletePhysics()` | Entity spawn/despawn with physics components | Lifecycle managed by ECS |

### Borrow Pattern

The `animate_system` signature avoids borrow conflicts:

```rust
fn animate_system(
    mut physics_io: ResMut<PhysicsIO>,      // mutable: writes pos, vel, grounded
    terrain: Res<RuntimeTerrain>,            // immutable: geometry source
    world: Res<RuntimeWorld>,                // immutable: geometry source
    time: Res<Time>,                         // immutable: delta time
) {
    // All collision/force functions are called here, not as separate systems
    let forces = accumulate_forces(&physics_io);  // immutable borrow
    // ... collision sweep, integration, grounded detection
}
```

If collision and forces were separate systems, Bevy's scheduler would serialize them anyway (both need `ResMut<PhysicsIO>`), adding scheduling overhead for zero parallelism.

### TRAP: Do NOT Make Physics a Per-Entity System (Yet)

For Phase 6 (single player), physics is one `Resource`. When NPC physics is added (Phase 7), each NPC gets `PhysicsIO` as a `Component`, and the animate system becomes:

```rust
fn animate_npcs_system(
    mut query: Query<&mut PhysicsIO, With<Npc>>,
    terrain: Res<RuntimeTerrain>,
    world: Res<RuntimeWorld>,
    time: Res<Time>,
) {
    for mut io in &mut query {
        animate_single(&mut io, &terrain, &world, time.delta_secs());
    }
}
```

The collision math is identical — only the data source changes from `ResMut<PhysicsIO>` to `Query<&mut PhysicsIO>`.
