# Batch Undo System Architecture (urdo.cpp / urdo.h)

## Overview

The undo/redo system provides atomic undo/redo capability for ALL editor operations: terrain editing (height/visual map modifications), mesh/sprite instance placement, and terrain patch creation/deletion. All edits flow through `URDO_*` functions.

**Architecture:** Doubly-linked list with cursor navigation + stack-based group nesting (up to 64 levels).

**Key Insight (SWAP pattern):** Height and visual operations SWAP data with terrain instead of copying, making undo and redo identical operations (O(1) time, symmetric logic).

---

## Data Structures

### struct URDO (base type)

**File:** urdo.cpp:85-103

**Fields:**
- `URDO* next` — Next operation in doubly-linked list (forward toward redo)
- `URDO* prev` — Previous operation in doubly-linked list (backward toward undo)
- `CMD cmd` — Discriminator tag for operation type (GROUP, PATCH_CREATE, PATCH_UPDATE_HEIGHT, PATCH_UPDATE_VISUAL, PATCH_DIAG, INST_CREATE)

**Methods:**
- `void Do(bool un)` — Dispatch polymorphically based on `cmd` tag (manual vtable via switch)
- `static URDO* Alloc(CMD c)` — Allocate typed URDO struct, append to undo chain and current group
- `void Free()` — Recursively free operation and any owned resources (detached patches, nested groups)

**Purpose:** Base struct for all undo operations. Uses C-style type tag polymorphism (no vtable overhead).

---

### enum URDO::CMD

**File:** urdo.cpp:90-98

**Values:**
- `CMD_GROUP` — Nested operation group (undone/redone atomically)
- `CMD_PATCH_CREATE` — Create or delete terrain patch (toggles attached state)
- `CMD_PATCH_UPDATE_HEIGHT` — Snapshot of height map before editing (SWAP on undo)
- `CMD_PATCH_UPDATE_VISUAL` — Snapshot of visual map before editing (SWAP on undo)
- `CMD_PATCH_DIAG` — Snapshot of diagonal flag before flipping (SWAP on undo)
- `CMD_INST_CREATE` — Create or delete mesh/sprite instance (toggles attached state)

**Purpose:** Type discriminator for dispatching `Do()` and `Free()` operations.

---

### struct URDO_Group : URDO

**File:** urdo.cpp:105-113

**Fields:**
- `URDO* group_head` — First operation in group's child list
- `URDO* group_tail` — Last operation in group's child list

**Static Methods:**
- `static void Open()` — Start new operation group, push onto stack
- `static void Close()` — End current operation group, seal as single undo unit

**Instance Methods:**
- `void Do(bool un)` — Execute all child operations in forward (redo) or reverse (undo) order

**Purpose:** Container for nested operations. Groups allow multiple edits (e.g., painting 10 patches) to be undone/redone atomically as one user action.

---

### struct URDO_PatchCreate : URDO

**File:** urdo.cpp:115-126

**Fields:**
- `int cx, cy` — Patch coordinates in terrain grid
- `Terrain* terrain` — Parent terrain container
- `Patch* patch` — The patch being created/deleted
- `bool attached` — True if patch is currently in terrain, false if detached (in URDO)

**Static Methods:**
- `static void Delete(Terrain* t, Patch* p)` — Record patch deletion (detaches from terrain)
- `static Patch* Create(Terrain* t, int x, int y, int z)` — Record patch creation (adds to terrain)

**Instance Methods:**
- `void Do(bool un)` — Toggle attached state (TerrainDetach if attached, TerrainAttach if detached)

**Purpose:** Records terrain patch add/remove operations. The SAME Do() handles both create and delete by toggling the `attached` flag (symmetric undo/redo).

**Called by:** `URDO_Create(Terrain*, int, int, int)`, `URDO_Delete(Terrain*, Patch*)`

---

### struct URDO_PatchUpdateHeight : URDO

**File:** urdo.cpp:128-136

**Fields:**
- `Patch* patch` — The patch whose height map is being modified
- `uint16_t height[HEIGHT_CELLS+1][HEIGHT_CELLS+1]` — Snapshot of height map (5x5 vertex grid)
- `uint16_t diag` — Snapshot of diagonal flags (which triangles face which way)

**Static Methods:**
- `static void Open(Patch* p)` — Allocate operation, copy current height map and diag from patch

**Instance Methods:**
- `void Do(bool un)` — SWAP height map and diag with patch (undo and redo are identical)

**Purpose:** Captures height map state before editing. The SWAP pattern makes undo and redo the SAME operation (just swap data back and forth). No separate undo vs redo logic needed.

**Called by:** `URDO_Patch(Patch* p, false)`

---

### struct URDO_PatchUpdateVisual : URDO

**File:** urdo.cpp:138-145

**Fields:**
- `Patch* patch` — The patch whose visual map is being modified
- `uint16_t visual[VISUAL_CELLS][VISUAL_CELLS]` — Snapshot of visual map (8x8 material grid)

**Static Methods:**
- `static void Open(Patch* p)` — Allocate operation, copy current visual map from patch

**Instance Methods:**
- `void Do(bool un)` — SWAP visual map with patch (undo and redo are identical)

**Purpose:** Captures visual map state before editing. Uses the SWAP pattern (identical to height update).

**Called by:** `URDO_Patch(Patch* p, true)`

---

### struct URDO_PatchDiag : URDO

**File:** urdo.cpp:147-154

**Fields:**
- `Patch* patch` — The patch whose diagonal flags are being modified
- `uint16_t diag` — Snapshot of diagonal bitmask

**Static Methods:**
- `static void Open(Patch* p)` — Allocate operation, copy current diag from patch

**Instance Methods:**
- `void Do(bool un)` — SWAP diag with patch (undo and redo are identical)

**Purpose:** Captures diagonal flag state before flipping. Uses the SWAP pattern.

**Called by:** `URDO_Diag(Patch* p)`

---

### struct URDO_InstCreate : URDO

**File:** urdo.cpp:156-187

**Fields:**
- `Inst* inst` — The mesh/sprite instance being created/deleted
- `bool attached` — True if instance is currently in world, false if detached (in URDO)

**Commented-out fields (161-178):** Original design stored all instance parameters for recreating from scratch. Current design stores the `Inst*` directly and toggles attachment state (lighter weight, avoids recreation overhead).

**Static Methods:**
- `static void Delete(Inst* i)` — Record instance deletion (SoftInstDel removes from world)
- `static Inst* Create(Mesh* m, int flags, double tm[16], int story_id)` — Record mesh instance creation
- `static Inst* Create(World* w, Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], int story_id)` — Record sprite instance creation
- `static Inst* Create(World* w, Item* item, int flags, float pos[3], float yaw, int story_id)` — Record item instance creation

**Instance Methods:**
- `void Do(bool un)` — Toggle attached state (SoftInstDel if attached, SoftInstAdd if detached)

**Purpose:** Records instance add/remove operations. The SAME Do() handles both create and delete by toggling the `attached` flag (symmetric undo/redo).

**Called by:** `URDO_Create(...)` overloads, `URDO_Delete(Inst*)`

---

## Global State

**File:** urdo.cpp:189-195

