# Asciicker Rust Port - MASTER ROADMAP
## Single Source of Truth for Research & Implementation Planning

**Phase:** Research (Active)  
**Status:** In Progress  
**Last Updated:** 2026-02-20

---

## TRAJECTORY - BEVY ENGINE (DECIDED)

### Previous Trajectory (DEPRECATED)
```
Asciicker C++ -> [Build Mage-core from scratch] -> Port game logic
```
**Problem**: Mage-core missing ECS, input, audio, UI - would take months to build.

### Chosen Trajectory (D001 - FINAL)
```
Asciicker C++ -> [Use Bevy Engine] -> Implement ASCII rendering (Mage-core style) -> Port game logic
```
**Bevy Engine** is the chosen foundation (Decision D001, 2026-02-19). Bevy provides a full engine (ECS, input, audio, UI) so we only need to implement custom ASCII rendering on top of it.

> **Note:** The `engine-port/` strategy documents are in `docs/engine-port/` (restored from archive) with STATUS: REFERENCE MATERIAL headers noting they predate the Bevy decision. Archive copies remain at `docs/archive/engine-port-magecore/`. New Bevy-aligned planning docs will be generated during GSD implementation planning.

---

## PHASE TRACKER

| Phase | Status | Progress |
|-------|--------|----------|
| 1. Codebase Audit | ✅ Complete | 100% |
| 2. Function Documentation | ✅ Complete | 100% (42 files) |
| 3. Rendering Deep Dive | ✅ Complete | 100% |
| 4. Alex Harri Research | ✅ Complete | 100% |
| 5. Bug & Assumption Audit | ✅ Complete | 100% |
| 6. Re-Audit Unknowns | ⏳ In Progress | 47% (42/89) |
| 7. Research Checkpoint | ✅ Complete | 100% |
| 8. Gap Analysis | ✅ Complete | 124 gaps |
| 9. Gap Plans (HIGH/MEDIUM) | ✅ Complete | 6 plans |
| 10. Implementation Planning Research | ✅ Complete | Deep dive docs |
| 11. Implementation | ⏳ Pending | ~5% (skeleton exists at asciicker-rust/, does NOT compile) |

---

## RESEARCH CATEGORIES STATUS

### Research Categories (17/17 Complete) ✅

| Category | Files | Status |
|----------|-------|--------|
| Core Rendering (render.cpp) | 2 docs | ✅ Complete |
| Terrain System (terrain.cpp) | 2 docs | ✅ Complete |
| World System (world.cpp) | 2 docs | ✅ Complete |
| Sprite System (sprite.cpp) | 1 doc | ✅ Complete |
| Audio System (audio.cpp) | 1 doc | ✅ Complete |
| Game Logic (game.cpp) | 1 doc | ✅ Complete |
| Input System (input.cpp) | 1 doc | ✅ Complete |
| Physics (physics.cpp) | 1 doc | ✅ Complete |
| Network (network.cpp) | 1 doc | ✅ Complete |
| UI/Menu (mainmenu.cpp) | 1 doc | ✅ Complete |
| Weather (weather.cpp) | 1 doc | ✅ Complete |
| Water Effects (water.cpp) | 1 doc | ✅ Complete |
| Editor (urdo.cpp) | 1 doc | ✅ Complete |
| Platform-specific | 1 doc | ✅ Complete |
| Alex Harri Integration | 2 docs | ✅ Complete |
| Bevy Engine Research | 2 docs | ✅ Complete |
| ECS Architecture / C++ DOD | 2 docs | ✅ Complete |

---

## 🔄 RESEARCH CHECKPOINT (Phase 7)

### Before Proceeding to Implementation

| Checkpoint Item | Status | Notes |
|-----------------|--------|-------|
| ✅ Trajectory decided (Bevy) | DONE | Using Bevy as engine foundation |
| ✅ Complete remaining HIGH unknowns (13) | IN PROGRESS | Now 12 resolved (still some remaining) |
| ✅ Research 9 remaining categories | DONE | All 17 categories complete |
| ✅ Document all findings | DONE | RESEARCH_CHECKPOINT.md created |

