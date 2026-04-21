> **STATUS: ACTIVE REFERENCE** — General ECS architecture research, February 2026.

# ECS (Entity Component System) Architecture for Game Development

## Overview

ECS (Entity Component System) is a software architectural pattern predominantly used in video game development for representing game world objects. It provides an alternative to traditional Object-Oriented Programming (OOP) by separating identity, data, and behavior into distinct, loosely coupled components.

## ECS vs OOP: Fundamental Differences

### Object-Oriented Approach

In traditional OOP, game objects are modeled as classes in inheritance hierarchies:

```java
// OOP Example (Java)
public class Player extends Character {
    private Transform transform;
    private Health health;
    private Inventory inventory;
    
    public void update() { /* ... */ }
    public void takeDamage() { /* ... */ }
}
```

**Key OOP Characteristics:**
- **"Is-a" relationships**: Inheritance defines what an object *is*
- **Encapsulation**: Data and behavior live together in objects
- **Deep inheritance trees**: Complex hierarchies (e.g., `GameObject → Character → NPC → Enemy → Boss`)
- **Polymorphism**: Methods defined in parent classes may be overridden

### ECS Approach

In ECS, game objects are built through composition rather than inheritance:

```
Entity: Just an ID (e.g., Entity #42)
  ├── Component: Position { x: 0, y: 0 }
  ├── Component: Velocity { x: 1, y: 0 }
  ├── Component: Health { current: 100, max: 100 }
  └── Component: Renderable { model: "player" }

System: MovementSystem
  → Reads Position + Velocity → Updates Position
```

**Key ECS Characteristics:**
- **"Has-a" relationships**: Composition defines what an object *has*
- **Data/behavior separation**: Components hold data, systems contain logic
- **Flat structure**: No inheritance hierarchies
- **Flexibility**: Entities gain capabilities by adding components

### Problems ECS Solves from OOP

1. **Diamond Problem**: Multiple inheritance ambiguities
2. **Rigid hierarchies**: Hard to add new behaviors without modifying classes
3. **Code duplication**: Similar entities share code through fragile inheritance
4. **Tight coupling**: Behavior tightly coupled to specific object types
5. **Testing difficulties**: Complex object dependencies

---

## Core Concepts

### 1. Entities

An **entity** is simply a unique identifier (ID) - a lightweight, unmanaged reference that associates multiple components together. Entities themselves contain no data or behavior.

```rust
// In Bevy ECS
struct Entity(u64);  // Just an ID

// Creating an entity with components
commands.spawn((
    Position { x: 0.0, y: 0.0 },
    Velocity { x: 1.0, y: 0.0 },
    Player,
));
```

Entities are identified by a unique ID and serve as handles that point to collections of components. An entity can represent anything: a player, enemy, projectile, tree, or UI element.

### 2. Components

**Components** are pure data containers - structs that hold attributes without any methods or logic. They represent the properties or data associated with entities.

```rust
// Components are just data
#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Player;
```

**Component Design Principles:**
- Store only data, never behavior
- Keep components small and focused
- Components should be composable
- Use composition over inheritance

### 3. Systems

**Systems** contain all the logic/behavior. They query entities that have specific component combinations and operate on them. Systems transform data from input state to output state.

```rust
// A movement system in Bevy
fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut query {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

// A greeting system
fn greet_people_system(query: Query<&Name, With<Person>>) {
    for name in &query {
        println!("Hello, {}!", name.0);
    }
}
```

**System Characteristics:**
- Pure logic with no persistent state
- Operate on specific component combinations
- Can run in parallel (data parallelism)
- Declarative: specify what data is needed, not how to get it

### Additional Concepts

#### Resources

**Resources** are singleton-like data types that exist globally in the world, accessible by all systems. They're used for shared state like time, input, asset storage, or game state.

```rust
#[derive(Resource)]
struct GameTime {
    delta_time: f32,
    elapsed: f32,
}

#[derive(Resource)]
struct Score {
    value: i32,
}
```

#### World

The **World** is the container that holds all entities, components, systems, and resources. It's the runtime environment for the ECS.

---

## Why ECS is Popular for Game Development

### 1. Flexibility and Extensibility

- **Runtime composition**: Add/remove components at runtime to change entity behavior
- **No recompilation**: New component types don't require engine changes
- **Modding support**: Easy to expose to scripts or modding APIs
- **Data-driven design**: Game designers can configure entities via data files

### 2. Clean Architecture

- **Loose coupling**: Systems are independent and communicate through components
- **Single responsibility**: Each system handles one specific concern
- **Testability**: Systems can be tested in isolation with mock data
- **Code reuse**: Same systems work on any entity with the right components

### 3. Data-Oriented Design (DOD)

ECS aligns with DOD principles:
- Data is organized for efficient access patterns
- Processing focuses on transforming data streams
- Memory layout optimized for modern CPU architectures

### 4. Domain Alignment

Game development naturally involves:
- Many similar objects (enemies, particles, projectiles)
- Shared behaviors across different object types
- Frequent addition/removal of capabilities

ECS matches this domain better than OOP.

