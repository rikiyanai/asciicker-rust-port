# game.cpp Analysis — Lines 4001-8000
# Generated: 2026-02-12
# Agent: Combat, AI, items

## SCOPE OVERVIEW

This range covers:
- **Character equipment and initialization (4001-4182)**: NPC buddy creation with random equipment
- **Game lifecycle functions (4184-4281)**: CreateGame, DeleteGame, FreeGame
- **Inventory item handling (4283-4846)**: ExecuteItem, CheckPick, CheckDrop, PickItem, DropItem, ScreenToCell
- **Character action state machine (4853-4999)**: SetActionNone, SetActionAttack, SetActionStand, SetActionFall, SetActionDead
- **Human equipment setters (5001-5098)**: SetWeapon, SetShield, SetHelmet, SetArmor, SetMount
- **Character speech system (5100-5159)**: Human::Say
- **Minimap rendering (5161-5394)**: DrawMiniText (static), RenderMinimap (static)
- **Game::Render main loop (5404-7873)**: Test harness, weather init, FPS tracking, physics integration, NPC AI pathfinding, combat hit detection, inventory UI, item pickup, status bars, keyboard, screenshot
- **Input handling (7875-8000+)**: OnSize, OnKeyb (partial - continues beyond 8000)

---

## GLOBAL VARIABLES (accessed in this range)

### `prime_game` (game.cpp global)
- **Type**: `Game*`
- **Purpose**: Singleton pointer to primary game instance (for single-player or main client)
- **Mutated by**: InitGame (line 4181), CreateGame (line 4193), DeleteGame (line 4207), FreeGame (line 4217)
- **Read by**: CreateGame (line 4192), DeleteGame (line 4207), FreeGame (line 4217)

### `player_head` (game.cpp global)
- **Type**: `Character*`
- **Purpose**: Head of doubly-linked list of all active characters (player + NPCs)
- **Mutated by**: InitGame (lines 4089-4096, 4114-4119), FreeGame (lines 4238, 4254, 4259)
- **Read by**: Game::Render NPC loop (line 5865)

### `player_tail` (game.cpp global)
- **Type**: `Character*`
- **Purpose**: Tail of character linked list
- **Mutated by**: InitGame (lines 4092-4094, 4118), FreeGame (lines 4242, 4259)

### `server` (game.cpp global)
- **Type**: `Server*`
- **Purpose**: Network server connection handle (null in offline mode)
- **Read by**: InitGame (line 4106), Game::Render (lines 5551, 7031, 7753)
- **Mutated by**: Game::Render (line 5552)

### `terrain` (game.cpp global)
- **Type**: `Terrain*`
- **Purpose**: Global terrain heightmap/quadtree
- **Read by**: InitGame (line 4098), HitTerrain calls, RenderMinimap (line 5669), UpdateSnowAccumulation (line 6671)

### `world` (game.cpp global)
- **Type**: `World*`
- **Purpose**: Global BSP world containing meshes and sprite instances
- **Read by**: InitGame (line 4080), Game::Render (line 6423), UpdateSpriteInst calls

### `item_proto_lib` (game.cpp global)
- **Type**: `ItemProto*` (array)
- **Purpose**: Prototype library for all item types (weapons, armor, consumables)
- **Read by**: InitGame equipment generation (lines 4010, 4022, 4034, 4048-4057), ConsumeAnim handlers (lines 7193-7214)

### `water` (game.cpp global)
- **Type**: `int`
- **Purpose**: Global water level (Y-coordinate threshold for water surface)
- **Read by**: Game::Render physics integration (line 5640)

### `blood` (game.cpp global)
- **Type**: `bool` (inferred)
- **Purpose**: Global flag to enable/disable blood particle effects
- **Read by**: BloodLeak calls (lines 5850, 6129, 6205)

### `inventory_sprite` (game.cpp global)
- **Type**: `Sprite*`
- **Purpose**: Sprite containing inventory UI frame graphics
- **Read by**: CheckPick (line 4478), CheckDrop (line 4518), inventory rendering (line 7091)

### `character_button` (game.cpp global)
- **Type**: `Sprite*`
- **Purpose**: Sprite containing character/inventory button graphics
- **Read by**: button rendering (lines 7674-7675)

### `consume_anim` (game.cpp global)
- **Type**: `ConsumeAnim[16]` (array)
- **Purpose**: Active item consumption animations (items shrinking/disappearing)
- **Mutated by**: ExecuteItem (lines 4331-4343), consume_anim loop (lines 7186-7238)
- **Read by**: consume_anim rendering loop (lines 7186-7238)

### `consume_anims` (game.cpp global)
- **Type**: `int`
- **Purpose**: Count of active consume animations (max 16)
- **Mutated by**: ExecuteItem (lines 4331-4343), consume_anim cleanup (lines 7218-7220)

### `attack_us_per_frame` (game.cpp global)
- **Type**: `uint64_t` (inferred)
- **Purpose**: Microseconds per frame for attack animations
- **Read by**: attack animation frame index calculations (lines 6253, 6336, 6448, 6561)

### `stand_us_per_frame` (game.cpp global)
- **Type**: `uint64_t` (inferred)
- **Purpose**: Microseconds per frame for stand-up/fall animations
- **Read by**: stand/fall animation frame index calculations (lines 4932, 4969, 6362, 6374, 6590, 6604)

### `fall_us_per_frame` (game.cpp global)
- **Type**: `uint64_t` (inferred)
- **Purpose**: Microseconds per frame for fall animations
- **Read by**: SetActionFall stamp recalc (line 4969)

### `fast_rand` (function, game.cpp)
- **Type**: `int fast_rand()`
- **Purpose**: Fast pseudorandom number generator
- **Called by**: InitGame (line 4075), combat knockback (lines 6283, 6289-6290, 6513, 6518-6520), NPC unstuck (line 6185)

### `TalkBox_blink` (game.cpp global)
- **Type**: `int`
- **Purpose**: Counter for blinking text cursor in talk boxes
- **Mutated by**: Game::Render (line 5588)
- **Read by**: TalkBox::Paint call (line 7679)

### `KeybAutoRepChar` (game.cpp global)
- **Type**: `char`
- **Purpose**: Character for keyboard auto-repeat
- **Read by**: Game::Render (lines 5580, 5590, 5592)
- **Mutated by**: OnKeyb (sets to char on KEYB_DOWN, clears on KEYB_UP)
- **Reset by**: Game::Render (lines 5584, 5599)

### `KeybAuroRepDelayStamp` (game.cpp global)
- **Type**: `uint64_t`
- **Purpose**: Timestamp for keyboard auto-repeat delay
- **Mutated by**: Game::Render (line 5596)
- **Read by**: Game::Render (line 5593)

### `PressKey` (game.cpp global)
- **Type**: `int`
- **Purpose**: Currently held key (for auto-repeat logic)
- **Read by**: Game::Render (line 5574)
- **Mutated by**: Game::Render (line 5582), OnKeyb

### `PressStamp` (game.cpp global)
- **Type**: `uint64_t`
- **Purpose**: Timestamp when PressKey was pressed
- **Read by**: Game::Render (line 5574)

### `weather` (game.cpp global)
- **Type**: `Weather*`
- **Purpose**: Global weather system state (snow particles, intensity)
- **Read by**: Game::Render (lines 6666-6684)
- **Mutated by**: CreateWeather() call (line 5547)

### `HEIGHT_CELLS` (macro/const, world.h)
- **Purpose**: Cells per terrain patch edge (heightmap resolution)
- **Read by**: RenderMinimap terrain sampling (line 5232)

### `VISUAL_CELLS` (macro/const, world.h)
- **Purpose**: Cells per terrain patch edge (visual/material map resolution)
- **Read by**: RenderMinimap visual sampling (line 5251)

