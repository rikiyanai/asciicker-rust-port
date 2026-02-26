# SDL2 Platform Backend Function Analysis

## Global State

### `wnd_head` (sdl.cpp:103)
- Global linked list head pointer for A3D_WND windows

### `wnd_tail` (sdl.cpp:104)
- Global linked list tail pointer for A3D_WND windows

### `sdl` (sdl.cpp:129)
- Static GlobalSDL instance managing SDL lifecycle and gamepad state

---

## Structures

### `GlobalSDL` (sdl.cpp:106-127)

**Signature:** `struct GlobalSDL`

**Purpose:** Static GlobalSDL instance managing SDL lifecycle and gamepad state.

**Called by:** Global initialization only (constructor runs at startup)

**Calls:** `SDL_Init()` in constructor, `SDL_Quit()` in destructor

**Globals read:** None

**Globals mutated:** None

**Side effects:** Initializes SDL on construction, closes gamepad and calls SDL_Quit on destruction

**Notes:** Constructor initializes SDL_Init, destructor closes gamepad and SDL_Quit. Managed as static instance `sdl` at sdl.cpp:129.

---

### `A3D_WND` (sdl.cpp:131-144)

**Signature:** `struct A3D_WND`

**Purpose:** Window state tracking for SDL-based OpenGL windows.

**Called by:** Functions like `a3dOpen()`, `a3dClose()`, `a3dLoop()`

**Calls:** None (data structure only)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (data structure only)

**Notes:** Fields: `prev` and `next` (doubly-linked list pointers), `win` (SDL_Window*), `rc` (SDL_GLContext), `mapped` (window visibility flag), `cookie` (opaque user pointer), `captured` (mouse capture flag), `platform_api` (PlatformInterface struct with callbacks).
## Functions

### `a3dOpen` (sdl.cpp:146-256)

**Signature:**
```c
A3D_WND* a3dOpen(const PlatformInterface* pi, const GraphicsDesc* gd, A3D_WND* share)
```

**Purpose:**
Create and initialize an SDL window with OpenGL context, attach to global linked list, call PlatformInterface init/resize callbacks.

**Called by:** 
No callers found via grep (entry point from engine initialization)

**Calls:**
- `malloc()` — allocate A3D_WND structure
- `SDL_GL_GetCurrentContext()`, `SDL_GL_GetCurrentWindow()` — save current GL state (RAII PUSH)
- `SDL_GL_MakeCurrent()` — restore GL state (RAII PUSH dtor)
- `SDL_GL_SetAttribute()` — configure GL depth, stencil, double-buffer, context flags, version, profile
- `SDL_CreateWindow()` — create SDL window with ALLOW_HIGHDPI, OPENGL, RESIZABLE, HIDDEN flags
- `SDL_SetWindowData()` — store wnd pointer in window's "a3d" data
- `SDL_GL_CreateContext()` — create OpenGL context (now current)
- `SDL_GL_GetDrawableSize()` — query drawable size (may differ from window size on HiDPI)
- `wnd->platform_api.init()` — callback to initialize platform-dependent resources
- `wnd->platform_api.resize()` — callback to notify of initial size

**Globals read:**
- `wnd_tail` — append new window to end of linked list

**Globals mutated:**
- `wnd_head` — set to wnd if list was empty (wnd_tail was 0)
- `wnd_tail` — always set to wnd (new tail)
- `wnd_tail->next` (if existed) — set to wnd

**Side effects:**
- SDL_Window created, OpenGL context created and made current
- Window hidden until `a3dSetVisible()` called
- Linked list modified
- Platform callbacks invoked (init, resize)

**Notes:**
- PUSH saves/restores GL context to allow context switching during creation (if sharing contexts)
- Gamepad haptic code commented out (TODO for future)
- Context attributes set before `SDL_GL_CreateContext()` call (attribute setting after would have no effect)

---

### `a3dClose` (sdl.cpp:258-293)

**Signature:**
```c
void a3dClose(A3D_WND* wnd)
```

**Purpose:**
Destroy SDL window and OpenGL context, unlink from global window list.

**Called by:**
No callers found via grep (cleanup function)

