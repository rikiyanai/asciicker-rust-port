---
phase: 06-physics-and-character
plan: 03
subsystem: rendering, game
tags: [water, reflection, perlin-noise, ripple, bevy-schedule, criterion, benchmark, game-plugin]

# Dependency graph
requires:
  - phase: 05-05
    provides: "Pipeline resolve, SampleBuffer, resolve_bridge, XTERM_256_PALETTE"
  - phase: 06-01
    provides: "PhysicsIO, PhysicsState, forces, collision, soup"
  - phase: 06-02
    provides: "Character, SpriteReq, equipment, CharacterSet, CharacterPlugin"
provides:
  - "Water reflection stage (Stage 5 REFLECTION) in render pipeline"
  - "Perlin noise ripple effect on reflected water cells"
  - "WaterConfig resource (water_z, ripple_time) in CpuRasterizerPlugin"
  - "GamePlugin with cross-plugin sync systems"
  - "WaterLevel resource (game-domain water surface height)"
  - "Physics criterion benchmarks (collision, forces, full frame)"
  - "PostUpdate render pipeline with RenderSet::Pipeline ordering"
affects: [07-game-systems, 07-01, 07-02]

# Tech tracking
tech-stack:
  added: [noise 0.9, criterion 0.5]
  patterns: [3-step-resolve-split, update-postupdate-chain-split, cross-plugin-system-set-ordering]

key-files:
  created:
    - engine-port/src/render/water.rs
    - engine-port/benches/physics_bench.rs
  modified:
    - engine-port/src/render/pipeline.rs
    - engine-port/src/render/mod.rs
    - engine-port/src/game/mod.rs
    - engine-port/src/main.rs
    - engine-port/Cargo.toml

key-decisions:
  - "WaterConfig owned by CpuRasterizerPlugin, WaterLevel owned by GamePlugin -- separate concerns"
  - "render_pipeline_system migrated from Update to PostUpdate with RenderSet::Pipeline label"
  - "3-step resolve split: resolve() -> apply_water_ripple_pass() -> RGBA conversion (preserves resolve_bridge.rs)"
  - "Intentional C++ bug replication in RGB cube decomposition (cb uses cr instead of cg) for visual fidelity"
  - "Linear torque model: yaw += torque * 45.0 * dt (deliberate simplification of C++ yaw velocity)"
  - "GamePlugin does NOT add sub-plugins -- main.rs registers all plugins independently"

patterns-established:
  - "3-step resolve split: palette-domain processing between resolve() and RGBA conversion"
  - "Update/PostUpdate chain split: camera+assembly in Update, render in PostUpdate"
  - "Cross-plugin SystemSet ordering: CharacterSet::SpritePush.before(RenderSet::Pipeline)"
  - "Game-domain resource sync pattern: WaterLevel -> PhysicsIO.water (PreUpdate) + WaterConfig.water_z (Update)"

requirements-completed: [FX-01, FX-02]

# Metrics
duration: 25min
completed: 2026-02-24
---

# Phase 6 Plan 3: Water Reflection + GamePlugin Summary

**Water reflection stage with Perlin noise ripple, GamePlugin cross-plugin sync, PostUpdate pipeline migration, and criterion physics benchmarks**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-24T15:55:47Z
- **Completed:** 2026-02-24T16:20:18Z
- **Tasks:** 2
- **Files modified:** 32 (including cargo fmt reformats)

## Accomplishments
- Implemented Stage 5 REFLECTION: flipped-Z view matrix re-renders terrain below water plane, marks reflected samples with spare_bits::REFLECTION
- Implemented Perlin noise ripple effect (Fbm<Perlin> 4-octave) applied in palette-index domain during 3-step resolve split
- Created GamePlugin with 6 cross-plugin sync systems (water, torque, camera follow, physics-to-character, mount collision)
- Migrated render_pipeline_system from Update to PostUpdate with RenderSet::Pipeline label for character sprite visibility
- Added 3 criterion benchmarks: collision_sweep (1.75us), forces_accumulation (5.2ns), full_physics_frame (128ns) -- all well under 2ms budget
- 337 lib tests passing, 3 ECS integration tests passing, 7 new game tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Water Reflection Stage + Perlin Ripple Effect** - `92cf03d` (feat)
2. **Task 2: GamePlugin Integration + Schedule Migration + Benchmarks** - `95aed6f` (feat)

**Plan metadata:** [pending] (docs: complete plan)

## Files Created/Modified

### Created
- `engine-port/src/render/water.rs` - Water reflection stage (flipped-Z re-render) and Perlin ripple effect with 6 unit tests
- `engine-port/benches/physics_bench.rs` - 3 criterion benchmarks (collision sweep, forces accumulation, full physics frame)

