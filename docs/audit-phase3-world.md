# Technical Audit: Implementation Phase 3 (Weeks 9-14) — World Systems

## Overview

This document provides a detailed technical audit of Implementation Phase 3, which covers World Systems including terrain quadtrees, BSP world management, and collision/physics. The audit analyzes each milestone's task breakdown, assumptions, gaps, gotchas, dependencies, and test strategy.

**Source Documents:**
- `/Users/r/Projects/asciicker rust port/docs/IMPLEMENTATION_PLAN.md` (Phase 3, lines 172-236)
- `/Users/r/Projects/asciicker rust port/docs/arch/terrain_cpp_part1.md`
- `/Users/r/Projects/asciicker rust port/docs/arch/world_cpp_part1.md`
- `/Users/r/Projects/asciicker rust port/docs/gaps-terrain-world.md`

**Phase Duration:** Weeks 9-14 (6 weeks total)
**Prerequisite Gate:** Milestone 2.1 complete (6-Stage Pipeline)

---

## Milestone 3.1: Terrain Quadtree (Weeks 9-10)

### 1. Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 3.1.1 | Implement .a3d terrain file loader | Parse binary .a3d terrain format; extract height maps and visual/material data (Corrected: terrain uses .a3d binary format, not .xp) |
| 3.1.2 | Implement quadtree data structure | Create Rust quadtree with Node and Patch types; implement level-based tree navigation |
| 3.1.3 | Implement patch creation and expansion | Handle AddTerrainPatch with auto-growth when coordinates fall outside current bounds |
| 3.1.4 | Implement neighbor resolution | Implement GetTerrainNeighbor using two-phase ascent-then-descent algorithm |
| 3.1.5 | Implement height interpolation (bilinear) | Implement bilinear interpolation for querying heights at arbitrary (x, y) coordinates |
| 3.1.6 | Implement diagonal orientation (Tap3x3) | Calculate diagonal bitfield using height gradient comparison for mesh triangulation |
| 3.1.7 | Implement height bounds propagation | Implement UpdateNodes to propagate lo/hi bounds up the tree after patch modifications |
| 3.1.8 | Integrate terrain into TERRAIN stage | Connect terrain quadtree to the existing 6-stage rendering pipeline |

### 2. Assumptions

| ID | Assumption | Verification |
|----|------------|--------------|
| D1 | All .a3d terrain file formats are fully reverse-engineered | Must have complete format specification from research phase (Corrected: terrain uses .a3d binary format, not .xp) |
| A5 | C++ rendering behavior can be exactly replicated in Rust | Terrain geometry must match C++ output exactly |
| - | Perspective math (focal, view_pos, view_dir) is complete | Required for terrain projection |
| - | HEIGHT_CELLS=4, VISUAL_CELLS constants verified | 5x5 vertices per patch, 8x8 visual cells |
| - | .a3d terrain binary file reading works (binary file reading (terrain .a3d is NOT gzip compressed)) | Terrain file loader |

### 3. Gaps

| Gap | Severity | Details | Source |
|-----|----------|---------|--------|
| No LOD system | HIGH | Terrain quadtree does NOT use Level of Detail; all patches have identical resolution (8x8). No Surface Area Heuristic (SAH) for cost-based splitting. | gaps-terrain-world.md |
| .a3d terrain format details | HIGH | IMPLEMENTATION_PLAN references .a3d terrain loader but format specifics (header, data layout) not detailed in Phase 3 tasks (Corrected: terrain uses .a3d binary format, not .xp) | IMPLEMENTATION_PLAN.md |
| Radius culling algorithm | MEDIUM | QueryTerrain with radius performs circle-AABB collision but algorithm details not in task list | gaps-terrain-world.md |
| Frustum culling details | MEDIUM | Plane removal optimization (when all 8 corners on positive side) not specified | gaps-terrain-world.md |
| Ghost patch generation | LOW | CalcTerrainGhost generates boundary data for non-existent patches but not in task list | terrain_cpp_part1.md |
| TexHeap/GPU upload | MEDIUM | UpdateTerrainHeightMap and UpdateTerrainVisualMap upload to GPU; TEXHEAP conditional compilation not addressed | terrain_cpp_part1.md |