- `static size_t bytes = 0` — Total memory used by undo/redo history (bytes counter for UI display)
- `static URDO* undo = 0` — Cursor pointing to last executed operation (can be undone)
- `static URDO* redo = 0` — Cursor pointing to next undone operation (can be redone)
- `static int group_open = 0` — Nesting depth of open groups (0 means no group open)
- `static int stack_depth = 0` — Current depth in group stack (0 to 63)
- `static URDO_Group* stack[64]` — Stack of open groups (for nested bracketing)

**Navigation Model:**
```
[op1] <-> [op2] <-> [op3] <-> [op4] <-> [op5]
                      ^undo     ^redo
```
- `undo` points to the last executed operation (can move left to undo)
- `redo` points to the next undone operation (can move right to redo)
- New operations append after `undo`, purge `redo` chain (divergent history)

---

## Core Functions

### `URDO::Do(bool un)` (urdo.cpp:203-216)

**File:** urdo.cpp:203-216

**Signature:** `void URDO::Do(bool un)`

**Parameters:**
- `un` — True for undo (reverse operation), false for redo (forward operation)

**Purpose:** Dispatch polymorphically to typed Do() method based on `cmd` tag.

**Calls:**
- `URDO_Group::Do(un)` if `cmd == CMD_GROUP`
- `URDO_PatchCreate::Do(un)` if `cmd == CMD_PATCH_CREATE`
- `URDO_PatchUpdateHeight::Do(un)` if `cmd == CMD_PATCH_UPDATE_HEIGHT`
- `URDO_PatchUpdateVisual::Do(un)` if `cmd == CMD_PATCH_UPDATE_VISUAL`
- `URDO_PatchDiag::Do(un)` if `cmd == CMD_PATCH_DIAG`
- `URDO_InstCreate::Do(un)` if `cmd == CMD_INST_CREATE`

**Called by:** `URDO_Undo`, `URDO_Redo`, `URDO_Group::Do` (recursive)

**Globals read:** `cmd` field (discriminator tag)
**Globals mutated:** None (delegates to child operations)
**Side effects:** Executes typed operation (modifies terrain or world state)

**Notes:** Manual vtable dispatch via switch. Avoids virtual function overhead. Uses C-style type casting to downcast `this` to specific subtype.

---

### `URDO::Free()` (urdo.cpp:224-273)

**File:** urdo.cpp:224-273

**Signature:** `void URDO::Free()`

**Purpose:** Recursively free operation and any owned resources (detached patches, nested groups).

**Calls:**
- `TerrainDispose(Patch*)` — Free detached patch (PATCH_CREATE with attached=false)
- `HardInstDel(Inst*)` — Free detached instance (INST_CREATE with attached=false)
- `free(this)` — Free the URDO struct itself

**Called by:** `PurgeUndo`, `PurgeRedo`, `URDO_Purge`, `URDO::Free` (recursive for GROUP)

**Globals read:** `bytes` (current memory counter), `stack_depth`, `stack[...]` (for GROUP cleanup)

**Globals mutated:** `bytes` (decremented by struct size and detached patch/instance size)

**Side effects:** Frees memory, recursive for CMD_GROUP (depth-first child traversal)

**Notes:** Different cleanup per type. PATCH_CREATE must free detached patch if attached=false. INST_CREATE must free detached instance if attached=false. GROUP recursively frees all child operations (group_head to group_tail). Other types just free the struct.

---

### `URDO::Alloc(CMD c)` (urdo.cpp:282-318)

**File:** urdo.cpp:282-318

**Signature:** `static URDO* URDO::Alloc(CMD c)`

**Parameters:**
- `c` — Operation type (determines struct size to allocate)

**Returns:** Pointer to newly allocated URDO struct (zero-initialized)

**Purpose:** Allocate typed URDO struct, append to undo chain and current group's child list.

**Calls:**
- `malloc(s)` — Allocate struct (size determined by switch on `c`)
- `memset(urdo, 0, s)` — Zero-initialize struct

**Called by:** All `URDO_*::Open` and `URDO_*::Create` static methods

**Globals read:** `stack_depth`, `stack[stack_depth-1]`, `undo`

**Globals mutated:** `bytes` (incremented by struct size), `undo` (set to new operation), current group's `group_head` and `group_tail` (if inside group)

**Side effects:** Dual bookkeeping — appends to global undo chain AND to current group's child list (if stack_depth > 0)

**Notes:** The dual bookkeeping is critical. New operations are added to:
1. Global undo chain (undo pointer, linear history for navigation)
2. Current group's child list (stack[stack_depth-1]->group_tail, tree structure for atomic undo)

This allows both linear navigation (undo/redo pointers walk the chain) and hierarchical grouping (groups contain child operations).

---

### `PurgeUndo()` (urdo.cpp:328-369)

**File:** urdo.cpp:328-369

**Signature:** `static void PurgeUndo()`

**Purpose:** Free all undoable operations (walk backward from `undo` pointer, free all operations).

**Calls:**
- `URDO::Free()` — Free each operation

**Called by:** `URDO_Purge`

**Globals read:** `undo`, `redo`, `stack_depth`, `stack[...]`

**Globals mutated:** `undo` (set to nullptr), `redo` (adjusted if crossing group boundaries), `stack_depth` (decremented when unwinding groups)

**Side effects:** Recursively frees all operations from `undo` backward to chain head. Handles nested groups by unwinding stack.

**Notes:** The algorithm walks backward from `undo` pointer, freeing operations. When encountering a GROUP, it must:
1. Restore undo/redo pointers to before the group (g->prev/g->next)
2. Recursively free child operations (g->group_head)
3. Continue unwinding if more groups on stack

This handles nested groups correctly (innermost groups freed first).

---

### `PurgeRedo()` (urdo.cpp:377-418)

**File:** urdo.cpp:377-418

**Signature:** `static void PurgeRedo()`

**Purpose:** Free all redoable operations (walk forward from `redo` pointer, free all operations).

**Calls:**
- `URDO::Free()` — Free each operation

**Called by:** `URDO_Open`, `URDO_Patch`, `URDO_Diag`, `URDO_PatchCreate::Delete`, `URDO_PatchCreate::Create`, `URDO_InstCreate::Delete`, `URDO_InstCreate::Create` (all when `!group_open`)

**Globals read:** `redo`, `undo`, `stack_depth`, `stack[...]`

**Globals mutated:** `redo` (set to nullptr), `undo` (adjusted if crossing group boundaries), `stack_depth` (decremented when unwinding groups)

**Side effects:** Recursively frees all operations from `redo` forward to chain end. Handles nested groups by unwinding stack.

**Notes:** Symmetric to PurgeUndo. The redo chain has the same nested group structure and needs identical recursive cleanup. Walks forward from redo pointer (instead of backward from undo pointer), freeing redoable operations. Called when new operations create divergent history (redo chain becomes invalid).

---

## Public API Functions

### `URDO_Purge()` (urdo.cpp:420-426)

**File:** urdo.cpp:420-426

**Signature:** `void URDO_Purge()`

**Purpose:** Free all undo/redo history (reset to empty state).

**Calls:**
- `PurgeUndo()`
- `PurgeRedo()`

**Called by:** `asciiid.cpp` (editor clear, world load, editor shutdown)
- Line 5569 (world load)
- Line 5911 (world load)
- Line 8036 (clear undo history menu action)
- Line 11231 (editor shutdown)

**Globals read:** `group_open` (asserts must be 0)

**Globals mutated:** `undo` (set to nullptr), `redo` (set to nullptr), `bytes` (reset to 0), `stack_depth` (reset to 0)

