> **STATUS: ACTIVE GAP ANALYSIS** — Generated 2026-02-20. Plan generated: plan-systems-gaps.md. Note: References to Mage Core in appendix are for the rendering layer evaluation (D001 chose Bevy as framework; Mage Core patterns may be used for ASCII rendering plugin).

# GAP ANALYSIS: Asciicker Audio, Network, and Input Systems

This document identifies gaps in the research coverage of the Asciicker audio, network, and input systems. It reviews the existing documentation in `batch_audio.md`, `input_cpp.md`, and `network_cpp.md`, and identifies features, edge cases, and platform-specific behaviors that were NOT fully covered.

---

## 1. Audio System Gaps

### 1.1 Platform-Specific Backend Initialization

**Gap:** Missing detailed documentation on audio backend initialization sequences and platform-specific quirks.

**What's covered:** The `batch_audio.md` documents the five backends (CoreAudio, PulseAudio, SDL, AudioWorklet, ScriptNode) and their initialization functions.

**What's missing:**
- CoreAudio: AudioQueue buffer allocation strategy, preferred sample rate handling
- PulseAudio: Stream connection state machine, context ready wait protocol details
- SDL: SDL_AudioSpec configuration values used, why 2048 buffer size was chosen
- AudioWorklet: Worklet node instantiation timing, message port setup
- ScriptNode: Why this legacy path exists, browser compatibility matrix

**Recommendation:** Document each backend's initialization sequence with timing diagrams and failure recovery paths.

### 1.2 Audio Format Constraints

**Gap:** Missing documentation on audio format constraints and requirements.

**What's covered:** The documentation mentions 44100 Hz, 16-bit stereo, Ogg Vorbis format.

**What's missing:**
- Maximum sample duration limits
- Mono vs stereo file handling specifics
- Sample rate mismatch handling (what happens if file is not 44100 Hz)
- Bit depth validation
- Supported Ogg Vorbis bitrate range
- stb_vorbis error handling and recovery

**Recommendation:** Document the exact audio format requirements and validation performed during sample loading.

### 1.3 Audio Command Queue Protocol

**Gap:** Incomplete documentation of the audio command protocol between game thread and audio thread.

**What's covered:** The documentation covers command sizes (2, 4, 12+ bytes) and their meanings.

**What's missing:**
- Maximum queue depth and overflow behavior
- Command processing order guarantees
- Thread-safety guarantees (memory ordering)
- Web Audio: message serialization format
- Web Audio: worklet message queue size limits
- What happens if commands arrive during audio thread shutdown

**Recommendation:** Document the complete command protocol including edge cases and error handling.

### 1.4 Sample Management Edge Cases

**Gap:** Missing documentation on sample loading edge cases and error handling.

**What's covered:** Basic loading flow, hash table lookup.

**What's missing:**
- What happens when `MAX_SAMPLES` (64) is exceeded
- Duplicate sample name handling
- Corrupted Ogg file handling
- File I/O error propagation
- Web: What happens if Emscripten FS is not populated
- Hash collision resolution performance implications

**Recommendation:** Document error handling paths and resource exhaustion scenarios.

### 1.5 Memory Leak Bug

**Gap:** The documented memory leak bug lacks recommended fix implementation details.

**What's covered:** `audit-unknown-audio-details.md` documents the bug (no sample unload function).

**What's missing:**
- Exact line numbers where fixes should be applied
- Interaction with Web Audio memory management
- Testing methodology to verify fix
- Memory profiling recommendations

**Recommendation:** Add implementation guidance for the sample unload fix.

### 1.6 Audio Mixing Details

**Gap:** Missing documentation on the audio mixing algorithm specifics.

**What's covered:** Documents 16-track mixer, int32 accumulator, saturation to [-32767, +32767].

**What's missing:**
- Why 65535 divisor is used instead of 65536 (precision issue noted but not explained)
- Track priority or muting behavior
- Forest loop volume independent of global volume
- What happens when all 16 tracks are in use
- Clip detection and reporting
- Audio clipping distortion characteristics

**Recommendation:** Document the mixing algorithm in detail with example calculations.

### 1.7 Platform Audio Device Handling

**Gap:** No documentation on audio device enumeration, selection, or hot-swapping.

**What's covered:** Backend initialization, fixed audio device.

**What's missing:**
- Default audio device selection criteria
- What happens when audio device is disconnected
- Support for multiple audio devices
- Audio device changes during runtime
- Sample rate matching between app and hardware

**Recommendation:** Document platform audio device handling and edge cases.

---

## 2. Input System Gaps

### 2.1 Platform Key Translation Details

**Gap:** Missing detailed key translation implementation for each platform.

