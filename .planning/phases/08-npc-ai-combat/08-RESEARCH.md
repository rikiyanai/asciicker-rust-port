# Phase 08: NPC AI and Combat — Research

## C++ Source Analysis
- **Files:** 
  - `game.cpp`: Lines 7028-7331 (AI behavior), 7383-7495 (Melee combat), 3703-3845 (NPC Spawning)
  - `enemygen.cpp`: 330 LOC, full file (Spawn point management)
  - `world.cpp`: Ray intersection functions
- **Key Functions:**
  - `Animate()`: Integrated physics for both player and NPCs
  - `HitWorld()` / `HitTerrain()`: Raycasting for line-of-sight and targeting
  - `GetNearbyItems()` / `GetNearbyCharacters()`: Proximity queries (to be replaced by `SpatialGrid`)
  - `EnemyGen::LoadEnemyGens()`: Spawn point loading
- **Data Structures:**
  - `EnemyGen`: Spawn point configuration (pos, population, equipment weights)
  - `NPC_Human`: Extends `Character`, adds AI target and stuck state
  - `PhysicsIO`: NPC-specific physics state (used as Component in Rust)
- **Constants:**
  - `attack_us_per_frame`: Animation-driven timing
  - `hit_tested`: Flag for single-hit per attack
  - `15.0 / distance`: Knockback magnitude scaling
  - `3.0 units`: Melee weapon range

## Crate Dependencies
- `smallvec = "1.13"`: Used in `SpatialGrid` for efficient per-cell entity storage (Decision D1)
- `rand = "0.8"`: For randomized equipment rolls and AI decisions (replaces `fast_rand`)

## ECS Architecture
- **Components:**
  - `NpcBundle`: `Transform`, `NpcState`, `NpcEquipment`, `PhysicsIO`, `SpatialGridCell`, `SpriteRef`
  - `SpatialGridCell(i32, i32, i32)`: Caches current cell for dynamic entities
- **Resources:**
  - `SpatialGrid`: 3D spatial hash for dynamic entities (NPCs, items, projectiles)
  - `EnemyGenRegistry`: List of loaded spawn points from `.a3d`
- **Events:**
  - `NpcDeathEvent`: Triggers death animations and loot drops
  - `CombatHitEvent`: Used for damage application and knockback
- **Schedules:**
  - `PreUpdate`: `SpatialGrid` cleanup (if needed)
  - `FixedUpdate`: NPC AI behavior writing to `PhysicsIO`, followed by Physics simulation
  - `Update`: Animation state updates, state machine transitions
  - `PostUpdate`: `SpatialGrid` sync (reads `Transform`, updates `SpatialGrid` Resource)

## Cross-Phase Dependencies
- **Reads:**
  - Phase 5: `.a3d` world data (spawn points), `RuntimeWorld` BSP (for static raycasting)
  - Phase 6: `PhysicsIO` logic, `CharacterPlugin` animation/state machine framework
  - Phase 7: `SpriteRef` and visual quality improvements
- **Provides:**
  - `SpatialGrid` for Phase 9 item pickup and Phase 12 entity replication
  - NPC entities for Phase 10 HUD tracking (Minimap) and Phase 13 scripting

## Open Questions
- Should `SpatialGrid` use a fixed cell size based on `VISUAL_CELLS` (8.0)? (User confirmed: yes, but with coarse Y cells)
- Exact distribution of blood decals on terrain — how to port `PaintTerrain` efficiently?