**Side effects:** Frees all undo/redo memory, resets system to empty state

**Notes:** Assert checks that no group is open. Cannot purge while editing (would corrupt nested group state).

---

### `URDO_CanUndo()` (urdo.cpp:428-441)

**File:** urdo.cpp:428-441

**Signature:** `bool URDO_CanUndo()`

**Returns:** True if undo is available, false otherwise

**Purpose:** Query whether undo is available (for UI enable/disable).

**Calls:** None

**Called by:** `asciiid.cpp` line 7988, 8026 (undo UI logic)

**Globals read:** `group_open`, `undo`, `stack_depth`, `stack[...]`

**Globals mutated:** None

**Side effects:** None (pure query)

**Notes:** Returns false if group is open (cannot undo while editing). Returns true if `undo` is non-null. If `undo` is null, walks stack to check if any parent group has operations before it (stack[d]->prev). This handles the case where the cursor is inside a group.

---

### `URDO_CanRedo()` (urdo.cpp:443-456)

**File:** urdo.cpp:443-456

**Signature:** `bool URDO_CanRedo()`

**Returns:** True if redo is available, false otherwise

**Purpose:** Query whether redo is available (for UI enable/disable).

**Calls:** None

**Called by:** `asciiid.cpp` line 8007, 8026 (redo UI logic)

**Globals read:** `group_open`, `redo`, `stack_depth`, `stack[...]`

**Globals mutated:** None

**Side effects:** None (pure query)

**Notes:** Returns false if group is open (cannot redo while editing). Returns true if `redo` is non-null. If `redo` is null, walks stack to check if any parent group has operations after it (stack[d]->next). This handles the case where the cursor is inside a group.

---

### `URDO_Bytes()` (urdo.cpp:458-461)

**File:** urdo.cpp:458-461

**Signature:** `size_t URDO_Bytes()`

**Returns:** Total memory used by undo/redo history (in bytes)

**Purpose:** Report memory usage for UI display.

**Calls:** None

**Called by:** `asciiid.cpp` line 8038 (undo panel memory display)

**Globals read:** `bytes`

**Globals mutated:** None

**Side effects:** None (pure query)

**Notes:** Tracks total memory used by all URDO structs, detached patches, and detached instances. Updated by `URDO::Alloc` (increment) and `URDO::Free` (decrement). Reported via UI to show user how much RAM undo uses.

---

### `URDO_Undo(int max_depth)` (urdo.cpp:471-511)

**File:** urdo.cpp:471-511

**Signature:** `void URDO_Undo(int max_depth)`

**Parameters:**
- `max_depth` — Controls granularity (0=undo one leaf operation, 64=undo everything)

**Purpose:** Undo last operation or entire group. Cannot call while group is open.

**Calls:**
- `URDO::Do(true)` — Execute undo on each operation

**Called by:** `asciiid.cpp` line 8001 (max_depth=0 for leaf undo), 8004 (max_depth=1 for group undo)

**Globals read:** `group_open` (asserts must be 0), `undo`, `redo`, `stack_depth`, `stack[...]`

**Globals mutated:** `undo` (moves backward), `redo` (moves forward), `stack_depth` (may increase when descending into groups)

**Side effects:** Executes undo operations (modifies terrain or world state)

**Notes:** The algorithm handles nested groups via stack traversal:
1. Pop stack if at group boundary (restore undo/redo to before group)
2. Descend into groups (push onto stack, move undo to group_tail)
3. Undo leaf operation (undo->Do(true), move undo pointer backward)
4. Unwind stack if depth exceeded (undo entire group atomically)

max_depth parameter controls granularity:
- 0 = undo one leaf operation (single height edit)
- 1 = undo one group (e.g., entire merge operation)
- 64 = undo everything (stack unwinds completely)

---

### `URDO_Redo(int max_depth)` (urdo.cpp:520-560)

**File:** urdo.cpp:520-560

**Signature:** `void URDO_Redo(int max_depth)`

**Parameters:**
- `max_depth` — Controls granularity (0=redo one leaf operation, 64=redo everything)

**Purpose:** Redo next undone operation. Cannot call while group is open.

**Calls:**
- `URDO::Do(false)` — Execute redo on each operation (note: line 551 has typo, calls `Do(true)` instead of `Do(false)`)

**Called by:** `asciiid.cpp` line 8020 (max_depth=1 for group redo), 8023 (max_depth=0 for leaf redo)

**Globals read:** `group_open` (asserts must be 0), `redo`, `undo`, `stack_depth`, `stack[...]`

**Globals mutated:** `redo` (moves forward), `undo` (moves backward), `stack_depth` (may increase when descending into groups)

**Side effects:** Executes redo operations (modifies terrain or world state)

**Notes:** Mirror of URDO_Undo, processes group operations in forward order. Algorithm:
1. Pop stack if at group boundary (restore undo/redo to after group)
2. Descend into groups (push onto stack, move redo to group_head)
3. Redo leaf operation (redo->Do(false), move redo pointer forward)
4. Unwind stack if depth exceeded (redo entire group atomically)

Same max_depth semantics as Undo.

**:** Line 551 calls `redo->Do(true)` instead of `redo->Do(false)`. This looks like a bug (redo should pass false for forward operation). However, for SWAP-based operations (height, visual, diag), this doesn't matter because swapping is symmetric (true vs false produces same result). For CREATE operations, the `attached` flag toggles the same way regardless. So this apparent bug is harmless due to the symmetric design.

---

### `URDO_Open()` (urdo.cpp:562-566)

**File:** urdo.cpp:562-566

**Signature:** `void URDO_Open()`

**Purpose:** Start a new operation group. All subsequent URDO_* calls until URDO_Close() are grouped as one undo unit.

**Calls:**
- `URDO_Group::Open()`

**Called by:** `asciiid.cpp` (many locations, wrapping multi-operation edits)
- Line 706 (merge operation start)
- Line 4061, 4158, 4191, 4776, 4887, 5066, 5173 (paint operations)
- Line 4869 (delete instances)
- Line 9557, 9594, 9707, 9804, 9896, 9905, 9991, 10000, 10460 (various editor operations)

**Globals read:** `group_open` (asserts < 64, max nesting)

**Globals mutated:** `group_open` (incremented), `stack_depth` (incremented), `stack[stack_depth-1]` (set to new group), `undo` (set to nullptr), `redo` (set to nullptr)

**Side effects:** Purges redo chain if top-level group (divergent history). Pushes new group onto stack. Resets undo/redo cursors to prepare for child operations.

**Notes:** Opening a top-level group (group_open==0) means user is making new edits after undo, so redo chain becomes invalid (divergent timeline). Purging redo frees that memory. Pushing onto stack allows subsequent URDO_* calls to add operations to the group's child list.

---

### `URDO_Close()` (urdo.cpp:568-572)

**File:** urdo.cpp:568-572

**Signature:** `void URDO_Close()`

**Purpose:** End current operation group. Seals grouped operations. Deletes group if empty (no operations added).

**Calls:**
- `URDO_Group::Close()`

**Called by:** `asciiid.cpp` (many locations, closing groups opened by URDO_Open)
- Line 728 (merge operation end)
- Line 4063, 4160, 4193, 4778, 4890, 5069, 5227 (paint operations)
- Line 4873 (delete instances)
- Line 9478, 9532, 9559, 9571, 9596, 9608, 9907, 10002 (various editor operations)

**Globals read:** `group_open` (asserts > 0, must have open group)

