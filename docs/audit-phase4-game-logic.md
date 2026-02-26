# Technical Audit: Implementation Phase 4 (Weeks 15-20)
## Game Logic

**Audit Date:** 2026-02-20  
**Phase:** 4 - Game Logic  
**Scope:** Milestones 4.1, 4.2, 4.3, 4.4

---

## Executive Summary

This audit examines the Game Logic implementation phase (Weeks 15-20) of the Asciicker Rust Port. The phase consists of four milestones covering Input System, Character Controller, Combat System, and AI System. Analysis draws from the implementation plan, C++ architecture documentation, physics system documentation, and the gaps analysis document.

**Key Findings:**
- All four milestones have clear dependencies forming a linear progression
- The gaps document identifies 5 MEDIUM severity items that overlap with Phase 4 tasks
- Several assumptions require verification before implementation can proceed
- Test strategies are under-specified across all milestones

---

## Milestone 4.1: Input System (Week 15)

### Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 4.1.1 | Implement keyboard input | Use Bevy input system for key detection |
| 4.1.2 | Implement mouse input | Mouse position and click detection |
| 4.1.3 | Implement gamepad input | Controller button/axis detection |
| 4.1.4 | Map A3D key codes | Translate original key codes to input events |

### Assumptions

| ID | Assumption | Risk Level |
|----|------------|------------|
| A2 | WGPU backend works on target platforms | Low - WGPU is well-tested |
| - | Bevy 0.18+ input API is stable | Medium - API changes possible |
| - | A3D key code mapping is documented | High - Unknown format |
| - | Gamepad API supports hot-plugging | Medium - Platform dependent |

### Gaps

1. **A3D Key Code Mapping**: The original key code format is not documented in the provided materials. Need to reverse-engineer from C++ source or find documentation.

2. **Input Latency Target**: Success criteria states "< 16ms" but doesn't specify measurement methodology or whether this is end-to-end latency.

3. **Input State Persistence**: No specification for input buffering during pause/menu states.

4. **Platform-Specific Handling**: Gamepad calibration and dead-zone handling not specified.

### Gotchas

- Bevy's input system changed significantly between versions; ensure API compatibility
- Keyboard input may require platform-specific handling for non-standard layouts
- Mouse capture/release needs careful state management to avoid losing focus
- Gamepad connection/disconnection events need graceful handling

### Dependencies

| Dependency | Milestone | Status Required |
|------------|-----------|-----------------|
| Physics System (3.3) | M3.3 | Complete |
| PhysicsIO interface | M3.3 | Working |
| Character entity setup | M3.3 | Complete |

### Test Strategy

**Unit Tests:**
- Key code mapping unit tests (compare against C++ reference)
- Input event generation verification
- Gamepad button mapping tests

**Integration Tests:**
- End-to-end input-to-movement pipeline
- Simultaneous multi-input handling (e.g., run + jump)
- Input during menu/pause transitions

**Performance Tests:**
- Input polling latency measurement
- Frame timing analysis for 16ms requirement

---

## Milestone 4.2: Character Controller (Weeks 16-17)

### Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 4.2.1 | Implement 8-directional movement | Arrow key movement in 8 directions |
| 4.2.2 | Implement camera following | Camera tracks player position |
| 4.2.3 | Implement Q/E camera rotation | Left/right camera rotation |
| 4.2.4 | Implement zoom control | View distance adjustment |
| 4.2.5 | Implement fly mode | Debug free-camera movement |

### Assumptions

| ID | Assumption | Risk Level |
|----|------------|------------|
| A5 | C++ rendering behavior can be exactly replicated in Rust | Medium - Subtle differences possible |
| - | Camera math (focal, view_pos, view_dir) from Phase 1 is complete | High - Critical dependency |
| - | Physics integration via PhysicsIO works correctly | Medium - Requires testing |
| - | Transform system handles camera entity correctly | Low - Bevy built-in |

### Gaps

1. **Camera Mathematics**: The gaps document identifies camera controls (scene_shift, cam_shift, zoom) as requiring implementation. The relationship between player direction, camera yaw, and facing direction needs clarification.

2. **Fly Mode Toggle**: From plan-game-logic-gaps.md - fly mode is controlled by PURE_TERM in C++ but should be toggleable in the port. Key binding not specified.

3. **Interpolation/Smoothing**: Camera following should use smooth interpolation (0.1 factor per frame per C++). Implementation details for Bevy tweening not specified.

4. **8-Directional Sprite Selection**: Animation system must select correct sprite based on quantized direction (0, 45, 90, 135, 180, 225, 270, 315 degrees).

5. **player_stp Counter**: PhysicsIO provides step counter for animation. Integration path not fully specified.

### Gotchas

