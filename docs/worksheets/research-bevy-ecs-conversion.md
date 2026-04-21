> **STATUS: ACTIVE REFERENCE WITH CORRECTIONS** — ECS migration analysis. CORRECTIONS: Code examples use deprecated Bevy APIs (add_plugin → add_plugins, delta_seconds → delta_secs, SpriteBundle removed in 0.14, storage names "DenseVec"/"SparseVec" → "Table"/"SparseSet"). Verify all code against Bevy 0.18 docs.

# Bevy ECS Conversion Research for Asciicker

This document provides in-depth research for converting the C++ asciicker game engine to Bevy ECS, covering general conversion patterns and specific application to asciicker's architecture.

---

## Table of Contents

1. [Converting C++ Structs to Bevy Components](#1-converting-c-structs-to-bevy-components)
2. [Converting Game Loop to Bevy Systems](#2-converting-game-loop-to-bevy-systems)
3. [Managing State in ECS](#3-managing-state-in-ecs)
4. [Performance Considerations](#4-performance-considerations)
5. [Asciicker-Specific Conversions](#5-asciicker-specific-conversions)
   - [Game Struct to ECS](#51-game-struct-to-ecs)
   - [Terrain Quadtree to Component+System](#52-terrain-quadtree-to-componentsystem)
   - [BSP World to Spatial System](#53-bsp-world-to-spatial-system)
   - [6-Stage Render Pipeline](#54-6-stage-render-pipeline)
6. [Implementation Roadmap](#6-implementation-roadmap)

---

## 1. Converting C++ Structs to Bevy Components

### Core Concept Mapping

In C++ OOP, data and behavior are typically bundled together in classes. In ECS, you separate data (Components) from behavior (Systems).

| C++ Pattern | Bevy ECS Equivalent | Notes |
|-------------|-------------------|-------|
| `struct Foo { int x; float y; }` | `#[derive(Component)] struct Foo { x: i32, y: f32 }` | Components are plain Rust structs |
| `class Player : public Entity` | Multiple components on one Entity | Use composition, not inheritance |
| `virtual void update() = 0` | System function | Behavior lives in systems |
| `std::vector<Component*>` | `Query<&Component>` | Iterate over component data |
| `entity->addComponent<T>()` | `commands.entity(e).insert(T)` | Add components to entities |

### Component Design Principles

1. **Data-Only**: Components should only contain data, never methods with logic
2. **Flat Structures**: Avoid nested complex types; use Entity IDs for relationships
3. **Sparse vs Dense**: Bevy uses archetype storage - group frequently accessed components together
4. **No Ownership**: Use Entity IDs to reference other entities

### Example Conversion

**C++:**
```cpp
struct Player {
    Vec3 position;
    Vec3 velocity;
    float health;
    int weapon_id;
    std::string name;
};

void Player::update(float dt) {
    position += velocity * dt;
}
```

**Bevy:**
```rust
#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component)]
struct PlayerName(String);

#[derive(Component)]
struct WeaponLink {
    entity: Entity, // Reference to weapon entity
}

fn player_movement_system(
    mut query: Query<(&mut Position, &Velocity)>,
    time: Res<Time>,
) {
    for (mut pos, vel) in &mut query {
        pos.x += vel.x * time.delta_seconds();
        pos.y += vel.y * time.delta_seconds();
        pos.z += vel.z * time.delta_seconds();
    }
}
```

### Component Storage Strategies

Bevy provides different storage types for components:

```rust
#[derive(Component)]
struct FrequentUpdate {
    value: f32,
} // Default: SparseSet - good for frequent add/remove

#[derive(Component)]
struct DenseStorage {
    data: Vec<f32>,
} // Force dense storage with: #[component(storage = "DenseVec")]

#[derive(Component)]
struct SparseStorage {
    id: u32,
} // Explicit sparse with: #[component(storage = "SparseVec")]
```

---

## 2. Converting Game Loop to Bevy Systems

### Traditional C++ Game Loop

```cpp
while (running) {
    float dt = calculate_delta_time();
    
    handle_input();
    update_physics(dt);
    update_ai(dt);
    update_animations(dt);
    render();
    
    present();
}
```

### Bevy System Architecture

In Bevy, you define systems as Rust functions, and Bevy's scheduler determines execution order:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            input_handling,
            physics_update,
            ai_update,
            animation_update,
        ))
        .run();
}
```

### System Types

| System Type | Use Case | Example |
|------------|----------|---------|
| `Startup` | One-time initialization | Spawn player, load assets |
| `Update` | Per-frame logic | Movement, AI |
| `PostUpdate` | After physics | Animation blending |
| `Last` | Final frame cleanup | Save state |
| `Render` | Rendering (in render app) | Draw calls |

### System Ordering

Use `before`, `after`, and `SystemSet` for ordering:

```rust
use bevy::prelude::*;

app.add_systems(Update, (
    input_handling,
    physics_update,
    ai_update,
    animation_update,
).chain()); // Run in sequence

// Or with explicit ordering
app.add_systems(Update, (
    input_handling,
    physics_update,
).chain().after(input_handling));

// With run conditions
app.add_systems(Update, enemy_ai.run_if(in_state(GameState::Playing)));
```

### Query Patterns

```rust
// Immutable borrow - read position
fn read_positions(query: Query<&Transform>) {
    for transform in &query {
        println!("{:?}", transform.translation);
    }
}

// Mutable borrow - modify position
fn modify_positions(mut query: Query<&mut Transform>) {
    for mut transform in &mut query {
        transform.translation.y += 0.01;
    }
}

// With filter - only entities that have both components
fn movement_system(mut query: Query<(&mut Transform, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.translation += velocity.0 * 0.016;
    }
}

// With change detection - only if changed
fn on_change_system(
    query: Query<&Transform, Changed<Transform>>
) {
    for transform in &query {
        println!("Transform changed!");
    }
}
```

### Resources (Global State)

For global state that doesn't belong to entities:

```rust
#[derive(Resource)]
struct GameConfig {
    gravity: f32,
    movement_speed: f32,
}

#[derive(Resource, Default)]
struct Score(u32);

fn add_score(mut score: ResMut<Score>, to_add: u32) {
    score.0 += to_add;
}
```

---

## 3. Managing State in ECS

### Bevy States

Bevy provides a state machine system:

```rust
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(OnEnter(GameState::MainMenu), setup_menu)
        .add_systems(OnExit(GameState::MainMenu), cleanup_menu)
        .add_systems(Update, (
            menu_input.run_if(in_state(GameState::MainMenu)),
            game_update.run_if(in_state(GameState::Playing)),
            pause_input.run_if(in_state(GameState::Playing)),
        ))
        .run();
}
```

### State Transitions

```rust
fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::Playing);
}

fn pause_game(
    input: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            _ => {}
        }
    }
}
```

### SubStates

For orthogonal state combinations:

```rust
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
enum PlayingState {
    #[default]
    Exploring,
    Combat,
    Cutscene,
}
```

### Entity Lifetime with States

```rust
// Despawn when entering a state
commands.spawn((
    EntityType,
    DespawnOnEnter::<GameState>::GameOver,
));

// Only exist in specific state
commands.spawn((
    HudElement,
    StateScoped::<GameState>(GameState::Playing),
));
```

### Alternative: Resource-Based State

For simpler state management:

```rust
#[derive(Resource, Default)]
enum GameMode {
    #[default]
    SinglePlayer,
    Multiplayer,
}

fn check_mode(mode: Res<GameMode>) {
    match mode.as_ref() {
        GameMode::SinglePlayer => { /* ... */ }
        GameMode::Multiplayer => { /* ... */ }
    }
}
```

---

## 4. Performance Considerations

### Query Optimization

1. **Fetch only needed components**
   ```rust
   // Bad: fetches Transform but only uses Position
   fn bad_query(query: Query<(&Position, &Transform)>) {}
   
   // Good: only fetch what you need
   fn good_query(query: Query<&Position>) {}
   ```

2. **Use component filters**
   ```rust
   fn player_movement(
       query: Query<&Velocity, With<Player>>,
   ) {}
   ```

3. **Avoid Or patterns when possible**
   ```rust
   // Slower
   fn or_query(query: Query<Or<(With<A>, With<B>)>>) {}
   
   // Faster: separate queries
   fn split_queries(
       query_a: Query<&A>,
       query_b: Query<&B>,
   ) {}
   ```

### Parallelism

Bevy automatically parallelizes systems:

```rust
// These run in parallel
app.add_systems(Update, (
    system_a,
    system_b,
    system_c,
));

// Chain forces sequential execution
app.add_systems(Update, (
    system_a,
    system_b,
).chain());
```

### Component Layout

Group frequently accessed components together in your code to improve cache locality:

```rust
// Spawn with components in order they'll be accessed
commands.spawn((
    Position,     // Accessed together
    Velocity,     // Accessed together
    Renderable,   // Accessed separately
    Health,       // Rarely accessed
));
```

### Spatial Partitioning

For collision detection and spatial queries:

```rust
// Using bevy_spatial for automatic kd-tree
use bevy_spatial::{AutomaticUpdate, KDTree3, SpatialAccess};

#[derive(Component, Default)]
struct SpatialEntity;

fn setup_spatial(app: &mut App) {
    app.add_plugin(AutomaticUpdate::<SpatialEntity>::new()
        .with_frequency(Duration::from_secs_f32(0.1)));
}

// Query neighbors
fn find_nearby(tree: Res<KDTree3<SpatialEntity>>) {
    if let Some((pos, _entity)) = tree.nearest_neighbour(target) {
        // Found nearest
    }
}
```

Or use `bevy_quadtree` for 2D:

```rust
// bevy_quadtree for 2D games
use bevy_quadtree::{QuadTreePlugin, CollisionRect, CollisionCircle};
```

### Commands Buffer

Defer entity modifications to avoid borrow conflicts:

```rust
fn spawn_system(mut commands: Commands) {
    commands.spawn((
        Position::default(),
        Velocity::default(),
    ));
}

fn despawn_system(mut commands: Commands, to_despawn: Query<Entity, With<Dead>>) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}
```

### Batching

Bevy automatically batches draw calls for identical meshes/materials:

```rust
// Use AssetId to identify batchable items
#[derive(Component)]
struct BatchId(AssetId<Mesh>);
```

---

## 5. Asciicker-Specific Conversions

Based on analysis of asciicker's C++ codebase structure (game.h, render.cpp, physics.cpp), here's how to map specific systems.

### 5.1 Game Struct to ECS

**Current C++ Structure (inferred from codebase):**
```cpp
class Game {
    Player player;
    std::vector<Entity> entities;
    World* world;
    Renderer* renderer;
    Input* input;
    
    void update(float dt);
    void render();
    void load_level(const char* path);
};
```

**Bevy ECS Conversion:**

```rust
// Resources for game-wide state
#[derive(Resource)]
pub struct GameSettings {
    pub gravity: f32,
    pub movement_speed: f32,
    pub render_distance: f32,
}

#[derive(Resource, Default)]
pub struct GameStats {
    pub frame_count: u64,
    pub entity_count: usize,
}

// Component bundles for common entity types
#[derive(Bundle)]
pub struct PlayerBundle {
    position: Position,
    velocity: Velocity,
    player: Player,
    renderable: Renderable,
    collider: Collider,
}

// System to initialize game
fn init_game(mut commands: Commands) {
    // Spawn player
    commands.spawn(PlayerBundle {
        position: Position::new(0.0, 0.0, 0.0),
        velocity: Velocity::zero(),
        player: Player,
        renderable: Renderable::default(),
        collider: Collider::new(1.0, 2.0, 1.0),
    });
    
    // Initialize resources
    commands.insert_resource(GameSettings {
        gravity: 9.81,
        movement_speed: 5.0,
        render_distance: 100.0,
    });
}
```

### 5.2 Terrain Quadtree to Component+System

**Current C++ Structure:**
```cpp
class TerrainQuadtree {
    struct Node {
        AABB bounds;
        std::vector<TerrainChunk*> chunks;
        Node* children[4]; // NW, NE, SW, SE
    };
    Node* root;
    
    void insert(TerrainChunk* chunk);
    std::vector<TerrainChunk*> query(const AABB& area);
};
```

**Bevy Conversion Options:**

Option A: Keep as external resource (recommended for static terrain):
```rust
#[derive(Resource)]
pub struct TerrainQuadtree {
    root: QuadNode,
    max_depth: u32,
    chunk_size: u32,
}

#[derive(Clone)]
struct QuadNode {
    bounds: Aabb,
    chunks: Vec<Entity>,
    children: [Option<Box<QuadNode>>; 4],
    subdivided: bool,
}

fn terrain_culling_system(
    camera: Query<&Transform, With<Camera>>,
    mut terrain_query: Query<&mut TerrainRenderable>,
    quadtree: Res<TerrainQuadtree>,
) {
    let cam_pos = camera.single().translation;
    let view_distance = 100.0;
    
    // Query visible chunks from quadtree
    let visible = quadtree.query(Aabb::from_center_radius(
        cam_pos, view_distance
    ));
    
    // Update visibility
    for (mut renderable, chunk) in terrain_query.iter_mut().zip(&visible) {
        renderable.visible = visible.contains(&chunk);
    }
}
```

Option B: Use existing crate (bevy_quadtree):
```rust
use bevy_quadtree::{QuadTreePlugin, CollisionRect, CollisionCircle};

fn main() {
    App::new()
        .add_plugins(QuadTreePlugin::<
            (CollisionRect, GlobalTransform),
            40,  // max entities per node
            8,   // max depth
            100, 100, 0, 0, // world size
            20, 114514 // outlet/inlet ratio, id
        >::default())
        .run();
}

fn spatial_query(
    quadtree: Res<QuadTree<114514>>,
    mut gizmos: Gizmos,
) {
    // Query entities in radius
    let results = quadtree.query_radius(position, radius);
    for entity in results {
        // Process
    }
}
```

**Terrain Chunk Component:**
```rust
#[derive(Component)]
pub struct TerrainChunk {
    pub chunk_id: IVec3,
    pub lod_level: u32,
    pub vertex_count: u32,
}

#[derive(Component)]
pub struct TerrainVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub ascii_char: char,
    pub color: Color,
}

