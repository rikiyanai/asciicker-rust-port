> **STATUS: ACTIVE REFERENCE** — C++ codebase structural analysis, February 2026.

# Asciicker C++ Codebase Architecture Analysis

## Executive Summary

The Asciicker C++ codebase does **NOT** use Object-Oriented Programming (OOP) as its primary paradigm. Instead, it employs a **Data-Oriented Design (DOD)** approach with C-style procedural programming. The codebase explicitly documents this architectural choice in `game.h`:

> "The engine uses a C-style struct-based architecture (Data-Oriented Design) rather than heavy C++ OOP."

This analysis examines the architecture to understand how to properly port it to Rust, which offers multiple programming paradigms including OOP (via traits and trait objects), procedural programming, and data-oriented approaches.

---

## 1. Programming Paradigm Analysis

### 1.1 Primary Paradigm: Data-Oriented Design (DOD)

The codebase follows Data-Oriented Design principles:

- **No C++ classes**: The main codebase contains zero `class` definitions (only found in third-party libraries like imgui)
- **Struct-based data containers**: All major data types are C-style structs
- **Separation of data and behavior**: Data structures hold state; free functions operate on them
- **Cache-friendly memory layouts**: The codebase uses flat arrays and explicit memory management

### 1.2 Evidence from Codebase

From `game.h` lines 3-15:
```cpp
// =============================================================================
// Game Architecture - Main Header
// =============================================================================
// This file defines the core data structures for the game engine.
// The engine uses a C-style struct-based architecture (Data-Oriented Design)
// rather than heavy C++ OOP.
```

### 1.3 Inheritance Without Polymorphism

The codebase uses C++ struct inheritance but **NOT** virtual functions or runtime polymorphism:

```cpp
// world.cpp - BSP tree hierarchy (no virtual functions)
struct BSP_Node : BSP { BSP* bsp_child[2]; };
struct BSP_NodeShare : BSP_Node { Inst* head; Inst* tail; };
struct BSP_Leaf : BSP { Inst* head; Inst* tail; };
struct Inst : BSP { /* ... */ };
struct MeshInst : Inst { /* ... */ };
struct SpriteInst : Inst { /* ... */ };
struct ItemInst : Inst { /* ... */ };

// game.h - Character hierarchy (no virtual functions)
struct Character { /* base entity */ };
struct Human : Character { /* player character */ };
struct NPC_Creature : Character, ItemOwner {};
struct NPC_Human : Human, ItemOwner {};
```

**Key observation**: This is "inheritance for code reuse" not "polymorphism for abstraction." There are no `virtual` functions anywhere in the main codebase.

---

## 2. Core Data Structures

### 2.1 The "God Object" - Game Struct

The `Game` struct serves as a central container for global state (often called a "God Object" in OOP):

```cpp
// game.h lines 317-544
struct Game
{
    uint64_t stamp;
    bool main_menu;
    
    // Rendering state
    int fps_window_pos;
    uint64_t fps_window[100];
    int font_size[2];
    int render_size[2];
    float zoom;
    
    // Gameplay state
    bool fly_mode;
    bool perspective;
    bool blood;
    
    // Subsystems (composition)
    Renderer* renderer;
    Physics* physics;
    
    // Player and inventory
    Human player;
    Inventory inventory;
    
    // Input subsystem
    Input input;
    
    // Menu state
    int menu_stack[4];
    int menu_depth;
};
```

**Architecture Pattern**: This is a **composition-based** design where the Game struct contains pointers to subsystems (Renderer, Physics) rather than inheriting from them.

### 2.2 Entity Component-like Structures

#### Character/Human Entities

```cpp
// game.h - Base character
struct Character
{
    Sprite* sprite;
    int anim;
    int frame;
    float pos[3];
    float dir;
    float impulse[2];
    
    bool SetActionNone(uint64_t stamp);
    bool SetActionAttack(uint64_t stamp);
    bool SetActionFall(uint64_t stamp);
    // ... methods (inline functions in header)
    
    Character* prev;  // Linked list
    Character* next;  // Linked list
    
    SpriteReq req;
    Character* master;
    Character* target;
};

// game.h - Human extends Character
struct Human : Character
{
    char name[32*4];
    int level;
    int cur_xp, max_xp;
    int cur_hp, max_hp;
    int cur_mp, max_mp;
    // ... equipment and stats
    
    bool SetWeapon(int w);
    bool SetShield(int s);
    // ...
};
```

#### SpriteReq - Parameter Object Pattern