### 4. Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Quadtree coordinate system | HIGH | Terrain uses world coordinates relative to Terrain::x, Terrain::y offset. GetTerrainPatch returns NULL if outside [0, 1<<level) range. |
| Level semantics | HIGH | Level 0 = single patch (z >= 0 in CreateTerrain), Level -1 = empty tree. Binary search via bit extraction: `i = ((x >> lev) & 1) \| (((y >> lev) & 1) << 1)` |
| Auto-expand edge cases | HIGH | Four expand loops handle out-of-bounds: x<0 (quad 0 or 2), y<0 (quad 0 or 1), x>=range (quad 2 or 3), y>=range (quad 1 or 3). Order matters. |
| Neighbor flag sync | MEDIUM | After AddTerrainPatch, ALL 8 neighbors' flags must be updated (bidirectional marking). Missing this causes seam artifacts. |
| Diagonal orientation bug | MEDIUM | Tap3x3::Update uses `>` instead of `>=` for boundary conditions (line 480, 492 in terrain.cpp) — needs verification or fix |
| Patch::parent pointer | MEDIUM | Parent pointers must be maintained for UpdateNodes to traverse upward. Set during AddTerrainPatch descent. |
| Height interpolation edge cases | MEDIUM | Corners: copy from diagonal neighbors if exist else fallback z. Edges: linear interpolation. Interior: inverse-distance weighted average. |
| DeleteTerrain root trim | MEDIUM | If root has only one non-NULL child after deletion, promote child and decrement level. Common edge case. |

### 5. Dependencies

| Dependency | Source | Required For |
|------------|--------|--------------|
| Milestone 2.1: 6-Stage Pipeline | IMPLEMENTATION_PLAN | TERRAIN stage integration point |
| .a3d terrain format research | D1 assumption | 3.1.1 file loader |
| binary file reading | Phase 2.3 | 3.1.1 .a3d terrain loading (binary file reading (terrain .a3d is NOT gzip compressed)) |
| auto_mat lookup (Phase 2.2) | Not direct dependency | Visual/material rendering |
| Perspective math | Phase 1.3 | Terrain projection in rendering |

### 6. Test Strategy

| Test | Method | Success Criteria |
|------|--------|------------------|
| Single patch terrain | Load minimal .a3d terrain with one patch | Height map loads, single patch created |
| Quadtree expansion | Add patches at increasing coordinates | Tree expands correctly, levels increment |
| Height query | Query heights at patch centers and edges | Bilinear interpolation matches C++ |
| Neighbor resolution | Query all 8 neighbors of a patch | Returns correct patches or NULL at boundaries |
| Height bounds propagation | Modify patch height, check ancestors | lo/hi bounds propagate correctly |
| Diagonal orientation | Visual inspection of terrain mesh | Triangles align with height gradients |
| Visual output match | Compare Rust render to C++ reference | Pixel-perfect match for test terrain |
| Ghost patch generation | Query non-existent patch coordinates | Ghost data interpolates from neighbors |

---

## Milestone 3.2: BSP World (Weeks 11-12)

### 1. Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 3.2.1 | Implement .a3d file loader | Parse .a3d world format; handle version detection (legacy vs modern), instance loading |
| 3.2.2 | Implement BSP tree data structure | Create Rust BSP with NODE, NODE_SHARE, LEAF, INST node types |
| 3.2.3 | Implement instance types | Implement MeshInst, SpriteInst, ItemInst with appropriate fields |
| 3.2.4 | Implement instance insertion | Implement BSP::InsertInst for dynamic insertion into tree |
| 3.2.5 | Implement Rebuild/SplitBSP | Implement tree construction using Surface Area Heuristic (SAH) |
| 3.2.6 | Implement spatial queries | Implement point, ray, and box queries against BSP tree |
| 3.2.7 | Implement mesh rendering | Connect instance rendering to WORLD stage |

### 2. Assumptions

| ID | Assumption | Verification |
|----|------------|--------------|
| D1 | All .a3d file formats are fully reverse-engineered | Must have complete format specification |
| A5 | C++ rendering behavior can be exactly replicated in Rust | Instance positioning must match |
| - | .xp sprite loading complete (Phase 2.3) | Sprite instances depend on sprite atlas |
| - | Mesh loading (.akm/PLY) complete | Mesh instances depend on mesh library |

