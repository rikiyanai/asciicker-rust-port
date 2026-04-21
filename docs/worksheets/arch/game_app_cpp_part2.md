# game_app.cpp Analysis - Part 2 (Lines 1883-3765)

**Scope:** Event loop, terminal input parsing, V8 JavaScript integration
**Analysis date:** 2026-02-12
**Line range:** 1883-3765 (second half of main() + V8 subsystem)

---

## Event Loop & Main Game Loop (Lines 1883-3524)

This section continues the `main()` function from Part 1, implementing the Linux/macOS terminal game loop with input handling, rendering, and network processing.

### Main Event Loop (game_app.cpp:2492-3456)

**Signature:** `while(running)` loop body inside `main()`
**Purpose:** Core game loop for terminal mode - polls input devices, updates game state, renders to terminal
**Called by:** `main()` function (starts at line 1689)
**Calls:** `GetTime()`, `GamePadMount()`, `poll()`, `read_js()`, `game->OnKeyb()`, `game->OnMouse()`, `game->OnSize()`, `server->Proc()`, `game->Render()`, `Print()`
**Globals read:** `running`, `xterm_kitty`, `gpm`, `jsfd`, `stamp`, `hold_down`, `hold_deadline`, `mouse_x`, `mouse_y`, `mouse_down`, `wh`, `server`, `game`, `perf_enabled`, `perf_out`
**Globals mutated:** `stamp`, `hold_down`, `hold_deadline`, `mouse_x`, `mouse_y`, `mouse_down`, `wh`, `buf`, `jsfd`, `perf_*` variables, `frames`
**Side effects:** 
- Polls stdin, gamepad, GPM mouse for input events
- Reads and parses ANSI/kitty escape sequences
- Updates held key timeouts (140ms auto-release for terminals without key-up events)
- Reallocates render buffer on terminal resize
- Processes network messages via `server->Proc()`
- Renders game frame to `buf` and prints to stdout
- Logs performance metrics when `ASCIICKER_PROFILE` env set
**Notes:**
- Implements hold-key emulation for terminals without key-up events (140ms deadline)
- Three poll() configurations: GPM+stdin+js, stdin+js, or stdin-only
- Kitty terminal protocol provides proper key up/down events
- SGR (1006) mouse protocol: `CSI < Bc;Px;Py;M` (press) or `m` (release)
- Performance profiling logs avg render/print time and FPS every 1 second

---

### Input Stream Parsing (game_app.cpp:2709-3362)

