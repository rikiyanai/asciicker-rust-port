# gamepad.cpp Architecture Analysis

**File:** `/Users/r/Downloads/asciicker-Y9-2/gamepad.cpp`  
**Lines:** 2318  
**Purpose:** Visual drag-drop gamepad configuration UI with mapping system

## System Overview

Implements a complete gamepad remapping system with visual drag-drop interface. Supports Xbox and PS5 controller layouts, maintains inverse lookup tables for efficient event routing, and persists mappings to disk.

### Key Responsibilities
- **Mapping Translation:** Input indices (0-255) → Output indices (0-20)
- **Visual Configuration:** Drag-drop UI for button/axis remapping
- **Event Processing:** Real-time axis/button event translation via mapping tables
- **Inverse Lookups:** Efficient "which inputs map to this output?" queries
- **Layout Switching:** Xbox ↔ PS5 controller visual representation

---

## Global State

### Mapping Tables

#### `gamepad_mapping[256]` (line 205)
**Type:** `static uint8_t[256]`  
**Purpose:** Active input→output mapping (user-configured, persisted to disk)  
**Values:**
- `0-20`: Output index (6 axes + 15 buttons)
- `0xFC-0xFE`: Special mappings (L-Joy, R-Joy, D-Pad)
- `0xFF`: Unmapped input
**Mutated by:** `SetGamePadMapping`, `GamePadContact`, `GamePadKeyb`, `ConnectGamePad`

#### `gamepad_mount_mapping[256]` (line 204)
**Type:** `static uint8_t[256]`  
**Purpose:** Default mapping from SDL at gamepad connect (used for "reset to default")  
**Mutated by:** `ConnectGamePad`

#### `button_mapping[15]` (line 191)
**Type:** `static uint8_t*[15]`  
**Purpose:** Inverse map: output button → list of input indices (0xFF terminated)  
**Allocated by:** `InvertMap` (dynamic allocation, variable-length arrays)  
**Example:** `button_mapping[0] = {5, 12, 0xFF}` means inputs 5 and 12 both map to button A  
**WHY inverse tables:** Visual UI needs "highlight all inputs that map to this output"

#### `axis_mapping[6]` (line 192)
**Type:** `static uint8_t*[6]`  
**Purpose:** Inverse map: output axis → list of input indices (0xFF terminated)  
**Allocated by:** `InvertMap` (dynamic allocation)

### Raw Input State

#### `gamepad_button[256]` (line 215)
**Type:** `static int16_t[256]`  
**Purpose:** Raw button states from SDL (0 = released, 32767 = pressed)  
**Mutated by:** `UpdateGamePadButton`  
**Used by:** `PaintGamePad` (visual feedback)

#### `gamepad_axis[256]` (line 214)
**Type:** `static int16_t[256]`  
**Purpose:** Raw axis states from SDL (-32768 to +32767)  
**Mutated by:** `UpdateGamePadAxis`  
**Used by:** `PaintGamePad`, `UpdateAxisOutput` (for special mappings 0xFC-0xFE)

#### `gamepad_input[256]` (line 206)
**Type:** `static int16_t[256]`  
**Purpose:** Processed input values (all positive, axes split into two half-indices)  
**WHY all positive:** Axes are bipolar but each half (negative/positive) treated as unipolar input  
**Mutated by:** `UpdateGamePadAxis`, `UpdateGamePadButton`

### Mapped Output State

#### `gamepad_axis_output[6]` (line 210)
**Type:** `static int16_t[6]`  
**Purpose:** Mapped axis states after applying gamepad_mapping[] (output space)  
**Mutated by:** `UpdateAxisOutput`  
**Read by:** Game code (via output events), `PaintGamePad` (stick visualization)

#### `gamepad_button_output[15]` (line 211)
**Type:** `static int16_t[15]`  
**Purpose:** Mapped button states (output space)  
**Mutated by:** `UpdateButtonOutput`  
**Read by:** Game code, `PaintGamePad` (button highlight)

### Gamepad Info

#### `gamepad_name[256]` (line 163)
**Type:** `static char[256]`  
**Purpose:** Connected gamepad name (e.g., "Xbox Wireless Controller")  
**Mutated by:** `ConnectGamePad`

#### `gamepad_axes` (line 164)
**Type:** `static int`  
**Purpose:** Number of axes on connected gamepad (typically 6)  
**Mutated by:** `ConnectGamePad`, `DisconnectGamePad`

#### `gamepad_buttons` (line 165)
**Type:** `static int`  
**Purpose:** Number of buttons on connected gamepad (typically 15)  
**Mutated by:** `ConnectGamePad`, `DisconnectGamePad`

#### `gamepad_connected` (line 166)
**Type:** `static bool`  
**Purpose:** Gamepad connection state  
**Mutated by:** `ConnectGamePad`, `DisconnectGamePad`

### Visual State

#### `gamepad_sprite` (line 162)
**Type:** `static Sprite*`  
**Purpose:** Loaded .xp sprite for gamepad visual (50×27 cells)  
**Mutated by:** `LoadGamePad`, `FreeGamePad`

#### `gamepad_assembly` (line 256)
**Type:** `static int`  
**Purpose:** Layout selection (0 = Xbox, 1 = PS5)  
**Mutated by:** `Swap`

#### `gamepad_half_axis_xy` (line 249)
**Type:** `static const int16_t*`  
**Purpose:** Pointer to active half-axis output positions (xbox_half_axis_xy or ps5_half_axis_xy)  
**Mutated by:** `Swap`

#### `gamepad_button_xy` (line 250)
**Type:** `static const int16_t*`  
**Purpose:** Pointer to active button output positions (xbox_button_xy or ps5_button_xy)  
**Mutated by:** `Swap`

#### `gamepad_layout_x, gamepad_layout_y` (line 221-222)
**Type:** `static int16_t`  
**Purpose:** Upper-left corner of visual layout (calculated in `PaintGamePad`)  
**Mutated by:** `PaintGamePad`

#### `gamepad_input_xy[2*256]` (line 218)
**Type:** `static int16_t[512]`  
**Purpose:** Screen coordinates of input slots (for drag-drop hit detection)  
**Mutated by:** `PaintGamePad`, `BlitButton`

### Drag-Drop State

