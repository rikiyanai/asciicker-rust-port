---
phase: 07-game-systems
plan: 05
subsystem: weather
tags: [weather, particles, ring-buffer, perlin-noise, composite]
dependency_graph:
  requires: [game-state, render-pipeline, ascii-cell-grid]
  provides: [weather-particles, weather-state-machine, weather-composite]
  affects: [ascii-cell-grid, game-plugin]
tech_stack:
  added: [noise-crate-perlin]
  patterns: [ring-buffer-pool, perlin-wind, post-resolve-composite]
key_files:
  created:
    - engine-port/src/game/weather.rs
  modified:
    - engine-port/src/game/mod.rs
decisions:
  - "D07-05-01: Depth testing against SampleBuffer deferred as polish (C++ also lacks it)"
  - "D07-05-02: fg=255 (white) for all weather particles matching C++ behavior"
  - "D07-05-03: Rain glyphs are extension over C++ (which only has snow)"
  - "D07-05-04: No automatic weather state trigger at runtime; exposed as public API"
metrics:
  duration: 14min
  completed: 2026-02-26
  tasks: 2
  tests_added: 16
  tests_total: 446
  files_created: 1
  files_modified: 1
requirements:
  - GAME-03
---

# Phase 7 Plan 05: Weather Particle Effects Summary

Ring-buffer particle pool (512 max) with snow/rain precipitation, Perlin noise-driven wind (C++ exact: freq=0.7, amp=2.0*intensity), composite to AsciiCellGrid after render pipeline resolve.

## Commits

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | ParticlePool ring buffer + WeatherState + systems | `905920e` | `engine-port/src/game/weather.rs` |
| 2 | Register weather systems in GamePlugin | `666c420` | `engine-port/src/game/mod.rs` |

## Implementation Details

### Task 1: Weather particle system (weather.rs)

Created `engine-port/src/game/weather.rs` (633 lines) containing:

**Data structures:**
- `WeatherParticle`: pos, vel, lifetime_remaining, glyph, fg color
- `WeatherState` enum: Clear(0)/LightSnow(1)/HeavySnow(2)/Blizzard(3) with spawn rates [0, 10, 30, 60]
- `PrecipitationType` enum: Snow/Rain (Rain is extension over C++)
- `ParticlePool`: fixed-size ring buffer of 512 particles with manual Default impl (P7-205 FIX: arrays >32 elements cannot auto-derive Default)
- `Weather` resource: state machine, Perlin noise, pool, spawn accumulator, perlin_time (f64)

**Constants (C++ exact values):**
- SNOW_SPEEDS: [15.0, 12.0, 9.0, 6.0] (units/sec by glyph variant)
- RAIN_SPEED: 25.0
- SNOW_GLYPHS: [0x2A, 0x2B, 0x2E, 0x2C] (*, +, ., comma)
- RAIN_GLYPHS: [0x7C, 0x2F, 0x3A] (|, /, :)
- Compile-time assert: SPAWN_RATES.len() == Blizzard + 1

**Systems:**
- `weather_update_system`: lerp intensity, Perlin wind, accumulate+spawn particles, update velocities
- `weather_composite_system`: project particles to screen via canonical `project_world_to_screen`, write to AsciiCellGrid with fg=255 (white), preserve existing bg_color (R20-W01 FIX)
- `set_weather_state`: public API for state transitions (no automatic trigger)

**Tests (16 total):**
- test_particle_pool_starts_empty
- test_particle_pool_spawn_increments_count
- test_particle_pool_wraps_at_capacity (512+1 spawns)
- test_particle_pool_iter_live_particles (P7-029: dead filtering)
- test_particle_pool_update_applies_velocity
- test_particle_pool_update_skips_dead
- test_weather_state_spawn_rates
- test_weather_intensity_lerp (R17-F224: after 20 frames = 1.0-0.95^20)
- test_set_weather_state_updates_target
- test_weather_default
- test_snow_glyph_constants
- test_rain_glyph_constants
- test_weather_update_spawns (R17-F225: HeavySnow+dt=1.0 = 30 particles)
- test_weather_composite_writes
- test_weather_clear_no_spawn
- test_rain_uses_rain_glyphs (R13-034)

### Task 2: GamePlugin registration (mod.rs)

- Added `pub mod weather;` to game/mod.rs
- Added `use crate::render::pipeline::render_pipeline_system;` for ordering
- `init_resource::<Weather>()` in GamePlugin::build
- `weather_update_system` in Update, gated on `in_state(GameState::Playing)`
- `weather_composite_system` in PostUpdate, `.after(render_pipeline_system)`, gated on Playing
- AUTHORITATIVE SCHEDULE DECISION confirmed: render_pipeline_system is in PostUpdate (Phase 6 migration)

## Verification Results

- `cargo build`: PASS (zero warnings)
- `cargo test --lib`: 446 passed, 0 failed, 2 ignored
- `cargo clippy -- -D warnings`: 0 errors from weather.rs (455 pre-existing in other files)
- Schedule verification: both render_pipeline_system and weather_composite_system in PostUpdate
- Ring buffer wraps correctly at 512
- Dead particle filtering works (P7-029)
- Noise crate compiles (already in Cargo.toml from Phase 7 Plan 04)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed integer overflow in test_rain_uses_rain_glyphs**
- **Found during:** Task 1 test execution
- **Issue:** `i * 2654435761u32` overflowed in debug mode for `i >= 2`
- **Fix:** Changed to `i.wrapping_mul(2654435761)`
- **Files modified:** engine-port/src/game/weather.rs
- **Commit:** 905920e

### Deferred Items

- **R19-005 (Depth testing):** Weather particles do not test against SampleBuffer depth. Snow/rain can appear inside caves or behind walls. C++ has the same visual artifact. Deferred as polish item.
- **R19-006 (Weather state trigger):** No system calls `set_weather_state` at runtime. Weather starts Clear and never changes. Debug key (F5 to cycle states) deferred to Phase 8.
- **Pre-existing clippy errors:** 455 clippy errors in other files (spatial_grid, network, font, etc.) are pre-existing and out of scope for this plan.

## Success Criteria Check

- [x] Ring-buffer 512 particles with correct wrap
- [x] Weather states control spawn rates
- [x] Perlin noise drives wind
- [x] Snow and rain particle types work
- [x] Particles composite after resolve (PostUpdate, after render_pipeline_system)
- [x] Zero heap allocation in update/spawn (fixed-size array)
- [x] weather_composite_system and render_pipeline_system both in PostUpdate

## Self-Check: PASSED

- [x] engine-port/src/game/weather.rs: FOUND
- [x] Commit 905920e: FOUND
- [x] Commit 666c420: FOUND
- [x] 446 tests passing (16 new weather tests)