**Signature:** stdin `read()` + stream buffer parsing inside event loop
**Purpose:** Parses ANSI escape sequences, kitty key protocol, and SGR mouse events from terminal stdin
**Called by:** Main event loop when `pfds[0].revents & POLLIN`
**Calls:** `read()`, `game->OnKeyb()`, `game->OnMouse()`, `fopen()`, `fprintf()` (keylog.txt)
**Globals read:** `stream`, `stream_bytes`, `kbd`, `hold_down`, `hold_deadline`, `stamp`, `game`, `wh`, `mouse_x`, `mouse_y`, `mouse_down`
**Globals mutated:** `stream`, `stream_bytes`, `kbd`, `hold_down`, `hold_deadline`
**Side effects:**
- Reads up to 256 bytes from stdin
- Appends to keylog.txt for debugging
- Parses printable chars, ANSI arrow keys, F1-F8, mouse SGR events, kitty key protocol
- Sends KEYB_CHAR, KEYB_PRESS, KEYB_DOWN, KEYB_UP events to game
- Implements hold-key deadlines for smooth WASDQE movement in non-kitty terminals
**Notes:**
- Static 256-byte stream buffer accumulates partial escape sequences
- Backspace (8) encoded as DEL (127) in terminal, converted back to 8 for KEYB_CHAR
- Kitty protocol: `ESC _ K <type> <mods> <code> ESC \` where type=p/t/r (press/repeat/release)
- SGR mouse: `ESC [ < button;x;y;M/m` where M=press, m=release, >=64=wheel, >=32=motion
- CTRL+C detection: `(mods & 4) && stream[i+5]=='U'` in kitty mode → sets `running=false`
- Hold-key lambda at line 2745: refreshes 140ms deadline on repeated input

---

### GPM Mouse Event Handling (game_app.cpp:2568-2672)

**Signature:** `if (pfds[1].revents & POLLIN)` block for GPM mouse events
**Purpose:** Reads and processes GPM (General Purpose Mouse) events on Linux console
**Called by:** Main event loop when GPM enabled
**Calls:** `read()`, `game->OnMouse()`
**Globals read:** `gpm`, `mouse_buf`, `mouse_read`, `mouse_write`, `mouse_x`, `mouse_y`, `mouse_down`, `wh`, `game`
**Globals mutated:** `mouse_buf`, `mouse_read`, `mouse_write`, `mouse_x`, `mouse_y`, `mouse_down`
**Side effects:**
- Reads up to 32 `Gpm_Event` structs into circular buffer
- Updates `mouse_x`, `mouse_y` with delta movement, clamps to `wh` bounds
- Sends MOUSE_WHEEL_UP/DOWN, MOUSE_LEFT/MIDDLE/RIGHT_BUT_DOWN/UP, MOUSE_MOVE events
- Wraps circular buffer when `mouse_write >= 32*sizeof(Gpm_Event)`
**Notes:**
- Only compiled when `USE_GPM` defined
- Circular buffer: `mouse_buf[64]`, indices `mouse_read`/`mouse_write` track bytes not events
- Delta motion: `mouse_x += event->dx; mouse_y += event->dy;`
- Edge detection: checks `!(mouse_down&GPM_B_LEFT) && (event->buttons & GPM_B_LEFT)` for transitions
- Wheel events: `event->wdy>0` → WHEEL_UP, `event->wdy<0` → WHEEL_DOWN

---

### Platform-Specific Terminal Initialization (game_app.cpp:2254-2393)

**Signature:** Linux/macOS terminal setup before event loop
**Purpose:** Configures TTY font, palette, GPM mouse, signal handlers, and detects terminal type
**Called by:** `main()` function after `MakeStamp = GetTime;`
**Calls:** `find_tty()`, `system()`, `Gpm_Open()`, `GetWH()`, `SetScreen()`, `sigaction()`
**Globals read:** `base_path`, `tty`, `tty_font`, `tty_fonts`, `pal_rgba`, `running`
**Globals mutated:** `tty`, `gpm`, `mouse_x`, `mouse_y`, `running`
**Side effects:**
- Saves current TTY font to `/tmp/asciicker.<tty>.psf` (or `$SNAP_USER_DATA`)
- Loads CP437 font: `setfont ${base_path}fonts/cp437_${size}x${size}.png.psf`
- Connects to GPM daemon on TTY consoles
- Sets xterm palette colors 16-231 via `ESC]4;N;#RRGGBB` sequences
- Registers signal handlers for SIGTERM, SIGHUP, SIGINT, SIGTRAP, SIGILL, SIGABRT, SIGKILL
**Notes:**
- TTY detection: `getenv("TERM")=="linux"` → calls `find_tty()`
- Xterm detection: `strncmp(term_env,"xterm",5)==0` → prints "VIRTUAL TERMINAL EMULGLATOR" (typo)
- GPM connection: `Gpm_Open(&conn, tty)` initializes mouse at center of terminal
- Signal handler: `exit_handler` registered for all signals (except those with SIG_IGN)
- Font size from `tty_fonts[tty_font]` global array

---

### Cleanup & Exit (game_app.cpp:3458-3524)

**Signature:** Post-loop cleanup in `main()` function
**Purpose:** Tears down V8, closes gamepad, network, resources, restores terminal
**Called by:** After event loop exits (`running=false`)
**Calls:** `free_v8()`, `akAPI_Free()`, `GamePadUnmount()`, `close()`, `Gpm_Close()`, `DeleteTerrain()`, `DeleteWorld()`, `free()`, `FreeGame()`, `DeleteGame()`, `SetScreen()`, `PurgeItemInstCache()`, `FreeSprites()`, `FreeAudio()`
**Globals read:** `jsfd`, `gpm`, `terrain`, `world`, `buf`, `game`, `perf_out`, `frames`, `begin`, `end`, `wh`
**Globals mutated:** `jsfd`, `perf_out`, `terrain`, `world`, `buf`, `game`
**Side effects:**
- Disposes V8 JavaScript engine
- Closes gamepad device file descriptor
- Closes GPM mouse connection
- Frees terrain, world, render buffer
- Restores terminal settings via `SetScreen(false)`
- Prints final FPS: `frames * 1000000.0 / (end-begin)`
- Calls `_CrtDumpMemoryLeaks()` on Windows
**Notes:**
- Performance log closed if separate from stderr
- Label `exit:` at line 3472 (unused goto target)
- FPS calculation: microsecond precision using `GetTime()`
- Memory leak detection only on Windows (`#ifdef _WIN32`)

---

## V8 JavaScript Integration (Lines 3526-3765)

This section implements embedded V8 JavaScript engine for game scripting (terminal and OpenGL builds only; web build uses native browser JavaScript via Emscripten).

