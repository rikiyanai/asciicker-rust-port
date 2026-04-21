# Implementation Plans: Medium Severity Systems Gaps

This document provides implementation plans for addressing medium severity gaps in the Audio, Input, and Network systems for the Asciicker Rust port. Each plan outlines the approach, Bevy equivalents or custom implementations, and estimated complexity.

---

## 1. Audio Platform Backends (Bevy Kira_audio)

### Gap Summary
The original Asciicker audio system uses platform-specific backends. The gaps identifies missing documentation on backend initialization, audio format constraints, command queue protocols, and memory management. The original has a memory leak bug where samples cannot be unloaded.

### Bevy Equivalent: bevy_kira_audio

The Rust port will use `bevy_kira_audio` which provides:
- Cross-platform audio (CoreAudio, ALSA, WASM automatically)
- Native OGG/MP3/FLAC/WAV support
- Proper memory management with automatic unloading
- 16+ channel mixing

### Implementation Approach

**Phase 1: Core Audio Infrastructure**
- Add dependency to Cargo.toml
- Configure audio plugin
- Create basic playback systems

**Phase 2: Sample Management**
- Recreate 64-sample hash table using Bevy asset management
- Sample registry by name with lookup

**Phase 3: Audio Mixing**
- 16-track mixer with int32 accumulator

### Complexity Estimate
- Phase 1: 2-3 days
- Phase 2: 3-4 days
- Phase 3: 2-3 days
- **Total: 10-15 days**

---

## 2. Input Event Processing Pipeline (Bevy Input)

### Gap Summary
Missing documentation on input event buffering, processing order, event coalescing, and frame synchronization.

### Bevy Equivalent: Bevy Input System

Bevy provides comprehensive input through `bevy_input` with both event-driven and polling-based handling.

### Implementation Approach

**Phase 1: Input Resource Setup**
- Configure Bevy input resources
- Map KeyCode to A3DKey enum

**Phase 2: Input State Management**
- Expand ShiftState to full input tracking
- Mouse, keyboard, gamepad state

**Phase 3: Platform Key Translation**
- Bevy/winit handles platform translation automatically

### Complexity Estimate
- Phase 1: 2 days
- Phase 2: 2-3 days
- Phase 3: 2 days
- **Total: 6-7 days**

---

## 3. Network Connection Lifecycle (Custom Implementation)

### Gap Summary
Missing documentation on connection establishment, termination, timeout values, retry logic, and formal state machine.

### Implementation Approach: Custom State Machine

No Bevy equivalent - implement custom networking.

**Phase 1: Connection State Machine**
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}
```

**Phase 2: Reconnection and Recovery**
- Exponential backoff (1s, 2s, 4s, 8s, max 30s)

**Phase 3: Protocol Implementation**
- Token-based protocol ('j' join, 'p' pose, 'c' chat)

### Complexity Estimate
- Phase 1: 3-4 days
- Phase 2: 4-5 days
- Phase 3: 3-4 days
- **Total: 10-13 days**

---

## 4. Gamepad Configuration

### Gap Summary
Missing gamepad mapping file format, default mappings, axis deadzone configuration, and hot-plug detection.

### Bevy Equivalent: Bevy Gamepad System

Bevy provides comprehensive gamepad support through `bevy_input::gamepad`.

### Implementation Approach

**Phase 1: Gamepad Discovery**
- Automatic detection
- Configurable deadzones

**Phase 2: Input Mapping**
- Button/axis to action mapping
- Default Xbox/PlayStation mappings

**Phase 3: Multi-Gamepad**
- Handle multiple connected controllers

### Complexity Estimate
- Phase 1: 1-2 days
- Phase 2: 2-3 days
- Phase 3: 1 day
- **Total: 4-6 days**

---

## 5. Latency Compensation

### Gap Summary
Missing client-side prediction, entity interpolation, and adaptive quality systems.

### Implementation Approach: Client-Side Prediction

**Phase 1: RTT Measurement**
- LAG message mechanism for round-trip time

**Phase 2: Client-Side Prediction**
- Local player movement prediction with sequence numbers

**Phase 3: Server Reconciliation**
- Re-simulation to correct prediction errors

**Phase 4: Entity Interpolation**
- Remote player interpolation with 50-100ms buffer

**Phase 5: Adaptive Quality**
- Network condition-based quality adjustment

### Complexity Estimate
- Phase 1: 1-2 days
- Phase 2: 3-4 days
- Phase 3: 2-3 days
- Phase 4: 3-4 days
- Phase 5: 2-3 days
- **Total: 11-16 days**

---

## Implementation Dependencies

| System | Phase | Dependencies |
|--------|-------|--------------|
| Input | 1-2 | None |
| Gamepad | 1-2 | None |
| Audio | 2-4 | None |
| Network | 3-5 | None |
| Latency | 5-8 | Network |

---

## Risk Assessment

| System | Risk | Mitigation |
|--------|------|------------|
| Audio (web) | Medium | Use AudioWorklet fallback |
| Input | Low | Bevy well-tested |
| Network | High | Custom - thorough testing |
| Gamepad | Low | Standard controllers |
| Latency | High | Prediction/interpolation complexity |

---

## Summary Table

| Gap | Solution | Complexity | Phase |
|-----|----------|------------|-------|
| Audio backends | bevy_kira_audio | Medium-High (10-15d) | 2-4 |
| Input pipeline | bevy_input | Medium (6-7d) | 1-2 |
| Network | Custom | High (10-13d) | 3-5 |
| Gamepad | bevy_input | Low-Medium (4-6d) | 1-2 |
| Latency | Custom | High (11-16d) | 5-8 |

*Plan created: 2026-02-20*