#[derive(Component)]
pub struct ChunkVisibility {
    pub is_visible: bool,
    pub distance_to_camera: f32,
}
```

### 5.3 BSP World to Spatial System

**Current C++ Structure (inferred):**
```cpp
class BSPWorld {
    struct Node {
        Plane partition;
        std::vector<Polygon*> front;
        std::vector<Polygon*> back;
        Polygon* leaf_data;
    };
    Node* root;
    
    void build();
    void find_leaves(const Ray& ray, std::vector<Leaf*>& results);
};
```

**Bevy Conversion:**

```rust
#[derive(Component)]
pub struct BSPNode {
    pub partition_plane: Plane,
    pub front_child: Option<Entity>,
    pub back_child: Option<Entity>,
    pub leaf_data: Option<BSPLeaf>,
    pub node_type: BSPNodeType,
}

#[derive(Component)]
pub struct BSPLeaf {
    pub leaf_id: u32,
    pub bounds: Aabb,
    pub polygons: Vec<Entity>,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BSPNodeType {
    Interior,
    Leaf,
}

#[derive(Resource)]
pub struct BSPWorld {
    pub root: Entity,
    pub nodes: Vec<BSPNode>,
    pub depth: u32,
}

// Ray casting system using BSP
fn bsp_raycast(
    bsp: Res<BSPWorld>,
    mut query: Query<&mut BSPNode>,
    ray: Ray,
) -> Option<RayHit> {
    let mut current = Some(bsp.root);
    let mut closest_hit = None;
    
    while let Some(node_entity) = current {
        let node = query.get(node_entity).ok()?;
        
        match node.node_type {
            BSPNodeType::Leaf => {
                // Check polygons in leaf
                if let Some(leaf) = &node.leaf_data {
                    // Ray-polygon intersection tests
                    closest_hit = check_leaf_intersections(leaf, ray, closest_hit);
                }
                break;
            }
            BSPNodeType::Interior => {
                // Traverse based on plane side
                let side = ray.direction.dot(node.partition_plane.normal);
                current = if side > 0.0 {
                    node.front_child
                } else {
                    node.back_child
                };
            }
        }
    }
    
    closest_hit
}

// Alternative: Use Bevy physics (Rapier) for collision
// For ASCII game, custom BSP may still be preferred for:
// - Exact control over partitioning
// - Memory efficiency
// - Specific visibility calculations
```

**Visibility System:**
```rust
#[derive(Component)]
pub struct VisibilityFlags {
    pub is_visible: bool,
    pub last_viewed: u64,
    pub distance_to_camera: f32,
}

fn update_visibility(
    camera: Query<&Transform, With<Camera>>,
    mut visibility: Query<(&Transform, &mut VisibilityFlags)>,
) {
    let cam_pos = camera.single().translation;
    
    for (transform, mut flags) in &mut visibility {
        let dist = transform.translation.distance(cam_pos);
        flags.is_visible = dist < settings.render_distance;
        flags.distance_to_camera = dist;
    }
}
```

### 5.4 6-Stage Render Pipeline

Based on typical ASCII renderers and asciicker's likely pipeline. The 6 stages would be:

| Stage | Purpose | Bevy System Set |
|-------|---------|-----------------|
| 1. Extract | Pull data from main world | `ExtractSchedule` |
| 2. Prepare | Set up vertex data, buffers | `RenderSystems::Prepare` |
| 3. Queue | Create render phases, batch items | `RenderSystems::Queue` |
| 4. Sort | Order by distance/depth | `RenderSystems::PhaseSort` |
| 5. Render | Execute draw calls | `RenderSystems::Render` |
| 6. Present | Output to screen | wgpu handles this |

**Custom Render Phase for ASCII:**

```rust
use bevy::{
    prelude::*,
    render::{
        render_phase::{PhaseItem, SortedRenderPhase, AddRenderCommand},
        render_resource::*,
        extract_component::ExtractComponent,
    },
};

// Custom phase for ASCII rendering
pub struct AsciiPhase;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Ascii3d;

pub struct AsciiRenderPlugin;

impl Plugin for AsciiRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SortedRenderPhasePlugin::<Ascii3d>::default());
        
        app.get_render_app().unwrap()
            .add_render_command::<Ascii3d, DrawAscii>();
    }
}

