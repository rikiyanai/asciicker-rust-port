# Game API Analysis — JavaScript/C++ Bridge Layer

**File:** `game_api.cpp`

**Purpose:** Provides high-performance bidirectional data exchange between JavaScript NPC scripts and C++ game engine via shared memory buffer and callback dispatch system.

---

## Architecture Overview

### Shared Memory Buffer Layout

```
akAPI_Buff (65568 bytes total):
├─ [0..65535]      : Data exchange region (int32/float32/string transfer)
└─ [65536..65567]  : Callback bitfield flags (256 bits / 32 bytes)
```

### Why Shared Buffer

Avoids marshaling overhead for 60+ calls per frame. JavaScript and C++ directly read/write fixed offsets in WASM memory heap. Zero-copy transfers for floats/ints.

### Data Exchange Protocol

- JavaScript writes arguments to buffer offsets → calls `akAPI_Call(id)` → C++ reads buffer
- C++ writes results to buffer offsets → returns → JavaScript reads buffer
- Fixed offsets documented per-function below

### Callback System

- JavaScript registers callbacks via `cb(idx, fnc)` → sets bitfield flag at `akAPI_Buff[65536 + (idx>>3)]`
- C++ checks bitfield via `akAPI_CheckCB(id)` before invoking → O(1) registered check
- Callback slots: 0=onSay, 1=onItem, 2=onFrame

---

## Function Reference

### `akAPI_Init` (game_api.cpp:68-278)

**Signature:** `void akAPI_Init()`

**Purpose:** Initializes the JavaScript API surface (ak object) by executing JavaScript code that defines getPos, setPos, say, jump, callback registration, etc.

**Called by:**
- `game_web.cpp:611` (Web platform initialization)
- `game_app.cpp:1816` (Native V8 platform initialization)

**Calls:**
- `akAPI_Exec()` (platform-specific code execution)

**Globals read:** None

**Globals mutated:** None (directly, but via `akAPI_Exec` modifies global JS context)

**Side effects:**
- Injects JavaScript helper functions into global scope: `akGetF32`, `akSetF32`, `akReadF32`, `akWriteF32`, `akGetI32`, `akSetI32`, `akReadI32`, `akWriteI32`, `akGetStr`, `akSetStr`, `akAPI_Back` (callback registry)
- Creates and freezes immutable `ak` object on JavaScript side
- Registers JavaScript callback dispatcher `akAPI_CB`
- Sets up callback registration function `cb` that manages bitfield flags

**Notes:**
- Runs JavaScript code via `CODE()` macro which wraps a code string
- Called once during platform initialization (either Emscripten or V8)
- The injected `ak` object exposes the full JavaScript API surface:
  - Position/Direction: `getPos/setPos`, `getDir/setDir`, `getYaw/setYaw`
  - Character State: `getName/setName`, `getMount/setMount`, `getAction/setAction`
  - Movement: `getMove/setMove` (velocity vector)
  - Environment: `getWater/setWater`, `getLight/setLight` (RGBA lighting), `getWeather/setWeather` (0-3)
  - Actions: `say(str)`, `jump()`
  - Queries: `isGrounded()`
  - Callbacks: `onSay(fnc)`, `onItem(fnc)`, `onFrame(fnc)`
- Uses `CODE()` macro to convert C++ string literal directly to JavaScript code
- Parameter `-1, true` to `akAPI_Exec()` means auto-detect length, execute in root scope

---

### `akAPI_CheckCB` (game_api.cpp:295-300)

**Signature:** `bool akAPI_CheckCB(int id)`

**Purpose:** Check if JavaScript callback is registered for given ID via O(1) bitfield lookup.

**Called by:**
- `game_api.cpp:322` (within `akAPI_OnSay`)
- `game_api.cpp:376` (within `akAPI_OnItem`)
- `game_api.cpp:419` (within `akAPI_OnFrame`)

**Calls:** None

**Globals read:** `akAPI_Buff`

**Globals mutated:** None

**Side effects:** None (read-only)

**Notes:**
- Callback ID 0-255 maps to bit `(id & 0x7)` in byte at offset `(id >> 3)`
- Byte 0 (`akAPI_Buff[65536]`) holds bits for callbacks 0-7
- Byte 1 (`akAPI_Buff[65537]`) holds bits for callbacks 8-15
- etc. (32 bytes total = 256 bits = 256 possible callbacks)
- Bitfield encoding allows O(1) check: bitwise AND with bitmask
- Current callback slots: 0=onSay, 1=onItem, 2=onFrame (slots 3-255 reserved)

