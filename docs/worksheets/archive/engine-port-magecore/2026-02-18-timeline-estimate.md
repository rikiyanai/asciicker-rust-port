---
title: "Timeline Estimate for Engine Port"
type: research
status: REFERENCE
date: 2026-02-18
# blocked_by: docs/worksheets/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md (not applicable to Rust port)
---

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

# Timeline Estimate for Engine Port

## Status: REFERENCE (originally deferred in C++ project)

---

## Prerequisite Timeline

### Pipeline Closeout (Historical -- not applicable to Rust port)

| Step | Duration | Description |
|------|----------|-------------|
| Step 0 | 1 day | Try It First |
| Step 1 | 0.5 day | WebUI Hotfix Audit |
| Step 2 | 0.5 day | Fix Policy Gating |
| Step 3 | 1-2 days | Targeted Fixes (conditional) |
| Step 4 | 1 day | Gate Verification |
| **Total** | **2-5 days** | |

*(Original C++ project blocker -- not applicable to the Rust port.)*

---

## Phase 1: Core Rendering Port

**Duration Estimate:** 4-8 weeks

### Week 1-2: Project Setup

| Task | Duration | Deliverable |
|------|----------|-------------|
| Create `mage-port` crate | 1 day | Cargo project structure |
| Define module layout | 1 day | `src/{rasterizer,sample,material,palette}.rs` |
| Port Sample struct | 1 day | `Sample { visual, diffuse, spare, height }` |
| Port SampleBuffer | 2 days | 2x supersampled buffer with border |
| Unit tests | 2 days | Sample/SampleBuffer tests |

### Week 3-4: Rasterizer Core

| Task | Duration | Deliverable |
|------|----------|-------------|
| Port Rasterize template → trait | 3 days | `rasterize<S: Sample, H: Shader>()` |
| Port Bresenham template → trait | 1 day | `bresenham<S: Sample>()` |
| Port terrain shader | 2 days | `TerrainShader` implementation |
| Port mesh shader | 2 days | `MeshShader` implementation |
| Unit tests | 2 days | Triangle/line rasterization tests |

### Week 5-6: Material System

| Task | Duration | Deliverable |
|------|----------|-------------|
| Port Material struct | 1 day | `Material { shade: [[MatCell; 16]; 4] }` |
| Port auto_mat generation | 3 days | RGB555 → {bg,fg,gl} lookup |
| Port MatCell blending | 2 days | Transparency, multiply, screen modes |
| Integration | 2 days | Connect to rasterizer |
| Unit tests | 2 days | Material lookup tests |

### Week 7-8: Integration

| Task | Duration | Deliverable |
|------|----------|-------------|
| Render() main entry | 2 days | 6-stage pipeline |
| RenderPatch callback | 2 days | Terrain rendering |
| RenderMesh callback | 2 days | Mesh rendering |
| Test scene | 2 days | Render comparison vs C++ |
| Optimization pass | 2 days | Profile and optimize |

---

## Phase 2: Asset Pipeline Integration

**Duration Estimate:** 2-4 weeks

**Conditional on:** Phase 1 PASS (P1-1, P1-2, P1-3)

### Week 1: XP Loader

| Task | Duration | Deliverable |
|------|----------|-------------|
| Gzip parsing | 1 day | Header validation, decompression |
| XP header parsing | 1 day | version, layers, width, height |
| Layer extraction | 2 days | Column-major cell reading |
| Swoosh merging | 2 days | Layer 3+ handling |
| Unit tests | 1 day | XP file parsing tests |

### Week 2: Sprite Assembly

| Task | Duration | Deliverable |
|------|----------|-------------|
| Atlas layout parsing | 2 days | angles, anims, projs from Layer 0 |
| Frame subdivision | 2 days | Grid extraction |
| Color quantization | 1 day | RGB → xterm-256 |
| Height encoding | 1 day | Layer 1 glyph interpretation |
| Integration | 2 days | Load player-0100.xp |

### Week 3-4: Font System

| Task | Duration | Deliverable |
|------|----------|-------------|
| Port font1.cmap | 1 day | ASCII → atlas index mapping |
| Port font1.xadv | 1 day | Variable-width advances |
| Port Font1Paint | 2 days | Text rendering |
| Skin recoloring | 1 day | Grey/Gold/Pink palettes |
| Integration | 2 days | UI text rendering |
| Testing | 1 day | Font rendering tests |

---

## Phase 3: Game Logic Migration

**Duration Estimate:** 8-16 weeks

**Conditional on:** Phase 2 PASS (P2-1, P2-2, P2-3)

### Week 1-2: ECS Foundation

