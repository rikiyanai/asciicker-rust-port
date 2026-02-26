# Phase 13: NPC Scripting — Research

## C++ Source Analysis
- **Files:**
  - `game.cpp`: V8 bridge implementation (~65KB shared memory)
  - `v8/`: Embedded JavaScript engine integration
- **Key Functions:**
  - `V8_Execute()`: Main script execution entry
  - `SetScriptPos()` / `GetScriptPos()`: Data transfer between C++ and JS
- **Data Structures:**
  - `ScriptBuffer`: Flat array for character/terrain state exchange
- **Constants:**
  - 65KB Shared Memory Limit: Used for V8 data exchange

## Crate Dependencies
- `mlua = { version = "0.9", features = ["lua54", "vendored"] }`: High-level Lua 5.4 bridge (Default Option)
- `notify = "6.1"`: For script file hot-reloading (watching asset directory)

## ECS Architecture
- **Components:**
  - `NpcScript`: Stores the script filename and handle to the compiled Lua function
- **Resources:**
  - `LuaRuntime`: Holds the `mlua::Lua` instance and global state
- **Events:**
  - `ScriptErrorEvent`: For logging Lua runtime errors to the game console
- **Schedules:**
  - `Update`: Running Lua `on_tick` hooks for NPCs with scripts
  - `FixedUpdate`: Running Lua hooks that affect physics (movement commands)

## Cross-Phase Dependencies
- **Reads:**
  - Phase 8: NPC AI behavior framework (hooks into same execution path)
  - Phase 5: Terrain/BSP queries for script-driven pathfinding/vision
- **Provides:**
  - Dynamic NPC behavior without recompiling the Rust engine

## Open Questions
- Should we stick with Lua (`mlua`) or consider WASM (`wasmtime`) for better sandboxing? (Plan 13-01 assumes Lua as the default choice per User)
- How to efficiently expose `SpatialGrid` queries to Lua without high serialization overhead?
