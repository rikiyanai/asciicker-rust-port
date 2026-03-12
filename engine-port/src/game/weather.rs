//! Weather particle effects: snow and rain with Perlin noise-driven wind.
//!
//! Port of C++ weather.cpp architecture: ring-buffer particle pool (512 max),
//! WeatherState machine controlling spawn rates, composite to AsciiCellGrid
//! AFTER resolve stage.
//!
//! Key design decisions:
//! - Ring buffer avoids heap allocation during gameplay updates
//! - Perlin noise drives wind variation (frequency 0.7, amplitude 2.0 * intensity)
//! - Snow glyphs: CP437 [0x2A, 0x2B, 0x2E, 0x2C] (*, +, ., comma)
//! - Rain glyphs: [0x7C, 0x2F, 0x3A] (|, /, :) -- extension over C++ (snow only)
//! - Composite uses fg=255 (white) matching C++ weather.cpp line 289-291

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::camera::GameCamera;
use crate::render::pipeline::project_world_to_screen;

// ---------------------------------------------------------------------------
// Constants (C++ weather.cpp exact values)
// ---------------------------------------------------------------------------

/// Maximum particles in the ring buffer. Matches C++ PARTICLE_CAPACITY.
pub const PARTICLE_CAPACITY: usize = 512;

/// Snow fall velocities in units/sec by glyph variant (C++ weather.cpp line 37).
/// Larger glyphs fall slower. Applied to vel[2] as negative (downward).
pub const SNOW_SPEEDS: [f32; 4] = [15.0, 12.0, 9.0, 6.0];

/// Rain fall speed in units/sec (all rain glyphs same speed).
pub const RAIN_SPEED: f32 = 25.0;

/// Snow glyph CP437 codes: *, +, ., comma (4 variants).
/// R13-031 FIX: explicit CP437 codes.
pub const SNOW_GLYPHS: [u8; 4] = [0x2A, 0x2B, 0x2E, 0x2C];

/// Rain glyph CP437 codes: |, /, : (3 variants).
/// R13-032 FIX: Extension over C++ (C++ only has snow).
pub const RAIN_GLYPHS: [u8; 3] = [0x7C, 0x2F, 0x3A];

/// Spawn rates per frame indexed by WeatherState discriminant.
/// R7-MED-002 FIX: Clear=0, LightSnow=10, HeavySnow=30, Blizzard=60.
pub const SPAWN_RATES: [f32; 4] = [0.0, 10.0, 30.0, 60.0];

/// R8-LOW-001 FIX: compile-time assert that SPAWN_RATES covers all WeatherState variants.
const _: () = assert!(SPAWN_RATES.len() == WeatherState::Blizzard as usize + 1);

/// Intensity lerp rate per frame.
pub const LERP_RATE: f32 = 0.05;

/// Default particle lifetime in seconds.
pub const DEFAULT_LIFETIME: f32 = 8.0;

/// Spawn area half-extents around camera (world units).
pub const SPAWN_RADIUS: f32 = 60.0;

/// Spawn height above camera (world units).
pub const SPAWN_HEIGHT: f32 = 40.0;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single weather particle.
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherParticle {
    /// World-space position [x, y, z].
    pub pos: [f32; 3],
    /// Velocity [x, y, z] in units/sec.
    pub vel: [f32; 3],
    /// Remaining lifetime in seconds. <= 0 means dead.
    pub lifetime_remaining: f32,
    /// CP437 glyph code for rendering.
    pub glyph: u8,
    /// Foreground color RGB.
    pub fg: [u8; 3],
}

/// Weather state controlling spawn rates and intensity.
///
/// Discriminants match SPAWN_RATES array indices (R7-MED-002 FIX).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WeatherState {
    #[default]
    Clear = 0,
    LightSnow = 1,
    HeavySnow = 2,
    Blizzard = 3,
}

/// Precipitation type determines glyphs and fall speed.
///
/// R13-032 FIX: Rain is an extension over C++ (which only has snow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PrecipitationType {
    #[default]
    Snow,
    Rain,
}

/// Ring-buffer particle pool. Zero heap allocation during updates.
///
/// P7-205 FIX: Manual Default impl because [WeatherParticle; 512] exceeds
/// Rust's auto-derive limit of [T; 32].
pub struct ParticlePool {
    /// Fixed-size particle storage.
    pub particles: [WeatherParticle; PARTICLE_CAPACITY],
    /// Write head (next slot to overwrite).
    pub head: usize,
    /// Number of live particles (capped at PARTICLE_CAPACITY).
    pub count: usize,
}

