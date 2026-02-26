# Phase 10: UI/HUD and Interaction — Research

## C++ Source Analysis
- **Files:**
  - `game.cpp`: HUD rendering sections, Minimap logic, Chat/Talk sections
  - `render.cpp`: `UnprojectCoords2D`/`3D` implementation
- **Key Functions:**
  - `UnprojectCoords2D()`: Mouse-to-world conversion using depth buffer
  - `PaintMiniMap()`: Terrain sampling and dot rendering
  - `Font1Paint()`: Base text rendering (already ported in Phase 7)
- **Data Structures:**
  - `TalkBox`: Buffered chat messages with fade-out timers
  - `DamageFloater`: Temporary world-space text entities
- **Constants:**
  - HUD Position: Bottom-left for HP/MP, Top-right for Minimap
  - Minimap Size: 32x16 cells
  - Chat Bubble Duration: ~5 seconds

## Crate Dependencies
- No new crates needed beyond Bevy core and Phase 7 `Font1`.

## ECS Architecture
- **Components:**
  - `TalkBubble`: Component on characters, stores current chat text and timer
  - `DamageFloater`: Component with world position and upward velocity
- **Resources:**
  - `HudState`: Tracks UI visibility, chat buffer, and targeting info
- **Events:**
  - `ChatEvent`: Incoming chat messages (local or network)
  - `InteractionEvent`: Player clicking on world items/NPCs
- **Schedules:**
  - `Update`: Floating damage number movement, chat bubble timers
  - `PostUpdate`: HUD rendering systems (using `Font1` set_cell() on `AsciiCellGrid`)

## Cross-Phase Dependencies
- **Reads:**
  - Phase 8: NPC positions for minimap dots, combat for damage values
  - Phase 9: Item entities for mouse targeting/interaction
  - Phase 7: `Font1` system for all UI rendering
  - Phase 5: View/Projection matrices for `UnprojectCoords`
- **Provides:**
  - Interactive targeting data for Phase 13 scripting

## Open Questions
- Should the Minimap sample the `RuntimeTerrain` quadtree directly or use a cached heightmap? (Plan 10-03 assumes direct sampling for simplicity)
- Damage floater implementation: should they be world-space entities or UI-space overlays?