#### `gamepad_contact` (line 168)
**Type:** `static int`  
**Purpose:** Contact ID tracking  
**Values:**
- `-1`: No contact
- `0`: Mouse contact (left button)
- `1+`: Touch contact (touch ID)
**Mutated by:** `GamePadContact`

#### `gamepad_contact_from[2]` (line 169)
**Type:** `static int[2]`  
**Purpose:** Drag start position (x, y cell coordinates)  
**Mutated by:** `GamePadContact`

#### `gamepad_contact_pos[2]` (line 170)
**Type:** `static int[2]`  
**Purpose:** Current drag position (updated on move events)  
**Mutated by:** `GamePadContact`

#### `gamepad_contact_output` (line 171)
**Type:** `static uint8_t`  
**Purpose:** Output index being dragged (0xFF = input slot, 0-20 = output)  
**Mutated by:** `GamePadContact`

#### `gamepad_contact_ui` (line 172)
**Type:** `static uint8_t`  
**Purpose:** UI element being dragged (0=clear, 1=init, 2=quit, 0xFF=none)  
**Mutated by:** `GamePadContact`

### Keyboard Navigation State

#### `gamepad_keyb_focus` (line 224)
**Type:** `static uint8_t`  
**Purpose:** Currently focused input slot (0xFF = no focus)  
**Mutated by:** `ConnectGamePad`, `GamePadKeyb`, `UpdateGamePadAxis`, `UpdateGamePadButton`  
**WHY auto-focus in UpdateGamePad*:** Passing axis/button threshold auto-focuses that input

#### `gamepad_keyb_edit` (line 225)
**Type:** `static uint8_t`  
**Purpose:** Edit mode state (0xFF = not editing, 0 = first char, 1 = second char)  
**Mutated by:** `GamePadKeyb`

#### `gamepad_keyb_char[2]` (line 226)
**Type:** `static char[2]`  
**Purpose:** Buffer for two-character keyboard input (e.g., "Ll" for left stick left)  
**Mutated by:** `GamePadKeyb`

### Animation State

#### `gamepad_swap_stamp` (line 396)
**Type:** `static uint64_t`  
**Purpose:** Timestamp when layout swap animation started (0 = not animating)  
**Mutated by:** `Swap`  
**Duration:** 16 frames (~1 second at 60fps)

### Callback State

#### `gamepad_close_cb` (line 158)
**Type:** `static void (*)(void*)`  
**Purpose:** Function pointer for close callback (called when user exits config UI)  
**Mutated by:** `GamePadOpen`

#### `gamepad_close_g` (line 159)
**Type:** `static void*`  
**Purpose:** User data passed to close callback  
**Mutated by:** `GamePadOpen`

---

## Constant Tables

### Layout Position Tables

#### `xbox_half_axis_xy[2*12]` (line 229)
**Type:** `static const int16_t[24]`  
**Purpose:** Xbox layout output positions for 12 half-axes (Ll,Lr,Lu,Ld,Rl,Rr,Ru,Rd,Lt,Lt,Rt,Rt)  
**Format:** Pairs of (x, y) coordinates relative to sprite top-left  
**Y-axis:** Top to bottom (sprite space)

#### `xbox_button_xy[2*15]` (line 234)
**Type:** `static const int16_t[30]`  
**Purpose:** Xbox layout output positions for 15 buttons (A,B,X,Y,E,G,F,L,R,Ls,Rs,Du,Dd,Dl,Dr)

#### `ps5_half_axis_xy[2*12]` (line 239)
**Type:** `static const int16_t[24]`  
**Purpose:** PS5 layout half-axis positions

#### `ps5_button_xy[2*15]` (line 244)
**Type:** `static const int16_t[30]`  
**Purpose:** PS5 layout button positions

#### `gamepad_swap_xy[2]` (line 395)
**Type:** `static const int16_t[2]`  
**Purpose:** Screen position of layout swap button (16, 17)

### Name Tables

#### `gamepad_half_axis_name[]` (line 258)
**Type:** `static const char*[13]`  
**Purpose:** Two-character names for half-axes (null-terminated)  
**Values:** `"Ll","Lr","Lu","Ld","Rl","Rr","Ru","Rd","Lt","Lt","Rt","Rt",0`

#### `gamepad_button_name[]` (line 263)
**Type:** `static const char*[16]`  
**Purpose:** Two-character names for buttons (null-terminated)  
**Values:** `"A ","B ","X ","Y ","E ","G ","F ","L ","R ","Ls","Rs","Du","Dd","Dl","Dr",0`

#### `gamepad_special_name[]` (line 268)
**Type:** `static const char*[4]`  
**Purpose:** Names for special mappings (0xFC-0xFE)  
**Values:** `0, "L-Joy", "R-Joy", "D-Pad"`

### Sprite Element Tables

#### `ui_proto[6]` (line 284)
**Type:** `static const InputElem[6]`  
**Purpose:** UI button prototypes (clear, init, quit, highlight frames)  
**Fields:** `src_x, src_y, w, h` (sprite atlas coordinates)

#### `axis_proto[9]` (line 295)
**Type:** `static const InputElem[10]`  
**Purpose:** Axis visual states (9 levels from -max to +max)  
**WHY 9 states:** Provides granular visual feedback for axis position

#### `button_proto[9]` (line 311)
**Type:** `static const InputElem[10]`  
**Purpose:** Button visual states (9 pressure levels)

#### `slot_proto[3]` (line 325)
**Type:** `static const InputElem[4]`  
**Purpose:** Input slot visual elements (normal, drag, prolong)

#### `gamepad_proto[]` (line 333)
**Type:** `static const SpriteElem[]`  
**Purpose:** Gamepad visual assembly (body, handles, sticks, buttons, triggers)  
**Fields:** `src_x, src_y, w, h, dst_x, dst_y, dst_x2, dst_y2`  
**WHY dual dst coords:** `dst_x/dst_y` = Xbox positions, `dst_x2/dst_y2` = PS5 positions

---

## Core Functions

### `LoadGamePad` (gamepad.cpp:415-421)

**Signature:** `void LoadGamePad()`  
**Purpose:** Load gamepad sprite asset from disk  
**Called by:** `game.cpp:3261` (game initialization)  
**Calls:** `LoadSprite`  
**Globals read:** `base_path`  
**Globals mutated:** `gamepad_sprite`  
**Side effects:** Disk I/O (loads "sprites/gamepad.xp")  
**Notes:** Must be called before `PaintGamePad` or UI will not render