#[derive(Component, ExtractComponent, Clone, Copy, Default)]
pub struct AsciiChar {
    pub char: char,
    pub color: Color,
    pub background: Color,
}

#[derive(Component)]
pub struct AsciiInstance {
    pub transform: Mat4,
    pub char_data: AsciiChar,
}

// Render command
pub struct DrawAscii;

impl RenderCommand<Ascii3d> for DrawAscii {
    type Param = (
        SRes<RenderAssets<AsciiMesh>>,
        SRes<AsciiPipeline>,
    );
    
    fn render<'w>(
        _item: &dyn PhaseItem,
        _view: Entity,
        (_meshes, _pipeline): SYSTEM_PARAM<'w>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        // ASCII character rendering logic
        // Could render to terminal buffer or texture
        RenderCommandResult::Success
    }
}
```

**Integration with Bevy's Render App:**

```rust
fn main() {
    let mut app = App::new();
    
    // Main world plugins
    app.add_plugins(DefaultPlugins);
    app.add_plugin(AsciiRenderPlugin);
    
    // Configure render app
    app.get_render_app().unwrap()
        .add_systems(ExtractSchedule, extract_ascii_data)
        .add_systems(Render, (
            prepare_ascii_buffers.in_set(RenderSystems::Prepare),
            queue_ascii_meshes.in_set(RenderSystems::Queue),
            sort_ascii_phase.in_set(RenderSystems::PhaseSort),
            render_ascii.in_set(RenderSystems::Render),
        ));
}

