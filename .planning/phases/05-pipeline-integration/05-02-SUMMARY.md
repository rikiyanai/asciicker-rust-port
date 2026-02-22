---
phase: 05-pipeline-integration
plan: 02
subsystem: world
tags: [bsp, sah, frustum-culling, spatial-acceleration, near-child-first]

# Dependency graph
requires:
  - phase: 02-asset-parsers
    provides: "A3dWorld parser with WorldInstance enum (Mesh, Sprite, Item)"
provides:
  - "RuntimeWorld resource with BSP tree construction via SAH"
  - "RuntimeInstance enum (Mesh, Sprite, Item) with bbox computation"
  - "Frustum-culled BSP traversal with near-child-first ordering"
  - "Sphere query for physics geometry collection (O(log n) via BSP pruning)"
  - "Instance flags (INST_VISIBLE, INST_USE_TREE, INST_VOLATILE, INST_SELECTED)"
  - "WorldPlugin registers RuntimeWorld resource"
affects: [05-pipeline-integration, 06-physics-and-character]

# Tech tracking
tech-stack:
  added: []
  patterns: ["BspNode enum with 4 node types", "SAH construction with median centroid split", "Near-child-first BSP traversal", "Sphere query via BSP pruning"]

key-files:
  created:
    - engine-port/src/world/instance.rs
    - engine-port/src/world/bsp.rs
  modified:
    - engine-port/src/world/mod.rs
    - engine-port/src/terrain/quadtree.rs

key-decisions:
  - "BspNode::NodeShare uses fixed-order traversal (no near-child-first) matching C++ behavior"
  - "Items always skip BSP tree (P5-066 FIX) and go to flat_list"
  - "Sphere query returns candidates at BSP node level, per-instance filtering done in RuntimeWorld::query_sphere"
  - "Split plane set to median centroid coordinate (P5-074 FIX)"

patterns-established:
  - "BspNode enum with Node/NodeShare/Leaf/Inst variants replaces C++ type tag + casts"
  - "SAH tests 3 axes, picks minimum cost split, classifies items into left/right/straddling"
  - "Frustum test with plane elimination: fully-inside planes removed from child tests"

requirements-completed: [WRLD-01, WRLD-02, WRLD-03, WRLD-04]

# Metrics
duration: 17min
completed: 2026-02-22
---

# Phase 5 Plan 02: BSP Tree Runtime Summary

**SAH-based BSP tree with 4 node types, frustum-culled near-child-first traversal, and sphere query for physics geometry collection**

## Performance

- **Duration:** 17 min
- **Started:** 2026-02-22T09:58:15Z
- **Completed:** 2026-02-22T10:15:31Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- RuntimeInstance enum (Mesh, Sprite, Item) with bbox computation from WorldInstance transform matrices
- BspNode enum with all 4 C++ node types: Node (split plane), NodeShare (straddling instances), Leaf, Inst
- SAH construction testing 3 axes with median centroid split plane and straddling classification
- Frustum-culled traversal with near-child-first ordering for front-to-back rendering
- Sphere query for physics geometry collection with O(log n) BSP pruning
- RuntimeWorld resource with build_from_parsed, query_visible, query_sphere
- WorldPlugin registers RuntimeWorld resource for other systems to access

## Task Commits

Each task was committed atomically:

1. **Task 1: RuntimeInstance types and BspNode enum with SAH construction** - `fba4f74` (feat)
2. **Task 2: RuntimeWorld resource and WorldPlugin integration** - `75e5b0a` (feat)

## Files Created/Modified
- `engine-port/src/world/instance.rs` - RuntimeInstance enum with bbox computation, flag accessors, WorldInstance conversion (337 lines)
- `engine-port/src/world/bsp.rs` - BspNode enum, SAH build, frustum query, sphere query (780 lines)
- `engine-port/src/world/mod.rs` - RuntimeWorld resource, WorldPlugin, query_visible, query_sphere (456 lines)
- `engine-port/src/terrain/quadtree.rs` - Fixed pre-existing Rust 2024 ref-pattern errors and clippy warnings (blocking fix)

## Decisions Made
- BspNode::NodeShare uses fixed-order [0, 1] traversal (no near-child-first), matching C++ behavior where NodeShare has no split_plane/split_axis
- Items always skip BSP tree even if USE_TREE flag is set (P5-066 FIX) since they are point-like and should be in the flat_list
- Sphere query at BSP level returns candidates; per-instance bbox filtering happens in RuntimeWorld::query_sphere which has access to individual instance data
- Split plane position is median centroid between the two partition groups (P5-074 FIX), not a placeholder zero

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing Rust 2024 ref-pattern errors in terrain/quadtree.rs**
- **Found during:** Task 1 (compilation)
- **Issue:** terrain/quadtree.rs used `if let Some(ref node) = child` which is an error in Rust 2024 edition (implicit borrowing makes explicit `ref` illegal)
- **Fix:** Removed `ref` from 5 pattern matches, added `#[allow(unused_variables)]` on one test
- **Files modified:** engine-port/src/terrain/quadtree.rs
- **Verification:** `cargo test --lib` and `cargo clippy -- -D warnings` pass
- **Committed in:** fba4f74 (Task 1 commit)

**2. [Rule 3 - Blocking] Fixed pre-existing clippy errors in terrain/quadtree.rs**
- **Found during:** Task 1 (clippy verification)
- **Issue:** Clippy `manual_flatten` warning for `for child in children { if let Some(node) = child` patterns
- **Fix:** Changed to `for node in children.iter().flatten()` in 2 locations
- **Files modified:** engine-port/src/terrain/quadtree.rs
- **Verification:** `cargo clippy -- -D warnings` clean
- **Committed in:** fba4f74 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 3 blocking: pre-existing Rust 2024 edition errors in terrain module)
**Impact on plan:** Both fixes necessary for compilation. No scope creep.

## Issues Encountered
- Sphere query test initially failed because 2-item BSP creates a Leaf (below MAX_LEAF_SIZE threshold), so leaf-level bbox check passes for both items. Fixed by using 10 items to force tree splits, allowing BSP pruning to work correctly.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- RuntimeWorld resource is available for Plan 05-05 (pipeline orchestrator) to query during the WORLD stage
- query_sphere API is available for Plan 06-01 (WorldGeometrySource) for physics geometry collection
- All 4 WRLD requirements verified: SAH construction, 4 node types, frustum-culled traversal, instance flags

---
*Phase: 05-pipeline-integration*
*Completed: 2026-02-22*
