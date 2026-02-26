# WebAssembly/Emscripten Platform Implementation Architecture
# Generated: 2026-02-12
# Agent: Handoff/Fix

## Overview

**game_web.cpp** is the platform abstraction layer for the Asciicker game engine compiled to WebAssembly via Emscripten. It bridges C++ game logic to browser APIs (WebGL, WebSocket, Vibration API) and manages the cooperative event-driven main loop required by JavaScript.

**game_web.html** is the Emscripten shell template that provides the WebGL rendering pipeline, input handling, and multiplayer bootstrap.

---

## C++ Functions

### `Buzz` (game_web.cpp:111-132)

**Signature:** `void Buzz()`
**Purpose:** Trigger haptic feedback (gamepad vibration or mobile device vibration).
**Called by:** Exported to JavaScript via `Module.cwrap()`; triggered by game events.
**Calls:** `EM_ASM()`
**Globals read:** None directly
**Globals mutated:** None
**Side effects:** Executes browser Vibration API or gamepad vibrationActuator.
**Notes:** Platform-specific bridge to browser vibration APIs.

### `GetTime` (game_web.cpp:138-143)

**Signature:** `uint64_t GetTime()`
**Purpose:** Get high-resolution monotonic timestamp in microseconds.
**Called by:** `game_web.cpp:539`, `game_web.cpp:577`, `game_web.cpp:809`
**Calls:** `clock_gettime()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Reads system monotonic clock via Emscripten POSIX emulation.
**Notes:** Microsecond precision for frame timing and logic stamping.

### `MakeStamp` (game_web.cpp:147-147)

**Signature:** `uint64_t (*MakeStamp)()`
**Purpose:** Function pointer for timestamp generation.
**Called by:** `mainmenu.cpp:1552`, `1843`
**Calls:** Points to `GetTime`
**Globals read:** None
**Globals mutated:** `MakeStamp` (initialization)
**Side effects:** None
**Notes:** Permits swapping timing source at runtime (e.g. for network sync).

### `SyncConf` (game_web.cpp:154-157)

**Signature:** `void SyncConf()`
**Purpose:** Flush Emscripten virtual filesystem changes to IndexedDB.
**Called by:** `mainmenu.cpp` and `game.cpp` via extern.
**Calls:** `EM_ASM()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Triggers asynchronous IndexedDB write.
**Notes:** Essential for preserving config changes across browser sessions.

### `GetConfPath` (game_web.cpp:162-165)

**Signature:** `const char* GetConfPath()`
**Purpose:** Return the path to the game's configuration file.
**Called by:** `mainmenu.cpp` and `game.cpp` via extern.
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns `/data/asciicker.cfg` (IndexedDB-backed virtual mount).

### `GetMaterialArr` (game_web.cpp:220-223)

**Signature:** `void* GetMaterialArr()`
**Purpose:** Return pointer to the global material array.
**Called by:** `render.cpp:3193`, `asciiid.cpp:855`, etc.
**Calls:** None
**Globals read:** `mat[256]`
**Globals mutated:** None
**Side effects:** None
**Notes:** Exposes the 256-entry material table to the renderer.

### `InitMaterials` (game_web.cpp:228-439)

**Signature:** `void InitMaterials()`
**Purpose:** Initialize all 256 terrain material definitions.
**Called by:** `game_web.cpp:655`
**Calls:** None
**Globals read:** None
**Globals mutated:** `mat[256]`
**Side effects:** Fills global material table with colors and glyphs.
**Notes:** Hand-coded for indices 0-8, randomized for 9-255.

### `PrevGLFont` (game_web.cpp:443-446)

**Signature:** `bool PrevGLFont()`
**Purpose:** Decrease font size (zoom out).
**Called by:** `mainmenu.cpp:2008`
**Calls:** `EM_ASM_INT()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Triggers browser-side `ZoomOut()`.
**Notes:** Font management is handled in the JavaScript layer.

### `NextGLFont` (game_web.cpp:450-453)

**Signature:** `bool NextGLFont()`
**Purpose:** Increase font size (zoom in).
**Called by:** `mainmenu.cpp:2000`
**Calls:** `EM_ASM_INT()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Triggers browser-side `ZoomIn()`.
**Notes:** Clamps to maximum available font size.

