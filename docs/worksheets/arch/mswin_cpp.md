# mswin.cpp Architecture

Windows 7+ native Win32/WGL platform backend for windowing and input handling. This file implements PlatformInterface callbacks for window lifecycle, keyboard/mouse input, graphics context management, and OS integration. Compiles only when `_WIN32 && !defined(USE_SDL)`.

## Global State

### Linked List of Windows
```
A3D_WND* wnd_head = 0;
A3D_WND* wnd_tail = 0;
```
Doubly-linked list maintaining all open windows. Used by a3dLoop() to iterate visible windows for rendering.

### Performance Counter Calibration
```
LARGE_INTEGER coarse_perf;     // QueryPerformanceCounter baseline
uint64_t coarse_micro;         // Microsecond offset (refreshed every 60s)
LARGE_INTEGER timer_freq;      // QueryPerformanceFrequency (constant)
```
High-precision timing state initialized on first window creation. Refreshed every 60 seconds to prevent overflow in relative time calculations.

### Key Translation Tables
- `ki_to_vk[256]` (lines 141-274): KeyInfo enum → Windows VK_* virtual key codes
- `vk_to_ki[256]` (lines 281-539): Windows VK_* codes → KeyInfo enum (data contract)

---

## Data Structures

### A3D_WND (lines 99-118)
```cpp
struct A3D_WND {
    PlatformInterface platform_api;  // Callback table to game layer
    HWND hwnd;
    HDC dc;
    HGLRC rc;
    A3D_WND* prev, *next;            // Linked list pointers
    void* cookie;                    // User data (opaque)
    int mouse_b, mouse_x, mouse_y;   // Mouse state (persistent)
    bool track, mapped;
    WndMode wndmode;                 // Normal, fullscreen, frameless
    int exit_full_xywh[4];           // Window rect before fullscreen
};
```

### A3D_PTY (lines 82-91)
Stub structure for Unix pty support (disabled on Windows). Commented out.

---

## Functions

### `a3dPushContext` (mswin.cpp:120-125)

**Signature:** `void a3dPushContext(A3D_PUSH_CONTEXT* ctx)`

**Purpose:** Save current WGL context state (DC + rendering context) for later restoration.

**Called by:** a3dOpen() (line 891-917 stack-based PUSH struct)

**Calls:** `wglGetCurrentDC()`, `wglGetCurrentContext()`

**Globals read:** None

**Globals mutated:** ctx->data[1] (DC), ctx->data[2] (HGLRC)

**Side effects:** Saves GL context state to stack-like struct.

**Notes:** Used during window creation to preserve existing context; restored via a3dPopContext.

---

### `a3dPopContext` (mswin.cpp:127-130)

**Signature:** `void a3dPopContext(const A3D_PUSH_CONTEXT* ctx)`

**Purpose:** Restore previously saved WGL context state.

**Called by:** a3dOpen() (line 907-910 PUSH destructor)

**Calls:** `wglMakeCurrent()`

**Globals read:** None

**Globals mutated:** None (WGL state modified)

**Side effects:** Restores GL rendering context to saved state.

**Notes:** Inverse of a3dPushContext; called in destructor to auto-restore on scope exit.

---

### `a3dSwitchContext` (mswin.cpp:132-135)

**Signature:** `void a3dSwitchContext(const A3D_WND* wnd)`

**Purpose:** Make a window's GL context current.

**Called by:** a3dWndProc() (line 586 on message entry)

**Calls:** `wglMakeCurrent()`

**Globals read:** None

**Globals mutated:** WGL state (current context)

**Side effects:** Switches active rendering context to target window.

**Notes:** Called before invoking platform_api callbacks to ensure correct GL context.

---

### `a3dWndProc` (mswin.cpp:543-875)

**Signature:** `LRESULT WINAPI a3dWndProc(HWND h, UINT m, WPARAM w, LPARAM l)`

**Purpose:** Windows message procedure. Dispatches WM_* messages to PlatformInterface callbacks.

**Called by:** Windows event loop (GetMessage/DispatchMessage), setup in a3dOpen() line 928.

