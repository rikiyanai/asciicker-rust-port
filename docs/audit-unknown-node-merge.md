# Audit: Terrain Quadtree Node Merge/Collapse Logic

**Source File**: `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp`  
**Date**: 2026-02-20  
**Status**: COMPLETE

---

## Summary

This document captures findings from research into terrain quadtree node merge/collapse logic in the original asciicker C++ codebase. The analysis covers:
1. Node merge or collapse functions
2. Node removal/merge when patches are deleted
3. The "ancestor cleanup" mentioned as STUBBED in audit notes

**Key Finding**: The terrain quadtree in `terrain.cpp` does NOT have explicit node merge/collapse functions. It only has "trim" logic that removes completely empty nodes. The STUBBED ancestor cleanup mentioned in audit notes is in `world.cpp` (BSP tree), NOT in `terrain.cpp`.

---

## 1. Node Merge/Collapse Functions

### Finding: NO Explicit Merge Functions Exist

There are **no functions** in `terrain.cpp` that explicitly merge or collapse sibling nodes into a single node. Specifically:

- **No `MergeNodes()` function** - No function combines 4 sibling patches into a parent node
- **No `CollapseNodes()` function** - No function promotes a single child to replace its parent
- **No "coalescing" logic** - The code does not check if siblings can be combined

### What DOES Exist: Node "Trim" Logic

Instead of merge/collapse, the code implements **"trim"** logic that removes nodes when they become completely empty:

1. **Leaf Trim** (lines 679-700 in `DelTerrainPatch()`)
2. **Root Trim** (lines 704-740 in `DelTerrainPatch()`)

These are NOT the same as merge/collapse:
- **Trim** = Remove node when ALL children are NULL
- **Merge/Collapse** = Combine non-empty siblings or promote single child

---

## 2. Nodes Removed/Merged When Patches Deleted

### Function: `DelTerrainPatch()`

**Location**: `terrain.cpp` lines 646-764

This function handles patch deletion with two types of cleanup:

### 2a. Leaf Trim (Lines 679-700)

```cpp
// leaf trim
QuadItem* q = p;

while (true)
{
    int c = 0;
    for (int i = 0; i < 4; i++)
    {
        if (n->quad[i] == q)
            n->quad[i] = 0;
        else
        if (n->quad[i])
            c++;
    }

    if (!c)
    {
        q = n;
        n = n->parent;
        free((Node*)q);
        t->nodes--;
    }
    else
        break;
}
```

**Behavior**: Walks UP the tree from the deleted patch. If a node has NO children remaining (c == 0), the node is freed. Continues until reaching a node with at least one child.

**Key Point**: This ONLY removes nodes when ALL 4 quadrants are empty. It does NOT merge or collapse partially-filled nodes.

### 2b. Root Trim (Lines 704-740)

```cpp
// root trim
n = (Node*)t->root;
while (true)
{
    int c = 0;
    int j = -1;
    for (int i = 0; i < 4; i++)
    {
        if (n->quad[i])
        {
            j = i;
            c++;
        }
    }

    if (c > 1)
        break;

    t->level--;

    if (j & 1)
        t->x -= 1 << t->level;
    if (j & 2)
        t->y -= 1 << t->level;

    t->root = n->quad[j];
    t->root->parent = 0;
    free(n);
    t->nodes--;

    if (t->level)
        n = (Node*)n->quad[j];
    else
        break;
}
```

**Behavior**: After leaf trim, if the root has only ONE child, that child becomes the new root and the tree level decrements. This effectively "collapses" one level of the tree.

**Key Point**: This IS a form of collapse, but only when the tree is maximally contracted (only 1 child at root).

### 2c. Neighbor Flag Cleanup (Lines 742-761)

After deletion, neighbor flags on adjacent patches are updated to remove references to the deleted patch:

```cpp
Patch* np[8] =
{
    flags & 0x01 ? GetTerrainPatch(t, x - 1, y - 1) : 0,
    // ... (8 neighbors)
};

for (int i = 0; i < 8; i++)
{
    if (np[i])
    {
        int j = (i + 4) & 7;
        np[i]->flags &= ~(1 << j);
    }
}
```

### Similar Function: `TerrainDetach()`

**Location**: `terrain.cpp` lines 2697-2812

The `TerrainDetach()` function (used for streaming) has identical trim logic:
- Leaf trim (lines 2727-2748)
- Root trim (lines 2752-2788)
- Neighbor flag cleanup (lines 2790-2809)

---

## 3. Ancestor Cleanup STUBBED - Clarification

### Finding: STUBBED is in world.cpp, NOT terrain.cpp

The audit notes mention "ancestor cleanup STUBBED" in reference to **BSP tree cleanup in world.cpp**, NOT terrain quadtree cleanup in terrain.cpp.

### Evidence from Audit Notes

From `docs/ENGINE_ARCHITECTURE.md` line 5409:
> **Notes:** Handles both tree and flat-list instances. Tree deletion is STUBBED for ancestor cleanup (lines 922-971 marked "do ancestors cleanup // ...").

From `world.cpp` line 33:
```cpp
//    traversal → DeleteInst() → ancestor cleanup (STUBBED, see lines ~1002-1048)
```

### What STUBBED Means in world.cpp

In `world.cpp`, when deleting instances from BSP tree leaves:
- The code removes the instance from the leaf
- The code that should "walk up tree to collapse empty parent nodes" is EMPTY (marked with comments like `// do ancestors cleanup // ...`)
- This causes empty BSP nodes to accumulate, degrading query performance over time

### Terrain.cpp Does NOT Have This Problem

The terrain quadtree implementation in `terrain.cpp` DOES properly clean up ancestors:
- Leaf trim removes empty nodes
- Root trim promotes single children
- This is NOT stubbed - it's fully implemented

---

## 4. Implications for Rust Port

### What Needs to Be Implemented

1. **Trim Logic (Already Implemented)**: The leaf trim and root trim logic in `DelTerrainPatch()` should be replicated in the Rust port

2. **No Merge Function Needed**: The quadtree doesn't need explicit merge/collapse because:
   - Trim handles the common case of removing empty nodes
   - The "grow upward" expansion strategy ensures patches can always be added
   - The tree naturally contracts through root trim

3. **Consider Adding Merge for Optimization**: If the Rust port wants to optimize memory further, consider adding:
   - **Sibling merge**: If 4 patches exist at the same level and could fit in a parent, combine them
   - This is NOT present in the C++ code but could be added as an optimization

### Key Code Locations to Port

| Function | Lines | Purpose |
|----------|-------|---------|
| `DelTerrainPatch()` | 646-764 | Patch deletion with trim |
| `TerrainDetach()` | 2697-2812 | Patch detachment with trim |
| `UpdateNodes()` | 617-644 | Propagate bounds up tree (called after changes) |

### Debug Considerations

The terrain.cpp includes debug output when `ASCIICKER_TERRAIN_DEBUG` environment variable is set. Consider similar debug infrastructure in Rust port.

---

## References

- `terrain.cpp`: Full implementation of quadtree operations
- `terrain.h`: Header with constants and public API
- `world.cpp`: Contains the STUBBED ancestor cleanup (BSP tree, not terrain)
- Existing audit: `docs/audit-unknown-terrain-expansion.md`