```cpp
// game.h - Equipment state for sprite selection
struct SpriteReq
{
    enum KIND { HUMAN = 0, WOLF = 1, BEE = 2 };
    KIND kind;
    int mount;   // MOUNT::NONE, WOLF, BEE
    int action;  // ACTION::NONE, ATTACK, FALL, DEAD
    int armor;   // ARMOR index
    int helmet;  // HELMET index
    int shield;  // SHIELD index
    int weapon;  // WEAPON index
};
```

This is a **parameter object** that decouples equipment state from character state for efficient sprite lookups in the rendering pipeline.

### 2.3 Terrain System - Quadtree

```cpp
// terrain.h - Public API (opaque pointers)
struct Terrain;
struct Patch;

// Functions operate on opaque pointers
Terrain* CreateTerrain(int z = -1);
void DeleteTerrain(Terrain* t);
Patch* GetTerrainPatch(Terrain* t, int x, int y);
void QueryTerrain(Terrain* t, /* ... */);
```

This uses the **opaque pointer pattern** (similar to PIMPL idiom) to hide implementation details.

### 2.4 World System - BSP Tree

```cpp
// world.h - Public API (opaque pointers)
struct World;
struct Mesh;
struct Inst;

World* CreateWorld();
void RebuildWorld(World* w, bool boxes = false);
Inst* CreateInst(World* w, /* ... */);
void QueryWorld(World* w, /* ... */);
```

---

## 3. Design Patterns Used

### 3.1 Opaque Pointer Pattern (PIMPL)

**Purpose**: Hide implementation details, reduce compilation dependencies

**Examples**:
- `struct Renderer;` in render.h (full definition in render.cpp)
- `struct Terrain;` in terrain.h
- `struct World;` in world.h
- `struct Physics;` in physics.h

**Rust equivalent**: Use traits with private implementations or enum with private variants.

### 3.2 Parameter Object Pattern

**Purpose**: Group related parameters for function calls

**Example**: `SpriteReq` groups equipment state for sprite lookup:
```cpp
Sprite* GetSprite(const SpriteReq* req, int clr = 0);
```

**Rust equivalent**: Structs passed as references.

### 3.3 Linked List for Entity Management

**Purpose**: Dynamic entity storage with O(1) insertion/removal

**Example**:
```cpp
struct Character
{
    Character* prev;
    Character* next;
};

struct Server
{
    Human* head;
    Human* tail;
};
```

**Rust equivalent**: Vec with swap-remove, or linked list crate.

### 3.4 Data-Driven Sprite Arrays (5D Arrays)

**Purpose**: O(1) sprite selection based on equipment

**Example** from sprite_constants.h:
```cpp
// 5D array: player[color][armor][helmet][shield][weapon]
Sprite* player[2][ARMOR::SIZE][HELMET::SIZE][SHIELD::SIZE][WEAPON::SIZE];
```

This is a **compile-time computed lookup table** - a DOD pattern that prioritizes performance over abstraction.

### 3.5 Spatial Indexing: Quadtree and BSP

**Terrain**: Quadtree for patch-based terrain
**World**: BSP tree for instance culling and raycasting

These are classic **spatial data structures** used for efficient queries.

---

## 4. Data vs. Behavior Relationship

### 4.1 Data-Behavior Separation

The codebase clearly separates data structures from behavior:

| Data (Structs) | Behavior (Functions) |
|----------------|----------------------|
| `Game` | `CreateGame()`, `DeleteGame()` |
| `Human` | `SetWeapon()`, `SetArmor()` |
| `Terrain` | `CreateTerrain()`, `QueryTerrain()` |
| `World` | `CreateWorld()`, `RebuildWorld()` |
| `Renderer` | `Render()`, `ProjectCoords()` |

### 4.2 No Encapsulation

Data fields are publicly accessible:
```cpp
struct Character
{
    Sprite* sprite;  // Public - no private fields
    int anim;        // Directly readable/writable
    float pos[3];   // Raw arrays, no accessors
};
```

### 4.3 Inline Methods

Some behavior is defined inline in headers (like simple setters):
```cpp
struct Input
{
    bool IsKeyDown(int k)
    {
        return (key[k >> 3] & (1 << (k & 7))) != 0;
    }
};
```

This is a **performance optimization** - these are tiny functions that benefit from inlining.

---

## 5. Key Subsystem Analysis

### 5.1 Renderer Subsystem

```cpp
// render.h - Public API
struct AnsiCell { uint8_t fg, bk, gl, spare; };
struct MatCell { uint8_t fg[3], bg[3]; uint8_t gl; uint8_t flags; };
struct Material { MatCell shade[4][16]; int mode; };
struct Renderer;

Renderer* CreateRenderer(uint64_t stamp);
void Render(Renderer* r, /* ... */);
```

