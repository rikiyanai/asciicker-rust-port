# game.cpp Function Analysis (Lines 8001-12077)

This document provides comprehensive function analysis for the input handling, contact/touch management, menu system, and peripheral subsystems in game.cpp.

## Input & Contact Management Functions

### `Game::StartContact` (game.cpp:8773-9351)

**Signature:** `void Game::StartContact(int id, int x, int y, int b)`

**Purpose:** Initialize a new input contact (mouse button, touch, or keyboard key press) and determine its action type (inventory interaction, camera torque, terrain force, item pickup, keyboard cap hit).

**Called by:**
- `Game::OnMouse` (game.cpp:9907, 9921, 9948)
- `Game::OnTouch` (game.cpp:10019)

**Calls:**
- `ScreenToCell()` — converts screen pixel coordinates to game world cell coordinates
- `CheckPick()` — detects which inventory item was clicked
- `PlayerHit()` — tests if click/touch intersects player sprite hitbox
- `keyb.GetCap()` — identifies which keyboard virtual key was pressed
- `Buzz()` — plays audio feedback
- `CancelItemContacts()` — cancels active item-related contacts
- `inventory.SetFocus()` — highlights focused inventory item
- `ExecuteItem()` — uses or consumes clicked item

**Globals read:**
- `show_inventory` — whether inventory panel is visible
- `player.talk_box` — active text input box (if any)
- `input.contact[4]` — array of 4 active input contacts
- `inventory.*` — inventory state (layout, items, scroll, bitmask)
- `items_count`, `items_inrange[]`, `items_xarr[]`, `items_ylo`, `items_yhi` — on-screen item list state
- `show_keyb` — whether virtual keyboard is visible
- `keyb.*` — keyboard UI state
- `render_size[2]`, `scene_shift`, `font_size[2]` — rendering metrics
- `server` — network server pointer (null in offline)
- `stamp` — current game timestamp

**Globals mutated:**
- `input.contact[id]` — sets action type, position, item, capability, margin, yaw state
- `inventory.focus`, `inventory.animate_scroll` — updates focused item and scroll animation
- `con->action` — touches/clicks set to NONE, ITEM_GRID_CLICK, ITEM_GRID_SCROLL, ITEM_LIST_CLICK, KEYBCAP, TORQUE, FORCE, or PLAYER
- `keyb_key[32]` — tracks which virtual keys are visually highlighted
- `KeybAutoRepCap`, `KeybAutoRepChar`, `KeybAuroRepDelayStamp` — autorepeat state for held keys

**Side effects:**
- Allocates/deallocates no memory; all state is in pre-allocated arrays
- May trigger sound effects (Buzz)
- May modify keyboard plane (shift key cycling)
- Syncs inventory state to network if `server` is connected

**Notes:**
Contact assignment is precedence-based: keyboard caps > item grid > item list > player sprite > terrain force. Once a contact action is assigned, it prevents other contacts from the same source (mouse/touch id) from claiming the same item/key. The function uses bitwise operations to track which virtual keys are "highlighted" on-screen (keyb_key array) and manages autorepeat state for terminal-style key input.

---

### `Game::MoveContact` (game.cpp:9353-9487)

**Signature:** `void Game::MoveContact(int id, int x, int y)`

**Purpose:** Update an ongoing input contact position and transition between actions (e.g., CLICK → DRAG when moved >2 cells, or update scroll offset during inventory scroll).

**Called by:**
- `Game::OnMouse` (game.cpp:9909, 9935, 9950, 9962)
- `Game::OnTouch` (game.cpp:10024)

**Calls:**
- `ScreenToCell()` — converts new screen position to cell coordinates
- `keyb.GetCap()` — re-tests which keyboard cap is under cursor
- `inventory.FocusNext()` — navigates inventory focus during ITEM_GRID_CLICK

**Globals read:**
- `input.contact[id]` — the contact to move
- `inventory.scroll`, `inventory.layout_max_scroll` — inventory scroll bounds
- `inventory_sprite->atlas` — sprite frame data (for bounds checks)
- `render_size[2]` — screen dimensions

**Globals mutated:**
- `input.contact[id].pos[2]` — updated to new x, y
- `input.contact[id].action` — may transition ITEM_GRID_CLICK → ITEM_GRID_DRAG, or ITEM_LIST_CLICK → ITEM_LIST_DRAG, or PLAYER → FORCE, or KEYBCAP → NONE
- `inventory.scroll`, `inventory.smooth_scroll` — updated during ITEM_GRID_SCROLL action
- `keyb_key[32]` — may unhighlight keycap if cursor moved away

**Side effects:**
- No allocations, file I/O, or audio
- Pure state update based on motion threshold (2+ cells) and UI logic

**Notes:**
The function implements gesture detection: small motions keep CLICK state (intent to execute), but >2 cells of motion transitions to DRAG (intent to move/reorder items or change keycap). The keycap motion branch (KEYBCAP action) un-highlights the old keycap visually if the cursor drifts away.

---

### `Game::EndContact` (game.cpp:9489-9702)

**Signature:** `void Game::EndContact(int id, int x, int y)`

**Purpose:** Finalize an input contact (button release, touch lift, key release) and execute the resulting action (item pickup, inventory rearrangement, menu selection, or camera pitch/yaw finalization).

**Called by:**
- `Game::OnMouse` (game.cpp:9915, 9942, 9956)
- `Game::OnTouch` (game.cpp:10028)
- `Game::OnKeyb` (game.cpp:8076 — via character input forwarding)