---

### `akAPI_OnSay` (game_api.cpp:318-340)

**Signature:** `bool akAPI_OnSay(const char* str, int len, bool* allowed)`

**Purpose:** C++ → JavaScript callback: Player attempts to speak (chat message). Allows JavaScript to filter/modify chat messages before broadcasting.

**Called by:**
- `game.cpp:8082` (within player chat submission logic)
- `game.cpp:8650` (within player chat submission logic)

**Calls:**
- `akAPI_CheckCB(id)` — check if callback registered
- `strlen(str)` — auto-detect length if len < 0
- `memcpy()` — write string to buffer
- `akAPI_CB(id)` — invoke JavaScript handler

**Globals read:** `akAPI_Buff`

**Globals mutated:** `akAPI_Buff` (data region)

**Side effects:**
- Copies string to shared buffer at offset 0 (max 255 chars, null-terminated)
- Invokes JavaScript onSay handler if registered
- Writes result flag to buffer for C++ to read

**Notes:**
- BUFFER PROTOCOL (INPUT):
  - C++ writes: `str` at offset 0 (max 255 chars, null-terminated)
- BUFFER PROTOCOL (OUTPUT):
  - JavaScript writes: `int32` at offset 0 (0=block, 1=allow)
- Parameters:
  - `str`: Chat message string (null-terminated)
  - `len`: String length (-1 = auto-detect via strlen, >255 = clamp to 255)
  - `allowed`: [out] true if JavaScript allowed message, false if blocked
- Returns: true if callback was invoked, false if no onSay handler registered
- Early return false if callback not registered (avoids buffer copy)

---

### `akAPI_OnItem` (game_api.cpp:372-403)

**Signature:** `bool akAPI_OnItem(int action, int story_id, int kind, int subkind, int weight, const char* desc, bool* allowed, int* out_story_id, const char** out_desc)`

**Purpose:** C++ → JavaScript callback: Player interacts with inventory item. Allows JavaScript to handle custom item logic (consume, modify, transform items).

**Called by:**
- `game.cpp:4303` (inventory pickup)
- `game.cpp:4753` (inventory drop)
- `game.cpp:4825` (inventory use)
- `game.cpp:9602` (inventory interaction)

**Calls:**
- `akAPI_CheckCB(id)` — check if callback registered
- `strlen(desc)` — measure description length
- `memcpy()` — write item data to buffer

**Globals read:** `akAPI_Buff`

**Globals mutated:** `akAPI_Buff` (data region)

**Side effects:**
- Copies item metadata (action, IDs, kind, subkind, weight, description) to shared buffer
- Invokes JavaScript onItem handler if registered
- Parses JavaScript return value and writes modifications back to C++ parameters

**Notes:**
- BUFFER PROTOCOL (INPUT):
  - `int32[0]` = action (pickup, drop, use, etc.)
  - `int32[1]` = story_id (unique item ID in world)
  - `int32[2]` = kind (item category)
  - `int32[3]` = subkind (item variant)
  - `int32[4]` = weight
  - `char[20..51]` = desc (max 31 chars, null-terminated)
- BUFFER PROTOCOL (OUTPUT):
  - `int32[0]` = 0 (block) or 1 (allow)
  - `int32[4]` = modified story_id (if changed)
  - `char[8..]` = modified desc (if changed)
- JavaScript can return multiple types and C++ parses:
  - bool (allow/block)
  - int (new story_id)
  - string (new desc)
  - array `[int, string]` (both story_id and desc)
  - object `{story_id, desc}` (named fields)
- Returns: true if callback was invoked, false if no onItem handler registered
- Default outputs preserve original values if callback not invoked or JavaScript returns blocking value
- Description clamped to 31 chars

---

### `akAPI_OnFrame` (game_api.cpp:416-424)

**Signature:** `bool akAPI_OnFrame()`

**Purpose:** C++ → JavaScript callback: Per-frame update tick. Allows JavaScript to run NPC AI logic every frame (60 Hz).

**Called by:**
- `game.cpp:6959` (within main game update loop)

**Calls:**
- `akAPI_CheckCB(id)` — check if callback registered
- `akAPI_CB(id)` — invoke JavaScript handler

**Globals read:** `akAPI_Buff`

