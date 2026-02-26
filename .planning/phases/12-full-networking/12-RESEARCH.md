# Phase 12: Full Networking — Research

## C++ Source Analysis
- **Files:**
  - `network.h`/`cpp`: Protocol definitions, snapshot system
  - `game.cpp`: Networking integration, server-side validation
- **Key Functions:**
  - `Server::BRC_POSE()`: Broadcast character positions and animation states
  - `Server::BRC_SWING()` / `BRC_DAMAGE()`: Combat synchronization
  - `NetPlay::Interpolate()`: Remote player smoothing
- **Data Structures:**
  - `Snapshot`: Full world state delta-compressed
  - `InputBuffer`: Circular buffer for client-side prediction
- **Constants:**
  - Tick Rate: 30Hz
  - Ping Rate: 10Hz
  - Interpolation Delay: ~100ms

## Crate Dependencies
- `bevy_replicon = "0.38"`: High-level entity replication (Decision D12)
- `bevy_replicon_renet2 = "0.13"`: Transport layer for `bevy_replicon`
- `serde = "1.0"`: Binary serialization for custom messages

## ECS Architecture
- **Components:**
  - `Replicated`: Marker for entities synced over the network
  - `RemoteInterpolation`: Buffers received positions for smoothing
- **Resources:**
  - `NetworkClock`: Tracks authoritative server tick
  - `NetworkMapping`: Map between server and local entity IDs
- **Events:**
  - `NetworkCombatEvent`: SWING/DAMAGE/DEATH messages
  - `NetworkInputEvent`: Client inputs sent to server
- **Schedules:**
  - `FixedUpdate`: Movement prediction and authoritative physics (Server)
  - `Update`: Remote entity interpolation, input collection
  - `PostUpdate`: State snapshotting and delta compression

## Cross-Phase Dependencies
- **Reads:**
  - Phase 8: NPC and combat logic for replication
  - Phase 9: Item ownership and interaction for sync
  - Phase 10: HUD chat messages for network broadcast
  - Phase 7: Initial networking skeleton (07-03)
- **Provides:**
  - Synchronized world state for multi-player gameplay

## Open Questions
- Should we use standard `bevy_replicon` snapshots or port the custom C++ delta compression exactly? (Plan 12-01 assumes `bevy_replicon` for speed)
- How to handle lag compensation for melee combat frames (hit frame 21)?