---

### `FreeGamePad` (gamepad.cpp:423-427)

**Signature:** `void FreeGamePad()`  
**Purpose:** Unload gamepad sprite and free memory  
**Called by:** No callers found via grep  
**Calls:** `FreeSprite`  
**Globals read:** `gamepad_sprite`  
**Globals mutated:** `gamepad_sprite`  
**Side effects:** Frees sprite memory  
**Notes:** Should be called during game cleanup

---

### `GetGamePad` (gamepad.cpp:429-436)

**Signature:** `const char* GetGamePad(int* axes, int* buttons)`  
**Purpose:** Query connected gamepad info  
**Called by:** `game.cpp:10761` (gamepad mount)  
**Calls:** None  
**Globals read:** `gamepad_connected`, `gamepad_name`, `gamepad_axes`, `gamepad_buttons`  
**Globals mutated:** None  
**Side effects:** None  
**Notes:** Returns NULL if no gamepad connected, otherwise returns name and populates axes/buttons

---

### `ConnectGamePad` (gamepad.cpp:889-931)

**Signature:** `void ConnectGamePad(const char* name, int axes, int buttons, const uint8_t mapping[])`  
**Purpose:** Initialize gamepad state on device connect  
**Called by:** `game.cpp:10761` (SDL gamepad mount event)  
**Calls:** `InvertMap`  
**Globals read:** None  
**Globals mutated:** `gamepad_name`, `gamepad_buttons`, `gamepad_axes`, `gamepad_connected`, `gamepad_mapping`, `gamepad_mount_mapping`, `gamepad_axis_output`, `gamepad_button_output`, `gamepad_input`, `gamepad_axis`, `gamepad_button`, `gamepad_contact`, `gamepad_keyb_focus`, `gamepad_keyb_edit`  
**Side effects:** Memory allocation (inverse tables via `InvertMap`)  
**Notes:** WHY rebuild inverse tables: new gamepad may have different button/axis count. Sets keyb_focus=0 to encourage keyboard navigation.

---

### `DisconnectGamePad` (gamepad.cpp:933-961)

**Signature:** `void DisconnectGamePad()`  
**Purpose:** Clean up gamepad state on device disconnect  
**Called by:** No callers found via grep  
**Calls:** `free`  
**Globals read:** `axis_mapping`, `button_mapping`  
**Globals mutated:** `axis_mapping`, `button_mapping`, `gamepad_name`, `gamepad_buttons`, `gamepad_axes`, `gamepad_connected`, `gamepad_contact`, `gamepad_keyb_focus`, `gamepad_keyb_edit`  
**Side effects:** Memory deallocation (inverse tables)  
**Notes:** Frees all inverse mapping arrays to prevent memory leak

---

### `UpdateGamePadAxis` (gamepad.cpp:625-709)

**Signature:** `int UpdateGamePadAxis(int a, int16_t v, uint32_t out[4])`  
**Purpose:** Apply mapping to axis event, return output indices  
**Called by:** `game.cpp:10808` (SDL axis event)  
**Calls:** `UpdateAxisOutput`, `UpdateButtonOutput`  
**Globals read:** `gamepad_keyb_edit`, `gamepad_axis`, `gamepad_mapping`, `gamepad_input`  
**Globals mutated:** `gamepad_keyb_focus`, `gamepad_axis`, `gamepad_input`  
**Side effects:** Auto-focus input when axis crosses ±16384 threshold  
**Notes:** WHY threshold focus: provides feedback when user physically moves stick. Axes split into TWO inputs: negative half (index 2*a), positive half (index 2*a+1). Special mappings 0xFC-0xFE trigger multi-output updates (joy/dpad).

**Algorithm:**
1. Auto-focus input if axis crosses ±16384 threshold (unless editing)
2. Split bipolar axis into unipolar neg/pos values
3. Check for special mappings (0xFC-0xFE): L-Joy, R-Joy, D-Pad
4. Update gamepad_input for changed halves
5. Call UpdateAxisOutput or UpdateButtonOutput for each mapped output
6. Return count of output events generated

---

### `UpdateGamePadButton` (gamepad.cpp:714-750)

**Signature:** `int UpdateGamePadButton(int b, int16_t v, uint32_t out[1])`  
**Purpose:** Apply mapping to button event, return output indices  
**Called by:** `game.cpp:10785` (SDL button event)  
**Calls:** `UpdateAxisOutput`, `UpdateButtonOutput`  
**Globals read:** `gamepad_keyb_edit`, `gamepad_button`, `gamepad_axes`, `gamepad_input`, `gamepad_mapping`  
**Globals mutated:** `gamepad_keyb_focus`, `gamepad_button`, `gamepad_input`  
**Side effects:** Auto-focus input when button crosses 16384 threshold  
**Notes:** WHY offset by 2*gamepad_axes: buttons start AFTER all axis indices (each axis uses TWO indices). Example: 6 axes → indices 0-11 for axes, 12+ for buttons.

**Algorithm:**
1. Auto-focus input if button crosses 16384 threshold (unless editing)
2. Convert to unipolar positive value
3. Update gamepad_input at offset 2*gamepad_axes + b
4. Lookup mapping, call UpdateAxisOutput or UpdateButtonOutput
5. Return count of output events generated

---

### `UpdateAxisOutput` (gamepad.cpp:438-530)

**Signature:** `static int UpdateAxisOutput(int a, uint32_t* out)`  
**Purpose:** Accumulate all inputs mapped to output axis, generate event if changed  
**Called by:** `UpdateGamePadAxis`, `UpdateGamePadButton`, `InvertMap`  
**Calls:** None  
**Globals read:** `axis_mapping`, `gamepad_input`, `gamepad_mapping`, `gamepad_axes`, `gamepad_axis`, `gamepad_axis_output`  
**Globals mutated:** `gamepad_axis_output`  
**Side effects:** None  
**Notes:** WHY accumulation: multiple inputs can map to same output (e.g., keyboard + gamepad both control movement). WHY special handling for signed axes: if both halves map to same output (unsigned), reconstruct bipolar value from both halves. WHY chromium bug workaround: hat value rescale `(hat+32767)*7/8 - 32767` for circular deadzone.