### Global V8 State (game_app.cpp:3528-3530)

```cpp
v8::Isolate* isolate = 0;
std::unique_ptr<v8::Platform> platform = 0;
v8::ArrayBuffer::Allocator* array_buffer_allocator = 0;
```

**Purpose:** Global V8 engine state shared across all V8 functions
**Notes:**
- `isolate`: V8 execution context (one per process)
- `platform`: V8 platform abstraction (threading, timers)
- `array_buffer_allocator`: Memory allocator for ArrayBuffers
- All initialized by `init_v8()`, cleaned up by `free_v8()`

---

### `ToCString` (game_app.cpp:3533-3535)

**Signature:** `const char* ToCString(const v8::String::Utf8Value& value)`
**Purpose:** Extracts C string from V8 UTF-8 value with null fallback
**Called by:** `akPrint()` (line 3565)
**Calls:** V8 dereference operator
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns `"<string conversion failed>"` if V8 conversion fails (defensive programming)

---

### `akAPI_CallV8` (game_app.cpp:3537-3548)

**Signature:** `void akAPI_CallV8(const v8::FunctionCallbackInfo<v8::Value>& args)`
**Purpose:** V8 callback wrapper that forwards integer ID to C++ `akAPI_Call()`
**Called by:** JavaScript via `akAPI_Call(id)` global function
**Calls:** `akAPI_Call()` (defined in game_api.cpp)
**Globals read:** None (uses `args.GetIsolate()` for context)
**Globals mutated:** None
**Side effects:** Executes C++ game API function based on ID
**Notes:**
- Validates single Int32 argument
- Extracts int via `args[0]->Int32Value(context).ToChecked()`
- Registered in V8 global template at line 3661

---

### `akPrint` (game_app.cpp:3550-3570)

**Signature:** `void akPrint(const v8::FunctionCallbackInfo<v8::Value>& args)`
**Purpose:** V8 callback for JavaScript `akPrint()` function (prints to stdout)
**Called by:** JavaScript via `akPrint(...)` global function
**Calls:** `printf()`, `fflush()`, `ToCString()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Prints all arguments to stdout with space separator, newline, and flush
**Notes:**
- Converts each arg to UTF-8 string via `v8::String::Utf8Value`
- Space-separated output: first arg no space, subsequent args prefixed with space
- Registered in V8 global template at line 3660

---

### `akAPI_CB` (game_app.cpp:3572-3600)

**Signature:** `void akAPI_CB(int id)`
**Purpose:** C++ to JavaScript callback dispatcher - invokes JavaScript `akAPI_CB(id)` function
**Called by:** `game_api.cpp` functions: `akAPI_Say()`, `akAPI_GetItem()`, `akAPI_OnFrame()`
**Calls:** V8 context lookup, `cb_fnc->Call()`, `GetTime()`
**Globals read:** `isolate`
**Globals mutated:** None
**Side effects:** Executes JavaScript callback function, prints exception on error, measures execution time
**Notes:**
- Looks up global `akAPI_CB` function registered by `akAPI_Init()` JavaScript
- Creates `v8::Local<v8::Value> id_val = v8::Int32::New(isolate,id);`
- Uses `v8::TryCatch` to handle JavaScript exceptions
- Commented-out timing print: `//printf("CALLBACK in %d us\n", (int)(t1-t0));`
- Callback IDs: 0=onSay, 1=onItem, 2=onFrame (see game_api.cpp:24 comment)

---

### `free_v8` (game_app.cpp:3602-3624)

**Signature:** `void free_v8()`
**Purpose:** Tears down V8 JavaScript engine and frees all resources
**Called by:** `main()` at line 3458 (terminal mode), declared at line 1691
**Calls:** `context->Exit()`, `isolate->Exit()`, `isolate->Dispose()`, `v8::V8::Dispose()`, `v8::V8::DisposePlatform()`, `delete array_buffer_allocator`
**Globals read:** `isolate`, `array_buffer_allocator`
**Globals mutated:** `akAPI_Buff`, `array_buffer_allocator`
**Side effects:** Disposes V8 isolate, platform, and allocator; prints "V8 DISPOSED."
**Notes:**
- Must exit context before exiting isolate
- Sets `akAPI_Buff = 0` to invalidate shared ArrayBuffer pointer
- Order: Exit context → Exit isolate → Dispose isolate → Dispose V8 → Dispose platform → Delete allocator
- Only called in terminal mode (not web mode which uses Emscripten JavaScript)

---

### `init_v8` (game_app.cpp:3626-3673)