**Calls:**
- `SDL_GL_GetCurrentContext()`, `SDL_GL_GetCurrentWindow()` — save GL state (RAII PUSH)
- `SDL_GL_MakeCurrent()` — restore GL state (RAII PUSH dtor)
- `SDL_GL_DeleteContext()` — destroy OpenGL context
- `SDL_DestroyWindow()` — destroy SDL window

**Globals read:**
- `wnd->prev`, `wnd->next` — linked list pointers

**Globals mutated:**
- `wnd_head` — set to wnd->next if wnd was head
- `wnd_tail` — set to wnd->prev if wnd was tail
- Neighbor nodes' prev/next pointers updated

**Side effects:**
- SDL window and OpenGL context destroyed
- Window unlinked from global list
- wnd memory freed

**Notes:**
- PUSH ensures context is restored before function returns
- Unlink is standard doubly-linked list removal

---

### `a3dGetRect` (sdl.cpp:295-312)

**Signature:**
```c
WndMode a3dGetRect(A3D_WND* wnd, int* xywh, int* client_wh)
```

**Purpose:**
Query window position/size and drawable size, return fullscreen mode.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GL_GetDrawableSize()` — query HiDPI-aware drawable size
- `SDL_GetWindowPosition()` — query window position
- `SDL_GetWindowSize()` — query window size
- `SDL_GetWindowFlags()` — query window state

**Globals read:**
- `wnd_tail` — if wnd is NULL, use last window

**Globals mutated:**
None

**Side effects:**
None (query only)

**Notes:**
- Returns WndMode::A3D_WND_FULLSCREEN if SDL_WINDOW_FULLSCREEN flag set, else A3D_WND_NORMAL
- Author notes "not critical" for FULLSCREEN detection — may not fully report desktop fullscreen state
- xywh outputs: [x, y, width, height]
- client_wh outputs: [drawable_width, drawable_height]

---

### `a3dSetRect` (sdl.cpp:314-329)

**Signature:**
```c
bool a3dSetRect(A3D_WND* wnd, const int* xywh, WndMode wnd_mode)
```

**Purpose:**
Set window position/size and fullscreen mode.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_SetWindowPosition()` — set x, y
- `SDL_SetWindowSize()` — set w, h
- `SDL_SetWindowFullscreen()` — enable/disable fullscreen with SDL_WINDOW_FULLSCREEN_DESKTOP or 0

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Window geometry and fullscreen state changed

**Notes:**
- xywh: [x, y, width, height]
- wnd_mode: A3D_WND_FULLSCREEN enables desktop fullscreen, others disable; A3D_WND_CURRENT leaves mode unchanged

---

### `a3dIsMaximized` (sdl.cpp:331-334)

**Signature:**
```c
bool a3dIsMaximized(A3D_WND* wnd)
```

**Purpose:**
Query if window is maximized.

**Called by:**
No callers found via grep

**Calls:**
None

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
None

**Notes:**
- Always returns false (unimplemented stub, author notes "not critical")

---

### `a3dSetVisible` (sdl.cpp:336-344)

**Signature:**
```c
void a3dSetVisible(A3D_WND* wnd, bool set)
```