### `HEIGHT_SCALE` (macro/const, game.cpp)
- **Purpose**: Multiplier for height values in physics calculations
- **Read by**: DropItem raycasts (lines 4797, 4805)

### `M_PI` (math.h constant)
- **Purpose**: Pi constant for trigonometry
- **Read by**: Numerous trig calculations throughout

---

## STRUCT DEFINITIONS

### `ConsumeAnim` (game.cpp, inferred from usage)
**Members:**
- `int pos[2]` — inventory grid position where animation plays
- `Sprite* sprite` — 2D sprite to animate (item icon)
- `uint64_t stamp` — timestamp when animation started

**Purpose**: Tracks an active item consumption animation (item shrinking/disappearing after use)

**Lifecycle**: Created when item consumed (ExecuteItem line 4337), rendered each frame (lines 7186-7238), destroyed after max_elaps frames (lines 7191-7221)

---

## ENUMS

### `ACTION` (inferred from usage)
- `ACTION::NONE` — Idle or walking
- `ACTION::ATTACK` — Attack animation playing
- `ACTION::STAND` — Standing up animation
- `ACTION::FALL` — Falling/dying animation (plays in reverse from stand)
- `ACTION::DEAD` — Dead, frozen at last fall frame

**Usage**: Character::req.action field, checked by SetAction* methods, physics force gating

### `WEAPON` / `PLAYER_WEAPON_INDEX` (inferred from usage)
- `WEAPON::NONE` — Unarmed
- `WEAPON::REGULAR_SWORD` / `PLAYER_WEAPON_INDEX::SWORD` — Melee sword
- `WEAPON::REGULAR_CROSSBOW` / `PLAYER_WEAPON_INDEX::CROSSBOW` — Ranged crossbow

**Usage**: Character::req.weapon field, equipment generation (line 4002), SetWeapon, attack animation dispatch (lines 6247, 6332, 6442, 6557)

### `MOUNT` (inferred from usage)
- `MOUNT::NONE` — On foot
- `MOUNT::WOLF` — Riding wolf
- `MOUNT::BEE` — Riding giant bee (flight capable)

**Usage**: Character::req.mount field, equipment restrictions (lines 4368, 4397, 4426, 4455), dismount on death (lines 6301, 6531)

### `ARMOR`, `HELMET`, `SHIELD` (inferred from usage)
- `ARMOR::NONE`, `HELMET::NONE`, `SHIELD::NONE` — No equipment
- Various indexed equipment types (indexed 0-N)

**Usage**: Character::req fields, equipment generation (lines 3999-4040), SetArmor/SetHelmet/SetShield

### `Input::Contact` action enum (inferred from usage)
- `Input::Contact::NONE` — No contact active
- `Input::Contact::FORCE` — Touch/mouse drag for movement force
- `Input::Contact::TORQUE` — Touch/mouse drag for camera rotation
- `Input::Contact::ITEM_GRID_CLICK` — Clicked inventory item
- `Input::Contact::ITEM_LIST_CLICK` — Clicked pickup list item
- `Input::Contact::ITEM_GRID_DRAG` — Dragging inventory item
- `Input::Contact::ITEM_LIST_DRAG` — Dragging pickup list item

**Usage**: Game::input.contact[0..3].action field, contact handling in Render

---

## FUNCTIONS (lines 4001-8000)

### Continuation of InitGame buddy initialization (game.cpp:4001-4101)

**Signature:** (continuation, not a function boundary)
**Purpose:** Generates random equipment for NPC buddy, creates physics instance, attaches to world, inserts into character linked list
**Called by:** InitGame (earlier in file)
**Calls:** CreateItem, GetSprite, CreateInst, SetInstSpriteData, AttachInst, CreatePhysics
**Globals read:** item_proto_lib, world, terrain, player_head, player_tail
**Globals mutated:** player_head, player_tail
**Side effects:** Allocates Item structs, creates world instance, modifies character linked list
**Notes:** Equipment generation uses rand() % 2 for armor/helmet/shield (picks one of two variants), rand() % 4 for weapons (with id gap workaround at line 4052). Random spawn offset within 21x21 cell radius (line 4075). Sets buddy->dist = 10 (commented out at line 4099, unknown purpose). This code is wrapped in `#ifndef EDITOR` (line 3963), not compiled in editor build.

---

### Continuation of InitGame player setup (game.cpp:4103-4182)

**Signature:** (continuation, not a function boundary)
**Purpose:** Copies player name, initializes network lag tracking, inserts player into character list, creates renderer and physics, initializes player stats and equipment, creates player world instance
**Called by:** InitGame (earlier in file)
**Calls:** strcpy, CreateRenderer, CreatePhysics, GetSprite, CreateInst, SetInstSpriteData
**Globals read:** server, player_head, player_tail, terrain, world, prime_game
**Globals mutated:** server->last_lag, server->lag_ms, server->lag_wait, player_head, player_tail, prime_game
**Side effects:** Creates Renderer, creates Physics, allocates player_inst world instance
**Notes:** Player initialized with HP=200, max_speed=150 (increased, comment line 4138), mount=WOLF, no armor/weapon. Fly mode forced on for PURE_TERM builds (line 4162). Player instance flags INST_USE_TREE | INST_VISIBLE | INST_VOLATILE, story_id=-1 (not in story system). Player inserted at head of character list (lines 4113-4119).

---

### `CreateGame` (game.cpp:4184-4203)

**Signature:** `Game* CreateGame()`
**Purpose:** Allocates and initializes a Game struct with default configuration, sets main_menu flag
**Called by:** game_web.cpp:772, term.cpp:426, game_app.cpp:2484
**Calls:** malloc, memset, ReadConf
**Globals read:** prime_game
**Globals mutated:** prime_game
**Side effects:** Allocates Game struct (sizeof(Game)), reads config file
**Notes:** Sets `main_menu = true` for non-EDITOR builds (lines 4198-4199), meaning game starts at main menu screen. EDITOR builds set `main_menu = false` (line 4196) to skip menu. `prime_game` singleton pointer set if null (lines 4192-4193). Does NOT call InitGame — that happens later when user clicks "Play" in MainMenu or when test mode forces init (line 5476).

---

### `DeleteGame` (game.cpp:4205-4211)

**Signature:** `void DeleteGame(Game* g)`
**Purpose:** Clears prime_game singleton pointer if match, frees Game struct
**Called by:** No callers found via grep
**Calls:** free
**Globals read:** prime_game
**Globals mutated:** prime_game
**Side effects:** Frees Game struct memory
**Notes:** Simple teardown, does NOT call FreeGame (which would clean up resources). This is  a mistake — callers should call FreeGame before DeleteGame. Contrast with FreeGame which performs full cleanup.

---

### `FreeGame` (game.cpp:4213-4281)

**Signature:** `void FreeGame(Game* g)`
**Purpose:** Full game teardown — cleans up player resources, deletes physics/renderer, removes buddies (non-EDITOR), unlinks player from character list
**Called by:** No callers found via grep
**Calls:** DeleteInst, DestroyItem, free, DeleteRenderer, DeletePhysics
**Globals read:** prime_game, player_head, player_tail
**Globals mutated:** prime_game, player_head, player_tail
**Side effects:** Deallocates inventory items, talk boxes, renderer, physics, NPC physics and items. Modifies character linked list.
**Notes:** Non-EDITOR builds iterate character list (lines 4244-4278), delete all NPCs whose `data != g->physics` (i.e. buddies, not player), call DeletePhysics on each, free NPC structs. Item inst pointers set to 0 before DestroyItem (line 4271) to prevent world instance cleanup. Player unlinked from character list (lines 4235-4242). Clears prime_game if match (lines 4217-4218).

---

### `Game::CancelItemContacts` (game.cpp:4283-4293)

