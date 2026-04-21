> **STATUS: ACTIVE REFERENCE** — The hybrid approach (Bevy ECS + Mage Core ASCII rendering) described as Pattern 3 aligns with current architecture decisions. Note: Line 5 incorrectly claims "Mage-core heritage" — Asciicker is a C++ engine; Mage Core is a separate Rust ASCII engine being evaluated for rendering integration. Some code examples reference non-existent Bevy APIs — verify against current Bevy 0.18+ docs.

# Bevy Migration Best Practices

## Overview

This document outlines strategies and patterns for migrating an existing game codebase to the Bevy game engine, specifically tailored for the asciicker project with its Mage-core heritage.

---

## Migration Phases

### Phase 1: Assessment and Planning

**Analyze Existing Codebase**
- Identify core game loop structure
- Catalog all systems: rendering, input, physics, game logic
- Map dependencies between modules
- Identify external dependencies (graphics, audio, windowing)

**Key Questions**
- Is the existing code organized in a way that facilitates modular replacement?
- Are there clear boundaries between rendering, input, and game logic?
- What percentage of code is rendering vs. game logic?

### Phase 2: Incremental Migration Strategy

**Recommended Approach: Parallel Development**

```
┌─────────────────────────────────────────────────────────┐
│                    Migration Strategy                   │
├─────────────────────────────────────────────────────────┤
│  1. Create new Bevy project (or add bevy_ecs crate)    │
│  2. Move one subsystem at a time                         │
│  3. Test each migration before proceeding              │
│  4. Keep old rendering until new one works              │
│  5. Gradually replace old code                          │
└─────────────────────────────────────────────────────────┘
```

**Why Incremental?**
- Reduces risk of complete rewrites
- Allows learning ECS patterns progressively
- Maintains working game throughout migration
- Easier to debug issues

---

## Porting Patterns

### Pattern 1: Embedded ECS (bevy_ecs crate)

Use only Bevy's ECS without the full engine:

```rust
// Use bevy_ecs without full Bevy for easier migration
use bevy_ecs::prelude::*;

fn main() {
    let mut world = World::new();
    let mut schedule = Schedule::default();
    
    schedule.add_systems(move_entities);
    
    world.spawn((Position { x: 0.0 }, Velocity { x: 1.0 }));
    
    loop {
        schedule.run(&mut world);
    }
}
```

**Pros:**
- Lower migration cost
- Can migrate subsystem-by-subsystem
- No need to rewrite everything at once
- Better for existing complex codebases

**Cons:**
- Missing Bevy plugins (audio, UI, asset loading)
- Must reimplement windowing, input handling

### Pattern 2: Full Bevy Migration

Complete rewrite using Bevy's full plugin ecosystem:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (input, update, render))
        .run();
}
```

**Pros:**
- Full ecosystem (audio, UI, assets, input)
- Hot reloading
- Community plugins
- Better long-term maintenance

**Cons:**
- Higher initial migration cost
- Steeper learning curve
- More API changes to track

### Pattern 3: Hybrid Approach

Keep custom rendering (Mage-core ASCII) while using Bevy for everything else:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            wgpu_settings: WgpuSettings { ... },
        }))
        .init_resource::<AsciiScreen>()      // Custom ASCII buffer
        .add_plugins(AsciiRenderPlugin)      // Custom render
        .add_systems(Update, (
            bevy_input::input_system,         // Use Bevy input
            game_logic_system,                // Your game logic
            update_ascii_buffer,              // Update ASCII
        ))
        .run();
}
```

This is the recommended approach for asciicker.

---

## Converting Game Loops to Bevy Systems

### Traditional Game Loop

```rust
// Traditional (imperative)
fn main() {
    let mut game = Game::new();
    
    loop {
        let dt = timer.tick();
        
        game.handle_input();
        game.update(dt);
        game.render();
        
        if game.should_exit() { break; }
    }
}
```

### Bevy Systems Approach

```rust
// Bevy (data-oriented)
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, (
            handle_input,
            update_game,
            render,
        ))
        .run();
}

fn handle_input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        game_state.should_exit = true;
    }
    // Map input to game actions
}

fn update_game(
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    // Game logic here
    if game_state.should_exit {
        // Handle exit through Bevy
    }
}

fn render(
    mut commands: Commands,
    game_state: Res<GameState>,
) {
    // Rendering logic
}
```

### Step-by-Step Conversion

1. **Extract initialization to Startup systems**
   ```rust
   // Before: main() sets up everything
   // After:
   fn setup(mut commands: Commands) {
       commands.spawn((Player, Position { x: 0.0 }, Velocity::default()));
   }
   ```

2. **Convert game loop to Update systems**
   ```rust
   // Before: while loop with state machine
   // After: systems that run every frame
   
   fn game_update(
       mut query: Query<(&mut Position, &Velocity)>,
       time: Res<Time>,
   ) {
       let dt = time.delta_seconds();
       for (mut pos, vel) in &mut query {
           pos.x += vel.x * dt;
           pos.y += vel.y * dt;
       }
   }
   ```

3. **Use Resources for global state**
   ```rust
   // Instead of global variables
   #[derive(Resource)]
   struct GameConfig {
       screen_width: u32,
       screen_height: u32,
       debug_mode: bool,
   }
   ```

4. **Convert entities to ECS components**
   ```rust
   // Before: struct GameObject { pos, vel, health, ... }
   // After:
   #[derive(Component)]
   struct Position { x: f32, y: f32 }
   
   #[derive(Component)]
   struct Velocity { x: f32, y: f32 }
   
   #[derive(Component)]
   struct Health { current: f32, max: f32 }
   ```

---

## ECS Patterns for Game Logic

