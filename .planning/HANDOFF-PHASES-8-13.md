# Handoff: Plan Phases 8-13 for Asciicker Rust Port

**Created:** 2026-02-24
**Purpose:** Provide all context needed for an agent to write GSD-format plan files for Phases 8-13.
**Deliverable:** Plan `.md` files in `.planning/phases/` directories + updated `ROADMAP.md`

---

## YOUR TASK

You must create detailed execution plans for **6 new phases (8-13)** following the exact format and conventions used by the existing plans in `.planning/phases/`. You are NOT executing these plans — you are ONLY writing the plan files.

### What to produce:

1. **Phase directories** — create these under `.planning/phases/`:
   - `08-npc-ai-combat/`
   - `09-inventory-items/`
   - `10-ui-hud-interaction/`
   - `11-full-menu-system/`
   - `12-full-networking/`
   - `13-npc-scripting/`

2. **Plan files** — YAML frontmatter + markdown body, one per plan (see FORMAT section below)

3. **Research files** — one `XX-RESEARCH.md` per phase directory

4. **ROADMAP.md update** — append Phase 8-13 entries to `.planning/ROADMAP.md`

5. **PROJECT.md update** — add new requirement IDs for Phases 8-13

---

## ARCHITECTURAL DECISIONS (ALREADY MADE — DO NOT CHANGE)

These decisions were made by the user. They are non-negotiable constraints.

### Decision 1: Two-Tier Spatial System (Static BSP + Dynamic SpatialGrid)

The BSP tree stays **read-only after load time**. Dynamic entities (NPCs, items, projectiles) use a separate `SpatialGrid` Resource.

```rust
#[derive(Resource)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32, i32), SmallVec<[Entity; 8]>>,
    cell_size: f32,
}
```

- **3D grid** indexed by `(x, y, z)` with coarse Y cell size (the world has bridges/multi-level terrain)
- O(1) insert/remove — entity changes cell when Transform moves
- Each dynamic entity gets a `SpatialGridCell(i32, i32, i32)` component caching its current cell
- One sync system in `PostUpdate`: if Transform changed cell → update component + grid Resource
- Proximity queries: check 3x3x3 = 27 neighboring cells
- Raycasting: BSP for static occlusion first, then DDA grid walk for dynamic entity hits

**Where this matters:**
- Phase 8 (NPC spawning, AI proximity, combat hit detection)
- Phase 9 (item pickup proximity, item world spawning)
- Phase 12 (entity replication — grid membership determines relevance)

**The BSP is NEVER rebuilt at runtime.** This departs from the C++ approach (which calls CreateInst/AttachInst to rebuild BSP on spawn). The SpatialGrid replaces that functionality with better performance.

### Decision 2: NPC Physics as ECS Components (not Resource)

The player's physics state is a single `PhysicsIO` Resource (already built in Phase 6). NPCs each get their own `PhysicsIO` as an **ECS Component** on their entity. The physics step system runs per-NPC in FixedUpdate.

### Decision 3: Departing from C++ is OK for Performance

The user explicitly stated: departing from C++ patterns is fine as long as UX fidelity (what the player sees and experiences) is intact. Performance improvements and better Rust/ECS patterns are encouraged.

### Decision 4: NPC Scripting — Defer Decision

Phase 13 (NPC Scripting) is a decision point. Write plans for the Lua option (mlua crate) as the default, but note in the plan that WASM (wasmtime) and hardcoded-Rust are alternatives. The user will choose at execution time.

---

## PHASE BREAKDOWN (WHAT EACH PHASE MUST CONTAIN)

### Phase 8: NPC AI and Combat (~4 plans)

**Goal:** Enemies spawn in the world, have AI behavior, and the player can fight them.

**Plan 08-01: SpatialGrid + Raycasting Infrastructure**
- Implement `SpatialGrid` Resource (3D uniform spatial hash, HashMap-based)
- `SpatialGridCell` component on dynamic entities
- Sync system: `PostUpdate`, reads `Transform`, updates grid
- Proximity query API: `nearby_entities(pos, radius) -> Vec<Entity>`
- Ray-vs-grid query: DDA cell walk, returns first hit entity
- Raycasting through BSP (Plucker coordinates) for static world — port `HitWorld`/`HitTerrain` from C++ physics
- ~200 LOC estimated
- C++ reference: world.cpp ray intersection, game.cpp:GetNearbyItems