**Calls:**
- `SetPhysicsYaw()` — finalizes camera yaw rotation from TORQUE contact
- `ExecuteItem()` — executes (uses/eats/drinks) an inventory item on right-click
- `CheckDrop()` — tests if dragged item can be placed at target location
- `DropItem()` — removes item from inventory (can only drop if not in-use)
- `PickItem()` — adds dropped item to inventory at target location
- `akAPI_OnItem()` — calls script hook for item pickup (allows/denies with callback)
- `inventory.InsertItem()` — places item in inventory at given coordinate
- `ScreenToCell()` — converts final position to cell coordinates
- `keyb_key[32]` manipulation — un-highlights virtual keys

**Globals read:**
- `input.contact[id]` — the contact to finalize
- `inventory.*` — inventory grid, item list, scroll state
- `show_inventory` — whether inventory is visible
- `items_inrange[]` — on-screen items available for pickup
- `player.talk_box` — chat input state
- `show_keyb` — virtual keyboard visibility
- `KeybAutoRepCap`, `KeybAutoRepChar` — autorepeat state to clear

**Globals mutated:**
- `input.contact[id].action` → Input::Contact::NONE
- `input.contact[id].drag` → 0
- `inventory.my_item[].xy[2]` — item position updated on successful drop
- `inventory.bitmask[*]` — recalculated for dropped item
- `player.talk_box` → null (may close if PLAYER contact clicked again)
- `show_keyb` → true/false (toggle on PLAYER contact)
- `keyb_key[32]` — un-highlighted for any KEYBCAP contact
- `physics`, `yaw_vel` — updated by SetPhysicsYaw

**Side effects:**
- May allocate/free `player.talk_box` (malloc/free)
- May send network packet (akAPI_OnItem call)
- May log chat message (ChatLog in inventory insert path)
- Plays sound effects (Buzz in some paths)

**Notes:**
EndContact is the key decision point for item drop vs. use. Right-click (drag==2) on ITEM_GRID_CLICK executes immediately. ITEM_GRID_DRAG calculates destination and updates inventory bitmask (clear old position, set new position). For ITEM_LIST_CLICK/DRAG, the function can pick items from the on-screen loot display and merge them into inventory. PLAYER contact with small motion (<1 cell) toggles chat; large motion turns it into FORCE (terrain click).

---

### `Game::GetContact` (game.cpp:9704-9708)

**Signature:** `int Game::GetContact(int id)`

**Purpose:** Query the drag button state of a contact (which mouse/touch button initiated it).

**Called by:**
- `Game::OnMouse` (game.cpp:9914, 9941, 9955, 9961)

**Calls:** None

**Globals read:**
- `input.contact[id].drag` — the button/touch state (0=no button, 1=left, 2=right, 3=middle)

**Globals mutated:** None

**Side effects:** None (pure query)

**Notes:**
Trivial getter that returns the drag field of a contact. Used to test if a specific button is held and to distinguish between button press types (left vs. right click).

---

## Mouse & Touch Input Functions

### `Game::OnMouse` (game.cpp:9735-9965)

**Signature:** `void Game::OnMouse(GAME_MOUSE mouse, int x, int y)`

**Purpose:** Route mouse events (button down/up, move, scroll) to contact handlers or menu/gamepad overlays. Dispatches to StartContact, MoveContact, or EndContact based on event type.

**Called by:**
- No direct callers found in grep ( called from platform layer: web, SDL, terminal)

**Calls:**
- `MainMenu_OnMouse()` — if main menu is open
- `MenuMouse()` — if in-game menu is open
- `GamePadContact()` — if gamepad UI is displayed
- `ScreenToCell()` — for inventory scroll detection
- `StartContact()` — on button down
- `MoveContact()` — on motion while contact active
- `EndContact()` — on button up

**Globals read:**
- `main_menu` — if true, dispatch to main menu handler
- `menu_depth` — if >=0, dispatch to menu handler
- `show_gamepad` — if true, dispatch to gamepad UI handler
- `input.but` — current multi-button state (bitmask)
- `input.contact[0]` — mouse contact (id=0)
- `scene_shift`, `render_size[2]` — for inventory scroll zone detection
- `inventory.*` — inventory state
- `stamp` — current timestamp

**Globals mutated:**
- `input.but` — updated with new button bitmask (bits 0, 1, 2 for L, R, M buttons)
- `inventory.smooth_scroll` — updated on mouse wheel

**Side effects:**
- May call menu handlers (which can modify game state)
- May output to screen (GamePadContact rendering)

**Notes:**
The function has conditional dispatch: if main_menu or menu_depth>=0 or show_gamepad, it delegates entirely to those subsystems. Otherwise, it handles player input. Button press is coalesced: if input.but != 0 already (multi-button scenario), it updates the bitmask without calling StartContact again. Mouse wheel events only affect inventory scroll (if visible on left side of screen). The function implements a simple deadzone for scroll detection (only if scene_shift is non-zero).

---

### `Game::OnTouch` (game.cpp:9967-10048)

**Signature:** `void Game::OnTouch(GAME_TOUCH touch, int id, int x, int y)`

**Purpose:** Route touch events (begin, move, end, cancel) to contact handlers or menu/gamepad overlays. Similar to OnMouse but for multi-touch (id=1,2,3) and without scroll wheel.

**Called by:**
- No direct callers found via grep (platform layer: web or mobile)

**Calls:**
- `MainMenu_OnTouch()` — if main menu open
- `MenuTouch()` — if in-game menu open
- `GamePadContact()` — if gamepad UI displayed
- `ScreenToCell()` — for coordinate conversion in gamepad path
- `StartContact()` — on TOUCH_BEGIN
- `MoveContact()` — on TOUCH_MOVE
- `EndContact()` — on TOUCH_END

**Globals read:**
- `main_menu`, `menu_depth`, `show_gamepad` — dispatch gates
- `input.contact[id]` — state for this touch
- `stamp` — current timestamp

