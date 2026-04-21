# Terrain.cpp Bug Fix Plan

This document outlines the critical bugs identified in terrain.cpp that must be fixed before any Rust porting work begins.

---

## Bug Description with Code Context

### Bug 1: terrain.cpp:613 — Double `if (x)` Instead of `if (y)`

**Location:** `GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y)` overload (terrain.cpp:589-615)

**Function Purpose:** Inverse query — given a Patch, return its world coordinates (x, y)

**Bug Description:**
Line 613 checks `if (x)` twice when it should check `if (y)` in one of the conditions. This is a logic error in the output assignment during the quadtree coordinate reconstruction algorithm.

**Code Context (from terrain.cpp:589-615):**
```cpp
void GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y) {
    // Algorithm: Ascends from patch to root, extracting quadrant bits 
    // at each level to reconstruct coordinates
    
    // BUG: Line 613 checks `if (x)` twice; second should be `if (y)`
    // The inverse query reconstructs (x, y) by walking up the quadtree
    // and extracting bit coordinates at each level
    
    // Output: Sets *x and *y to world-relative coordinates
    // (subtracts t->x, t->y from internal coordinates)
}
```

**Variables in Scope:**
- `t` — Terrain pointer (contains base offset t->x, t->y)
- `p` — Patch pointer (the patch to find coordinates for)
- `x` — Output int pointer for x coordinate (world-relative)
- `y` — Output int pointer for y coordinate (world-relative)

**What the Bug Does:**
The algorithm walks from the patch up to the root, extracting quadrant bits at each level to reconstruct the coordinates. When assigning the reconstructed coordinates to the output parameters, the code incorrectly uses `if (x)` twice instead of checking `if (y)` for the y-coordinate assignment. This would cause the y-coordinate to be incorrectly computed or possibly not written at all.

---

### Bug 2: terrain.cpp:480,492 — Boundary Condition Uses `>` Instead of `>=`

**Location:** `Tap3x3::Sample()` function (terrain.cpp:413-555)

**Function Purpose:** Helper class for height map analysis with boundary handling

**Bug Description:**
Lines 480 and 492 use `>` instead of `>=` for boundary conditions. The comment says "assuming '>' is fresher" — this needs verification.

**Code Context (from terrain.cpp:413-555):**
```cpp
// In Tap3x3::Sample() - boundary clamping function
// If coordinate falls outside [0, HEIGHT_CELLS], adjusts to neighbor 
// patch and re-projects

// Known TODO (line 480, 492): Boundary condition uses `>` instead of `>=`
// (comment says "assuming '>' is fresher" — needs verification)

// NULL safety: If neighbor doesn't exist, clamps to edge (line 500-514)
```

**Variables in Scope:**
- `x`, `y` — Input coordinates to sample within the patch
- Returns height value at the sampled location

**What the Bug Does:**
Using `>` instead of `>=` means the boundary edge cases are handled differently. If the comment suggests "assuming '>' is fresher", this might be intentional, but it should be verified. The issue is that the exact boundary behavior is uncertain and needs verification against the original intent.

---

### Bug 3: terrain.cpp:805 (Line 1671) — Variable Scope Error

**Location:** `QueryTerrainSample(Patch* p, int x, int y, ...)` (terrain.cpp:1630-1691)

**Function Purpose:** Sample height at center of each visual cell (8×8 grid), invoking callback with interpolated 3D coordinates

**Bug Description:**
Line 1671 contains condition `u < y` where `y` parameter is out of scope in that context.

**Code Context (from terrain.cpp:1630-1691):**
```cpp
void QueryTerrainSample(Patch* p, int x, int y, 
    void(*cb)(Patch* p, int u, int v, double coords[3], void* cookie), 
    void* cookie) {
    // Grid sampling: Centers of visual cells mapped to height field
    // via bilinear interpolation
    
    // Callback signature: cb(p, u, v, coords, cookie) where:
    // - u, v are local patch coordinates (0-7 for 8x8 visual cells)
    // - coords = {x + u + 0.5, y + v + 0.5, interpolated_height}
    
    // BUG: Line 1671 contains condition `u < y` where `y` parameter 
    // is out of scope in that context
}
```

**Variables in Scope:**
- `p` — Patch pointer
- `x`, `y` — World coordinates (passed to callback for offset calculation)
- `u`, `v` — Local patch coordinates (0-7 for 8×8 visual cells)
- In the callback context, `y` (world y) is NOT available, only `v` (local patch y)

**What the Bug Does:**
The code incorrectly references `y` (the world y-coordinate parameter) instead of `v` (the local patch y-coordinate). This is a classic variable scope error where an outer scope variable is incorrectly used instead of the correct local loop variable.

---

## Proposed Fixes

### Fix 1: terrain.cpp:613

