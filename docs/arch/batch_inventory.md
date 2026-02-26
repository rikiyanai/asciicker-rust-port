# inventory.cpp - Complete Function Inventory

**File:** `/Users/r/Downloads/asciicker-Y9-2/inventory.cpp` (760 lines)

**Purpose:** Grid-based inventory system with bitmask collision detection, directional focus navigation, item stacking, ownership transfer, and viewport scrolling. Items occupy rectangular regions in an 8x20 cell grid with O(1) collision checks for placement validation.

**Key System Properties:**
- Grid dimensions: 8 cells wide x 20 cells tall = 160 cells total
- Cell size: 4x4 pixels (sprite dimensions divided by 4)
- Bitmask occupancy: 20 bytes (160 cells / 8 bits per byte)
- Item states: EDIT (editor-only), WORLD (lying on ground), OWNED (in inventory)
- Ownership model: Items transition between states during gameplay lifecycle

---

## Function Inventory (8 Required Fields Per Function)

### `CreateItem` (inventory.cpp:180-185)

**Signature:** `Item* CreateItem()`

**Purpose:** Allocates and zero-initializes a new Item instance with all fields set to 0

**Called by:**
- `asciiid.cpp:10322` — editor item creation in ImGui panel
- `asciiid.cpp:10330` — editor world clone creation
- `game.cpp:3740` — armor drop from defeated enemy (human)
- `game.cpp:3752` — helmet drop from defeated enemy (human)
- `game.cpp:3764` — shield drop from defeated enemy (human)
- `game.cpp:3777` — weapon drop from defeated enemy (human)
- `game.cpp:3880` — armor drop from defeated enemy (creature)
- `game.cpp:3892` — helmet drop from defeated enemy (creature)
- `game.cpp:3904` — shield drop from defeated enemy (creature)
- `game.cpp:3917` — weapon drop from defeated enemy (creature)
- `game.cpp:4007` — armor drop from defeated buddy/ally
- `game.cpp:4019` — helmet drop from defeated buddy/ally
- `game.cpp:4031` — shield drop from defeated buddy/ally
- `game.cpp:4044` — weapon drop from defeated buddy/ally
- `world.cpp:3496` — EDIT to WORLD item cloning on map edit
- `world.cpp:3515` — EDIT to WORLD item cloning on map edit
- `world.cpp:3548` — EDIT to WORLD item cloning on map edit
- `world.cpp:5217` — item instantiation from .a3d file
- `world.cpp:5230` — clone for multiplayer

**Calls:**
- `malloc()` — standard C memory allocation
- `memset()` — zero-initialize allocated memory

**Globals read:** None

**Globals mutated:** None

**Side effects:**
- Allocates heap memory (must be freed by caller via `DestroyItem()`)
- All Item fields initialized to 0: proto=NULL, inst=NULL, count=0, purpose=0, xy[]={0,0}

**Notes:** Zero-initialization ensures all fields start in known state. Item struct is opaque to caller (proto, inst, count, purpose fields set after allocation). Must be paired with DestroyItem() to avoid memory leak.

---

### `DestroyItem` (inventory.cpp:191-196)

**Signature:** `void DestroyItem(Item* item)`

**Purpose:** Frees Item memory and deletes world instance if attached (detaches from BSP tree)

**Called by:**
- `inventory.cpp:737` — RemoveItem when consuming item (pos=NULL)
- `game.cpp:4223` — game shutdown, destroy all player inventory items
- `game.cpp:4272` — NPC/corpse cleanup during game shutdown
- `world.cpp:3594` — DeleteItemInsts during world cleanup (flat first deletion)
- `world.cpp:4468` — DeleteItemInsts during world cleanup
- `world.cpp:5737` — DeleteInst cleanup when removing world item

**Calls:**
- `DeleteInst()` — removes instance from BSP tree if inst != NULL

**Globals read:**
- `world` (extern) — required by DeleteInst() call

**Globals mutated:**
- World BSP tree state (via DeleteInst) — instance removed from spatial tree

**Side effects:**
- Frees heap memory allocated by CreateItem()
- If item->inst is non-NULL, removes from world BSP tree (affects visibility culling)
- Leaves world pointer in undefined state (caller must ensure world is valid)

**Notes:** Called when item is consumed (potion, food), removed from world permanently, or during game cleanup. Note: inst can be non-null for WORLD items; DeleteInst() is called first to avoid leak. After this, the Item pointer is invalid (do not dereference).

---

### `Inventory::UpdateLayout` (inventory.cpp:209-250)

**Signature:** `void Inventory::UpdateLayout(int render_width, int render_height, int scene_shift, int bars_pos)`

**Purpose:** Recalculates inventory viewport dimensions and scroll bounds based on window size and UI bar positions