**Plan 08-02: EnemyGen Spawn System**
- Read spawn points from .a3d world data (already parsed in Phase 2/5)
- `NpcBundle` with: Transform, NpcState, NpcEquipment, PhysicsIO (component), SpatialGridCell, SpriteRef
- Randomized equipment from probability distributions (0-10 scale per piece — see C++ enemygen.cpp)
- Spawn NPCs at world load, register in SpatialGrid
- C++ reference: enemygen.cpp (330 LOC)

**Plan 08-03: NPC AI Behavior**
- Target selection: chase nearest player / return to spawn / idle
- 4-phase stuck detection: reverse → go around → jump → reset position
- Buddy/companion following (follow player at offset)
- Avoidance force: deflect away from nearby NPCs (SpatialGrid proximity query)
- AI runs in `Update` schedule, writes to NPC's `PhysicsIO` component
- C++ reference: game.cpp:7028-7331 (~800 LOC)

**Plan 08-04: Melee Combat + Death/Respawn**
- Animation-driven hit detection: frame 21 of attack animation = hit frame
- Damage calculation, knockback impulse (15.0/distance)
- Death handling: play death animation, remove from SpatialGrid, despawn after delay
- Blood decals on terrain (write to terrain material cells)
- Player death → respawn timer → position selection → state reset
- C++ reference: game.cpp:7383-7495 (~600 LOC)

**Dependencies:** Phase 8 depends on Phase 7 completion (specifically 07-04 for sprite rendering quality).

### Phase 9: Inventory and Items (~4 plans)

**Goal:** Items exist in the world, can be picked up, equipped, consumed.

**Plan 09-01: ItemProto/Item Type Catalog**
- Item type definitions: weapons (sword, crossbow, mace, hammer, axe, flail), shields, helmets, armor
- Consumables: food (12 types), drinks (3), potions (7), rings, accessories
- `ItemProto` component with stats, sprite reference, grid size, item category
- Item database as a Resource (loaded from data, not hardcoded)
- C++ reference: inventory.h (234 LOC)

**Plan 09-02: Grid-Based Inventory UI**
- 8x20 cell grid, bitmask collision detection for item placement
- Scroll animation, focus navigation (keyboard/mouse)
- Render inventory overlay on AsciiCellGrid using Font1 (built in 07-04)
- C++ reference: inventory.cpp (759 LOC)

**Plan 09-03: Item World Interaction**
- Pickup: proximity query via SpatialGrid → transfer to inventory
- Drop: remove from inventory → spawn entity at player position → register in SpatialGrid
- Use/consume: trigger effect (heal, buff, etc.), play animation
- `GetNearbyItems` query using SpatialGrid (NOT BSP — items are dynamic entities)
- C++ reference: game.cpp:5468-5920 (~500 LOC)

**Plan 09-04: Equipment Lifecycle**
- Equip/unequip updates 5D sprite lookup (already built in Phase 6)
- Consume animations
- Contact-based drag-and-drop within inventory grid
- C++ reference: game.cpp (various equipment sections)

**Dependencies:** Phase 9 depends on Phase 8 (needs SpatialGrid from 08-01, NPC entities for loot drops).

### Phase 10: UI/HUD and Interaction (~4 plans)

**Goal:** Player can see health, interact with the world via mouse, and chat.

**Plan 10-01: HPBar + MPBar**
- 4-row health/mana display using Font1 set_cell() on AsciiCellGrid
- Percentage text, dynamic width based on max HP/MP
- Position: bottom-left corner of screen
- C++ reference: game.cpp HUD rendering sections

**Plan 10-02: TalkBox (Chat UI)**
- Chat input with CP437 conversion
- Word wrap, cursor, scrollable bordered frame
- Per-character talk bubbles with fade-out timer
- Render as overlay on AsciiCellGrid
- C++ reference: game.cpp chat/talk sections

**Plan 10-03: Minimap**
- 32x16 terrain sampling in top-right corner
- Height-based coloring (green low → brown high → white peaks)
- NPC dots: red = enemy, green = buddy
- Player direction arrow
- C++ reference: game.cpp minimap rendering