**Exact Code Change:**
```cpp
// Before (BUGGY):
if (x) {
    // ... x coordinate assignment
}
if (x) {  // BUG: should be if (y)
    // ... y coordinate assignment  
}

// After (FIXED):
if (x) {
    // ... x coordinate assignment
}
if (y) {  // FIXED: correctly check y pointer
    // ... y coordinate assignment
}
```

**Note:** The exact fix depends on the actual code at line 613. The second condition should check the `y` pointer (or a derived y value) to determine whether to assign the y-coordinate, matching the pattern used for the x-coordinate.

---

### Fix 2: terrain.cpp:480,492

**Exact Code Change:**
```cpp
// Before (NEEDS VERIFICATION):
if (coord > HEIGHT_CELLS) {  // Line 480 or 492
    // handle out of bounds
}

// After (VERIFY FIRST):
// Option A - if '>' is correct (fresher behavior):
// Keep as-is, but document the intentional behavior
//
// Option B - if '>=' was intended:
if (coord >= HEIGHT_CELLS) {
    // handle out of bounds
}
```

**Verification Required:** Examine the surrounding code comments and test with boundary inputs to determine if `>` or `>=` is the correct behavior.

---

### Fix 3: terrain.cpp:805 (Line 1671)

**Exact Code Change:**
```cpp
// Before (BUGGY):
// In callback or loop context within QueryTerrainSample
if (u < y) {  // BUG: y is world coord, should be v (local patch coord)
    // ... some condition
}

// After (FIXED):
if (u < v) {  // FIXED: v is local patch y-coordinate (0-7)
    // ... correct condition
}
```

**Note:** The exact fix depends on the context at line 1671. The variable `y` (world y-coordinate) should be replaced with `v` (local patch y-coordinate in the 8×8 visual cell grid).

---

## Related Bugs to Fix Together

| Bug ID | File | Line | Severity | Description |
|--------|------|------|----------|-------------|
| BUG-001 | terrain.cpp | 613 | CRITICAL | `if (x)` appears twice, should check `y` |
| BUG-002 | terrain.cpp | 480, 492 | HIGH | Boundary `>` vs `>=` assumption |
| BUG-003 | terrain.cpp | 805 (1671) | CRITICAL | Condition `u < y` where `y` out of scope |

**Why Fix Together:**
- All three bugs are in the terrain system and affect coordinate handling
- BUG-001 and BUG-003 both involve coordinate variable confusion
- BUG-002 affects the same Sample() function that BUG-003 may relate to
- All three bugs could cause incorrect terrain rendering or crashes
- Fixing them together ensures the terrain coordinate system is consistent

---

## How to Verify the Fixes

### Verification for Bug 1 (terrain.cpp:613)

1. **Code Review:** Read the actual code at line 613 to confirm the exact fix
2. **Unit Test:** Create test that:
   - Creates a terrain with patches at known coordinates
   - Calls `GetTerrainPatch(Terrain* t, Patch* p, int* x, int* y)` 
   - Verifies the returned (x, y) match the known patch location
3. **Integration Test:** Use in-game patch query from multiple locations and verify coordinates

### Verification for Bug 2 (terrain.cpp:480,492)

1. **Code Review:** Check the comment history to understand the "fresher" note
2. **Boundary Test:** Create patches and sample at exact boundaries:
   - Sample at x = HEIGHT_CELLS (should trigger boundary handling)
   - Sample at x = HEIGHT_CELLS - 1 (should be in-bounds)
3. **Visual Test:** Render terrain with height differences at boundaries and verify no gaps

### Verification for Bug 3 (terrain.cpp:805/1671)

1. **Code Review:** Read line 1671 to see the exact context
2. **Callback Test:** 
   - Register a callback that logs all (u, v) pairs received
   - Verify all 64 (0-7, 0-7) pairs are visited
3. **Shadow Test:** Run terrain shadow calculation and verify shadows render correctly

### General Terrain Verification

1. **Height Map:** Create terrain, modify heights, verify correct retrieval
2. **Neighbor Queries:** Test 8-neighbor lookup around patch boundaries
3. **Quadtree Operations:** Add/delete patches, verify tree structure remains valid
4. **Memory Safety:** Run with valgrind/ASan to detect any memory issues

---

## Dependencies

- **Requires:** Access to original terrain.cpp source code for exact line verification
- **Blocks:** Any terrain-related Rust porting work (terrain.cpp is HIGH priority in porting)

---

## References

- Audit Report: `research-bug-assumption-audit.md` — Section 4 (Potential Bugs)
- Architecture: `terrain_cpp_part1.md` — Functions at lines 183-203, 730-744, 795-806
- Implementation Plan: `research-implementation-plan.md` — Section 4 (Terrain System)
