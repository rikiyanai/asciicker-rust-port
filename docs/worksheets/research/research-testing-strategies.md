> **STATUS: ACTIVE REFERENCE** — Testing strategy research for the Rust port.

# Testing Strategies for Asciicker: Golden Files, Property-Based Testing, and Non-Determinism Handling

## Executive Summary

This document examines testing strategies applicable to the Asciicker Rust port, with particular focus on rendering verification, game algorithm validation, and handling non-deterministic output. The research covers four testing paradigms—golden file testing, property-based testing, visual regression testing, and snapshot testing—and provides concrete recommendations for implementing each within the Asciicker codebase.

---

## 1. Golden File Testing for Rendering

### 1.1 Concept Overview

Golden file testing (also called approval testing) captures the expected output of a function or system and stores it in a reference file. Subsequent test runs compare the current output against this golden file, failing if differences exist. This approach is particularly valuable for rendering systems where visual correctness is more important than a specific implementation path.

### 1.2 Application to Asciicker

Asciicker's core rendering produces ASCII/CP437 character output to framebuffers. Golden file testing can verify:

- **Frame output**: The final ASCII frame rendered to terminal or OpenGL texture
- **Sprite rendering**: Individual sprite-to-ASCII conversion results
- **Terrain glyph mapping**: How height values map to CP437 characters
- **Animation frames**: Sequence of frames for walk cycles, attacks, etc.

### 1.3 Implementation Approach

#### Capture Golden Renders

```rust
// Example: Golden frame capture test
#[test]
fn test_sprite_render_wolfie_idle_south() {
    // Arrange: Load the wolfie sprite with equipment code AHSW=0000
    let sprite = load_sprite("wolfie-0000.xp").unwrap();
    
    // Act: Render sprite at position (10, 5) facing south (row 0)
    let frame = render_sprite_to_frame(&sprite, Position::new(10, 5), Direction::South, 0);
    
    // Assert: Compare against golden file
    let golden_path = testdata_path("sprites/wolfie-0000_idle_south.txt");
    golden::Assert::matches(&frame.to_string(), golden_path);
}
```

#### Directory Structure

```
tests/
  golden/
    sprites/
      wolfie-0000_idle_south.txt
      wolfie-0000_walk_south_0001.txt
      player-0100_idle_east.txt
    frames/
      scene_001_initial.txt
      scene_002_after_move.txt
    terrain/
      flat_terrain_glyphs.txt
      hill_terrain_glyphs.txt
```

#### Tools and Libraries

| Tool | Purpose | Rust Support |
|------|---------|---------------|
| `goldenfile` | Core golden file testing | Crate: `goldenfile` |
| `insta` | Snapshot testing with inline support | Crate: `insta` |
| `similar-asserts` | Better diff output | Crate: `similar-asserts` |

The `goldenfile` crate provides the most straightforward golden file testing:

```rust
use goldenfile::Mint;
use std::io::Write;

#[test]
fn test_render_output() {
    let mut mint = Mint::new("tests/golden");
    let mut diff = mint.new_golden_file("output.txt").unwrap();
    
    // Write actual output
    write!(diff, "{}", render_frame(&game_state));
    
    // Test will automatically compare
}
```

### 1.4 ASCII-Specific Considerations

Unlike bitmap rendering, ASCII rendering has unique characteristics:

- **Character boundary precision**: Each cell is discrete—exact glyph matches expected
- **Terminal vs. texture**: Output may go to terminal (ANSI escape codes) or OpenGL texture
- **Color encoding**: Foreground/background color pairs in CP437 require exact matching

For terminal output, consider normalizing:
- Strip cursor positioning commands
- Handle color code variations (256-color vs. true color)
- Normalize line endings

---

## 2. Property-Based Testing for Game Algorithms

### 2.1 Concept Overview

Property-based testing (PBT) verifies that code satisfies arbitrary properties across thousands of randomly generated inputs. Rather than testing specific cases, PBT generates diverse inputs and checks invariants. This is especially valuable for game algorithms where edge cases are numerous.

### 2.2 Application to Asciicker

#### 5D Sprite Lookup Algorithm

The Asciicker sprite system uses a 5-dimensional array for equipment lookup:

```
player[color][armor][helmet][shield][weapon]
```

Properties to verify:
- All valid index combinations return a valid sprite
- Out-of-bounds indices panic or return appropriate errors
- Same indices always return the same sprite reference

#### Collision Detection

Properties:
- No two solid entities occupy the same space
- Collision response is consistent across equivalent inputs
- Entities cannot pass through solid terrain