**Signature:** `void Game::CancelItemContacts()`
**Purpose:** Clears all item-related touch/mouse contacts (cancel drag operations)
**Called by:** No callers found via grep
**Calls:** None
**Globals read:** None
**Globals mutated:** None (mutates Game::input.contact[0..3].action)
**Side effects:** Resets Input::Contact::action to NONE for all item drag/click contacts
**Notes:** Loops 4 contacts (touch points 0-3), checks if action is ITEM_GRID_CLICK, ITEM_LIST_CLICK, ITEM_GRID_DRAG, or ITEM_LIST_DRAG (lines 4287-4290), sets action to NONE. Used to cancel item drags when UI state changes.

---

### `Game::ExecuteItem` (game.cpp:4295-4473)

**Signature:** `void Game::ExecuteItem(int my_item)`
**Purpose:** Consumes or equips an inventory item based on item kind (food/potion/weapon/shield/helmet/armor/ring)
**Called by:** No callers found via grep
**Calls:** akAPI_OnItem, ConsumeAnim setup, Inventory::RemoveItem, Human::SetWeapon, Human::SetShield, Human::SetHelmet, Human::SetArmor, Human::SetMount
**Globals read:** consume_anim, consume_anims, inventory, player, stamp
**Globals mutated:** consume_anim, consume_anims, inventory, player.req.weapon/shield/helmet/armor/mount
**Side effects:** Decrements item count or removes item from inventory, creates consume animation, changes player equipment sprite, calls story API
**Notes:** Item kind dispatch (lines 4320-4472): 'F' (food) / 'P' (potion) / 'D' (drink) → consume, decrement count, create ConsumeAnim if last (lines 4331-4343), call Inventory::RemoveItem. 'R' (ring) → toggle in_use flag (line 4353). 'W' (weapon) → call SetWeapon, unequip conflicting weapon, dismount if mounted (lines 4357-4383). 'S' (shield) / 'H' (helmet) / 'A' (armor) → similar pattern (lines 4386-4471). Calls akAPI_OnItem with 'U' (unequip) or 'E' (equip) token (line 4304), checks `allowed` flag (lines 4314-4317). ConsumeAnim array limited to 16 (line 4331), uses memmove to shift array if full. Equipment conflicts resolved by scanning inventory for conflicting items of same kind (lines 4372-4378, etc).

---

### `Game::CheckPick` (game.cpp:4475-4511)

**Signature:** `int Game::CheckPick(const int cp[2])`
**Purpose:** Given cell coords, returns inventory item index under cursor, or -1 if none
**Called by:** No callers found via grep
**Calls:** None
**Globals read:** inventory_sprite, inventory, render_size
**Globals mutated:** None
**Side effects:** None
**Notes:** Clamps scroll (lines 4483-4487), checks if cp inside inventory frame (line 4494), converts to bitmask coords (lines 4496-4497), loops inventory.my_items (lines 4499-4507), checks if cp inside item's xy + sprite bounds (lines 4501-4505). Returns first match index or -1. Used for hit-testing inventory items under mouse/touch.

---

### `Game::CheckDrop` (game.cpp:4513-4723)

**Signature:** `bool Game::CheckDrop(int c, int drop_xy[2], AnsiCell* ptr, int width, int height)`
**Purpose:** Checks if contact c can drop dragged item at current position, optionally paints yellow hilight rect, returns true if valid drop location
**Called by:** Game::Render inventory rendering (line 7183)
**Calls:** ScreenToCell, FillRect (indirect), AverageGlyph
**Globals read:** inventory_sprite, inventory, input
**Globals mutated:** ptr (paints hilight or DROP/PICK indicators)
**Side effects:** Draws yellow hilight rect if fit, draws red if collision, draws "DROP" or "PICK" text indicators outside inventory
**Notes:** Only processes ITEM_LIST_DRAG or ITEM_GRID_DRAG contacts (lines 4515-4516). Clamps scroll (lines 4520-4524), converts contact pos to cell coords (lines 4526-4527), checks if inside inventory frame (line 4534). Calculates quantized drop position (lines 4540-4559), tests bitmask collision (lines 4563-4606). Special case for ITEM_GRID_DRAG with drag==1 or count==1: excludes self's bitmask space from collision test (lines 4570-4590). If fit and ptr non-null, paints yellow hilight rect (lines 4614-4644). If outside inventory and ITEM_LIST_DRAG, paints "PICK" indicator (lines 4651-4673). If outside inventory and ITEM_GRID_DRAG, paints "DROP" indicator (lines 4678-4708), but only if item not in_use (line 4691). Sets drop_xy output (lines 4608-4612, 4710-4713). Returns true if valid drop (lines 4647, 4716), false otherwise (line 4722). Used to validate and visualize item drag-drop operations.

---

### `Game::PickItem` (game.cpp:4725-4774)

**Signature:** `bool Game::PickItem(Item* item)`
**Purpose:** Adds item to inventory, auto-calculates grid position (bottom-up, left-to-right scan), calls story API
**Called by:** Game::Render item pickup handler (line 7462)
**Calls:** akAPI_OnItem, Inventory::InsertItem, GetInstStoryID
**Globals read:** inventory
**Globals mutated:** inventory
**Side effects:** Inserts item into inventory.my_item array, updates bitmask
**Notes:** Calculates item cell dimensions from sprite (lines 4728-4729), scans inventory bitmask for free space (lines 4731-4745), bottom-up scan (y loop 4732), left-to-right scan (x loop 4734), tests all cells in item rect (lines 4737-4743). First fit found, calls akAPI_OnItem with 'P' (PICK) token (lines 4752-4759), checks allowed flag (lines 4761-4765), calls Inventory::InsertItem (line 4768), returns true. If no space found, returns false (line 4773). Used when player presses '1'-'N' key over item in pickup list (line 7462).

---

### `Game::DropItem` (game.cpp:4776-4845)

**Signature:** `bool Game::DropItem(int index)`
**Purpose:** Removes item from inventory, places world instance near player via raycast, calls story API
**Called by:** No callers found via grep
**Calls:** HitTerrain, HitWorld, akAPI_OnItem, Inventory::RemoveItem
**Globals read:** inventory, player, terrain, world, prev_yaw
**Globals mutated:** inventory
**Side effects:** Creates world item instance at raycast position, removes from inventory
**Notes:** Asserts index valid (line 4780), generates random angle (line 4781), calculates drop position 2 units in front of player (lines 4784-4789), raycasts downward from player.pos[2] + 3*HEIGHT_SCALE to find ground (lines 4792-4810), prefers terrain hit, falls back to world mesh hit. If hit found, converts to float pos (lines 4814-4819), calls akAPI_OnItem with 'D' (DROP) token (lines 4824-4832), checks allowed (lines 4834-4838), calls Inventory::RemoveItem with pos and prev_yaw to create world instance (line 4841). Returns true if placed (line 4844), false if no hit (line 4844, implicit false if !ok at line 4812).

---

### `Game::ScreenToCell` (game.cpp:4847-4851)

**Signature:** `void Game::ScreenToCell(int p[2]) const`
**Purpose:** Converts screen pixel coords to cell coords (for font-based rendering), mutates p[2] in-place
**Called by:** CheckDrop (line 4527), contact item hit testing
**Calls:** None
**Globals read:** input.size[0..1], render_size[0..1], font_size[0..1]
**Globals mutated:** None (mutates p[] parameter)
**Side effects:** None
**Notes:** Formula: `p[0] = (2*p[0] - input.size[0] + render_size[0] * font_size[0]) / (2 * font_size[0])` (line 4849), similar for p[1] (line 4850). Converts from screen pixel space (input.size) to cell space (render_size / font_size). Y coordinate inverts (input.size[1]-1 - 2*p[1]) because screen Y is top-down, cell Y is bottom-up.

---

### `Character::SetActionNone` (game.cpp:4853-4872)