### 3. Gaps

| Gap | Severity | Details | Source |
|-----|----------|---------|--------|
| Ancestor cleanup STUBBED | CRITICAL | When instances deleted from BSP leaves, code does NOT walk up to collapse empty parent nodes. Causes memory accumulation. | gaps-terrain-world.md |
| NODE_SHARE unimplemented | HIGH | BSP_TYPE_NODE_SHARE for straddling instances is allocated but detection not implemented. Instances span split planes go to leaves. | gaps-terrain-world.md |
| .a3d format details | HIGH | IMPLEMENTATION_PLAN references .a3d loader but serialization specifics (version handling, story_id, INST_VOLATILE) not in task list | gaps-terrain-world.md |
| 8 HitWorld variants | MEDIUM | Raycasting uses 8 variants for different ray direction octants; plane inequality details needed | gaps-terrain-world.md |
| Plucker ray format | MEDIUM | Ray format uses Plucker coordinates (ray[0-2]=p×v, ray[3-5]=v, ray[6-8]=p, ray[9]=distance) — needs documentation | gaps-terrain-world.md |
| positive_only flag | LOW | Backface culling for reflection rays — parameter purpose not in original plan | gaps-terrain-world.md |
| Mesh name lookup O(n) | LOW | LoadWorld iterates mesh list for each instance — no caching | gaps-terrain-world.md |

### 4. Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Instance lifecycle | HIGH | AddInst adds to flat list (not tree). Rebuild() extracts INST_USE_TREE instances and builds BSP. Must call Rebuild after instance changes. |
| INST_USE_TREE flag | HIGH | Instances without INST_USE_TREE stay in flat list; won't be in BSP queries. Editor vs. game filtering: editor sees INST_VOLATILE, game doesn't |
| SAH cost threshold | HIGH | SplitBSP creates leaf if best_cost too high. Need to determine threshold value from C++ behavior or make configurable |
| Instance bbox update | MEDIUM | MeshInst::UpdateBox recomputes world bbox by transforming all mesh vertices. Must call after transform changes |
| HitWorld plane tests | HIGH | 8 variants use different hardcoded inequalities based on ray direction octant. Must select correct variant from ray direction signs |
| ItemInst pooling | MEDIUM | ItemInst uses free pool ( AllocItemInst/FreeItemInst). Pool unbounded — may accumulate memory. Call PurgeItemInstCache at shutdown |
| Instance deletion leaks | HIGH | DelInst ancestor cleanup STUBBED — document as known limitation or implement proper cleanup |
| Straddling instances | MEDIUM | Instances spanning split planes not handled specially. May cause query misses if not in correct leaf |
| Format version detection | MEDIUM | If first int32 < 0: version = -num_of_instances, then read count. Legacy: first int32 >= 0 is count directly |
| story_id conditional | LOW | Only present in versioned format (> 0). Must check version before reading |
| INST_VOLATILE filtering | LOW | SaveWorld skips INST_VOLATILE instances. Editor items recreated from templates at load |

### 5. Dependencies

| Dependency | Source | Required For |
|------------|--------|--------------|
| Milestone 3.1: Terrain Quadtree | IMPLEMENTATION_PLAN | BSP queries may need terrain integration |
| Milestone 2.3: Sprite System | IMPLEMENTATION_PLAN | SpriteInst rendering |
| .a3d format research | D1 assumption | 3.2.1 file loader |
| Mesh loader (.akm/PLY) | Separate effort | MeshInst geometry |

### 6. Test Strategy

| Test | Method | Success Criteria |
|------|--------|------------------|
| Empty world load | Load minimal .a3d | Parses without error, empty BSP |
| Single instance | Add one MeshInst | Instance in flat list, Rebuild moves to BSP |
| BSP construction | Add multiple instances, call Rebuild | Tree partitions space correctly |
| Point query | Query BSP for point inside instance bbox | Returns correct instance |
| Ray query | Cast ray through scene | Returns closest hit, correct intersection point |
| Box query | Query BSP for box overlap | Returns all overlapping instances |
| Instance deletion | Delete instance from BSP | Instance removed, tree structure maintained |
| Ancestor cleanup | Delete many instances, check memory | Document if cleanup not implemented |
| Visual output match | Compare Rust render to C++ reference | Instances at correct positions |
| Format compatibility | Load legacy and modern .a3d files | Both parse correctly |

