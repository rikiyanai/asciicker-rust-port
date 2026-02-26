# Phase 11: Full Menu System — Research

## C++ Source Analysis
- **Files:**
  - `mainmenu.cpp`: 2,919 LOC, Hierarchical menu logic, background dithering, loading FSM
  - `gamepad.cpp`: 2,318 LOC, Visual mapping UI
- **Key Functions:**
  - `MainMenuContext::OnKeyb()`: Stack-based hierarchical navigation
  - `MainMenuContext::Paint()`: Hierarchical rendering with scrolling
  - `ScaleImg()`: Aspect-correct background scaling with dithering
- **Data Structures:**
  - `MainMenu`: Recursive struct with strings, sub-menus, and action callbacks
  - `MainMenuContext`: Stores depth stack, scroll state, and loading progress
- **Constants:**
  - `menu_depth` limit: 4
  - Dither hidden value: 20
  - Loading progress steps: 0-3 (Counts down)

## Crate Dependencies
- `serde_json = "1.0"`: For settings persistence (config saving/loading)
- `glob = "0.3"`: For level file listing in the Level Selection screen

## ECS Architecture
- **Components:**
  - `MenuNode`: Part of the hierarchical menu entity tree (if using entities)
- **Resources:**
  - `MenuStack`: Tracks current menu hierarchy, selection index, and scroll
  - `UserSettings`: Persistent configuration (Video, Controls, Audio)
- **Events:**
  - `MenuActionEvent`: Triggered when an item is selected (e.g., ChangeRes, LoadLevel)
- **Schedules:**
  - `Update`: Menu navigation input handling, state transitions
  - `PostUpdate`: Menu rendering (dithered background + text overlays)

## Cross-Phase Dependencies
- **Reads:**
  - Phase 7: `Font1` for menu item rendering, initial menu skeleton (07-02)
  - Phase 2: `.a3d` world loader for the LoadLevel action
- **Provides:**
  - Game configuration settings for all subsystems (Audio, Render, Physics)

## Open Questions
- Should we use Bevy UI or stick to custom `AsciiCellGrid` rendering? (Plan 11-02 assumes custom `AsciiCellGrid` for C++ fidelity)
- Is the full `gamepad.cpp` visual mapping required for MVP? (Marked as optional in 11-04)
