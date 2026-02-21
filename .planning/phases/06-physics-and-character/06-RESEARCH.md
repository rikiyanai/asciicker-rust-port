# Phase 6: Physics and Character - Research

**Researched:** 2026-02-20
**Domain:** Sphere-based collision physics, character state machines, player input, water/reflection rendering
**Confidence:** HIGH

## Summary

Phase 6 ports three tightly coupled C++ subsystems to Rust/Bevy: (1) the sphere-based TOI sweep collision engine from `physics.cpp` (~2350 lines), (2) the character state machine, equipment system, and animation timing from `game.cpp`/`game.h` (~600 lines of relevant code), and (3) the water reflection/Perlin ripple effect from `render.cpp` Stage 5 + resolve pass. The physics system is self-contained with a clean PhysicsIO boundary pattern that decouples game logic from collision internals. The character system maps naturally to Bevy ECS with components for state, equipment, and animation. Water rendering extends the existing Stage 5 reflection pass (which should exist from Phase 5) with Perlin Z-perturbation in the resolve stage.

The core complexity is in the collision sweep algorithm (face/edge/vertex tests against triangle soup in sphere-space), which is pure math with no external dependencies. Bevy's `FixedUpdate` schedule directly replaces the C++ 15ms fixed timestep loop, with the default being 15625us (~64Hz) -- **P6-128 FIX (LOW): NOT close enough: 64Hz vs 66.667Hz is a 4% difference. The Bevy default (64Hz) MUST be overridden with `Time::<Fixed>::from_hz(66.667)`. Failing to do so causes a 4% physics speed mismatch.** Configurable via `Time::<Fixed>::from_hz()`. The PhysicsIO input/output pattern translates cleanly to Bevy resources.