**Signature:** `bool Character::SetActionNone(uint64_t stamp)`
**Purpose:** Transitions character to ACTION::NONE (idle/walk), updates sprite if needed
**Called by:** Game::Render attack animation end (lines 6326, 6351, 6551, 6579), player TELEPORT command (line 5514)
**Calls:** GetSprite
**Globals read:** None (uses this->req, this->clr, this->sprite)
**Globals mutated:** this->req.action, this->sprite, this->anim, this->frame, this->action_stamp
**Side effects:** Changes sprite pointer if action change requires different sprite
**Notes:** Early return if already NONE (lines 4855-4856). Attempts to change req.action to NONE (line 4858), calls GetSprite with new req (line 4860), reverts if sprite null (lines 4861-4864). Sets anim=0, frame=0, action_stamp=stamp (lines 4868-4870). Returns true on success (line 4871), false if sprite unavailable. Used when attack/stand/fall animation completes.

---

### `Character::SetActionAttack` (game.cpp:4874-4908)

**Signature:** `bool Character::SetActionAttack(uint64_t stamp)`
**Purpose:** Transitions character to ACTION::ATTACK, initializes attack animation frame based on weapon type
**Called by:** Game::Render NPC AI combat (line 6062), player crossbow shoot (line 6744), player attack input (elsewhere)
**Calls:** GetSprite
**Globals read:** None (uses this->req)
**Globals mutated:** this->req.action, this->sprite, this->anim, this->frame, this->action_stamp, this->hit_tested
**Side effects:** Changes sprite pointer, resets hit_tested flag
**Notes:** Early return if already ATTACK (lines 4876-4877). Blocks if currently FALL, STAND, or DEAD (lines 4878-4881). Attempts to change req.action to ATTACK (line 4884), calls GetSprite (line 4886), reverts if null (lines 4887-4890). Sets anim=0, frame based on weapon: crossbow starts at frame 0 (line 4896), melee starts at frame 2 (line 4902). Sets action_stamp=stamp, hit_tested=false (lines 4904-4905). Returns true on success (line 4907), false if sprite unavailable or blocked. hit_tested flag ensures hit test happens exactly once per attack (checked at lines 6259, 6451).

---

### `Character::SetActionStand` (game.cpp:4910-4942)

**Signature:** `bool Character::SetActionStand(uint64_t stamp)`
**Purpose:** Transitions character to ACTION::STAND (standing up animation), adjusts stamp to match current frame if coming from FALL
**Called by:** No callers found via grep
**Calls:** GetSprite
**Globals read:** None (uses this->req)
**Globals mutated:** this->req.action, this->sprite, this->action_stamp, this->anim, this->frame
**Side effects:** Changes sprite pointer
**Notes:** Early return if already STAND (lines 4912-4913). Blocks if not currently FALL or DEAD (lines 4915-4916). Attempts to change req.action to STAND (line 4919), calls GetSprite (line 4921), reverts if null (lines 4922-4925). If coming from FALL, recalculates action_stamp to match current frame (lines 4929-4932), preserving animation continuity. If coming from DEAD, sets anim=0, frame=0, action_stamp=stamp (lines 4935-4938). Returns true on success (line 4941), false if sprite unavailable or blocked. STAND animation plays forward from frame 0 to length-1, then transitions to NONE (line 6377).

---

### `Character::SetActionFall` (game.cpp:4944-4978)

**Signature:** `bool Character::SetActionFall(uint64_t stamp)`
**Purpose:** Transitions character to ACTION::FALL (dying animation), adjusts stamp to match current frame if coming from STAND (plays STAND in reverse)
**Called by:** Game::Render death handlers (lines 6313, 6365, 6543, 6595)
**Calls:** GetSprite
**Globals read:** None (uses this->req)
**Globals mutated:** this->req.action, this->sprite, this->anim, this->frame, this->action_stamp
**Side effects:** Changes sprite pointer
**Notes:** Early return if already FALL (lines 4946-4947). Blocks if currently DEAD (lines 4949-4950). Attempts to change req.action to FALL (line 4953), calls GetSprite (line 4955), reverts if null (lines 4956-4959). Clamps anim if out of bounds (lines 4963-4964). If coming from STAND, recalculates action_stamp to preserve animation continuity (lines 4965-4969), uses (len - frame) to reverse playback. If coming from other action, sets anim=0, frame=length-1 (last frame) (lines 4972-4975). Returns true on success (line 4978), false if sprite unavailable or blocked. FALL animation plays backward from length-1 to 0, then transitions to DEAD (line 6365).

---

### `Character::SetActionDead` (game.cpp:4981-4999)

**Signature:** `bool Character::SetActionDead(uint64_t stamp)`
**Purpose:** Transitions character to ACTION::DEAD (frozen at last fall frame), terminal state
**Called by:** Game::Render fall animation end (lines 6365, 6595)
**Calls:** GetSprite
**Globals read:** None (uses this->req)
**Globals mutated:** this->req.action, this->sprite, this->anim, this->frame, this->action_stamp
**Side effects:** Changes sprite pointer
**Notes:** Does NOT check if already DEAD (no early return). Attempts to change req.action to DEAD (line 4984), calls GetSprite (line 4986), reverts if null (lines 4987-4990). Sets anim=0, frame=0, action_stamp=stamp (lines 4994-4996). Returns true on success (line 4998), false if sprite unavailable. Terminal state — no animation plays, character stays at frame 0 of DEAD sprite. DEAD action checked in Game::Render to block AI forces (line 5886).

---

### `Human::SetWeapon` (game.cpp:5001-5020)

**Signature:** `bool Human::SetWeapon(int w)`
**Purpose:** Equips weapon, updates sprite
**Called by:** Game::ExecuteItem weapon case (lines 4361, 4370)
**Calls:** GetSprite
**Globals read:** None (uses this->req, this->clr)
**Globals mutated:** this->req.weapon, this->sprite
**Side effects:** Changes sprite pointer
**Notes:** Blocks if currently ATTACK (lines 5003-5004). Early return if already equipped (lines 5005-5006). Attempts to change req.weapon to w (line 5009), calls GetSprite (line 5011), reverts if null (lines 5012-5015). Returns true on success (line 5019), false if sprite unavailable or blocked. No animation reset — frame/anim unchanged.

---

### `Human::SetShield` (game.cpp:5022-5039)

**Signature:** `bool Human::SetShield(int s)`
**Purpose:** Equips shield, updates sprite
**Called by:** Game::ExecuteItem shield case (lines 4390, 4399)
**Calls:** GetSprite
**Globals read:** None (uses this->req, this->clr)
**Globals mutated:** this->req.shield, this->sprite
**Side effects:** Changes sprite pointer
**Notes:** Early return if already equipped (lines 5024-5025). Does NOT block during ATTACK (unlike SetWeapon). Attempts to change req.shield to s (line 5028), calls GetSprite (line 5030), reverts if null (lines 5031-5034). Returns true on success (line 5038), false if sprite unavailable.

---

### `Human::SetHelmet` (game.cpp:5041-5058)

**Signature:** `bool Human::SetHelmet(int h)`
**Purpose:** Equips helmet, updates sprite
**Called by:** Game::ExecuteItem helmet case (lines 4419, 4428)
**Calls:** GetSprite
**Globals read:** None (uses this->req, this->clr)
**Globals mutated:** this->req.helmet, this->sprite
**Side effects:** Changes sprite pointer
**Notes:** Early return if already equipped (lines 5043-5044). Attempts to change req.helmet to h (line 5047), calls GetSprite (line 5049), reverts if null (lines 5050-5053). Returns true on success (line 5057), false if sprite unavailable. Identical structure to SetShield.

---

### `Human::SetArmor` (game.cpp:5060-5077)