fn extract_ascii_data(
    mut commands: Commands,
    query: Query<(Entity, &AsciiChar, &GlobalTransform)>,
) {
    for (entity, char_data, transform) in &query {
        commands.get_or_spawn(entity).insert(AsciiInstance {
            transform: transform.compute_matrix(),
            char_data: *char_data,
        });
    }
}
```

**Simplified Approach: Use Bevy 2D with custom font**

For asciicker, consider using Bevy's 2D rendering with a monospace font:

```rust
use bevy::{prelude::*, sprite::Sprite};

#[derive(Component)]
pub struct AsciiSprite {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
}

fn spawn_ascii_text(
    mut commands: Commands,
    assets: Res<FontAssets>,
) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::splat(16.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            texture: assets.ascii_texture.clone(),
            ..default()
        },
        AsciiSprite {
            character: '@',
            foreground: Color::WHITE,
            background: Color::BLACK,
        },
    ));
}

// Batch rendering system
fn update_ascii_sprites(
    mut query: Query<(&AsciiSprite, &mut Sprite)>,
) {
    for (ascii, mut sprite) in &mut query {
        // Update texture coordinates based on character
        // This could map to a texture atlas of ASCII characters
    }
}
```

---

## 6. Implementation Roadmap

### Phase 1: Foundation
1. Set up Bevy project with `Cargo.toml`
2. Create core components (Position, Velocity, etc.)
3. Implement basic game loop with systems
4. Set up state management

### Phase 2: Core Systems
5. Input handling system
6. Basic physics/movement
7. Player controller

### Phase 3: World Integration
8. Terrain quadtree as resource
9. BSP world structure
10. Spatial queries

### Phase 4: Rendering
11. ASCII render pipeline
12. Camera system
13. Visibility/culling

### Phase 5: Optimization
14. Query optimization
15. Batching
16. Profiling and tuning

### Suggested Project Structure

```
asciicker/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── player.rs
│   │   ├── terrain.rs
│   │   └── render.rs
│   ├── systems/
│   │   ├── mod.rs
│   │   ├── input.rs
│   │   ├── movement.rs
│   │   ├── physics.rs
│   │   └── render.rs
│   ├── resources/
│   │   ├── mod.rs
│   │   ├── game_state.rs
│   │   ├── terrain.rs
│   │   └── bsp_world.rs
│   └── plugins/
│       ├── mod.rs
│       ├── game.rs
│       └── ascii_render.rs
└── assets/
    └── fonts/
```

### Key Dependencies

```toml
[dependencies]
bevy = "0.14"  # Or latest stable
bevy_spatial = "0.8"  # For kd-tree spatial queries
# bevy_quadtree = "0.16"  # Alternative for 2D

[profile.dev]
opt-level = 1

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

---

## Summary

Converting from C++ OOP to Bevy ECS involves:

1. **Components**: Flat data structs, no methods
2. **Systems**: Functions that operate on component queries
3. **Resources**: Global singleton state
4. **States**: Enum-based state machines with transitions
5. **Commands**: Deferred entity modifications

For asciicker specifically:
- Game struct becomes Resources + initialization systems
- Terrain quadtree becomes a Resource with query systems
- BSP world becomes a spatial query system
- Render pipeline integrates with Bevy's render app

The key benefits: automatic parallelism, cache-friendly data layout, and composable entity definitions.

---

*Document Version: 1.0*
*Last Updated: 2026-02-20*