### Checkpoint Goals
1. **Resolve 13 remaining HIGH priority unknowns** - Partial (some remain)
2. **Complete critical research categories** (Game Logic, Input, Physics) - ✅ Done
3. **Document all findings in markdown** - ✅ Done
4. **Get explicit sign-off before implementation** - ⏳ PENDING

---

## UNKNOWN UNKNOWNS STATUS

| Category | Total | Resolved | Remaining | % |
|----------|-------|----------|-----------|---|
| CRITICAL | 6 | 6 | 0 | 100% |
| HIGH | 25 | 12 | 13 | 48% |
| MEDIUM | 30 | 15 | 15 | 50% |
| LOW | 28 | 9 | 19 | 32% |
| **TOTAL** | **89** | **42** | **47** | **47%** |

### Remaining Unknowns by Area

| Area | Count | Top Priority Items |
|------|-------|---------------------|
| Rendering | 4 | Matrix values, HEIGHT_SCALE |
| Terrain | 3 | Expansion threshold, .xp format |
| Integration | 4 | Font, k-d tree params, benchmarks |
| Serialization | 3 | .a3d versions, URDO format |
| Audio | 3 | stb_vorbis, sample unload |
| Sprite | 3 | XP layers, glyph table |
| **TOTAL** | **20** | - |

> NOTE: This table shows major categories only. Full breakdown in RE-AUDIT-MASTER.md

---

## PLANNING GAPS

### Architecture Decision (RESOLVED ✅)

| Decision | Previous | Now | Status |
|----------|----------|-----|--------|
| Engine foundation | Build Mage-core first | Use Bevy | ✅ RESOLVED |
| Rendering approach | Standalone WGPU | Bevy WGPU integration | ✅ RESOLVED |
| Input system | Build from scratch | Bevy built-in | ✅ RESOLVED |
| ECS | None in Mage-core | Bevy ECS | ✅ RESOLVED |

### Implementation Planning Gaps

| Gap | Severity | Blocks | Resolution Path |
|-----|----------|--------|-----------------|
| Perspective matrix values | HIGH | Perspective mode | Derive from code |
| Original font for vectors | HIGH | Visual accuracy | Contact Alex Harri |
| .xp file format spec | HIGH | Terrain loading | Reverse engineer |
| Performance benchmarks | HIGH | Optimization | Build & measure |
| k-d tree params | MEDIUM | Integration | Find in source |
| Cache quantization | MEDIUM | Integration | Analyze code |
| .a3d version history | MEDIUM | Serialization | Collect samples |
| URDO format | MEDIUM | Undo/redo | Reverse engineer |

### Decision Points

| Decision | Options | Status | Outcome |
|----------|---------|--------|---------|
| Perspective vs Isometric | Both/Isometric only | ✅ RESOLVED (D004-D005) | Perspective REQUIRED |
| auto_mat vs k-d tree | Keep/Replace | ✅ RESOLVED (D010) | Keep auto_mat initially, hybrid later |
| CPU vs GPU sampling | CPU-first/GPU | ✅ RESOLVED (D003) | Custom CPU rasterizer |
| 2D vs 6D vectors | 2D/6D | ⏳ Pending | Needs performance data |
| Ancestor cleanup | Implement/Skip | ⏳ Pending | Needs research |

---

## BUGS TO FIX (Pre-Port)

| Bug ID | File | Line | Status | Fix Plan |
|--------|------|------|--------|----------|
| TERRAIN-001 | terrain.cpp | 613 | ⏳ Pending | Change `if(x)` to `if(y)` |
| TERRAIN-002 | terrain.cpp | 805 | ⏳ Pending | Change `u < y` to `u < v` |
| TERRAIN-003 | terrain.cpp | 1671 | ⏳ Pending | Same as TERRAIN-002 |
| TERRAIN-004 | terrain.cpp | 480,492 | ⏳ Pending | Verify `>` vs `>=` intent |

---

## IMPLEMENTATION MILESTONES

### Existing Skeleton (asciicker-rust/)

> **NOTE:** A Bevy 0.18.0 skeleton (~385 LOC) exists at `asciicker-rust/`. It defines some components (Position, Sprite, Character, TerrainPatch, Camera) and has partial rendering stubs (SampleBuffer, RenderPhase enum, triangle rasterization). However it does NOT compile (4 missing modules), has zero tests, and 541MB of stale build artifacts. GSD Phase 1 will decide whether to salvage, restructure, or restart.