**Algorithm:**
1. Walk inverse map axis_mapping[a] (0xFF terminated list)
2. For each input index:
   - If unsigned axis (both halves map to same output): reconstruct bipolar value
   - If half-axis: add/subtract based on 0x40 bit (polarity)
   - If special mapping (0xFC-0xFE): decode hat angle to X/Y component
3. Clamp accumulator to [-32767, +32767]
4. Compare with previous gamepad_axis_output[a]
5. If changed: update output, pack event (value | axis_index<<24 | type<<16), return 1
6. Else: return 0

---

### `UpdateButtonOutput` (gamepad.cpp:532-616)

**Signature:** `static int UpdateButtonOutput(int b, uint32_t* out)`  
**Purpose:** Accumulate all inputs mapped to output button, generate event if changed  
**Called by:** `UpdateGamePadAxis`, `UpdateGamePadButton`, `InvertMap`  
**Calls:** None  
**Globals read:** `button_mapping`, `gamepad_input`, `gamepad_mapping`, `gamepad_axis`, `gamepad_button_output`  
**Globals mutated:** `gamepad_button_output`  
**Side effects:** None  
**Notes:** Similar to UpdateAxisOutput but for buttons. WHY directional decoding for buttons 11-14: D-pad buttons (Du,Dd,Dl,Dr) can be mapped from hat axis, need to decode angle to directional buttons.

**Algorithm:**
1. Walk inverse map button_mapping[b] (0xFF terminated list)
2. For each input index:
   - If half-axis: add/subtract based on 0x40 bit
   - If special mapping (0xFC-0xFE): decode hat angle to directional button (Du,Dd,Dl,Dr)
3. Clamp accumulator to [0, +32767]
4. Compare with previous gamepad_button_output[b]
5. If changed: update output, pack event, return 1
6. Else: return 0

---

### `InvertMap` (gamepad.cpp:767-884)

**Signature:** `static void InvertMap(int mappings)`  
**Purpose:** Rebuild inverse lookup tables from gamepad_mapping[]  
**Called by:** `ConnectGamePad`, `SetGamePadMapping`, `GamePadContact`, `GamePadKeyb`  
**Calls:** `malloc`, `UpdateAxisOutput`, `UpdateButtonOutput`  
**Globals read:** `gamepad_mapping`, `axis_mapping`, `button_mapping`  
**Globals mutated:** `axis_mapping`, `button_mapping`  
**Side effects:** Memory allocation (malloc for variable-length inverse arrays)  
**Notes:** WHY rebuild from scratch: simpler than incremental update, no risk of stale pointers, fast enough (256 iterations < 1ms). WHY 0xFF sentinel: marks end of variable-length list, no separate length storage needed. WHY two passes: first pass counts lengths, second pass populates arrays.

**Algorithm:**
1. **First pass:** Count length needed for each output (scan gamepad_mapping[])
2. Allocate new inverse arrays with +1 for 0xFF sentinel
3. **Second pass:** Populate arrays by iterating gamepad_mapping[] again
4. Terminate each array with 0xFF sentinel
5. Update all outputs (force recalculation via UpdateAxisOutput/UpdateButtonOutput)

---

### `SetGamePadMapping` (gamepad.cpp:1725-1761)

**Signature:** `void SetGamePadMapping(const uint8_t* map)`  
**Purpose:** Load mapping from disk (256 bytes binary)  
**Called by:** `game.cpp` (mapping persistence)  
**Calls:** `InvertMap`, `free`  
**Globals read:** `axis_mapping`, `button_mapping`, `gamepad_axes`, `gamepad_buttons`  
**Globals mutated:** `gamepad_mapping`, `axis_mapping`, `button_mapping`, `gamepad_contact`, `gamepad_keyb_focus`, `gamepad_keyb_edit`  
**Side effects:** Memory deallocation (old inverse tables), memory allocation (new inverse tables)  
**Notes:** WHY free old inverse tables before InvertMap: InvertMap allocates new arrays, old arrays must be freed to avoid memory leak. WHY break UI state: prevents dangling references to old mapping state.

**Algorithm:**
1. Break any UI interaction (reset contact, keyb state)
2. Free old inverse tables (axis_mapping, button_mapping)
3. Copy 256 bytes from map to gamepad_mapping[]
4. Rebuild inverse tables (InvertMap)
5. Set keyb_focus=0 to encourage keyboard navigation

---

### `GetGamePadMapping` (gamepad.cpp:1763-1766)

**Signature:** `const uint8_t* GetGamePadMapping()`  
**Purpose:** Return current mapping pointer for save to disk  
**Called by:** Game code (mapping persistence)  
**Calls:** None  
**Globals read:** `gamepad_connected`, `gamepad_mapping`  
**Globals mutated:** None  
**Side effects:** None  
**Notes:** Returns NULL if no gamepad connected, otherwise returns pointer to gamepad_mapping[256]

---

### `GamePadOpen` (gamepad.cpp:1769-1782)

**Signature:** `void GamePadOpen(void (*close_cb)(void* g), void* g)`  
**Purpose:** Open gamepad config UI, register close callback  
**Called by:** Game/menu code when user enters gamepad config  
**Calls:** None  
**Globals read:** None  
**Globals mutated:** `gamepad_close_cb`, `gamepad_close_g`, `gamepad_contact`, `gamepad_keyb_focus`, `gamepad_keyb_edit`  
**Side effects:** None  
**Notes:** Sets keyb_focus=0 to show focus as encouragement for using keyboard navigation. Close callback invoked when user presses quit button or 'Q' key.

---

### `PaintGamePad` (gamepad.cpp:966-1571)

**Signature:** `void PaintGamePad(AnsiCell* ptr, int width, int height, uint64_t stamp)`  
**Purpose:** Render visual gamepad configuration UI  
**Called by:** `game.cpp:7792`, `mainmenu.cpp:1725`  
**Calls:** `CalcLayout`, `BlitSprite`, `BlitButton`  
**Globals read:** `gamepad_sprite`, all visual state globals, all mapping globals, `gamepad_button_output`, `gamepad_axis_output`  
**Globals mutated:** `gamepad_layout_x`, `gamepad_layout_y`, `gamepad_input_xy`  
**Side effects:** Writes to framebuffer (ptr)  
**Notes:** WHY calculate layout: adapts to screen size, fits all inputs on screen. WHY track input_xy: needed for drag-drop hit detection. WHY overlay elements: buttons/triggers show pressed state via conditional rendering. WHY animate swap: smooth transition between Xbox/PS5 layouts.