- Camera and player facing are separate in C++ - camera yaw != player direction
- Zoom implementation affects projection matrix; need to verify aspect ratio handling
- Fly mode must disable physics constraints but maintain transform updates
- Q/E rotation accumulates - need to track cumulative rotation vs absolute
- Collision response during movement needs careful sync with physics system

### Dependencies

| Dependency | Milestone | Status Required |
|------------|-----------|-----------------|
| Input System | M4.1 | Complete |
| Physics System | M3.3 | Complete |
| PhysicsIO interface | M3.3 | Working |
| Camera math (perspective) | M1.3 | Complete |
| Sprite system | M2.3 | Complete |

### Test Strategy

**Unit Tests:**
- Direction quantization accuracy (8 directions)
- Zoom level bounds checking
- Fly mode toggle state machine

**Integration Tests:**
- Movement through collision geometry
- Camera rotation with player movement
- Zoom transitions (smooth vs instant)
- Fly mode physics bypass verification

**Visual Regression Tests:**
- Compare output against C++ reference for known camera positions
- Verify sprite selection matches C++ for all 8 directions

---

## Milestone 4.3: Combat System (Weeks 18-19)

### Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 4.3.1 | Implement melee attack | Sword attack with animation |
| 4.3.2 | Implement ranged attack | Crossbow projectile system |
| 4.3.3 | Implement damage calculation | HP reduction on hit |
| 4.3.4 | Implement knockback | Impulse application on hit |
| 4.3.5 | Implement death handling | Fall animation, death state |

### Assumptions

| ID | Assumption | Risk Level |
|----|------------|------------|
| A5 | C++ behavior replication | Medium - Complex state machine |
| - | Animation frame timing constants are correct (30ms idle, 30ms fall, 20ms attack) | Low - Documented in C++ |
| - | Sprite system supports action state transitions | Medium - Requires verification |
| - | Damage formula (rand() % 100) is intentional | Low - Documented behavior |

### Gaps

1. **Attack Animation Frames**: From game_logic_cpp.md - sword attack uses specific frame indices at specific times. Hit test occurs at frame 21. Complete frame sequence needs extraction from C++.

2. **Crossbow Projectile**: The C++ shows arrow release but projectile physics not fully documented. Need to determine if this is client-side prediction or server-authoritative.

3. **Damage Formula**: Current implementation uses simple `rand() % 100`. The gaps document notes this is simplified. Future enhancement path not specified.

4. **Mount Dismounting**: When mounted character dies, they dismount instead. State transition logic needs careful implementation.

5. **Blood Particle System**: Blood leak rendering on terrain uses PaintTerrain(). Terrain modification system integration needed.

6. **Animation-Driven Hit Testing**: Hit detection is time-based within animation, not physics-based. This is architecturally significant.

### Gotchas

- Cannot change equipment during attack animation (SetWeapon returns false)
- Crossbow freezes movement during attack (x_force, y_force = 0)
- Attack animation frame 2 is start for non-crossbow weapons
- Death direction faces away from attacker
- hit_tested flag prevents multiple damage applications per attack

### Dependencies

| Dependency | Milestone | Status Required |
|------------|-----------|-----------------|
| Character Controller | M4.2 | Complete |
| Sprite System | M2.3 | Complete |
| Physics (knockback) | M3.3 | Complete |
| Animation System | M2.3 | Working |

### Test Strategy

**Unit Tests:**
- State machine transition validation (allowed vs disallowed transitions)
- Frame timing accuracy
- Damage calculation bounds (0-99)

**Integration Tests:**
- Melee hit detection at various distances
- Direction checking (90-degree arc)
- Knockback impulse application
- Death animation completion
- Equipment change blocking during attack

**Visual Regression Tests:**
- Attack animation sequence
- Blood splatter placement
- Death direction orientation

---

## Milestone 4.4: AI System (Week 20)

### Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 4.4.1 | Implement enemy spawning | Spawn from enemygen positions |
| 4.4.2 | Implement target selection | Choose nearest/combat-relevant target |
| 4.4.3 | Implement movement toward target | Pathfinding or direct movement |
| 4.4.4 | Implement stuck detection | Track position history |
| 4.4.5 | Implement stuck resolution | Escalating recovery behaviors |
| 4.4.6 | Implement buddy AI | Friendly NPC companion behavior |

### Assumptions

| ID | Assumption | Risk Level |
|----|------------|------------|
| A5 | C++ AI behavior replication | Medium - Complex logic |
| - | EnemyGen data structure is loaded | High - Need to verify |
| - | Target selection formula is correct | Low - Documented in C++ |
| - | Stuck detection thresholds work in practice | Medium - May need tuning |

### Gaps

1. **EnemyGen Format**: Spawning requires enemy generator data. The .a3d format contains this but may need verification.

2. **Buddy AI Details**: From plan-game-logic-gaps.md - buddy system is explicitly called out as needing implementation. 2 buddies spawn at game start with specific offset positions.