**Globals mutated:** `group_open` (decremented), `stack_depth` (decremented), `undo` (set to group or restored to parent)

**Side effects:** Seals group as single undo unit. Deletes empty groups (group_head == nullptr).

**Notes:** If group_head is null, no operations were added to the group, so the group node itself is useless. Free it and restore undo pointer. If group has operations, sealed group becomes a single node in parent's list. The group's child operations (group_head to group_tail) remain accessible via the group node, but global undo pointer now points to the group itself.

---

### `URDO_Create(Mesh*, int, double[16], int)` (urdo.cpp:574-579)

**File:** urdo.cpp:574-579

**Signature:** `Inst* URDO_Create(Mesh* m, int flags, double tm[16], int story_id)`

**Parameters:**
- `m` — Mesh resource to instantiate
- `flags` — Instance flags (INST_USE_TREE, etc.)
- `tm` — 4x4 transform matrix (column-major double[16])
- `story_id` — Story/quest ID for scripting

**Returns:** Newly created Inst pointer

**Purpose:** Create mesh instance with undo support (replacement for CreateInst).

**Calls:**
- `URDO_InstCreate::Create(m, flags, tm, story_id)`

**Called by:** `asciiid.cpp` line 599 (mesh duplication), 10169 (mesh placement)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_InstCreate::Create` — creates INST_CREATE operation

**Side effects:** Creates instance, adds to world, records in undo history

**Notes:** Assert checks that nesting depth is < 64 (prevent stack overflow). Delegates to URDO_InstCreate::Create which allocates the operation, creates the instance via CreateInst, and records it in undo history.

---

### `URDO_Create(World*, Sprite*, int, float[3], float, int, int, int[4], int)` (urdo.cpp:581-586)

**File:** urdo.cpp:581-586

**Signature:** `Inst* URDO_Create(World* w, Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], int story_id)`

**Parameters:**
- `w` — World container
- `s` — Sprite resource
- `flags` — Instance flags
- `pos` — Position [x, y, z]
- `yaw` — Rotation angle
- `anim` — Animation ID
- `frame` — Starting frame
- `reps` — Repetition counts [unused]
- `story_id` — Story/quest ID

**Returns:** Newly created Inst pointer

**Purpose:** Create sprite instance with undo support (replacement for CreateInst).

**Calls:**
- `URDO_InstCreate::Create(w, s, flags, pos, yaw, anim, frame, reps, story_id)`

**Called by:** `asciiid.cpp` line 6368, 6414, 6463, 10257 (sprite placement)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_InstCreate::Create` — creates INST_CREATE operation

**Side effects:** Creates instance, adds to world, records in undo history

**Notes:** Assert checks nesting depth. Delegates to URDO_InstCreate::Create.

---

### `URDO_Create(World*, Item*, int, float[3], float, int)` (urdo.cpp:588-593)

**File:** urdo.cpp:588-593

**Signature:** `Inst* URDO_Create(World* w, Item* item, int flags, float pos[3], float yaw, int story_id)`

**Parameters:**
- `w` — World container
- `item` — Inventory item resource
- `flags` — Instance flags
- `pos` — Position [x, y, z]
- `yaw` — Rotation angle
- `story_id` — Story/quest ID

**Returns:** Newly created Inst pointer

**Purpose:** Create item instance with undo support (replacement for CreateInst).

**Calls:**
- `URDO_InstCreate::Create(w, item, flags, pos, yaw, story_id)`

**Called by:** `asciiid.cpp` line 10327 (item placement)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_InstCreate::Create` — creates INST_CREATE operation

**Side effects:** Creates instance, adds to world, records in undo history

**Notes:** Assert checks nesting depth. Delegates to URDO_InstCreate::Create.

---

### `URDO_Delete(Inst*)` (urdo.cpp:595-599)

**File:** urdo.cpp:595-599

**Signature:** `void URDO_Delete(Inst* i)`

**Parameters:**
- `i` — Instance to delete

**Purpose:** Delete mesh/sprite instance with undo support (replacement for DeleteInst).

**Calls:**
- `URDO_InstCreate::Delete(i)`

**Called by:** `asciiid.cpp` line 4870, 4889, 10195, 10225, 10296 (instance deletion)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_InstCreate::Delete` — creates INST_CREATE operation with attached=false

**Side effects:** Detaches instance from world (SoftInstDel), records in undo history

**Notes:** Assert checks nesting depth. Delegates to URDO_InstCreate::Delete which calls SoftInstDel (detaches from world but keeps pointer valid) and records the operation.

---

### `URDO_Create(Terrain*, int, int, int)` (urdo.cpp:601-605)

**File:** urdo.cpp:601-605

**Signature:** `Patch* URDO_Create(Terrain* t, int x, int y, int z)`

**Parameters:**
- `t` — Terrain container
- `x` — Patch X coordinate
- `y` — Patch Y coordinate
- `z` — Base height for new patch

**Returns:** Newly created Patch pointer

**Purpose:** Create terrain patch with undo support (replacement for AddTerrainPatch).

**Calls:**
- `URDO_PatchCreate::Create(t, x, y, z)`