**Algorithm:**
1. Calculate optimal layout (CalcLayout): fits inputs, minimizes aspect ratio error
2. Compute upper-left corner (gamepad_layout_x, gamepad_layout_y)
3. Blit gamepad sprite elements (body, handles, sticks, buttons, triggers)
   - Skip overlay elements if button not pressed (threshold check)
   - Apply stick offset based on gamepad_axis_output (visual feedback)
   - Animate layout swap if gamepad_swap_stamp != 0
4. Blit axis input slots (render axis visual, slot box, mapping label)
5. Blit button input slots (render button visual, slot box, mapping label)
6. Record input_xy coordinates for hit detection
7. Paint UI buttons (clear, init, quit) with highlight on contact
8. Paint keyboard focus highlight if gamepad_keyb_focus != 0xFF
9. Paint edit cursor if gamepad_keyb_edit != 0xFF (blinking cursor)
10. Paint drag ghost if gamepad_contact >= 0 (dragged output name)

---

### `BlitButton` (gamepad.cpp:1573-1631)

**Signature:** `static void BlitButton(AnsiCell* ptr, int width, int height, int x, int y, int w, int h, int b, int col, int row, int row_y, const int col_x[])`  
**Purpose:** Render single button input slot with visual state and mapping label  
**Called by:** `PaintGamePad`  
**Calls:** `BlitSprite`  
**Globals read:** `gamepad_sprite`, `gamepad_button`, `gamepad_mapping`, `gamepad_axes`, `gamepad_button_name`, `gamepad_half_axis_name`  
**Globals mutated:** `gamepad_input_xy`  
**Side effects:** Writes to framebuffer (ptr)  
**Notes:** WHY index calculation: button visual state (0-8) derived from button pressure (0-32767). WHY record input_xy: needed for drag-drop hit detection.

**Algorithm:**
1. Fetch sprite frame
2. Calculate button visual state index (0-8) from gamepad_button[b]
3. Calculate destination position (col_x[col], row_y + row*3)
4. Blit button visual from button_proto[i]
5. Draw button number label (two digits)
6. Blit slot box next to button visual
7. Record slot position in gamepad_input_xy[2*(2*gamepad_axes+b)]
8. If mapped (not 0xFC, not 0xFF): draw mapping label (two-character output name)

---

### `CalcLayout` (gamepad.cpp:1633-1718)

**Signature:** `static bool CalcLayout(int width, int height, int layout[])`  
**Purpose:** Calculate optimal layout fitting all inputs on screen  
**Called by:** `PaintGamePad`  
**Calls:** None  
**Globals read:** `gamepad_axes`, `gamepad_buttons`  
**Globals mutated:** None  
**Side effects:** None  
**Notes:** WHY aspect ratio optimization: finds layout (ec, er) closest to screen aspect ratio while fitting all inputs. WHY ec (extra columns): buttons can be placed in multiple columns. WHY er (extra rows): vertical expansion for remaining inputs. WHY rows_per_ec=6: fixed row count per column.

**Algorithm:**
1. Iterate ec (extra columns) from 0 to 4
2. For each ec, calculate required er (extra rows): `er = (N - 6*ec + roundup) / (3+ec)`
3. Calculate layout dimensions (dw, dh)
4. Calculate aspect ratio error vs. screen aspect ratio
5. Track best layout: minimize error, prefer fitting on screen
6. Return layout[5]: `{best_ec, best_er, best_dw, best_dh, rows_per_ec}`
7. Return true if fits on screen, false otherwise

---

### `GamePadContact` (gamepad.cpp:1795-2048)

**Signature:** `void GamePadContact(int id, int ev, int x, int y, uint64_t stamp)`  
**Purpose:** Handle mouse/touch input for drag-drop mapping  
**Called by:** `game.cpp:9776,10010`, `mainmenu.cpp:1937,1962`  
**Calls:** `InvertMap`, `Swap`  
**Globals read:** All mapping globals, all visual position globals, `gamepad_swap_stamp`, `gamepad_keyb_edit`, `gamepad_close_cb`, `gamepad_close_g`  
**Globals mutated:** `gamepad_contact`, `gamepad_contact_from`, `gamepad_contact_pos`, `gamepad_contact_output`, `gamepad_contact_ui`, `gamepad_mapping`, `gamepad_keyb_focus`  
**Side effects:** Invokes close callback on quit, rebuilds inverse tables on mapping change  
**Notes:** WHY contact state machine: tracks begin/move/end/cancel across mouse and multi-touch. WHY sqrdist thresholds: allows "snap to nearest" when dropping. WHY check UI buttons first: prevents drag-drop from conflicting with UI actions.

**Event codes:**
- `ev=0`: Contact begin (mouse down, touch start)
- `ev=1`: Contact move (mouse drag, touch move)
- `ev=2`: Contact end (mouse up, touch end)
- `ev=3`: Contact cancel (touch cancel)

**Algorithm (ev=0, begin):**
1. Check if animating (swap): ignore if gamepad_swap_stamp != 0
2. Check UI buttons (clear, init, quit): set gamepad_contact_ui if hit
3. Check output positions (half-axes, buttons): set gamepad_contact_output if hit
4. Check special axis slots (L-Joy, R-Joy, D-Pad): cycle special mapping if hit
5. Store contact ID, from/pos coordinates

**Algorithm (ev=1, move):**
1. Update gamepad_contact_pos
2. If UI button drag: check if still inside button bounds, cancel if not

**Algorithm (ev=2, end):**
1. If UI button: execute action (clear, init, quit, swap)
   - Clear: memset gamepad_mapping to 0xFF, rebuild inverse
   - Init: copy gamepad_mount_mapping, rebuild inverse
   - Quit: invoke close callback
   - Swap: toggle layout (Xbox ↔ PS5)
2. Else: find nearest input slot (sqrdist <= 2)
3. If click (not drag) and output=0xFF: ignore
4. Apply mapping: `gamepad_mapping[input] = gamepad_contact_output`
5. Rebuild inverse tables (InvertMap)
6. Set gamepad_keyb_focus to input