### Component Design

**Data-Only Components**
```rust
// Good: Pure data
#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

// Avoid: Components with behavior
#[derive(Component)]
struct Player {
    name: String,
    fn take_damage(&mut self) { ... }  // Don't do this
}
```

**Composition Over Inheritance**
```rust
// Instead of inheritance hierarchies...
// struct Enemy { health, damage, ai_behavior }
// struct Player { health, inventory, stats }

// Use composition
#[derive(Component)] struct Enemy;
#[derive(Component)] struct Player;
#[derive(Component)] struct Health { current: f32, max: f32 }
#[derive(Component)] struct Damage { value: f32 }
#[derive(Component)] struct AIBehavior { pattern: AIPattern }
#[derive(Component)] struct Inventory { items: Vec<Item> }
```

### System Design

**Single Responsibility**
```rust
// Split into focused systems
fn movement_system(mut query: Query<(&mut Position, &Velocity)>) { ... }
fn collision_system(query: Query<(&Position, &Collider)>) { ... }
fn health_system(mut query: Query<(&Health, &Damage)>) { ... }
fn animation_system(query: Query<(&Sprite, &AnimationState)>) { ... }
```

**Query Design**
```rust
// Basic query
fn system(query: Query<&Position>) { ... }

// With mutation
fn system(query: Query<&mut Position>) { ... }

// Filtered
fn system(query: Query<&Position, With<Player>>) { ... }

// With optional
fn system(query: Query<(&Position, Option<&Damage>)>) { ... }

// Multiple components
fn system(query: Query<(&mut Position, &Velocity)>) { ... }
```

### Resource Patterns

**Global State**
```rust
#[derive(Resource)]
struct Score {
    value: i32,
}

fn add_score(mut score: ResMut<Score>, to_add: Res<ScoreEvent>) {
    score.value += to_add.0;
}
```

**Configuration**
```rust
#[derive(Resource)]
struct GameSettings {
    difficulty: Difficulty,
    show_fps: bool,
    master_volume: f32,
}
```

---

## Mage-core to Bevy Specifics

### Rendering Migration

| Mage-core | Bevy Approach |
|-----------|---------------|
| Custom WGPU pipeline | Custom render plugin or Bevy sprites |
| ASCII buffer (Image) | `Assets<Image>` + texture updates |
| Present/blit | Custom system or Bevy 2D renderer |
| Font atlas | `FontAtlasSet` or custom texture |

### Input Migration

| Mage-core | Bevy Approach |
|-----------|---------------|
| Modifiers only | Full `ButtonInput<KeyCode>` |
| - | Mouse input (`MouseMotion`, `MouseButton`) |
| - | Gamepad support (`Gamepad`, `GamepadButton`) |

### Architecture Mapping

```
Mage-core          →    Bevy
─────────────────────────────────────────────
lib.rs             →    App + plugins
app.rs (tick/present) →  Update schedule
render.rs (WGPU)  →    RenderPlugin + custom
input.rs           →    bevy_input
colour.rs          →    bevy_color
image.rs           →    Components + Assets
present.rs         →    Custom system
```

---

## Testing Strategies

### Unit Testing Systems

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_movement() {
        let mut world = World::new();
        
        // Spawn test entity
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            Velocity { x: 5.0, y: 3.0 },
        ));
        
        // Run system
        movement_system(&mut world);
        
        // Verify
        let pos = world.query::<&Position>().single(&world);
        assert!((pos.x - 5.0).abs() < 0.01);
    }
}
```

### Integration Testing

```rust
fn test_game_state_transition() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    
    // Setup game state
    app.insert_resource(GameState::Playing);
    
    // Trigger events
    app.world().spawn((Player, Position::default()));
    
    // Run systems
    app.update();
    
    // Assert
    let state = app.world().resource::<GameState>();
    assert_eq!(*state, GameState::Paused);
}
```

---

## Migration Checklist

### Pre-Migration
- [ ] Document existing architecture
- [ ] Identify boundaries between subsystems
- [ ] Choose migration pattern (embedded/full/hybrid)
- [ ] Set up CI/CD for testing

### Phase 1: Setup
- [ ] Add Bevy dependency
- [ ] Create basic Bevy app structure
- [ ] Verify empty app compiles and runs

### Phase 2: Core Systems
- [ ] Migrate input handling (use Bevy input)
- [ ] Create ECS components for game entities
- [ ] Implement game logic as systems
- [ ] Add state management (Bevy States)

### Phase 3: Rendering
- [ ] Integrate custom ASCII rendering
- [ ] Implement font atlas handling
- [ ] Create present/blit system

### Phase 4: Polish
- [ ] Remove old code
- [ ] Optimize systems
- [ ] Add asset loading (Bevy asset system)
- [ ] Implement audio (bevy_kira_audio)

### Post-Migration
- [ ] Run full test suite
- [ ] Performance profiling
- [ ] Verify all features work
- [ ] Clean up dead code

---

## Common Pitfalls

1. **Fighting the ECS**: Trying to maintain OOP patterns in ECS
2. **Over-abstracting**: Creating components too early
3. **Ignoring change detection**: Not using `Mut`/`ResMut` properly
4. **System ordering**: Not understanding Bevy schedules
5. **Resource misuse**: Using Resources where Components are better

---

## References

- [Bevy Book](https://bevy.org/learn/book/)
- [Bevy ECS Examples](https://github.com/bevyengine/bevy/tree/main/examples)
- [Migration Guides](https://bevy.org/learn/migration-guides/)
- [Bevy ECS Crate](https://crates.io/crates/bevy_ecs)

---

*Document Version: 1.0*
*Created: 2026-02-20*