**Calls:**
- `GetWindowLongPtr()`, `SetWindowLongPtr()`, `SetTimer()`, `KillTimer()`, `QueryPerformanceCounter/Frequency()`
- `wglMakeCurrent()`, `wglDeleteContext()`, `ReleaseDC()`, `DestroyWindow()`
- `ValidateRect()`, `TrackMouseEvent()`, `SetCapture()`, `ReleaseCapture()`
- `GetKeyState()`, `GetCursorPos()`, `ScreenToClient()`
- Platform callbacks: `wnd->platform_api.resize()`, `.render()`, `.close()`, `.keyb_key()`, `.keyb_char()`, `.keyb_focus()`, `.mouse()`

**Globals read:** `wnd_head`, `wnd_tail` (linked list), `coarse_perf`, `coarse_micro`, `timer_freq`
**Globals mutated:** `wnd_head`, `wnd_tail` (linked list), `coarse_perf`, `coarse_micro`, `timer_freq`

**Side effects:**
- Creates/destroys windows, manages linked list
- Initializes high-precision timer on first window
- Handles all keyboard, mouse, and window lifecycle events
- Swaps GL buffers via wglSwapMultipleBuffers()

**Message Handlers:**
| Message | Handler |
|---------|---------|
| WM_CREATE | Allocate A3D_WND, init timer, link list, set user data |
| WM_DESTROY | Cleanup GL context, unlink from list, free struct |
| WM_CLOSE | Call platform_api.close() callback or DestroyWindow() |
| WM_SIZE | Call platform_api.resize(w, h) |
| WM_PAINT | Validate rect, return 0 (no redraw, use timer-driven rendering) |
| WM_ERASEBKGND | Return 0 (no background erase) |
| WM_KEYDOWN/UP, WM_SYSKEYDOWN/UP | VK_* → KeyInfo via vk_to_ki[], detect numlock/extended keys, call keyb_key() |
| WM_CHAR | Call keyb_char(wchar_t) |
| WM_SYSCOMMAND | Suppress F10/ALT menu (return 0 for SC_KEYMENU) |
| WM_DELETE (special) | Synthesize keyb_char(127) for Delete key |
| WM_LBUTTONDOWN/UP, WM_RBUTTONDOWN/UP, WM_MBUTTONDOWN/UP | Update mouse_b flags, call mouse() with LEFT/RIGHT/MIDDLE_DN/UP |
| WM_MOUSEMOVE | Update mouse_x/y, track enter/leave, call mouse(ENTER/MOVE) |
| WM_MOUSELEAVE | Call mouse(LEAVE) |
| WM_MOUSEWHEEL | Parse wheel delta, call mouse(WHEEL_UP/DN) n times |
| WM_SETFOCUS/KILLFOCUS | Call keyb_focus(true/false) |
| WM_ENTERMENULOOP, WM_ENTERSIZEMOVE | SetTimer for continuous render during menu/resize |
| WM_EXITMENULOOP, WM_EXITSIZEMOVE | KillTimer to stop render |
| WM_TIMER | If w==1 && wnd==wnd_head: refresh coarse_perf/coarse_micro. If w==2,3: call render() |

**Notes:**
- Extended key detection via (l >> 24) & 1 to distinguish left/right modifiers
- Numlock overrides arrow keys on numeric keypad (vk_to_ki logic)
- Auto-repeat flag: (l & (1 << 30)) → A3D_AUTO_REPEAT or'd into KeyInfo
- Mouse capture managed via SetCapture/ReleaseCapture; released when all buttons up
- Keyboard focus tracked for focus in/out events

---

### `a3dOpen` (mswin.cpp:887-1096)

**Signature:** `A3D_WND* a3dOpen(const PlatformInterface* pi, const GraphicsDesc* gd, A3D_WND* share)`

**Purpose:** Create a new window, initialize OpenGL context, attach to window list.

**Called by:** game.cpp (assumed)

**Calls:**
- `GetModuleHandle()`, `RegisterClass()`, `CreateWindow()`, `GetDC()`, `GetClientRect()`, `GetWindowLongPtr()`, `GetWindowTextLength()`
- `ChoosePixelFormat()`, `SetPixelFormat()`, `wglCreateContext()`, `wglCreateContextAttribsARB()`, `wglMakeCurrent()`, `wglDeleteContext()`
- `wglGetCurrentDC()`, `wglGetCurrentContext()` (PUSH context init)
- Platform callback: `wnd->platform_api.init()`, `.resize()`

**Globals read:** `wnd_head`, `wnd_tail` (linked list), `timer_freq` (if first window), `coarse_perf` (if first window), `coarse_micro` (if first window)
**Globals mutated:** `wnd_head`, `wnd_tail` (linked list), `timer_freq` (if first window), `coarse_perf` (if first window), `coarse_micro` (if first window)