### Modified
- `engine-port/Cargo.toml` - Added noise 0.9, criterion 0.5 dev-dependency, bench harness config
- `engine-port/src/render/pipeline.rs` - WaterConfig param, Stage 5 REFLECTION call, 3-step resolve split, removed sprite_queue.clear()
- `engine-port/src/render/mod.rs` - WaterConfig resource, advance_water_time_system, PostUpdate migration, chain split
- `engine-port/src/game/mod.rs` - Full GamePlugin with WaterLevel, GameSet, 6 sync systems, 7 tests
- `engine-port/src/main.rs` - Plugin ordering documentation comment
- 25 additional files reformatted by cargo fmt

## Decisions Made

1. **WaterConfig vs WaterLevel ownership**: WaterConfig (render-domain) owned by CpuRasterizerPlugin. WaterLevel (game-domain) owned by GamePlugin. Synced via separate systems in different schedules.

2. **3-step resolve split** (R19-F02/F09): Instead of modifying resolve_bridge.rs (DO-NOT-MODIFY constraint), split resolve into: (a) resolve() fills AnsiCell with xterm-256 palette indices, (b) apply_water_ripple_pass() shifts palette indices for reflected cells, (c) glyph selection + RGBA conversion loop. Ripple operates in palette-index domain.

3. **C++ bug replication** (R19-F07): Intentionally replicate C++ RGB cube decomposition bug where blue channel uses red component instead of green. Visual fidelity with original engine takes priority.

4. **Linear torque model**: Simplified C++ yaw velocity system to `yaw += torque * 45.0 * dt`. Frame-rate independent and sufficient for ASCII visual fidelity.

5. **No sub-plugin registration**: GamePlugin wires cross-plugin sync only. PhysicsPlugin, CharacterPlugin, CpuRasterizerPlugin registered independently in main.rs to avoid Bevy duplicate-plugin panics.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fbm struct has private fields**
- **Found during:** Task 1 (water.rs Perlin noise)
- **Issue:** `Fbm::<Perlin> { octaves: 4, ..Default::default() }` fails because Fbm fields are private in noise 0.9
- **Fix:** Changed to `Fbm::<Perlin>::default().set_octaves(4)` using MultiFractal trait builder method
- **Files modified:** engine-port/src/render/water.rs (3 locations: 1 production + 2 tests)
- **Verification:** cargo test passes, cargo clippy clean
- **Committed in:** 92cf03d (Task 1 commit)

**2. [Rule 3 - Blocking] Clippy manual_range_contains lint**
- **Found during:** Task 1 (water.rs color range check)
- **Issue:** `c < 16 || c > 231` triggers clippy manual_range_contains warning with -D warnings
- **Fix:** Changed to `!(16..=231).contains(&c)`
- **Files modified:** engine-port/src/render/water.rs
- **Verification:** cargo clippy -- -D warnings passes clean
- **Committed in:** 92cf03d (Task 1 commit)

**3. [Rule 3 - Blocking] Bench harness referenced before file existed**
- **Found during:** Task 1 (Cargo.toml update)
- **Issue:** Adding `[[bench]] name = "physics_bench"` to Cargo.toml before creating the bench file caused cargo build to fail
- **Fix:** Created placeholder benches/physics_bench.rs with empty main and criterion stubs, then rewrote fully in Task 2
- **Files modified:** engine-port/benches/physics_bench.rs
- **Verification:** cargo build succeeds
- **Committed in:** 92cf03d (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking issues - Rule 3)
**Impact on plan:** All auto-fixes necessary for compilation. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Verification Results

- `cargo test --lib` -- 337 passed, 0 failed, 2 ignored
- `cargo test --test ecs_character_integration` -- 3 passed
- `cargo clippy -- -D warnings` -- clean
- `cargo fmt -- --check` -- clean
- `cargo bench --bench physics_bench` -- all 3 benchmarks pass
  - collision_sweep_1_entity_50_tris: ~1.75us
  - forces_accumulation: ~5.2ns
  - full_physics_frame: ~128ns (well under 2ms budget)

## Next Phase Readiness
- Phase 6 complete: physics core (06-01), character system (06-02), water+game integration (06-03) all done
- Ready for Phase 7 (Game Systems): GamePlugin provides the cross-plugin wiring foundation
- WaterLevel resource available for Phase 7 map loading to set per-map water height
- RenderSet::Pipeline and CharacterSet ordering established for future system additions

## Self-Check: PASSED

- [x] src/render/water.rs exists
- [x] benches/physics_bench.rs exists
- [x] src/game/mod.rs exists
- [x] 06-03-SUMMARY.md exists
- [x] Commit 92cf03d found (Task 1)
- [x] Commit 95aed6f found (Task 2)

---
*Phase: 06-physics-and-character*
*Completed: 2026-02-24*
