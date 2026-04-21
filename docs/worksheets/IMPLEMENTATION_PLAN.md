# Asciicker Rust Port - Implementation Plan

## Overview

This document defines the complete implementation plan with assumptions, success criteria, and gates for each milestone.

---

## Assumptions

### Technology Assumptions

| ID | Assumption |
|----|------------|
| A1 | Bevy 0.18+ is stable and compatible with our target platforms |
| A2 | WGPU backend works on target platforms (Windows, macOS, Linux, Web) |
| A3 | bevy_kira_audio provides sufficient audio capabilities |
| A4 | Rust 1.80+ supports needed features (const generics, async) |
| A5 | C++ rendering behavior can be exactly replicated in Rust |

### Data Assumptions

| ID | Assumption |
|----|------------|
| D1 | All .xp and .a3d file formats are fully reverse-engineered |
| D2 | Perspective math (focal, view_pos, view_dir) is complete |
| D3 | Glyph coverage table (256 entries) is complete |
| D4 | auto_mat lookup table (32K entries) format is complete |

### Resource Assumptions

| ID | Assumption |
|----|------------|
| R1 | Development machine can compile Bevy projects |
| R2 | Access to original .xp, .a3d, and .akm test files exists |
| R3 | Budget for ~3-6 months full-time development |

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-3)

#### Milestone 1.1: Project Setup (Week 1)

**Tasks:**
- [ ] Initialize Bevy project with Cargo
- [ ] Configure dependencies (bevy, bevy_kira_audio, serde, etc.)
- [ ] Set up project structure (components/, systems/, rendering/, etc.)
- [ ] Create initial build and verify it runs

**Assumptions:**
- A1, A4

**Success Criteria:**
- [ ] `cargo build` succeeds without errors
- [ ] Application launches and displays window
- [ ] Basic event loop runs without crashes

**Gate:** None (starting point)

---

#### Milestone 1.2: ASCII Buffer System (Week 2)

**Tasks:**
- [ ] Create ASCII buffer textures (fg, bg, chars)
- [ ] Implement font atlas loading from PNG
- [ ] Create render target for ASCII output
- [ ] Implement basic shader to display ASCII buffer

**Assumptions:**
- A2, D3

**Success Criteria:**
- [ ] Three textures created: fg_color, bg_color, char_code
- [ ] Font atlas loads and displays character at index 0
- [ ] Window shows rendered ASCII output

**Gate:** Milestone 1.1 complete

---

#### Milestone 1.3: Triangle Rasterizer (Week 3)

**Tasks:**
- [ ] Implement triangle rasterization to ASCII buffer
- [ ] Implement depth buffer (SampleBuffer)
- [ ] Implement RGB555 color packing
- [ ] Implement basic perspective projection

**Assumptions:**
- A5, D2

**Success Criteria:**
- [ ] Triangle renders to buffer with correct colors
- [ ] Depth test works (closer triangles overwrite farther)
- [ ] Perspective divide applies correctly
- [ ] Output matches C++ reference render

**Gate:** Milestone 1.2 complete

---

### Phase 2: Rendering Pipeline (Weeks 4-8)

#### Milestone 2.1: 6-Stage Pipeline (Weeks 4-5)

**Tasks:**
- [ ] Implement CLEAR stage
- [ ] Implement TERRAIN stage
- [ ] Implement WORLD stage (mesh instances)
- [ ] Implement SHADOW stage (player shadow projection)
- [ ] Implement REFLECTION stage
- [ ] Implement RESOLVE stage (2x supersample downsample)
- [ ] Implement SPRITES stage

**Assumptions:**
- A5

**Success Criteria:**
- [ ] All 6 stages execute in order
- [ ] Output matches C++ reference for test scene
- [ ] Performance: 60fps achievable

**Gate:** Milestone 1.3 complete

---

#### Milestone 2.2: auto_mat Lookup (Week 6)

**Tasks:**
- [ ] Implement RGB555 → xterm256 quantization
- [ ] Implement 32K-entry auto_mat lookup table
- [ ] Implement dithering (glyph selection)
- [ ] Integrate with RESOLVE stage

**Assumptions:**
- D4

**Success Criteria:**
- [ ] auto_mat table generates correctly
- [ ] Color quantization produces acceptable results
- [ ] Dither glyphs apply based on diffuse

**Gate:** Milestone 2.1 complete