**Side effects:**
- Registers window class if first window
- Allocates A3D_WND struct via malloc
- Creates Win32 window via CreateWindowEx()
- Creates OpenGL context via wglCreateContextAttribsARB
- Calls platform_api.init() and platform_api.resize() to initialize game layer
- Returns NULL on any failure

**GL Setup Process:**
1. Create temporary context with wglCreateContext()
2. Make current with wglMakeCurrent()
3. Query wglCreateContextAttribsARB extension function
4. Delete temporary context
5. Create final core/compat context via wglCreateContextAttribsARB() with version/profile from gd
6. Make final context current

**Error Handling:**
Returns false (NULL) on any failure: RegisterClass, CreateWindow, GetDC, ChoosePixelFormat, SetPixelFormat, wglCreateContext, wglCreateContextAttribsARB, wglMakeCurrent. Properly cleans up partial allocations.

**Notes:**
- PUSH stack-based context guard preserves existing context across setup
- Window style includes WS_POPUP added after CreateWindow (hack for CW_USEDEFAULT)
- Pixel format descriptor configured based on GraphicsDesc flags (color bits, depth, double buffer, etc.)
- share parameter allows shared GL resources (context sharing)
- Device context stored in A3D_WND for later buffer swaps

---

### `a3dLoop` (mswin.cpp:1098-1157)

**Signature:** `void a3dLoop()`

**Purpose:** Main event loop. Processes all Win32 messages and renders all visible windows until all close.

**Called by:** game.cpp (assumed main loop)

**Calls:**
- `GetWindowLong()`, `GetClientRect()` (for resize notifications)
- `PeekMessage()`, `TranslateMessage()`, `DispatchMessage()` (message pump)
- `wglMakeCurrent()`, `wglSwapMultipleBuffers()`, `SwapBuffers()`
- Platform callback: `wnd->platform_api.resize()`, `.render()`
- `UnregisterClass()` (at exit)

**Globals read:** `wnd_head`, `wnd_tail` (linked list)
**Globals mutated:** None

**Side effects:**
- Blocks until all windows destroyed
- Pumps Win32 message queue
- Renders all windows and swaps buffers
- Cleans up window class registration at exit

**Render Loop Process:**
1. Force resize notifications to all windows on entry
2. While wnd_head not NULL:
   a. PeekMessage all queued messages and dispatch
   b. For each visible window:
      - Make context current
      - Call platform_api.render()
      - Enqueue for buffer swap
   c. Batch swap via wglSwapMultipleBuffers() (up to WGL_SWAPMULTIPLE_MAX)
   d. Repeat

**Notes:**
- Visibility checked via GetWindowLong(GWL_STYLE) & WS_VISIBLE
- Batch swaps reduce CPU overhead for multi-window setups
- Unregisters window class on cleanup

---

### `a3dClose` (mswin.cpp:1159-1184)

**Signature:** `void a3dClose(A3D_WND* wnd)`

**Purpose:** Close a window (triggers WM_DESTROY).

**Called by:** game.cpp (assumed)

**Calls:** `DestroyWindow()`, `wglGetCurrentDC()`, `wglGetCurrentContext()`, `wglMakeCurrent()`

**Globals read:** WGL state (via PUSH context guard)
**Globals mutated:** WGL state

**Side effects:** Sends WM_DESTROY to window, which unlinks from list and frees A3D_WND struct.

**Notes:** Uses stack-based PUSH context guard to preserve calling context across cleanup.

---

### `a3dSetCookie` (mswin.cpp:1186-1189)

**Signature:** `void a3dSetCookie(A3D_WND* wnd, void* cookie)`

**Purpose:** Attach opaque user data pointer to window.

**Called by:** game.cpp (assumed)

**Calls:** None

**Globals read:** None
**Globals mutated:** None

**Side effects:** Writes wnd->cookie

**Notes:** Simple getter/setter pair; no validation.

---

### `a3dGetCookie` (mswin.cpp:1191-1194)

**Signature:** `void* a3dGetCookie(A3D_WND* wnd)`

**Purpose:** Retrieve opaque user data pointer.

**Called by:** game.cpp (assumed)

**Calls:** None

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Notes:** See a3dSetCookie.

---

### `a3dGetTime` (mswin.cpp:1209-1235)

**Signature:** `uint64_t a3dGetTime()`