**Globals mutated:**
- `input.contact[id].action` → Input::Contact::NONE (on TOUCH_CANCEL)
- `input.contact[id].drag` → 0 (on TOUCH_CANCEL)
- `keyb_key[*]` — un-highlighted if KEYBCAP contact cancelled

**Side effects:**
- May call menu handlers
- May call GamePadContact

**Notes:**
OnTouch validates id (1-3) before processing (id=0 is reserved for mouse). The TOUCH_CANCEL path handles cleanup for cancelled touches, including un-highlighting virtual keys if a keycap was active. The function does not handle scroll wheel (touch has no scroll analog).

---

## Game State & Input Lifecycle Functions

### `Game::OnFocus` (game.cpp:10050-10075)

**Signature:** `void Game::OnFocus(bool set)`

**Purpose:** Clear all input state when game loses focus (set=false) to prevent stuck keys, pending drags, or autorepeat after window/tab switch. Preserve pad connection state.

**Called by:**
- No direct callers found (platform layer)

**Calls:**
- `MainMenu_OnFocus()` — if main menu active

**Globals read:**
- `input` — current input state
- `main_menu` — if true, delegate to main menu focus handler

**Globals mutated:**
- `KeybAutoRepCap` → 0
- `KeybAutoRepChar` → 0
- `input.contact[0..3]` → {action=NONE, drag=0}
- `input.*` (except pad_connected) → memset to 0
- `input.size[]`, `input.pad_connected` — preserved

**Side effects:**
- Clears all keyboard repeat state
- Resets all contact states

**Notes:**
Critical for correctness: if player switches tabs while holding a key or dragging an item, OnFocus(false) ensures the dangling input doesn't persist on re-focus. The function preserves pad_connected and screen size because those are configuration state, not per-frame input.

---

### `Game::OnMessage` (game.cpp:10077-10081)

**Signature:** `void Game::OnMessage(const uint8_t* msg, int len)`

**Purpose:** Placeholder for future network message handling from JavaScript or network layer.

**Called by:** No callers found (stub)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (stub)

**Notes:**
Marked as NET_TODO: indicates this is a planned feature for receiving messages from the multiplayer server or web platform. Currently a no-op.

---

### `Game::OnPadMount` (game.cpp:10083-10104)

**Signature:** `void Game::OnPadMount(bool connect)`

**Purpose:** Initialize or reset gamepad input state when a physical gamepad connects or disconnects. Configure axis/button state and delegate to menu if menu is open.

**Called by:**
- `GamePadMount()` (game.cpp:10771 — gamepad driver)
- `GamePadUnmount()` (game.cpp:10779 — gamepad driver)

**Calls:**
- `MenuPadMount()` — if menu open
- `MainMenu_OnPadMount()` — if main menu open

**Globals read:**
- `input.pad_connected`, `stamp` — current gamepad state and time
- `menu_depth`, `main_menu` — dispatch gates

**Globals mutated:**
- `input.pad_connected` → connect
- `input.pad_button` → 0
- `input.pad_autorep` → 0
- `input.pad_item` → 0
- `input.pad_stamp` → stamp
- `input.pad_axis[0..31]` → memset to 0
- Calls `OnPadAxis(-1, 0)` to re-center all axes

**Side effects:**
- May call menu/main menu handlers
- May trigger menu/main menu focus changes

**Notes:**
The function resets all gamepad input to a clean state on connect. On disconnect, the same reset occurs. The call to OnPadAxis(-1, 0) with axis=-1 is a refresh signal (not a specific axis) that recalculates keyboard plane and sector based on analog stick position. If a menu is open, it receives OnPadMount notification (allowing it to update help text or UI).

---

### `Game::OnPadButton` (game.cpp:10106-10557)

**Signature:** `void Game::OnPadButton(int b, bool down)`

**Purpose:** Handle digital gamepad button presses/releases. Route button input to menus, inventory, chat, or gameplay actions (attack, jump, item use). Manage autorepeat for D-pad and delete key in chat.

**Called by:**
- `GamePadButton()` (game.cpp:10798 — gamepad driver, button events)
- `GamePadAxis()` (game.cpp:10827 — gamepad driver, axis-mapped-to-button)

**Calls:**
- `MainMenu_OnPadButton()` — if main menu open
- `MenuPadButton()` — if menu open
- `ExecuteItem()` — on button 2 (Y) release if item contact active
- `EndContact()` — finalize item contact on Y release
- `inventory.FocusNext()` — navigate inventory
- `PickItem()` — pick up item on button 3 (X) release
- `ToggleMenu()` — on button 6 (start)
- `CancelItemContacts()` — on button 9 (left shoulder) or button 10 (chat toggle)
- `Buzz()` — audio feedback on send (button 3 in chat)
- `player.talk_box->Input()`, `player.talk_box->MoveCursorX/Y()` — chat manipulation
- `akAPI_Exec()` — execute hacker mode commands (\ prefix in chat)
- `akAPI_OnSay()` — script hook for chat message
- `ChatLog()` — log chat message
- `OnKeyb()` — forward keypad cap input (KEYB_CHAR for virtual keyboard)

**Globals read:**
- `main_menu`, `menu_depth`, `show_gamepad` — dispatch gates
- `input.pad_autorep` — autorepeat state
- `player.talk_box` — chat input state
- `show_inventory` — inventory visibility
- `inventory.*` — inventory state
- `items_count`, `items_inrange[]` — on-screen items
- `stamp` — current timestamp
- `server` — network connection
- `keyb.*` — virtual keyboard state
- `player.*` — player state (position, talk history, etc.)

