> **STATUS: ACTIVE REFERENCE WITH CORRECTIONS** — The rendering foundation has been updated from standalone Mage Core to Bevy + Mage Core render plugin. CORRECTION: Line 100 states "9×9 vertex heightmap per patch" — this is WRONG. The correct value is 5×5 (HEIGHT_CELLS=4, producing (4+1)²=25 vertices). See docs/skills/world-loading.md. CORRECTION: Terrain uses .a3d format, not .xp (see FAILURE_LOG F003).

# Asciicker Rust Port - Implementation Research Summary

This document consolidates research findings from analyzing the Asciicker C++ codebase and the Mage-core Rust engine to create a comprehensive porting plan.

---

## CRITICAL PREREQUISITE: Fix Known C++ Bugs Before Porting

### Step 0: Fix Critical Bugs in terrain.cpp (MUST FIX FIRST)

Before any porting work begins, the following critical bugs in terrain.cpp must be fixed in the original C++ codebase:

| Bug ID | File | Line | Severity | Description |
|--------|------|------|----------|-------------|
| BUG-001 | terrain.cpp | 613 | CRITICAL | `if (x)` appears twice, should check `y` |
| BUG-002 | terrain.cpp | 480, 492 | HIGH | Boundary `>` vs `>=` assumption |
| BUG-003 | terrain.cpp | 805 (1671) | CRITICAL | Condition `u < y` where `y` out of scope |

**Why Fix First:**
- BUG-001 causes incorrect coordinate reconstruction in patch queries
- BUG-003 causes incorrect condition in terrain sampling
- BUG-002 affects boundary handling behavior (needs verification)
- All bugs could cause crashes or incorrect rendering in the ported code
- Fixing ensures the Rust port has correct behavior to replicate

**See:** `implementation-plan-terrain-fix.md` for detailed fix specifications and verification steps

---

## Executive Summary

Porting Asciicker from C++ to Rust is a significant undertaking that involves multiple complex subsystems. The research identified ~35 C++ source files documenting game logic, rendering, world management, physics, audio, networking, and editor functionality. The Mage-core Rust engine provides a solid foundation for ASCII rendering but lacks many features needed for a complete game engine.

---

## Subsystem Analysis

### 1. Rendering System (Priority: CRITICAL)

**Source Files:** `render.cpp`, `render.h`

**Core Components:**
- Sample Buffer: 2x supersampled depth/color buffer for anti-aliasing
: Bresenham lines, barycentric- Rasterization triangles, perspective-correct interpolation
- 6-Stage Pipeline: CLEAR → TERRAIN → WORLD → SHADOW → REFLECTION → RESOLVE → SPRITES

**Key Data Structures:**
```cpp
struct Sample {
    float height;      // Depth value
    uint16_t visual;   // RGB555 color
    uint8_t diffuse;  // Lighting value
    uint8_t spare;    // Flags
};
```

**Porting Approach:** Use Mage-core's WGPU pipeline as base, implement custom rasterization shaders

---

### 2. Game Logic System (Priority: CRITICAL)

**Source Files:** `game.cpp`, `game.h`

**Core Components:**
- Character state machine (NONE→ATTACK→FALL→STAND→DEAD)
- 5D equipment sprite lookup [action][mount][weapon][shield][helmet][armor]
- Physics integration with force accumulation
- AI pathfinding and collision
- Combat system (melee/ranged)
- Inventory grid (8×20)
- Unified input (mouse/touch/keyboard/gamepad)
- UI rendering (minimap, HP/MP, menus)

**Porting Approach:** Implement from scratch using Rust idioms, leverage ECS pattern

---

### 3. World System (Priority: HIGH)

**Source Files:** `world.cpp`, `world.h`

**Core Components:**
- BSP tree with Surface Area Heuristic (SAH)
- Instance management (MeshInst, SpriteInst, ItemInst)
- Raycasting with Plucker coordinates
- 8 octant-specific raycast variants
- .a3d serialization

**Porting Approach:** Port BSP implementation, use Rust's type system for safety

---

### 4. Terrain System (Priority: HIGH)

**Source Files:** `terrain.cpp`, `terrain.h`

**Core Components:**
- Quadtree with auto-expansion/shrinkage
- 9×9 vertex heightmap per patch
- 8×8 material cells per patch
- Diagonal orientation bitfield
- Neighbor flags for quadtree navigation

**Porting Approach:** Direct port of quadtree implementation

**NOTE:** Before porting terrain system, MUST fix bugs BUG-001, BUG-002, BUG-003 (see Step 0 above)