**Plan 10-04: Screen-to-World Unprojection + Damage Floaters**
- `UnprojectCoords2D`/`UnprojectCoords3D` — inverse of the projection matrix
- Mouse click → world position for targeting, item interaction
- Damage floaters: numbers that float up and fade out
- Debug info overlay (FPS, position, entity count)
- C++ reference: game.cpp unprojection functions

**Dependencies:** Phase 10 depends on Phase 8 (needs NPCs for minimap dots, combat for damage floaters) and Phase 9 (needs items for interaction targeting).

### Phase 11: Full Menu System (~3-4 plans)

**Goal:** Complete menu hierarchy with settings persistence.

**Plan 11-01: Menu State Machine + Navigation**
- Hierarchical menu tree: VIDEO → zoom/fullscreen/perspective/blood, CONTROLS → keyboard/mouse/touch/gamepad, MUTE, EXIT
- Keyboard and mouse navigation
- Extends the skeleton from Phase 7 Plan 07-02
- C++ reference: mainmenu.cpp (2,919 LOC — structure and flow)

**Plan 11-02: Menu Rendering + Transitions**
- Background sprite with palette animation
- Dither fade transitions between screens
- Render menus on AsciiCellGrid using Font1
- C++ reference: mainmenu.cpp rendering sections

**Plan 11-03: Level Selection + Settings Persistence**
- Level selection screen, .a3d file listing and loading flow
- Settings save/load (config file, serde)
- Apply settings at runtime (resolution, audio volume, controls)

**Plan 11-04: Gamepad Configuration (optional)**
- Visual mapping screen for gamepad buttons
- C++ reference: gamepad.cpp (2,318 LOC) — this is large and may be deferred
- Mark as OPTIONAL in the plan

**Dependencies:** Phase 11 depends on Phase 7 (07-02 menu skeleton). Does NOT depend on 8-10.

### Phase 12: Full Networking (~5 plans)

**Goal:** Authoritative server with entity replication, combat protocol, lag compensation.

**Plan 12-01: Snapshot Replication**
- Baseline/delta entity replication via bevy_replicon
- Replicated components: position, anim, frame, action, mount, sprite, HP, state_flags, authoritative_tick
- C++ reference: netplay snapshot system

**Plan 12-02: Combat Network Protocol**
- SWING → BRC_SWING → target validates range → DAMAGE → BRC_DAMAGE → DEATH → BRC_DEATH → RESPAWN → BRC_RESPAWN
- Server-authoritative hit validation
- C++ reference: game.cpp combat networking

**Plan 12-03: Item Replication**
- 8 mutation kinds: respawn_reset, pickup, drop, owner_set/clear, consume, equip_set/clear
- Server-authoritative item ownership
- SpatialGrid sync across server/client (server is authority)

**Plan 12-04: Input-Based Movement**
- Client sends normalized input + yaw at 30Hz
- Server runs physics authoritatively
- Client-side prediction with server reconciliation
- C++ reference: netplay input system

**Plan 12-05: Lag Compensation**
- Ping measurement at 10Hz
- Remote player interpolation via target_pos[] ring buffer
- Smoothing for jitter
- C++ reference: netplay interpolation

**Dependencies:** Phase 12 depends on Phase 8 (NPCs), Phase 9 (items), Phase 10 (HUD). Extends Phase 7 Plan 07-03 networking skeleton.

### Phase 13: NPC Scripting (~3 plans)

**Goal:** User-extensible NPC behavior via embedded scripting.

**DEFAULT OPTION: Lua via mlua crate** (user will confirm at execution time)