**Called by:** `asciiid.cpp` line 530 (patch creation), 9465 (patch creation in editor)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_PatchCreate::Create` — creates PATCH_CREATE operation with attached=true

**Side effects:** Creates patch, adds to terrain, records in undo history

**Notes:** Assert checks nesting depth. Delegates to URDO_PatchCreate::Create which calls AddTerrainPatch and records the operation.

---

### `URDO_Delete(Terrain*, Patch*)` (urdo.cpp:607-611)

**File:** urdo.cpp:607-611

**Signature:** `void URDO_Delete(Terrain* t, Patch* p)`

**Parameters:**
- `t` — Terrain container
- `p` — Patch to delete

**Purpose:** Delete terrain patch with undo support (replacement for DelTerrainPatch).

**Calls:**
- `URDO_PatchCreate::Delete(t, p)`

**Called by:** `asciiid.cpp` line 9458 (patch deletion in editor)

**Globals read:** `group_open` (asserts < 64)

**Globals mutated:** Via `URDO_PatchCreate::Delete` — creates PATCH_CREATE operation with attached=false

**Side effects:** Detaches patch from terrain (TerrainDetach), records in undo history

**Notes:** Assert checks nesting depth. Delegates to URDO_PatchCreate::Delete which calls TerrainDetach (removes from quadtree but keeps pointer valid) and records the operation.

---

### `URDO_Patch(Patch*, bool)` (urdo.cpp:613-619)

**File:** urdo.cpp:613-619

**Signature:** `void URDO_Patch(Patch* p, bool visual)`

**Parameters:**
- `p` — Patch to snapshot
- `visual` — True to snapshot visual map, false to snapshot height map

**Purpose:** Call before changing height map or visual map to capture pre-edit state.

**Calls:**
- `URDO_PatchUpdateVisual::Open(p)` if visual == true
- `URDO_PatchUpdateHeight::Open(p)` if visual == false

**Called by:** `asciiid.cpp` (many locations, before terrain edits)
- Line 531, 3836, 3872, 3900, 4037, 4140, 4180, 4557, 4654, 4741 (visual=true for visual map edits)
- Line 540, 4959, 5181 (visual=false for height map edits)

**Globals read:** None directly (delegates to Open methods)

**Globals mutated:** Via Open methods — creates PATCH_UPDATE_HEIGHT or PATCH_UPDATE_VISUAL operation

**Side effects:** Captures current height or visual map state in undo history

**Notes:** Must be called BEFORE modifying patch data. Captures the pre-edit state so it can be restored on undo. The SWAP pattern means undo and redo are identical (just swap data back and forth).

---

### `URDO_Diag(Patch*)` (urdo.cpp:621-624)

**File:** urdo.cpp:621-624

**Signature:** `void URDO_Diag(Patch* p)`

**Parameters:**
- `p` — Patch whose diagonal flags will be modified

**Purpose:** Call before flipping diagonal flags to capture pre-edit state.

**Calls:**
- `URDO_PatchDiag::Open(p)`

**Called by:** `asciiid.cpp` line 561, 9786 (diagonal flip operations)

**Globals read:** None directly (delegates to Open method)

**Globals mutated:** Via Open method — creates PATCH_DIAG operation

**Side effects:** Captures current diagonal flag state in undo history

**Notes:** Must be called BEFORE modifying diagonal flags. Captures the pre-edit state so it can be restored on undo. The SWAP pattern means undo and redo are identical.

---

## Typed Operation Methods

### `URDO_Group::Open()` (urdo.cpp:632-647)

**File:** urdo.cpp:632-647

**Signature:** `static void URDO_Group::Open()`

**Purpose:** Start new operation group, push onto stack.

**Calls:**
- `PurgeRedo()` — If top-level group (divergent history)
- `URDO::Alloc(CMD_GROUP)` — Allocate group operation

**Called by:** `URDO_Open()`

**Globals read:** `group_open`, `stack_depth`

**Globals mutated:** `group_open` (incremented), `stack[stack_depth]` (set to new group), `stack_depth` (incremented), `undo` (set to nullptr), `redo` (set to nullptr)

**Side effects:** Purges redo chain if top-level group. Pushes group onto stack. Resets undo/redo cursors.

**Notes:** Opening a top-level group (group_open==0) means user is making new edits after undo, so redo chain becomes invalid (divergent timeline). Purging redo frees that memory. Pushing onto stack allows subsequent URDO_* calls to add operations to the group's child list.

---

### `URDO_Group::Close()` (urdo.cpp:655-674)

**File:** urdo.cpp:655-674

**Signature:** `static void URDO_Group::Close()`

**Purpose:** End current operation group, seal as single undo unit.

**Calls:**
- `URDO::Free()` — If group is empty (group_head == nullptr)

**Called by:** `URDO_Close()`

**Globals read:** `group_open`, `stack_depth`, `stack[stack_depth]`

**Globals mutated:** `group_open` (decremented), `stack_depth` (decremented), `undo` (set to group or restored to parent)

**Side effects:** Seals group. Deletes empty groups.

**Notes:** If group_head is null, no operations were added to the group, so the group node itself is useless. Free it and restore undo pointer. If group has operations, sealed group becomes a single node in parent's list. The group's child operations (group_head to group_tail) remain accessible via the group node, but global undo pointer now points to the group itself.

---

### `URDO_Group::Do(bool un)` (urdo.cpp:676-696)

**File:** urdo.cpp:676-696

**Signature:** `void URDO_Group::Do(bool un)`

**Parameters:**
- `un` — True for undo (reverse order), false for redo (forward order)

**Purpose:** Execute all child operations in forward or reverse order.

**Calls:**
- `URDO::Do(un)` — Recursively execute each child operation

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `group_head`, `group_tail`

**Globals mutated:** None directly (child operations modify state)

**Side effects:** Executes all operations in the group (modifies terrain or world state)

**Notes:** If un==true (undo), walks from group_tail to group_head (reverse order). If un==false (redo), walks from group_head to group_tail (forward order). This ensures operations are undone in reverse execution order and redone in forward execution order.

---

### `URDO_PatchUpdateHeight::Open(Patch*)` (urdo.cpp:698-708)

**File:** urdo.cpp:698-708

**Signature:** `static void URDO_PatchUpdateHeight::Open(Patch* p)`

**Parameters:**
- `p` — Patch to snapshot

**Purpose:** Allocate operation, copy current height map and diag from patch.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_PATCH_UPDATE_HEIGHT)` — Allocate operation
- `memcpy(urdo->height, GetTerrainHeightMap(p), sizeof(uint16_t)*(HEIGHT_CELLS+1)*(HEIGHT_CELLS+1))` — Copy height map
- `GetTerrainDiag(p)` — Copy diagonal flags

**Called by:** `URDO_Patch(p, false)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates PATCH_UPDATE_HEIGHT operation

**Side effects:** Captures current height map and diag state

**Notes:** Must be called BEFORE modifying height map. Captures the pre-edit state so it can be restored on undo.

---

### `URDO_PatchUpdateVisual::Open(Patch*)` (urdo.cpp:710-719)

**File:** urdo.cpp:710-719

**Signature:** `static void URDO_PatchUpdateVisual::Open(Patch* p)`

**Parameters:**
- `p` — Patch to snapshot

**Purpose:** Allocate operation, copy current visual map from patch.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_PATCH_UPDATE_VISUAL)` — Allocate operation
- `memcpy(urdo->visual, GetTerrainVisualMap(p), sizeof(uint16_t)*VISUAL_CELLS*VISUAL_CELLS)` — Copy visual map

**Called by:** `URDO_Patch(p, true)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates PATCH_UPDATE_VISUAL operation

**Side effects:** Captures current visual map state

**Notes:** Must be called BEFORE modifying visual map. Captures the pre-edit state so it can be restored on undo.

---

### `URDO_PatchUpdateHeight::Do(bool)` (urdo.cpp:727-743)

**File:** urdo.cpp:727-743

**Signature:** `void URDO_PatchUpdateHeight::Do(bool un)`

**Parameters:**
- `un` — True for undo, false for redo (both are identical due to SWAP)

**Purpose:** SWAP height map and diag with patch.

**Calls:**
- `GetTerrainHeightMap(patch)` — Get pointer to patch's height map
- `GetTerrainDiag(patch)` — Get current diagonal flags
- `UpdateTerrainHeightMap(patch)` — Propagate changes to GPU and quadtree
- `SetTerrainDiag(patch, d)` — Update diagonal flags

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `patch`, `height`, `diag` (operation fields)

**Globals mutated:** Terrain's height map and diag (via GetTerrainHeightMap/SetTerrainDiag)

**Side effects:** SWAPS terrain height data with stored snapshot, updates GPU and quadtree

**Notes:** The SWAP pattern is the core insight. After swap, URDO holds NEW data (which becomes OLD data on next undo/redo), and terrain holds restored OLD data. Next Do() swaps again, restoring NEW. This makes undo and redo IDENTICAL operations (both just swap). O(1) time complexity (loop over height map cells, no allocation). No separate undo vs redo logic needed.

---

### `URDO_PatchUpdateVisual::Do(bool)` (urdo.cpp:750-762)

**File:** urdo.cpp:750-762

**Signature:** `void URDO_PatchUpdateVisual::Do(bool un)`

**Parameters:**
- `un` — True for undo, false for redo (both are identical due to SWAP)

**Purpose:** SWAP visual map with patch.

**Calls:**
- `GetTerrainVisualMap(patch)` — Get pointer to patch's visual map
- `UpdateTerrainVisualMap(patch)` — Propagate changes to GPU

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `patch`, `visual` (operation fields)

**Globals mutated:** Terrain's visual map (via GetTerrainVisualMap)

**Side effects:** SWAPS terrain visual data with stored snapshot, updates GPU

**Notes:** Same SWAP pattern as height update. After swap, URDO holds NEW visual data, terrain holds OLD visual data. Next Do() swaps again. Undo and redo are identical (both swap). This symmetry simplifies the code and ensures undo/redo correctness (no asymmetric bugs).

---

### `URDO_PatchDiag::Open(Patch*)` (urdo.cpp:765-774)

**File:** urdo.cpp:765-774

**Signature:** `static void URDO_PatchDiag::Open(Patch* p)`

**Parameters:**
- `p` — Patch to snapshot

**Purpose:** Allocate operation, copy current diag from patch.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_PATCH_DIAG)` — Allocate operation
- `GetTerrainDiag(p)` — Copy diagonal flags