---

### `GamePadKeyb` (gamepad.cpp:2050-2318)

**Signature:** `void GamePadKeyb(int key, uint64_t stamp)`  
**Purpose:** Handle keyboard input for mapping configuration  
**Called by:** `game.cpp:8011`, `mainmenu.cpp:1911`  
**Calls:** `InvertMap`, `Swap`  
**Globals read:** All mapping globals, all keyboard state globals  
**Globals mutated:** `gamepad_keyb_focus`, `gamepad_keyb_edit`, `gamepad_keyb_char`, `gamepad_mapping`  
**Side effects:** Invokes close callback on 'Q', rebuilds inverse tables on mapping change  
**Notes:** WHY two-character input: some outputs have two-character names (e.g., "Ll", "Ls"). WHY modal edit state: separates navigation (arrow keys) from editing (char input). WHY focus on input[0] when focus=0xFF: encourages exploration.

**Key codes:**
- `key=0`: Space (clear/cycle mapping)
- `key=1`: Enter (toggle edit mode)
- `key=2`: Backslash/Escape/Backspace (unused)
- `key=3-6`: Arrow keys (up=3, down=4, left=5, right=6)
- `key='z'/'Z'`: Swap layout (Xbox ↔ PS5)
- `key='c'/'C'`: Clear all mappings
- `key='i'/'I'`: Init (reset to default)
- `key='q'/'Q'`: Quit
- `key>32`: Character input (A-Z, case-insensitive)

**Algorithm (arrow keys 3-6):**
1. If focus=0xFF: set focus=0
2. Get current focus position (ix, iy) from gamepad_input_xy
3. Find nearest input in arrow direction (minimize distance)
4. Update gamepad_keyb_focus

**Algorithm (space):**
1. If focus != 0xFF and not editing:
   - If normal mapping: clear to 0xFF
   - If special mapping (0xFC-0xFE): cycle to next special (0xFE→0xFD→0xFC→0xFF)
2. Rebuild inverse tables

**Algorithm (enter):**
1. If not editing: enter edit mode (gamepad_keyb_edit=0)
2. If editing with 1 char: apply mapping if 'l' or 'r' (Ls, Rs), exit edit mode
3. If editing with 0 chars: exit edit mode

**Algorithm (char input):**
1. Convert to lowercase
2. Match against gamepad_half_axis_name and gamepad_button_name
3. If one-character match (A,B,X,Y,E,G,F): apply immediately, exit edit mode
4. If two-character match first char (L,R,D): store char, advance to edit=1
5. If two-character match second char: apply mapping, exit edit mode
6. Rebuild inverse tables, update gamepad_mapping[gamepad_keyb_focus]

---

### `Swap` (gamepad.cpp:398-413)

**Signature:** `static void Swap(uint64_t stamp)`  
**Purpose:** Toggle between Xbox and PS5 layouts with animation  
**Called by:** `GamePadContact`, `GamePadKeyb`  
**Calls:** None  
**Globals read:** `gamepad_assembly`  
**Globals mutated:** `gamepad_assembly`, `gamepad_half_axis_xy`, `gamepad_button_xy`, `gamepad_swap_stamp`  
**Side effects:** None  
**Notes:** WHY animation: smooth visual transition between layouts. WHY toggle: XOR gamepad_assembly with 1. WHY pointer swap: gamepad_half_axis_xy and gamepad_button_xy point to different const tables.

**Algorithm:**
1. XOR gamepad_assembly with 1 (0→1, 1→0)
2. Update gamepad_half_axis_xy pointer (xbox_half_axis_xy or ps5_half_axis_xy)
3. Update gamepad_button_xy pointer (xbox_button_xy or ps5_button_xy)
4. Store animation start timestamp (gamepad_swap_stamp = stamp)
5. Animation duration: 16 weight steps * 65536 ticks/step ≈ 1 second at 60fps

---

## Mapping Encoding

### Output Index Encoding

**Axes (bit 7 = 0):**
- Bit 6: Polarity (0 = positive, 1 = negative)
- Bits 0-5: Axis index (0-5)
- Example: `0x00` = L-stick right (axis 0, positive)
- Example: `0x40` = L-stick left (axis 0, negative)

**Buttons (bit 7 = 1):**
- Bit 6: Unused
- Bits 0-5: Button index (0-14)
- Example: `0x80` = Button A (button 0)
- Example: `0x8F` = Button Dr (button 14)

**Special mappings (0xFC-0xFE):**
- `0xFE`: L-Joy (maps entire axis to left stick X/Y)
- `0xFD`: R-Joy (maps entire axis to right stick X/Y)
- `0xFC`: D-Pad (maps entire axis to D-pad buttons)
- `0xFF`: Unmapped

### Input Index Encoding

**Axes (0 to 2*gamepad_axes-1):**
- Each axis consumes TWO indices: negative half (2*a), positive half (2*a+1)
- Example: 6 axes → indices 0-11
  - Axis 0 negative: index 0
  - Axis 0 positive: index 1
  - Axis 1 negative: index 2
  - Axis 1 positive: index 3
  - ...

**Buttons (2*gamepad_axes to 2*gamepad_axes+gamepad_buttons-1):**
- Each button consumes ONE index
- Example: 6 axes, 15 buttons → indices 12-26
  - Button 0: index 12
  - Button 1: index 13
  - ...

---

## Inverse Map Data Structure

### Variable-Length 0xFF-Terminated Lists

Each output has a **dynamically allocated** array of input indices that map to it. Arrays are 0xFF-terminated (no separate length field).

**Example:**
```c
// Output button A (index 0) has inputs 5, 12 mapped to it:
button_mapping[0] = malloc(3); // 2 inputs + 1 sentinel
button_mapping[0][0] = 5;      // SDL button 5 → button A
button_mapping[0][1] = 12;     // SDL button 12 → button A
button_mapping[0][2] = 0xFF;   // sentinel

// Output axis L-stick-X (index 0) has input 0 mapped to it:
axis_mapping[0] = malloc(2);   // 1 input + 1 sentinel
axis_mapping[0][0] = 0;        // Axis 0 positive half → L-stick right
axis_mapping[0][1] = 0xFF;     // sentinel
```