**Signature:** `void init_v8()`
**Purpose:** Initializes V8 JavaScript engine with game API bindings
**Called by:** `main()` at line 1700 (terminal mode), declared at line 1690
**Calls:** `v8::platform::NewDefaultPlatform()`, `v8::V8::InitializePlatform()`, `v8::V8::Initialize()`, `v8::Isolate::New()`, `isolate->Enter()`, `v8::ObjectTemplate::New()`, `global_templ->Set()`, `v8::Context::New()`, `context->Enter()`, `v8::ArrayBuffer::New()`, `context->Global()->Set()`
**Globals read:** None
**Globals mutated:** `platform`, `isolate`, `array_buffer_allocator`, `akAPI_Buff`
**Side effects:** 
- Creates V8 platform and isolate
- Registers global functions `akPrint` and `akAPI_Call`
- Creates shared ArrayBuffer (`akAPI_V8AB`) of size `AKAPI_BUF_SIZE`
- Prints "INITIALIZED V8 <version>"
**Notes:**
- Asserts that `V8_INTL_SUPPORT` and `V8_USE_EXTERNAL_STARTUP_DATA` are NOT defined (monolith build)
- Platform: `v8::platform::NewDefaultPlatform()` for default threading/timers
- Allocator: `v8::ArrayBuffer::Allocator::NewDefaultAllocator()`
- Global template: Exposes `akPrint()` and `akAPI_Call()` to JavaScript
- ArrayBuffer: Creates `AKAPI_BUF_SIZE` byte buffer, clears to zero, stores pointer in `akAPI_Buff`
- Context persists for entire program lifetime (never exited until `free_v8()`)

---

### `akAPI_Exec` (game_app.cpp:3675-3764)

**Signature:** `void akAPI_Exec(const char* str, int len, bool root)`
**Purpose:** Compiles and executes JavaScript code string in V8 engine
**Called by:** 
- `main()` at line 1702 (initializes akAPI JavaScript code)
- `game.cpp` at lines 5136, 8076, 8644, 9158, 10466 (executes player commands from TalkBox)
**Calls:** `malloc()`, `memcpy()`, `strlen()`, `GetTime()`, `v8::String::NewFromUtf8()`, `v8::Script::Compile()`, `script->Run()`, `free()`
**Globals read:** `isolate`
**Globals mutated:** None
**Side effects:**
- Compiles JavaScript string to V8 script
- Executes script in current V8 context
- Prints exceptions to stdout
- Measures compile+execute time (commented out print at line 3764)
**Notes:**
- **root=false (default):** Wraps code in IIFE `(function(ak,akPrint){...}.apply(akAPI_This,[ak,akPrint]))`
  - Isolates variables (forces `this.variable=` for persistence)
  - Only exposes `ak` and `akPrint` to user code
  - Applied to `akAPI_This` context (JavaScript object for custom scripts)
- **root=true:** Executes code directly in global scope (used for akAPI initialization)
- Prefix at lines 3690-3712: 1177 bytes (approx)
- Suffix at line 3713: 33 bytes
- Exception handling: `v8::TryCatch` catches both compile and runtime errors
- Commented-out result printing at lines 3747-3750

---

## Platform-Specific Implementations

### Terminal Mode (game_app.cpp)

**Lines covered:** 1883-3765
**Platforms:** Linux, macOS
**Features:**
- Native V8 JavaScript engine (init_v8, free_v8, akAPI_Exec)
- ANSI/kitty terminal input parsing
- GPM mouse support (USE_GPM)
- TTY font management (setfont)
- Signal handlers (SIGTERM, SIGINT, etc.)

### Web Mode (game_web.cpp)

**Alternate implementation:** Not in this file
**Platform:** Emscripten (browser)
**Features:**
- Uses Emscripten JavaScript bridge (`EM_ASM`, `EM_JS`)
- Browser-native `window.akAPI_CB()` instead of V8 `akAPI_CB()`
- See game_web.cpp:537 for `akAPI_Exec()` stub, line 582 for `akAPI_CB()`

### Server Mode (game_svr.cpp)

**Alternate implementation:** Not in this file
**Platform:** Headless server
**Features:**
- `akAPI_Exec()` stub at game_svr.cpp:118 (no JavaScript execution)
- No V8 dependency for server builds

---

## Key Data Structures

### Input Stream State (game_app.cpp:2711-2712)

```cpp
static int stream_bytes = 0;
static char stream[256];
```

**Purpose:** Accumulates partial ANSI escape sequences across `read()` calls
**Notes:** 256-byte circular buffer, `stream_bytes` tracks valid byte count