**Purpose:** Return current time in microseconds using high-precision performance counter.

**Called by:** game.cpp physics/animation loop (assumed)

**Calls:** `QueryPerformanceFrequency()`, `QueryPerformanceCounter()`

**Globals read:** `coarse_perf`, `coarse_micro`, `timer_freq`
**Globals mutated:** None

**Side effects:** None (read-only counters)

**Conversion Formula:** `microseconds = coarse_micro + (counter - coarse_perf) * 1000000 / timer_freq`

**Notes:**
- coarse_perf/coarse_micro refreshed every 60 seconds (via WM_TIMER) to prevent overflow
- SafeTimer::Get1/Get2 indirection  support runtime selection (both identical)
- Wraps every 584542 years (2^64 / 1M / 60 / 60 / 24 / 365)

---

### `a3dGetKeyb` (mswin.cpp:1247-1254)

**Signature:** `bool a3dGetKeyb(A3D_WND* wnd, KeyInfo ki)`

**Purpose:** Query current keyboard state for a key (no wnd parameter used).

**Called by:** game.cpp input polling (assumed)

**Calls:** `GetKeyState()`

**Globals read:** ki_to_vk[] translation table

**Globals mutated:** None

**Side effects:** None

**Notes:** Validates KeyInfo in range [0, A3D_MAPEND); returns false for unmapped keys.

---

### `a3dSetTitle` (mswin.cpp:1256-1265)

**Signature:** `void a3dSetTitle(A3D_WND* wnd, const char* utf8_name)`

**Purpose:** Set window title from UTF-8 string.

**Called by:** game.cpp (assumed)

**Calls:** `MultiByteToWideChar()`, `SetWindowTextW()`, malloc/free

**Globals read:** None
**Globals mutated:** None

**Side effects:** Modifies window title bar.

**Notes:** Converts UTF-8 → UTF-16 on stack, then calls Win32 API.

---

### `a3dGetTitle` (mswin.cpp:1267-1284)

**Signature:** `int a3dGetTitle(A3D_WND* wnd, char* utf8_name, int size)`

**Purpose:** Get window title as UTF-8 string.

**Called by:** game.cpp (assumed)

**Calls:** `GetWindowTextLength()`, `GetWindowTextW()`, `WideCharToMultiByte()`, malloc/free

**Globals read:** None
**Globals mutated:** None

**Side effects:** Writes utf8_name buffer (clamped to size); always null-terminates.

**Return Value:** Byte count written (or 3*wchars_num estimate if no buffer).

**Notes:** Handles incomplete UTF-8 conversion by replacing unmappable chars with '?'.

---

### `a3dSetVisible` (mswin.cpp:1286-1291)

**Signature:** `void a3dSetVisible(A3D_WND* wnd, bool visible)`

**Purpose:** Show or hide window.

**Called by:** game.cpp (assumed)

**Calls:** `ShowWindow()`

**Globals read:** `wnd->mapped` (state cached)
**Globals mutated:** None

**Side effects:** Updates window visibility via ShowWindow(SW_SHOW/SW_HIDE).

**Notes:** None

---

### `a3dGetVisible` (mswin.cpp:1293-1296)

**Signature:** `bool a3dGetVisible(A3D_WND* wnd)`

**Purpose:** Return window visibility state (cached).

**Called by:** game.cpp (assumed)

**Calls:** None

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Notes:** Returns wnd->mapped state; may not reflect true OS state if ShowWindow called externally.

---

### `a3dGetRect` (mswin.cpp:1298-1366)

**Signature:** `WndMode a3dGetRect(A3D_WND* wnd, int* xywh, int* client_wh)`

**Purpose:** Query window position/size and client area. Clips docked windows to monitor workarea.

**Called by:** game.cpp (assumed)

**Calls:** `GetClientRect()`, `GetWindowRect()`, `GetWindowPlacement()`, `MonitorFromWindow()`, `GetMonitorInfo()`

**Globals read:** `wnd->wndmode`, `wnd->exit_full_xywh`
**Globals mutated:** None

**Side effects:** Modifies xywh[] array by clipping to monitor bounds (NORMAL mode only).

**Return Value:** wnd->wndmode (A3D_WND_NORMAL, A3D_WND_FULLSCREEN, A3D_WND_FRAMELESS)

**Logic:**
- If xywh: Fill with window rect, then clip to monitor workarea if NORMAL and docked
- If client_wh: Fill with client area rect