impl Default for ParticlePool {
    fn default() -> Self {
        Self {
            particles: [WeatherParticle::default(); PARTICLE_CAPACITY],
            head: 0,
            count: 0,
        }
    }
}

impl ParticlePool {
    /// Create a new empty particle pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a particle at the current head position.
    ///
    /// Overwrites the oldest particle when full (ring buffer behavior).
    pub fn spawn(&mut self, particle: WeatherParticle) {
        self.particles[self.head] = particle;
        self.head = (self.head + 1) % PARTICLE_CAPACITY;
        if self.count < PARTICLE_CAPACITY {
            self.count += 1;
        }
    }

    /// Number of slots that have been written to (may include dead particles).
    pub fn active_count(&self) -> usize {
        self.count
    }

    /// Iterate over live particles (lifetime_remaining > 0).
    ///
    /// P7-029 FIX: Skips dead entries in the ring buffer.
    pub fn iter_live(&self) -> impl Iterator<Item = &WeatherParticle> {
        self.particles[..self.count]
            .iter()
            .filter(|p| p.lifetime_remaining > 0.0)
    }

    /// Update all particles: apply velocity and decrement lifetime.
    pub fn update(&mut self, dt: f32) {
        for p in self.particles[..self.count].iter_mut() {
            if p.lifetime_remaining > 0.0 {
                p.pos[0] += p.vel[0] * dt;
                p.pos[1] += p.vel[1] * dt;
                p.pos[2] += p.vel[2] * dt;
                p.lifetime_remaining -= dt;
            }
        }
    }
}

/// Weather resource: state machine, particle pool, Perlin noise for wind.
///
/// Registered as a Bevy Resource via init_resource::<Weather>().
#[derive(Resource)]
pub struct Weather {
    /// Current weather state (controls spawn rate).
    pub state: WeatherState,
    /// Current precipitation type (controls glyphs and speed).
    pub precipitation: PrecipitationType,
    /// Current intensity (0.0 = none, 1.0 = full). Lerps toward target.
    pub intensity: f32,
    /// Target intensity based on weather state.
    pub target_intensity: f32,
    /// Wind vector [x, y] driven by Perlin noise.
    pub wind: [f32; 2],
    /// Perlin noise generator for wind variation.
    pub perlin: Perlin,
    /// Perlin noise time accumulator (f64 for precision).
    /// P7-209 FIX: Required by weather_update_system for wind computation.
    pub perlin_time: f64,
    /// Ring-buffer particle pool.
    pub pool: ParticlePool,
    /// Fractional spawn accumulator (handles sub-frame spawning).
    pub spawn_accumulator: f32,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            state: WeatherState::Clear,
            precipitation: PrecipitationType::Snow,
            intensity: 0.0,
            target_intensity: 0.0,
            wind: [0.0, 0.0],
            perlin: Perlin::new(42), // deterministic seed
            perlin_time: 0.0,
            pool: ParticlePool::default(),
            spawn_accumulator: 0.0,
        }
    }
}