---

## Milestone 3.3: Collision/Physics (Weeks 13-14)

### 1. Task Breakdown

| # | Task | Description |
|---|------|-------------|
| 3.3.1 | Implement sphere-AABB collision | Test sphere against instance bounding boxes |
| 3.3.2 | Implement terrain height queries | Query quadtree for height at character position |
| 3.3.3 | Implement gravity and jumping | Apply downward acceleration, handle jump input |
| 3.3.4 | Implement water buoyancy | Detect water areas, reduce gravity when submerged |
| 3.3.5 | Implement line-of-sight raycasting | Use HitWorld variants for LOS checks |

### 2. Assumptions

| ID | Assumption | Verification |
|----|------------|--------------|
| A5 | C++ physics behavior can be exactly replicated in Rust | Must match gravity values, jump heights, buoyancy |
| - | Terrain quadtree operational (3.1 complete) | Height queries depend on terrain |
| - | BSP world operational (3.2 complete) | Collision detection depends on spatial queries |
| - | Input system NOT yet implemented | No keyboard/mouse for jump input yet — may need stub |

### 3. Gaps

| Gap | Severity | Details | Source |
|-----|----------|---------|--------|
| Physics constants | HIGH | Gravity, jump velocity, water buoyancy values not specified in IMPLEMENTATION_PLAN | IMPLEMENTATION_PLAN.md |
| Water detection method | HIGH | How to detect water areas? Terrain material ID? Separate water plane? | Not documented |
| Collision response | HIGH | IMPLEMENTATION_PLAN only lists detection, not response (push back, slide along surfaces) | IMPLEMENTATION_PLAN.md |
| Character controller | MEDIUM | Phase 4.2 (Weeks 16-17) covers movement and camera; collision may need to integrate with future controller | IMPLEMENTATION_PLAN.md |
| Physics timestep | MEDIUM | Fixed vs variable timestep, interpolation not addressed | - |
| Height at non-patch locations | MEDIUM | Terrain quadtree may not have patch at character location; need ghost interpolation | terrain_cpp_part1.md |
| Multiple collision surfaces | LOW | Character may touch multiple objects; priority/resolution not specified | - |
| Ray-box vs ray-mesh | LOW | Sphere-AABB is fast but mesh collision requires ray-face tests | world_cpp_part1.md |

### 4. Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Terrain height at character | HIGH | Character may be between patches. Must use ghost interpolation (CalcTerrainGhost) for heights at non-patch coordinates |
| Water buoyancy physics | HIGH | Need to determine: Is water a terrain material ID? Separate water plane? How is buoyancy calculated? |
| Rayclosest distance tracking | HIGH | HitWorld updates ray[9] with closest hit distance. Must initialize to FLT_MAX and check after query |
| positive_only for LOS | MEDIUM | LOS checks may want backface culling for reflection; pass positive_only=true for that case |
| Multiple ray variants | HIGH | Must select correct HitWorld variant based on ray direction octant (bits: X sign, Y sign, Z sign) |
| Collision detection scope | MEDIUM | Sphere-AABB is fast broad phase, but may need mesh face collision for accuracy. What's the requirement? |
| Physics-gravity coupling | MEDIUM | Gravity pulls down; terrain height query determines ground contact. Need clear state machine (grounded, jumping, falling) |
| Jump input dependency | HIGH | Input system is Phase 4.1 (Week 15). May need stub interface or delay integration until input ready |

### 5. Dependencies

| Dependency | Source | Required For |
|------------|--------|--------------|
| Milestone 3.1: Terrain Quadtree | IMPLEMENTATION_PLAN | 3.3.2 terrain height queries |
| Milestone 3.2: BSP World | IMPLEMENTATION_PLAN | 3.3.1 sphere-AABB, 3.3.5 raycasting |
| Milestone 4.1: Input System | IMPLEMENTATION_PLAN | 3.3.3 jump input (future integration) |