**Notes:** Docking detection compares GetWindowPlacement() against GetMonitorInfo().rcWork; clips left/right/top/bottom overflow.

---

### `a3dIsMaximized` (mswin.cpp:1368-1372)

**Signature:** `bool a3dIsMaximized(A3D_WND* wnd)`

**Purpose:** Check if window is maximized.

**Called by:** game.cpp (assumed)

**Calls:** `GetWindowLong()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Return Value:** true if wndmode==NORMAL && WS_MAXIMIZE set in window style

**Notes:** None

---

### `a3dSetRect` (mswin.cpp:1374-1540)

**Signature:** `bool a3dSetRect(A3D_WND* wnd, const int* xywh, WndMode wnd_mode)`

**Purpose:** Set window position/size and mode (normal, fullscreen, frameless). Handles mode transitions.

**Called by:** game.cpp (assumed)

**Calls:**
- `GetWindowLong()`, `SetWindowLong()`, `SetWindowPos()`, `GetWindowRect()`
- `EnumDisplayMonitors()`, `MonitorFromWindow()`, `GetMonitorInfo()`, `GetWindowPlacement()`
- Helper: `a3dGetRect()`

**Globals read:** `wnd->wndmode`, `wnd->exit_full_xywh`
**Globals mutated:** `wnd->wndmode`, `wnd->exit_full_xywh`

**Side effects:**
- Modifies window styles (removes/adds WS_CAPTION, WS_THICKFRAME, etc.)
- Repositions window via SetWindowPos()
- Stores exit rect on fullscreen entry (for later restore)

**Return Value:** false if wnd not mapped or mode switch fails; true on success

**Mode Transitions:**
| Target | Action |
|--------|--------|
| A3D_WND_FULLSCREEN | Remove decorations, enumerate monitors, span from leftmost to rightmost, topmost to bottommost, store exit rect |
| A3D_WND_FRAMELESS | Remove decorations, restore from fullscreen exit rect if applicable |
| A3D_WND_NORMAL | Add decorations, restore from fullscreen exit rect or use current rect |

**Monitor Enumeration:**
Finds left_mon, right_mon, top_mon, bottom_mon based on window overlap; spans fullscreen rect across all.

**Notes:**
- xywh parameter optional; if NULL, queries current rect via a3dGetRect()
- exit_full_xywh[] saved on fullscreen entry to restore normal rect on exit
- SetWindowPos called with SWP_FRAMECHANGED to force style update

---

### `a3dGetMouse` (mswin.cpp:1544-1565)

**Signature:** `MouseInfo a3dGetMouse(A3D_WND* wnd, int* x, int* y)`

**Purpose:** Query current mouse position and button state.

**Called by:** game.cpp input polling (assumed)

**Calls:** `GetCursorPos()`, `ScreenToClient()`, `GetKeyState()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** None (read-only)

**Return Value:** MouseInfo bitmask (LEFT | RIGHT | MIDDLE) indicating pressed buttons; wheel state not included.

**Notes:** Converts screen coords to client coords; x/y only written if non-NULL.

---

### `a3dSetFocus` (mswin.cpp:1567-1571)

**Signature:** `void a3dSetFocus(A3D_WND* wnd)`

**Purpose:** Set keyboard focus to window.

**Called by:** game.cpp (assumed)

**Calls:** `SetFocus()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Updates OS focus to this window.

**Notes:** None

---

### `a3dGetFocus` (mswin.cpp:1573-1577)

**Signature:** `bool a3dGetFocus(A3D_WND* wnd)`

**Purpose:** Check if window has keyboard focus.

**Called by:** game.cpp (assumed)

**Calls:** `GetFocus()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Return Value:** true if GetFocus() == wnd->hwnd

**Notes:** None

---

### `a3dCharSync` (mswin.cpp:1579-1582)

**Signature:** `void a3dCharSync(A3D_WND* wnd)`

**Purpose:** Sync character input state (no-op on Windows).

**Called by:** game.cpp (assumed)

**Calls:** None

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Notes:** Placeholder for cross-platform abstraction; OS handles char sync automatically.

---

### `a3dLoadImage` (mswin.cpp:1586-1622)

**Signature:** `bool a3dLoadImage(const char* path, void* cookie, void(*cb)(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf))`

**Purpose:** Load PNG image from file and invoke callback with decoded data.

