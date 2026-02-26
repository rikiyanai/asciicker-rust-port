# Technical Audits: Phase 3 & 4 - Complete Analysis

This document consolidates technical audits for Implementation Phases 3 and 4, with detailed task breakdowns, assumptions, gaps, gotchas, and sequential task lists.

---

# PHASE 3: WORLD SYSTEMS (Weeks 9-14)

## Milestone 3.1: Terrain Quadtree (Weeks 9-10)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 3.1.1 | Implement .a3d terrain file loader (binary file reading (terrain .a3d is NOT gzip compressed)) | M2.1 complete | .a3d loads | (Corrected: terrain uses .a3d binary format, not .xp — see FAILURE_LOG F003)
| 3.1.2 | Parse .a3d header (version, width, height) | 3.1.1 | Header parsed |
| 3.1.3 | Extract height map data (5x5 vertices (HEIGHT_CELLS=4)) | 3.1.2 | Heights extracted |
| 3.1.4 | Extract visual/material data (8x8 cells) | 3.1.3 | Materials extracted |
| 3.1.5 | Implement quadtree data structure (Node, Patch) | 3.1.1 | Structs defined |
| 3.1.6 | Implement patch insertion (AddTerrainPatch) | 3.1.5 | Patches insert |
| 3.1.7 | Implement "grow upward" expansion | 3.1.6 | Tree expands |
| 3.1.8 | Implement neighbor resolution (GetTerrainNeighbor) | 3.1.7 | Neighbors found |
| 3.1.9 | Implement height interpolation (bilinear) | 3.1.8 | Smooth heights |
| 3.1.10 | Implement diagonal orientation (Tap3x3) | 3.1.9 | Correct diagonals |
| 3.1.11 | Implement bounds propagation (UpdateNodes) | 3.1.10 | Bounds propagate |
| 3.1.12 | Integrate terrain into TERRAIN stage | 3.1.11 | Terrain renders |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| D1 | .a3d terrain format reverse-engineered (Corrected: terrain uses .a3d binary format, not .xp — see FAILURE_LOG F003) | **PARTIAL** | Medium |
| A5 | C++ rendering replicable | From Phase 1 | Medium |
| - | HEIGHT_CELLS=4, VISUAL_CELLS=8 | **VERIFY** | Medium |

### Gaps

- No LOD system (all patches identical resolution)
- .a3d terrain format details not fully specified (Corrected: terrain uses .a3d binary format, not .xp — see FAILURE_LOG F003)
- Radius culling algorithm not detailed
- Frustum culling details not specified

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Quadtree coordinate system (world-relative) | Wrong positions | Use world coords |
| Level semantics (root = level 0) | Wrong traversal | Document clearly |
| Neighbor flag sync | Missing edges | Sync after insert |
| Patch boundary gaps | Visual artifacts | Verify neighbor lookup |

---

## Milestone 3.2: BSP World (Weeks 11-12)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 3.2.1 | Implement .a3d file loader | M3.1 complete | .a3d loads |
| 3.2.2 | Parse .a3d header (version, count) | 3.2.1 | Header parsed |
| 3.2.3 | Parse mesh instances (MeshInst) | 3.2.2 | Meshes parsed |
| 3.2.4 | Parse sprite instances (SpriteInst) | 3.2.3 | Sprites parsed |
| 3.2.5 | Parse item instances (ItemInst) | 3.2.4 | Items parsed |
| 3.2.6 | Implement BSP tree data structure | 3.2.1 | Struct defined |
| 3.2.7 | Implement instance insertion (BSP::InsertInst) | 3.2.6 | Instances insert |
| 3.2.8 | Implement instance deletion (DelInst) | 3.2.7 | Instances delete |
| 3.2.9 | Implement spatial queries (point, box, ray) | 3.2.8 | Queries work |
| 3.2.10 | Implement mesh rendering | 3.2.9 | Meshes render |
| 3.2.11 | Implement ancestor cleanup (fix STUBBED) | 3.2.10 | Memory managed |
| 3.2.12 | Integrate world into WORLD stage | 3.2.11 | World renders |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| D1 | .a3d format reverse-engineered | **PARTIAL** | Medium |
| - | NODE_SHARE not implemented | Feature deferred | Low |

### Gaps