**Called by:**
- `game.cpp:7087` — Game::Render main loop before drawing inventory UI

**Calls:** None (read-only access to inventory_sprite)

**Globals read:**
- `inventory_sprite` (extern Sprite*) — sprite frame width/height for viewport dimensions

**Globals mutated:**
- `Inventory::layout_width` — fixed at 39 characters (sprite frame width)
- `Inventory::layout_height` — actual visible height clamped between sprite height and max_height
- `Inventory::layout_max_height` — maximum height = 7 (top) + 4*height+1 (grid) + 5 (bottom) = 93
- `Inventory::layout_max_scroll` — how far viewport can scroll = max_height - visible_height
- `Inventory::layout_x, layout_y` — viewport position (left side of screen, vertically centered)
- `Inventory::layout_frame[4]` — inner grid bounds (excludes border): {left, top, right, bottom}
- `Inventory::layout_reps[3]` — repetition counts for three tileable regions

**Side effects:**
- Updates internal layout state
- Calculates scroll bounds and distribution across three sprite regions
- Does not trigger visual updates (caller must redraw)

**Notes:** Called every frame before inventory rendering. Three-region vertical tiling: inventory sprite has 3 vertically tileable regions; extra space distributed evenly to make smooth expansion. Descent parameter lowers inventory when health/status bars visible at bottom (bars_pos - 5). layout_frame[] defines inner grid bounds excluding 3-char left border, 7-char top, 6-char right/bottom margins.

---

### `Inventory::FocusNext` (inventory.cpp:333-484)

**Signature:** `void Inventory::FocusNext(int dx, int dy)`

**Purpose:** Finds best item to focus when user presses arrow keys using distance-weighted scoring with perpendicular penalty to prefer aligned items

**Called by:**
- `game.cpp:8276` — keyboard UP arrow (dx=0, dy=1)
- `game.cpp:8279` — keyboard DOWN arrow (dx=0, dy=-1)
- `game.cpp:8282` — keyboard LEFT arrow (dx=-1, dy=0)
- `game.cpp:8285` — keyboard RIGHT arrow (dx=1, dy=0)
- `game.cpp:10316` — gamepad D-pad UP (dx=0, dy=1)
- `game.cpp:10331` — gamepad D-pad DOWN (dx=0, dy=-1)
- `game.cpp:10352` — gamepad D-pad LEFT (dx=-1, dy=0)
- `game.cpp:10372` — gamepad D-pad RIGHT (dx=1, dy=0)

**Calls:**
- `SetFocus()` — changes focus index to best candidate

**Globals read:** None

**Globals mutated:**
- `Inventory::animate_scroll` — set to true to trigger scroll animation
- `Inventory::smooth_scroll` — stores source position for animation

**Side effects:**
- Changes focus index via SetFocus()
- Triggers smooth scroll animation
- No visual updates directly (caller must redraw)

**Notes:** Most algorithmically complex inventory function. Uses edge-to-edge distance measurement with 4x perpendicular penalty to strongly prefer aligned items over diagonal candidates. Algorithm steps:
1. Major axis determination: major_x = (dx²>dy²)
2. Focus point calculation: leading edge in direction of movement
3. Proximity point (closest point on candidate edge)
4. Dot product rejection: vx*dx + vy*dy >= 0 (rejects candidates behind direction)
5. Distance scoring: Horizontal: e = vx² + 4*vy² + cy², Vertical: e = 4*vx² + vy² + cx²

Example: Moving RIGHT from item at (1,5): focus point fx=6, fy=12. Item at (5,5) scores 16 (aligned). Item at (5,9) scores 164 (diagonal, 4 cells below), so 10x worse due to 4x penalty. Doubled coordinates provide sub-cell precision without floating point.

---

### `Inventory::SetFocus` (inventory.cpp:491-494)

**Signature:** `void Inventory::SetFocus(int index)`

**Purpose:** Changes currently focused item index for keyboard navigation

**Called by:**
- `inventory.cpp:483` — FocusNext changes focus to best candidate
- `inventory.cpp:751` — RemoveItem adjusts focus after removal (down)
- `inventory.cpp:753` — RemoveItem adjusts focus after removal (unchanged)
- `game.cpp:8923` — mouse click on inventory item
- `game.cpp:10314-10374` — gamepad touch selection (via indirect call in game loop)

**Calls:** None

**Globals read:** None

**Globals mutated:**
- `Inventory::focus` — currently focused item index

**Side effects:**
- Changes internal focus state
- Does not trigger scroll animation (caller must set animate_scroll if desired)
- Does not validate index bounds (caller responsible for valid range)

**Notes:** Simple setter that updates focus index. Used by FocusNext for directional navigation, RemoveItem for adjustment after deletion, and mouse/gamepad input handlers. Called by FocusNext but also directly by game.cpp input handlers. Does not perform scroll animation (caller must set animate_scroll=true separately if smooth scroll is desired).