**WHY 0xFF sentinel:**
- Variable length: different outputs may have 0, 1, 2, or 10+ inputs mapped
- No separate length storage: simplifies iteration (`while (*dep != 0xFF)`)
- 0xFF is invalid output index (valid range 0-20)

---

## Integration Points

### Platform Backend (SDL)

**Called by gamepad.cpp:**
- None (gamepad.cpp is platform-agnostic)

**Calls to gamepad.cpp:**
- `LoadGamePad()` — game initialization
- `ConnectGamePad(name, axes, buttons, mapping)` — SDL_CONTROLLERDEVICEADDED event
- `DisconnectGamePad()` — SDL_CONTROLLERDEVICEREMOVED event
- `UpdateGamePadAxis(a, pos, out)` — SDL_CONTROLLERAXISMOTION event
- `UpdateGamePadButton(b, pos, out)` — SDL_CONTROLLERBUTTONDOWN/UP event

### Game Loop (game.cpp, mainmenu.cpp)

**Reads from gamepad.cpp:**
- `gamepad_axis_output[6]` — mapped axis states
- `gamepad_button_output[15]` — mapped button states

**Calls to gamepad.cpp:**
- `PaintGamePad(ptr, width, height, stamp)` — render config UI
- `GamePadContact(id, ev, x, y, stamp)` — mouse/touch input
- `GamePadKeyb(key, stamp)` — keyboard input
- `SetGamePadMapping(map)` — load mapping from disk
- `GetGamePadMapping()` — save mapping to disk
- `GamePadOpen(close_cb, g)` — enter config UI

---

## Memory Management

### Inverse Table Lifecycle

**Allocation:** `InvertMap` (called on connect, mapping change)
- `malloc(length+1)` for each output that has ≥1 input mapped
- NULL for outputs with 0 inputs mapped

**Deallocation:** `SetGamePadMapping`, `DisconnectGamePad`
- `free(axis_mapping[i])` for each non-NULL pointer
- `free(button_mapping[i])` for each non-NULL pointer
- Set pointers to NULL after freeing

**WHY rebuild from scratch:**
- Simpler than incremental update (no old→new delta tracking)
- Safer (no risk of stale pointers or incomplete updates)
- Fast enough (256 iterations < 1ms for human interaction)

---

## Visual Layout System

### Layout Algorithm

**Goal:** Fit all inputs on screen while minimizing aspect ratio error.

**Parameters:**
- `ec` (extra columns): 0-4, number of button columns beyond base 3
- `er` (extra rows): calculated from total inputs and ec
- `rows_per_ec`: 6 (fixed row count per column)

**Dimensions:**
- `dw = 43 + (ec ? 16 + 13*(ec-1) : 0)`
- `dh = (er ? 26 + 3*er : 24) - (8-rows_per_ec)*3`

**Optimal layout:**
1. Iterate ec from 0 to 4
2. Calculate er for each ec
3. Calculate aspect ratio: `log(dw/dh)` vs. `log(width/height)`
4. Track best (ec, er) with minimum aspect ratio error
5. Prefer layouts that fit on screen (dw≤width && dh≤height)

### Coordinate Systems

**Sprite space (top-down):**
- Origin: top-left of sprite
- Y-axis: top to bottom (0 = top, 26 = bottom)
- Used by: gamepad_proto[], ui_proto[], axis_proto[], button_proto[]

**Screen space (bottom-up):**
- Origin: bottom-left of screen
- Y-axis: bottom to top (0 = bottom, height-1 = top)
- Used by: AnsiCell* ptr, PaintGamePad rendering

**Conversion:**
- Screen Y = `gamepad_layout_y - sprite_y - (sprite_h - 1)`
- Sprite Y = `gamepad_layout_y - screen_y`

---

## Special Mappings (0xFC-0xFE)

### L-Joy (0xFE)

Maps a single POV-hat axis to left joystick X/Y.

**Encoding:** Both halves set to 0xFE (`gamepad_mapping[2*a] = gamepad_mapping[2*a+1] = 0xFE`)

**Decoding (in UpdateAxisOutput):**
1. Retrieve raw hat angle from `gamepad_axis[in]`
2. Apply chromium bug workaround: `hat = (hat+32767)*7/8 - 32767`
3. Convert to radians: `fa = hat * PI / 32767`
4. For left stick X (axis 0): `accum -= 32767*sin(fa)`
5. For left stick Y (axis 1): `accum += 32767*cos(fa)`

### R-Joy (0xFD)

Maps a single POV-hat axis to right joystick X/Y.

**Encoding:** `gamepad_mapping[2*a] = gamepad_mapping[2*a+1] = 0xFD`

**Decoding:** Same as L-Joy but for axes 2-3 (right stick).

### D-Pad (0xFC)

Maps a single POV-hat axis to D-pad buttons (Du,Dd,Dl,Dr).

**Encoding:** `gamepad_mapping[2*a] = gamepad_mapping[2*a+1] = 0xFC`

**Decoding (in UpdateButtonOutput):**
1. Retrieve raw hat angle from `gamepad_axis[in]`
2. Apply chromium bug workaround
3. Convert to radians
4. For each D-pad button (11-14):
   - Du (11): `delta = 32767*cos(fa)`, accumulate if delta < 0
   - Dd (12): `delta = 32767*cos(fa)`, accumulate if delta > 0
   - Dl (13): `delta = -32767*sin(fa)`, accumulate if delta < 0
   - Dr (14): `delta = -32767*sin(fa)`, accumulate if delta > 0

---

## Chromium Bug Workaround

**Location:** `UpdateAxisOutput` (line 495), `UpdateButtonOutput` (line 562)

**Issue:** Chromium WebAssembly gamepad API reports hat values with incorrect circular deadzone.

**Workaround:** Rescale hat value before trigonometric conversion:
```c
hat = (hat+32767)*7/8 - 32767;
```

**Effect:** Compensates for incorrect deadzone, restores accurate directional decoding.

---

## Auto-Focus System

**Trigger:** Physical gamepad input crosses threshold (±16384 for axes, +16384 for buttons).

**Purpose:** Provides visual feedback showing which input is active.

**Implementation:**
- `UpdateGamePadAxis` (line 631-639): Focus axis half when crossing ±16384
- `UpdateGamePadButton` (line 720-723): Focus button when crossing +16384