---

#### Milestone 2.3: Sprite System (Weeks 7-8)

**Tasks:**
- [ ] Implement .xp file loader (gzip decompression)
- [ ] Implement sprite atlas extraction
- [ ] Implement sprite billboard transformation
- [ ] Implement animation frame advancement
- [ ] Integrate sprites into SPRITES stage

**Assumptions:**
- D1

**Success Criteria:**
- [ ] .xp files load without error
- [ ] Sprites render at correct screen positions
- [ ] Animation plays correctly
- [ ] Output matches C++ reference sprite render

**Gate:** Milestone 2.1 complete

---

### Phase 3: World Systems (Weeks 9-14)

#### Milestone 3.1: Terrain Quadtree (Weeks 9-10)

**Tasks:**
- [ ] Implement .a3d terrain file loader (Corrected: terrain uses .a3d format, not .xp — see FAILURE_LOG F003)
- [ ] Implement quadtree data structure
- [ ] Implement patch creation and expansion
- [ ] Implement neighbor resolution
- [ ] Implement height interpolation (bilinear)
- [ ] Integrate terrain into TERRAIN stage

**Assumptions:**
- D1

**Success Criteria:**
- [ ] Terrain loads from .a3d file
- [ ] Quadtree queries return correct heights
- [ ] Visual output matches C++ reference

**Gate:** Milestone 2.1 complete

---

#### Milestone 3.2: BSP World (Weeks 11-12)

**Tasks:**
- [ ] Implement .a3d file loader
- [ ] Implement BSP tree data structure
- [ ] Implement instance insertion
- [ ] Implement spatial queries (point, ray, box)
- [ ] Implement mesh rendering

**Assumptions:**
- D1

**Success Criteria:**
- [ ] World loads from .a3d file
- [ ] BSP tree correctly partitions space
- [ ] Spatial queries return correct instances
- [ ] Output matches C++ reference

**Gate:** Milestone 3.1 complete

---

#### Milestone 3.3: Collision/Physics (Weeks 13-14)

**Tasks:**
- [ ] Implement sphere-AABB collision
- [ ] Implement terrain height queries
- [ ] Implement gravity and jumping
- [ ] Implement water buoyancy
- [ ] Implement line-of-sight raycasting

**Assumptions:**
- A5

**Success Criteria:**
- [ ] Character collides with terrain
- [ ] Gravity pulls character down
- [ ] Water reduces gravity (buoyancy)
- [ ] Raycasting returns correct hit/no-hit

**Gate:** Milestone 3.2 complete

---

### Phase 4: Game Logic (Weeks 15-20)

#### Milestone 4.1: Input System (Week 15)

**Tasks:**
- [ ] Implement keyboard input (Bevy input)
- [ ] Implement mouse input
- [ ] Implement gamepad input
- [ ] Map A3D key codes to input events

**Assumptions:**
- A2

**Success Criteria:**
- [ ] All key presses detected
- [ ] Mouse position and clicks detected
- [ ] Gamepad buttons detected
- [ ] Input latency < 16ms

**Gate:** Milestone 3.3 complete

---

#### Milestone 4.2: Character Controller (Weeks 16-17)

**Tasks:**
- [ ] Implement player movement (8-directional)
- [ ] Implement camera following
- [ ] Implement Q/E camera rotation
- [ ] Implement zoom control
- [ ] Implement fly mode (debug)

**Assumptions:**
- A5

**Success Criteria:**
- [ ] Player moves with arrow keys
- [ ] Camera follows player
- [ ] Q/E rotates camera view
- [ ] Zoom changes view distance
- [ ] Fly mode allows free movement

**Gate:** Milestone 4.1 complete

---

#### Milestone 4.3: Combat System (Weeks 18-19)

**Tasks:**
- [ ] Implement melee attack
- [ ] Implement ranged attack (crossbow)
- [ ] Implement damage calculation
- [ ] Implement knockback
- [ ] Implement death handling

**Assumptions:**
- A5

**Success Criteria:**
- [ ] Attack animation plays
- [ ] Hit detection works (distance-based)
- [ ] Damage applies to HP
- [ ] Enemy dies at 0 HP

**Gate:** Milestone 4.2 complete

---

#### Milestone 4.4: AI System (Week 20)