---

### `Inventory::InsertItem` (inventory.cpp:503-664)

**Signature:** `bool Inventory::InsertItem(Item* item, int xy[2], const char* desc=0, const int* story_id=0)`

**Purpose:** Transfers item from world/corpse to player inventory, handles ownership transitions and bitmask updates

**Called by:**
- `game.cpp:4768` — PickItem() adds item from world to player inventory
- `game.cpp:9617` — PickItem() adds item from extended reach area

**Calls:**
- `DeleteInst()` — removes item from BSP tree (world items)
- `GetInstSpriteData()` — retrieves NPC Character pointer from Inst
- `UpdateSpriteInst()` — updates NPC sprite to show removed equipment
- `GetSprite()` — retrieves NPC's updated sprite
- `strncpy()` — copies item description into MyItem descriptor

**Globals read:**
- `world` (extern) — BSP tree for DeleteInst() call
- `item_proto_lib` (extern) — not directly accessed in function

**Globals mutated:**
- `Inventory::my_items` — incremented
- `Inventory::focus` — set to new item index
- `Inventory::animate_scroll` — set to true
- `Inventory::smooth_scroll` — set to current scroll position
- `Inventory::bitmask[]` — cells marked as occupied via bitwise OR
- World BSP tree (via DeleteInst) — instance removed from spatial tree
- NPC inventory state (if source is NPC) — item removed from NPC has[]
- NPC sprite state (if item was equipped) — equipment sprites updated

**Side effects:**
- Updates bitmask occupancy for grid collision detection
- Removes from world BSP tree (if WORLD item)
- Removes from NPC inventory (if looting corpse)
- Updates NPC sprite to show removed equipment
- Sets auto-focus to newly inserted item
- Triggers scroll animation
- Overwrites previous item state at my_items[my_items]

**Notes:** Handles two distinct ownership transitions:
1. WORLD → OWNED (pickup from ground): DeleteInst(), detach from BSP
2. OWNED (NPC) → OWNED (player): transfer from corpse inventory, update NPC sprite