**Called by:** `URDO_Diag(p)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates PATCH_DIAG operation

**Side effects:** Captures current diagonal flag state

**Notes:** Must be called BEFORE modifying diagonal flags. Captures the pre-edit state so it can be restored on undo.

---

### `URDO_PatchDiag::Do(bool)` (urdo.cpp:776-781)

**File:** urdo.cpp:776-781

**Signature:** `void URDO_PatchDiag::Do(bool un)`

**Parameters:**
- `un` — True for undo, false for redo (both are identical due to SWAP)

**Purpose:** SWAP diag with patch.

**Calls:**
- `GetTerrainDiag(patch)` — Get current diagonal flags
- `SetTerrainDiag(patch, d)` — Update diagonal flags

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `patch`, `diag` (operation fields)

**Globals mutated:** Terrain's diag (via SetTerrainDiag)

**Side effects:** SWAPS diagonal flags with stored snapshot

**Notes:** Same SWAP pattern. After swap, URDO holds NEW diag, terrain holds OLD diag. Next Do() swaps again. Undo and redo are identical.

---

### `URDO_PatchCreate::Delete(Terrain*, Patch*)` (urdo.cpp:783-795)

**File:** urdo.cpp:783-795

**Signature:** `static void URDO_PatchCreate::Delete(Terrain* t, Patch* p)`

**Parameters:**
- `t` — Terrain container
- `p` — Patch to delete

**Purpose:** Record patch deletion (detaches from terrain).

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_PATCH_CREATE)` — Allocate operation
- `TerrainDetach(t, p, &urdo->cx, &urdo->cy)` — Detach patch from quadtree

**Called by:** `URDO_Delete(Terrain*, Patch*)`

**Globals read:** `group_open`

**Globals mutated:** `bytes` (incremented by TerrainDetach return value), via `Alloc` — creates PATCH_CREATE operation with attached=false

**Side effects:** Detaches patch from terrain (removes from quadtree but keeps pointer valid)

**Notes:** TerrainDetach returns byte count of detached patch, which is added to `bytes` for memory tracking. The patch is detached but not freed (kept in URDO for potential undo).

---

### `URDO_PatchCreate::Create(Terrain*, int, int, int)` (urdo.cpp:797-811)

**File:** urdo.cpp:797-811

**Signature:** `static Patch* URDO_PatchCreate::Create(Terrain* t, int x, int y, int z)`

**Parameters:**
- `t` — Terrain container
- `x` — Patch X coordinate
- `y` — Patch Y coordinate
- `z` — Base height for new patch

**Returns:** Newly created Patch pointer

**Purpose:** Record patch creation (adds to terrain).

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_PATCH_CREATE)` — Allocate operation
- `AddTerrainPatch(t, x, y, z)` — Create and attach patch to quadtree

**Called by:** `URDO_Create(Terrain*, int, int, int)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates PATCH_CREATE operation with attached=true

**Side effects:** Creates patch, adds to terrain

**Notes:** AddTerrainPatch creates a new patch and inserts it into the quadtree. The operation is recorded in undo history with attached=true.

---

### `URDO_PatchCreate::Do(bool)` (urdo.cpp:819-831)

**File:** urdo.cpp:819-831

**Signature:** `void URDO_PatchCreate::Do(bool un)`

**Parameters:**
- `un` — True for undo, false for redo (both are identical due to toggle)

**Purpose:** Toggle attached state (TerrainDetach if attached, TerrainAttach if detached).

**Calls:**
- `TerrainDetach(terrain, patch, &cx, &cy)` — If attached==true
- `TerrainAttach(terrain, patch, cx, cy)` — If attached==false

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `terrain`, `patch`, `cx`, `cy`, `attached` (operation fields)

**Globals mutated:** `attached` (toggled), `bytes` (adjusted by TerrainDetach/TerrainAttach return values)

**Side effects:** Toggles patch attachment state (in terrain or detached in URDO)

**Notes:** Create and delete are inverse operations, so the SAME Do() function handles both by toggling whether the patch is attached to terrain. When attached=true, Do() detaches (TerrainDetach, mimics delete). When attached=false, Do() reattaches (TerrainAttach, mimics create). This symmetry means undo and redo are identical (just toggle the same flag), avoiding duplicate code.

---

### `URDO_InstCreate::Delete(Inst*)` (urdo.cpp:833-843)

**File:** urdo.cpp:833-843

**Signature:** `static void URDO_InstCreate::Delete(Inst* i)`

**Parameters:**
- `i` — Instance to delete

**Purpose:** Record instance deletion (SoftInstDel removes from world).

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_INST_CREATE)` — Allocate operation
- `SoftInstDel(i)` — Detach instance from world (keeps pointer valid)

**Called by:** `URDO_Delete(Inst*)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates INST_CREATE operation with attached=false

**Side effects:** Detaches instance from world (removes from BSP but keeps pointer valid)

**Notes:** SoftInstDel removes the instance from the world's BSP tree but does NOT free the instance memory. The pointer remains valid for potential undo.

---

### `URDO_InstCreate::Create(Mesh*, int, double[16], int)` (urdo.cpp:845-856)

**File:** urdo.cpp:845-856

**Signature:** `static Inst* URDO_InstCreate::Create(Mesh* m, int flags, double tm[16], int story_id)`

**Parameters:**
- `m` — Mesh resource to instantiate
- `flags` — Instance flags
- `tm` — 4x4 transform matrix (column-major double[16])
- `story_id` — Story/quest ID

**Returns:** Newly created Inst pointer

**Purpose:** Record mesh instance creation.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_INST_CREATE)` — Allocate operation
- `CreateInst(m, flags, tm, 0, story_id)` — Create and attach instance

**Called by:** `URDO_Create(Mesh*, int, double[16], int)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates INST_CREATE operation with attached=true

**Side effects:** Creates instance, adds to world

**Notes:** CreateInst creates a new MeshInst and inserts it into the world's BSP tree. The operation is recorded in undo history with attached=true. The name parameter is passed as 0 (null) to CreateInst.

---

### `URDO_InstCreate::Create(World*, Sprite*, int, float[3], float, int, int, int[4], int)` (urdo.cpp:858-869)

**File:** urdo.cpp:858-869

**Signature:** `static Inst* URDO_InstCreate::Create(World* w, Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], int story_id)`

**Parameters:**
- `w` — World container
- `s` — Sprite resource
- `flags` — Instance flags
- `pos` — Position [x, y, z]
- `yaw` — Rotation angle
- `anim` — Animation ID
- `frame` — Starting frame
- `reps` — Repetition counts [unused]
- `story_id` — Story/quest ID

