# Phase 7: Game Systems - Research

**Researched:** 2026-02-20
**Domain:** Audio, networking, weather particles, game state machine, visual quality (shape-vector glyph matching)
**Confidence:** MEDIUM (multiple subsystems, some version compatibility unverified at runtime)

## Summary

Phase 7 is a composite "final mile" phase covering five distinct subsystems: audio (bevy_kira_audio 16-track mixer), multiplayer networking (client-server with entity replication), weather particle effects (ASCII snow/rain), game state machine (Loading/Playing/Paused with main menu), and visual quality upgrades (Alex Harri 6D shape-vector glyph matching + 3 font skins). Each subsystem is relatively independent, making this phase parallelizable internally.

The audio subsystem is the most straightforward: bevy_kira_audio 0.25 provides Bevy 0.18 compatibility with typed audio channels and dynamic channel support -- mapping cleanly to the C++ engine's 16-track `PlyTrack` mixer. Networking is the highest-risk subsystem: bevy_replicon 0.38 (with bevy_renet transport) or lightyear 0.26 both support Bevy 0.18, but the C++ engine's custom WebSocket binary protocol needs adaptation. The Alex Harri shape-vector integration is the most algorithmically complex subsystem, requiring a k-d tree (kiddo 5.2 crate) and quantized cache at the RESOLVE stage. Weather and game state machine are moderate-complexity ECS pattern work.