---

## Performance Benefits

### 1. Cache Locality

ECS stores components of the same type in contiguous memory (SoA - Structure of Arrays):

```
Traditional OOP (Array of Structures):
[Entity1: {pos, vel, health}][Entity2: {pos, vel, health}]...

ECS (Structure of Arrays):
Position: [pos1, pos2, pos3, pos4, ...]
Velocity: [vel1, vel2, vel3, vel4, ...]
Health:    [hp1, hp2, hp3, hp4, ...]
```

**Benefits:**
- Processing 1000 entities loads relevant data into cache
- Fewer cache misses = faster execution
- Predictable memory access patterns

### 2. Parallel Processing

Systems can run in parallel because:
- They operate on disjoint sets of components
- No shared mutable state between independent systems
- Data dependencies are explicit and analyzable

```
System A (Position + Velocity) ──┐
                                   ├─→ Run in parallel
System B (Health + Damage)       ──┘
```

### 3. Reduced Memory Overhead

- No vtable pointers (no virtual function calls)
- No object headers
- Components packed tightly in memory
- Sparse sets for efficient component storage

### 4. Batch Processing

Systems iterate over all entities with matching components, processing them in tight loops - optimal for SIMD and vectorization.

### 5. No Dynamic Dispatch

- No polymorphism overhead
- Compile-time resolved component queries
- Inlining opportunities for compilers

### Performance Comparison Example

A Unity case study showed that switching from OOP (MonoBehaviours) to ECS allowed rendering 10,000+ enemies at 60 FPS, compared to ~100 enemies with OOP before frame drops occurred.

---

## Common ECS Frameworks

### Rust

| Framework | Description | Stars | Downloads |
|-----------|-------------|-------|-----------|
| **Bevy ECS** | Built into Bevy engine; highly ergonomic, feature-rich | 44k+ | 4.4M+ |
| **specs** | Popular parallel ECS with storage types | 2.5k | 925k |
| **hecs** | Fast, minimal, ergonomic | - | 317k |
| **legion** | High performance, inspired by Artemis | 834 | 236k |
| **sparsey** | Based on sparse sets | 191 | 21k |
| **apecs** | Async/parallel ECS | 75 | 43k |

#### Bevy ECS Example

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (movement, greeting))
        .run();
}

#[derive(Component)]
struct Position { x: f32, y: f32 }

#[derive(Component)]
struct Velocity { x: f32, y: f32 }

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

fn setup(mut commands: Commands) {
    commands.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }, Person, Name("Alice".into())));
    commands.spawn((Position { x: 5.0, y: 3.0 }, Velocity { x: -0.5, y: 0.5 }, Person, Name("Bob".into())));
}

fn movement(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}

fn greeting(query: Query<&Name, With<Person>>) {
    for name in &query {
        println!("Hello, {}!", name.0);
    }
}
```

#### Specs Example

```rust
use specs::prelude::*;

struct Position { x: f32, y: f32 }
impl Component for Position { type Storage = VecStorage<Self>; }

struct Velocity { x: f32, y: f32 }
impl Component for Velocity { type Storage = VecStorage<Self>; }

struct MovementSystem;

impl<'a> System<'a> for MovementSystem {
    type SystemData = (WriteStorage<'a, Position>, ReadStorage<'a, Velocity>);
    
    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        for (mut pos, vel) in (&mut pos, &vel).join() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }
}
```

### C/C++

- **EntityX**: Lightweight C++ ECS
- **Anax**: Extensible C++ ECS
- **ECS** (Google): Game closure's ECS

### C#

- **Unity DOTS (Entities)**: Official Unity ECS implementation
- **Arch**: .NET ECS inspired by Bevy
- **CommonECS**: Simple C# ECS

### Other Languages

- **Flecs** (C, C++, Python, JavaScript, Rust): Fast ECS with multi-language support
- **Entitas** (C#, JavaScript, Swift, C++): Popular in Unity community
- **A-Frame** (JavaScript): WebVR framework using ECS

### Game Engines with Native ECS

- **Unity**: DOTS (Data-Oriented Technology Stack) with Entities package
- **Unreal Engine**: Uses Actor-Component model (not pure ECS)
- **O3DE**: Entity-component system architecture
- **Bevy**: Rust game engine built entirely on ECS

---

## Summary

ECS provides a data-oriented alternative to OOP that offers:

- **Flexibility** through composition over inheritance
- **Performance** through cache-friendly data layout and parallelism
- **Clean architecture** through separation of data and behavior
- **Maintainability** through decoupled, testable systems

While ECS requires a different mental model and may have a steeper learning curve for developers accustomed to OOP, it has become the architecture of choice for performance-critical games and simulations, especially those requiring large numbers of entities.

---

## References

- [Bevy ECS Documentation](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Specs Book](https://amethyst.github.io/specs/docs/worksheets/tutorials/)
- [Unity Entities Documentation](https://docs.unity3d.com/Packages/com.unity.entities@latest)
- [Wikipedia: Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system)
- [Veloren ECS Documentation](https://book.veloren.net/contributors/developers/ecs.html)