**Signature:** `bool Human::SetArmor(int a)`
**Purpose:** Equips armor, updates sprite
**Called by:** Game::ExecuteItem armor case (lines 4448, 4457)
**Calls:** GetSprite
**Globals read:** None (uses this->req, this->clr)
**Globals mutated:** this->req.armor, this->sprite
**Side effects:** Changes sprite pointer
**Notes:** Early return if already equipped (lines 5062-5063). Attempts to change req.armor to a (line 5066), calls GetSprite (line 5068), reverts if null (lines 5069-5072). Returns true on success (line 5076), false if sprite unavailable. Identical structure to SetShield/SetHelmet.

---

### `Human::SetMount` (game.cpp:5079-5098)

**Signature:** `bool Human::SetMount(int m)`
**Purpose:** Mounts/dismounts creature (wolf/bee/none), updates sprite
**Called by:** Game::ExecuteItem weapon/shield/helmet/armor cases (lines 4369, 4398, 4427, 4456), death handlers (lines 6303, 6533), potion consumption (lines 7196, 7202, 7208)
**Calls:** GetSprite
**Globals read:** None (uses this->req)
**Globals mutated:** this->req.mount, this->sprite
**Side effects:** Changes sprite pointer
**Notes:** Blocks if not ACTION::NONE (lines 5081-5082). Early return if already mounted (lines 5084-5085). Attempts to change req.mount to m (line 5087), calls GetSprite (line 5089), reverts if null (lines 5090-5093). Returns true on success (line 5097), false if sprite unavailable or blocked. Equipment items (weapon/shield/helmet/armor) force dismount before equip (see ExecuteItem).

---

### `Human::Say` (game.cpp:5100-5159)

**Signature:** `void Human::Say(const char* str, int len, uint64_t stamp)`
**Purpose:** Posts a chat message above character's head, sends to server if connected
**Called by:** No callers found via grep
**Calls:** TalkBox::Reflow, akAPI_Exec, server->Send, ChatLog, malloc, free, memmove, memcpy, memset
**Globals read:** server
**Globals mutated:** this->talks, this->talk[], server (sends message)
**Side effects:** Allocates TalkBox (or reuses oldest if limit reached), sends network message, writes to ChatLog
**Notes:** Limit of 3 active talk boxes per human (lines 5106-5112). If at limit, reuses oldest talk box (line 5108, memmove to shift array). Allocates new TalkBox if under limit (line 5114). Copies message (up to 256 bytes) (lines 5116-5120), calls TalkBox::Reflow to wrap text (line 5125), clamps size to 7 rows (line 5127). Hacker mode: if message starts with '\\' but not '\\\\', calls akAPI_Exec instead of posting (lines 5131-5137). Otherwise, stores talk box (lines 5140-5145), sends STRUCT_REQ_TALK to server if connected (lines 5147-5154), writes to ChatLog (line 5156), increments talks (line 5157). Talk boxes rendered in Game::Render (lines 7046-7076), fade out after 30 dy units (calculated from elapsed time, line 5057).

---

### `DrawMiniText` (game.cpp:5161-5184) [STATIC]

**Signature:** `static void DrawMiniText(AnsiCell* ptr, int width, int height, int x, int y, const char* text, uint8_t fg, uint8_t bk, int max_w)`
**Purpose:** Draws single line of text at cell coords with fg/bk color, clipped to max_w
**Called by:** RenderMinimap (lines 5211-5212), Game::Render info display (lines 7670-7671)
**Calls:** None
**Globals read:** None
**Globals mutated:** ptr (writes AnsiCell data)
**Side effects:** Draws text to AnsiCell buffer
**Notes:** Bounds check y (lines 5164-5165), x (lines 5167-5168), calculates limit = x + max_w clamped to width (lines 5170-5172). Loops text chars (lines 5174-5183), breaks on newline (lines 5177-5178), writes glyph/fg/bk to cell (lines 5179-5182). Used for minimap labels and debug overlays.

---

### `RenderMinimap` (game.cpp:5187-5394) [STATIC]

**Signature:** `static void RenderMinimap(AnsiCell* ptr, int width, int height, float player_x, float player_y, float player_z, float player_dir, float yaw, float zoom, Character* player_head, Terrain* terrain)`
**Purpose:** Renders top-right minimap with terrain heightmap, NPCs, player position/direction
**Called by:** Game::Render (line 6690)
**Calls:** DrawMiniText, GetTerrainPatch, GetTerrainHeightMap, GetTerrainVisualMap, floor, fmod, snprintf
**Globals read:** HEIGHT_CELLS, VISUAL_CELLS
**Globals mutated:** ptr (writes AnsiCell data)
**Side effects:** Draws minimap to AnsiCell buffer
**Notes:** Constants: MAP_W=32, MAP_H=16, MAP_X=width-MAP_W-1 (top-right), MAP_Y=1+MAP_INFO_LINES, SCALE=16.0 (world units per cell). Early exit if screen too small (lines 5200-5201). Draws 2 info lines above map with pos/yaw/dir/zoom (lines 5204-5213). Draws minimap background (lines 5216-5299): for each cell, calculate world pos (lines 5228-5229), get terrain patch (line 5235), sample heightmap and visual map (lines 5240-5255), color by height and material (lines 5258-5288): water (<0x8000 or mat==0) → dark blue '~', grass (mat==1) → green shades '.', other → gray shades ':'. Draws NPCs (lines 5301-5325): loop player_head, project to minimap coords (lines 5305-5308), draw enemy as red '*' (lines 5313-5316), buddy/player as green 'o' (lines 5318-5322). Draws player at center as white '@' (lines 5328-5337), draws direction arrow 2 cells ahead (lines 5339-5355), chooses arrow glyph based on direction (lines 5350-5353). Draws border (lines 5357-5393): horizontal lines '-' (lines 5358-5375), vertical lines '|' (lines 5376-5393). Only rendered when !show_inventory && !main_menu (line 6688).

---

### `Game::Render` (game.cpp:5404-7873)

**Signature:** `void Game::Render(uint64_t _stamp, AnsiCell* ptr, int width, int height)`
**Purpose:** Main game loop — processes input, updates physics, animates characters, handles combat, renders frame, draws UI
**Called by:** Platform layer (game_web.cpp, term.cpp, game_app.cpp) — called once per frame
**Calls:** Hundreds of functions (see detailed breakdown below)
**Globals read:** Numerous (terrain, world, server, player_head, weather, inventory_sprite, character_button, item_proto_lib, consume_anim, etc.)
**Globals mutated:** Numerous (weather, stamp, TalkBox_blink, KeybAutoRepChar, PressKey, consume_anims, etc.)
**Side effects:** File I/O (test mode stdin, screenshot save), stdout printf (test mode, weather debug), network sends, world modifications (terrain painting, sprite instances), memory allocation (consume anims, talk boxes)
**Notes:** This is the CORE game loop, ~2469 lines. I'll break it into subsections:

#### Test Harness (lines 5406-5541)
- Checks env var ASCIICKER_TEST_MODE (line 5411)
- Sets stdin non-blocking (lines 5414-5416)
- Prints TEST_MODE_ACTIVE (line 5418)
- Forces main_menu=false (line 5422)
- Force-inits game if physics missing (lines 5425-5478): loads a3d from hardcoded path (line 5430), creates terrain/world, calls InitGame (line 5476)
- Reads stdin commands (lines 5483-5531): MOVE_FORWARD sets A3D_W key bit (line 5499), STOP clears key/impulse (lines 5502-5506), TELEPORT sets player pos and calls SetPhysicsPos (lines 5508-5522)
- Prints state every 10 frames (lines 5534-5539): pos, action, W_DOWN, stamp, physics pointer

#### Weather System Init (lines 5543-5549)
- Static weather_init guard (line 5544)
- Calls CreateWeather() once (line 5547)
- Sets weather_init=true (line 5548)

#### FPS Tracking (lines 5551-5559)
- Updates server->stamp if server connected (lines 5551-5552)
- Calculates FPS from rolling window (lines 5554-5559)

#### Lag Prevention (lines 5561-5562)
- If lag > 0.5s, reset stamp to prevent physics time jumps (line 5562)

