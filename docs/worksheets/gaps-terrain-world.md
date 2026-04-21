> **STATUS: ACTIVE GAP ANALYSIS** — Generated 2026-02-20. CORRECTIONS: HEIGHT_CELLS=4 (not 8), vertex grid is 5×5 (not 9×9). See terrain.h:60.

# Gap Analysis: Terrain and World Systems

## Overview

This document identifies areas NOT covered in the existing terrain.cpp and world.cpp documentation:

- `/Users/r/Projects/asciicker rust port/docs/worksheets/arch/terrain_cpp_part1.md`
- `/Users/r/Projects/asciicker rust port/docs/worksheets/arch/terrain_cpp_part2.md`
- `/Users/r/Projects/asciicker rust port/docs/worksheets/arch/world_cpp_part1.md`
- `/Users/r/Projects/asciicker rust port/docs/worksheets/arch/world_cpp_part2.md`

---

## 1. BSP Tree Features Not Documented

### 1.1 Ancestor Cleanup - STUBBED Implementation

**Status:** NOT IMPLEMENTED - Memory Leak Risk

The BSP tree in world.cpp has STUBBED ancestor cleanup code. When instances are deleted from BSP leaves, the code does not walk up the tree to collapse empty parent nodes. This causes memory to accumulate over many delete operations.

**Locations (world.cpp):**
- MeshInst deletion: lines 922-971 marked "do ancestors cleanup // ..."
- SpriteInst deletion: lines 1031-1081
- ItemInst deletion: lines 1146-1197
- TODO comment at lines 1140-1145

**Impact:** After many instance deletions, the BSP tree may contain empty internal nodes that are never reclaimed.

**Rust Port Implication:** This is a bug to replicate - the Rust implementation should either implement proper ancestor cleanup or document this as a known limitation.

### 1.2 NODE_SHARE Straddling Instance Handling

**Status:** PARTIALLY DOCUMENTED

The BSP tree has a BSP_TYPE_NODE_SHARE node type for instances that straddle split planes. However:

- SplitBSP creates BSP_NodeShare nodes (line 1578 comments mention this capability)
- The code allocates NODE_SHARE size for future upgrades
- The actual straddling instance detection is NOT implemented

**Current Behavior:** Instances that span split planes are not handled specially - they go into leaf nodes rather than being tracked at internal nodes.

**Reference:** world.cpp:1578 - "these should become NODE_SHARE later"

### 1.3 BSP Tree Instance Insertion Algorithm (BSP::InsertInst)

**Status:** NOT DETAILED IN EXISTING DOCS

The documentation covers Rebuild() and SplitBSP(), but does not detail BSP::InsertInst() which handles dynamic insertion:

**Location:** world.cpp:5394-5525

**Algorithm Summary:**
1. Start at node, compute bbox overlap with instance
2. If NODE: check both children overlap; recurse into overlapping children; create NODE_SHARE if both children overlap
3. If NODE_SHARE: check children overlap; iterate existing share list for duplicates; add to share list
4. If LEAF: add instance to leaf's instance list
5. If empty tree: add to list without splitting

**Not Documented:** The duplicate detection logic and the decision tree for when to create new nodes vs. add to existing leaves.

### 1.4 Eight HitWorld Variants - Plane Inequality Details

**Status:** PARTIALLY COVERED

world_cpp_part2.md covers HitWorld3-7 but does not detail the specific plane inequalities used for each octant.

**Each variant uses different hardcoded plane tests:**

| Variant | Ray Direction | Plane Tests |
|---------|---------------|-------------|
| HitWorld0 | (+X, +Y, +Z) | 6 plane-box tests |
| HitWorld1 | (-X, +Y, +Z) | 6 plane-box tests |
| HitWorld2 | (+X, -Y, +Z) | 6 plane-box tests |
| HitWorld3 | (-X, -Y, +Z) | 6 plane-box tests |
| HitWorld4 | (+X, +Y, -Z) | 6 plane-box tests |
| HitWorld5 | (-X, +Y, -Z) | 6 plane-box tests |
| HitWorld6 | (+X, -Y, -Z) | 6 plane-box tests |
| HitWorld7 | (-X, -Y, -Z) | 6 plane-box tests |

**Not Documented:** The specific inequality formulas at lines 2086-2091, 2232-2237, etc. These encode the ray direction assumptions to reject non-intersecting bboxes early.

---

## 2. Terrain LOD and Culling Details

### 2.1 No LOD System - SAH Not Used

**Status:** CONFIRMED NOT IMPLEMENTED

The terrain quadtree does NOT use Level of Detail (LOD). Specifically:

- **No SAH (Surface Area Heuristic):** The quadtree expansion is based on coordinate bounds, not cost-based splitting
- **Simple "grow upward" strategy:** Tree expands when coordinates fall outside current bounds
- **All patches have identical resolution:** 8x8 visual cells, 5x5 height vertices (HEIGHT_CELLS=4) (Corrected: HEIGHT_CELLS=4, not 8. See terrain.h:60)