**What's covered:** High-level mention of SDL, X11, Win32, Web backends with translation tables.

**What's missing:**
- **SDL:** Exact scancode to A3D key mapping table (the 128-element array)
- **X11:** XLookupKeysym usage, keyboard layout handling, modifier state tracking
- **Win32:** Virtual key code to A3D translation, scancode vs virtual key handling
- **Web:** Emscripten keyboard event handling, key code normalization

**Recommendation:** Document each platform's key translation algorithm with complete mapping tables.

### 2.2 Input Event Processing Pipeline

**Gap:** Missing documentation on input event buffering and processing order.

**What's covered:** Three-stage pipeline concept, layered dispatch.

**What's missing:**
- Event queue size limits
- Event processing order (keyboard vs mouse vs gamepad)
- Input coalescing behavior
- Frame synchronization (when input state is captured vs processed)
- vsync interaction with input processing

**Recommendation:** Document the complete input event flow with timing diagrams.

### 2.3 Gamepad Configuration Details

**Gap:** Incomplete documentation of gamepad mapping system.

**What's covered:** Basic gamepad API, button/axis support, mapping table concept.

**What's missing:**
- Gamepad mapping file format (binary structure)
- Default mapping for Xbox/PS5 controllers
- Button index to output mapping algorithm
- Axis deadzone configuration
- Auto-reconnect behavior and timing
- Multiple gamepad handling (which gamepad is active)
- Hot-plug detection

**Recommendation:** Document the complete gamepad configuration system including file format.

### 2.4 Touch and Gesture Handling

**Gap:** Missing documentation on touch gesture recognition.

**What's covered:** Contact array, basic touch events (BEGIN, MOVE, END, CANCEL).

**What's missing:**
- Touch-to-mouse emulation details
- Multi-touch gesture recognition (pinch zoom, rotate)
- Touch contact tracking algorithm
- Touch latency compensation
- Pressure sensitivity handling
- Touch cancellation conditions

**Recommendation:** Document touch gesture recognition and handling.

### 2.5 Input State Reset Behavior

**Gap:** Missing documentation on input state management during state transitions.

**What's covered:** Basic input state structures.

**What's missing:**
- What happens to input state during window resize (`OnSize`)
- Input state during menu transitions
- Input state during game pause
- Input state during network disconnect
- Input state during editor mode entry/exit

**Recommendation:** Document input state reset scenarios and behavior.

### 2.6 Modifier Key and IME Handling

**Gap:** Missing documentation on advanced keyboard input handling.

**What's covered:** Modifier key tracking concept.

**What's missing:**
- Modifier key combination handling (Ctrl+Shift+Key)
- Sticky key behavior
- Key repeat rate configuration
- IME (Input Method Editor) composition handling
- Unicode input via composition
- Key binding conflict resolution

**Recommendation:** Document advanced keyboard input handling.

### 2.7 Focus and Window State Handling

**Gap:** Missing documentation on window focus handling.

**What's covered:** `keyb_focus` callback exists.

**What's missing:**
- What happens when window loses focus (input continues? paused?)
- Focus loss during critical operations
- Re-acquiring focus behavior
- Multi-monitor focus handling
- Fullscreen vs windowed focus differences

**Recommendation:** Document window focus handling behavior.

---

## 3. Network System Gaps

### 3.1 Connection Lifecycle

**Gap:** Missing documentation on connection establishment and termination.

**What's covered:** TCP transport, WebSocket framing, basic message types.

**What's missing:**
- Connection timeout values
- DNS resolution handling
- Connection retry logic
- Graceful disconnect protocol
- Abnormal disconnect detection
- Connection state machine (connecting, connected, disconnecting, disconnected)

**Recommendation:** Document the complete connection lifecycle with state diagram.

### 3.2 Reconnection and Recovery

**Gap:** Missing documentation on network failure recovery.

**What's covered:** Basic client-server model.

**What's missing:**
- Auto-reconnect behavior
- Reconnection timing (backoff algorithm)
- What happens to game state during disconnect
- Partial message recovery
- Server unreachable handling
- Network change detection (WiFi to cellular)

**Recommendation:** Document network failure scenarios and recovery paths.

### 3.3 Message Queueing and Flow Control

**Gap:** Missing documentation on message handling during network issues.

**What's covered:** Basic message types and broadcast model.

**What's missing:**
- Message queue size limits
- Outgoing message buffering during high latency
- Incoming message overflow handling
- Message prioritization (pose vs chat vs lag)
- Bandwidth throttling
- MTU handling and message fragmentation

**Recommendation:** Document message queueing and flow control mechanisms.

### 3.4 Server-Side Implementation

**Gap:** Missing documentation on server-side behavior.