**Globals mutated:**
- `input.pad_button` — bitwise OR/AND for button state
- `input.pad_autorep` — set to b+1 on autorepeat buttons, 0 on release
- `input.pad_stamp` — timestamp of autorepeat start
- `input.pad_item` — selected item in loot list (0-indexed + 1)
- `input.jump`, `input.shoot` — set true on attack/jump buttons
- `player.talk_box` — allocated/freed/modified for chat
- `player.talks` — chat history counter
- `player.talk[]` — chat messages logged
- `show_inventory` — toggled
- `inventory.*` — item selection, focus, position

**Side effects:**
- May allocate player.talk_box (malloc)
- May free player.talk_box (free)
- May send network packet (server->Send on chat)
- May call script hooks (akAPI_OnSay, akAPI_Exec)
- May play audio (Buzz)
- May log to chat log (ChatLog)

**Notes:**
This is one of the most complex functions: it handles both gameplay input (buttons 0-3, 11-14) and chat input (buttons 2-3, 6, 10-14) with different behavior depending on whether player.talk_box is active. Button mapping: 0=A (attack), 1=B (jump), 2=Y (item/delete), 3=X (pickup/send), 5=guide, 6=start (menu), 9=LB, 10=RB (chat toggle). Autorepeat is set up for D-pad (11-14) and delete (2) in chat mode. The function also handles virtual keyboard interaction (sending character input through OnKeyb).

---

### `Game::OnPadAxis` (game.cpp:10559-10677)

**Signature:** `void Game::OnPadAxis(int a, int16_t pos)`

**Purpose:** Handle analog stick motion for keyboard plane/sector selection, camera direction in keyboard UI, and steer gamepad help display.

**Called by:**
- `GamePadAxis()` (game.cpp:10824 — gamepad driver)
- `OnPadMount()` (game.cpp:10092 — initialization)
- `OnPadButton()` (game.cpp:10556 — refresh after button)

**Calls:**
- `MainMenu_OnPadAxis()` — if main menu open
- `MenuPadAxis()` — if menu open (no-op stub)

**Globals read:**
- `main_menu`, `menu_depth`, `show_gamepad` — dispatch gates
- `show_keyb` — virtual keyboard visibility
- `input.pad_button[0..1]` — buttons to check for keyboard direction mode
- `input.pad_axis[0..31]` — current all axis positions
- `keyb.*` — keyboard UI state (plane, sector, dir, pad_plane flag)

**Globals mutated:**
- `input.pad_axis[a]` → pos (if a >= 0)
- `keyb.dir` — direction on keyboard (0-11)
- `keyb.sect` — section/plane of keyboard (0-2)
- `keyb.plane` — layer of keyboard (0-2)
- `keyb.pad_plane` — flag to indicate stick set the plane (allows reset to 0)

**Side effects:**
- Modifies virtual keyboard display (no rendering side effect, state only)

**Notes:**
The function has two modes: (1) if show_keyb && (input.pad_button & 3), the analog stick adjusts the keyboard cursor direction (8-way + center). (2) Otherwise, the stick selects keyboard plane (L/R analog motion) and row/direction on plane (vertical motion). The pad_plane flag prevents the user's mouse/touch selection from being overridden by stick motion; once stick moves vertically, pad_plane is true and vertical stick motion controls plane. If stick returns to center (deadzone), plane resets to 0 only if pad_plane is true.

---

## Menu System Functions

### `Game::OpenMenu` (game.cpp:11032-11071)

**Signature:** `void Game::OpenMenu(int method)`

**Purpose:** Transition game to menu state, clearing chat, inventory, contacts, and setting up menu navigation.

**Called by:**
- `Game::ToggleMenu()` (game.cpp:11089 — player toggle)
- `menu_gamepad()` (game.cpp:10957 — through menu action callback)
- `main_menu()` (game.cpp:10964 — through menu action callback)

**Calls:**
- `CancelItemContacts()` — clears pending item drags

**Globals read:**
- `menu_depth` — check if menu already open
- `player.talk_box` — to close if open
- `show_keyb`, `keyb_key[32]` — to hide keyboard UI
- `input.contact[4]` — to clear contact state

**Globals mutated:**
- `menu_depth` → 0 (open at depth 0)
- `menu_temp` → 0
- `menu_down` → 0
- `menu_down_x`, `menu_down_y` → 0
- `show_gamepad` → false
- `player.talk_box` → null (freed if allocated)
- `show_keyb` → false
- `KeybAutoRepCap`, `KeybAutoRepChar` → 0
- `input.contact[0..3].action` → Input::Contact::NONE
- `show_inventory` → false
- `show_buts` → false
- `menu_stack[0]` → method-dependent (0 for normal, -1 for side menus)

**Side effects:**
- May free player.talk_box (malloc)
- Clears all input state

**Notes:**
Menu method parameter: 0=main menu, 1=left-side menu, 2=right-side menu (inventory/options). The function sets menu_stack[0] based on method, with -1 indicating a side menu (special behavior for left/right selection). All contact state is cleared to ensure no ghost interactions persist while menu is open.

---

### `Game::CloseMenu` (game.cpp:11073-11082)

**Signature:** `void Game::CloseMenu()`

**Purpose:** Transition back to gameplay state, restoring show_buts and clearing menu depth.

**Called by:**
- `Game::ToggleMenu()` (game.cpp:11087)
- `menu_yes_exit()` (game.cpp:10868 — exit menu action)
- `menu_no_exit()` (game.cpp:10878 — exit menu action)
- `Game::MenuKeyb()` (game.cpp:11212, 11262 — on escape/backspace)
- `Game::MenuMouse()` (game.cpp:11322, 11350)
- `Game::MenuTouch()` (game.cpp:11402, 11440)

**Calls:** None

**Globals read:**
- `menu_depth` — check if menu open

**Globals mutated:**
- `show_buts` → true (restore action buttons)
- `menu_depth` → -1 (menu closed)
- `input.but` → 0 (clear button state)

**Side effects:** None