#### Input Auto-Repeat (lines 5564-5600)
- Gamepad dirpad auto-repeat (lines 5565-5572): 20Hz repeat rate
- Keyboard key auto-repeat release (lines 5574-5585): releases PressKey after 50ms
- Keyboard char auto-repeat (lines 5590-5599): half sec delay, then 30ms per char

#### Render Size Update (lines 5602-5610)
- Updates keyb_hide if size changed (lines 5602-5607)
- Sets render_size[0..1] = width, height (lines 5609-5610)

#### Main Menu Early Return (lines 5612-5616)
- If main_menu, calls MainMenu_Render and returns (no game logic)

#### Light Normalization (lines 5618-5625)
- Normalizes light direction vector (lines 5618-5625)

#### Physics Force Accumulation (lines 5628-5794)
**WHY**: Game fuses multiple input sources (keyboard, gamepad, touch, API) into single PhysicsIO struct before physics integration.
- Initializes PhysicsIO io (lines 5634-5641): sets impulse from player.impulse, zeros forces, sets water level, jump=false
- Touch/Mouse Force Handling (lines 5647-5716): Contact::FORCE calculates x_force, y_force from touch pos relative to screen center (contact 0) or drag delta (other contacts), normalizes to magnitude 1 (lines 5668-5673)
- Touch/Mouse Torque Handling (lines 5678-5716): Contact::TORQUE calculates yaw from mouse drag (contact 0, absolute mode), or accumulates from touch margin (other contacts, relative mode), sets physics yaw (line 5696)
- Keyboard Force (lines 5721-5745): WASD/arrows → x_force, y_force (lines 5731-5738), shift halves speed (lines 5726-5727), gamepad left stick adds to force (lines 5751-5753), normalizes if len > 1 (lines 5740-5744)
- Keyboard Torque (lines 5756-5780): Q/E/Delete/Insert/PageUp/PageDown → io.torque (lines 5758-5759), fly mode: 2/X keys → z_force (lines 5761-5776)
- Keyboard Jump (lines 5791): input.jump → io.jump
- API Move Override (lines 5793-5794): input.api_move[] overrides x_force, y_force if api_move[2] > 0
- Force Zeroing by Action State (lines 5796-5811): FALL/STAND/DEAD block all forces and jump (lines 5796-5803), ATTACK with crossbow blocks forces/jump (lines 5805-5811)
- Bee Mount Speed Limiter (lines 5816-5825): If grounded and mounted on bee, clamps force magnitude to 0.5

#### Player Physics Integration (lines 5827-5860)
**WHY**: Animate() integrates forces, applies gravity (reduced by water), sweeps terrain collisions, updates position.
- Test mode debug print every 10 frames (lines 5827-5833)
- Calls Animate(physics, _stamp, &io, &player.req, true) (line 5842), returns steps (number of substeps)
- If grounded and blood enabled, calls BloodLeak(&player, steps) (lines 5849-5850)
- Stores prev_grounded = io.grounded (line 5852)
- Stores player.impulse from io (lines 5854-5855)
- Clears input.jump if steps > 0 (lines 5857-5860)

#### NPC Physics and AI (lines 5862-6427)
**WHY**: Each NPC (buddy or enemy) runs pathfinding AI, then calls Animate() with AI-calculated forces.
- Loops player_head (lines 5865-6427)
- Skips if h->data == physics (player) (line 5869)
- Initializes NPC PhysicsIO (lines 5872-5880), clears target (lines 5882-5884)
- **AI Pathfinding (lines 5886-6065)**:
  - Skips if DEAD or FALL (line 5886)
  - Finds closest enemy (lines 5908-5957): loops player_head, checks enemy flag, calculates distance, prioritizes if recently shot by (lines 5923-5928), weighs by followers (line 5930)
  - Finds closest buddy (lines 5937-5954): for collision avoidance
  - Sets target (lines 5959-5988): prefers enemy if in range and close to master, else master
  - Calculates move force toward target (lines 5991-6053): if dist > min_target_dist, set pio.x_force, pio.y_force toward target (lines 6005-6016), slide around buddy if collision (lines 6018-6053)
  - Triggers attack if close to enemy target (lines 6055-6063)
  - Unstick logic (lines 6071-6110): if stuck, set jump flag (lines 6071-6075), reverse force (lines 6077-6082), go around (lines 6084-6098), keep jumping (lines 6101-6105), reset after 400 ticks (lines 6107-6109)
- Calls Animate(p, _stamp, &pio, &h->req, false) (line 6125)
- Calls BloodLeak(h, s) if grounded (lines 6128-6129)
- **Unstuck Position Restore (lines 6131-6189)**: tracks last 2 good positions (lines 6133-6142), if stuck threshold crossed, teleports to last good pos (lines 6145-6152), increments stuck counter if no progress (lines 6154-6189)
- **Combat Animation (lines 6211-6406)**: Action switch (line 6241):
  - **ACTION::ATTACK / SWORD (lines 6247-6329)**: frame lookup table with swoosh frames (line 6252), calculates frame_index from action_stamp (line 6253), hit test at frame 21 (lines 6259-6318): finds target, checks distance < 3, deals rand() % 100 damage (line 6278), applies knockback impulse (lines 6295-6297), paints blood terrain decal (lines 6280-6293), triggers FALL if HP <= 0 (lines 6299-6314), dismounts if on mount (lines 6301-6304), else sets dir to face away and calls SetActionFall (lines 6308-6313). Sets h->frame from lookup table (line 6328), calls SetActionNone at end (line 6326).
  - **ACTION::ATTACK / CROSSBOW (lines 6332-6353)**: simple delay, no hit test yet (comment "here we should release arrow" line 6347), calls SetActionNone after 10 frames (line 6351).
  - **ACTION::FALL (lines 6359-6368)**: plays animation backward (line 6367), calls SetActionDead at end (line 6365).
  - **ACTION::STAND (lines 6371-6380)**: plays animation forward (line 6379), calls SetActionNone at end (line 6377).
  - **ACTION::DEAD (lines 6383-6386)**: nutting (no-op).
  - **ACTION::NONE (lines 6389-6405)**: idle frame 0 if player_stp < 0, else walking anim (lines 6395-6402).
- Force dir toward target if attacking (lines 6408-6413)
- Update h->pos from pio.pos (lines 6416-6419)
- Call UpdateSpriteInst to update world instance (line 6423)

#### Player Combat Animation (lines 6430-6639)
**WHY**: Player combat uses same animation system as NPCs, but with player-specific target finding.
- Stores prev_yaw (line 6430)
- Updates player.pos from io (lines 6432-6434)
- Action switch (line 6436):
  - **ACTION::ATTACK / SWORD (lines 6442-6554)**: Same as NPC sword (lines 6447-6553), but player target finding uses 4x4 range instead of 3x3 (line 6470), checks direction within 90deg cone (lines 6472-6487).
  - **ACTION::ATTACK / CROSSBOW (lines 6557-6581)**: Same as NPC crossbow.
  - **ACTION::FALL (lines 6587-6598)**: Same as NPC fall.
  - **ACTION::STAND (lines 6601-6612)**: Same as NPC stand.
  - **ACTION::DEAD (lines 6615-6618)**: nutting.
  - **ACTION::NONE (lines 6621-6637)**: Same as NPC none.
- Update player.dir from io (line 6642)
- Call UpdateSpriteInst for player_inst (line 6646)

#### Inventory Scene Shift Animation (lines 6648-6662)
- If show_inventory, slide scene_shift toward inventory_width (39 cells) (lines 6650-6654)
- If !show_inventory, slide scene_shift toward 0 (lines 6657-6661)

#### Weather Update (lines 6664-6684)
- Call UpdateWeather(_stamp, player.pos[0], player.pos[1]) (line 6668)
- If terrain, call UpdateSnowAccumulation(weather, terrain, _stamp) (line 6671)
- Static weather_log debug print every 120 frames (lines 6680-6683)