### `exit_handler` (game_web.cpp:457-481)

**Signature:** `void exit_handler(int signum)`
**Purpose:** Handle game exit request.
**Called by:** `mainmenu.cpp:2122`
**Calls:** `EM_ASM()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Attempts to close window or navigate back.
**Notes:** Uses browser APIs to handle page navigation or closure.

### `ToggleFullscreen` (game_web.cpp:485-504)

**Signature:** `void ToggleFullscreen(Game* g)`
**Purpose:** Enter or exit fullscreen mode.
**Called by:** `mainmenu.cpp:2019`
**Calls:** `EM_ASM()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Executes browser Fullscreen API.
**Notes:** Requires user gesture for successful execution.

### `IsFullscreen` (game_web.cpp:507-525)

**Signature:** `bool IsFullscreen(Game* g)`
**Purpose:** Check current fullscreen status.
**Called by:** `mainmenu.cpp:2018`, `2024`, `2034`
**Calls:** `EM_ASM_INT()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Queries `document.fullscreenElement`.

### `main` (game_web.cpp:529-532)

**Signature:** `int main(int argc, char* argv[])`
**Purpose:** Entry point stub (not used in web build).
**Called by:** None
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Cooperative loop mode via `emscripten_set_main_loop` bypasses `main`.

### `akAPI_Exec` (game_web.cpp:537-579)

**Signature:** `void akAPI_Exec(const char* str, int len, bool root)`
**Purpose:** Execute JavaScript code from C++.
**Called by:** `game_api.cpp:70`, `game_app.cpp:1702`
**Calls:** `EM_ASM()`, `GetTime()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Executes JS in browser context; potential for sandbox escapes if not careful.
**Notes:** Supports both root and sandboxed execution modes.

### `akAPI_CB` (game_web.cpp:582-588)