**Globals mutated:** None

**Side effects:** Invokes JavaScript onFrame handler if registered

**Notes:**
- BUFFER PROTOCOL: No arguments or return values
- Typical use: NPC behavior trees, pathfinding updates, state machines
- Called every frame during active NPC simulation
- Returns: true if callback was invoked, false if no onFrame handler registered
- Early return false if callback not registered (avoids unnecessary call)

---

### `akAPI_Free` (game_api.cpp:430-434)

**Signature:** `void akAPI_Free()`

**Purpose:** Cleanup shared buffer allocation (stub).

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (commented out, buffer deallocation handled by platform)

**Notes:**
- Not implemented: akAPI_Buff allocated by platform-specific code
  - `game_app.cpp`: V8 external memory (ArrayBuffer)
  - `game_web.cpp`: Emscripten malloc
- Platform owns deallocation responsibility
- Code is commented as placeholder for future use

---

### `akAPI_Call` (game_api.cpp:483-685)

**Signature:** `extern "C" void akAPI_Call(int id)`

**Purpose:** Command Dispatcher — JavaScript → C++ API Entry Point. Single dispatch function reduces WASM import overhead vs 20+ individual exports.

**Called by:** JavaScript via WASM (exported from Emscripten build)

**Calls:**
- `SetPhysicsPos()` (case 1)
- `SetPhysicsDir()` (case 3)
- `SetPhysicsYaw()` (case 5)
- `ConvertToCP437()` (case 7)
- `SetMount()` (case 9)
- `SetActionNone/SetActionAttack/SetActionFall/SetActionDead/SetActionStand()` (case 11)
- `GetWeather()` (case 18)
- `SetWeather()` (case 19)
- `Say()` (case 100)

**Globals read:**
- `game` (main game state pointer)
- `game->main_menu` (menu active flag)
- `game->player.pos` (player position)
- `game->player.dir` (player direction)
- `game->prev_yaw` (previous yaw angle)
- `game->player.name` (player name)
- `game->player.req.mount` (mount ID)
- `game->player.req.action` (action state)
- `game->input.api_move` (API-controlled movement)
- `game->water` (water level)
- `game->light` (RGBA lighting)
- `game->stamp` (game timestamp)
- `game->input.jump` (jump input flag)
- `game->prev_grounded` (grounded state)

**Globals mutated:**
- `game->player.pos` (case 1)
- `game->player.dir` (case 3)
- `game->player.name` (case 7)
- `game->player.name_cp437` (case 7)
- `game->player.req.mount` (case 9)
- `game->player.req.action` (case 11)
- `game->input.api_move` (case 13)
- `game->water` (case 15)
- `game->light` (case 17)
- `game->input.jump` (case 101)

**Side effects:**
- Physics state changes (position, rotation, movement)
- Character state changes (name, mount, action, animations)
- Environment changes (water, light, weather)
- Input simulation (jump trigger)
- NPC AI callbacks via `Say()`

**Notes:**
- Early return if `game == NULL` or `game->main_menu` is true (API blocked during menu)
- Dispatch table with 20 getter/setter pairs (IDs 0-19), 2 action calls (100-101), 1 query (200)
- BUFFER PROTOCOL (all data exchanged via `akAPI_Buff`):
  - float[3] at buf[0]: 3D position (x, y, z) or movement vector (vx, vy, vz)
  - float[4] at buf[0]: RGBA color (r, g, b, a)
  - float at buf[0]: Single scalar (direction angle, water level, yaw)
  - int32 at buf[0]: Integer flags (mount ID, action enum, grounded boolean)
  - string at buf[0]: Null-terminated UTF-8 text (player name, chat message)
- Case 1 (setPos): Uses `SetPhysicsPos()` to teleport, bypassing collision (for scripted cutscenes, respawn)
- Case 7 (setName): Converts UTF-8 to CP437 encoding for terminal rendering
- Case 11 (setAction): Dispatches to specific `SetAction*()` function per ACTION enum, passes `game->stamp` for animation sync
- Case 100 (say): Calls `player.Say()` with string and length to generate chat bubble/dialogue
- Case 101 (jump): Sets input flag for physics system to apply jump impulse next frame
- Case 14/15 (water): Note in source: comment says "getWater : function() { akAPI_Call(13)..." but dispatches on ID 14 (off-by-one in JS comment)
- Returns void; all I/O via shared buffer

---

## Dispatch Table Reference