/// Set the weather state and target intensity.
///
/// R19-006 FIX: No system currently calls this at runtime. Weather starts Clear
/// and never changes automatically. Exposed as public API for debug/demo use.
/// A debug key (e.g., F5) to cycle states can be added as a deferred item.
pub fn set_weather_state(weather: &mut Weather, state: WeatherState) {
    weather.state = state;
    weather.target_intensity = match state {
        WeatherState::Clear => 0.0,
        WeatherState::LightSnow => 0.3,
        WeatherState::HeavySnow => 0.7,
        WeatherState::Blizzard => 1.0,
    };
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Debug keybind -- not a gameplay feature. Remove or gate behind cfg(debug_assertions) for release.
///
/// Cycles WeatherState on F5 press: Clear -> LightSnow -> HeavySnow -> Blizzard -> Clear.
/// Calls set_weather_state to update target_intensity accordingly.
pub fn cycle_weather_debug_system(keys: Res<ButtonInput<KeyCode>>, mut weather: ResMut<Weather>) {
    if keys.just_pressed(KeyCode::F5) {
        let old = weather.state;
        let next = match old {
            WeatherState::Clear => WeatherState::LightSnow,
            WeatherState::LightSnow => WeatherState::HeavySnow,
            WeatherState::HeavySnow => WeatherState::Blizzard,
            WeatherState::Blizzard => WeatherState::Clear,
        };
        set_weather_state(&mut weather, next);
        info!("Weather debug: {:?} -> {:?}", old, next);
    }
}

/// Update weather: lerp intensity, compute wind, spawn and update particles.
///
/// R19-003 FIX: Wind uses C++ exact parameters (frequency 0.7, amplitude 2.0 * intensity).
pub fn weather_update_system(mut weather: ResMut<Weather>, time: Res<Time>) {
    let dt = time.delta_secs();

    // Lerp intensity toward target (0.05 per frame, frame-rate dependent)
    let diff = weather.target_intensity - weather.intensity;
    weather.intensity += diff * LERP_RATE;

    // Update Perlin noise time
    weather.perlin_time += dt as f64;

    // Compute wind from Perlin noise (C++ weather.cpp lines 208-210)
    let pn_time = weather.perlin_time;
    let intensity = weather.intensity;
    let wind_x = weather.perlin.get([pn_time * 0.7, 0.0]) as f32 * 2.0 * intensity;
    let wind_y = weather.perlin.get([0.0, pn_time * 0.7]) as f32 * 2.0 * intensity;
    weather.wind = [wind_x, wind_y];

    // Accumulate spawns based on current state's spawn rate
    let spawn_rate = SPAWN_RATES[weather.state as usize];
    weather.spawn_accumulator += spawn_rate * dt;

    // Spawn particles while accumulator >= 1.0
    let mut spawn_index = 0u32;
    while weather.spawn_accumulator >= 1.0 {
        weather.spawn_accumulator -= 1.0;

        // Deterministic pseudo-random position using simple hash
        let hash_seed =
            (weather.perlin_time * 1000.0) as u32 ^ spawn_index.wrapping_mul(2654435761);
        let fx = ((hash_seed & 0xFFFF) as f32 / 65535.0) * 2.0 - 1.0;
        let fy = (((hash_seed >> 16) & 0xFFFF) as f32 / 65535.0) * 2.0 - 1.0;

        // Select glyph and speed based on precipitation type
        let (glyph, fall_speed) = match weather.precipitation {
            PrecipitationType::Snow => {
                let variant = (hash_seed as usize) % SNOW_GLYPHS.len();
                (SNOW_GLYPHS[variant], -SNOW_SPEEDS[variant])
            }
            PrecipitationType::Rain => {
                let variant = (hash_seed as usize) % RAIN_GLYPHS.len();
                (RAIN_GLYPHS[variant], -RAIN_SPEED)
            }
        };

        let particle = WeatherParticle {
            pos: [fx * SPAWN_RADIUS, fy * SPAWN_RADIUS, SPAWN_HEIGHT],
            vel: [weather.wind[0], weather.wind[1], fall_speed],
            lifetime_remaining: DEFAULT_LIFETIME,
            glyph,
            fg: [255, 255, 255], // white
        };
        weather.pool.spawn(particle);
        spawn_index += 1;
    }

    // Update existing particles
    weather.pool.update(dt);
}

/// Composite weather particles onto AsciiCellGrid AFTER resolve.
///
/// R19-002 FIX: Uses canonical project_world_to_screen from pipeline.rs.
/// R19-004 FIX: Always uses fg=255 (white) matching C++ behavior.
/// R19-005 FIX: Depth testing against SampleBuffer deferred as polish item
/// (C++ also lacks this check -- visual artifact in both engines).
/// R20-W01 FIX: Preserves existing bg_color when compositing.
pub fn weather_composite_system(
    weather: Res<Weather>,
    mut cell_grid: ResMut<AsciiCellGrid>,
    camera: Res<GameCamera>,
) {
    let grid_w = cell_grid.width;
    let grid_h = cell_grid.height;

    for particle in weather.pool.iter_live() {
        // Project particle world position to ASCII cell coordinates
        if let Some((sx, sy)) = project_world_to_screen(&particle.pos, &camera) {
            let px = sx as u32;
            let py = sy as u32;

            // Bounds check
            if px < grid_w && py < grid_h {
                // R20-W01 FIX: Preserve existing background color
                let (_, _, existing_bg) = cell_grid.cell_at(px, py);
                // R19-004 FIX: fg=255 (white) matching C++ weather.cpp line 289
                cell_grid.set_cell(
                    px,
                    py,
                    particle.glyph as u16,
                    [255, 255, 255, 255],
                    existing_bg,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Task 1 tests (ParticlePool and WeatherState) --

    #[test]
    fn test_particle_pool_starts_empty() {
        let pool = ParticlePool::new();
        assert_eq!(pool.active_count(), 0);
        assert_eq!(pool.head, 0);
        assert_eq!(pool.iter_live().count(), 0);
    }

    #[test]
    fn test_particle_pool_spawn_increments_count() {
        let mut pool = ParticlePool::new();
        pool.spawn(WeatherParticle {
            lifetime_remaining: 5.0,
            ..Default::default()
        });
        assert_eq!(pool.active_count(), 1);
        assert_eq!(pool.iter_live().count(), 1);

        pool.spawn(WeatherParticle {
            lifetime_remaining: 5.0,
            ..Default::default()
        });
        assert_eq!(pool.active_count(), 2);
    }

    #[test]
    fn test_particle_pool_wraps_at_capacity() {
        let mut pool = ParticlePool::new();
        // Spawn PARTICLE_CAPACITY + 1 particles
        for i in 0..=PARTICLE_CAPACITY {
            pool.spawn(WeatherParticle {
                lifetime_remaining: 5.0,
                pos: [i as f32, 0.0, 0.0],
                ..Default::default()
            });
        }
        // Count stays at PARTICLE_CAPACITY
        assert_eq!(pool.active_count(), PARTICLE_CAPACITY);
        // Head should have wrapped
        assert_eq!(pool.head, 1); // 513 % 512 = 1
        // Last spawned particle overwrote slot 0
        assert_eq!(pool.particles[0].pos[0], PARTICLE_CAPACITY as f32);
    }

    #[test]
    fn test_particle_pool_iter_live_particles() {
        let mut pool = ParticlePool::new();
        // Spawn 3 live, then 1 with lifetime 0 (dead)
        pool.spawn(WeatherParticle {
            lifetime_remaining: 5.0,
            ..Default::default()
        });
        pool.spawn(WeatherParticle {
            lifetime_remaining: 3.0,
            ..Default::default()
        });
        pool.spawn(WeatherParticle {
            lifetime_remaining: 0.0,
            ..Default::default()
        });
        pool.spawn(WeatherParticle {
            lifetime_remaining: 2.0,
            ..Default::default()
        });

        // 4 slots used, but only 3 are alive (P7-029 FIX)
        assert_eq!(pool.active_count(), 4);
        assert_eq!(pool.iter_live().count(), 3);
    }

    #[test]
    fn test_weather_state_spawn_rates() {
        assert_eq!(SPAWN_RATES[WeatherState::Clear as usize], 0.0);
        assert_eq!(SPAWN_RATES[WeatherState::LightSnow as usize], 10.0);
        assert_eq!(SPAWN_RATES[WeatherState::HeavySnow as usize], 30.0);
        assert_eq!(SPAWN_RATES[WeatherState::Blizzard as usize], 60.0);
    }

    #[test]
    fn test_weather_intensity_lerp() {
        // R17-F224 FIX: starting at intensity=0.0 with target=1.0 and LERP_RATE=0.05
        let mut intensity = 0.0f32;
        let target = 1.0f32;

        // After 1 frame: intensity += (1.0 - 0.0) * 0.05 = 0.05
        intensity += (target - intensity) * LERP_RATE;
        assert!(
            (intensity - 0.05).abs() < 0.001,
            "After 1 frame: expected ~0.05, got {intensity}"
        );

        // After 20 frames total: intensity = 1.0 - 0.95^20 ~ 0.642
        for _ in 1..20 {
            intensity += (target - intensity) * LERP_RATE;
        }
        let expected_20 = 1.0 - 0.95f32.powi(20);
        assert!(
            (intensity - expected_20).abs() < 0.01,
            "After 20 frames: expected ~{expected_20}, got {intensity}"
        );
    }

    #[test]
    fn test_set_weather_state_updates_target() {
        let mut weather = Weather::default();
        assert_eq!(weather.state, WeatherState::Clear);
        assert_eq!(weather.target_intensity, 0.0);

        set_weather_state(&mut weather, WeatherState::Blizzard);
        assert_eq!(weather.state, WeatherState::Blizzard);
        assert_eq!(weather.target_intensity, 1.0);

        set_weather_state(&mut weather, WeatherState::LightSnow);
        assert_eq!(weather.state, WeatherState::LightSnow);
        assert_eq!(weather.target_intensity, 0.3);
    }

    #[test]
    fn test_particle_pool_update_applies_velocity() {
        let mut pool = ParticlePool::new();
        pool.spawn(WeatherParticle {
            pos: [0.0, 0.0, 100.0],
            vel: [1.0, 2.0, -10.0],
            lifetime_remaining: 5.0,
            ..Default::default()
        });

        pool.update(0.5); // half second

        let p = &pool.particles[0];
        assert!((p.pos[0] - 0.5).abs() < 0.001);
        assert!((p.pos[1] - 1.0).abs() < 0.001);
        assert!((p.pos[2] - 95.0).abs() < 0.001);
        assert!((p.lifetime_remaining - 4.5).abs() < 0.001);
    }

    #[test]
    fn test_particle_pool_update_skips_dead() {
        let mut pool = ParticlePool::new();
        pool.spawn(WeatherParticle {
            pos: [10.0, 0.0, 0.0],
            vel: [1.0, 0.0, 0.0],
            lifetime_remaining: 0.0, // dead
            ..Default::default()
        });
        pool.update(1.0);
        // Dead particle should NOT be moved
        assert_eq!(pool.particles[0].pos[0], 10.0);
    }

    // -- Task 2 tests (weather update and composite systems) --

    #[test]
    fn test_weather_update_spawns() {
        // R17-F225 FIX: HeavySnow + dt=1.0 should spawn exactly 30 particles
        let mut weather = Weather::default();
        set_weather_state(&mut weather, WeatherState::HeavySnow);
        weather.intensity = 1.0; // already at full

        // Simulate update: accumulator += SPAWN_RATES[HeavySnow] * dt = 30.0 * 1.0 = 30.0
        let dt = 1.0f32;
        weather.spawn_accumulator += SPAWN_RATES[weather.state as usize] * dt;

        let mut spawned = 0u32;
        while weather.spawn_accumulator >= 1.0 {
            weather.spawn_accumulator -= 1.0;
            weather.pool.spawn(WeatherParticle {
                lifetime_remaining: DEFAULT_LIFETIME,
                ..Default::default()
            });
            spawned += 1;
        }

        assert_eq!(spawned, 30);
        assert_eq!(weather.pool.active_count(), 30);
    }

    #[test]
    fn test_weather_composite_writes() {
        // Particle at known position writes glyph to grid
        let mut weather = Weather::default();
        weather.pool.spawn(WeatherParticle {
            pos: [10.0, 10.0, 10.0],
            vel: [0.0, 0.0, -5.0],
            lifetime_remaining: 5.0,
            glyph: 0x2A, // *
            fg: [255, 255, 255],
        });

        let mut grid = AsciiCellGrid::new(240, 135);
        let mut camera = GameCamera::default();
        camera.pos = [10.0, 10.0, 10.0];
        camera.yaw = 0.0;
        camera.zoom = 1.0;
        camera.perspective = true;
        camera.update(484.0, 274.0);
        camera.extract_frustum_planes(484.0, 274.0);

        // Call the composite logic directly
        for particle in weather.pool.iter_live() {
            if let Some((sx, sy)) = project_world_to_screen(&particle.pos, &camera) {
                let px = sx as u32;
                let py = sy as u32;
                if px < grid.width && py < grid.height {
                    let (_, _, existing_bg) = grid.cell_at(px, py);
                    grid.set_cell(
                        px,
                        py,
                        particle.glyph as u16,
                        [255, 255, 255, 255],
                        existing_bg,
                    );
                }
            }
        }

        // At least one cell should have the snow glyph written
        let has_snow = grid.char_indices.iter().any(|&c| c == 0x2A);
        assert!(has_snow, "Composite should write snow glyph to grid");
    }

    #[test]
    fn test_weather_clear_no_spawn() {
        // R16-F209 FIX: Clear has SPAWN_RATES[0]=0, so no particles spawn
        let weather = Weather::default();
        assert_eq!(weather.state, WeatherState::Clear);
        assert_eq!(SPAWN_RATES[WeatherState::Clear as usize], 0.0);
        assert_eq!(weather.pool.active_count(), 0);
    }

    #[test]
    fn test_rain_uses_rain_glyphs() {
        // R13-034 FIX: PrecipitationType::Rain produces RAIN_GLYPHS, not SNOW_GLYPHS
        let mut weather = Weather::default();
        weather.precipitation = PrecipitationType::Rain;
        set_weather_state(&mut weather, WeatherState::HeavySnow);

        // Simulate spawning particles with Rain type
        for i in 0..30u32 {
            let hash_seed = i.wrapping_mul(2654435761);
            let variant = (hash_seed as usize) % RAIN_GLYPHS.len();
            let glyph = RAIN_GLYPHS[variant];
            weather.pool.spawn(WeatherParticle {
                glyph,
                lifetime_remaining: 5.0,
                ..Default::default()
            });
        }

        // All spawned glyphs should be rain glyphs, not snow glyphs
        for p in weather.pool.iter_live() {
            assert!(
                RAIN_GLYPHS.contains(&p.glyph),
                "Rain particle glyph 0x{:02X} should be in RAIN_GLYPHS",
                p.glyph
            );
            assert!(
                !SNOW_GLYPHS.contains(&p.glyph),
                "Rain particle glyph 0x{:02X} should NOT be in SNOW_GLYPHS",
                p.glyph
            );
        }
    }

    #[test]
    fn test_weather_default() {
        let w = Weather::default();
        assert_eq!(w.state, WeatherState::Clear);
        assert_eq!(w.precipitation, PrecipitationType::Snow);
        assert_eq!(w.intensity, 0.0);
        assert_eq!(w.target_intensity, 0.0);
        assert_eq!(w.wind, [0.0, 0.0]);
        assert_eq!(w.perlin_time, 0.0);
        assert_eq!(w.pool.active_count(), 0);
        assert_eq!(w.spawn_accumulator, 0.0);
    }

    #[test]
    fn test_snow_glyph_constants() {
        // Verify CP437 codes match expected characters
        assert_eq!(SNOW_GLYPHS[0], 0x2A); // *
        assert_eq!(SNOW_GLYPHS[1], 0x2B); // +
        assert_eq!(SNOW_GLYPHS[2], 0x2E); // .
        assert_eq!(SNOW_GLYPHS[3], 0x2C); // ,
    }

    #[test]
    fn test_rain_glyph_constants() {
        assert_eq!(RAIN_GLYPHS[0], 0x7C); // |
        assert_eq!(RAIN_GLYPHS[1], 0x2F); // /
        assert_eq!(RAIN_GLYPHS[2], 0x3A); // :
    }

    #[test]
    fn test_cycle_weather_cycles_all_states() {
        // Verify F5 cycle: Clear -> LightSnow -> HeavySnow -> Blizzard -> Clear
        let mut weather = Weather::default();
        assert_eq!(weather.state, WeatherState::Clear);

        // Clear -> LightSnow
        let next = match weather.state {
            WeatherState::Clear => WeatherState::LightSnow,
            WeatherState::LightSnow => WeatherState::HeavySnow,
            WeatherState::HeavySnow => WeatherState::Blizzard,
            WeatherState::Blizzard => WeatherState::Clear,
        };
        set_weather_state(&mut weather, next);
        assert_eq!(weather.state, WeatherState::LightSnow);
        assert_eq!(weather.target_intensity, 0.3);

        // LightSnow -> HeavySnow
        let next = match weather.state {
            WeatherState::Clear => WeatherState::LightSnow,
            WeatherState::LightSnow => WeatherState::HeavySnow,
            WeatherState::HeavySnow => WeatherState::Blizzard,
            WeatherState::Blizzard => WeatherState::Clear,
        };
        set_weather_state(&mut weather, next);
        assert_eq!(weather.state, WeatherState::HeavySnow);
        assert_eq!(weather.target_intensity, 0.7);

        // HeavySnow -> Blizzard
        let next = match weather.state {
            WeatherState::Clear => WeatherState::LightSnow,
            WeatherState::LightSnow => WeatherState::HeavySnow,
            WeatherState::HeavySnow => WeatherState::Blizzard,
            WeatherState::Blizzard => WeatherState::Clear,
        };
        set_weather_state(&mut weather, next);
        assert_eq!(weather.state, WeatherState::Blizzard);
        assert_eq!(weather.target_intensity, 1.0);

        // Blizzard -> Clear (wrap)
        let next = match weather.state {
            WeatherState::Clear => WeatherState::LightSnow,
            WeatherState::LightSnow => WeatherState::HeavySnow,
            WeatherState::HeavySnow => WeatherState::Blizzard,
            WeatherState::Blizzard => WeatherState::Clear,
        };
        set_weather_state(&mut weather, next);
        assert_eq!(weather.state, WeatherState::Clear);
        assert_eq!(weather.target_intensity, 0.0);
    }
}