#### Terrain Generation

Properties:
- Generated terrain stays within valid height bounds
- Terrain is always walkable (no impossible slopes without stairs)
- Serialization/deserialization produces identical terrain

### 2.3 Implementation with Proptest

The `proptest` crate provides property-based testing for Rust:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_sprite_lookup_5d(color in 0..4, armor in 0..5, helmet in 0..4, shield in 0..4, weapon in 0..8) {
        let sprite = get_player_sprite(color, armor, helmet, shield, weapon);
        prop_assert!(sprite.is_some(), "Valid indices should return sprite");
    }
    
    #[test]
    fn test_sprite_lookup_out_of_bounds(color in 10..20) {
        let result = std::panic::catch_unwind(|| {
            get_player_sprite(color, 0, 0, 0, 0)
        });
        prop_assert!(result.is_err() || result.unwrap().is_none());
    }
    
    #[test]
    fn test_equipment_string_roundtrip(armor in 0..5, helmet in 0..4, shield in 0..4, weapon in 0..8) {
        let code = equipment_to_ahsw_code(armor, helmet, shield, weapon);
        let (a, h, s, w) = ahsw_code_to_equipment(code);
        prop_assert_eq!((a, h, s, w), (armor, helmet, shield, weapon));
    }
}
```

### 2.4 Strategies for Game-Specific Properties

#### Reversibility

Many game operations should be reversible:

```rust
proptest! {
    #[test]
    fn test_position_serialization_roundtrip(pos in any::<Position>()) {
        let serialized = pos.serialize();
        let deserialized = Position::deserialize(&serialized);
        prop_assert_eq!(deserialized, Some(pos));
    }
}
```

#### Invariants

Game state must maintain critical invariants:

```rust
proptest! {
    #[test]
    fn test_health_always_valid(health in -100i32..1000) {
        let entity = Entity { health };
        prop_assert!(entity.is_alive() == (entity.health > 0));
    }
}
```

#### Equivalence Classes

Group inputs into equivalence classes and test representative cases:

```rust
// Instead of testing every direction, test direction groups
proptest! {
    #[test]
    fn test_sprite_direction_groups(dir in 0..8u8) {
        let group = match dir {
            0 | 1 => "south",
            2 | 3 => "west",
            4 | 5 => "north",
            6 | 7 => "east",
            _ => unreachable!()
        };
        // Verify same group renders similarly
    }
}
```

### 2.5 Algorithms Requiring Property Tests

Based on the Asciicker architecture:

| Algorithm | Property | Recommended Strategy |
|-----------|----------|---------------------|
| 5D sprite lookup | Index validity | Exhaustively test all 4×5×4×4×8 = 2560 combinations |
| AHSW code encoding/decoding | Roundtrip consistency | 1000 random iterations |
| Collision detection | No overlap after resolution | 500 random entity configurations |
| Terrain height interpolation | Bounded output | 1000 random interpolations |
| Animation frame selection | Correct frame for time | Test boundary conditions |
| XP file parsing | Parse→serialize→parse idempotent | 100 random valid XP files |

---

## 3. Visual Regression Testing Tools

### 3.1 Concept Overview

Visual regression testing captures rendering output and compares it against baseline images to detect unintended visual changes. While traditionally applied to bitmap graphics, similar principles apply to ASCII rendering.

### 3.2 ASCII-Specific Approaches

#### Text Diff Tools

For ASCII art, standard text comparison may be too strict:

```rust
use similar::{ChangeTag, TextDiff};

