# Implementation Plan: BSP Tree Ancestor Cleanup

## Overview

This document outlines the implementation plan for fixing the HIGH severity terrain gap: **Ancestor cleanup STUBBED in BSP tree**.

**Gap Reference:** `/Users/r/Projects/asciicker rust port/docs/worksheets/gaps-terrain-world.md` (Section 1.1)

**Source Location:** `/Users/rikihernandez/Downloads/Aciicker-Y9-2/world.cpp` lines 922-1197

---

## 1. Understanding the Problem

### 1.1 What the Cleanup Should Do

When an instance is deleted from a BSP tree leaf node, the following should happen:

1. **Remove instance from leaf:** The instance is unlinked from the doubly-linked list in the leaf
2. **Check if leaf is empty:** If `leaf->head == NULL` after removal
3. **Walk up the tree:** Check each ancestor node to see if it now has no content
4. **Collapse empty nodes:** If a parent node has both children empty (or the leaf is empty and has no children), delete the parent and continue upward
5. **Repeat until root or non-empty node:** Continue until reaching the root or a node that still has content

### 1.2 Current Stubbed Behavior

The C++ code has TODO comments explaining what should happen but the actual cleanup is not implemented:

```cpp
// Lines 922-926 (MeshInst deletion, BSP_TYPE_LEAF case):
if (leaf->head == 0)
{
    // do ancestors cleanup
    // ...
}
```

**Three identical stub locations:**
- Lines 922-926: MeshInst deletion, leaf becomes empty
- Lines 951-955: MeshInst deletion, NodeShare becomes empty  
- Lines 967-971: MeshInst deletion, Node becomes empty
- Lines 1031-1035: SpriteInst deletion, leaf becomes empty
- Lines 1060-1064: SpriteInst deletion, NodeShare becomes empty
- Lines 1076-1080: SpriteInst deletion, Node becomes empty
- Lines 1146-1150: ItemInst deletion, leaf becomes empty
- Lines 1175-1180: ItemInst deletion, NodeShare becomes empty
- Lines 1192-1197: ItemInst deletion, Node becomes empty

### 1.3 Impact of Not Fixing

- **Memory accumulation:** Empty leaf nodes remain allocated after all instances are deleted
- **Performance degradation:** Queries must traverse empty nodes, wasting CPU cycles
- **Tree bloat:** Over many delete operations, the BSP tree grows with dead weight
- **Rust port inherits bug:** If not fixed, the Rust implementation will have the same issues

---

## 2. BSP Tree Structure Analysis

### 2.1 Node Hierarchy (from world.cpp:265-299)

```cpp
struct BSP
{
    TYPE type;           // BSP_TYPE_NODE, BSP_TYPE_NODE_SHARE, BSP_TYPE_LEAF, BSP_TYPE_INST
    float bbox[6];        // Axis-aligned bounding box in world coords
    BSP* bsp_parent;     // Pointer to parent node (NULL if root or detached)
};

struct BSP_Node : BSP
{
    BSP* bsp_child[2];   // Two children (can be NULL)
};

struct BSP_NodeShare : BSP_Node
{
    Inst* head;          // Doubly-linked list of instances
    Inst* tail;
};

struct BSP_Leaf : BSP
{
    Inst* head;          // Doubly-linked list of instances
    Inst* tail;
};
```

### 2.2 Key Observations

1. **Parent pointer exists:** Each BSP node has `bsp_parent` - this enables walking upward
2. **Three node types can be empty:**
   - `BSP_Leaf`: Empty when `head == NULL`
   - `BSP_NodeShare`: Empty when `head == NULL && bsp_child[0] == NULL && bsp_child[1] == NULL`
   - `BSP_Node`: Empty when `bsp_child[0] == NULL && bsp_child[1] == NULL`
3. **Deletion happens in three instance types:** MeshInst, SpriteInst, ItemInst

---

## 3. Implementation Options

### 3.1 Option A: Implement Full Ancestor Cleanup (RECOMMENDED)

**Pros:**
- Fixes the actual bug in the original codebase
- Improves performance and memory usage
- Demonstrates thorough understanding of the codebase
- Cleaner final implementation