**What's covered:** Client-side network implementation.

**What's missing:**
- Server player limit enforcement
- Server-side validation of client poses
- Server broadcast timing
- Server message ordering guarantees
- Server anti-cheat measures
- Server-side rate limiting

**Recommendation:** Document server-side network behavior.

### 3.5 Protocol Edge Cases

**Gap:** Missing documentation on protocol edge cases and error handling.

**What's covered:** Token-based protocol, basic message types.

**What's missing:**
- **Token collision:** The 'j' token is used for both join response and broadcast join - document why this doesn't cause issues
- Invalid token handling
- Malformed message handling
- Partial message recovery
- Endianness on big-endian platforms
- Protocol version negotiation
- Message size limits and validation

**Recommendation:** Document protocol edge cases with examples.

### 3.6 Latency Compensation

**Gap:** Incomplete documentation of latency compensation mechanisms.

**What's covered:** LAG message for RTT measurement, WebSocket ping/pong concept.

**What's missing:**
- Client-side prediction implementation
- Server-side lag compensation (how server handles delayed inputs)
- Interpolation of remote player positions
- Extrapolation for high-latency scenarios
- Lag spike handling
- Network quality indicators (good/moderate/poor)
- Adaptive quality based on network conditions

**Recommendation:** Document complete latency compensation strategy.

### 3.7 WebSocket Implementation Details

**Gap:** Missing detailed WebSocket implementation documentation.

**What's covered:** WebSocket framing exists, RFC 6455 support.

**What's missing:**
- WebSocket handshake details
- Ping/pong timing (interval, timeout)
- Frame fragmentation details
- Mask handling (client-to-server masking requirement)
- Connection upgrade process
- Subprotocol support

**Recommendation:** Document WebSocket implementation details.

### 3.8 Thread Safety and Concurrency

**Gap:** Missing documentation on network thread safety.

**What's covered:** Platform abstraction for threading primitives.

**What's missing:**
- Network thread vs game thread synchronization
- Message thread safety guarantees
- Concurrent send/receive handling
- Lock ordering to prevent deadlocks
- Atomic operation usage

**Recommendation:** Document threading model for network system.

### 3.9 Security Considerations

**Gap:** Missing documentation on network security.

**What's covered:** None.

**What's missing:**
- Player name sanitization
- Chat message filtering
- Packet injection prevention
- Connection validation
- Rate limiting on server
- DoS protection

**Recommendation:** Document network security measures.

---

## 4. Cross-Cutting Concerns

### 4.1 Platform-Specific Behaviors Summary

| Area | Platform | Gap |
|------|----------|-----|
| Audio | All | Device enumeration, hot-swap handling |
| Audio | Web | Worklet initialization timing |
| Input | X11 | Key translation details |
| Input | Win32 | Key translation details |
| Input | Web | Key event normalization |
| Input | All | Focus handling specifics |
| Network | All | Connection state machine |

### 4.2 Error Handling Gaps Summary

| System | Missing Error Handling |
|--------|----------------------|
| Audio | Sample load failure recovery |
| Audio | Audio device disconnect |
| Input | Invalid key code handling |
| Input | Gamepad disconnect during use |
| Network | Partial message recovery |
| Network | Connection timeout recovery |

### 4.3 Performance Considerations Gaps

| System | Missing Performance Info |
|--------|-------------------------|
| Audio | Mixing CPU usage per platform |
| Audio | Memory usage scaling |
| Input | Input latency measurements |
| Network | Bandwidth usage per message type |
| Network | Message processing latency |

---

## 5. Recommendations Summary

### High Priority

1. **Audio Command Protocol:** Document complete protocol including queue behavior
2. **Network Connection Lifecycle:** Document connection states and recovery
3. **Input Event Pipeline:** Document event buffering and processing order
4. **Protocol Edge Cases:** Document token collisions, endianness handling

### Medium Priority

1. **Gamepad Mapping:** Document complete file format and algorithm
2. **Latency Compensation:** Document prediction and interpolation
3. **Platform Key Translation:** Document each platform's translation
4. **Audio Format Constraints:** Document exact requirements

### Low Priority

1. **Security Considerations:** Document sanitization and validation
2. **Performance Metrics:** Add CPU/memory/bandwidth measurements
3. **Touch Gestures:** Document gesture recognition
4. **IME Handling:** Document input method support

---

## Appendix: Related Documentation

- `batch_audio.md` - Audio system architecture (primary reference)
- `input_cpp.md` - Input system architecture (primary reference)
- `network_cpp.md` - Network system architecture (primary reference)
- `audit-unknown-audio-details.md` - Audio implementation details
- `research-bevy-magecore-input.md` - Input system research for Rust port