#### Main Render Call (lines 6675-6676)
- Call ::Render(renderer, ...) to render 3D world to AnsiCell buffer (line 6675)

#### Snow Particle Compositing (lines 6678-6684)
- If weather intensity > 0, call CompositeSnowParticles(weather, ptr, ...) (line 6684)

#### Minimap Overlay (lines 6687-6692)
- If !show_inventory && !main_menu, call RenderMinimap(...) (line 6690)

#### Player Crossbow Shoot (lines 6694-6953)
**WHY**: Crossbow uses raycast-based targeting, not melee distance check.
- If input.shoot && weapon == CROSSBOW && cooldown elapsed (line 6694-6696):
- Find closest enemy in 60deg cone, distance 2-30, slope < 8 (lines 6703-6742)
- Call player.SetActionAttack(_stamp) (line 6744)
- Set physics dir to target angle (line 6748)
- Unproject sprite meta_xy to world coords (lines 6752-6765), call UnprojectCoords3D (line 6765)
- Calculate shoot_to (lines 6767-6778): target pos if found, else 1000 units ahead
- Raycast HitWorld and HitTerrain (lines 6780-6832): find closest obstacle
- Hide player_inst during raycast (lines 6797, 6834)
- If obstacle closer than target, adjust shoot_to (lines 6821-6831)
- Set player.shoot_stamp, player.shooting=true, player.shoot_target (lines 6837-6852)
- Clear input.shoot (lines 6955-6956)

#### Story API Frame Callback (line 6959)
- Call akAPI_OnFrame() (line 6959)

#### Nearby Items List (line 6961)
- Call GetNearbyItems(renderer) (line 6961), returns Item** array

#### Status Bar (lines 6969-7013)
**WHY**: Shows online/offline status and FPS at top of screen.
- Formats status text: "ON LINE 1234 | 12.3 fps" or "OFF LINE | 12.3 fps" (lines 6975-6995)
- Paints full-width status bar at top row (lines 6996-7012)

#### Talk Box Rendering (lines 7030-7082)
**WHY**: Speech bubbles above characters' heads.
- Loops player_head (lines 7043-7082)
- For each human (line 7046), loops human->talks (lines 7053-7075)
- Calculates vertical offset from stamp (lines 7055-7057), dy = elaps / speed (line 7057)
- If dy <= 30, project pos to screen (line 7062), call TalkBox::Paint (line 7063)
- If dy > 30, free talk box and remove from array (lines 7066-7074)

#### Inventory Layout Update (line 7087)
- Call inventory.UpdateLayout(width, height, scene_shift, bars_pos) (line 7087)

#### Inventory UI Rendering (lines 7089-7347)
**WHY**: Draws inventory grid, item sprites, scroll indicators, focus rect.
- If scene_shift > 0 (inventory partially visible):
- Blit inventory_sprite frame sections (lines 7091-7115): header, body tiles, footer
- Animate scroll to focused item (lines 7117-7139)
- Smooth scroll interpolation (lines 7141-7166)
- CheckDrop for all contacts (lines 7180-7184): paints yellow hilight if valid drop
- Render consume animations (lines 7186-7238): items shrinking after consumption, triggers mount changes via hardcoded sprite checks (lines 7193-7216)
- Render inventory items (lines 7240-7300): loop my_items, blit sprite at xy position, skip if in_contact (defer to contact rendering), paint focus rect for focused item (lines 7288-7299)
- Paint focus rect and item desc (lines 7302-7317)
- Paint scroll indicators (lines 7319-7346): horizontal line at clip boundary if scrolled

#### Item Pickup List (lines 7352-7596)
**WHY**: Shows items on ground that player can pick up.
- If !player.talk_box (not typing):
- Calculate max items that fit on screen (lines 7354-7366)
- Store items_count, items_inrange (lines 7370-7371)
- Clamp input.pad_item (line 7374)
- Paint pickup list border (lines 7384-7410)
- Loop items (lines 7412-7541): paint item sprite, check if in_contact (defer if dragging), auto-pickup on '1'-'N' key (lines 7457-7466), blit sprite (line 7468), paint border segments (lines 7476-7540)
- Paint pad_item hilight rect (lines 7547-7596)
- Else (talking): set items_count=0 (lines 7598-7601)

#### Contact Item Rendering (lines 7604-7634)
**WHY**: Draws dragged items attached to mouse/touch cursor.
- Loop contact_items (lines 7604-7634)
- Get contact pos, convert to cell coords (lines 7607-7608)
- Offset by frame size (lines 7612-7615)
- Trap in_use items inside inventory (lines 7619-7631)
- Blit sprite (line 7633)

#### HP/MP Bars (lines 7636-7676)
**WHY**: Shows player health/mana at bottom of screen.
- Calculate bar widths (lines 7639-7650)
- Paint HP bar with player.HP / player.MAX_HP (line 7659)
- Paint MP bar with 1.0 (placeholder, line 7660)
- If enough space, paint debug info (lines 7662-7672): pos, yaw, dir, zoom
- Blit character buttons (lines 7674-7675)

#### Player Talk Box (lines 7678-7679)
- If player.talk_box, call Paint with blink cursor (line 7679)

#### Button Animation (lines 7681-7684)
- Animate bars_pos toward 7 if show_buts, toward 0 if !show_buts (lines 7681-7684)

#### Keyboard Rendering (lines 7686-7747)
- If show_keyb or animating, call keyb.Paint(...) (line 7724)
- Animate keyb_hide (lines 7727-7747): slide down if show_keyb, slide up if !show_keyb

#### Input Cleanup (line 7749)
- Clear input.last_hit_char (line 7749)

#### Update stamp (line 7751)
- Set stamp = _stamp (line 7751)

#### Network Lag Ping (lines 7753-7768)
- If server && not lag_wait && 100ms elapsed, send STRUCT_REQ_LAG (lines 7755-7767)

#### Network Pose Broadcast (lines 7770-7788)
- If server && steps > 0, send STRUCT_REQ_POSE with player state (lines 7770-7788)

#### Gamepad Debug (line 7791)
- If show_gamepad, call PaintGamePad(...) (line 7792)

#### Menu Rendering (line 7794)
- Call PaintMenu(...) (line 7794)

#### Camera Overlay (lines 7800-7834)
- If show_cam_overlay && menu_depth < 0, format and paint debug text (lines 7800-7833): pos, yaw, zoom, light, water, map name

#### Screenshot (lines 7836-7872)
**WHY**: Saves frame buffer to shot.xp file.
- If input.shot, write shot.xp in REXPaint format (lines 7839-7869), column-major order, call WriteShotJson (line 7870)

---

### `Game::OnSize` (game.cpp:7875-7886)

**Signature:** `void Game::OnSize(int w, int h, int fw, int fh)`
**Purpose:** Called when window/terminal resizes, resets input state, updates size fields
**Called by:** Platform layer (SDL/browser/terminal resize handlers)
**Calls:** memset, MainMenu_OnSize
**Globals read:** None
**Globals mutated:** this->input (zeroed except pad_connected), this->font_size[0..1]
**Side effects:** Clears all input state (contacts, keys, etc.)
**Notes:** Preserves input.pad_connected (lines 7877-7879), sets input.size[0..1] = w, h (lines 7880-7881), sets font_size[0..1] = fw, fh (lines 7882-7883). Calls MainMenu_OnSize (line 7885) to update main menu layout. Clearing input prevents stale contact/key state after resize.

---

### `Game::OnKeyb` (game.cpp:7924-8000+)