**Primary recommendation:** Use bevy_kira_audio 0.25 for audio, bevy_replicon 0.38 + bevy_renet for networking (simpler than lightyear for the C++ protocol's scope), kiddo 5.2 for k-d tree, noise-rs for Perlin wind, and Bevy's native `States` + `SubStates` for game state machine.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| AUD-01 | bevy_kira_audio integration with basic sound effect playback | bevy_kira_audio 0.25 confirmed compatible with Bevy 0.18; AudioPlugin + AudioChannel API documented; Ogg Vorbis supported via Kira backend |
| AUD-02 | 16-track audio mixer matching C++ engine architecture | DynamicAudioChannels resource enables named runtime channels; alternatively 16 typed AudioChannel<TrackN> resources; C++ PlyTrack struct maps to per-channel state |
| NET-01 | Basic client-server multiplayer (entity replication, position sync) | bevy_replicon 0.38 provides server-authoritative replication for Bevy 0.18; bevy_renet transport for TCP/UDP; C++ protocol has 7 message types (Join/Exit/Pose/Talk/Lag) |
| NET-02 | Binary protocol compatible with or inspired by C++ WebSocket protocol | C++ uses token-based packed binary structs; Rust repr(C, packed) + serde or manual serialization; WebSocket framing via tungstenite if browser compatibility needed |
| GAME-01 | Game state machine (Loading -> Playing -> Paused) | Bevy States derive macro with OnEnter/OnExit/OnTransition schedules; SubStates for nested state (e.g. InGame -> Paused); state_changed run condition |
| GAME-02 | Main menu with basic navigation | Stack-based menu from C++ MainMenu struct maps to Bevy UI with states; data-driven menu definition pattern; 3 font skins for item rendering |
| GAME-03 | Weather effects (rain, snow particle systems) | C++ weather.cpp has ring-buffer particle pool (512 particles), 4 states (CLEAR/LIGHT/HEAVY/BLIZZARD), Perlin wind; port as ECS particle system with noise-rs |
| VIS-01 | Alex Harri 6D shape-vector glyph matching at RESOLVE stage | 6D sampling vectors from SampleBuffer; kiddo 5.2 k-d tree for nearest-neighbor; quantized cache (30-bit key); replaces auto_mat glyph selection only (colors kept) |
| VIS-03 | Font system with CP437 glyphs (3 skins: grey, gold, pink) | C++ font1.cpp loads font-1.xp with recolor tables; 5x5 pixel glyphs, 4x13 atlas; 3 skins via color remapping; port as Bevy resource with BlitSprite-equivalent |
</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy_kira_audio | 0.25 | Audio playback, channels, mixer | Official Bevy audio plugin; 0.25 = Bevy 0.18 compatible; Kira backend handles Ogg/MP3/WAV/FLAC |
| bevy_replicon | 0.38 | Server-authoritative entity replication | Mature Bevy networking abstraction; transport-agnostic; supports singleplayer + dedicated server modes |
| bevy_renet | (transitive via bevy_replicon_renet2 = "0.13") | Network transport for bevy_replicon | TCP/UDP transport; bevy_replicon_renet2 integrates the two; simpler than lightyear for basic replication |
| kiddo | 5.2 | k-d tree nearest-neighbor search | Fastest Rust k-d tree; const-generic dimensions (supports 6D); f32 support; `nearest_one` avoids heap alloc |
| noise | 0.9+ | Perlin noise for weather wind simulation | Standard Rust noise library; 2D/3D Perlin + Simplex; used for wind vector variation like C++ siv::PerlinNoise |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| bevy_replicon_renet2 | 0.13 | Transport bridge between bevy_replicon and renet2 | Required to connect bevy_replicon's replication layer to actual network I/O. **P7-213 FIX (LOW):** Old name `bevy_replicon_renet` (without "2") does NOT support Bevy 0.18 — per MEMORY.md and P7-109 FIX in 07-03, the correct crate is `bevy_replicon_renet2 = "0.13"`. |
| serde + bincode | latest | Binary serialization for network protocol | If adapting C++ binary protocol to Rust; bincode for compact binary encoding |
| tungstenite | latest | WebSocket framing (optional) | Only if browser client compatibility is required for NET-02 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| bevy_replicon + bevy_renet | lightyear 0.26 | Lightyear has client-side prediction, rollback, WebTransport/WASM support -- more features but heavier; overkill for basic position sync |
| bevy_kira_audio | bevy_seedling | Newer alternative (Benchmark Score 91.5 vs 58.7); less battle-tested but potentially better API; evaluate if bevy_kira_audio proves problematic |
| kiddo | Hand-rolled k-d tree | Alex Harri's TypeScript KdTree is ~100 lines; kiddo is production-grade with SIMD opts; use kiddo unless API mismatch |
| noise crate | simdnoise | SIMD-accelerated noise; faster for bulk generation; overkill for weather wind (only 2 noise samples per frame) |

**Installation:**
```toml
[dependencies]
bevy_kira_audio = "0.25"
bevy_replicon = "0.38"
bevy_replicon_renet2 = "0.13"  # P7-109: correct crate name (not bevy_replicon_renet); version per MEMORY.md
# **R6-H01 FIX:** Replaced wildcard `version = "*"` (prohibited by crates.io) and wrong crate
# name `bevy_replicon_renet` with correct `bevy_replicon_renet2 = "0.13"` per MEMORY.md and P7-109.
kiddo = "5.2"
noise = "0.9"
serde = { version = "1", features = ["derive"] }
bincode = "1"
```

## Architecture Patterns

### Recommended Project Structure
```
src/
  audio/
    mod.rs              # AudioPlugin: bevy_kira_audio setup, 16 dynamic channels
    mixer.rs            # TrackState resource, play/stop/volume commands
    samples.rs          # AUDIO_FILE enum, sample loading from assets/samples/
  network/
    mod.rs              # NetworkPlugin: bevy_replicon + renet setup
    protocol.rs         # Message types (Join/Exit/Pose/Talk/Lag) with serde
    server.rs           # Server systems: accept connections, broadcast state
    client.rs           # Client systems: send pose, receive broadcasts
  game/
    mod.rs              # GamePlugin: state machine, menu, weather orchestration
    state.rs            # GameState enum (Loading, Playing, Paused), transitions
    menu.rs             # MainMenu resource, menu item definitions, navigation
    weather.rs          # WeatherPlugin: particle pool, accumulation, wind
  render/
    shape_vector.rs     # Alex Harri 6D sampling, quantized cache, k-d tree lookup
    font.rs             # Font1 system: 3 skins, glyph painting, size measurement
```

### Pattern 1: Bevy Game State Machine
**What:** Use Bevy's native `States` derive macro with `OnEnter`/`OnExit` schedules for clean state transitions.
**When to use:** Game flow control (menu -> loading -> playing -> paused)
**Example:**
```rust
// Source: Context7 Bevy docs (docs.rs/bevy/latest/bevy/state)
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum GameState {
    #[default]
    MainMenu,
    Loading,
    Playing,
    Paused,
}

// Systems run only in specific states
app.init_state::<GameState>()
   .add_systems(OnEnter(GameState::Loading), start_loading)
   .add_systems(Update, game_loop.run_if(in_state(GameState::Playing)))
   .add_systems(OnEnter(GameState::Paused), show_pause_menu);
```

### Pattern 2: Audio Channel Per Track
**What:** Use bevy_kira_audio's DynamicAudioChannels for the 16-track mixer, mapping to C++ PlyTrack.
**When to use:** Playing sound effects with independent volume and position per track.
**Example:**
```rust
// Source: Context7 bevy_kira_audio docs
use bevy_kira_audio::prelude::*;

fn setup_audio(mut commands: Commands) {
    // DynamicAudioChannels provides named channels at runtime
}

fn play_footstep(
    dynamic_channels: Res<DynamicAudioChannels>,
    asset_server: Res<AssetServer>,
) {
    let channel = dynamic_channels.create_channel("track_0");
    channel.play(asset_server.load("samples/footsteps.ogg"))
        .with_volume(Volume::Amplitude(0.5_f64));
        // **P7-110 FIX (HIGH):** Code updated from stale `Volume::new(0.5)` to correct Bevy 0.18
        // API `Volume::Amplitude(0.5_f64)`. `Volume::new()` does not exist in bevy_kira_audio 0.25.
}
```

### Pattern 3: Shape Vector at RESOLVE Stage
**What:** Replace auto_mat glyph selection with 6D shape-vector nearest-neighbor lookup during the RESOLVE pass. Keep auto_mat for fg/bg color.
**When to use:** Converting SampleBuffer 2x2 blocks to AnsiCell output.
**Example:**
```rust
use kiddo::KdTree;

// **P7-119 FIX (LOW):** Code updated to show correct LruCache type (was stale HashMap).
// LruCache is required to bound memory at 8192 entries (R61 FIX). HashMap grows without limit.
struct ShapeVectorMatcher {
    tree: KdTree<f32, 6>,            // 6D k-d tree of character vectors
    cache: LruCache<u32, u8>,        // quantized 30-bit key -> CP437 glyph (bounded: 8192 entries)
    characters: Vec<CharacterEntry>, // glyph + 6D vector pairs
}
// Note: use lru::LruCache with capacity NonZeroUsize::new(8192).unwrap()
// Do NOT use HashMap here — unbounded growth causes memory issues in complex scenes (R61).

impl ShapeVectorMatcher {
    fn find_glyph(&mut self, sampling_vector: [f32; 6]) -> u8 {
        let key = quantize_to_key(&sampling_vector);
        if let Some(&glyph) = self.cache.get(&key) {
            return glyph;
        }
        let nearest = self.tree.nearest_one::<SquaredEuclidean>(&sampling_vector);
        // nearest.item: u64 (KdTree<f32,6> alias stores u64 items; cast to usize for indexing).
        // When adding: tree.add(&entry.vector, idx as u64).
        let glyph = self.characters[nearest.item as usize].glyph;
        self.cache.insert(key, glyph);
        glyph
    }
}

// P7-001 FIX: Use 5-bit quantization (32 levels) to match "5 bits per component, 30-bit key".
// Previous code used *8.0/.min(7) (3-bit, 8 levels) but shifted by 5 bits, wasting 2 bits per slot.
fn quantize_to_key(vector: &[f32; 6]) -> u32 {
    let mut key: u32 = 0;
    for &v in vector {
        let quantized = ((v * 32.0).floor() as u32).min(31); // 5 bits: 0-31
        key = (key << 5) | quantized;
    }
    key
}
```

### Pattern 4: Ring-Buffer Particle Pool for Weather
**What:** Fixed-capacity ring buffer for snow/rain particles, matching C++ ParticlePool design. No heap allocation during updates.
**When to use:** Weather particle systems that need predictable performance.
**Example:**
```rust
const PARTICLE_CAPACITY: usize = 512;

#[derive(Resource)]
struct ParticlePool {
    particles: [Particle; PARTICLE_CAPACITY],
    head: usize,
    count: usize,
}

impl ParticlePool {
    fn spawn(&mut self, particle: Particle) {
        self.particles[self.head] = particle;
        self.head = (self.head + 1) % PARTICLE_CAPACITY;
        if self.count < PARTICLE_CAPACITY {
            self.count += 1;
        }
    }
}
```

### Anti-Patterns to Avoid
- **Spawning Bevy entities per weather particle:** 512 entities churning every frame is expensive ECS overhead. Use a single Resource with a flat array instead.
- **Blocking audio loading on game thread:** Load samples via Bevy AssetServer async; never block the main loop for Ogg decode.
- **Full entity replication without filtering:** Only replicate visible/nearby entities. bevy_replicon supports visibility filtering -- use it to avoid bandwidth explosion.
- **Running shape-vector matching on GPU for v1:** The CPU path with kiddo's k-d tree and quantized cache is sufficient for 240x135 (32,400 cells). GPU acceleration is a v2 optimization.
- **Mixing auto_mat and shape-vector in the same cell:** The RESOLVE stage should use one or the other per cell, not blend them. Shape-vector replaces glyph selection; auto_mat still provides fg/bg colors.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| k-d tree for nearest-neighbor | Custom tree implementation | kiddo 5.2 crate | SIMD-optimized, const-generic dimensions, heap-free `nearest_one`, benchmarked against alternatives |
| Audio mixing and Ogg decoding | Custom PCM mixer | bevy_kira_audio (wraps Kira) | Thread-safe audio callback, sample management, Ogg/Vorbis decoding via symphonia |
| Entity replication protocol | Custom replication from scratch | bevy_replicon | Handles delta compression, authority model, visibility, component filtering |
| WebSocket framing | RFC 6455 implementation | tungstenite crate | Handles masking, frame splitting, control frames, upgrade handshake |
| Perlin noise | Custom noise generator | noise crate | Correct gradient noise implementation; C++ used siv::PerlinNoise which has same algorithm |
| State machine scheduling | Manual if-else state checks | Bevy States + OnEnter/OnExit | Compile-time state validation, automatic schedule gating, transition events |

**Key insight:** Phase 7 aggregates many small-to-medium subsystems. Each one has a well-established Rust crate. The risk is not in any individual subsystem but in integrating them all cleanly into the existing ECS architecture from Phases 1-6.

## Common Pitfalls

### Pitfall 1: bevy_kira_audio Version Mismatch
**What goes wrong:** Using bevy_kira_audio 0.24 (Bevy 0.17) instead of 0.25 (Bevy 0.18) causes compile errors on AudioPlugin trait changes.
**Why it happens:** Version numbering between bevy_kira_audio and Bevy is not aligned (0.25 != Bevy 0.25).
**How to avoid:** Pin `bevy_kira_audio = "0.25"` explicitly. The STATE.md already flags this: "bevy_kira_audio must be 0.25 (not 0.24) for Bevy 0.18 compatibility."
**Warning signs:** Trait bound errors on `AudioPlugin`, missing `DynamicAudioChannels` type.

### Pitfall 2: Network Protocol Endianness
**What goes wrong:** C++ protocol uses `#pragma pack(push,1)` with little-endian assumption. Rust `repr(C, packed)` matches layout but Rust doesn't guarantee endianness.
**Why it happens:** Cross-platform serialization differences between C++ and Rust.
**How to avoid:** Use explicit `u16::from_le_bytes()` / `to_le_bytes()` for all multi-byte fields, or use `bincode` with little-endian config.
**Warning signs:** Garbled player IDs or positions on big-endian systems (unlikely on x86/ARM but correctness matters).

### Pitfall 3: Shape Vector Cache Thrashing in Complex Scenes
**What goes wrong:** At edges and high-detail regions, every cell produces a unique quantized key, defeating the cache and requiring full k-d tree traversal for all 32,400 cells.
**Why it happens:** 5-bit quantization per component (32 levels) may not cluster enough in visually complex scenes.
**How to avoid:** Monitor cache hit rate at runtime. If below 50%, consider coarser quantization (4 bits = 16 levels, 24-bit key) or temporal caching (reuse previous frame's result if sampling vector changed less than threshold).
**Warning signs:** Frame time spikes in scenes with many edges; profiler showing >5ms in shape-vector matching.

### Pitfall 4: Weather Particles Rendering Behind Terrain
**What goes wrong:** Particles composited too early in the pipeline get overwritten by terrain/world rendering.
**Why it happens:** C++ calls `CompositeSnowParticles` after the RESOLVE stage but before sprite blit. If called before RESOLVE, particles are invisible.
**How to avoid:** Composite weather particles into the AnsiCell grid AFTER the RESOLVE stage and AFTER deferred sprite blit. This matches C++ ordering: Render -> Resolve -> Sprite Blit -> Weather Composite.
**Warning signs:** Particles visible in debug but invisible in final output.

### Pitfall 5: Game State Transitions During Loading
**What goes wrong:** Transitioning to `GameState::Playing` before assets finish loading causes panics on missing handles.
**Why it happens:** Bevy asset loading is async. State transition is immediate.
**How to avoid:** Use a loading system that checks `AssetServer::is_loaded_with_dependencies()` or tracks loading progress via an `AssetEvent` listener before transitioning.
**Warning signs:** Panics on `Assets<T>::get()` returning None; "asset not loaded" warnings in console.

### Pitfall 6: Font Skin Recolor Table Ordering
**What goes wrong:** Gold and pink skins render with wrong colors because the recolor table byte ordering differs from expected.
**Why it happens:** C++ `LoadFont1` uses a compact recolor format: `{count, old_r, old_g, old_b, new_r, new_g, new_b, ..., 0, 0}`. Misinterpreting this format produces garbled colors.
**How to avoid:** The font system loads `sprites/font-1.xp` three times with different recolor tables. Match the exact C++ recolor byte sequences for gold (grey->yellow) and pink (grey->magenta).
**Warning signs:** Menu text renders in wrong colors; gold items appear grey or magenta.

### Pitfall 7: DynamicAudioChannels vs Typed AudioChannels
**What goes wrong:** Using 16 separate typed `AudioChannel<Track0>` through `AudioChannel<Track15>` creates excessive boilerplate and 16 separate Bevy resources.
**Why it happens:** The typed channel approach is idiomatic for bevy_kira_audio but doesn't scale to 16 tracks.
**How to avoid:** Use `DynamicAudioChannels` resource with string keys ("track_0" through "track_15"). This maps naturally to the C++ round-robin `ply_track[PLY_TRACKS]` array.
**Warning signs:** Excessive resource parameters in system signatures; compile-time explosion from 16 generic types.

## Code Examples

### Audio: 16-Track Mixer Setup
```rust
// Source: Context7 bevy_kira_audio 0.25 docs + C++ audio.cpp architecture
use bevy_kira_audio::prelude::*;

const PLY_TRACKS: usize = 16;

#[derive(Resource)]
struct AudioMixer {
    volume: f32,           // 0.0 - 1.0 (C++ uses 0-32768)
    forest_sample: Option<Handle<AudioSource>>,
    track_round_robin: usize,
}

fn setup_audio(app: &mut App) {
    app.add_plugins(AudioPlugin)
       .init_resource::<AudioMixer>();
    // DynamicAudioChannels is automatically available after AudioPlugin
}

fn play_sound_effect(
    dynamic_channels: Res<DynamicAudioChannels>,
    mut mixer: ResMut<AudioMixer>,
    asset_server: Res<AssetServer>,
) {
    let track_name = format!("track_{}", mixer.track_round_robin);
    let channel = dynamic_channels.create_channel(&track_name);
    channel.play(asset_server.load("samples/footsteps.ogg"))
        .with_volume(Volume::Amplitude(mixer.volume as f64));
        // **P7-110 FIX (HIGH):** Code updated from stale `Volume::new(mixer.volume as f64)` to
        // correct Bevy 0.18 API `Volume::Amplitude(mixer.volume as f64)`.
        // `Volume::new()` does not exist in bevy_kira_audio 0.25.
    mixer.track_round_robin = (mixer.track_round_robin + 1) % PLY_TRACKS;
}
```

### Network: bevy_replicon Message Types
```rust
// Source: C++ network_cpp.md protocol + bevy_replicon docs
use serde::{Serialize, Deserialize};

// C++ STRUCT_REQ_JOIN equivalent
#[derive(Serialize, Deserialize, Clone, Debug)]
struct JoinRequest {
    name: String, // max 31 chars
}

// C++ STRUCT_BRC_POSE equivalent
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PoseUpdate {
    anim: u8,
    frame: u8,
    action_mount: u8,
    pos: [f32; 3],
    dir: f32,
    sprite: u16,
}

// C++ STRUCT_BRC_TALK equivalent
#[derive(Serialize, Deserialize, Clone, Debug)]
struct TalkMessage {
    text: String, // max 256 bytes
}
```

### Shape Vector: 6D Sampling from SampleBuffer
```rust
// Source: Alex Harri TypeScript generateSamplingData.ts + Asciicker RESOLVE stage
// Sampling circle positions from six-samples.json metadata
const SAMPLING_POINTS: [[f32; 2]; 6] = [
    [0.27, 0.18],  // upper-left
    [0.73, 0.18],  // upper-right
    [0.27, 0.50],  // middle-left
    [0.73, 0.50],  // middle-right
    [0.27, 0.82],  // lower-left
    [0.73, 0.82],  // lower-right
];

fn sample_cell_vector(
    buffer: &SampleBuffer,
    materials: &[Material],  // P7-003 FIX: required for terrain lightness path
    cell_x: usize,
    cell_y: usize,
) -> [f32; 6] {
    let mut vector = [0.0f32; 6];
    // Each ASCII cell corresponds to a 2x2 block in the supersampled buffer
    // **P7-214 NOTE (LOW):** `+2` is the 2-pixel left/top border of SampleBuffer (matches
    // resolve.rs lines 50-51: `let sx = 2 + 2 * cx`). The total border is 4px (2 each side),
    // and this `+2` correctly offsets to the first valid sample cell. Comment is accurate.
    let sx = cell_x * 2 + 2; // +2 for border offset (2px on each side; left/top adds 2)
    let sy = cell_y * 2 + 2;

    for (i, [px, py]) in SAMPLING_POINTS.iter().enumerate() {
        // Map normalized [0,1] coordinates to the 2x2 sample block
        let sample_x = sx as f32 + px * 2.0;
        let sample_y = sy as f32 + py * 2.0;

        // **P7-202 FIX (CRITICAL):** `buffer.get_sample(usize, usize)` does NOT exist.
        // The correct API is `SampleBuffer::sample_at(x: u32, y: u32) -> &Sample`.
        // Also: the code below originally did point sampling (truncating to integer), but
        // P7-057 FIX (AUTHORITATIVE) requires BILINEAR interpolation (circleRadius=0.19375
        // normalised → 0.3875 buffer pixels, sub-pixel — bilinear captures this correctly).
        // Correct bilinear sampling across 4 integer-coordinate samples:
        let x0 = sample_x.floor() as u32;
        let y0 = sample_y.floor() as u32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let tx = sample_x - sample_x.floor();
        let ty = sample_y - sample_y.floor();
        // All four sample_at calls use u32 (NOT usize):
        let s00 = sample_to_lightness(buffer.sample_at(x0, y0), materials);
        let s10 = sample_to_lightness(buffer.sample_at(x1, y0), materials);
        let s01 = sample_to_lightness(buffer.sample_at(x0, y1), materials);
        let s11 = sample_to_lightness(buffer.sample_at(x1, y1), materials);
        vector[i] = s00 * (1.0 - tx) * (1.0 - ty)
                  + s10 * tx * (1.0 - ty)
                  + s01 * (1.0 - tx) * ty
                  + s11 * tx * ty;
    }
    vector
}

// **P7-202 FIX (CRITICAL) / P7-047 FIX:** `sample.to_rgb888()` does NOT exist on `Sample`.
// Use the dual-path implementation from P7-002/P7-003 FIX.
// Signature updated to accept `materials` parameter for terrain lightness.
fn sample_to_lightness(sample: &Sample, materials: &[Material]) -> f32 {
    // Convert RGB555 (mesh) or material index (terrain) to perceptual lightness [0, 1]
    let (r, g, b) = if sample.spare & MESH_FLAG != 0 {
        // Mesh path: visual is RGB555 — expand to RGB888
        rgb555_to_rgb888(sample.visual)
    } else {
        // Terrain path: visual is a material index — look up the shade table fg color
        let mat_idx = sample.visual as usize;
        if mat_idx < materials.len() {
            let mat = &materials[mat_idx];
            (mat.shade_table[0][0], mat.shade_table[0][1], mat.shade_table[0][2])
        } else {
            (0u8, 0u8, 0u8)
        }
    };
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}
```

### Weather: Particle Spawn and Update
```rust
// Source: C++ weather.cpp ParticlePool + weather state machine
use noise::{NoiseFn, Perlin};

// **P7-056 FIX:** Plan 07-05 uses `lifetime_remaining: f32` (simpler countdown) instead
// of `birth_us: u64`/`lifetime_us: u64` from C++ design. Struct below is authoritative.
#[derive(Clone, Copy, Default)]
struct WeatherParticle {
    pos: [f32; 3],
    vel: [f32; 3],
    lifetime_remaining: f32,  // P7-056: replaces birth_us/lifetime_us from C++ design
    glyph: u8,
    fg: [u8; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum WeatherState {
    #[default]
    Clear = 0,
    LightSnow = 1,
    HeavySnow = 2,
    Blizzard = 3,
}

const SPAWN_RATES: [f32; 4] = [0.0, 10.0, 30.0, 60.0];
const STATE_INTENSITY: [f32; 4] = [0.0, 0.3, 0.7, 1.0];
const SNOW_GLYPHS: [u8; 4] = [0x2A, 0x2B, 0x2E, 0x2C]; // * + . ,
const SNOW_SPEEDS: [f32; 4] = [15.0, 12.0, 9.0, 6.0];

#[derive(Resource)]
struct Weather {
    state: WeatherState,
    intensity: f32,
    target_intensity: f32,
    precipitation: PrecipitationType,
    wind: [f32; 2],
    perlin: Perlin,
    perlin_time: f64,
    pool: ParticlePool,
    spawn_accumulator: f32, // particle spawn accumulator (fractional count from previous frame)
}
```

### Game State: Loading Flow
```rust
// Source: Bevy States docs + C++ mainmenu.cpp loading state machine
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum GameState {
    #[default]
    MainMenu,
    Loading,
    Playing,
    Paused,
}

#[derive(Resource)]
struct LoadingProgress {
    stage: u8,       // 3=init, 2=patches, 1=world rebuild, 0=done
    patch_iter: u32,
    patch_total: u32,
}

fn check_loading_complete(
    progress: Res<LoadingProgress>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if progress.stage == 0 {
        next_state.set(GameState::Playing);
    }
}

fn setup_game_states(app: &mut App) {
    app.init_state::<GameState>()
       .add_systems(OnEnter(GameState::Loading), start_loading)
       .add_systems(
           Update,
           check_loading_complete.run_if(in_state(GameState::Loading)),
       )
       .add_systems(OnEnter(GameState::Playing), enter_gameplay)
       .add_systems(OnEnter(GameState::Paused), show_pause_overlay)
       .add_systems(OnExit(GameState::Paused), hide_pause_overlay);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Custom audio backends per platform (CoreAudio/PulseAudio/SDL) | Kira audio library wrapping platform APIs | 2023+ | bevy_kira_audio abstracts all backends; no need to port 5 C++ platform paths |
| Manual WebSocket RFC 6455 implementation | tungstenite / bevy_replicon crates | 2024+ | C++ hand-rolled WS framing; Rust has battle-tested crates |
| Brightness-to-density glyph mapping (auto_mat) | 6D shape-vector nearest-neighbor matching | 2024 (Alex Harri blog) | Dramatically better edge preservation; structural glyph selection |
| Global state machine with int flags (game_loading) | Bevy States derive macro with schedule integration | Bevy 0.11+ | Type-safe states; OnEnter/OnExit schedules; compile-time verification |
| siv::PerlinNoise C++ library | noise-rs crate | Stable | Same algorithm, Rust-native; no FFI needed |

**Deprecated/outdated:**
- bevy_kira_audio 0.24: Bevy 0.17 only. Use 0.25 for Bevy 0.18.
- lightyear 0.25: Bevy 0.17 only. Use 0.26 for Bevy 0.18 (if choosing lightyear over replicon).
- bevy_replicon 0.37: Bevy 0.17. Use 0.38 for Bevy 0.18.

## Open Questions

1. **lightyear vs bevy_replicon for NET-01/NET-02**
   - What we know: Both support Bevy 0.18. bevy_replicon is simpler and transport-agnostic. lightyear has client-side prediction and WebTransport.
   - What's unclear: How much prediction/interpolation is needed for Asciicker's relatively slow character movement. The C++ engine has no client-side prediction (TRAP-G03 notes manual state sync).
   - Recommendation: Start with bevy_replicon + bevy_renet (simpler). If latency compensation is needed later, migration to lightyear is possible since bevy_replicon's API is a subset of what lightyear provides.

2. **Shape vector alphabet selection: six-samples vs default (with directional crunch)**
   - What we know: The six-samples alphabet uses 6 internal sampling points with NO external points (no directional crunch). The default alphabet uses 6 internal + 10 external points (directional crunch for edge enhancement).
   - What's unclear: Whether the visual improvement from directional crunch justifies the extra complexity (10 additional samples per cell, affects_mapping indirection).
   - Recommendation: Start with six-samples (simpler, no external points). Add directional crunch as a toggle in a follow-up. The k-d tree vectors are the same dimensionality (6D) for both; only the pre-processing differs.

3. **Alphabet JSON as compiled-in data vs loaded asset**
   - What we know: The six-samples.json contains ~80 characters with 6D vectors. Total data is ~15KB. The vectors are font-specific and change if the font changes.
   - What's unclear: Whether font-1.xp's glyph shapes match the vectors generated from the web font in Alex Harri's tool.
   - Recommendation: Include the six-samples.json as a compiled-in `include_str!` for v1. Generate custom vectors from font-1.xp in a v2 build step if needed.

4. **Weather: rain implementation alongside snow**
   - What we know: C++ weather.cpp only implements snow. GAME-03 requires "rain, snow particle systems."
   - What's unclear: What rain should look like in ASCII (vertical lines? dots? different glyphs?).
   - Recommendation: Implement rain as a variant of the snow particle system with different glyphs ('|', '/', ':'), faster fall speeds, and no terrain accumulation. Share the ring buffer and wind system.

5. **Font system integration with GPU shader**
   - What we know: C++ Font1Paint writes directly to the AnsiCell buffer. The Bevy GPU shader reads from AsciiCellGrid textures. The font system needs to write to AsciiCellGrid, not a raw buffer.
   - What's unclear: Whether font rendering should happen CPU-side (write to AsciiCellGrid resource) or GPU-side (separate text overlay texture).
   - Recommendation: CPU-side AsciiCellGrid overlay, matching C++ approach. Font1Paint equivalent writes AnsiCells directly to the grid after the render pipeline produces the scene output. This is simpler and matches the existing architecture.

## C++ Reference Mapping

For the planner -- how each C++ subsystem maps to Rust:

| C++ Component | Lines | Rust Equivalent | Key Differences |
|---------------|-------|-----------------|-----------------|
| audio.cpp DriverAudioCB/DriverAudioCmd | ~200 | bevy_kira_audio DynamicAudioChannels | Kira handles mixing internally; no manual PCM mix loop |
| audio.cpp PlyTrack[16] | ~20 | DynamicAudioChannels with 16 named channels | Channel abstraction replaces raw track array |
| audio.cpp CallAudio queue | ~100 | Bevy events + AudioChannel commands | ECS event system replaces mutex-protected queue |
| network.h/cpp protocol structs | ~210 | serde + bincode structs | Rust serialization replaces C packed structs |
| network.cpp TCP_READ/WRITE | ~980 | bevy_renet transport | Handles connection management, framing |
| game.cpp Server (entity sync) | ~500 | bevy_replicon Replication component | Automatic component sync replaces manual broadcast |
| mainmenu.cpp MainMenu struct | ~200 | Bevy UI + GameState enum | Data-driven menu items; Bevy state for navigation depth |
| mainmenu.cpp ScaleImg/Paint | ~600 | Bevy UI rendering system | Background rendering via Bevy 2D; menu via AsciiCellGrid overlay |
| mainmenu.cpp loading FSM | ~100 | Bevy States with OnEnter/Update/OnExit | Type-safe transitions replace int flags |
| weather.cpp WeatherState + particles | ~449 | Weather resource + ParticlePool | Same ring buffer pattern; noise-rs for Perlin |
| weather.cpp CompositeSnowParticles | ~80 | System writing to AsciiCellGrid post-RESOLVE | Same compositing approach |
| font1.cpp LoadFont1/Font1Paint | ~389 | Font1 resource with skin sprites | Same .xp loading via existing XP parser; BlitSprite equivalent |
| render.cpp auto_mat LUT glyph selection | ~100 (resolve) | ShapeVectorMatcher replaces glyph lookup only | auto_mat still used for fg/bg color; shape vector replaces glyph |

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_bevy_kira_audio_0_25_0_bevy_kira_audio` -- AudioPlugin, AudioChannel, DynamicAudioChannels API
- Context7 `/websites/rs_bevy` -- States, SubStates, ComputedStates, OnEnter/OnExit schedules, state_changed condition
- Context7 `/websites/rs_bevy_renet` -- RenetServerPlugin, RenetClientPlugin, update_system
- [bevy_kira_audio GitHub README](https://github.com/NiklasEi/bevy_kira_audio) -- Version compatibility: 0.25 = Bevy 0.18
- [bevy_replicon GitHub](https://github.com/projectharmonia/bevy_replicon) -- Version 0.38 = Bevy 0.18.0; feature list
- [kiddo docs.rs](https://docs.rs/kiddo) -- Version 5.2.4; f32 support; const-generic dimensions; `nearest_one`
- Alex Harri TypeScript reference: `KdTree.ts` (104 lines), `CharacterMatcher.ts` (52 lines), `effects.ts` (20 lines), `generateSamplingData.ts` (212 lines) -- local at `../reference/alexharri-ascii/website_repo/website-master/src/components/AsciiScene/`
- Alphabet JSON files (six-samples.json, default.json) -- local at same path under `alphabets/`
- C++ architecture docs: `batch_audio.md`, `network_cpp.md`, `mainmenu_cpp.md`, `weather_cpp.md` -- local at `docs/arch/`
- Alex Harri integration research: `docs/research/alexharri-asciicker-integration.md` -- local research document

### Secondary (MEDIUM confidence)
- [lightyear releases](https://github.com/cBournhonesque/lightyear/releases) -- 0.26.0 = Bevy 0.18 (verified via GitHub release notes)
- [noise crate docs.rs](https://docs.rs/noise) -- Perlin noise API, 2D/3D support
- [simdnoise GitHub](https://github.com/verpeteren/rust-simd-noise) -- SIMD-accelerated alternative (not recommended for this use case)
- C++ engine skill packs: `docs/skills/engine-render.md`, `docs/skills/game-mechanics.md`

### Tertiary (LOW confidence)
- bevy_seedling as alternative audio plugin: found via Context7 with high benchmark score (91.5) but no direct Bevy 0.18 compatibility verification
- lightyear 0.26 Bevy 0.18 compatibility: verified via release notes but not tested at runtime
- kiddo 6D performance: documentation confirms const-generic support but benchmarks only show 2D/3D/4D explicitly

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM-HIGH -- All core libraries verified for Bevy 0.18 compatibility via official sources; version pinning known
- Architecture: MEDIUM -- C++ architecture well-documented; Bevy integration patterns established in Phases 1-6; shape-vector integration follows researched phased plan
- Pitfalls: MEDIUM -- Common issues documented from C++ reference and Bevy ecosystem experience; version mismatch is the highest-probability pitfall
- Alex Harri integration: MEDIUM -- TypeScript reference code read directly; algorithm well-understood; kiddo API verified; open question on alphabet vector compatibility with CP437 font

**Research date:** 2026-02-20
**Valid until:** 2026-03-20 (30 days -- stable libraries, locked Bevy 0.18 version)