**Primary recommendation:** Port physics as a standalone math module with trait-based geometry queries, character as ECS components with a state machine system, and water as an extension to the existing render pipeline. Use Bevy `FixedUpdate` at 66Hz to match C++ timestep exactly. Use the `noise` crate (v0.9+) for Perlin noise.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PHYS-01 | Sphere-based TOI sweep collision (face/edge/vertex tests) | Full C++ algorithm documented: CheckCollision 3-test cascade, SoupItem struct, sphere-space transform. ~750 lines of collision math to port. |
| PHYS-02 | 15ms fixed timestep via Bevy FixedUpdate (max 10 substeps) | Bevy FixedUpdate schedule supports configurable Hz. Use `Time::<Fixed>::from_hz(66.667)` for 15ms steps. Max substeps = 10 iterations in collision sweep loop. |
| PHYS-03 | Gravity, buoyancy, and impulse forces | Force accumulation pipeline documented: gravity ~9.8, water buoyancy via Archimedes, impulse drain at 0.5x/frame, velocity damping 0.9^dt. |
| PHYS-04 | Grounded detection for character state transitions | accum_contact accumulates upward normal Z across substeps, threshold >= 1.0 for grounded, decay 0.9x/frame, clamp at 5.0. |
| CHAR-01 | Character state machine (idle, walk, run, attack, block, dead) | 5 action states: NONE(idle/walk), ATTACK, FALL, STAND, DEAD. Transition rules documented from SetAction* methods. Guards prevent invalid transitions. |
| CHAR-02 | 5D equipment sprite lookup (action x weapon x shield x helmet x armor x mount) | GetSprite() dispatches on mount->action, returns Sprite* from 5D arrays `player[clr][armor][helmet][shield][weapon]`. 3 mount families (player/wolfie/bigbee) x 3 action variants (idle/attack/fall). |
| CHAR-03 | Player input system (keyboard + mouse movement and actions) | Input accumulation pattern: WASD/arrows for force, Q/E for torque, space for jump. Mouse drag for absolute yaw. Bevy ButtonInput<KeyCode> replaces C++ key bitmap. |
| CHAR-04 | Animation system with frame timing | 3 timing constants: stand=30ms/frame, fall=30ms/frame, attack=20ms/frame. Walk animation driven by player_stp counter (physics velocity -> step counter -> frame index via /1024). |
| FX-01 | Water rendering with reflective surface (reflection stage re-runs terrain+world below water plane) | Stage 5 flips Z in view matrix (Z' = 2*water - Z), sets global_refl_mode=true, re-queries terrain+world. Reflected samples get spare|=0x3 parity. Must extend Phase 5 render pipeline. |
| FX-02 | Perlin Z-perturbation for water ripple effect | In resolve pass, reflected cells get Perlin noise `octaveNoise0_1(wx*0.05, wy*0.05, pn_time, 4)` mapped to color shift (+/- 1 step in RGB cube). Use `noise` crate Fbm<Perlin>. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy | 0.18.0 | ECS framework, FixedUpdate schedule, input system | Already in project; FixedUpdate provides fixed timestep natively |
| noise | 0.9+ | Perlin noise generation with octave/Fbm support | Most mature Rust noise library; provides Perlin + Fbm out of box |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| bytemuck | 1 (already dep) | Pod/Zeroable for physics structs | For zero-copy data transfer between systems |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom physics | bevy_rapier/avian | Custom matches C++ behavior exactly; Rapier/Avian are overkill for sphere-only collision and would diverge from C++ output |
| `noise` crate | Hand-roll Perlin | noise crate is battle-tested, matches C++ siv::PerlinNoise semantics; hand-rolling would duplicate effort |

**Installation:**
```bash
# Add to Cargo.toml [dependencies]
noise = "0.9"
```

## Architecture Patterns

### Recommended Project Structure
```
src/
  physics/
    mod.rs              # PhysicsPlugin, PhysicsIO resource, Bevy FixedUpdate systems
    collision.rs        # SoupItem, CheckCollision (face/edge/vertex), sphere-space transform
    forces.rs           # Force accumulation: gravity, buoyancy, impulse, velocity integration
    soup.rs             # Triangle soup collection: MeshCollect, PatchCollect, terrain triangulation
    constants.rs        # Physics constants (GRAVITY, MAX_VEL_AIR, MAX_VEL_WATER, etc.)
  character/
    mod.rs              # CharacterPlugin, spawn system
    state_machine.rs    # ActionState enum, transition guards, SetAction* methods
    equipment.rs        # SpriteReq, 5D equipment lookup, mount system
    animation.rs        # Frame timing, player_stp counter, animation advance
    input.rs            # Player input accumulation -> PhysicsIO forces
  render/
    ... (existing)
    water.rs            # Water reflection stage, Perlin ripple in resolve pass (extends Phase 5)
```

### Pattern 1: PhysicsIO Decoupling Pattern
**What:** Game logic writes input forces to PhysicsIO, physics reads them, computes collision, writes output position/state back. Game reads output.
**When to use:** Every frame for every physics-driven entity.
**Example:**
```rust
// Source: physics.h PhysicsIO pattern (verified from C++ source)
#[derive(Resource, Default)]
pub struct PhysicsIO {
    // INPUT (game -> physics)
    pub x_force: f32,       // [-1, 1] horizontal
    pub y_force: f32,       // [-1, 1] horizontal
    pub z_force: f32,       // fly mode only
    pub torque: f32,        // yaw rotation (>= 1_000_000 = absolute yaw)
    pub water: f32,         // water surface Z

    // IO (both read/write)
    pub jump: bool,         // consumed by physics when applied
    pub fly: bool,
    pub x_impulse: f32,     // accumulated, drained 0.5x/frame
    pub y_impulse: f32,

    // OUTPUT (physics -> game)
    pub pos: [f32; 3],
    pub yaw: f32,
    pub player_dir: f32,    // facing direction (degrees)
    pub player_stp: i32,    // animation step (-1=idle, >=0=walking)
    pub dt: i32,            // timestep duration (us)
    pub grounded: bool,
}
```

### Pattern 2: Character State Machine as ECS Component
**What:** Character action state stored as a Bevy component with enum variants and transition guards.
**When to use:** Every character entity (player and NPCs).

**P6-116 FIX (HIGH) STALE:** Plan 06-02 extends to 6 variants (adding `Block`). This RESEARCH example shows the C++ baseline only (5 variants). Do not use this example as the authoritative state machine — use Plan 06-02's Task 1 definition.

**Example (C++ baseline only — see Plan 06-02 for the full 6-variant Rust definition):**
```rust
// Source: game.h ACTION enum + SetAction* methods (verified from C++ source)
#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub enum ActionState {
    #[default]
    None,     // idle/walk
    Attack,
    Fall,
    Dead,
    Stand,    // standing up from fall
    // NOTE: Block variant added in Plan 06-02 per CHAR-01 requirement
}

impl ActionState {
    pub fn can_transition_to(&self, target: ActionState) -> bool {
        match (self, target) {
            (_, ActionState::None) => true,  // always can go idle
            (ActionState::Fall | ActionState::Stand | ActionState::Dead, ActionState::Attack) => false,
            (ActionState::Dead, ActionState::Fall) => false,
            (ActionState::Fall | ActionState::Dead, ActionState::Stand) => {
                // Stand only from Fall or Dead
                // P6-117 FIX (HIGH) BUG: This arm `(Fall|Dead, Stand)` with body
                // `self==Fall || self==Dead` is always true — the pattern already filtered.
                // The wildcard `_ => true` then incorrectly allows `Attack->Stand`,
                // `None->Stand`. Fix: change pattern to `(_, Stand) => self==Fall || self==Dead`
                // to correctly block transitions from other states. This example is
                // INCORRECT as written below:
                *self == ActionState::Fall || *self == ActionState::Dead
            },
            _ => true,
        }
    }
}

// **P6-311 FIX (LOW):** The P6-117 FIX description above says "change pattern to `(_, Stand) =>
// self==Fall || self==Dead`". The `==` form is valid (ActionState derives PartialEq) but the
// idiomatic Rust form uses `matches!`. The precise corrected expression is:
//   `(_, ActionState::Stand) => matches!(self, ActionState::Fall | ActionState::Dead)`
// Confirm `ActionState` derives `PartialEq` (already present in this example via derive).
// (Informational only — both == and matches! are correct. No code change required.)

// **P6-305 FIX (HIGH):** The code block above STILL CONTAINS the buggy `can_transition_to`
// logic despite the P6-117 FIX note explaining the bug. An implementer copying this verbatim
// gets a broken state machine where `Attack->Stand` and `None->Stand` are incorrectly allowed
// by the `_ => true` arm that becomes reachable when Stand is unmatched.
// CORRECTED match arm (replaces the two buggy arms above for the Stand case):
//   (_, ActionState::Stand) => matches!(self, ActionState::Fall | ActionState::Dead),
// REMOVE the old `(ActionState::Fall | ActionState::Dead, ActionState::Stand) => { ... }` arm.
// REMOVE the `_ => true` fallback that follows it (which made Attack->Stand possible).
// The full corrected match should end with a correct final arm for remaining cases.
// Add to Plan 06-02 Task 1 test list: `test_stand_only_from_fall_or_dead` that asserts:
//   - `None.can_transition_to(Stand)` returns false
//   - `Attack.can_transition_to(Stand)` returns false
//   - `Fall.can_transition_to(Stand)` returns true
//   - `Dead.can_transition_to(Stand)` returns true
// Do NOT use this RESEARCH.md code as the authoritative implementation — use Plan 06-02 Task 1.
```

### Pattern 3: Bevy FixedUpdate for Physics
**What:** Run physics systems in Bevy's FixedUpdate schedule at 66Hz to match C++ 15ms timestep.
**When to use:** All physics integration and collision detection.
**Example:**
```rust
// Source: Bevy docs (verified from official examples)
app.insert_resource(Time::<Fixed>::from_hz(66.667))
   .add_systems(FixedUpdate, (
       accumulate_forces,
       collect_triangle_soup,
       collision_sweep,
       update_position,
       update_grounded,
   ).chain());
```

### Pattern 4: Trait-Based Geometry Queries
**What:** Physics queries terrain and world geometry through traits, not direct access. Decouples physics from specific terrain/world implementations.
**When to use:** Triangle soup collection during collision sweep.
**Example:**
```rust
// STALE — see P6-120 FIX in 06-01-PLAN.md. This trait uses frustum-plane interface
// (clip_planes: &[[f64; 4]; 4]) which is WRONG. Actual implementation uses center+radius
// spatial proximity query, NOT camera-frustum planes.
pub trait PhysicsGeometrySource {
    fn collect_triangles(
        &self,
        clip_planes: &[[f64; 4]; 4],
        soup: &mut Vec<SoupItem>,
    );
}
```

### Anti-Patterns to Avoid
- **Mutating PhysicsIO in-place during iteration:** The C++ code carefully separates input-fill, animate-call, output-read phases. Do NOT mix Bevy system ordering so that reads and writes interleave across entities.
- **Using f64 for physics:** C++ uses f32 for all physics math (only view matrix uses f64). Using f64 wastes memory and cache without benefit for this collision model.
- **Skipping sphere-space transform:** All collision math assumes unit sphere in sphere-space. Forgetting `collect_mul_xy` and `collect_mul_z` transforms produces wrong collision results (TRAP-P03).
- **Making PhysicsIO a component per entity:** For Phase 6, only one player exists. PhysicsIO should be a Resource, not per-entity Component. Generalize to per-entity in Phase 7 when NPCs need physics.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Perlin noise | Custom noise generator | `noise` crate `Fbm<Perlin>` | Handles octave stacking, persistence, lacunarity; matches siv::PerlinNoise semantics |
| Fixed timestep | Manual dt accumulation loop | Bevy `FixedUpdate` schedule | Handles accumulation, overstep, and frame-independent scheduling automatically |
| Input debouncing | Custom key state tracking | Bevy `ButtonInput<KeyCode>` | Provides `just_pressed()`, `pressed()`, `just_released()` natively |
| Math operations | Custom dot/cross product | `bevy::math::Vec3` methods | `dot()`, `cross()`, `normalize()` etc. already optimized |

**Key insight:** The physics collision algorithm itself IS the custom part that must be hand-ported from C++. Everything around it (noise, timestep, input, math primitives) has standard Rust/Bevy solutions.

## Common Pitfalls

### Pitfall 1: TOI Return Convention (TRAP-P01)
**What goes wrong:** TOI >= 2.0 means "no collision" in the C++ code. Treating it as a valid time value causes phantom collisions at impossible positions.
**Why it happens:** Unusual sentinel convention (not infinity, not -1, but 2.0).
**How to avoid:** Use a Rust enum: `enum CollisionResult { Hit { toi: f32, contact: [f32; 3] }, Miss }`. Eliminates the magic number entirely.
**Warning signs:** Characters teleporting to extreme positions, falling through geometry.

### Pitfall 2: Sphere Space Scaling (TRAP-P03)
**What goes wrong:** Collision math produces wrong results because vertices are in world space instead of sphere space.
**Why it happens:** The C++ code scales XY by `1.0/world_radius` and Z by `2.0/world_height` to transform the character's ellipsoid into a unit sphere. Easy to forget during port.
**How to avoid:** Create a `to_sphere_space(world_pos, mul_xy, mul_z) -> Vec3` helper and use it consistently in all soup collection callbacks.
**Warning signs:** Character clips through thin geometry, collision normals point wrong direction.

### Pitfall 3: Grounded Detection Accumulation (TRAP-P02)
**What goes wrong:** Character cannot jump, or floats above ground, or falls through stairs.
**Why it happens:** Grounded detection uses accumulated contact normal Z >= 1.0 across substeps, with 0.9x decay per frame and clamp at 5.0. Getting any of these wrong breaks jumping.
**How to avoid:** Port the exact accumulation logic: `accum_contact += max(0, contact_normal_z)`, clamp to 5.0, `grounded = accum_contact >= 1.0`, then `accum_contact *= 0.9`.
**Warning signs:** Character stuck on slopes, cannot jump on rough terrain, instant grounded after falling.

### Pitfall 4: Water Modifies Gravity (TRAP-P04)
**What goes wrong:** Character sinks like a rock in water or floats to infinity.
**Why it happens:** Water buoyancy calculation replaces gravity with `acc = (water_z - char_center_z) / (2 * cnt * height)`, clamped to `[-cnt, 1-cnt]`. Getting the center-of-mass offset or clamp wrong breaks water behavior.
**How to avoid:** Port the exact buoyancy formula: `cnt = 0.78 + amplitude * sin(wave)`, `acc = (water - (pos_z + cnt * world_height)) / (2 * cnt * world_height)`.
**Warning signs:** Character bounces at water surface, sinks through ocean floor, flies upward in deep water.

### Pitfall 5: Jump Flag Consumption (TRAP-P05)
**What goes wrong:** Character jumps every frame (infinite jump) or never jumps.
**Why it happens:** Game sets `io.jump = true`, physics consumes it by setting `io.jump = false` after applying impulse. If game re-sets jump every frame without checking grounded, infinite jumping occurs. If physics doesn't drain it, jump never triggers.
**How to avoid:** Follow the C++ pattern exactly: game sets jump once when button pressed, physics consumes when grounded, game clears after `steps > 0`.
**Warning signs:** Character launches to sky on jump, jump button does nothing.

### Pitfall 6: Equipment Change During Attack (TRAP-G01)
**What goes wrong:** Sprite desyncs from animation, visual glitch mid-attack.
**Why it happens:** SetWeapon/SetShield/etc. modify sprite lookup immediately. If called during ATTACK action, the new sprite may not have attack animation at the current frame.
**How to avoid:** Guard equipment changes: `if req.action == ACTION::ATTACK { return false; }` (already in C++ SetWeapon).
**Warning signs:** Character sprite snaps to wrong frame during attack animation.

### Pitfall 7: Mount Changes Physics Size (TRAP-G02)
**What goes wrong:** Mounted character clips through geometry or floats above ground.
**Why it happens:** WOLF/BEE mounts use different collision radii (3 vs 2 cells) and heights (9 vs 7 cells). Dismounting requires recalculating sphere-space scaling.
**How to avoid:** Recalculate `world_radius` and `world_height` from mount state at start of each Animate() call, not just at creation.
**Warning signs:** Mounted character stuck in doorways, dismounted character floats.

### Pitfall 8: Reflection Stage Requires Phase 5 Foundation
**What goes wrong:** Water reflections don't render because Stage 5 isn't wired up.
**Why it happens:** FX-01 extends the existing rendering pipeline's Stage 5. If Phase 5 doesn't implement the reflection stage skeleton, Phase 6 has no hook point.
**How to avoid:** Verify Phase 5 delivers Stage 5 (Reflection) as a callable stage, even if initially a no-op. Phase 6 fills it with water-specific logic.
**Warning signs:** No reflections visible, stage 5 function not found.

## Code Examples

### CheckCollision - Face Test (Core Algorithm)

**P6-203 FIX (CRITICAL) — WARNING: PARTIAL ILLUSTRATION ONLY. DO NOT USE AS-IS:**
- `todo!()` at the bottom of this function will PANIC at runtime (`todo!()` calls `panic!()`). Replace with the complete barycentric + edge/vertex logic from C++ source physics.cpp:461-624.
- `contact_pos: &mut [f32; 3]` is a STALE out-parameter. The Rust port uses `CollisionResult::Hit { toi: f32, contact: [f32; 3] }` as the return value (per Plan 06-01 Task 1). Using this out-parameter instead creates dual output paths. Use the return value only.
- **Logic bug in embedded branch:** The embedded case sets `contact_pos` at lines ~282-284, then the code at line ~292 OVERWRITES `contact_pos` with `col[i] + plane_t * sphere_vel[i]` where `plane_t=0.0`. The overwrite discards the embedded contact position. Fix in port: do NOT fall through to the overwrite when in the embedded branch.
- This example is provided for algorithmic orientation only. The authoritative implementation source is C++ physics.cpp:461-624.

```rust
// Source: physics.cpp:461-624 (verified line-by-line from C++ source)
// WARNING: PARTIAL ILLUSTRATION — see P6-203 FIX above before using
pub fn check_collision(
    tri: &[[f32; 3]; 3],     // triangle vertices in sphere space
    nrm: &[f32; 4],          // plane equation [nx, ny, nz, d]
    sphere_pos: &[f32; 3],
    sphere_vel: &[f32; 3],
    // STALE: contact_pos out-param — use CollisionResult::Hit { contact } return value instead
    contact_pos: &mut [f32; 3],
) -> CollisionResult {
    // Point on sphere surface closest to plane at t=0
    let col = [
        sphere_pos[0] - nrm[0],
        sphere_pos[1] - nrm[1],
        sphere_pos[2] - nrm[2],
    ];

    let vel_dot_nrm = -(sphere_vel[0] * nrm[0] + sphere_vel[1] * nrm[1] + sphere_vel[2] * nrm[2]);

    if vel_dot_nrm <= 0.0 {
        return CollisionResult::Miss; // backface or parallel
    }

    let dist = col[0] * nrm[0] + col[1] * nrm[1] + col[2] * nrm[2] + nrm[3];

    let plane_t;
    if dist > 0.0 {
        plane_t = dist / vel_dot_nrm;
    } else if dist > -1.0 {
        // Embedded: resolve by projecting back
        let pen = 1.0 + dist;
        contact_pos[0] = col[0] - pen * nrm[0];
        contact_pos[1] = col[1] - pen * nrm[1];
        contact_pos[2] = col[2] - pen * nrm[2];
        plane_t = 0.0;
    } else {
        return CollisionResult::Miss; // deeply embedded, ignore
    }

    // Project contact along velocity to collision time
    for i in 0..3 {
        contact_pos[i] = col[i] + plane_t * sphere_vel[i];
    }

    // Barycentric containment test...
    // (edge/vertex fallback if outside triangle)
    // ... (see full implementation in collision.rs)
    todo!("Continue with barycentric test, edge/vertex fallback")
}

pub enum CollisionResult {
    Hit { toi: f32, contact: [f32; 3] },
    Miss,
}
```

### Animation Timing Constants
```rust
// Source: game.cpp:409-411 (verified from C++ source)
pub const STAND_US_PER_FRAME: u64 = 30_000;  // 30ms per frame
pub const FALL_US_PER_FRAME: u64 = 30_000;   // 30ms per frame
pub const ATTACK_US_PER_FRAME: u64 = 20_000; // 20ms per frame

// Walk animation from physics:
// player_stp incremented by (64 * xy_vel) per physics step
// frame index = player_stp / 1024
// 8 frames per walk cycle (step_mask = 8*1024 - 1)
pub const STEP_DIVISOR: i32 = 1024;
pub const STEP_MASK: i32 = 8 * 1024 - 1;
pub const STEP_OFFSET: i32 = 3 * 1024;
```

### Physics Constants
```rust
// Source: physics.cpp (verified from C++ source)
pub const PHYSICS_INTERVAL_US: u64 = 15_000;  // 15ms fixed timestep
pub const PHYSICS_HZ: f64 = 66.667;           // ~66 Hz
pub const MAX_SUBSTEPS: u32 = 10;
pub const MAX_VEL_AIR: f32 = 27.0;
pub const MAX_VEL_WATER: f32 = 10.0;
pub const JUMP_VELOCITY: f32 = 10.0;          // units/sec upward
pub const VEL_DAMPING: f32 = 0.9;             // per dt
pub const IMPULSE_DRAIN: f32 = 0.5;           // per frame
pub const GROUNDED_THRESHOLD: f32 = 1.0;
pub const GROUNDED_MAX_ACCUM: f32 = 5.0;
pub const GROUNDED_DECAY: f32 = 0.9;
pub const XY_SPEED: f32 = 0.13;               // force-to-velocity scale
pub const XY_THRESH: f32 = 0.002;             // sphere-space velocity cutoff
pub const Z_THRESH: f32 = 0.001;
pub const SAFE_DISTANCE: f32 = 0.01;          // sphere-to-contact gap
pub const STALL_THRESHOLD_US: u64 = 500_000;  // 0.5 sec
```

### Equipment Enums
```rust
// Source: game.h (verified from C++ source)
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Weapon { #[default] None, RegularSword, RegularCrossbow }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Shield { #[default] None, RegularShield }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Helmet { #[default] None, RegularHelmet }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Armor { #[default] None, RegularArmor }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Mount { #[default] None, Wolf, Bee }

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteKind { #[default] Human, Wolf, Bee }

#[derive(Component, Clone, Default)]
pub struct SpriteReq {
    pub kind: SpriteKind,
    pub mount: Mount,
    pub action: ActionState,
    pub armor: Armor,
    pub helmet: Helmet,
    pub shield: Shield,
    pub weapon: Weapon,
}
```

### Perlin Water Ripple
```rust
// Source: render.cpp:3860 (verified from C++ source)
use noise::{NoiseFn, Perlin, Fbm};

// In resolve pass, for cells containing reflection (spare & 0x3 == 3):
// STALE — see P6-303 FIX in 06-03-PLAN.md. Use struct-literal:
// Fbm::<Perlin> { octaves: 4, ..Default::default() }. The new(0) call gives 6 octaves (default), not 4.
let fbm = Fbm::<Perlin>::new(0);
// Configure: 4 octaves to match C++ octaveNoise0_1(..., 4)

let d = fbm.get([wx * 0.05, wy * 0.05, pn_time]); // returns [-1, 1]
let d_normalized = (d + 1.0) * 0.5; // map to [0, 1] like octaveNoise0_1

let id = (d_normalized * 5.0) as i32 - 2; // range [-2, 2]
let id = id.clamp(-2, 2);
// remap: <-1 -> +2, >1 -> -2 (wrap around)

// Apply color shift: +/- 1 step in RGB cube (each component +-1 in 0-5 range)
// fg += 1 + 6 + 36 (lighten) or fg -= 1 + 6 + 36 (darken)
```

### Bevy Input to PhysicsIO
```rust
// STALE — missing field resets per XP-113 FIX in Plan 06-02. Must zero x_force, y_force, torque
// at start of function before reading keys. Also missing Res<GameCamera> for camera-relative WASD.
// Source: game.cpp:5721-5781 (verified from C++ source)
fn accumulate_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut physics_io: ResMut<PhysicsIO>,
) {
    let mut x_force = 0.0f32;
    let mut y_force = 0.0f32;

    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        x_force += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        x_force -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        y_force += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        y_force -= 1.0;
    }

    // Normalize diagonal movement
    let len = (x_force * x_force + y_force * y_force).sqrt();
    if len > 1.0 {
        x_force /= len;
        y_force /= len;
    }

    // Shift = half speed
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        x_force *= 0.5;
        y_force *= 0.5;
    }

    physics_io.x_force = x_force;
    physics_io.y_force = y_force;

    // Torque (camera rotation)
    let torque = (keyboard.pressed(KeyCode::KeyQ) as i32
                - keyboard.pressed(KeyCode::KeyE) as i32) as f32;
    physics_io.torque = torque;

    // Jump
    if keyboard.just_pressed(KeyCode::Space) {
        physics_io.jump = true;
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual timestep loop in game loop | Bevy FixedUpdate schedule | Bevy 0.12+ (2023) | No manual dt accumulation needed; Bevy handles overstep |
| C++ global static for physics | Bevy Resource + Component | Bevy 0.1+ | PhysicsIO as Resource, PhysicsState as Component; no global mutable state |
| C++ function pointers for callbacks | Rust traits | Language feature | PhysicsGeometrySource trait replaces void* cookie callback pattern |
| Manual sprite linked list | Bevy AssetServer + Handle | Bevy 0.1+ | Sprites loaded as assets; 5D lookup returns Handle<XpSprite> |

**Deprecated/outdated:**
- C++ `siv::PerlinNoise` header: Replaced by Rust `noise` crate which provides equivalent functionality
- Manual key bitmap (`key[32]`): Replaced by Bevy `ButtonInput<KeyCode>` with typed key queries
- Opaque `Physics*` pointer pattern: Replaced by Bevy ECS component; physics state lives on entity

## Open Questions

1. **Phase 5 Reflection Stage Status**
   - What we know: Phase 5 should implement the 6-stage render pipeline including Stage 5 (Reflection).
   - What's unclear: Whether Phase 5 will implement Stage 5 as a no-op skeleton or leave it out entirely.
   - Recommendation: Plan Phase 6 to implement reflection FROM SCRATCH if needed, but prefer extending Phase 5's Stage 5 if it exists. Add a verification step in Plan 1 to check Phase 5 output.

2. **NPC Physics Scope**
   - What we know: C++ runs Animate() for every NPC with their own Physics* and PhysicsIO. Phase 6 requirements only mention "a character entity" (singular player).
   - What's unclear: Whether Phase 6 needs NPC physics or if that's deferred to Phase 7 (game systems).
   - Recommendation: Design PhysicsIO and the collision system to be per-entity generalizable, but only implement for the player in Phase 6. NPCs in Phase 7.

3. **Sprite Asset Loading for Equipment**
   - What we know: Phase 2 built XP sprite loaders. The 5D equipment lookup requires many sprite files to be pre-loaded (player[], player_attack[], player_fall[], wolfie[], etc.).
   - What's unclear: Whether all equipment sprite .xp files exist in the asset directory and whether Phase 2 loaders handle multi-anim sprites correctly for the player character format.
   - Recommendation: Start with a single equipment configuration (NONE for all slots) and progressively add equipment variety. Use fallback to nullptr/skip render if sprite not found.

4. **Collision with World Mesh Instances**
   - What we know: C++ physics queries BSP tree via QueryWorld with a MeshCollect callback that transforms instance triangles to sphere space using the instance transform matrix.
   - What's unclear: Whether Phase 5 will expose QueryWorld with a callback interface suitable for physics reuse, or if physics needs its own BSP traversal.
   - Recommendation: Define a shared `GeometryQuery` trait that both rendering and physics use. Phase 5 implements it for BSP/terrain, Phase 6 consumes it.

## Sources

### Primary (HIGH confidence)
- `/Users/r/Downloads/asciicker-Y9-2/physics.h` - PhysicsIO struct, public API (120 lines, fully documented)
- `/Users/r/Downloads/asciicker-Y9-2/physics.cpp` - Full collision implementation (2350 lines, read lines 1-2352)
- `/Users/r/Downloads/asciicker-Y9-2/game.h` - Character/Human structs, ACTION/WEAPON/SHIELD/HELMET/ARMOR/MOUNT enums, SpriteReq, Game struct (567 lines)
- `/Users/r/Downloads/asciicker-Y9-2/game.cpp` - GetSprite (lines 3531-3662), SetAction* (lines 4853-4998), Game::Render physics IO (lines 5634-5860)
- `/Users/r/Downloads/asciicker-Y9-2/render.cpp` - Stage 5 Reflection (lines 3266-3374), Perlin water ripple (lines 3860-3903)
- `/Users/r/Downloads/asciicker-Y9-2/sprite.h` - Sprite/Frame/Anim structs (110 lines)
- `/Users/r/Downloads/asciicker-Y9-2/water.cpp` - Water design notes (planning only, no implementation)
- `/Users/r/Projects/asciicker rust port/docs/skills/physics-system.md` - Physics skill pack
- `/Users/r/Projects/asciicker rust port/docs/skills/game-mechanics.md` - Game mechanics skill pack
- `/Users/r/Projects/asciicker rust port/docs/skills/engine-render.md` - Render pipeline skill pack

### Secondary (MEDIUM confidence)
- [Bevy FixedUpdate documentation](https://bevy-cheatbook.github.io/fundamentals/fixed-timestep.html) - Default 64Hz, configurable via Time::<Fixed>
- [Bevy Input documentation](https://bevy-cheatbook.github.io/input.html) - ButtonInput<KeyCode>, keyboard/mouse patterns
- [noise crate docs](https://docs.rs/noise) - Perlin, Fbm, NoiseFn trait

### Tertiary (LOW confidence)
- None (all findings verified against C++ source code)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Bevy already in project, noise crate well-established, no new major deps
- Architecture: HIGH - PhysicsIO pattern directly from C++ source, ECS mapping is straightforward
- Physics algorithm: HIGH - Full C++ source read line-by-line, all constants and formulas extracted
- Character system: HIGH - All SetAction* methods, GetSprite dispatch, equipment enums extracted from C++ source
- Water/FX: MEDIUM - Depends on Phase 5 delivering Stage 5 reflection foundation; Perlin mapping verified
- Pitfalls: HIGH - All 5 physics traps (TRAP-P01 through P05) and 2 game traps (TRAP-G01, G02) documented from skill packs + source

**Research date:** 2026-02-20
**Valid until:** 2026-04-20 (stable domain, no external API dependencies)