**Cons:**
- More complex implementation
- Requires careful testing
- May have edge cases (root node deletion, tree restructuring)

**Implementation approach:**
```rust
fn cleanup_ancestors(node: &mut BSP) {
    let mut current = node.bsp_parent;
    
    while let Some(parent) = current {
        match parent.type {
            BSP_TYPE_LEAF => {
                // Leaf is already empty (that's why we called cleanup)
                // Just need to check if we should remove the leaf from its parent
                if let Some(grandparent) = parent.bsp_parent {
                    // Similar logic to NODE case below
                    // ...
                    current = grandparent;
                } else {
                    // Root is a leaf - cannot collapse further
                    break;
                }
            }
            BSP_TYPE_NODE_SHARE => {
                if parent.head.is_null() && 
                   parent.bsp_child[0].is_null() && 
                   parent.bsp_child[1].is_null() {
                    // This node is empty - need to remove from grandparent
                    // and continue cleanup
                    if let Some(grandparent) = parent.bsp_parent {
                        // Remove parent from grandparent's children
                        // Then continue up the tree
                        current = grandparent;
                    } else {
                        // Parent is root - can't delete root, but can mark as empty
                        break;
                    }
                } else {
                    // Node still has content, stop cleanup
                    break;
                }
            }
            BSP_TYPE_NODE => {
                if parent.bsp_child[0].is_null() && parent.bsp_child[1].is_null() {
                    // Both children are empty - delete this node
                    if let Some(grandparent) = parent.bsp_parent {
                        // Remove parent from grandparent's children
                        // Then continue up the tree
                        current = grandparent;
                    } else {
                        // Parent is root - special handling needed
                        break;
                    }
                } else {
                    // Node still has content, stop cleanup
                    break;
                }
            }
            _ => break,
        }
    }
}
```

### 3.2 Option B: Document as Known Limitation

**Pros:**
- Simpler - no code changes required
- Clearly documents the original behavior
- Can be revisited later if needed

**Cons:**
- Inherits the performance/memory bug in the Rust port
- May cause issues in long-running applications
- Less complete port

### 3.3 Option C: Lazy Cleanup with Periodic Rebuild

**Pros:**
- Simple to implement
- Avoids complexity of incremental cleanup

**Cons:**
- Doesn't match original behavior exactly
- May have memory spikes during rebuild
- Adds maintenance overhead

---

## 4. Recommended Implementation Plan

### 4.1 Decision: **Option A - Implement Full Ancestor Cleanup**

Given this is a HIGH severity gap documented in the analysis, implementing proper cleanup is the recommended approach.

### 4.2 Implementation Steps

#### Step 1: Define BSP Node Types (Rust)

Create Rust structs that mirror the C++ hierarchy:

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum BspType {
    Node,
    NodeShare,
    Leaf,
    Inst,
}

pub struct Bsp {
    pub bsp_type: BspType,
    pub bbox: [f32; 6],      // xmin, xmax, ymin, ymax, zmin, zmax
    pub bsp_parent: Option<NonNull<Bsp>>,
}

pub struct BspNode {
    pub base: Bsp,
    pub bsp_child: [Option<NonNull<Bsp>>; 2],
}

pub struct BspLeaf {
    pub base: Bsp,
    pub head: Option<NonNull<Inst>>,
    pub tail: Option<NonNull<Inst>>,
}