### Milestone 1: Pre-Port Prep (Not Started)
- [ ] Fix terrain.cpp bugs (Step 0) - C++ source
- [ ] Complete remaining research categories (9 areas)
- [ ] Resolve remaining HIGH priority unknowns (13)
- [ ] Finalize implementation plan with Bevy

### Milestone 2: Bevy Foundation (Not Started)
- [ ] Set up Rust project with Bevy dependency
- [ ] Implement ASCII triple-buffer (fg, bg, chars textures)
- [ ] Create custom render phase for ASCII pipeline
- [ ] Load font atlas (PNG or dynamic)
- [ ] Test basic ASCII rendering

### Milestone 3: Rendering Core (Not Started)
- [ ] Port SampleBuffer with 2x supersampling
- [ ] Port rasterization (Bresenham, triangle)
- [ ] Port 6-stage pipeline (CLEAR → TERRAIN → WORLD → SHADOW → REFLECTION → RESOLVE → SPRITES)
- [ ] Port RGB555 → xterm256 quantization
- [ ] Port auto_mat lookup table

### Milestone 4: Game Logic to ECS (Not Started)
- [ ] Map Asciicker structs to Bevy components
- [ ] Port Terrain as ECS component + systems
- [ ] Port World BSP as ECS component + systems
- [ ] Port Sprites as ECS component + systems
- [ ] Port Physics as ECS component + systems

### Milestone 5: Integration (Not Started)
- [ ] Integrate Alex Harri k-d tree (if decision = yes)
- [ ] Implement 6D vector matching
- [ ] Implement cache quantization
- [ ] Performance tuning

---

## KEY DECISIONS STATUS

Decisions required before Milestone 2:

1. **Perspective Mode**: ✅ RESOLVED (D004-D005) - Perspective is REQUIRED. Q/E rotation and toggle features depend on it.
2. **auto_mat vs k-d tree**: ✅ RESOLVED (D010) - Keep auto_mat initially for speed; hybrid approach with k-d tree added later (D030 deferred to after Phase 2).
3. **CPU vs GPU**: ✅ RESOLVED (D003) - Custom ASCII rendering via CPU rasterizer. No GPU sampling initially.
4. **Ancestor Cleanup**: Pending - Implement properly or document limitation?

---

## FILE INDEX

### Primary Documents
| File | Purpose |
|------|---------|
| `docs/agents/AGENTS.md` | Entry point |
| `MASTER_ROADMAP.md` | **This file** - single source of truth |
| `AUDIT_MANIFEST.md` | Audit manifest (archived — see docs/archive/) |

### Research Documents
| File | Purpose |
|------|---------|
| `docs/research-rendering-deep-dive.md` | Asciicker rendering analysis |
| `docs/research-bug-assumption-audit.md` | Original audit |
| `docs/RE-AUDIT-MASTER.md` | Re-audit results |
| `docs/audit-assumptions-verified.md` | Verified assumptions |
| `docs/alexharri_ascii_renderer_technology.md` | Alex Harri tech |

### Engine Research
| File | Purpose |
|------|---------|
| `docs/research-bevy-engine.md` | Bevy engine overview |
| `docs/research-ecs-architecture.md` | ECS architecture |
| `docs/research-cpp-architecture-analysis.md` | Asciicker DOD analysis |
| `docs/research-bevy-ascii-rendering.md` | ASCII in Bevy |
| `docs/research-bevy-magecore-integration.md` | **Integration strategy** |

### Implementation Planning
| File | Purpose |
|------|---------|
| `docs/research-implementation-plan.md` | Full implementation plan |
| `docs/implementation-plan-terrain-fix.md` | Terrain bug fixes |

---

## NEXT ACTIONS

### Immediate (This Session)
1. ✅ Complete remaining research categories - DONE (17/17)
2. ⏳ Resolve remaining 13 HIGH unknowns (12 resolved, 13 remain)
3. ✅ Make key architectural decisions - DONE (D001-D005, D010)

### Before Porting
1. Fix terrain.cpp bugs (Step 0)
2. ✅ Complete all research categories (17/17 done)
3. Finalize implementation plan

---

*This is the single source of truth for the Asciicker Rust Port project.*
*All other planning documents should reference this file.*
