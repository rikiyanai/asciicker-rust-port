> **STATUS: ACTIVE REFERENCE** — Consolidated implementation planning notes, February 2026.

# Implementation Plan Research - Consolidated

## Overview

This document consolidates research on best practices for implementing the Asciicker C++ to Rust port using Bevy.

---

## 1. Porting Strategy

### Recommended: Incremental Strangler Fig Pattern

```
Phase 1: Core Foundation
├── Set up Bevy project
├── Implement ASCII rendering (no game logic yet)
└── Verify rendering works

Phase 2: Replace Core Systems
├── Replace renderer with Rust
├── Add terrain quadtree
├── Add world BSP
└── Test integrated rendering

Phase 3: Game Logic
├── Convert game state to ECS components
├── Implement physics
├── Add input handling
└── Full gameplay loop

Phase 4: Polish
├── Audio integration
├── Network (if needed)
├── Editor tools
└── Performance optimization
```

---

## 2. Testing Strategy

### Visual Regression Testing
```rust
#[test]
fn test_rendering_matches_expected() {
    let render_output = render_frame(&test_scene);
    let expected = load_golden_file("test_scene.png");
    assert_pixels_close(render_output, expected, tolerance=0.01);
}
```

### Property-Based Testing
```rust
#[test]
fn test_kd_tree_queries_correct() {
    let tree = build_kd_tree(&sample_vectors);
    for _ in 0..1000 {
        assert_eq!(tree.nearest(&query), tree.nearest(&query));
    }
}
```

---

## 3. ECS Architecture

### Component Design
```rust
#[derive(Component)]
pub struct Position(pub Vec3);

#[derive(Component, Bundle)]
pub struct CharacterBundle {
    position: Position,
    velocity: Velocity,
    character_state: CharacterState,
    stats: CharacterStats,
}
```

### System Organization
```
systems/
├── init/           # Startup
├── input/          # Input handling
├── physics/        # Movement, collision
├── ai/             # AI behavior
├── rendering/      # Frame composition
└── ui/             # HUD, menus
```

---

## 4. Rendering Pipeline

### Asciicker → Bevy Mapping

| Asciicker | Bevy |
|-----------|------|
| SampleBuffer | Custom render target textures |
| 6-stage pipeline | Multiple render phases |
| RGB555 | Custom color conversion |
| auto_mat | k-d tree (optional) |

### Render Phase Order
```rust
#[derive(Phase)]
pub enum RenderPhase {
    Clear,
    Terrain,
    World,
    Shadow,
    Reflection,
    Resolve,
    Sprites,
    UI,
}
```

---

## 5. Module Dependencies

```
Rendering (no deps)
    ↓
Terrain + World (needs rendering)
    ↓
Physics (needs terrain + world)
    ↓
Game Logic (needs physics)
    ↓
Input + UI (needs game logic)
    ↓
Audio + Network (optional)
```

---

## 6. Risk Mitigation

| Area | Risk | Mitigation |
|------|------|------------|
| Rendering | Visual regression | Golden file tests |
| Physics | Behavior changes | Property tests |
| Serialization | Data loss | Round-trip tests |
| Performance | Slowdown | Benchmarks |

---

## 7. Implementation Milestones

### M1: Empty Shell (Week 1)
- Bevy project compiles
- Window opens
- Event loop works

### M2: Rendering Foundation (Week 2-3)
- ASCII buffer textures
- Font atlas loads
- Triangle rasterizes

### M3: Complete Rendering (Week 4-5)
- 6-stage pipeline
- auto_mat/k-d tree
- Sprites render

### M4: Terrain + World (Week 6-7)
- Quadtree terrain
- BSP world
- Collision detection

### M5: Game Loop (Week 8-9)
- Player movement
- Input handling
- Basic combat

### M6: Full Game (Week 10+)
- All systems
- UI/HUD
- Save/Load

---

## 8. Key Dependencies

```toml
[dependencies]
bevy = "0.18"
bevy_kira_audio = "0.24"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
goldenfile = "1.4"
proptest = "1.0"
```

---

## 9. Reference Documents

| Document | Purpose |
|----------|---------|
| `plan-SampleBuffer-bridge.md` | Rendering integration |
| `plan-ancestor-cleanup.md` | Memory management |
| `plan-integration-decisions.md` | Key decisions |
| `plan-rendering-gaps.md` | Rendering details |
| `plan-game-logic-gaps.md` | Game systems |
| `plan-systems-gaps.md` | Audio/Input/Network |
| `research-bevy-migration.md` | Migration patterns |

---

*Research completed: 2026-02-20*