- Ancestor cleanup STUBBED (memory leak risk)
- NODE_SHARE not implemented
- 8 HitWorld variants not documented
- SAH cost threshold not applicable

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Instance lifecycle (flat → Rebuild → BSP) | Wrong data | Follow order |
| INST_USE_TREE flag | Missing queries | Check flag |
| 8 octant variants | Complexity | Use generic first |
| Empty parent nodes | Memory leak | Implement cleanup |

---

## Milestone 3.3: Collision/Physics (Weeks 13-14)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 3.3.1 | Implement sphere-AABB collision | M3.2 complete | Collision detects |
| 3.3.2 | Implement terrain height query | 3.3.1 | Height found |
| 3.3.3 | Implement gravity system | 3.3.2 | Gravity applies |
| 3.3.4 | Implement jumping | 3.3.3 | Jump works |
| 3.3.5 | Implement water buoyancy | 3.3.4 | Buoyancy works |
| 3.3.6 | Implement line-of-sight raycasting | 3.3.5 | Raycast works |
| 3.3.7 | Integrate physics into game loop | 3.3.6 | Physics runs |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A5 | C++ physics replicable | **VERIFY** | Medium |
| - | Physics constants (gravity, jump velocity) | **UNKNOWN** | High |

### Gaps

- Physics constants unknown (gravity, jump velocity)
- Water detection undefined
- Collision response not specified
- Ghost interpolation for terrain height

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Terrain height at position | Wrong collision | Interpolate |
| Water buoyancy formula | Wrong physics | Research |
| Jump depends on grounded state | Wrong input | Verify state machine |
| Raycast through terrain | Wrong LOS | Test edge cases |

---

# PHASE 4: GAME LOGIC (Weeks 15-20)

## Milestone 4.1: Input System (Week 15)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 4.1.1 | Implement keyboard input (Bevy) | M3.3 complete | Keys detected |
| 4.1.2 | Implement mouse position | 4.1.1 | Position detected |
| 4.1.3 | Implement mouse buttons | 4.1.2 | Clicks detected |
| 4.1.4 | Implement gamepad detection | 4.1.3 | Gamepad found |
| 4.1.5 | Implement gamepad buttons | 4.1.4 | Buttons work |
| 4.1.6 | Implement gamepad axes | 4.1.5 | Axes work |
| 4.1.7 | Map A3D key codes to input events | 4.1.6 | Codes mapped |
| 4.1.8 | Route input to game systems | 4.1.7 | Input flows |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A2 | WGPU/Bevy input works | From Phase 1 | Low |
| - | A3D key code mapping | **UNDOCUMENTED** | High |

### Gaps

- A3D key code format undocumented
- Input event buffering not specified
- Input state reset behavior not specified

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Platform key translation | Wrong keys | Test all platforms |
| Input latency | Laggy | Keep < 16ms |
| Focus handling | Wrong input | Verify focus |

---

## Milestone 4.2: Character Controller (Weeks 16-17)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 4.2.1 | Implement 8-directional movement | M4.1 complete | Movement works |
| 4.2.2 | Implement speed limits | 4.2.1 | Speed capped |
| 4.2.3 | Implement camera follow | 4.2.2 | Camera follows |
| 4.2.4 | Implement Q/E rotation | 4.2.3 | Rotation works |
| 4.2.5 | Implement zoom control | 4.2.4 | Zoom works |
| 4.2.6 | Implement fly mode | 4.2.5 | Fly mode works |
| 4.2.7 | Implement smooth interpolation | 4.2.6 | Smooth movement |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A5 | Camera math replicable | **VERIFY** | Medium |
| - | Camera parameters (offset, smoothing) | **UNKNOWN** | Medium |

### Gaps

- Camera math details not specified
- Fly mode toggle key binding
- Interpolation smoothing values

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Q/E rotation conflicts with perspective | Wrong view | Test both modes |
| Zoom affects focal length | Perspective changes | Recalculate |
| Fly mode needs no gravity | Falls otherwise | Disable physics |

---

## Milestone 4.3: Combat System (Weeks 18-19)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 4.3.1 | Implement melee attack (sword) | M4.2 complete | Attack works |
| 4.3.2 | Implement attack animation | 4.3.1 | Animation plays |
| 4.3.3 | Implement hit detection (distance-based) | 4.3.2 | Hits detect |
| 4.3.4 | Implement ranged attack (crossbow) | 4.3.3 | Range works |
| 4.3.5 | Implement projectile physics | 4.3.4 | Projectile flies |
| 4.3.6 | Implement damage calculation | 4.3.5 | Damage applies |
| 4.3.7 | Implement knockback | 4.3.6 | Knockback works |
| 4.3.8 | Implement death handling | 4.3.7 | Death works |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| - | Animation frame sequences | **UNKNOWN** | High |
| - | Damage formula | **UNKNOWN** | Medium |