| ID | JavaScript API | C++ Action | Buffer Input | Buffer Output |
|----|----|----|----|-----|
| 0 | `ak.getPos(arr3, ofs)` | Read `game->player.pos` → write float[3] to buf[0] | None | float[3] position |
| 1 | `ak.setPos(arr3, ofs)` | Read float[3] from buf[0] → `SetPhysicsPos()` | float[3] position | None |
| 2 | `ak.getDir()` | Read `game->player.dir` → write float to buf[0] | None | float direction angle |
| 3 | `ak.setDir(flt)` | Read float from buf[0] → `SetPhysicsDir()` | float direction | None |
| 4 | `ak.getYaw()` | Read `game->prev_yaw` → write float to buf[0] | None | float yaw angle |
| 5 | `ak.setYaw(flt)` | Read float from buf[0] → `SetPhysicsYaw()` | float yaw | None |
| 6 | `ak.getName()` | Read `game->player.name` → write string to buf[0] | None | string name |
| 7 | `ak.setName(str)` | Read string from buf[0] → set player name, convert to CP437 | string name | None |
| 8 | `ak.getMount()` | Read mount ID → write int32 to buf[0] | None | int32 mount |
| 9 | `ak.setMount(int)` | Read int32 from buf[0] → `SetMount()` | int32 mount | None |
| 10 | `ak.getAction()` | Read action state → write int32 to buf[0] | None | int32 action |
| 11 | `ak.setAction(int)` | Read int32 from buf[0] → dispatch `SetAction*()` | int32 action | None |
| 12 | `ak.getMove(arr3, ofs)` | Read `game->input.api_move` → write float[3] to buf[0] | None | float[3] movement |
| 13 | `ak.setMove(arr3, ofs)` | Read float[3] from buf[0] → set `api_move` | float[3] movement | None |
| 14 | `ak.getWater()` | Read water level → write float to buf[0] | None | float water |
| 15 | `ak.setWater(flt)` | Read float from buf[0] → set `game->water` | float water | None |
| 16 | `ak.getLight(arr4, ofs)` | Read RGBA light → write float[4] to buf[0] | None | float[4] RGBA |
| 17 | `ak.setLight(arr4, ofs)` | Read float[4] from buf[0] → set `game->light` | float[4] RGBA | None |
| 18 | `ak.getWeather()` | Read weather state → write int32 to buf[0] | None | int32 weather |
| 19 | `ak.setWeather(int)` | Read int32 from buf[0] → `SetWeather()` | int32 weather | None |
| 100 | `ak.say(str)` | Read string from buf[0] → `player.Say()` | string message | None |
| 101 | `ak.jump()` | Set `game->input.jump = true` | None | None |
| 200 | `ak.isGrounded()` | Read grounded state → write int32 to buf[0] | None | int32 grounded |

---

---

### `akGetI32` (game_api.cpp:208-211)

**Signature:** `function akGetI32(buf_ofs)` (JavaScript helper)

**Purpose:** Read a 32-bit signed integer from the shared buffer at specified byte offset

**Called by:**
- akAPI_OnItem callback (line 208, 211, 212, 213)
- ak.getMount() wrapper (reads mount ID from buffer[0])
- ak.getAction() wrapper (reads action ID from buffer[0])
- ak.getWeather() wrapper (reads weather state from buffer[0])
- ak.isGrounded() wrapper (reads grounded state from buffer[0])

**Calls:**
- DataView.getInt32() (JavaScript typed array API)

**Globals read:** akAPI_Buff (shared memory buffer)

**Globals mutated:** None (read-only accessor)

**Side effects:** None (pure JavaScript function, reads from shared buffer)

**Notes:** Part of the zero-copy data exchange protocol between JavaScript and C++. Returns little-endian int32 from buffer at given offset. Used by callback handlers and API getters to retrieve results written by C++ handlers.

---

## Data Contract Summary

- **Buffer size:** 65568 bytes (65536 data + 32 callback flags)
- **Access pattern:** Zero-copy direct read/write at fixed offsets
- **Callback registration:** Bitfield encoding (1 bit per slot, 256 slots total)
- **String encoding:** UTF-8 (JavaScript side), CP437 conversion in C++ for terminal rendering
- **Type conversions:** JavaScript Number → int32/float32, boolean → int32 (0/1)
- **Error handling:** API calls blocked during main menu initialization, returns void on invalid state