**Signature:** `void akAPI_CB(int id)`
**Purpose:** Trigger JavaScript callback by ID.
**Called by:** `game_api.cpp:334`, `393`, `422`
**Calls:** `EM_ASM()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Invokes JS `akAPI_CB` function.
**Notes:** Decouples C++ events from JS script handlers.

### `Main` (game_web.cpp:592-777)

**Signature:** `int Main()`
**Purpose:** Primary initialization (called after Emscripten load).
**Called by:** `game_web.cpp:791`
**Calls:** `malloc()`, `EM_ASM()`, `akAPI_Init()`, `InitMaterials()`, `LoadSprites()`, `CreateGame()`
**Globals read:** None
**Globals mutated:** `akAPI_Buff`, `render_buf`, `game`
**Side effects:** Allocates major buffers and initializes all engine subsystems.
**Notes:** This is the actual entry point for the web platform.

### `Load` (game_web.cpp:783-792)

**Signature:** `void Load(const char* name)`
**Purpose:** Initialize game with player identity.
**Called by:** `Connect()` (JS)
**Calls:** `Main()`
**Globals read:** None
**Globals mutated:** `player_name`, `player_name_cp437`
**Side effects:** Triggers full game initialization via `Main()`.
**Notes:** Conversion of name to CP437 ensures compatible rendering.

### `Render` (game_web.cpp:796-831)

**Signature:** `void* Render(int width, int height)`
**Purpose:** Execute frame rendering.
**Called by:** `AsciickerLoop()` (JS) at 60 FPS.
**Calls:** `game->Render()`, `GetTime()`
**Globals read:** `game`, `render_buf`
**Globals mutated:** `render_buf`
**Side effects:** Fills render buffer with AnsiCell data for WebGL upload.
**Notes:** Returns pointer to WASM memory used as a GL texture source.

### `Size` (game_web.cpp:835-839)

**Signature:** `void Size(int w, int h, int fw, int fh)`
**Purpose:** Handle viewport resize events.
**Called by:** `Resize()` (JS)
**Calls:** `game->OnSize()`
**Globals read:** `game`
**Globals mutated:** None
**Side effects:** Updates game camera and projection state.
**Notes:** `fw`/`fh` are physical font cell dimensions.

### `Keyb` (game_web.cpp:843-847)

**Signature:** `void Keyb(int type, int val)`
**Purpose:** Handle keyboard input.
**Called by:** JS keyboard handlers
**Calls:** `game->OnKeyb()`
**Globals read:** `game`
**Globals mutated:** None
**Side effects:** Updates engine input state.
**Notes:** Bridges browser `KeyboardEvent` to internal key codes.

### `Mouse` (game_web.cpp:851-855)

**Signature:** `void Mouse(int type, int x, int y)`
**Purpose:** Handle mouse/pointer input.
**Called by:** JS mouse handlers
**Calls:** `game->OnMouse()`
**Globals read:** `game`
**Globals mutated:** None
**Side effects:** Updates engine mouse state.
**Notes:** Coordinates are scaled by DPI ratio.

### `Touch` (game_web.cpp:859-863)

**Signature:** `void Touch(int type, int id, int x, int y)`
**Purpose:** Handle mobile touch events.
**Called by:** JS touch handlers
**Calls:** `game->OnTouch()`
**Globals read:** `game`
**Globals mutated:** None
**Side effects:** Updates multi-touch tracking state.
**Notes:** Supports up to 16 concurrent touch points.

### `GamePad` (game_web.cpp:867-925)

**Signature:** `void GamePad(int ev, int idx, float val)`
**Purpose:** Handle browser Gamepad API events.
**Called by:** `setGamePadHandlers()` (JS)
**Calls:** `GamePadUnmount()`, `GamePadMount()`, `GamePadButton()`, `GamePadAxis()`
**Globals read:** `gamepad`
**Globals mutated:** `gamepad_axes`, `gamepad_buttons`, `gamepad_mapping`
**Side effects:** Configures input mapping and updates controller state.
**Notes:** Normalizes various controller types to a standard layout.

### `Focus` (game_web.cpp:929-933)

**Signature:** `void Focus(int set)`
**Purpose:** Handle window focus/blur.
**Called by:** `setFocusHandlers()` (JS)
**Calls:** `game->OnFocus()`
**Globals read:** `game`
**Globals mutated:** None
**Side effects:** Pauses or resumes game logic.
**Notes:** `set=1` for focus, `set=0` for blur.

### `Join` (game_web.cpp:938-963)

**Signature:** `void* Join(const char* name, int id, int max_cli)`
**Purpose:** Join or leave a multiplayer session.
**Called by:** `Connect()` (JS)
**Calls:** `malloc()`, `free()`
**Globals read:** `server`
**Globals mutated:** `server`
**Side effects:** Allocates or deallocates network session state.
**Notes:** Returns pointer to outgoing packet buffer.

### `Packet` (game_web.cpp:966-972)

**Signature:** `void Packet(const uint8_t* ptr, int size)`
**Purpose:** Process incoming WebSocket packet.
**Called by:** `Connect()` onmessage handler (JS)
**Calls:** `server->Proc()`
**Globals read:** `server`
**Globals mutated:** None
**Side effects:** Updates world state based on network messages.
**Notes:** `ptr` points to binary data from browser WebSocket.

---

## JavaScript Helper Functions (game_web.html)

### JavaScript: `audioResume` (game_web.html:303-312)

**Purpose:** Resume Web Audio context to bypass browser autoplay restrictions.
**Called by:** Input handlers (mouse, key, touch, gamepad)
**Calls:** `audio_ctx.resume()`
**Notes:** Required for sound to play after user interaction.

### JavaScript: `Send` (game_web.html:325-334)

**Purpose:** Send outgoing packet to WebSocket.
**Called by:** C++ `Server::Send()` via `EM_ASM_INT`
**Calls:** `ak_connection.send()`
**Notes:** Reads from WASM memory view.

### JavaScript: `ConsoleLog` (game_web.html:336-342)

**Purpose:** Log UTF-8 string from WASM to browser console.
**Called by:** C++ `Server::Log()`
**Calls:** `console.log()`
**Notes:** Decodes binary buffer as string.

### JavaScript: `Connect` (game_web.html:349-521)

**Purpose:** Bootstrap solo or multiplayer mode.
**Called by:** `StartGame()`
**Calls:** `Load()`, `Resize()`, `Join()`, `Packet()`, `requestAnimationFrame()`
**Notes:** Handles server connection timeout and solo fallback.

### JavaScript: `FindFont` (game_web.html:579-625)

**Purpose:** Select optimal font size for screen dimensions and DPI.
**Called by:** `Resize()`, `ZoomIn()`, `ZoomOut()`
**Calls:** None
**Notes:** Crucial for mobile vs desktop layout adaptation.

### JavaScript: `Resize` (game_web.html:659-697)

**Purpose:** Re-calculate render grid dimensions.
**Called by:** `FindFont()`, window events, zoom actions
**Calls:** `Size()` (C++)
**Notes:** Enforces minimum resolution of 45x36 cells.

### JavaScript: `AsciickerLoop` (game_web.html:707-813)

**Purpose:** Main 60FPS animation loop.
**Called by:** `requestAnimationFrame()`
**Calls:** `PollGamePad()`, `Render()`, WebGL drawing functions
**Notes:** Orchestrates data flow from C++ to WebGL texture.

### JavaScript: `AsciickerBoot` (game_web.html:1470-1511)

**Purpose:** Preload all bitmap font assets.
**Called by:** `Module.onRuntimeInitialized`
**Calls:** `AsciickerInit()` after preloading
**Notes:** Fails boot if essential fonts are missing.

### JavaScript: `AsciickerInit` (game_web.html:1513-1643)

**Purpose:** Final WebGL and bridge setup.
**Called by:** `AsciickerBoot()`
**Calls:** `Module.cwrap()`, WebGL init, input handler installation
**Notes:** Installs mouse, keyboard, touch, and gamepad listeners.

---

### `Server::Send` (game_web.cpp:181-192)

**Signature:** `bool Server::Send(const uint8_t* data, int size)`

**Purpose:** Send binary data over WebSocket via Emscripten JavaScript bridge. Wraps JavaScript `Send()` function exposed to C++ via EM_ASM.

**Called by:**
- Network layer when transmitting game state updates

**Calls:**
- `EM_ASM_INT()` — invoke JavaScript `Send()` function

**Globals read:** None

**Globals mutated:**
- JavaScript WebSocket state (via bridge)

**Side effects:**
- Transmits data over network connection
- May fail if WebSocket not connected

**Notes:**
- Returns success status from JavaScript layer
- Data marshalling handled by Emscripten runtime
- Part of minimal network abstraction for web builds

---

### `Server::Proc` (game_web.cpp:195-198)

**Signature:** `void Server::Proc()`

**Purpose:** Process pending network events (stub for web builds). Web networking is event-driven via JavaScript callbacks, so explicit polling is not needed.

**Called by:**
- Game loop network tick

**Calls:** None (empty implementation)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:**
- No-op for web builds (event-driven model)
- Desktop builds use this for socket polling
- Maintains API parity across platforms

---

### `Server::Log` (game_web.cpp:200-202)

**Signature:** `void Server::Log(const char* str)`

**Purpose:** Output network debug/error messages to browser console via Emscripten.

**Called by:**
- Network layer for connection events, errors, state changes

**Calls:**
- `printf()` — Emscripten redirects to browser console

**Globals read:** None

**Globals mutated:**
- Browser console output

**Side effects:**
- Logs message to browser developer tools console

**Notes:**
- Convenience wrapper for platform-agnostic logging
- Emscripten's `printf` automatically routes to `console.log`