**Notes:**
Inverse of OpenMenu. Restores show_buts (action buttons visible) and sets menu_depth to -1 to signal menu is closed. input.but is cleared to prevent button state from persisting across menu open/close.

---

### `Game::ToggleMenu` (game.cpp:11084-11090)

**Signature:** `void Game::ToggleMenu(int method)`

**Purpose:** Toggle menu open/closed state, dispatching to OpenMenu or CloseMenu.

**Called by:**
- `Game::OnKeyb()` (game.cpp:8314 — on '\' or '|' character)
- `Game::StartContact()` (game.cpp:8797, 8799 — on right-side button click)
- `Game::OnPadButton()` (game.cpp:10275 — on button 6/start)

**Calls:**
- `CloseMenu()` — if menu_depth >= 0
- `OpenMenu()` — if menu_depth < 0

**Globals read:**
- `menu_depth` — current menu state

**Globals mutated:** (via called functions)

**Side effects:** (via called functions)

**Notes:**
Simple dispatcher. Used for player-triggered menu toggles (backslash key, menu buttons, gamepad start button).

---

### `Game::HitMenu` (game.cpp:11092-11145)

**Signature:** `int Game::HitMenu(int hx, int hy)`

**Purpose:** Test whether a screen click hits the menu title or a menu item, returning the item index, -1 for title, -2 for empty space, or -3 if menu not open.

**Called by:**
- `Game::MenuMouse()` (game.cpp:11319, 11342, 11399)
- `Game::MenuTouch()` (game.cpp:11399, 11421, 11432)

**Calls:**
- `ScreenToCell()` — convert screen pixels to cell coordinates
- `Font1Size()` — measure text dimensions

**Globals read:**
- `menu_depth` — if <0, menu not open
- `game_menu` — menu item array
- `menu_stack[]` — current menu path
- `render_size[2]` — screen dimensions

**Globals mutated:** None

**Side effects:** None (pure hit test)

**Notes:**
Returns -3 if menu_depth<0 (menu closed). Right-aligns menu at (render_size[0]-5, render_size[1]-10). Iterates menu items backwards (top to bottom on screen). Hits are conservative: only match if hx is to the right of the text (not covering the full width).

---

### `Game::PaintMenu` (game.cpp:11147-11196)

**Signature:** `void Game::PaintMenu(AnsiCell* ptr, int width, int height)`

**Purpose:** Render the in-game menu to the framebuffer, including title, items, and visual indicators (arrows, checkboxes).

**Called by:**
- Game::Render()  (not found in excerpt, but called from render loop)

**Calls:**
- `Font1Size()` — measure menu text
- `Font1Paint()` — draw text to framebuffer

**Globals read:**
- `menu_depth` — if <0, skip rendering
- `game_menu` — menu structure
- `menu_stack[]` — current menu path
- `render_size[2]` — screen dimensions

**Globals mutated:** None (framebuffer ptr is output parameter)

**Side effects:**
- Writes to framebuffer (ptr)

**Notes:**
Renders title at (width-5, height-10, right-aligned), then menu items below in descending Y. Selected item (menu_stack[menu_depth]) is drawn in FONT1_GOLD_SKIN; others in FONT1_GREY_SKIN. Indicators: "\x03" for sub-menus, "\x02" for true checkboxes, "\x01" for false checkboxes. Title is always FONT1_PINK_SKIN.

---

### `Game::MenuKeyb` (game.cpp:11198-11296)

**Signature:** `void Game::MenuKeyb(GAME_KEYB keyb, int key)`

**Purpose:** Route keyboard input to menu navigation (arrow keys, enter, escape) and execute menu actions.

**Called by:**
- Game::OnKeyb()  (not in excerpt, but called from keyboard handler)

**Calls:**
- `CloseMenu()` — on escape or backspace at depth 0
- Menu action functions — via game_menu[i].action (pointers like menu_perspective, menu_blood, etc.)

**Globals read:**
- `menu_down` — if true, mouse/touch has captured menu (skip keyboard)
- `game_menu` — menu structure
- `menu_depth`, `menu_stack[]` — current menu navigation
- `keyb` type and `key` code

**Globals mutated:**
- `menu_depth` — increased on right/enter, decreased on left/backspace
- `menu_stack[]` — navigated on up/down
- `menu_temp` — stores previous selection for -1 (back) state

**Side effects:**
- May call menu action functions (which may toggle game state, write config, etc.)

**Notes:**
Special cases: backslash/pipe closes menu directly. Shift+backspace treated as backspace. Enter/newline treated as KEYB_PRESS with A3D_ENTER. If menu_down (mouse/touch active), keyboard is ignored. Negative menu_stack[menu_depth] (-1) indicates a temporary state waiting for direction selection. Up/down navigation cycles through items.

---

### `Game::MenuMouse` (game.cpp:11299-11385)

**Signature:** `void Game::MenuMouse(GAME_MOUSE mouse, int x, int y)`

**Purpose:** Handle mouse clicks and movement in menu, dispatching to menu actions or closing menu.

**Called by:**
- Game::OnMouse() (game.cpp:9755 — if menu_depth >= 0)

**Calls:**
- `HitMenu()` — test click location
- `CloseMenu()` — on click outside menu or on title at depth 0
- Menu action functions — via game_menu[i].action
- Sub-menu navigation — increment menu_depth, populate menu_stack[menu_depth]

**Globals read:**
- `menu_down` — if 2, touch has captured menu
- `game_menu`, `menu_depth`, `menu_stack[]` — menu navigation

**Globals mutated:**
- `menu_down` → 1 (mouse captured), 0 (released)
- `menu_stack[menu_depth]` → hit result or -1

**Side effects:**
- May call menu action functions

**Notes:**
If menu_down==2, touch controls menu; keyboard is ignored. On MOUSE_MOVE, re-test hit and clear selection if cursor drifted away. On LEFT_BUT_DOWN, capture menu and set selection. On LEFT_BUT_UP, execute action if still over same item. Negative index (-1) at top means "go back" action; out-of-bounds (<-1) closes menu.

---

### `Game::MenuTouch` (game.cpp:11387-11483)

**Signature:** `void Game::MenuTouch(GAME_TOUCH touch, int id, int x, int y)`

**Purpose:** Handle multi-touch input in menu (only touch id=1 is processed; id>1 ignored).

**Called by:**
- Game::OnTouch() (game.cpp:9990 — if menu_depth >= 0)

**Calls:**
- `HitMenu()` — test touch location
- `CloseMenu()` — on touch outside menu or on title at depth 0
- Menu action functions — via game_menu[i].action
- Sub-menu navigation — increment menu_depth

**Globals read:**
- `menu_down` — if 1, mouse has captured menu
- `game_menu`, `menu_depth`, `menu_stack[]` — menu navigation

**Globals mutated:**
- `menu_down` → 2 (touch captured), 0 (released)
- `menu_stack[menu_depth]` → hit result or -1

**Side effects:**
- May call menu action functions

**Notes:**
Only id=1 (first touch) is processed; other touches return early. Mirrors MenuMouse behavior but for touch events. TOUCH_MOVE re-tests hit. TOUCH_CANCEL resets menu_down and selection. TOUCH_END executes action if still over same item.

---

### `Game::MenuPadMount` (game.cpp:11485-11487)

**Signature:** `void Game::MenuPadMount(bool connected)`

**Purpose:** Stub for menu-specific gamepad connect handling.

**Called by:**
- Game::OnPadMount() (game.cpp:10096 — if menu_depth >= 0)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:**
Empty implementation. Can be extended to show gamepad help or adjust menu controls on gamepad connect.

---

### `Game::MenuPadButton` (game.cpp:11489-11610)

**Signature:** `void Game::MenuPadButton(int b, bool down)`

**Purpose:** Route gamepad buttons to menu navigation (D-pad, A button, start to close) and execute actions.

**Called by:**
- Game::OnPadButton() (game.cpp:10150 — if menu_depth >= 0 and down=true)

**Calls:**
- `CloseMenu()` — on button 6 (start) or button 13 at depth 0 (D-pad left)
- Menu action functions — via game_menu[i].action

**Globals read:**
- `menu_down` — if true, mouse/touch has captured menu
- `game_menu`, `menu_depth`, `menu_stack[]` — menu navigation

**Globals mutated:**
- `menu_stack[]` — navigated on D-pad up/down/left/right
- `menu_temp` — stores selection state
- `menu_depth` — increased on D-pad right/A, decreased on D-pad left

**Side effects:**
- May call menu action functions

**Notes:**
Button mapping: 0=A (select), 5=guide (unused), 6=start (close at depth 0), 9=LB, 10=RB (unused), 11-14=D-pad up/down/left/right. Up/down navigate items. Left goes back (decrement menu_depth). Right enters sub-menu or navigates if already in sub-menu. Negative menu_stack[menu_depth] (-1) stores deselected state; up/down from -1 restores menu_temp.

---

### `Game::MenuPadAxis` (game.cpp:11612-11614)

**Signature:** `void Game::MenuPadAxis(int a, int16_t pos)`

**Purpose:** Stub for menu-specific analog stick handling.

**Called by:**
- Game::OnPadAxis() (game.cpp:10578 — if menu_depth >= 0)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:**
Empty implementation. D-pad input is handled by MenuPadButton; analog stick input is not currently used in menus.

---

## Summary

The input system in game.cpp (lines 8001-12077) provides:

1. **Contact Management** (StartContact, MoveContact, EndContact, GetContact) — abstraction for mouse, touch, keyboard input into unified "contact" state machine with gesture detection.

2. **Input Routing** (OnMouse, OnTouch, OnFocus, OnMessage, OnPadMount, OnPadButton, OnPadAxis) — platform-layer event dispatch with proper focus handling and multi-touch support.

3. **Menu System** (OpenMenu, CloseMenu, ToggleMenu, HitMenu, PaintMenu, MenuKeyb, MenuMouse, MenuTouch, MenuPadMount, MenuPadButton, MenuPadAxis) — hierarchical menu navigation with keyboard, mouse, touch, and gamepad support.

All functions use pre-allocated global state (`input`, `inventory`, `keyb`, `player`, `menu_*` globals) to avoid heap allocation per-frame. Network integration (server->Send) and script hooks (akAPI_OnSay, akAPI_Exec) are called from contact handlers to support multiplayer and Lua scripting.


### `PlayerHit` (game.cpp:8726-8770)

**Signature:** `static bool PlayerHit(Game* g, int x, int y)`

**Purpose:** Tests if screen point (x,y) intersects player sprite hitbox or talkbox input field.

**Called by:** `Game::StartContact` (game.cpp:9256,9444)

**Calls:** `Game::ScreenToCell` — converts screen to cell coordinates

**Globals read:** `g->player.talk_box`, `g->player.sprite`, `g->player.dir`, `g->player.frame`, `g->player.anim`, `g->prev_yaw`, `g->render_size[2]`, `g->scene_shift`

**Globals mutated:** None

**Side effects:** None

**Notes:** Computes sprite frame index from player direction (accounts for animation angles). Checks if pixel is non-transparent (fg!=255 or bk!=255 and gl!=32/219). Also tests talkbox UI bounds when open.


### `FirstFree` (game.cpp:9719-9732)

**Signature:** `int FirstFree(int size, int* arr)`

**Purpose:** Finds smallest unused integer id in [1,size] by scanning array.

**Called by:** `Game::HitMenu` (game.cpp:9793 — assigns button IDs)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Linear scan O(n*size). Returns -1 if all IDs 1..size are present in arr. Used to allocate unique button instance IDs in menu.


### `PaintTerrain` (game.cpp:10721-10735)

**Signature:** `void PaintTerrain(float* xy, float r, int matid)`

**Purpose:** Applies material ID stamp to terrain cells within radius r from position xy.

**Called by:** `BloodLeak` (game.cpp:10749 — blood paint), `Game::Render` (game.cpp:6292,6522 — damage blood effects)

**Calls:** `QueryTerrain` — queries and modifies terrain cells

**Globals read:** `terrain` — global terrain quadtree

**Globals mutated:** None directly; modifies terrain cells via QueryTerrain callback

**Side effects:** Modifies terrain cell data (sets matid) for cells within radius

**Notes:** EDITOR-build only. Uses MatIDStamp::SetMatCB callback to set material ID. Radius multiplier 0.501 ensures coverage of all cells intersecting circle.


### `BloodLeak` (game.cpp:10734-10758)

**Signature:** `void BloodLeak(Character* c, int steps)`

**Purpose:** Spawns blood paint on terrain beneath character at intervals based on leak flag.

**Called by:** `Game::Render` (game.cpp:5850 — player damage), `Game::Render` (game.cpp:6129,6205 — NPC damage)

**Calls:** `PaintTerrain` — paints blood material (matid=5), `fast_rand` — random offset generation

**Globals read:** fast_rand — global RNG state

**Globals mutated:** `c->leak_steps` — leak step accumulator, `c->leak` — leak counter decremented

**Side effects:** Mutates Character fields, modifies terrain via PaintTerrain

**Notes:** Every 5 leak_steps, decrements leak counter and paints random radius (0-1.9) blood at random position in circle radius 1.0 around character.


### `GamePadMount` (game.cpp:10759-10771)

**Signature:** `void GamePadMount(const char* name, int axes, int buttons, const uint8_t mapping[])`

**Purpose:** Global callback when gamepad connects; loads saved mapping, notifies game.

**Called by:** `GamePadOnConnect` (gamepad.cpp:11050 — gamepad driver)

**Calls:** `ConnectGamePad` — driver-level connection, `ReadGamePadConf` — load saved mapping, `SetGamePadMapping` — apply mapping, `WriteConf` — via Game::OnPadMount

**Globals read:** `prime_game` — primary game instance

**Globals mutated:** None directly; calls Game::OnPadMount which mutates game state

**Side effects:** Platform gamepad driver connection, file I/O (ReadGamePadConf)

**Notes:** Calls prime_game->OnPadMount(true) to notify game of controller connect. Reads and applies saved button/axis mapping from disk.


### `GamePadUnmount` (game.cpp:10773-10778)

**Signature:** `void GamePadUnmount()`

**Purpose:** Global callback when gamepad disconnects; notifies game.

**Called by:** `GamePadOnDisconnect` (implied)

**Calls:** `DisconnectGamePad` — driver-level disconnection

**Globals read:** `prime_game` — primary game instance

**Globals mutated:** None

**Side effects:** Platform gamepad driver disconnection

**Notes:** Calls prime_game->OnPadMount(false) to notify game of controller disconnect.

### `GamePadButton` (game.cpp:10782-10805)

**Signature:** `void GamePadButton(int b, int16_t pos)`

**Purpose:** Global callback for button state changes; maps to axis/button outputs and dispatches to game.

**Called by:** `GamePadOnButton` (gamepad.cpp:11245)

**Calls:** `UpdateGamePadButton` — translates input index to output mapping, `Game::OnPadAxis` / `Game::OnPadButton` — dispatch to game

**Globals read:** `prime_game` — primary game instance

**Globals mutated:** None

**Side effects:** Dispatches input events to game

**Notes:** Maps single input index to multiple outputs (axis mapping, button mapping). Output format: map = (type << 16) | (index << 24) | value. Type 0 = axis, Type 1 = button.

### `GamePadAxis` (game.cpp:10805-10826)

**Signature:** `void GamePadAxis(int a, int16_t pos)`

**Purpose:** Global callback for axis state changes; maps to axis/button outputs and dispatches to game.

**Called by:** `GamePadOnAxis` (gamepad.cpp:11271)

**Calls:** `UpdateGamePadAxis` — translates input index to output mapping, `Game::OnPadAxis` / `Game::OnPadButton` — dispatch to game

**Globals read:** `prime_game` — primary game instance

**Globals mutated:** None

**Side effects:** Dispatches input events to game

**Notes:** Maps single axis to 1-4 outputs (D-Pad or L/R joystick mapping). Same output format as GamePadButton.

### `menu_perspective` (game.cpp:10844-10848)

**Signature:** `void menu_perspective(Game* g)`

**Purpose:** Toggles camera perspective mode and saves config.

**Called by:** Menu action callback

**Calls:** `WriteConf` — saves setting to disk

**Globals read:** None

**Globals mutated:** `g->perspective` — toggled

**Side effects:** File I/O (WriteConf)

**Notes:** Inverts g->perspective boolean.

### `menu_perspective_getter` (game.cpp:10850-10853)

**Signature:** `bool menu_perspective_getter(Game* g)`

**Purpose:** Returns current perspective mode state for menu display.

**Called by:** Menu getter callback

**Calls:** None

**Globals read:** `g->perspective`

**Globals mutated:** None

**Side effects:** None

**Notes:** Getter for perspective toggle state.

### `menu_blood` (game.cpp:10855-10859)

**Signature:** `void menu_blood(Game* g)`

**Purpose:** Toggles blood effects and saves config.

**Called by:** Menu action callback

**Calls:** `WriteConf` — saves setting to disk

**Globals read:** None

**Globals mutated:** `g->blood` — toggled

**Side effects:** File I/O (WriteConf), terrain changes (if PaintTerrain used)

**Notes:** Inverts g->blood boolean. Controls whether blood paint appears on terrain.

### `menu_blood_getter` (game.cpp:10861-10864)

**Signature:** `bool menu_blood_getter(Game* g)`

**Purpose:** Returns current blood effects state for menu display.

**Called by:** Menu getter callback

**Calls:** None

**Globals read:** `g->blood`

**Globals mutated:** None

**Side effects:** None

**Notes:** Getter for blood toggle state.

### `menu_yes_exit` (game.cpp:10867-10876)

**Signature:** `void menu_yes_exit(Game* g)`

**Purpose:** Confirms menu exit and terminates application.

**Called by:** Menu action callback (exit confirm dialog)

**Calls:** `exit` or `exit_handler`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Process termination (exit)

**Notes:** Calls exit(0) on SDL, exit_handler(0) otherwise.

### `menu_no_exit` (game.cpp:10878-10883)

**Signature:** `void menu_no_exit(Game* g)`

**Purpose:** Cancels menu exit and returns to previous menu.

**Called by:** Menu action callback (exit cancel)

**Calls:** None

**Globals read:** None

**Globals mutated:** `g->menu_depth` — decremented, `g->menu_temp` — restored from stack

**Side effects:** None

**Notes:** Pops menu depth, restores previous menu state.

### `menu_fullscreen` (game.cpp:10892-10898)

**Signature:** `void menu_fullscreen(Game* g)`

**Purpose:** Toggles fullscreen display mode.

**Called by:** Menu action callback

**Calls:** `ToggleFullscreen` — platform-specific fullscreen toggle

**Globals read:** None

**Globals mutated:** None

**Side effects:** Window mode change

**Notes:** SERVER build stub. Calls ToggleFullscreen on non-SERVER builds.

### `menu_fullscreen_getter` (game.cpp:10900-10906)

**Signature:** `bool menu_fullscreen_getter(Game* g)`

**Purpose:** Returns current fullscreen state for menu display.

**Called by:** Menu getter callback

**Calls:** `IsFullscreen` — platform-specific fullscreen query

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** SERVER build returns false. Calls IsFullscreen on non-SERVER builds.

### `menu_mute` (game.cpp:10908-10916)

**Signature:** `void menu_mute(Game* g)`

**Purpose:** Toggles audio mute and saves config.

**Called by:** Menu action callback

**Calls:** `AudioMute` — audio output control, `WriteConf` — saves setting to disk

**Globals read:** None

**Globals mutated:** `g->mute` — toggled

**Side effects:** Audio state change, file I/O (WriteConf)

**Notes:** SERVER build stub. Toggles g->mute, calls AudioMute on non-SERVER builds.

### `menu_mute_getter` (game.cpp:10916-10920)

**Signature:** `bool menu_mute_getter(Game* g)`

**Purpose:** Returns current mute state for menu display.

**Called by:** Menu getter callback

**Calls:** None

**Globals read:** `g->mute`

**Globals mutated:** None

**Side effects:** None

**Notes:** Getter for mute toggle state.

### `menu_zoomin` (game.cpp:10922-10928)

**Signature:** `void menu_zoomin(Game* g)`

**Purpose:** Increases font size (zoom in).

**Called by:** Menu action callback

**Calls:** `NextGLFont` — advances to next larger font

**Globals read:** None

**Globals mutated:** None

**Side effects:** Font size change

**Notes:** SERVER build stub. Calls NextGLFont on non-SERVER builds.

### `menu_zoomout` (game.cpp:10929-10935)

**Signature:** `void menu_zoomin(Game* g)`

**Purpose:** Decreases font size (zoom out).

**Called by:** Menu action callback

**Calls:** `PrevGLFont` — retreats to previous smaller font

**Globals read:** None

**Globals mutated:** None

**Side effects:** Font size change

**Notes:** SERVER build stub. Calls PrevGLFont on non-SERVER builds.

### `gamepad_close` (game.cpp:10936-10952)

**Signature:** `void gamepad_close(void* _g)`

**Purpose:** Callback for gamepad config UI close; saves mapping and hides UI.

**Called by:** `GamePadClose` — gamepad UI system

**Calls:** `GetGamePadMapping` — retrieves current mapping, `GetGamePad` — retrieves gamepad info, `WriteGamePadConf` — saves to disk

**Globals read:** None

**Globals mutated:** `g->show_gamepad` — set false, `g->show_buts` — set true

**Side effects:** File I/O (WriteGamePadConf)

**Notes:** Restores buttons display (show_buts). Saves current button/axis mapping to disk config.

### `menu_gamepad` (game.cpp:10952-10958)

**Signature:** `void menu_gamepad(Game* g)`

**Purpose:** Opens gamepad configuration UI.

**Called by:** Menu action callback

**Calls:** `Game::CloseMenu` — closes current menu, `GamePadOpen` — opens gamepad UI

**Globals read:** None

**Globals mutated:** `g->show_gamepad` — set true, `g->show_buts` — set false

**Side effects:** UI state change (opens gamepad config)

**Notes:** Hides buttons display while in gamepad config UI. Passes gamepad_close callback to GamePadOpen.

### `main_menu` (game.cpp:10960-10968)

**Signature:** `void main_menu(Game* g)`

**Purpose:** Opens main menu from game.

**Called by:** Menu action callback

**Calls:** `Game::CloseMenu` — closes current menu

**Globals read:** None

**Globals mutated:** `g->main_menu` — set true

**Side effects:** UI state change (main menu display)

**Notes:** EDITOR build stub. Sets main_menu flag (which triggers MainMenu_Show in CreateGame loop).