pub struct BspNodeShare {
    pub base: Bsp,
    pub bsp_child: [Option<NonNull<Bsp>>; 2],
    pub head: Option<NonNull<Inst>>,
    pub tail: Option<NonNull<Inst>>,
}
```

#### Step 2: Implement Cleanup Function

Create a recursive or iterative function to walk up the tree:

```rust
/// Cleans up empty ancestor nodes after an instance deletion.
/// 
/// Returns true if cleanup was performed, false if no cleanup needed.
pub fn cleanup_empty_ancestors(w: &mut World, empty_node: *mut Bsp) -> bool {
    let mut current = empty_node;
    let mut did_cleanup = false;
    
    unsafe {
        loop {
            let parent = match (*current).bsp_parent {
                Some(p) => p,
                None => break,  // Reached root
            };
            
            let parent_type = (*parent.as_ptr()).bsp_type;
            
            match parent_type {
                BspType::Leaf => {
                    let leaf = &mut *(parent.as_ptr() as *mut BspLeaf);
                    if leaf.head.is_none() {
                        // Leaf is empty, but we can't collapse a leaf from its parent
                        // (leafs don't have children to check)
                        break;
                    }
                    break;  // Leaf still has content
                }
                BspType::NodeShare => {
                    let share = &mut *(parent.as_ptr() as *mut BspNodeShare);
                    if share.head.is_none() && 
                       share.bsp_child[0].is_none() && 
                       share.bsp_child[1].is_none() {
                        // NodeShare is empty - remove from grandparent and continue
                        did_cleanup = true;
                        remove_node_from_parent(parent.as_ptr());
                        current = parent.as_ptr();
                    } else {
                        break;  // Still has content
                    }
                }
                BspType::Node => {
                    let node = &mut *(parent.as_ptr() as *mut BspNode);
                    if node.bsp_child[0].is_none() && node.bsp_child[1].is_none() {
                        // Node is empty - remove from grandparent and continue
                        did_cleanup = true;
                        remove_node_from_parent(parent.as_ptr());
                        current = parent.as_ptr();
                    } else {
                        break;  // Still has content
                    }
                }
                _ => break,
            }
        }
    }
    
    did_cleanup
}
```

#### Step 3: Integrate into DelInst Functions

Add cleanup calls after instance removal in all three deletion functions:

```rust
// In MeshInst::DelInst, after line 920 (leaf->head == 0 check):
if leaf.head.is_none() {
    cleanup_empty_ancestors(w, leaf as *mut Bsp);
}

// Similar for SpriteInst::DelInst and ItemInst::DelInst
```

#### Step 4: Handle Edge Cases

- **Root node deletion:** If the root becomes empty, set root to NULL
- **NodeShare special case:** Must check both instance list AND children
- **Memory management:** Free the allocated nodes properly

---

## 5. Testing Strategy

### 5.1 Unit Tests

1. **Single instance deletion:** Delete the only instance in a leaf
2. **Multiple instances, partial delete:** Delete some but not all instances
3. **Tree collapse:** Delete instances that cause parent nodes to become empty
4. **Multi-level collapse:** Test deeply nested trees where multiple levels become empty

### 5.2 Integration Tests

1. **Rebuild after deletions:** Verify BSP tree still queries correctly after cleanup
2. **Serialization round-trip:** Save/load world with deleted instances
3. **Performance benchmark:** Compare query times before/after many deletions

---

## 6. Alternative: Document as Known Limitation

If implementation complexity is too high, document as known limitation:

```rust
/// NOTE: The original C++ implementation has STUBBED ancestor cleanup.
/// After instance deletion, empty parent nodes are NOT collapsed.
/// This is a known limitation that may cause memory accumulation
/// in long-running applications with many instance deletions.
///
/// See: /Users/r/Projects/asciicker rust port/docs/worksheets/gaps-terrain-world.md
///
/// TODO: Consider implementing proper ancestor cleanup for
///       production-quality port.
```

---

## 7. Conclusion

**Recommendation:** Implement Option A (Full Ancestor Cleanup)

**Rationale:**
1. This is a HIGH severity gap that directly impacts performance and memory
2. The cleanup algorithm is well-understood from the TODO comments in the C++ code
3. The BSP tree structure has parent pointers that make this implementation straightforward
4. A correct implementation will result in a more complete and professional port

**Estimated complexity:** Medium
- Understanding BSP tree structure: Low
- Implementing cleanup algorithm: Medium  
- Testing edge cases: Medium
- Integration with 3 deletion functions: Low

---

## References

- Source stubbed code: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/world.cpp:922-1197`
- Gap analysis: `/Users/r/Projects/asciicker rust port/docs/worksheets/gaps-terrain-world.md`
- BSP structure: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/world.cpp:265-299`
- Architecture docs: `/Users/r/Projects/asciicker rust port/docs/worksheets/arch/world_cpp_part1.md`
- Research: `/Users/r/Projects/asciicker rust port/docs/worksheets/research-cpp-architecture-analysis.md`