**Called by:** a3dSetIcon() (line 1688)

**Calls:** `upng_new_from_file()`, `upng_get_error()`, `upng_decode()`, `upng_get_format()`, `upng_get_width()`, `upng_get_height()`, `upng_get_bpp()`, `upng_get_buffer()`, `upng_get_pal_buffer()`, `upng_get_pal_size()`, `upng_free()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Allocates upng decoder state (freed before return)

**Return Value:** true on success; false on file open, error, or decode failure

**Notes:** Uses vendored uPNG library; callback invoked immediately (TODO comment suggests queuing for later).

---

### `_a3dSetIconData` (mswin.cpp:1624-1678)

**Signature:** `void _a3dSetIconData(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)`

**Purpose:** Callback to set window icon from decoded image data.

**Called by:** a3dLoadImage() (line 1618), a3dSetIconData() (line 1682)

**Calls:**
- `GetDC()`, `CreateDIBSection()`, `ReleaseDC()`
- `Convert_UI32_AARRGGBB()` (converts image format → 32-bit ARGB)
- `CreateBitmap()`, `CreateIconIndirect()`, `SendMessage(WM_SETICON)`, `DeleteObject()`, `DestroyIcon()`

**Globals read:** None
**Globals mutated:** None

**Side effects:**
- Creates GDI bitmap from image data
- Creates icon from bitmap pair (color + mask)
- Sets window small and large icons via WM_SETICON
- Cleans up old icons

**Notes:** Converts input format to RGBA via Convert_UI32_AARRGGBB; mono mask created via CreateBitmap().

---

### `a3dSetIconData` (mswin.cpp:1680-1684)

**Signature:** `bool a3dSetIconData(A3D_WND* wnd, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)`

**Purpose:** Public wrapper for _a3dSetIconData.

**Called by:** game.cpp (assumed)

**Calls:** `_a3dSetIconData()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Delegates to _a3dSetIconData.

**Return Value:** Always true

**Notes:** Wrapper provides consistent return type contract.

---

### `a3dSetIcon` (mswin.cpp:1686-1689)

**Signature:** `bool a3dSetIcon(A3D_WND* wnd, const char* path)`

**Purpose:** Load PNG from file and set as window icon.

**Called by:** game.cpp (assumed)

**Calls:** `a3dLoadImage()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Loads image, creates icon, updates window.

**Return Value:** true on success (from a3dLoadImage)

**Notes:** Convenience wrapper combining a3dLoadImage + _a3dSetIconData.

---

### `a3dListDir` (mswin.cpp:1691-1716)

**Signature:** `int a3dListDir(const char* dir_path, bool(*cb)(A3D_DirItem item, const char* name, void* cookie), void* cookie)`

**Purpose:** List directory contents, invoke callback for each entry.

**Called by:** game.cpp file browser (assumed)

**Calls:** `FindFirstFileA()`, `FindNextFileA()`, `FindClose()`, snprintf

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Return Value:** -1 on error (invalid dir), 0 if empty, n=count of entries enumerated

**Notes:** Callback returns false to break early; callback receives A3D_DIRECTORY or A3D_FILE item type.

---

### `a3dSetCurDir` (mswin.cpp:1718-1721)

**Signature:** `bool a3dSetCurDir(const char* dir_path)`

**Purpose:** Change current working directory.

**Called by:** game.cpp file UI (assumed)

**Calls:** `SetCurrentDirectoryA()`

**Globals read:** OS CWD state
**Globals mutated:** OS CWD state

**Side effects:** Changes process working directory

**Return Value:** Result from SetCurrentDirectoryA

**Notes:** None

---

### `a3dGetCurDir` (mswin.cpp:1723-1732)

**Signature:** `bool a3dGetCurDir(char* dir_path, int size)`

**Purpose:** Get current working directory with trailing backslash.

**Called by:** game.cpp file UI (assumed)

**Calls:** `GetCurrentDirectoryA()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** None

**Return Value:** true if len+1 < size; false if buffer too small or GetCurrentDirectoryA fails

**Notes:** Appends '\\' to dir_path; requires size >= 2+strlen(dir).

---

### `a3dCreateThread` (mswin.cpp:1749-1765)

**Signature:** `A3D_THREAD* a3dCreateThread(void* (*entry)(void*), void* arg)`

**Purpose:** Create a worker thread.

**Called by:** game.cpp async tasks (assumed)

