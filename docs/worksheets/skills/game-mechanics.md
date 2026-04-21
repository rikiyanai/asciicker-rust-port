---
name: game-mechanics
description: Use when working with game logic, character systems, equipment, AI, and combat mechanics in Asciicker.
---

# Skill: Game Mechanics

Core gameplay systems including characters, equipment, AI, and combat.

## Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `game.cpp` | ~11600 | Main game loop, character systems, AI |
| `game.h` | ~570 | Game struct, Human/Character definitions |
| `inventory.cpp` | ~3100 | Inventory system |
| `enemygen.cpp` | ~1150 | Enemy spawner logic |

## Core Structures

### Game (God Object)
Holds global state, input, and subsystems:
- `stamp` - Game timestamp
- `physics` - Player physics state
- `renderer` - Rendering state
- `player` - Human struct for player
- `inventory` - Item storage
- `input` - Accumulated input state

### Character
Base entity for players and NPCs:
- `sprite` - Visual representation
- `anim`, `frame` - Animation state
- `pos[3]`, `dir` - Position/facing
- `HP`, `MAX_HP` - Health
- `req` - SpriteReq for equipment lookup

### Human (extends Character)
Full player state:
- Equipment slots (weapon, shield, helmet, armor, mount)
- Stats (level, XP, HP, MP, speed, power)
- Protection values (hit, fire)
- Nutrition (vitamins, minerals, proteins, fats, carbs, water)
- Talk system for chat bubbles

## Equipment System

5D sprite array indexing:
```
player[color][armor][helmet][shield][weapon]
```

Enums define bounds:
- `ACTION`: NONE, ATTACK, FALL, DEAD, STAND
- `WEAPON`: NONE, REGULAR_SWORD, REGULAR_CROSSBOW
- `SHIELD`: NONE, REGULAR_SHIELD
- `HELMET`: NONE, REGULAR_HELMET
- `ARMOR`: NONE, REGULAR_ARMOR
- `MOUNT`: NONE, WOLF, BEE

## Input System

`Game::Input` accumulates events:
- `key[32]` - Keyboard state bitmap
- `contact[4]` - Mouse/touch contacts (0=mouse, 1-2=touch)
- `pad_*` - Gamepad state
- `api_move[3]` - External API input

Contact actions: NONE, KEYBCAP, PLAYER, TORQUE, FORCE, ITEM_LIST_CLICK, etc.

## AI/Combat

NPC behavior in `game.cpp`:
- Pathfinding toward targets
- Attack timing via `action_stamp`
- Hit detection and damage
- Blood/gore leak system
- Enemy revival from `enemygen`

## Known Traps

### TRAP-G01: Equipment Changes Sprite Instantly
`SetWeapon()`, `SetShield()`, etc. modify sprite lookup. Animation may desync if called mid-attack.

### TRAP-G02: Mount Changes Physics Size
MOUNT::WOLF and MOUNT::BEE use different collision radii. Dismount requires physics reinit.

### TRAP-G03: Server vs Local State
`Server::others[]` holds networked players. Local `player` struct is separate. State sync is manual.

### TRAP-G04: Inventory Index vs Item Pointer
`items_inrange[]` holds pointers, `inventory` holds indices. Mixing them causes crashes.

### TRAP-G05: Talk Boxes Are Temporarily Allocated
`Human::talk_box` points to pooled memory. Don't cache across frames.

## Port Considerations

- **Complexity:** ~15000+ lines across game/inventory/enemygen
- **Coupling:** Tightly coupled to physics, rendering, networking

---

## Bevy Mapping

### ECS / Non-ECS Mapping Table

Not everything in game.cpp maps to ECS. The table below is the authoritative guide:

#### Good ECS Candidates (use Components + Systems)

| C++ Construct | Bevy Target | Rationale |
|---------------|-------------|-----------|
| `Character` struct (base) | Entity with component bundle | Multiple instances (player + NPCs), per-entity state |
| `pos[3]`, `dir`, `yaw` | `Transform` component | Standard Bevy transform, queryable |
| `HP`, `MAX_HP`, combat state | `CombatState` component | Per-entity, queried by damage systems |
| `anim`, `frame`, animation | `AnimationState` component | Per-entity, queried by sprite resolve |
| Equipment slots (weapon, shield, helmet, armor) | `SpriteReq` component (single bundled component) | Queried by `query_character_sprites` |
| `ACTION` enum (NONE, ATTACK, FALL, DEAD, STAND) | `ActionState` enum component | Per-entity state machine, `match` arms |
| NPC type (creature vs human) | `NpcKind` enum component | Queryable variant, exhaustive match |
| `Input` accumulation | System in `Update` schedule | Reads Bevy input, writes to `PhysicsIO` |
| Target selection (nearest enemy) | System querying all `Character` entities | Natural ECS query pattern |

#### Poor ECS Candidates (use plain Rust structs/functions)