| Task | Duration | Deliverable |
|------|----------|-------------|
| Choose ECS crate | 1 day | hecs or bevy_ecs |
| Define components | 2 days | Position, Yaw, Animation, Equipment, etc. |
| Define systems | 2 days | Physics, Animation, Input systems |
| Basic scheduler | 1 day | System ordering |
| Testing | 1 day | ECS unit tests |

### Week 3-4: Character System

| Task | Duration | Deliverable |
|------|----------|-------------|
| Character entity | 2 days | Human struct as entity bundle |
| Equipment state | 2 days | Mount, Armor, Helmet, Shield, Weapon |
| Animation state | 2 days | Action state machine |
| Sprite selection | 2 days | Equipment → sprite atlas lookup |
| Testing | 2 days | Character creation/modification tests |

### Week 5-6: Input Handling

| Task | Duration | Deliverable |
|------|----------|-------------|
| Keyboard input | 2 days | UTF-8 → CP437 conversion |
| Mouse input | 1 day | Position, click handling |
| Gamepad input | 2 days | Configurable mappings |
| Touch input | 2 days | Gesture recognition |
| Testing | 1 day | Input event tests |

### Week 7-8: Physics

| Task | Duration | Deliverable |
|------|----------|-------------|
| Collision detection | 3 days | Ray-world intersection |
| Movement physics | 2 days | Velocity, gravity |
| Terrain collision | 2 days | Height-based collision |
| Testing | 1 day | Physics tests |

### Week 9-12: World System

| Task | Duration | Deliverable |
|------|----------|-------------|
| BSP tree structure | 3 days | Node types, traversal |
| Instance management | 2 days | Create, delete, update |
| Mesh loading | 3 days | AKM format parsing |
| World serialization | 2 days | A3D format read/write |
| Frustum culling | 2 days | QueryWorld callback |
| Testing | 2 days | World system tests |

### Week 13-16: Integration & Polish

| Task | Duration | Deliverable |
|------|----------|-------------|
| Game loop integration | 2 days | Connect all systems |
| UI rendering | 3 days | HP bar, inventory, menus |
| macOS build | 2 days | Apple Silicon support |
| Performance optimization | 3 days | Profile, optimize hot paths |
| Bug fixes | 4 days | Integration testing |
| Documentation | 2 days | README, API docs |

---

## Total Timeline

| Phase | Duration | Conditional On |
|-------|----------|----------------|
| Pipeline Closeout | 2-5 days | N/A (Rust port) |
| Phase 1: Core Rendering | 4-8 weeks | - |
| Phase 2: Asset Pipeline | 2-4 weeks | Phase 1 PASS |
| Phase 3: Game Logic | 8-16 weeks | Phase 2 PASS |
| **Total** | **15-29 weeks** | |

### Best Case: 15 weeks (3.5 months)
### Worst Case: 29 weeks (7 months)

---

## Milestone Checkpoints

| Milestone | Week | Gate | Go/No-Go |
|-----------|------|------|----------|
| Pipeline Verification | 0 | Closeout Step 4 | Both gates PASS |
| First Render | 4 | P1-1 | Test scene matches C++ |
| Sample Buffer Parity | 6 | P1-2 | Unit tests pass |
| Material System | 8 | P1-3 | Auto-mat matches |
| XP Loading | 10 | P2-1 | player-0100.xp loads |
| Sprite Rendering | 12 | P2-2 | Sprite on screen |
| Font Rendering | 14 | P2-3 | Text on screen |
| Character System | 18 | P3-1 | Entity with equipment |
| Input Handling | 20 | P3-2 | Keyboard/mouse/gamepad |
| macOS Build | 24 | P3-3 | Builds on Apple Silicon |
| Playable Demo | 28 | All | Core mechanics working |

---

## Resource Requirements

### Personnel

| Role | Allocation | Duration |
|------|------------|----------|
| Lead Developer | Full-time | All phases |
| Rust Expert | Part-time | Phase 1-2 |
| Artist/Designer | As needed | Phase 3 |
| QA | Part-time | All phases |

### Infrastructure

| Resource | Purpose |
|----------|---------|
| macOS machine (Apple Silicon) | Build target |
| Linux machine | CI runner |
| Windows machine | Cross-platform testing |
| GPU (optional) | Performance testing |

---

## Risk Buffer

Add 20% buffer for:
- Unexpected bugs
- Learning curve (Rust, WGPU)
- Platform-specific issues
- Scope creep

**Buffered Timeline:** 18-35 weeks (4-8 months)

---

## References

- Closeout Plan: `docs/worksheets/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md`
- Capability Matrix: `2026-02-18-capability-matrix.md`
- Architecture Mapping: `2026-02-18-architecture-mapping.md`
- Mage Core: `/Users/r/Projects/ascii research/Mage-core/`