**Calls:** `malloc()`, `CreateThread()` via A3D_THREAD::wrap

**Globals read:** None
**Globals mutated:** None

**Side effects:** Allocates A3D_THREAD struct, creates Win32 thread

**Return Value:** A3D_THREAD handle on success; NULL on CreateThread failure

**Notes:** wrap() static method calls entry(arg) and stores result back in arg field; cleaned up in a3dWaitForThread.

---

### `a3dWaitForThread` (mswin.cpp:1767-1774)

**Signature:** `void* a3dWaitForThread(A3D_THREAD* thread)`

**Purpose:** Wait for thread completion and retrieve return value.

**Called by:** game.cpp (assumed)

**Calls:** `WaitForSingleObject()`, `CloseHandle()`, free

**Globals read:** None
**Globals mutated:** None

**Side effects:** Blocks until thread finishes, frees A3D_THREAD struct

**Return Value:** Value returned by entry function (stored in thread->arg)

**Notes:** None

---

### `a3dCreateMutex` (mswin.cpp:1781-1786)

**Signature:** `A3D_MUTEX* a3dCreateMutex()`

**Purpose:** Create a mutual exclusion lock.

**Called by:** game.cpp (assumed)

**Calls:** `malloc()`, `InitializeCriticalSection()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Allocates A3D_MUTEX struct

**Return Value:** A3D_MUTEX handle

**Notes:** None

---

### `a3dDeleteMutex` (mswin.cpp:1788-1792)

**Signature:** `void a3dDeleteMutex(A3D_MUTEX* mutex)`

**Purpose:** Destroy a mutex.

**Called by:** game.cpp (assumed)

**Calls:** `DeleteCriticalSection()`, free

**Globals read:** None
**Globals mutated:** None

**Side effects:** Frees mutex struct

**Notes:** None

---

### `a3dMutexLock` (mswin.cpp:1794-1797)

**Signature:** `void a3dMutexLock(A3D_MUTEX* mutex)`

**Purpose:** Acquire mutex (blocks if held).

**Called by:** game.cpp (assumed)

**Calls:** `EnterCriticalSection()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Blocks until mutex acquired

**Notes:** None

---

### `a3dMutexUnlock` (mswin.cpp:1799-1802)

**Signature:** `void a3dMutexUnlock(A3D_MUTEX* mutex)`

**Purpose:** Release mutex.

**Called by:** game.cpp (assumed)

**Calls:** `LeaveCriticalSection()`

**Globals read:** None
**Globals mutated:** None

**Side effects:** Unlocks critical section

**Notes:** None

---

## Disabled Functions (Unix PTY Support)

Functions a3dOpenPty, a3dReadPTY, a3dWritePTY, a3dResizePTY, a3dClosePTY (lines 1805-1940) are disabled via `#if 0` on Windows. Original implementation used forkpty for Unix; Windows support via CreateProcess + pipe I/O not implemented. Commented code references WM_COPYDATA message channel for resize notifications.

---

## Data Contracts

### [DATA-CONTRACT:KEYINFO]
- `ki_to_vk[]` (lines 141-274): KeyInfo enum → VK_* codes
- `vk_to_ki[]` (lines 281-539): VK_* codes → KeyInfo enum
- Both must stay synchronized with platform.h KeyInfo enum changes

### [FLOW:INPUT]
Stage 1 (Win32 messages) → Stage 2 (a3dWndProc dispatches) → Stage 3 (platform_api callbacks)

### [PLATFORM:WIN32]
- Window class registration on first a3dOpen()
- Pixel format descriptor negotiated per GraphicsDesc
- WGL context created via wglCreateContextAttribsARB with version/profile from gd
- Multiple windows share context via wglCreateContextAttribsARB(..., share ? share->rc : 0, ...)
- Buffer swaps batched via wglSwapMultipleBuffers() in a3dLoop()

### [PLATFORM:TIMING]
- QueryPerformanceCounter for microsecond precision
- Baseline refreshed every 60s (WM_TIMER) to prevent overflow
- a3dGetTime() = coarse_micro + (counter - coarse_perf) * 1000000 / timer_freq

---

## Integration Points

- **platform.h**: Defines PlatformInterface callback table, KeyInfo, MouseInfo, GraphicsDesc
- **game.cpp**: Provides PlatformInterface implementation (init, render, resize, keyb_key, keyb_char, keyb_focus, mouse, close)
- **upng.h**: PNG image loading (vendored library)