**WHY threshold:** Prevents noise/drift from triggering focus, requires intentional user input.

**WHY disabled during edit:** Prevents auto-focus from interrupting keyboard mapping workflow (`if (gamepad_keyb_edit == 0xFF)`).

---

## Animation System

### Layout Swap Animation

**Trigger:** User presses 'Z' key or clicks swap button.

**Duration:** 16 weight steps × 65536 ticks/step ≈ 1.05 seconds at 1MHz tick rate.

**Implementation (Swap):**
1. Toggle `gamepad_assembly` (0↔1)
2. Update pointer tables (`gamepad_half_axis_xy`, `gamepad_button_xy`)
3. Store `gamepad_swap_stamp = stamp`

**Implementation (PaintGamePad):**
1. Check `gamepad_swap_stamp != 0`
2. Calculate weight: `weight = (stamp - gamepad_swap_stamp) >> 14` (range 0-16)
3. Interpolate position: `dx = (dx * weight + dx0 * (16-weight)) / 16`
4. Clear stamp when weight ≥ 16 (animation complete)

**WHY interpolation:** Smooth visual transition between Xbox/PS5 button positions.

**WHY swap_mask:** Only specific elements animate (buttons, sticks, D-pad, labels), not body/handles.

### Edit Cursor Blink

**Period:** 500,000 microseconds (0.5 seconds).

**Implementation (PaintGamePad line 1506-1513):**
1. If `stamp - blink_stamp > 500000`: reset `blink_stamp = stamp`
2. Show cursor if `stamp - blink_stamp < 250000` (first half of period)
3. Hide cursor if `stamp - blink_stamp ≥ 250000` (second half of period)

**Visual:** Full-block glyph (219) at cursor position.

---

## Drag-Drop Protocol

### State Machine

**State 0 (no contact):** `gamepad_contact = -1`
- Waiting for ev=0 (begin)

**State 1 (dragging):** `gamepad_contact = 0` (mouse) or `1+` (touch ID)
- Tracking ev=1 (move), waiting for ev=2 (end) or ev=3 (cancel)

**State 2 (dropped):** Process mapping change, return to state 0

### Hit Detection

**UI buttons:** Rectangular bounds check (ui_proto[i].src_x/y/w/h).

**Output positions:** Squared distance ≤ 2 (gamepad_half_axis_xy[], gamepad_button_xy[]).

**Input slots:** Squared distance ≤ 2 (gamepad_input_xy[]).

**WHY squared distance:** Avoids sqrt call, threshold 2 ≈ 1.4 cell radius.

### Snap-to-Nearest

When dropping output onto inputs (ev=2):
1. Find input with minimum squared distance to drop position
2. If sqrdist ≤ 2: apply mapping
3. Else: ignore drop (no mapping change)

**WHY snap:** Allows imprecise drop (don't need pixel-perfect alignment).

---

## Keyboard Mapping Workflow

### One-Character Outputs (A, B, X, Y, E, G, F)

**User flow:**
1. Focus input slot (arrow keys or physical gamepad input)
2. Press character key (e.g., 'A')
3. Mapping applied immediately

**Implementation (GamePadKeyb line 2180):**
- Match character against gamepad_button_name
- If one-character name: apply mapping, exit edit mode

### Two-Character Outputs (Ll, Lr, Lu, Ld, Rl, Rr, Ru, Rd, Lt, Rt, Ls, Rs, Du, Dd, Dl, Dr)

**User flow:**
1. Focus input slot
2. Press first character (e.g., 'L')
3. Enter edit mode (gamepad_keyb_edit=1)
4. Press second character (e.g., 'l')
5. Mapping applied

**Implementation (GamePadKeyb line 2204-2208):**
- First char: store in gamepad_keyb_char[0], advance to edit=1
- Second char: match both chars, apply mapping, exit edit mode

**Special case (Ls, Rs):**
- After entering 'L' or 'R', pressing Enter applies Ls or Rs immediately
- Implementation (line 2124-2140)

---

## Output Event Format

### Axis Event

**Format:** `uint32_t = (value&0xFFFF) | (axis_index<<24) | (0<<16)`
- Bits 0-15: Axis value (signed -32767 to +32767, truncated to 16 bits)
- Bit 16: Type flag (0 = axis)
- Bits 24-31: Axis index (0-5)

**Example:** Left stick right = 16384
- `out = (16384 & 0xFFFF) | (0 << 24) | (0 << 16) = 0x00004000`

### Button Event

**Format:** `uint32_t = (value&0xFFFF) | (button_index<<24) | (1<<16)`
- Bits 0-15: Button value (unsigned 0 to +32767, truncated to 16 bits)
- Bit 16: Type flag (1 = button)
- Bits 24-31: Button index (0-14)

**Example:** Button A pressed = 32767
- `out = (32767 & 0xFFFF) | (0 << 24) | (1 << 16) = 0x00017FFF`

---

## Data Contract: Mapping File

**Format:** Binary, 256 bytes, no header.

**Structure:**
- Byte 0-255: Input index → Output index
- Value 0x00-0x3F: Axis output (bit 7=0, bit 6=polarity, bits 0-5=axis index)
- Value 0x80-0xBF: Button output (bit 7=1, bits 0-5=button index)
- Value 0xFC-0xFE: Special mappings (L-Joy, R-Joy, D-Pad)
- Value 0xFF: Unmapped

**Endianness:** N/A (byte array, no multi-byte values).

**Persistence:** Loaded via `SetGamePadMapping`, saved via `GetGamePadMapping`.

---

## Summary Statistics

**Total functions:** 12 (8 public, 4 static)
**Total globals:** 30+ (mapping tables, input/output state, visual state, UI state)
**Constant tables:** 13 (layout positions, names, sprite elements)
**Input indices:** 256 (theoretical max, typical: 2*6 + 15 = 27)
**Output indices:** 21 (6 axes as 12 half-axes + 15 buttons, but output arrays are [6] + [15])
**Mapping encoding:** 8-bit (0x00-0xFF)
**Inverse tables:** Variable-length, 0xFF-terminated, dynamically allocated
**Visual layouts:** 2 (Xbox, PS5)
**Animation systems:** 2 (layout swap, edit cursor blink)
**UI interaction modes:** 3 (drag-drop, keyboard navigation, keyboard edit)

---