### Gaps

- Attack animation frame sequences undocumented
- Crossbow projectile physics
- Blood particle system

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Attack timing (frame 21 for melee) | Wrong hit | Extract from C++ |
| Damage formula (rand() % 100) | Wrong damage | Replicate |
| Knockback physics | Wrong push | Research |

---

## Milestone 4.4: AI System (Week 20)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 4.4.1 | Implement enemy spawning (EnemyGen) | M4.3 complete | Enemies spawn |
| 4.4.2 | Implement target selection | 4.4.1 | Target chosen |
| 4.4.3 | Implement movement toward target | 4.4.2 | Movement works |
| 4.4.4 | Implement stuck detection | 4.4.3 | Stuck detected |
| 4.4.5 | Implement stuck resolution (4 stages) | 4.4.4 | Unstuck works |
| 4.4.6 | Implement4.4. buddy AI | 5 | Buddy works |
| 4.4.7 | Implement shoot_by tracking | 4.4.6 | Tracking works |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| - | EnemyGen format | **UNKNOWN** | High |
| - | AI state machine | **PARTIAL** | Medium |

### Gaps

- EnemyGen format undocumented
- Buddy AI state machine details
- shoot_by priority system

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Stuck resolution 4 stages | AI stuck forever | Implement all |
| Follower count affects targeting | Wrong target | Verify formula |
| Buddy vs enemy behavior | Wrong AI | Separate states |

---

# GATE SUMMARY

| Gate | Must Complete | Enables |
|------|---------------|---------|
| 3 | M2.1 | M3.1 |
| 4 | M2.2, M2.3 | M3.1 |
| 5 | M2.2 | M3.1 |
| 6 | M2.3 | M3.1 |
| 7 | M3.1 | M3.2 |
| 8 | M3.2 | M3.3 |
| 9 | M3.3 | M4.1 |
| 10 | M4.1 | M4.2 |
| 11 | M4.2 | M4.3 |
| 12 | M4.3 | M4.4 |
| 13 | M4.4 | M5.1 |

---

# SEQUENTIAL TASK SUMMARY

| Phase | Milestone | Tasks | Gate |
|-------|-----------|-------|------|
| **Phase 3** | 3.1 Terrain Quadtree | 12 | 3-6 |
| **Phase 3** | 3.2 BSP World | 12 | 7 |
| **Phase 3** | 3.3 Collision/Physics | 7 | 8 |
| **Phase 4** | 4.1 Input System | 8 | 9 |
| **Phase 4** | 4.2 Character Controller | 7 | 10 |
| **Phase 4** | 4.3 Combat System | 8 | 11 |
| **Phase 4** | 4.4 AI System | 7 | 12 |

---

# PHASE 3+4 CRITICAL DATA DEPENDENCIES

| Dependency | Status | Location |
|------------|--------|----------|
| .a3d terrain format (Corrected: terrain uses .a3d binary format, not .xp — see FAILURE_LOG F003) | **PARTIAL** | Need header layout |
| .a3d world format | **PARTIAL** | Need instance structures |
| A3D key codes | **UNKNOWN** | Need platform.h mapping |
| Animation timing | **UNKNOWN** | Need C++ source |
| EnemyGen format | **UNKNOWN** | Need C++ source |
| Physics constants | **UNKNOWN** | Need C++ source |
| Camera parameters | **UNKNOWN** | Need C++ source |

---

# VERIFICATION CHECKLIST BEFORE PHASES

### Before Phase 3
- [ ] Verify .a3d terrain format complete (Corrected: terrain uses .a3d binary format, not .xp — see FAILURE_LOG F003)
- [ ] Verify .a3d world format complete
- [ ] Verify BSP insertion algorithm
- [ ] Verify physics constants from C++

### Before Phase 4
- [ ] Verify A3D key code mapping
- [ ] Verify animation frame data
- [ ] Verify EnemyGen format
- [ ] Verify camera parameters

---

*Technical Audit Complete: Phase 3 & 4*