**Reference:** audit-reaudit-terrain.md confirms "There is no Surface Area Heuristic (SAH) implementation in terrain.cpp"

### 2.2 Radius Culling - Circle-AABB Test

**Status:** MENTIONED BUT NOT DETAILED

QueryTerrain with radius parameter (terrain.cpp:1921-1989) performs circle-AABB collision:

**Not Documented Details:**
- Computes squared distances from circle center to 4 corners of rect
- If all 4 corners inside circle (hit == 4): fast non-culled recursion for children
- If not all inside: per-child radius test
- Axis-aligned strip tests: if fit_x or fit_y, checks strip overlap

**Algorithm (terrain.cpp:222-226):**
```
- For each corner of AABB: compute squared distance to circle center
- If all corners inside: use fast path
- Otherwise: test axis-aligned strips for overlap
```

### 2.3 Terrain Frustum Culling - Plane Removal Optimization

**Status:** COVERED but optimization not detailed

The QueryTerrain with frustum planes (terrain.cpp:1803-1904) has an optimization not fully documented:

**Not Documented Detail:**
- If all 8 corners of AABB on positive side of a plane, that plane is removed from further checks
- This progressively simplifies the culling test as the ray traverses deeper into the tree
- When all planes eliminated, switches to faster non-culled query

---

## 3. Collision Detection Details

### 3.1 Plucker Ray Representation

**Status:** NOT DOCUMENTED IN TERRAIN/WORLD DOCS

The ray format uses Plucker coordinates for efficient geometric tests:

**Format (world.cpp:2972-2980):**
```
ray[0-2] = p × v (cross product - plane equation)
ray[3-5] = v (direction)
ray[6-8] = p (origin)
ray[9] = FLT_MAX (distance threshold - updated on closest hit)
```

**Not Documented:** The mathematical basis for why Plucker coordinates enable efficient ray-box tests, and how the 8 HitWorld variants leverage this.

### 3.2 positive_only Flag - Backface Culling

**Status:** MENTIONED BUT NOT EXPLAINED

The HitWorld and HitPatch functions have a `positive_only` parameter:

**Not Documented Details:**
- When true: only hits triangles with normals facing the ray (backface culling)
- Used for reflection rays where you don't want to hit the back of surfaces
- Comment at world.cpp:2227-2230 indicates future optimization for rays starting above geometry

### 3.3 Ray-Triangle Intersection - Per-Octant Optimization

**Status:** NOT DOCUMENTED

The 8 HitWorld variants use different plane inequalities optimized for specific ray directions:

**Not Documented:** Why 8 variants? The optimization works because:
- For each octant, certain plane-Bbox tests are always false
- By hardcoding which tests to skip, the code avoids unnecessary floating-point operations
- The sign_case is computed from ray direction: bit0=X sign, bit1=Y sign, bit2=Z sign

### 3.4 Terrain Ray-Patch Intersection - Diagonal Handling

**Status:** PARTIALLY COVERED

HitPatch (terrain.cpp:2007-2092) tests each 8x8 cell with 2 triangles:

**Not Documented Details:**
- diag bitfield determines triangle split direction per cell
- If diag bit set: diagonal from (hx, hy) to (hx+1, hy+1)
- If diag bit clear: diagonal from (hx, hy+1) to (hx+1, hy)
- Each cell requires 2 RayIntersectsTriangle calls

---

## 4. Serialization Edge Cases

### 4.1 Format Version Handling - Legacy vs Modern

**Status:** COVERED in audit-unknown-a3d-format.md but not in main docs

The .a3d format has version detection:

**Not in terrain_cpp_part2.md / world_cpp_part2.md:**
```
if (num_of_instances < 0) {
    format_version = -num_of_instances;  // Negative = versioned
    read num_of_instances;
}
```

**Versions:**
- Legacy: first int32 >= 0 is directly instance count (no version field)
- Modern: first int32 < 0, then version, then count

### 4.2 story_id Conditional Reading

**Status:** NOT IN MAIN DOCS

world.cpp:5086-5088 has conditional story_id reading:

**Not Documented:**
```cpp
if (format_version > 0) {
    read story_id;  // Only in versioned format
}
```

### 4.3 INST_VOLATILE Filtering During Save

**Status:** NOT EMPHASIZED

SaveWorld skips editor-only instances:

**Not Documented Detail (world.cpp:4830):**
- INST_VOLATILE flag instances are NOT saved
- Only runtime (WORLD) items are persisted
- Editor (EDIT) items are recreated from templates at load time

### 4.4 Enemy Generator Serialization

**Status:** NOT IN MAIN DOCS

Enemy generators are serialized in a separate section:

**Format (44 bytes each):**
| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0 | 12 | float[3] | pos XYZ |
| 12 | 4 | int32 | alive_max |
| 16 | 4 | int32 | revive_min |
| 20 | 4 | int32 | revive_max |
| 24 | 4 | int32 | armor |
| 28 | 4 | int32 | helmet |
| 32 | 4 | int32 | shield |
| 36 | 4 | int32 | sword |
| 40 | 4 | int32 | crossbow |

### 4.5 Mesh Name Lookup During Load

**Status:** NOT IN MAIN DOCS

LoadWorld resolves mesh instance names by searching the loaded mesh list:

**Not Documented (world.cpp:5115-5121):**
- Iterates through head_mesh to tail_mesh
- Compares loaded name with saved mesh_id
- No caching - O(n) per instance
- If mesh not found: instance creation fails silently (returns null but continues)

---

## 5. Memory Management Details

### 5.1 ItemInst Pool Allocator - Cache Behavior

**Status:** PARTIALLY COVERED but not detailed

The ItemInst uses a free pool:

**Not Documented Details:**
- Global `item_inst_cache` head pointer
- AllocItemInst: pops from cache head OR malloc if empty
- FreeItemInst: pushes to cache head (no size limit)
- PurgeItemInstCache: frees entire cache at shutdown
- **No thread safety** - not an issue for single-threaded game

**Pattern:** LIFO free list (stack) - most recently freed items reused first.

### 5.2 TexHeap Texture Allocation

**Status:** MENTIONED but not detailed

Terrain patches use TexHeap for GPU memory:

**Not Documented Details:**
- Each patch has TexAlloc managing 2 texture slots (height + visual)
- When TEXHEAP defined: GPU allocation happens on AddTerrainPatch
- When TEXHEAP not defined: no GPU texture management
- UpdateTerrainHeightMap and UpdateTerrainVisualMap upload to GPU slots
- TerrainDispose frees TexAlloc if it was the last reference

### 5.3 Patch/Node Allocation Strategy

**Status:** NOT IN MAIN DOCS

**Not Documented:**
- AddTerrainPatch: malloc for new Nodes and Patch
- DelTerrainPatch: free for Patch, Nodes trimmed if empty
- TerrainAttach: allocates intermediate nodes during descent
- TerrainDetach: frees nodes during ascent if they become empty

### 5.4 Dynamic Array Growth in GetAllTerrainPatches

**Status:** NOT IN MAIN DOCS

GetAllTerrainPatches uses dynamic growth:

**Not Documented:**
- Starts at cap = 16
- Doubles cap on each reallocation (cap = cap * 2)
- Growth strategy: exponential with power-of-2
- No shrink - array never shrinks after growth

---

## Summary of Gaps

| Category | Gap | Severity | Source Location |
|----------|-----|----------|-----------------|
| BSP | Ancestor cleanup STUBBED | HIGH | world.cpp:922-1197 |
| BSP | NODE_SHARE not implemented | MEDIUM | world.cpp:1578 |
| BSP | InsertInst algorithm | MEDIUM | world.cpp:5394-5525 |
| BSP | 8 HitWorld plane details | LOW | world.cpp:2086-2930 |
| Terrain | No LOD/SAH confirmed | DOCUMENTED | audit-reaudit-terrain |
| Terrain | Radius culling algorithm | LOW | terrain.cpp:1921-1989 |
| Terrain | Plane removal optimization | LOW | terrain.cpp:1803-1904 |
| Collision | Plucker ray format | MEDIUM | world.cpp:2972-2980 |
| Collision | positive_only usage | LOW | world.cpp:2227 |
| Collision | 8-variant octant rationale | LOW | world.cpp:3016 |
| Serialization | Version handling | MEDIUM | world.cpp:5019-5038 |
| Serialization | story_id conditional | LOW | world.cpp:5086-5088 |
| Serialization | INST_VOLATILE filter | LOW | world.cpp:4830 |
| Serialization | Enemy gen format | LOW | world.cpp:5000+ |
| Serialization | Mesh name lookup O(n) | LOW | world.cpp:5115-5121 |
| Memory | Pool cache unbounded | LOW | world.cpp:567-592 |
| Memory | TexHeap slot management | MEDIUM | terrain.cpp:1590-1598 |
| Memory | Dynamic array growth | LOW | terrain.cpp:3275-3310 |

---

## Recommendations for Rust Port

1. **Critical:** Document the ancestor cleanup bug - either implement proper cleanup or explicitly replicate the limitation

2. **High:** Implement proper NODE_SHARE handling for straddling instances if performance becomes an issue

3. **Medium:** The Plucker ray format should use a named struct for clarity (ray[10] is confusing)

4. **Medium:** Consider bounded pool or alternative to unbounded ItemInst cache

5. **Low:** Cache mesh name lookups during LoadWorld for O(1) repeated access