| C++ Construct | Bevy Target | Why NOT ECS |
|---------------|-------------|-------------|
| Character state transitions (`SetActionAttack`, `SetActionFall`) | `match` arms on `ActionState` enum | FSM with validation guards — enum + match is idiomatic Rust (AP-5) |
| Equipment 5D lookup `player[color][armor][helmet][shield][weapon]` | Static data table (`Resource`) | Shared lookup data, not per-entity. `SpriteReq` component bundles the 5 indices |
| AI stuck detection (reverse, perpendicular, jump, reset) | Plain Rust function per NPC per tick | Sequential per-NPC logic, not parallelizable across entities (AP-5) |
| Inventory grid + bitmask collision | Plain Rust struct (`Inventory`) with methods | Single-player UI data structure. Making slots into entities is over-granular (AP-4) |
| Combat hit detection (melee range scan) | Single system, NOT event-driven | Linear scan of `Query<&Transform, With<Character>>` — simple and correct |
| Ranged combat (crossbow raycast) | Plain Rust function called from combat system | One-off computation, not a per-frame query (AP-5) |
| Knockback impulse | Write to `PhysicsIO.x_impulse`/`y_impulse` | Not a separate physics event — direct field write |
| Nutrition (vitamins, minerals, etc.) | Fields on `PlayerStats` struct | Single-player data, not queryable by systems |
| Talk box / chat bubble | `Resource` or direct UI write | Temporary display data, not per-entity |
| `enemygen` spawning logic | Startup system or timer system | Runs periodically, spawns entities, but the logic itself is one function |

### Character Component Design

**WARNING: Do NOT decompose Character into many tiny components.**

The C++ `Character` struct has ~30 fields. Start with **3-4 fat components**:

| Component | Fields | Queried By |
|-----------|--------|------------|
| `ActionState` (enum) | Current action, transition timestamp | Animation, AI, combat |
| `SpriteReq` | color, armor, helmet, shield, weapon indices | `query_character_sprites` |
| `CombatState` | HP, MAX_HP, damage cooldown, blood/gore | Combat system, UI |
| `NavigationState` | target, stuck_counter, stuck_mode | AI system only |

Split further **only when a concrete system needs a subset** (AP-4: "don't split until you have a reason"). Premature splitting creates more components to synchronize and more opportunities for silent query mismatches.

### TRAP-G06: Silent ECS Query Mismatches

When spawning Character entities, **ALL required components must be present** or queries will silently skip the entity. No compiler error, no runtime error — the entity just doesn't appear.

**Mitigation:** Use Required Components (Bevy 0.15+):

```rust
#[derive(Component)]
#[require(Transform, AnimationState, ActionState, SpriteReq, CombatState)]
struct Character;
```

This ensures that spawning a `Character` entity without `AnimationState` produces a **compile-time error** instead of a silent runtime omission.

### TRAP-G07: Character Component Bundle — Single Spawner

Define a **single `spawn_character()` function** (or `impl Command`) that spawns ALL required components. Never spawn Character components piecemeal from multiple systems.

> **Note:** The signature below is illustrative. The canonical `spawn_character` definition is in Plan 06-02 (spawner-pattern), which defines the exact parameters, component set, and return type.

```rust
fn spawn_character(commands: &mut Commands, config: CharacterConfig) -> Entity {
    commands.spawn((
        Character,           // marker, triggers #[require] checks
        Transform::from_translation(config.position),
        ActionState::Stand,
        AnimationState::default(),
        SpriteReq::from_equipment(&config.equipment),
        CombatState::new(config.max_hp),
    )).id()
}
```

**Why:** Component discoverability (AP-1). "Where do I find what components a Character has?" — answer: `spawn_character()`. Not scattered across 5 systems.

### TRAP-G08: No Compile-Time Safety for Entity Composition

ECS in Rust loses the "if it compiles, it runs" guarantee for entity composition. After **ANY component refactoring**, run integration tests that:

1. Spawn a Character entity with all components via `spawn_character()`
2. Run one frame (`app.update()`)
3. Assert the entity was processed by physics, animation, AND rendering queries
4. Assert no `Query` returned zero results when it should have matched

```rust
#[test]
fn character_entity_processed_by_all_systems() {
    let mut app = App::new();
    // ... add plugins ...
    let entity = spawn_character(&mut app.world_mut(), test_config());
    app.update();
    // Verify entity has Transform updated by physics
    // Verify entity appears in SpriteQueue (rendered)
    // Verify entity's ActionState was processed
}
```

**Why:** Adding a new component requirement to a query silently excludes existing entities that lack it. This test catches that regression.

### Pattern: Enum Components for Entity Variants

Use **enum-valued components** instead of separate marker components for entity variants:

```rust
// CORRECT: Enum component
#[derive(Component)]
enum NpcKind {
    Creature,
    Human,
}

// WRONG: Separate marker components
#[derive(Component)] struct Creature;
#[derive(Component)] struct Human;
```

**Why:** Enum enables exhaustive `match` in systems. When a new variant is added (e.g., `NpcKind::Undead`), the compiler forces every `match` to handle it. Separate markers silently miss new variants in queries (AP-3).

### System Organization

Each Bevy Plugin should document all systems it registers:

```rust
impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app
            // Update: reads Bevy input, writes PhysicsIO forces
            .add_systems(Update, accumulate_player_input)
            // FixedUpdate: physics (see physics-system.md)
            // PostUpdate: sync physics output to Transform
            .add_systems(PostUpdate, sync_physics_to_transform)
            // PostUpdate: query character components, write SpriteQueue
            .add_systems(PostUpdate, query_character_sprites
                .after(sync_physics_to_transform));
    }
}
```

This makes system discoverability explicit (AP-2). A porter can read the Plugin impl to understand what runs when.