**Returns:** Newly created Inst pointer

**Purpose:** Record sprite instance creation.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_INST_CREATE)` — Allocate operation
- `CreateInst(w, s, flags, pos, yaw, anim, frame, reps, 0, story_id)` — Create and attach instance

**Called by:** `URDO_Create(World*, Sprite*, int, float[3], float, int, int, int[4], int)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates INST_CREATE operation with attached=true

**Side effects:** Creates instance, adds to world

**Notes:** CreateInst creates a new SpriteInst and inserts it into the world's BSP tree. The operation is recorded in undo history with attached=true. The name parameter is passed as 0 (null) to CreateInst.

---

### `URDO_InstCreate::Create(World*, Item*, int, float[3], float, int)` (urdo.cpp:871-882)

**File:** urdo.cpp:871-882

**Signature:** `static Inst* URDO_InstCreate::Create(World* w, Item* item, int flags, float pos[3], float yaw, int story_id)`

**Parameters:**
- `w` — World container
- `item` — Inventory item resource
- `flags` — Instance flags
- `pos` — Position [x, y, z]
- `yaw` — Rotation angle
- `story_id` — Story/quest ID

**Returns:** Newly created Inst pointer

**Purpose:** Record item instance creation.

**Calls:**
- `PurgeRedo()` — If not inside group (divergent history)
- `URDO::Alloc(CMD_INST_CREATE)` — Allocate operation
- `CreateInst(w, item, flags, pos, yaw, story_id)` — Create and attach instance

**Called by:** `URDO_Create(World*, Item*, int, float[3], float, int)`

**Globals read:** `group_open`

**Globals mutated:** Via `Alloc` — creates INST_CREATE operation with attached=true

**Side effects:** Creates instance, adds to world

**Notes:** CreateInst creates a new ItemInst and inserts it into the world's BSP tree. The operation is recorded in undo history with attached=true.

---

### `URDO_InstCreate::Do(bool)` (urdo.cpp:885-897)

**File:** urdo.cpp:885-897

**Signature:** `void URDO_InstCreate::Do(bool un)`

**Parameters:**
- `un` — True for undo, false for redo (both are identical due to toggle)

**Purpose:** Toggle attached state (SoftInstDel if attached, SoftInstAdd if detached).

**Calls:**
- `SoftInstDel(inst)` — If attached==true
- `SoftInstAdd(inst)` — If attached==false

**Called by:** `URDO::Do(bool)` (dispatch)

**Globals read:** `inst`, `attached` (operation fields)

**Globals mutated:** `attached` (toggled)

**Side effects:** Toggles instance attachment state (in world or detached in URDO)

**Notes:** Create and delete are inverse operations, so the SAME Do() function handles both by toggling whether the instance is attached to world. When attached=true, Do() detaches (SoftInstDel, mimics delete). When attached=false, Do() reattaches (SoftInstAdd, mimics create). This symmetry means undo and redo are identical (just toggle the same flag), avoiding duplicate code.

---

## External Dependencies (terrain.h)

**File:** terrain.h:117-149, 187-188

### `AddTerrainPatch(Terrain*, int, int, int)` (terrain.h)
**Signature:** `Patch* AddTerrainPatch(Terrain* t, int x, int y, int z)`
**Purpose:** Create and attach terrain patch to quadtree
**Called by:** `URDO_PatchCreate::Create`

### `TerrainDetach(Terrain*, Patch*, int*, int*)` (terrain.h)
**Signature:** `size_t TerrainDetach(Terrain* t, Patch* p, int* x, int* y)`
**Purpose:** Detach patch from quadtree without freeing (for undo), returns byte count
**Called by:** `URDO_PatchCreate::Do`, `URDO_PatchCreate::Delete`

### `TerrainAttach(Terrain*, Patch*, int, int)` (terrain.h)
**Signature:** `size_t TerrainAttach(Terrain* t, Patch* p, int x, int y)`
**Purpose:** Re-attach detached patch to quadtree (for redo), returns byte count
**Called by:** `URDO_PatchCreate::Do`

### `TerrainDispose(Patch*)` (terrain.h)
**Signature:** `size_t TerrainDispose(Patch* p)`
**Purpose:** Free detached patch, returns byte count
**Called by:** `URDO::Free` (for PATCH_CREATE with attached=false)

### `GetTerrainHeightMap(Patch*)` (terrain.h)
**Signature:** `uint16_t* GetTerrainHeightMap(Patch* p)`
**Purpose:** Get pointer to patch's height map array
**Called by:** `URDO_PatchUpdateHeight::Open`, `URDO_PatchUpdateHeight::Do`

### `GetTerrainVisualMap(Patch*)` (terrain.h)
**Signature:** `uint16_t* GetTerrainVisualMap(Patch* p)`
**Purpose:** Get pointer to patch's visual map array
**Called by:** `URDO_PatchUpdateVisual::Open`, `URDO_PatchUpdateVisual::Do`

### `GetTerrainDiag(Patch*)` (terrain.h)
**Signature:** `uint16_t GetTerrainDiag(Patch* p)`
**Purpose:** Get patch's diagonal flags
**Called by:** `URDO_PatchUpdateHeight::Open`, `URDO_PatchUpdateHeight::Do`, `URDO_PatchDiag::Open`, `URDO_PatchDiag::Do`

### `SetTerrainDiag(Patch*, uint16_t)` (terrain.h)
**Signature:** `void SetTerrainDiag(Patch* p, uint16_t diag)`
**Purpose:** Set patch's diagonal flags
**Called by:** `URDO_PatchUpdateHeight::Do`, `URDO_PatchDiag::Do`

### `UpdateTerrainHeightMap(Patch*)` (terrain.h)
**Signature:** `void UpdateTerrainHeightMap(Patch* p)`
**Purpose:** Propagate height map changes to GPU and quadtree bounds
**Called by:** `URDO_PatchUpdateHeight::Do`

### `UpdateTerrainVisualMap(Patch*)` (terrain.h)
**Signature:** `void UpdateTerrainVisualMap(Patch* p)`
**Purpose:** Propagate visual map changes to GPU
**Called by:** `URDO_PatchUpdateVisual::Do`

---

## External Dependencies (world.h)

**File:** world.h:102-105, 209-211

### `CreateInst(World*, Item*, int, float[3], float, int)` (world.h)
**Signature:** `Inst* CreateInst(World* w, Item* item, int flags, float pos[3], float yaw, int story_id)`
**Purpose:** Create item instance
**Called by:** `URDO_InstCreate::Create(World*, Item*, ...)`

### `CreateInst(World*, Sprite*, int, float[3], float, int, int, int[4], const char*, int)` (world.h)
**Signature:** `Inst* CreateInst(World* w, Sprite* s, int flags, float pos[3], float yaw, int anim, int frame, int reps[4], const char* name, int story_id)`
**Purpose:** Create sprite instance
**Called by:** `URDO_InstCreate::Create(World*, Sprite*, ...)`

### `CreateInst(Mesh*, int, const double[16], const char*, int)` (world.h)
**Signature:** `Inst* CreateInst(Mesh* m, int flags, const double tm[16], const char* name, int story_id)`
**Purpose:** Create mesh instance
**Called by:** `URDO_InstCreate::Create(Mesh*, ...)`