**Signature:** `void Game::OnKeyb(GAME_KEYB keyb, int key)` [PARTIAL — continues beyond line 8000]
**Purpose:** Processes keyboard events (KEYB_DOWN, KEYB_UP, KEYB_CHAR, KEYB_PRESS), dispatches to active UI layer
**Called by:** Platform layer (SDL/browser/terminal keyboard handlers)
**Calls:** GetWeather, SetWeather, MainMenu_OnKeyb, MenuKeyb, gamepad key mapping
**Globals read:** weather, main_menu, menu_depth, show_gamepad
**Globals mutated:** show_cam_overlay, input.shot, main_menu state, menu state, gamepad config state
**Side effects:** Triggers weather change (F3/tilde), screenshot (F10), camera overlay toggle (F9)
**Notes:**
- Weather cycling (lines 7936-7945): F3 or tilde (when not auto-repeat) cycles weather 0→1→2→3→0, prints to stdout
- Camera overlay toggle (lines 7946-7947): F9 toggles show_cam_overlay
- Screenshot trigger (lines 7948-7949): F10 sets input.shot=true
- Layered dispatch (lines 7952-8000+):
  - If main_menu, call MainMenu_OnKeyb and return (lines 7957-7961)
  - If menu_depth >= 0, call MenuKeyb and return (lines 7963-7967)
  - If show_gamepad, process gamepad config keys (lines 7969-8000+): CHAR maps space→0, enter→1, esc→2, printable→key (lines 7974-7987), PRESS/DOWN maps enter→1, esc→2, arrows→3/4/5/6 (lines 7991-8000, continues beyond 8000)

**:** The OnKeyb function continues beyond line 8000. The remainder  handles in-game key mappings (movement, attack, inventory, etc.), but those details are outside this analysis scope.

---

## SUMMARY STATISTICS

**Functions documented:** 27 complete functions + 1 partial (OnKeyb)
**Static functions:** 2 (DrawMiniText, RenderMinimap)
**Member functions:** 24 (Game: 11, Character: 5, Human: 6, Inventory: 0 documented here but called)
**Global variables accessed:** 27+
**Struct definitions:** 1 (ConsumeAnim, inferred)
**Enum definitions:** 5 categories (ACTION, WEAPON, MOUNT, ARMOR, Input::Contact)
**Line count analyzed:** 4000 lines (4001-8000)

**Largest function:** Game::Render (2469 lines, 5404-7873)
**Most complex function:** Game::Render (NPC AI pathfinding, combat, inventory, networking, UI)
**Most called function:** GetSprite (called by all SetAction* and SetEquipment* methods)

**Combat system summary:**
- Distance-based hit detection (sphere test, not ray test)
- One-time hit test per attack (hit_tested flag)
- Random damage 0-99 (no equipment modifiers)
- Knockback impulse based on distance
- Death triggers dismount or FALL action
- Melee range 3 units (player 4), crossbow range 2-30 with 60deg cone

**AI system summary:**
- Target selection: closest enemy (weighted by followers) or master
- Pathfinding: direct line to target with buddy collision avoidance
- Unstuck logic: reverse, go around, jump, teleport after 400 ticks
- Attack range: melee 3 units, archer 10 units
- Aggro: recent shoot_by targets get 0.2x distance weight for 5 seconds

**Inventory system summary:**
- 2D grid with bitmask occupancy (1 bit per cell)
- Bottom-up, left-to-right auto-packing
- Drag-drop with collision detection
- Equipment items force dismount
- Consume animations (16 max) trigger mount changes via hardcoded sprite checks
- Story API hooks for pick/drop/equip/unequip

**Test harness summary:**
- Activated by ASCIICKER_TEST_MODE env var
- Accepts stdin commands: MOVE_FORWARD, STOP, TELEPORT x y z
- Force-inits game with hardcoded a3d path
- Prints state every 10 frames
- Non-blocking stdin (fcntl O_NONBLOCK on Unix)

**Weather integration:**
- CreateWeather() called once on first frame
- UpdateWeather() called per frame with player pos
- UpdateSnowAccumulation() updates terrain if weather active
- CompositeSnowParticles() overlays particles on frame buffer
- F3/tilde cycles weather modes

**Minimap system:**
- 32x16 cells, top-right corner
- 16 world units per cell (zoomed out)
- Samples terrain heightmap and visual map
- Color codes: water (blue), grass (green), other (gray)
- Draws NPCs (red=enemy, green=buddy), player (white '@'), direction arrow
- 2 info lines above map with pos/yaw/dir/zoom

**Key findings:**
1. **No ray-based hit detection in melee combat** — uses distance sphere test only. Comment at line 6225 acknowledges this limitation.
2. **Hardcoded item sprite checks for mount changes** — lines 7193-7216 use item_proto_lib[38/39/40].sprite_2d pointer comparison instead of item kind field. Brittle, breaks if item order changes.
3. **No multiplayer reconciliation** — client sends pose updates, no server correction. Lag compensation missing.
4. **Fixed 16-entry consume animation limit** — line 4331 shifts array if full, oldest animation discarded. Could cause visual glitches if many items consumed rapidly.
5. **Test harness hardcoded path** — line 5430 hardcodes "a3d/game_map_y8.a3d", not configurable. Safety concern (TODO comment at 5430).
6. **Attack damage has no equipment modifiers** — rand() % 100 at lines 6278, 6507. Comment at line 6229 acknowledges TODO for base damage from weapon, armor reduction, critical hits.
7. **NPC unstuck teleport** — lines 6145-6152 teleport stuck NPC to last good position after 100 ticks. No collision check, could teleport into walls.
8. **Weather debug prints to stdout** — lines 6680-6683, 7943. Should use ChatLog instead for production builds.

## CALLER ANALYSIS (sampled, not exhaustive due to scope)

**CreateGame called by:**
- game_web.cpp:772 (web platform init)
- term.cpp:426 (terminal platform init)
- game_app.cpp:2484 (SDL platform init)

**InitGame called by:**
- Game::Render test harness (line 5476, force-init when ASCIICKER_TEST_MODE active)
- MainMenu "Play" button handler (not in this range, earlier in file)

**Game::Render called by:**
- Platform layer frame loops (game_web.cpp, term.cpp, game_app.cpp) — NOT directly called within game.cpp

**SetActionNone called by:**
- Game::Render attack animation end (lines 6326, 6351, 6551, 6579)
- Game::Render TELEPORT command (line 5514)

**SetActionAttack called by:**
- Game::Render NPC AI (line 6062)
- Game::Render crossbow shoot (line 6744)
- User input handlers (not in this range)

**SetActionFall called by:**
- Game::Render death handlers (lines 6313, 6365, 6543, 6595)

**SetActionDead called by:**
- Game::Render fall animation end (lines 6365, 6595)

**ExecuteItem called by:**
- User input handlers (not in this range,  OnKeyb inventory activation)

**PickItem called by:**
- Game::Render item pickup list (line 7462)

**DropItem called by:**
- CheckDrop drag-drop completion handler (not in this range)

**Human::Say called by:**
- User chat input handlers (not in this range)
- Network message handlers (not in this range)

**RenderMinimap called by:**
- Game::Render (line 6690, only if !show_inventory && !main_menu)

**BloodLeak called by:**
- Game::Render player physics (line 5850)
- Game::Render NPC physics (lines 6129, 6205)

**Animate called by:**
- Game::Render player physics (line 5842)
- Game::Render NPC physics (lines 6125, 6202)

**GetSprite called by:**
- All SetAction* methods (lines 4860, 4886, 4921, 4955, 4986)
- All SetEquipment* methods (lines 5011, 5030, 5049, 5068, 5089)

**NOTES ON MISSING CALLERS:**
Several functions have no callers found in this codebase:
- DeleteGame —  called by platform shutdown, not in game.cpp
- FreeGame —  called by world reload or shutdown
- CancelItemContacts —  called when inventory closed
- DropItem —  called from CheckDrop completion handler (not in this range)

These are  called from platform layer (game_web.cpp, game_app.cpp, term.cpp) or from input handlers later in game.cpp (beyond line 8000).