### Hold-Key Emulation (game_app.cpp:2430-2433)

```cpp
uint64_t hold_deadline[256];
uint8_t hold_down[256];
```

**Purpose:** Emulates key-up events for terminals that only send key-down (140ms timeout)
**Notes:** 
- `hold_deadline[key]`: Microsecond timestamp when key should auto-release
- `hold_down[key]`: Boolean flag (1=down, 0=up)
- Applied to WASD, QE, IX, arrow keys for smooth movement

### Performance Profiling State (game_app.cpp:2415-2421)

```cpp
bool perf_init = false;
bool perf_enabled = false;
FILE* perf_out = nullptr;
uint64_t perf_window_start = 0;
uint64_t perf_render_sum = 0;
uint64_t perf_print_sum = 0;
int perf_frames = 0;
```

**Purpose:** Tracks render/print timing for performance analysis
**Enabled by:** `ASCIICKER_PROFILE` environment variable
**Output:** `ASCIICKER_PROFILE_LOG` file path or stderr
**Format:** `[perf] render X.XXms print Y.YYms fps Z.Z (WxH)`

### GPM Mouse Buffer (game_app.cpp:2571-2572)

```cpp
static int mouse_read = 0;
static int mouse_write = 0;
static Gpm_Event mouse_buf[64];
```

**Purpose:** Circular buffer for GPM mouse events
**Notes:** Byte-indexed (not event-indexed), wraps at `32*sizeof(Gpm_Event)` to preserve alignment

---

## Input Protocol Details

### Kitty Keyboard Protocol (game_app.cpp:3133-3344)

**Escape sequence:** `ESC _ K <type> <mods> <code> ESC \`
**Type byte (line 3142-3148):**
- `p` → type=+1 (press)
- `t` → type=0 (repeat)
- `r` → type=-1 (release)

**Mods byte (line 3151-3154):**
- `A`-`P` → mods = 0-15
- Bit 0: Shift
- Bit 1: Alt
- Bit 2: Ctrl

**Code length (lines 3156-3162):**
- 1 byte: Basic keys (0-9, A-Z, arrows, F1-F16, etc.)
- 2 bytes: Extended keys (F17-F25, KP0-KP9, modifiers)

**Special handling:**
- CTRL+C (line 3163): `(mods & 4) && stream[i+5]=='U'` → sets `running=false`
- Shift state (line 3171): `kbd[128+'a'] = (mods&1) && type>=0`
- Extended keys (line 3176-3228): `stream[i+5]=='B'` prefix for two-byte codes

### SGR Mouse Protocol (game_app.cpp:3036-3130)

**Escape sequence:** `ESC [ < button;x;y;M/m`
**Button encoding (line 3065):**
- `but = val[0] & 0x3`
- 0 = left, 1 = middle, 2 = right

**Special modes (lines 3067-3082):**
- `val[0] >= 64` → Wheel event (but=0 → UP, but=1 → DOWN)
- `val[0] >= 32` → Motion event (no button change)
- `val[0] < 32` → Button event (M=press, m=release)

**Coordinate handling:**
- `val[1]-1` = x coordinate (1-indexed in protocol, 0-indexed in game)
- `val[2]-1` = y coordinate

---

## Verification Checklist

- [x] ALL line numbers >= 1883 AND <= 3765
- [x] Functions documented with required schema
- [x] Grep-backed caller lists (no speculation)
- [x] ALL structs/globals/enums documented
- [x] Side effects listed for I/O, memory, state mutations
- [x] WHY/algorithm notes included

---

## Function Index

| Function | Lines | Purpose |
|----------|-------|---------|
| Main event loop | 2492-3456 | Terminal game loop with input/render |
| Input stream parsing | 2709-3362 | ANSI/kitty/SGR protocol parser |
| GPM mouse handler | 2568-2672 | Linux console mouse events |
| Terminal init | 2254-2393 | TTY font, palette, GPM, signals |
| Cleanup & exit | 3458-3524 | Resource teardown and FPS report |
| ToCString | 3533-3535 | V8 string extraction helper |
| akAPI_CallV8 | 3537-3548 | V8 callback → C++ akAPI_Call |
| akPrint | 3550-3570 | V8 callback → stdout print |
| akAPI_CB | 3572-3600 | C++ → V8 callback dispatcher |
| free_v8 | 3602-3624 | V8 engine teardown |
| init_v8 | 3626-3673 | V8 engine initialization |
| akAPI_Exec | 3675-3764 | JavaScript compile & execute |

---

**Total entries documented:** 12 functions + 6 data structures + 2 protocol sections

