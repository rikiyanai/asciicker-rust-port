# Phase 09: Inventory and Items — Research

## C++ Source Analysis
- **Files:**
  - `inventory.h`: 234 LOC, Item definitions and inventory grid constants
  - `inventory.cpp`: 759 LOC, Grid collision, insertion/removal, directional navigation
  - `game.cpp`: 5468-5920 (Item world interaction, pickup/drop)
- **Key Functions:**
  - `Inventory::InsertItem()`: Handles bitmask collision and ownership transfer
  - `Inventory::FocusNext()`: Directional arrow-key navigation logic
  - `GetNearbyItems()`: Proximity detection for pickup (C++ uses BSP, Rust uses `SpatialGrid`)
- **Data Structures:**
  - `Inventory`: 8x20 grid, 160 bits bitmask, `MyItem` array
  - `ItemProto`: Template for item types (kind, sub_kind, sprites)
  - `Item`: Instance with purpose (EDIT/WORLD/OWNED)
- **Constants:**
  - `Inventory::width = 8`, `Inventory::height = 20`
  - Item kinds: 'W'eapon, 'S'hield, 'H'elmet, 'A'rmor, 'P'otion, 'F'ood, etc.
  - Sub-kinds for 5D sprite indexing

## Crate Dependencies
- `serde = { version = "1.0", features = ["derive"] }`: For data-driven item catalog (Optional, but recommended)

## ECS Architecture
- **Components:**
  - `Inventory`: Component on players/NPCs, stores item entities and bitmask
  - `ItemInstance`: Component on item entities, stores reference to `ItemProto`
  - `Equipped`: Marker component indicating an item is currently active
- **Resources:**
  - `ItemProtoLibrary`: Catalog of all item types (loaded from data)
- **Events:**
  - `ItemPickupEvent`: From player interaction to inventory insertion
  - `ItemDropEvent`: From inventory removal to world spawning
- **Schedules:**
  - `Update`: Item interaction queries, UI input handling
  - `PostUpdate`: Inventory UI rendering (overlay on `AsciiCellGrid`)

## Cross-Phase Dependencies
- **Reads:**
  - Phase 8: `SpatialGrid` for proximity queries
  - Phase 6: Character equipment lookup tables
  - Phase 7: `Font1` for inventory text rendering
- **Provides:**
  - Item ownership data for Phase 12 networking replication
  - Interactive targets for Phase 10 HUD targeting

## Open Questions
- Should the `ItemProtoLibrary` be hardcoded initially or loaded from an external file? (Plan 09-01 assumes data-loaded but can start hardcoded)
- How to handle multi-cell item sprites in the 8x20 grid — current C++ uses `sprite_width / 4`.