**Rendering Pipeline** (6 stages):
1. Clear
2. Terrain
3. World
4. Shadow
5. Reflection
6. Resolve

The renderer uses template-based "duck typing" for shaders:
```cpp
// render.cpp comment
// Rasterize<Sample, Shader> uses compile-time duck typing — any Shader that
// implements Blend(Sample*, float z, float bc[3]) can rasterize triangles.
```

### 5.2 Physics Subsystem

```cpp
// physics.h - IO structure pattern
struct PhysicsIO
{
    // INPUT: Set by game before Animate()
    float x_force, y_force, z_force;
    float torque;
    float water;
    bool jump;
    
    // OUTPUT: Set by physics during Animate()
    float pos[3];
    float yaw;
    float player_dir;
    int player_stp;
    bool grounded;
};

int Animate(Physics* phys, uint64_t stamp, PhysicsIO* io, 
            const SpriteReq* req, bool me);
```

This is an **input/output parameter pattern** - the game fills input fields, physics updates output fields.

### 5.3 Network Subsystem

```cpp
// network.h - Binary protocol structures
#pragma pack(push,1)
struct STRUCT_REQ_JOIN { uint8_t token; char name[31]; };
struct STRUCT_BRC_POSE { uint8_t token; uint8_t anim; float pos[3]; /* ... */ };
#pragma pack(pop)
```

Uses **binary serialization** with no padding for network efficiency.

---

## 6. Implications for Rust Port

### 6.1 Recommended Approach: Data-Oriented + ECS

Given the DOD nature of the C++ codebase, a Rust port should:

1. **Use structs with data fields** - Not OOP classes
2. **Prefer composition over inheritance** - Like the `Game` struct with `renderer*` and `physics*` pointers
3. **Use traits for abstraction** - Where C++ uses function pointers or opaque handles
4. **Consider ECS for entities** - The Character/Human hierarchy maps well to ECS components

### 6.2 Direct Mappings

| C++ Pattern | Rust Equivalent |
|-------------|------------------|
| `struct X` | `struct X` |
| `struct X : Y` | `struct X { inner: Y }` or composition |
| `virtual void f()` | `trait X { fn f(&self); }` |
| `X*` | `&X` or `Box<X>` or `Rc<X>` |
| Opaque `struct X;` | `pub struct X(NonExhaustive)` or enum |
| Global `extern X` | `lazy_static` or `std::sync::OnceLock` |

### 6.3 Alternative: OOP with Traits

If a more idiomatic Rust OOP approach is desired:

```rust
// Trait-based polymorphism (if needed)
trait Entity {
    fn position(&self) -> [f32; 3];
    fn set_position(&mut self, pos: [f32; 3]);
}

// Composition over inheritance
struct Game {
    renderer: Box<dyn Renderer>,
    physics: Box<dyn Physics>,
    // ...
}
```

However, this would diverge from the C++ codebase's DOD philosophy.

---

## 7. Summary

### 7.1 Architecture Classification

| Aspect | Finding |
|--------|---------|
| **Primary Paradigm** | Data-Oriented Design (DOD) |
| **Secondary Paradigm** | Procedural Programming |
| **OOP Usage** | None (no classes, no virtual functions) |
| **Inheritance** | Struct inheritance for code reuse only |
| **Polymorphism** | None (no runtime dispatch) |
| **Data/Behavior** | Separated (structs hold data, functions operate on them) |

### 7.2 Main Components

1. **Game** - Central container (God object)
2. **Character/Human** - Player and NPC entities (linked list)
3. **Terrain** - Quadtree-based heightmap system
4. **World** - BSP-tree-based instance management
5. **Renderer** - Software ASCII rasterizer
6. **Physics** - Character movement and collision
7. **Inventory** - Item management system
8. **Network** - Binary protocol for multiplayer

### 7.3 Design Patterns

- Opaque Pointer Pattern (PIMPL)
- Parameter Object Pattern
- Linked List for Entities
- 5D Lookup Tables (compile-time)
- Spatial Indexing (Quadtree, BSP)
- Input/Output Parameter Pattern

### 7.4 Conclusion

The Asciicker C++ codebase is a **textbook example of Data-Oriented Design** in game development. It prioritizes:
- **Performance** over abstraction
- **Data layout** over encapsulation
- **Flat memory** over object hierarchies

For the Rust port, maintaining this DOD philosophy would be most faithful to the original design. However, Rust's type system and ownership model can provide additional safety guarantees while preserving the performance characteristics.

---

*Analysis performed on: asciicker-Y9-2 codebase*
*Generated: 2026-02-19*