fn compare_ascii_frames(actual: &str, expected: &str) -> VisualDiffResult {
    let diff = TextDiff::from_lines(expected, actual);
    
    // Group changes for readable output
    let changes: Vec<_> = diff
        .iter_all_changes()
        .filter(|c| c.tag() != ChangeTag::Equal)
        .collect();
    
    VisualDiffResult {
        identical: changes.is_empty(),
        change_count: changes.len(),
        diff_output: format!("{:?}", changes)
    }
}
```

#### Semantic Comparison

ASCII rendering may have acceptable variations:

```rust
fn frames_semantically_equal(actual: &str, expected: &str) -> bool {
    // Normalize whitespace
    let normalize = |s: &str| s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    
    normalize(actual) == normalize(expected)
}
```

### 3.3 Perceptual Hashing for ASCII

For more lenient matching, implement a simple perceptual hash:

```rust
fn ascii_frame_hash(frame: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    // Hash character frequency (ignoring exact position)
    let mut char_counts = [0u32; 256];
    for byte in frame.bytes() {
        char_counts[byte as usize] += 1;
    }
    
    // Hash the frequency distribution
    char_counts.hash(&mut hasher);
    hasher.finish()
}
```

### 3.4 Tools Comparison

| Tool | Type | Best For | ASCII Support |
|------|------|----------|---------------|
| `insta` | Snapshot | Rendered output | Good (text-based) |
| `goldentest` | Golden files | Known-good comparisons | Good |
| `pixelmatch` | Image comparison | Bitmap sprites | Requires conversion |
| `skia-gold` | Cloud-based | Large-scale rendering | Requires image conversion |

---

## 4. Snapshot Testing for Serialization

### 4.1 Concept Overview

Snapshot testing captures the output of serialization or deserialization and stores it for future comparison. Unlike golden files (which test behavior), snapshots verify data structure consistency.

### 4.2 Application to Asciicker

#### XP Sprite Format

The REXPaint XP format is gzip-compressed binary. Snapshot tests should verify:

- Parsing produces expected layer/cell structure
- Serialization roundtrips correctly
- Invalid files produce appropriate errors

#### A3D Mesh Format

Asciicker's custom 3D mesh format requires snapshot tests for:

- Valid mesh parsing
- Edge cases (empty meshes, maximum complexity)
- Version compatibility

### 4.3 Implementation with Insta

The `insta` crate provides excellent snapshot testing with inline and file-based snapshots:

```rust
#[test]
fn test_xp_parsing_wolfie() {
    let data = std::fs::read("testdata/sprites/wolfie-0000.xp").unwrap();
    let sprite = XpParser::parse(&data).unwrap();
    
    // Snapshot the parsed structure
    insta::assert_debug_snapshot!(sprite, @r###"
        XpSprite {
            version: 1,
            layers: [
                XpLayer {
                    width: 18,
                    height: 96,
                    cells: 1728,
                },
                XpLayer {
                    width: 18,
                    height: 96,
                    cells: 1728,
                },
                XpLayer {
                    width: 18,
                    height: 96,
                    cells: 1728,
                },
            ],
        }
    "###);
}
```

### 4.4 Serialization Roundtrip Tests

Critical for data integrity:

```rust
#[test]
fn test_xp_roundtrip() {
    let original = std::fs::read("testdata/sprites/wolfie-0000.xp").unwrap();
    let parsed = XpParser::parse(&original).unwrap();
    let serialized = XpSerializer::serialize(&parsed);
    
    // Re-parse and compare
    let reparsed = XpParser::parse(&serialized).unwrap();
    
    assert_eq!(parsed.layers.len(), reparsed.layers.len());
    for (orig_layer, reprocessed_layer) in parsed.layers.iter().zip(reparsed.layers.iter()) {
        assert_eq!(orig_layer.width, reprocessed_layer.width);
        assert_eq!(orig_layer.height, reprocessed_layer.height);
        // Compare cell-by-cell for exact match
    }
}
```

### 4.5 Recommended Snapshot Tests

| Data Type | Snapshot Content | Update Frequency |
|-----------|------------------|------------------|
| XP files | Parsed structure summary | When format changes |
| A3D meshes | Vertex/index counts, bounds | When loader changes |
| Save files | Full serialized output | When save format changes |
| Configuration | Parsed config struct | Rarely (stable API) |

---

## 5. Handling Non-Deterministic Output

### 5.1 Sources of Non-Determinism in Games

Asciicker exhibits several non-deterministic behaviors:

| Source | Affected Systems | Severity |
|--------|-----------------|----------|
| Floating-point arithmetic | Physics, rendering positions | Medium |
| Random number generation | Combat, terrain generation, AI | High |
| Frame-rate dependent timing | Animation, physics | High |
| Thread scheduling | Multi-threaded systems | Medium |
| Hardware differences | Floating-point precision | Low |

### 5.2 Strategies for Testing Non-Deterministic Code

#### Seed-Based Randomness

Control randomness via seeds:

```rust
fn seeded_random(seed: u64) -> impl Rng {
    SmallRng::seed_from_u64(seed)
}

#[test]
fn test_terrain_generation_deterministic() {
    let seed = 12345;
    let terrain1 = generate_terrain(seeded_random(seed));
    let terrain2 = generate_terrain(seeded_random(seed));
    
    assert_eq!(terrain1, terrain2, "Same seed must produce identical terrain");
}
```

#### Fixed Timestep Simulation

For frame-rate dependent code, use fixed timesteps:

```rust
const FIXED_DT: f32 = 1.0 / 60.0;

#[test]
fn test_physics_deterministic() {
    let mut world = World::new();
    world.add_entity(Entity::player());
    
    // Run exactly 60 frames
    for _ in 0..60 {
        world.step(FIXED_DT);
    }
    
    // Player should be at exact same position every time
    let position = world.entity(Entity::player()).position();
    assert!((position.x - 10.0).abs() < 0.001);
}
```

#### Tolerances for Floating-Point

Accept near-matches for floating-point:

```rust
fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
    (a - b).abs() < epsilon
}