### 6. Test Strategy

| Test | Method | Success Criteria |
|------|--------|------------------|
| Sphere-AABB collision | Position sphere intersecting AABB | Collision detected, correct response |
| Terrain height query | Query at patch center, edge, corner | Returns interpolated height |
| Ghost height query | Query between patches | Returns interpolated ghost data |
| Gravity simulation | Apply gravity over time | Character accelerates downward |
| Ground collision | Character above terrain, apply gravity | Stops at terrain surface |
| Jump | Apply jump velocity, gravity | Character rises then falls |
| Water buoyancy | Move character into water area | Gravity reduced, character rises/sinks based on buoyancy |
| LOS raycast | Cast ray between two points | Returns hit/no-hit correctly |
| Ray closest hit | Cast ray through multiple objects | Returns closest intersection |

---

## Cross-Milestone Analysis

### Integration Points

| Integration | Between | Details |
|-------------|---------|---------|
| Terrain → BSP | 3.1 and 3.2 | Terrain provides height field; BSP provides object collision. Need unified collision system |
| BSP → Collision | 3.2 and 3.3 | Sphere-AABB uses instance bboxes from BSP. Ray queries use HitWorld functions |
| Terrain → Collision | 3.1 and 3.3 | Character position queries terrain for ground height. Water detection may use terrain materials |

### Shared Data Structures

| Structure | Used By | Notes |
|-----------|---------|-------|
| Terrain quadtree | 3.1, 3.3 | Height queries for physics |
| BSP tree | 3.2, 3.3 | Spatial queries for collision |
| Instance types | 3.2, 3.3 | Collision detection targets |
| HitWorld ray functions | 3.2, 3.3 | LOS and ray collision |

### Risk Summary

| Risk | Likelihood | Impact | Phase |
|------|------------|--------|-------|
| Ancestor cleanup bug | Certain | Memory leak | 3.2 |
| .xp/.a3d format gaps | Medium | Implementation blockers | 3.1, 3.2 |
| Physics constants unknown | High | Cannot match C++ behavior | 3.3 |
| Water detection undefined | High | Cannot implement buoyancy | 3.3 |
| Input/collision coupling | Medium | Integration delays | 3.3 |

---

## Recommendations

### Immediate Actions Before Phase 3 Start

1. **Complete .xp terrain format research** — Document header, compression, height map layout, visual/material data format
2. **Complete .a3d world format research** — Document version handling, instance serialization, story_id, INST_VOLATILE
3. **Document physics constants** — Extract gravity, jump velocity, buoyancy values from C++ source or gameplay
4. **Define water detection** — Determine how water areas are identified (material ID? separate layer?)

### Design Decisions Needed

1. **Ancestor cleanup** — Replicate bug or implement proper cleanup? Recommend: document as limitation initially
2. **Collision response** — Push-back only or full physics? Start with simple push-back
3. **Physics timestep** — Fixed 60Hz or variable? Recommend: fixed timestep for determinism

### Testing Infrastructure

1. **Golden file tests** — Compare terrain height queries and BSP spatial queries against C++ reference
2. **Physics unit tests** — Test gravity, jumping, buoyancy in isolation before integration
3. **Integration tests** — Test collision detection with actual game content

---

## Appendix: Key Constants Reference

### From terrain_cpp_part1.md

| Constant | Value | Usage |
|----------|-------|-------|
| HEIGHT_CELLS | 4 | Height vertices per patch dimension (5x5 total) |
| VISUAL_CELLS | 8 | Visual cells per patch dimension (8x8 total) |
| Quadtree levels | Dynamic | Expands on-demand, no fixed max level |

### From world_cpp_part1.md

| Constant | Value | Usage |
|----------|-------|-------|
| BSP node types | 4 | NODE, NODE_SHARE, LEAF, INST |
| HitWorld variants | 8 | Optimized for ray direction octants |
| Ray format | 10 doubles | Plucker coordinates + distance |

---

*Audit Document: 2026-02-20*
*Phase 3 World Systems Technical Audit*