**Plan 13-01: Lua Runtime Integration**
- Embed Lua 5.4 via `mlua` crate
- Sandboxed execution environment (no filesystem/network access)
- Script loading from asset directory
- C++ reference: game.cpp V8 bridge (65KB shared memory approach — we use mlua's native Rust↔Lua FFI instead)

**Plan 13-02: NPC Script API**
- Expose to Lua: entity position, health, target, nearby_entities, move_to, attack, speak
- Callback hooks: on_spawn, on_tick, on_damaged, on_death, on_player_nearby
- Read-only world queries (terrain height, BSP raycast result)

**Plan 13-03: Script Hot-Reload + Error Handling**
- Watch script files for changes, reload without restart
- Script errors logged but never crash the game
- Per-NPC script assignment via component

**ALTERNATIVES (note in plan, do not write separate plans):**
- WASM via wasmtime: better sandboxing, heavier runtime, 3 plans equivalent effort
- Hardcoded Rust: zero plans (covered by Phase 8 AI), but loses user extensibility

**Dependencies:** Phase 13 depends on Phase 8 (NPC entities and AI framework).

---

## CRITICAL PATH

```
Phase 7 (current, planned) → tech demo
     ↓
Phase 8: NPC + Combat → enemies, fighting  [REQUIRED for playability]
     ↓
Phase 9: Inventory → items, equipment      [REQUIRED for playability]
     ↓
Phase 10: HUD + Interaction → UI           [REQUIRED for playability]
     ↓
     === PLAYABLE SINGLE-PLAYER GAME ===
     ↓
Phase 11: Menus → polish (parallel with 8-10, only depends on 7)
Phase 12: Networking → multiplayer (depends on 8+9+10)
Phase 13: Scripting → extensibility (depends on 8)
```

Phase 11 can be planned/executed in parallel with 8-10 since it only depends on Phase 7.

---

## PLAN FILE FORMAT

Every plan file MUST follow this exact format. Study existing plans in `.planning/phases/07-game-systems/` for reference.

### YAML Frontmatter (required fields):

```yaml
---
phase: XX-phase-name        # e.g., "08-npc-ai-combat"
plan: NN                    # e.g., 01, 02, 03
type: execute
wave: N                     # execution wave within phase (1 = first)
depends_on: ["YY-ZZ"]      # list of plan IDs this depends on
files_modified:             # every file this plan creates or modifies
  - engine-port/src/path/to/file.rs
autonomous: true            # always true for these phases
requirements:               # requirement IDs from PROJECT.md
  - REQ-ID

must_haves:
  truths:                   # boolean assertions that must be true when done
    - "assertion 1"
    - "assertion 2"
  artifacts:                # files that must exist with minimum content
    - path: "engine-port/src/path/file.rs"
      provides: "description of what it provides"
      min_lines: 50
  key_links:                # integration points between files
    - from: "file A"
      to: "file B or crate"
      via: "how they connect"
      pattern: "grep pattern to verify"
---
```

### Markdown Body (required sections):

```markdown
<objective>
One paragraph: what this plan builds, why it matters, what files it produces.
</objective>

<execution_context>
@/Users/r/.claude/get-shit-done/workflows/execute-plan.md
@/Users/r/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/XX-phase-name/XX-RESEARCH.md

Key references:
- relevant C++ source files and line ranges
- relevant crate documentation
- relevant existing Rust modules
</context>

<tasks>
  <task id="X.1" title="Task Title" est="S|M|L">
    Detailed description of what to implement.
    Include specific function signatures, struct definitions, constant values.
    Reference C++ source locations.
  </task>
  <!-- more tasks -->
</tasks>

<verification>
  <check type="unit_test">Description of test</check>
  <check type="integration_test">Description of test</check>
  <check type="compile">cargo build must succeed</check>
  <check type="runtime">Description of runtime check</check>
</verification>

<risks>
  <risk id="RXX" severity="HIGH|MEDIUM|LOW" mitigation="how to handle">
    Description of risk
  </risk>
</risks>

<notes>
Any additional context, warnings, or cross-references.
</notes>
```

---

## RESEARCH FILE FORMAT

Each phase directory needs an `XX-RESEARCH.md` file. Format:

```markdown
# Phase XX: Phase Name — Research

## C++ Source Analysis
- File: path, LOC count, key functions
- Data structures and their Rust equivalents
- Constants and magic numbers

## Crate Dependencies
- What new crates are needed (with version pins)
- API surface used

## ECS Architecture
- New Components, Resources, Events
- System schedule placement (Update, FixedUpdate, PostUpdate)
- System ordering constraints

## Cross-Phase Dependencies
- What this phase reads from prior phases
- What this phase provides to later phases

## Open Questions
- Any unresolved design decisions (flag for user)
```

---

## REQUIREMENT ID CONVENTIONS

Existing requirement prefixes in PROJECT.md:
- FOUND-XX (foundation)
- ASSET-XX (asset parsers)
- RAST-XX (rasterizer)
- GPU-XX (GPU output)
- PIPE-XX (pipeline)
- PHYS-XX (physics)
- CHAR-XX (character)
- AUD-XX (audio)
- NET-XX (networking)
- MENU-XX (menu)
- VFX-XX (visual effects)
- WTHR-XX (weather)

New requirement prefixes for Phases 8-13:
- NPC-XX (Phase 8: NPC AI and combat)
- ITEM-XX (Phase 9: inventory and items)
- HUD-XX (Phase 10: UI/HUD and interaction)
- FMENU-XX (Phase 11: full menu system — "FMENU" to avoid collision with MENU)
- FNET-XX (Phase 12: full networking — "FNET" to avoid collision with NET)
- SCRIPT-XX (Phase 13: NPC scripting)

---

## FILES TO READ BEFORE STARTING

These files contain critical context. Read them ALL before writing any plans.

1. `.planning/ROADMAP.md` — current phase statuses, format for adding new phases
2. `.planning/PROJECT.md` — existing requirements, format for adding new ones
3. `.planning/phases/07-game-systems/07-01-PLAN.md` — example plan (audio, simple)
4. `.planning/phases/07-game-systems/07-04-PLAN.md` — example plan (visual quality, complex)
5. `.planning/phases/06-physics-and-character/06-01-PLAN.md` — example plan (physics)
6. `.planning/phases/07-game-systems/07-RESEARCH.md` — example research file
7. `.planning/RISK-ASSESSMENT.md` — existing risks R01-R62 (continue numbering from R63)
8. `engine-port/src/game/mod.rs` — current game module (what exists)
9. `engine-port/src/physics/mod.rs` — current physics module (PhysicsIO Resource)
10. `engine-port/src/render/pipeline.rs` — rendering pipeline (SampleBuffer, AsciiCellGrid)

### C++ Source Reference (for porting accuracy):

11. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/game.cpp` — 14,226 LOC, the big one
12. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/inventory.h` — item type definitions
13. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/inventory.cpp` — inventory grid logic
14. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/enemygen.cpp` — NPC spawn system
15. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/mainmenu.cpp` — menu system
16. `/Users/rikihernandez/Downloads/Aciicker-Y9-2/gamepad.cpp` — gamepad config

### Skill Packs (condensed C++ subsystem knowledge):

17. `docs/worksheets/skills/game-mechanics.md` — character, combat, AI reference
18. `docs/worksheets/skills/physics-system.md` — collision, forces, constants
19. `docs/worksheets/skills/engine-render.md` — rendering pipeline reference
20. `docs/worksheets/skills/world-loading.md` — BSP, terrain, .a3d format

---

## CONSTRAINTS AND WARNINGS

1. **Do NOT modify any existing plan files** — only create new ones
2. **Do NOT create Phase 8+ code** — only plan files and research docs
3. **Risk IDs continue from R63** (R01-R62 already exist)
4. **Bevy version is 0.18.0** — all crate compatibility must target this
5. **The BSP is read-only at runtime** — never plan to call BSP rebuild
6. **SpatialGrid is the dynamic entity index** — all NPC/item proximity uses this
7. **PhysicsIO is a Resource for player, Component for NPCs** — this is decided
8. **Font1 system exists** (built in 07-04) — all UI text renders through it
9. **Every plan must have `must_haves` with verifiable truths** — no vague success criteria
10. **files_modified must be exhaustive** — list every file the plan touches
11. **Cross-plan dependencies must be explicit** in `depends_on` — no implicit ordering
12. **C++ line numbers** — include specific line ranges when referencing game.cpp (it's 14K lines, "game.cpp" alone is insufficient)

---

## EXECUTION ORDER FOR THE PLANNING AGENT

1. Read all 20 files listed in "FILES TO READ BEFORE STARTING"
2. Write research files (XX-RESEARCH.md) for each phase — this forces you to study the C++ source
3. Write plan files in phase order (08 → 09 → 10 → 11 → 12 → 13)
4. Update ROADMAP.md with Phase 8-13 entries
5. Update PROJECT.md with new requirement IDs
6. Update RISK-ASSESSMENT.md with new risks (R63+)
7. Self-check: verify all `depends_on` references resolve to real plan IDs
8. Self-check: verify all `files_modified` paths are consistent across plans (no two plans in the same wave modifying the same file without explicit dependency)
