> **STATUS: ACTIVE REFERENCE** — Deep dive implementation research, February 2026.

# Implementation Planning - Deep Dive

This document provides in-depth analysis of the three key areas for implementing the Asciicker port.

---

## 1. Strangler Fig Pattern - Deep Dive

### Core Concept

The Strangler Fig pattern involves gradually replacing a system piece by piece while keeping the old system running until the new system fully takes over.

### Application to Asciicker

```
C++ Asciicker                    Rust Bevy Port
┌─────────────────────┐         ┌─────────────────────┐
│  Original Engine    │   ←→    │  New Engine         │
│  - render.cpp      │   FFI   │  - ASCII rendering │
│  - terrain.cpp     │         │  - Bevy ECS        │
│  - world.cpp       │         │  - Custom systems   │
│  - game.cpp        │         │                     │
└─────────────────────┘         └─────────────────────┘
```

### Porting Order (Strangler Applied)

| Step | Module | Risk | Dependencies |
|------|--------|------|--------------|
| 1 | ASCII buffer/textures | Low | None |
| 2 | Triangle rasterizer | Medium | Buffer |
| 3 | 6-stage pipeline | Medium | Rasterizer |
| 4 | Terrain quadtree | Medium | Rendering |
| 5 | BSP world | Medium | Terrain |
| 6 | Physics | Medium | Terrain+World |
| 7 | Game logic | High | Physics |
| 8 | Input/UI | Medium | Game logic |

### FFI Strategy

Use CXX for safe Rust↔C++ interop:
```rust
// Cargo.toml
cxx = "1.0"

// bridge.rs
cxx::bridge! {
    unsafe extern "C++" {
        include!("asciicker.h");
        fn original_render_function(frame: &mut Frame);
    }
}
```

---

## 2. Testing Strategy - Deep Dive

### Golden File Testing for Rendering

```rust
// tests/rendering/golden.rs
use goldenfile::Mint;
use std::io::Write;

#[test]
fn test_triangle_rasterization() {
    let mut mint = Mint::new("tests/rendering/golden");
    
    let mut output = String::new();
    render_triangle(&mut output, test_triangle());
    
    let expected = mint.read_file("triangle_simple.txt").unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_resolution_independence() {
    // Test same scene at different resolutions
    for &(w, h) in &[(80, 24), (160, 48), (40, 12)] {
        let output = render_scene(w, h);
        let golden = load_golden(format!("scene_{}x{}.txt", w, h));
        assert_text_diff!(output, golden);
    }
}
```

### Property-Based Testing for Algorithms

```rust
// tests/properties/kdtree.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_kd_tree_nearest_equals_linear(vectors in prop::collection::vec(
        prop::collection::vec(0f32..1.0, 6), 10
    )) {
        let kd = KdTree::build(&vectors);
        let query = [0.5f32; 6];
        
        let kd_result = kd.nearest(&query);
        let linear_result = vectors.iter()
            .min_by(|a, b| distance(a, &query).partial_cmp(&distance(b, &query)).unwrap())
            .unwrap();
        
        assert_eq!(kd_result, linear_result);
    }
}
```

### Serialization Round-Trip Testing

```rust
// tests/serialization/roundtrip.rs
#[test]
fn test_terrain_patch_roundtrip() {
    let original = create_test_terrain();
    
    let mut bytes = Vec::new();
    original.save(&mut bytes).unwrap();
    
    let loaded = Terrain::load(&bytes).unwrap();
    
    assert_eq!(original.width, loaded.width);
    assert_eq!(original.height, loaded.height);
    assert_eq!(original.patches.len(), loaded.patches.len());
}
```

### Non-Determinism Handling

```rust
// Use fixed seeds for reproducible tests
#[test]
fn test_combat_deterministic() {
    let seed = 12345;
    let mut rng = StdRng::seed_from_u64(seed);
    
    let result1 = combat_round(&player, &enemy, &mut rng);
    let result2 = combat_round(&player, &enemy, &mut StdRng::seed_from_u64(seed));
    
    assert_eq!(result1, result2);
}
```

---

## 3. Bevy ECS Conversion - Deep Dive

### Converting Asciicker Structs

**Before (C++):**
```cpp
struct Character {
    int x, y, z;
    int hp, mp, xp;
    int weapon, armor, helmet, shield;
    Character* next;
};
```

**After (Bevy ECS):**
```rust
#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Component)]
pub struct CharacterStats {
    pub hp: i32,
    pub mp: i32,
    pub xp: i32,
    pub level: i32,
}

#[derive(Component)]
pub struct Equipment {
    pub weapon: u8,
    pub armor: u8,
    pub helmet: u8,
    pub shield: u8,
}

// Queries become simple
fn combat_system(
    mut query: Query<(&CharacterStats, &Equipment), With<Player>>,
    mut events: EventWriter<DamageEvent>,
) {
    for (stats, equipment) in &mut query {
        // Combat logic
    }
}
```

### Converting Game Loop

**Before (C++):**
```cpp
void GameLoop() {
    while (running) {
        HandleInput();
        UpdatePhysics();
        UpdateAI();
        Render();
    }
}
```