**Purpose:**
Show or hide window.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_ShowWindow()` — if set is true
- `SDL_HideWindow()` — if set is false

**Globals read:**
None

**Globals mutated:**
- `wnd->mapped` — set to value of set parameter

**Side effects:**
- Window shown/hidden
- mapped state updated

**Notes:**
- mapped tracks visible state for render loop filtering

---

### `a3dGetVisible` (sdl.cpp:346-349)

**Signature:**
```c
bool a3dGetVisible(A3D_WND* wnd)
```

**Purpose:**
Query if window is visible.

**Called by:**
No callers found via grep

**Calls:**
None

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
None

**Notes:**
- Returns wnd->mapped (not queried from SDL)

---

### `a3dSetCookie` (sdl.cpp:351-354)

**Signature:**
```c
void a3dSetCookie(A3D_WND* wnd, void* cookie)
```

**Purpose:**
Store opaque user pointer in window.

**Called by:**
No callers found via grep

**Calls:**
None

**Globals read:**
None

**Globals mutated:**
- `wnd->cookie` — set to cookie value

**Side effects:**
None (storage only)

**Notes:**
- Cookie typically used to store game state or engine pointers

---

### `a3dGetCookie` (sdl.cpp:356-359)

**Signature:**
```c
void* a3dGetCookie(A3D_WND* wnd)
```

**Purpose:**
Retrieve opaque user pointer stored in window.

**Called by:**
No callers found via grep

**Calls:**
None

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
None

**Notes:**
- Inverse of a3dSetCookie()

---

### `A3D2SDL[]` (sdl.cpp:365-509)

**Signature:**
```c
int A3D2SDL[] = { ... }  // 128+ entries
```

**Purpose:**
Translation table from platform.h KeyInfo enum to SDL_Scancode for keyboard state queries.

**Called by:**
- `a3dGetKeyb()` — line 1236

**Calls:**
None (data table)

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
None

**Notes:**
- Index is KeyInfo enum value, value is SDL_SCANCODE_*
- Used by a3dGetKeyb() to query current keyboard state via SDL_GetKeyboardState()

---

### `SDL2A3D[]` (sdl.cpp:515-659)

**Signature:**
```c
KeyInfo SDL2A3D[] = { ... }  // 128 entries
```

**Purpose:**
Translation table from SDL_Scancode to platform.h KeyInfo enum for event loop translation.

**Called by:**
- `a3dLoop()` — line 947

**Calls:**
None (data table)

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
None

**Notes:**
- Index is SDL_SCANCODE_* value, value is KeyInfo enum
- Covers SDL_SCANCODE range 0-127; larger scancodes handled via switch statement in a3dLoop()

---

### `a3dLoop` (sdl.cpp:661-1149)

**Signature:**
```c
void a3dLoop(const LoopInterface* li)
```

**Purpose:**
Main SDL event loop: poll events, translate to platform callbacks, render visible windows.

**Called by:**
No callers found via grep (entry point from engine)

**Calls:**
- `SDL_GameControllerEventState()` — enable gamepad events
- `SDL_GL_GetDrawableSize()` — get window drawable size
- `wnd->platform_api.resize()` — notify of initial window size
- `SDL_PollEvent()` — retrieve next SDL event
- `SDL_GameControllerOpen()` — open gamepad device
- `SDL_GameControllerName()` — get gamepad name string
- `SDL_GameControllerFromInstanceID()` — look up gamepad by instance ID
- `SDL_GameControllerClose()` — close gamepad device
- `SDL_NumJoysticks()` — count available joysticks
- `SDL_IsGameController()` — check if joystick is a game controller
- `SDL_GameControllerGetStringForAxis()` — axis name (debug only)
- `SDL_GameControllerGetStringForButton()` — button name (debug only)
- `SDL_GetWindowID()` — query window ID from SDL_Event
- `SDL_GetMouseState()` — query current mouse position and button state
- `SDL_GL_GetDrawableSize()` — get window drawable size for bounds checking
- `SDL_CaptureMouse()` — enable/disable mouse capture
- `wnd->platform_api.keyb_char()` — character input callback
- `wnd->platform_api.keyb_key()` — key press/release callback
- `wnd->platform_api.keyb_focus()` — focus gained/lost callback
- `wnd->platform_api.mouse()` — mouse event callback
- `wnd->platform_api.close()` — window close request callback
- `wnd->platform_api.render()` — render callback
- `SDL_GL_MakeCurrent()` — activate GL context
- `SDL_GL_SetSwapInterval()` — set vsync (0=off, 1=on)
- `SDL_GL_SwapWindow()` — present rendered frame

**Globals read:**
- `wnd_head`, `wnd_tail` — iterate windows for resize notifications and rendering
- `SDL2A3D[]` — translate SDL scancode to KeyInfo

**Globals mutated:**
- `sdl.gamepad` — set/cleared on controller device events
- `wnd->captured` — set/cleared on mouse button down/up
- `wnd->mapped` — read (not mutated) for render filtering

**Side effects:**
- SDL events consumed from queue
- Platform callbacks invoked (input, render)
- Windows rendered and framebuffers swapped
- Event loop runs until Running becomes false (SDL_QUIT event)

**Notes:**
- Event loop structure: while (Running) { while (SDL_PollEvent()) { translate event → callback } render all windows }
- Gamepad mapping array encodes axis/button layout: bit 7 = axis vs button, bit 6 = flip, bits 0-4 = index
- Mouse capture enabled on button down, disabled on button up (SDL has no capture-lost event, so unreliable)
- DELETE/BACKSPACE/ENTER keysym events manually translated to UTF-8 character codes (SDL_TEXTINPUT doesn't report them)
- Render: iterate visible windows, make context current, call render callback, defer SDL_GL_SwapWindow() to last window (vsync applied to final swap)
- Gamepad remapping on disconnect: if active controller disconnected, try reconnecting to another available controller

---

### `_a3dSetIconData` (sdl.cpp:1151-1160)

**Signature:**
```c
void _a3dSetIconData(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Convert image buffer to RGBA8, create SDL_Surface, set as window icon, clean up.

**Called by:**
- `a3dSetIconData()` — line 1164
- Callback from `a3dLoadImage()` — line 1170

**Calls:**
- `malloc()` — allocate uint32_t RGBA buffer
- `Convert_UI32_AARRGGBB()` — convert image format to 32-bit AARRGGBB
- `SDL_CreateRGBSurfaceFrom()` — create surface from buffer (no copy)
- `SDL_SetWindowIcon()` — set window icon
- `SDL_FreeSurface()` — free surface (does not free buffer)
- `free()` — free allocated buffer

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Window icon set
- Temporary buffer allocated and freed

**Notes:**
- cookie is cast to A3D_WND* to get wnd->win
- SDL_CreateRGBSurfaceFrom() does not copy data, only wraps buffer (surface freed before function returns, but buffer is freed immediately after so surface does not access freed memory)

---

### `a3dSetIconData` (sdl.cpp:1162-1166)

**Signature:**
```c
bool a3dSetIconData(A3D_WND* wnd, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Wrapper to set window icon from image buffer.

**Called by:**
No callers found via grep

**Calls:**
- `_a3dSetIconData()` — actual implementation

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Window icon set (via _a3dSetIconData)

**Notes:**
- Public API wrapper, always returns true

---

### `a3dSetIcon` (sdl.cpp:1168-1171)

**Signature:**
```c
bool a3dSetIcon(A3D_WND* wnd, const char* path)
```

**Purpose:**
Load icon image from file and set as window icon.

**Called by:**
No callers found via grep

**Calls:**
- `a3dLoadImage()` — load PNG and invoke _a3dSetIconData callback

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- File read, image loaded, window icon set

**Notes:**
- Delegates to a3dLoadImage() with _a3dSetIconData callback

---

### `a3dSetTitle` (sdl.cpp:1173-1176)

**Signature:**
```c
void a3dSetTitle(A3D_WND* wnd, const char* utf8_name)
```

**Purpose:**
Set window title.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_SetWindowTitle()` — set title

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Window title changed

**Notes:**
- utf8_name should be UTF-8 encoded string

---

### `a3dGetTitle` (sdl.cpp:1178-1191)

**Signature:**
```c
int a3dGetTitle(A3D_WND* wnd, char* utf8_name, int size)
```

**Purpose:**
Query window title.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GetWindowTitle()` — get title
- `strlen()` — compute title length
- `memcpy()` — copy title to output buffer

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- utf8_name buffer filled (if non-NULL)

**Notes:**
- Returns length of title (truncated to size-1)
- Null-terminates output buffer

---

### `a3dLoadImage` (sdl.cpp:1194-1230)

**Signature:**
```c
bool a3dLoadImage(const char* path, void* cookie, void(*cb)(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf))
```

**Purpose:**
Load PNG image from file, invoke callback with image data.

**Called by:**
- `a3dSetIcon()` — line 1170

**Calls:**
- `upng_new_from_file()` — open PNG file
- `upng_get_error()` — check for errors
- `upng_free()` — free PNG reader
- `upng_decode()` — decode PNG
- `upng_get_format()` — get pixel format
- `upng_get_width()` — get width
- `upng_get_height()` — get height
- `upng_get_bpp()` — get bits per pixel
- `upng_get_buffer()` — get decompressed pixel data
- `upng_get_pal_buffer()` — get palette data (if indexed)
- `upng_get_pal_size()` — get palette size

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- File read, PNG decoded
- Callback invoked with image data
- PNG reader freed

**Notes:**
- Returns false if file not found or decode error
- Callback receives decompressed pixel buffer and optional palette
- TODO comment suggests image loading should be queued and called on event loop idle (not inline)

---

### `a3dGetKeyb` (sdl.cpp:1232-1244)

**Signature:**
```c
bool a3dGetKeyb(A3D_WND* wnd, KeyInfo ki)
```

**Purpose:**
Query current keyboard state for a specific key.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GetKeyboardState()` — get array of all key states
- A3D2SDL[] lookup — translate KeyInfo to SDL_Scancode

**Globals read:**
- `A3D2SDL[]` — translate KeyInfo to SDL_Scancode

**Globals mutated:**
None

**Side effects:**
None (query only)

**Notes:**
- Returns true if key is currently pressed
- Returns false if KeyInfo is out of range (< 0 or >= 128)
- Returns false if SDL_Scancode array size is too small

---

### `a3dPushContext` (sdl.cpp:1246-1250)

**Signature:**
```c
void a3dPushContext(A3D_PUSH_CONTEXT* ctx)
```

**Purpose:**
Save current OpenGL context and window.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GL_GetCurrentWindow()` — save window
- `SDL_GL_GetCurrentContext()` — save context

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- ctx->data[0] and ctx->data[1] filled with current GL state

**Notes:**
- Typically used with a3dPopContext() to save/restore GL state around a3dOpen()/a3dClose()
- ctx should be A3D_PUSH_CONTEXT structure (opaque to this function)

---

### `a3dPopContext` (sdl.cpp:1252-1255)

**Signature:**
```c
void a3dPopContext(const A3D_PUSH_CONTEXT* ctx)
```

**Purpose:**
Restore OpenGL context and window from saved state.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GL_MakeCurrent()` — restore context

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- OpenGL context and window restored

**Notes:**
- Inverse of a3dPushContext()

---

### `a3dSwitchContext` (sdl.cpp:1257-1260)

**Signature:**
```c
void a3dSwitchContext(const A3D_WND* wnd)
```

**Purpose:**
Make window's OpenGL context current.

**Called by:**
No callers found via grep

**Calls:**
- `SDL_GL_MakeCurrent()` — activate context

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- GL context changed to wnd->rc on wnd->win

**Notes:**
- Direct context switch (no push/pop save)

---

### `a3dGetTime` (sdl.cpp:1278-1283) — UNIX/LINUX/MACOS

**Signature:**
```c
uint64_t a3dGetTime()  // Unix
```

**Purpose:**
Get current monotonic time in microseconds.

**Called by:**
No callers found via grep

**Calls:**
- `clock_gettime()` — get CLOCK_MONOTONIC time

**Globals read:**
None (ts is static local)

**Globals mutated:**
None

**Side effects:**
None (query only)

**Notes:**
- Returns microseconds since unspecified epoch
- Monotonic: never goes backwards (unaffected by NTP, leap seconds)
- Wraps every 584542 years
- Conversion: ts.tv_sec * 1000000 + ts.tv_nsec / 1000

---

### `a3dListDir` (sdl.cpp:1285-1332) — UNIX/LINUX/MACOS

**Signature:**
```c
int a3dListDir(const char* dir_path, bool(*cb)(A3D_DirItem item, const char* name, void* cookie), void* cookie)
```

**Purpose:**
List directory contents, invoke callback for each item.

**Called by:**
No callers found via grep

**Calls:**
- `opendir()` — open directory
- `readdir()` — read next directory entry
- `snprintf()` — format full path
- `lstat()` — stat file (for DT_UNKNOWN case)
- `closedir()` — close directory
- Callback function — invoked for each file/directory

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Directory traversed, callback invoked for each item

**Notes:**
- Handles DT_UNKNOWN by calling lstat() to determine type
- Returns count of items passed to callback (before callback returns false)
- Returns -1 if directory open fails
- Callback should return true to continue, false to stop iteration

---

### `a3dSetCurDir` (sdl.cpp:1335-1338) — UNIX/LINUX/MACOS

**Signature:**
```c
bool a3dSetCurDir(const char* dir_path)  // Unix
```

**Purpose:**
Change current working directory.

**Called by:**
No callers found via grep

**Calls:**
- `chdir()` — change directory

**Globals read:**
None

**Globals mutated:**
None (process-global state, not function-local)

**Side effects:**
- Current working directory changed

**Notes:**
- Returns true if chdir() succeeds (return 0)

---

### `a3dGetCurDir` (sdl.cpp:1340-1354) — UNIX/LINUX/MACOS

**Signature:**
```c
bool a3dGetCurDir(char* dir_path, int size)  // Unix
```

**Purpose:**
Query current working directory and append trailing slash.

**Called by:**
No callers found via grep

**Calls:**
- `getcwd()` — get current working directory
- `strlen()` — compute path length

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- dir_path filled with current directory and trailing /

**Notes:**
- Returns false if dir_path is NULL or getcwd() fails
- Appends trailing / if space available
- Returns true if getcwd() succeeds and path fits in size

---

### `a3dGetTime` (sdl.cpp:1374-1400) — WINDOWS

**Signature:**
```c
uint64_t a3dGetTime()  // Windows
```

**Purpose:**
Get current high-precision time in microseconds via Windows QueryPerformanceCounter.

**Called by:**
No callers found via grep

**Calls:**
- `QueryPerformanceFrequency()` — get timer frequency
- `QueryPerformanceCounter()` — get current timer value

**Globals read:**
- `timer_freq` — frequency (set by Get functions)
- `coarse_perf` — baseline counter value
- `coarse_micro` — baseline microseconds

**Globals mutated:**
- `Get` — function pointer swapped from Get1 to Get2 (once)

**Side effects:**
- QueryPerformanceCounter() called (no mutation)

**Notes:**
- SafeTimer is local nested struct with two implementations
- Get1 sets timer_freq once, then Get is reassigned to Get2
- Get2 uses cached timer_freq from Get1
- This pattern avoids QueryPerformanceFrequency() overhead after first call
- Conversion: (counter - baseline) * 1000000 / frequency + baseline_us
- Returns microseconds

---

### `a3dListDir` (sdl.cpp:1402-1427) — WINDOWS

**Signature:**
```c
int a3dListDir(const char* dir_path, bool(*cb)(A3D_DirItem item, const char* name, void* cookie), void* cookie)  // Windows
```

**Purpose:**
List directory contents using Windows API, invoke callback for each item.

**Called by:**
No callers found via grep

**Calls:**
- `snprintf()` — format search pattern (dir_path/*)
- `FindFirstFileA()` — open directory search
- `FindNextFileA()` — get next entry
- `FindClose()` — close search
- Callback function — invoked for each file/directory

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Directory searched, callback invoked for each item

**Notes:**
- Returns -1 if FindFirstFileA fails with INVALID_HANDLE_VALUE
- Returns 0 if FindFirstFileA returns NULL
- Returns count of items enumerated
- Uses FILE_ATTRIBUTE_DIRECTORY flag to distinguish directory from file
- Callback receives A3D_DIRECTORY or A3D_FILE item type

---

### `a3dSetCurDir` (sdl.cpp:1429-1432) — WINDOWS

**Signature:**
```c
bool a3dSetCurDir(const char* dir_path)  // Windows
```

**Purpose:**
Change current working directory (Windows).

**Called by:**
No callers found via grep

**Calls:**
- `SetCurrentDirectoryA()` — change directory

**Globals read:**
None

**Globals mutated:**
None (process-global state)

**Side effects:**
- Current working directory changed

**Notes:**
- Returns result of SetCurrentDirectoryA() (nonzero = success)

---

### `a3dGetCurDir` (sdl.cpp:1434-1443) — WINDOWS

**Signature:**
```c
bool a3dGetCurDir(char* dir_path, int size)  // Windows
```

**Purpose:**
Query current working directory and append trailing backslash (Windows).

**Called by:**
No callers found via grep

**Calls:**
- `GetCurrentDirectoryA()` — get current directory

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- dir_path filled with current directory and trailing \

**Notes:**
- Returns false if len + 1 >= size (buffer too small)
- GetCurrentDirectoryA() returns length of path (0 on failure)
- Appends \ if space available (len + 1 < size)
- Returns true only if path fits with trailing backslash and null terminator