Bitmask update: For each cell occupied by item, set bit: bitmask[i>>3] |= 1<<(i&7). Returns false if my_items >= max_items (inventory full). Returns true on success. Assertion fails if item not found in NPC inventory (shouldn't happen). Two code paths: NPC corpse looting (lines 519-615) and world pickup (lines 617-664).

---

### `Inventory::RemoveItem` (inventory.cpp:676-760)

**Signature:** `bool Inventory::RemoveItem(int index, float pos[3], float yaw)`

**Purpose:** Drops item from inventory back to world or destroys (consumes) based on destination

**Called by:**
- `inventory.cpp:737` — internal call when consuming item (pos=NULL)
- `game.cpp:4345` — consume item (potion, food) via RemoveItem(my_item, 0, 0)
- `game.cpp:4841` — drop item to world with position and rotation

**Calls:**
- `DestroyItem()` — when consuming item (pos=NULL)
- `CreateInst()` — when dropping to world (pos != NULL)
- `AttachInst()` — attaches dropped item to BSP tree

**Globals read:**
- `world` (extern) — required by CreateInst() and AttachInst()

**Globals mutated:**
- `Inventory::my_items` — decremented
- `Inventory::focus` — adjusted if removed item affects focus
- `Inventory::animate_scroll` — set to true
- `Inventory::smooth_scroll` — set to current scroll position
- `Inventory::bitmask[]` — cells marked as free via bitwise AND with NOT
- World BSP tree (via CreateInst/AttachInst) — instance added to spatial tree (if dropping)
- Item state (via DestroyItem or CreateInst) — changes purpose and inst

**Side effects:**
- Clears bitmask occupancy: bitmask[i>>3] &= ~(1<<(i&7)) for each cell
- Shifts remaining items down in array to fill gap
- Clears last slot (my_item[my_items].item = 0)
- Adjusts focus if removed item was before/at current focus
- Triggers scroll animation
- Destroys or drops item based on pos parameter

**Notes:** Ownership transitions:
1. OWNED → WORLD (drop to ground): CreateInst(), attach to BSP, sets purpose=WORLD
2. OWNED → destroyed (consume): DestroyItem(), warns if story_id >= 0

Bitmask clearing: For each cell, clear bit: bitmask[i>>3] &= ~(1<<(i&7)). Focus adjustment: if focus > index or focus == my_items, decrement focus; otherwise keep focus unchanged. Array compaction: shift remaining items down to preserve order (lines 741-745). Assertion validates: index >= 0, index < my_items, item is OWNED, item not attached to world (inst==0).

---

## Global Variables

| Variable | Type | Scope | Purpose |
|----------|------|-------|---------|
| `world` | `World*` (extern) | Shared with game.h | BSP tree for world items, used by CreateInst, DeleteInst, AttachInst |
| `inventory_sprite` | `Sprite*` (extern) | Shared with game.h | Sprite frames for inventory UI rendering, used in UpdateLayout |
| `item_proto_lib` | `const ItemProto*` | Shared with inventory.h | Item prototype library (unused in inventory.cpp, defined in external data) |

---

## Data Structures

### `Item` (defined in inventory.h)
- `proto`: const ItemProto* — item prototype (kind, sprite, dimensions)
- `inst`: Inst* — world instance (NULL for OWNED, non-NULL for WORLD/EDIT)
- `count`: int — quantity (typically 1)
- `purpose`: int — Item::EDIT, Item::WORLD, or Item::OWNED

### `Inventory::MyItem` (defined in inventory.h)
- `item`: Item* — pointer to item
- `xy[2]`: int — grid position (x, y) in cells
- `in_use`: bool — equipped/active flag
- `story_id`: int — unique story identifier
- `desc[32]`: char — custom description

### `Inventory` (defined in inventory.h)
- `my_items`: int — count of items in inventory (0-max_items)
- `max_items`: int — capacity (default 20)
- `width, height`: int — grid dimensions (8, 20)
- `focus`: int — currently focused item index
- `scroll, smooth_scroll`: int — viewport scroll position
- `animate_scroll`: bool — trigger scroll animation
- `layout_*`: various — viewport dimensions, position, bounds
- `bitmask[20]`: uint8_t — 160-bit occupancy map

---

## Integration Points

- **game.cpp**: Calls InsertItem for item pickup (lines 4768, 9617), RemoveItem for drop/consume (lines 4345, 4841), FocusNext for arrow key navigation (lines 8276-8285, 10316-10372), SetFocus for mouse click (line 8923), UpdateLayout before inventory rendering (line 7087)
- **asciiid.cpp**: Calls CreateItem for editor item creation (lines 10322, 10330)
- **world.cpp**: Calls CreateItem/DestroyItem for EDIT/WORLD transitions (lines 3496, 3515, 3548, 5217, 5230, 5737)

---

## Algorithm Details

### Bitmask Collision Detection
- Cell index: i = x + y*width (linear index 0-159)
- Byte position: i>>3 (divide by 8)
- Bit position: i&7 (modulo 8)
- Set bit: `bitmask[i>>3] |= 1<<(i&7)` → O(1)
- Clear bit: `bitmask[i>>3] &= ~(1<<(i&7))` → O(1)
- Test bit: `bitmask[i>>3] & (1<<(i&7))` → O(1)

### Directional Focus Navigation (FocusNext)
Uses doubled coordinates (2x scale) for sub-cell precision:
- **Step 1:** Major axis: major_x = (dx²>dy²)
- **Step 2:** Focus point (fx, fy) on current item's LEADING EDGE
- **Step 3:** Proximity point (px, py) — closest point on candidate edge
- **Step 4:** Center point (cx, cy) — for tie-breaking
- **Step 5:** Dot product rejection: vx*dx + vy*dy >= 0
- **Step 6:** Distance scoring:
  - Horizontal (major_x): e = vx² + 4*vy² + cy²
  - Vertical: e = 4*vx² + vy² + cx²

The 4x perpendicular penalty strongly prefers aligned items over diagonal candidates.

---

## Testing Considerations

- **CreateItem/DestroyItem**: Verify zero-initialization, heap allocation, memory cleanup
- **UpdateLayout**: Verify correct viewport dimensions, scroll bounds, region distribution
- **FocusNext**: Verify directional navigation, alignment preference, edge-to-edge distance
- **InsertItem**: Verify ownership transitions (WORLD→OWNED, NPC→OWNED), bitmask updates, focus auto-set
- **RemoveItem**: Verify drop to world (CreateInst), consume (DestroyItem), bitmask clearing, focus adjustment

---

## Known Issues/TODOs

- No explicit bounds checking on xy[] coordinates (caller responsible)
- focus index not validated on SetFocus() calls
- Bitmask cell indices assume width=8, height=20 (hardcoded)
- NPC equipment sprite updates only for Human type (creatures cannot equip)
- Story item consumption warning printed but not enforced

---

## Summary Statistics

| Property | Value |
|----------|-------|
| Total lines | 760 |
| Functions | 7 |
| Global variables read | 2 |
| Global variables mutated | 1 (inventory state) |
| Callers (total across all functions) | ~25 call sites |
| Grid capacity | 8×20 = 160 cells |
| Bitmask size | 20 bytes |
| Item ownership states | 3 (EDIT, WORLD, OWNED) |