**After (Bevy):**
```rust
// Systems run automatically based on schedule
app.add_systems(Update, handle_input);
app.add_systems(Update, update_physics);
app.add_systems(Update, update_ai.after(update_physics));
app.add_systems(PostUpdate, render.after(update_ai));
```

### Terrain Quadtree → Bevy

```rust
// Store quadtree as resource
#[derive(Resource)]
pub struct TerrainQuadtree {
    root: Option<QuadNode>,
    patches: HashMap<PatchCoord, TerrainPatch>,
}

// Query system
fn terrain_height_at(
    terrain: Res<TerrainQuadtree>,
    position: Vec2,
) -> f32 {
    terrain.query_height(position.x, position.y)
}
```

### 6-Stage Render Pipeline → Bevy

```rust
// Define phases
#[derive(Phase)]
enum RenderPhase {
    Clear,
    Terrain,
    World,
    Shadow,
    Reflection,
    Resolve,
    Sprites,
}

// Add systems to phases
app.add_systems(RenderPhase::Clear, clear_buffer);
app.add_systems(RenderPhase::Terrain, render_terrain);
app.add_systems(RenderPhase::World, render_world);
app.add_systems(RenderPhase::Sprites, render_sprites);
```

---

## 4. Detailed Milestones

### M1: Empty Shell (Week 1)
```
Week 1 Goals:
├── Bevy project setup
├── Cargo dependencies configured
├── Basic window opens
└── Event loop runs

Deliverables:
├── Cargo.toml with bevy 0.18
├── Main.rs with App::new().run()
└── README with build instructions
```

### M2: Rendering Foundation (Weeks 2-3)
```
Week 2-3 Goals:
├── ASCII texture buffers created
├── Font atlas loads
├── Triangle rasterizes to buffer
└── Buffer displays to screen

Deliverables:
├── ASCII buffer module (fg, bg, chars textures)
├── Font atlas with 256 characters
├── Triangle rasterizer
└── Basic window showing ASCII triangle
```

### M3: Complete Rendering (Weeks 4-5)
```
Week 4-5 Goals:
├── 6-stage pipeline works
├── Sprites render correctly
├── auto_mat or k-d tree integrates
└── Performance acceptable (60fps)

Deliverables:
├── Complete render pipeline
├── Sprite system
├── Character selection (auto_mat or k-d)
└── Benchmark results
```

### M4: Terrain + World (Weeks 6-7)
```
Week 6-7 Goals:
├── .xp terrain files load
├── Quadtree terrain queries work
├── .a3d world files load
└── BSP spatial queries work

Deliverables:
├── Terrain quadtree system
├── World BSP system
├── Collision detection
└── Save/load round-trip tests
```

### M5: Game Loop (Weeks 8-9)
```
Week 8-9 Goals:
├── Player controller works
├── Input handling (keyboard/mouse/gamepad)
├── Physics movement
└── Basic combat

Deliverables:
├── Player movement
├── Input system
├── Physics system
└── Basic gameplay test
```

### M6: Full Game (Weeks 10+)
```
Goals:
├── Complete game systems
├── UI/HUD
├── Audio
└── Polish

Deliverables:
├── All game features
├── Menus and HUD
├── Sound effects
└── Beta release
```

---

## 5. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Rendering quality differs | Medium | High | Golden file tests |
| Performance below 60fps | Medium | High | Early benchmarks |
| Missing features | Low | Medium | Scope management |
| Algorithm differences | Medium | Medium | Property tests |
| Memory leaks | Medium | High | Rust safety |

---

## 6. Decision Points for Implementation

Before starting M1:

1. **FFI or Full Rewrite?**
   - FFI = Slower, safer
   - Full Rewrite = Faster long-term, risk short-term
   - Recommendation: Full Rewrite (clean slate)

2. **Auto_mat or k-d tree?**
   - Auto_mat = Fast, proven
   - k-d tree = Better visuals, more work
   - Recommendation: Start with auto_mat, add k-d later

3. **Perspective or Isometric?**
   - Isometric = Simpler, but LOSES Q/E camera rotation
   - Perspective = REQUIRED for port fidelity
   - We have the math documented: focal = 2.0 * max(w,h), 1/viewer_dist divide
   - Recommendation: **MUST do perspective** - game has toggle, Q/E rotation

---

## 7. Project Structure

```
asciicker/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── components/      # Bevy components
│   │   ├── mod.rs
│   │   ├── position.rs
│   │   ├── character.rs
│   │   └── ...
│   ├── systems/         # Bevy systems
│   │   ├── mod.rs
│   │   ├── input.rs
│   │   ├── physics.rs
│   │   └── ...
│   ├── rendering/       # ASCII rendering
│   │   ├── mod.rs
│   │   ├── buffer.rs
│   │   ├── rasterize.rs
│   │   └── ...
│   ├── terrain/        # Quadtree terrain
│   │   ├── mod.rs
│   │   ├── quadtree.rs
│   │   └── ...
│   └── world/          # BSP world
│       ├── mod.rs
│       ├── bsp.rs
│       └── ...
├── tests/              # Tests
│   ├── rendering/
│   ├── properties/
│   └── serialization/
├── assets/             # Game assets
│   ├── fonts/
│   ├── terrain/
│   └── sprites/
└── benches/            # Benchmarks
```

---

*Deep dive completed: 2026-02-20*