### `SoftInstAdd(Inst*)` (world.h)
**Signature:** `void SoftInstAdd(Inst* i)`
**Purpose:** Re-attach instance to world (for redo)
**Called by:** `URDO_InstCreate::Do`

### `SoftInstDel(Inst*)` (world.h)
**Signature:** `void SoftInstDel(Inst* i)`
**Purpose:** Detach instance from world without freeing (for undo)
**Called by:** `URDO_InstCreate::Delete`, `URDO_InstCreate::Do`

### `HardInstDel(Inst*)` (world.h)
**Signature:** `void HardInstDel(Inst* i)`
**Purpose:** Free detached instance
**Called by:** `URDO::Free` (for INST_CREATE with attached=false)

---

## Call Graph Summary

### asciiid.cpp calls (86 direct URDO calls)

**URDO_Create(Terrain*, int, int, int):**
- Line 530 (patch creation on terrain edit)
- Line 9465 (patch creation in editor)

**URDO_Patch(Patch*, bool):**
- Lines 531, 3836, 3872, 3900, 4037, 4140, 4180, 4557, 4654, 4741 (visual=true)
- Lines 540, 4959, 5181 (visual=false)

**URDO_Diag(Patch*):**
- Lines 561, 9786 (diagonal flip)

**URDO_Create(Mesh*, int, double[16], int):**
- Lines 599, 10169 (mesh placement)

**URDO_Create(World*, Sprite*, ...):**
- Lines 6368, 6414, 6463, 10257 (sprite placement)

**URDO_Create(World*, Item*, ...):**
- Line 10327 (item placement)

**URDO_Delete(Inst*):**
- Lines 4870, 4889, 10195, 10225, 10296 (instance deletion)

**URDO_Delete(Terrain*, Patch*):**
- Line 9458 (patch deletion)

**URDO_Open():**
- Lines 706, 4061, 4158, 4191, 4776, 4869, 4887, 5066, 5173, 9557, 9594, 9707, 9804, 9896, 9905, 9991, 10000, 10460 (group start)

**URDO_Close():**
- Lines 728, 4063, 4160, 4193, 4778, 4873, 4890, 5069, 5227, 9478, 9532, 9559, 9571, 9596, 9608, 9907, 10002 (group end)

**URDO_CanUndo():**
- Lines 7988, 8026 (UI enable/disable)

**URDO_CanRedo():**
- Lines 8007, 8026 (UI enable/disable)

**URDO_Undo(int):**
- Lines 8001 (max_depth=0 for leaf), 8004 (max_depth=1 for group)

**URDO_Redo(int):**
- Lines 8020 (max_depth=1 for group), 8023 (max_depth=0 for leaf)

**URDO_Purge():**
- Lines 5569, 5911, 8036, 11231 (clear history)

**URDO_Bytes():**
- Line 8038 (memory display)

---

## Key Invariants

1. **group_open > 0** while between Open/Close (operations accumulate in group)
2. **Cannot undo/redo while group is open** (assert guards in URDO_Undo/Redo)
3. **PurgeRedo() called before new operations outside group** (divergent history)
4. **Detached patches/instances freed when operation freed** (no memory leaks)
5. **stack_depth tracks group nesting** (stack[64] holds group pointers)
6. **SWAP operations are symmetric** (undo and redo are identical for height/visual/diag)
7. **CREATE operations toggle attached flag** (undo and redo are identical for patch/inst)
8. **bytes counter tracks total memory** (incremented on Alloc, decremented on Free)
9. **Undo/redo cursors move along doubly-linked list** (prev/next pointers)
10. **Groups form tree mapped onto linear list** (dual bookkeeping in Alloc)

---

## Memory Tracking

**bytes global variable:**
- Incremented by `URDO::Alloc` (adds struct size)
- Incremented by `TerrainDetach` (adds detached patch size)
- Decremented by `URDO::Free` (subtracts struct size)
- Decremented by `TerrainDispose` (subtracts detached patch size when freeing)
- Decremented by `TerrainAttach` (subtracts reattached patch size)

**Per-operation sizes:**
- `sizeof(URDO_Group)` — Group container
- `sizeof(URDO_PatchCreate)` — Patch create/delete (plus detached patch data if attached=false)
- `sizeof(URDO_PatchUpdateHeight)` — Height snapshot (includes 5x5 array + diag)
- `sizeof(URDO_PatchUpdateVisual)` — Visual snapshot (includes 8x8 array)
- `sizeof(URDO_PatchDiag)` — Diag snapshot (minimal)
- `sizeof(URDO_InstCreate)` — Instance create/delete (no extra data, inst freed by HardInstDel if attached=false)

---

## Summary Statistics

**WROTE 38 function entries:**
1. URDO::Do(bool)
2. URDO::Free()
3. URDO::Alloc(CMD)
4. PurgeUndo()
5. PurgeRedo()
6. URDO_Purge()
7. URDO_CanUndo()
8. URDO_CanRedo()
9. URDO_Bytes()
10. URDO_Undo(int)
11. URDO_Redo(int)
12. URDO_Open()
13. URDO_Close()
14. URDO_Create(Mesh*, int, double[16], int)
15. URDO_Create(World*, Sprite*, int, float[3], float, int, int, int[4], int)
16. URDO_Create(World*, Item*, int, float[3], float, int)
17. URDO_Delete(Inst*)
18. URDO_Create(Terrain*, int, int, int)
19. URDO_Delete(Terrain*, Patch*)
20. URDO_Patch(Patch*, bool)
21. URDO_Diag(Patch*)
22. URDO_Group::Open()
23. URDO_Group::Close()
24. URDO_Group::Do(bool)
25. URDO_PatchUpdateHeight::Open(Patch*)
26. URDO_PatchUpdateHeight::Do(bool)
27. URDO_PatchUpdateVisual::Open(Patch*)
28. URDO_PatchUpdateVisual::Do(bool)
29. URDO_PatchDiag::Open(Patch*)
30. URDO_PatchDiag::Do(bool)
31. URDO_PatchCreate::Delete(Terrain*, Patch*)
32. URDO_PatchCreate::Create(Terrain*, int, int, int)
33. URDO_PatchCreate::Do(bool)
34. URDO_InstCreate::Delete(Inst*)
35. URDO_InstCreate::Create(Mesh*, int, double[16], int)
36. URDO_InstCreate::Create(World*, Sprite*, int, float[3], float, int, int, int[4], int)
37. URDO_InstCreate::Create(World*, Item*, int, float[3], float, int)
38. URDO_InstCreate::Do(bool)

**WROTE 6 global entries:**
1. bytes (size_t) — Total memory used by undo/redo history
2. undo (URDO*) — Cursor pointing to last executed operation
3. redo (URDO*) — Cursor pointing to next undone operation
4. group_open (int) — Nesting depth of open groups
5. stack_depth (int) — Current depth in group stack
6. stack[64] (URDO_Group*) — Stack of open groups

**WROTE 8 struct entries:**
1. URDO (base struct) — 3 fields, 3 methods
2. URDO::CMD (enum) — 6 values
3. URDO_Group — 2 fields, 3 methods
4. URDO_PatchCreate — 4 fields, 3 methods
5. URDO_PatchUpdateHeight — 3 fields, 2 methods
6. URDO_PatchUpdateVisual — 2 fields, 2 methods
7. URDO_PatchDiag — 2 fields, 2 methods
8. URDO_InstCreate — 2 fields (+commented-out 9 fields), 4 methods