**Tasks:**
- [ ] Implement enemy spawning
- [ ] Implement target selection
- [ ] Implement movement toward target
- [ ] Implement stuck detection/resolution
- [ ] Implement buddy AI

**Assumptions:**
- A5

**Success Criteria:**
- [ ] Enemies spawn at enemygen positions
- [ ] Enemies move toward player
- [ ] Enemies attack when in range
- [ ] Stuck enemies attempt to resolve

**Gate:** Milestone 4.3 complete

---

### Phase 5: Polish (Weeks 21-26)

#### Milestone 5.1: UI/HUD (Weeks 21-22)

**Tasks:**
- [ ] Implement HP/MP bars
- [ ] Implement minimap
- [ ] Implement inventory display
- [ ] Implement main menu
- [ ] Implement pause menu

**Assumptions:**
- A5

**Success Criteria:**
- [ ] HP bar shows player health
- [ ] Minimap shows terrain
- [ ] Inventory displays items
- [ ] Menus navigate correctly

**Gate:** Milestone 4.4 complete

---

#### Milestone 5.2: Audio (Week 23)

**Tasks:**
- [ ] Integrate bevy_kira_audio
- [ ] Implement music playback
- [ ] Implement sound effects
- [ ] Implement footstep sounds

**Assumptions:**
- A3

**Success Criteria:**
- [ ] Music plays during gameplay
- [ ] Sound effects trigger on actions
- [ ] Audio doesn't crackle or lag

**Gate:** Milestone 5.1 complete

---

#### Milestone 5.3: Save/Load (Week 24)

**Tasks:**
- [ ] Implement .a3d save format
- [ ] Implement .a3d load format
- [ ] Implement config save/load
- [ ] Implement screenshot export

**Assumptions:**
- D1

**Success Criteria:**
- [ ] Game saves to .a3d file
- [ ] Game loads from .a3d file
- [ ] Saved game matches loaded game state
- [ ] Config persists between sessions

**Gate:** Milestone 5.2 complete

---

#### Milestone 5.4: Performance Optimization (Weeks 25-26)

**Tasks:**
- [ ] Profile rendering performance
- [ ] Optimize hot paths
- [ ] Implement LOD if needed
- [ ] Test on target platforms

**Assumptions:**
- A2

**Success Criteria:**
- [ ] 60fps on target hardware
- [ ] Memory usage acceptable (<500MB)
- [ ] No memory leaks in long sessions

**Gate:** Milestone 5.3 complete

---

## Gate Summary

| Gate | Must Complete | Enables |
|------|---------------|---------|
| Gate 0 | - | Milestone 1.1 |
| Gate 1 | M1.1 | M1.2 |
| Gate 2 | M1.2 | M1.3 |
| Gate 3 | M1.3 | M2.1 |
| Gate 4 | M2.1 | M2.2, M2.3, M3.1 |
| Gate 5 | M2.2 | M3.1 |
| Gate 6 | M2.3 | M3.1 |
| Gate 7 | M3.1 | M3.2 |
| Gate 8 | M3.2 | M3.3 |
| Gate 9 | M3.3 | M4.1 |
| Gate 10 | M4.1 | M4.2 |
| Gate 11 | M4.2 | M4.3 |
| Gate 12 | M4.3 | M4.4 |
| Gate 13 | M4.4 | M5.1 |
| Gate 14 | M5.1 | M5.2 |
| Gate 15 | M5.2 | M5.3 |
| Gate 16 | M5.3 | M5.4 |
| Gate 17 | M5.4 | RELEASE |

---

## Critical Decision Points

| Decision | Point | Options |
|----------|-------|---------|
| k-d tree vs auto_mat | After M2.2 | Keep auto_mat or add k-d tree |
| Network implementation | After M4.4 | Skip, basic, or full |
| Editor tools | After M5.4 | Defer or implement |

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Rendering quality mismatch | Medium | High | Golden file tests |
| Performance < 60fps | Medium | High | Early profiling |
| Missing file format data | Low | High | Research complete |
| Bevy API changes | Low | Medium | Pin versions |

---

## Success Criteria Summary

| Phase | Criteria |
|-------|----------|
| Phase 1 | Basic rendering works |
| Phase 2 | Full pipeline matches C++ |
| Phase 3 | World interactions work |
| Phase 4 | Game is playable |
| Phase 5 | Release-ready |

---

*Implementation Plan: 2026-02-20*