---

### 5. Sprite System (Priority: HIGH)

**Source Files:** `sprite.cpp`, `sprite.h`

**Core Components:**
- XP format parser (gzip-compressed REXPaint)
- Reference-counted sprite management
- Blit/dither operations
- CP437 glyph handling
- Color quantization (RGB → xterm256)

**Porting Approach:** Direct port with Rust's `Rc<RefCell<>>`

---

### 6. Physics System (Priority: MEDIUM)

**Source Files:** `physics.cpp`, `physics.h`

**Core Components:**
- Sphere-triangle collision (face/edge/vertex)
- Time-of-impact (TOI) sweep
- Terrain quadtree traversal
- World mesh BSP queries
- Gravity, water buoyancy

**Porting Approach:** Consider using `parry3d` crate or custom implementation

---

### 7. Audio System (Priority: MEDIUM)

**Source Files:** `audio.cpp`, `audio.h`

**Core Components:**
- stb_vorbis Ogg decoding
- 16-track mixer
- Platform backends (CoreAudio/PulseAudio/SDL/WebAudio)
- Material-based footstep sounds

**Porting Approach:** Use `rodio` or `cpal` crate, existing Rust Ogg crates

---

### 8. Network System (Priority: MEDIUM)

**Source Files:** `network.cpp`, `game_svr.cpp`

**Core Components:**
- TCP sockets with WebSocket framing
- HTTP header parsing for upgrade
- Thread-per-client model
- RWLock client management
- Binary protocol (JOIN/POSE/TALK/LAG)

**Porting Approach:** Use `tokio` + `tungstenite` for async WebSocket

---

### 9. Editor System (Priority: LOW for initial port)

**Source Files:** `asciiid.cpp` (11,584 lines)

**Core Components:**
- OpenGL rendering with ImGui
- Terrain automation (auto-material, mesh baking)
- MCP command processor
- .a3d save/load

**Porting Approach:** Use `imgui-rs`, defer complex features

---

## Recommended Porting Strategy

### Phase 1: Core Rendering (Weeks 1-4)
1. Set up WGPU pipeline (base: Mage-core)
2. Implement ASCII cell buffer → GPU texture
3. Port font loading system
4. Implement basic sprite rendering

### Phase 2: Game Foundation (Weeks 5-10)
1. World system (BSP tree)
2. Terrain system (quadtree) — AFTER BUG FIX
3. Sprite system (.xp loader)
ES4. Basic game loop

### Phase 3: Gameplay (Weeks 11-18)
1. Character/equipment system
2. Physics/collision
3. AI system
4. Inventory system
5. UI system

### Phase 4: Polish (Weeks 19-24)
1. Audio system
2. Network multiplayer
3. Editor (optional)

---

## Key Technical Decisions

| Decision | Recommendation |
|----------|----------------|
| **Rendering Backend** | WGPU (from Mage-core) |
| **Windowing** | winit |
| **UI Framework** | imgui-rs |
| **Audio** | rodio + lewton (Ogg) |
| **Physics** | Custom or parry3d |
| **Networking** | tokio + tungstenite |
| **Serialization** | bincode + serde |
| **Build System** | Cargo |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Template-heavy C++ code | High | Rewrite with traits/generics |
| Global mutable state | High | Use state management pattern |
| Performance-critical rendering | Medium | Profile early, optimize hot paths |
| Bit manipulation | Low | Direct translation to Rust |
| 3rd party C++ deps (stb_vorbis) | Medium | Use Rust alternatives |
| **Unfixed terrain bugs** | CRITICAL | Fix BUG-001, BUG-002, BUG-003 before porting |

---

## Dependencies to Replace

| C++ Library | Rust Alternative |
|-------------|------------------|
| stb_vorbis | lewton, vorbis |
| stb_truetype | rusttype, ab_glyph |
| SDL2 | winit + wgpu |
| ImGui | imgui-rs |

---

## Conclusion

The port is feasible but requires careful planning. Starting with Mage-core as the rendering foundation and porting subsystems in priority order will minimize risk. **Critical: All terrain.cpp bugs must be fixed in the C++ source before porting begins** to ensure the Rust implementation has correct behavior to replicate.

Expect 6+ months for a feature-complete initial port.

---

## Appendix: Bug Fix Reference

See `implementation-plan-terrain-fix.md` for complete details on:
- Exact code context for each bug
- Proposed code changes
- Verification test cases
- Related bugs that should be fixed together