#[test]
fn test_interpolated_position() {
    let pos = interpolate_position(start, end, 0.5);
    assert!(approx_eq(pos.x, (start.x + end.x) / 2.0, 0.001));
}
```

### 5.3 Golden File Handling for Non-Deterministic Output

#### Separate Golden Files

Mark non-deterministic tests explicitly:

```rust
#[test]
#[ignore = "Non-deterministic: requires fixed seed"] 
fn test_random_combat_outcome() {
    // This test may produce different results
}
```

#### Update Flags

Provide update mechanisms for when golden files legitimately change:

```rust
// Run with: cargo test -- --update
#[test]
fn test_combat_output() {
    let result = run_combat_simulation(seeded_random(42));
    golden::Assert::matches(&result.to_string(), "combat_001.txt");
}
```

### 5.4 Box2D Determinism Testing Pattern

The Box2D physics engine provides an excellent model for deterministic testing:

```rust
#[test]
fn test_determinism_falling_hinges() {
    // Run simulation twice
    let result1 = run_physics_simulation();
    let result2 = run_physics_simulation();
    
    // Compare state hashes
    let hash1 = compute_state_hash(&result1);
    let hash2 = compute_state_hash(&result2);
    
    assert_eq!(hash1, hash2, "Simulation must be deterministic");
    
    // Also verify against known-good hash
    assert_eq!(hash1, 0x5e70e5fe, "Simulation state matches expected");
}
```

### 5.5 Recommendations for Asciicker

| System | Non-Determinism Strategy |
|--------|-------------------------|
| Terrain generation | Seed-based, test with fixed seeds |
| Combat RNG | Seed all random calls, test specific seeds |
| Animation timing | Use fixed timestep in tests |
| Physics | Fixed timestep, tolerance-based comparisons |
| Rendering | Should be deterministic if inputs are fixed |

---

## 6. Implementation Roadmap

### Phase 1: Foundation

1. **Set up testing infrastructure**
   - Add `proptest`, `insta`, `goldenfile` to dev dependencies
   - Create `tests/golden/` directory structure

2. **Snapshot tests for file formats**
   - XP parser snapshot tests
   - A3D mesh parser snapshot tests

### Phase 2: Core Algorithm Testing

3. **Property-based tests for sprite lookup**
   - 5D index validation
   - AHSW code roundtrip

4. **Golden tests for rendering**
   - Capture representative sprite renders
   - Capture terrain glyph mappings

### Phase 3: Non-Determinism Handling

5. **Determinism tests**
   - Physics determinism
   - Random number generation seeding
   - Terrain generation determinism

6. **Tolerance-based comparisons**
   - Floating-point comparisons with epsilon
   - Frame timing with tolerance

---

## 7. Appendix: Recommended Test Dependencies

```toml
[dev-dependencies]
proptest = "1.5"
insta = { version = "1.40", features = ["yaml", "glob"] }
goldenfile = "1.9"
similar = "2.6"
```

---

## References

- [Golden File Testing (Matt Proud, 2025)](https://matttproud.com/blog/posts/golden-file-testing.html)
- [TensorFlow Federated Golden Tests](https://www.tensorflow.org/federated/golden_tests)
- [Rust goldenfile crate](https://docs.rs/goldenfile/latest/goldenfile/)
- [Rust insta crate](https://insta.rs/)
- [Proptest documentation](https://altsysrq.github.io/proptest/doc/proptest/)
- [Box2D Determinism Testing](https://box2d.org/posts/2024/08/determinism/)
- [Property-Based Testing for Chess (Stack Overflow)](https://stackoverflow.com/questions/55274003/property-based-testing-for-a-chess-game)
- [Unreal Engine Screenshot Comparison](https://dev.epicgames.com/documentation/en-us/unreal-engine/screenshot-comparison-tool-in-unreal-engine)
- [Unity Graphics Test Framework](https://docs.unity3d.com/Packages/com.unity.testframework.graphics@7.2/manual/index.html)
- [Taming Time in Game Engines (André Leite, 2025)](https://andreleite.com/posts/2025/game-loop/fixed-timestep-game-loop/)