3. **Follower System**: Target selection weights followers in calculation. Implementation needed for tracking follower counts.

4. **shoot_by Priority**: When a character is shot, they get priority as target for 5 seconds. Event system integration needed.

5. **Buddy AI State Machine**: Idle, Following, Fighting, Helping states. Transition logic needs implementation.

6. **Stuck Resolution Escalation**: 4 stages (100-400 stuck counter) with different behaviors. Complete state machine needed.

### Gotchas

- Target distance weighting uses `(followers + 4)` factor
- Crossbow enemies maintain 10-unit distance; melee engage at 3 units
- Stuck resolution tries: reverse direction, perpendicular movement, keep jumping, then reset
- Buddy spawn offset is (2, 0, 2) from player
- AI only engages if within 20 units of enemy and 40 units of master
- Buddy help triggers when player HP < 30%

### Dependencies

| Dependency | Milestone | Status Required |
|------------|-----------|-----------------|
| Combat System | M4.3 | Complete |
| Character Entity System | M3.2 | Complete |
| Physics System | M3.3 | Complete |
| EnemyGen loading | M3.2 | Working |

### Test Strategy

**Unit Tests:**
- Target selection priority ordering
- Stuck detection threshold accuracy
- State machine transitions

**Integration Tests:**
- Enemy pathfinding around obstacles
- Multiple AI coordination (buddy system)
- Combat engagement/disengagement
- Stuck recovery behavior

**Simulation Tests:**
- Large-scale enemy counts
- Buddy AI with multiple enemies
- Edge cases (target death mid-combat)

---

## Cross-Milestone Analysis

### Dependency Flow

```
M3.3 (Physics) → M4.1 (Input) → M4.2 (Controller) → M4.3 (Combat) → M4.4 (AI)
```

### Shared Components Identified

| Component | Used By | Notes |
|-----------|---------|-------|
| PhysicsIO | M4.1, M4.2, M4.3, M4.4 | Central interface |
| Character Entity | M4.2, M4.3, M4.4 | All game logic |
| Transform | M4.2, M4.4 | Movement, AI |
| Action State Machine | M4.3, M4.4 | Combat, Animation |
| SpriteReq | M4.2, M4.3 | Equipment visualization |

### Overlapping Items with Gaps Document

The plan-game-logic-gaps.md identifies these items that overlap with Phase 4:

| Gap Item | Phase 4 Coverage | Notes |
|----------|------------------|-------|
| Fly Mode | M4.2.5 | Debated - appears in gaps but is in implementation plan |
| Camera Controls | M4.2.3, M4.2.4 | scene_shift, cam_shift, zoom |
| AI Behaviors | M4.4.1-4.4.6 | Follower system, buddy AI |
| Multiplayer Lag | Not covered | Defer to Phase 5+ |
| UI Features | Not covered | Defer to Phase 5 |

### Risk Summary

| Risk | Likelihood | Impact | Milestone |
|------|------------|--------|-----------|
| A3D key code mapping unknown | High | Medium | 4.1 |
| Animation timing mismatch | Medium | High | 4.2, 4.3 |
| Camera following smoothness | Medium | Medium | 4.2 |
| Combat hit detection timing | Medium | High | 4.3 |
| AI stuck resolution effectiveness | Medium | Medium | 4.4 |
| Buddy AI state complexity | Medium | Medium | 4.4 |

---

## Recommendations

### Immediate Actions Before Starting Phase 4

1. **Verify A3D Key Code Mapping**: Extract key code definitions from C++ source or reverse-engineer from game binaries.

2. **Document Animation Frame Sequences**: Extract complete frame data for all action types from C++ source.

3. **Verify EnemyGen Format**: Confirm enemy generator data structure is accessible from .a3d loading.

4. **Complete PhysicsIO Integration**: Ensure the physics interface is fully functional before building input on top.

### Implementation Order Recommendation

1. **Start with M4.1**: Input system is foundational
2. **M4.2 benefits from incremental build**: Can test movement before full camera
3. **M4.3 requires animation completion**: Wait until sprite system is verified
4. **M4.4 is most complex**: Allow extra time for AI调试

### Testing Infrastructure

Establish these before Phase 4:
- Input recording/playback for regression testing
- Frame capture for visual comparison
- AI behavior logging for debugging
- Physics state snapshot for deterministic testing

---

## Appendix: Document References

| Document | Purpose |
|----------|---------|
| IMPLEMENTATION_PLAN.md | Phase definitions and gates |
| arch/game_logic_cpp.md | C++ architecture reference |
| arch/physics_cpp.md | Physics system reference |
| plan-game-logic-gaps.md | Gap analysis and implementation plans |

---

*End of Technical Audit - Phase 4 Game Logic*
