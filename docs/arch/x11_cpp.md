# x11.cpp — X11/GLX Platform Backend Analysis

X11/GLX native windowing and input system for Linux, BSD, and macOS (with XQuartz).

**Compile flag:** `(defined(__linux__) || defined(__APPLE__)) && !defined(USE_SDL)`

**Dependencies:** X11, GLX, Xinerama, pthread, libutil (pty)

---

## Global State

### `dpy` (Display*)
Global X11 display connection. Opened once in `a3dOpen()`, closed in `a3dLoop()` when last window closes.

### `wnd_head` / `wnd_tail` (A3D_WND*)
Doubly-linked list of all open windows. Maintained across `a3dOpen()`, event loop, and `a3dClose()`.

### `head_pty` / `tail_pty` (A3D_PTY*)
Doubly-linked list of all active pseudo-terminals. Populated by `a3dOpenPty()`, cleared by `a3dClosePTY()`.

---

## Data Structures

### `A3D_WND` (lines 121–147)
Window state record. Contains:
- `win`: X11 Window handle
- `rc`: GLXContext for OpenGL rendering
- `im`/`ic`: X Input Method / Input Context (i18n text input)
- `platform_api`: PlatformInterface callback struct
- `mouse_b`, `mouse_x`, `mouse_y`: Mouse tracking state
- `gwa_width`/`gwa_height`: Cached window dimensions
- `wndrect[4]`: Stored window rect (x,y,w,h)
- `wndmode`: Window mode (NORMAL, FULLSCREEN, FRAMELESS)
- `mapped`: Visibility flag
- `force_key`: Temporary key code for xkey callbacks

### `A3D_PTY` (lines 152–161)
Pseudo-terminal record. Contains:
- `fd`: Master file descriptor
- `pd[2]`: Notification pipe (for interrupting select)
- `pid`: Child process ID
- `vt`: Associated terminal emulator (A3D_VT pointer)

---

## Lookup Tables

### `caps[]` (lines 163–282)
String names for KeyInfo enum values (A3D_NONE, A3D_BACKSPACE, ... A3D_OEM_QUOTATION).

### `ki_to_kc[]` (lines 284–415)
Maps KeyInfo enum → X11 keycode. Used by `a3dGetKeyb()` to poll key state.

### `kc_to_ki[]` (lines 417–548)
Maps X11 keycode (0–127) → KeyInfo enum. Used in KeyPress/KeyRelease event handlers.

---

## GLX Context Management

### `a3dPushContext` (x11.cpp:550-555)

**Signature:**
```c
void a3dPushContext(A3D_PUSH_CONTEXT* ctx)
```

**Purpose:** Save current GLX context to caller-managed buffer for later restoration.

**Called by:** Wrapper code before context switching.

**Calls:** `glXGetCurrentDrawable()`, `glXGetCurrentContext()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Stores context in caller-provided buffer.

**Notes:** Used to avoid overwriting important GL state during temporary context switches in `a3dOpen()` and `a3dClose()`.

---

### `a3dPopContext` (x11.cpp:557-561)

**Signature:**
```c
void a3dPopContext(const A3D_PUSH_CONTEXT* ctx)
```

**Purpose:** Restore saved GLX context.

**Called by:** RAII destructors in `a3dOpen()`, `a3dClose()`.

**Calls:** `glXMakeCurrent()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Restores OpenGL context.

**Notes:** No-op if `dpy` is null.

---

### `a3dSwitchContext` (x11.cpp:563-567)

**Signature:**
```c
void a3dSwitchContext(const A3D_WND* wnd)
```

**Purpose:** Make GLX context current for given window.

**Called by:** Event loop during rendering pass.

**Calls:** `glXMakeCurrent()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Modifies active GL context.

**Notes:** Used to switch between multiple windows during render pass in `a3dLoop()`.

---

## Window Management

### `a3dOpen` (x11.cpp:570-1023)

**Signature:**
```c
A3D_WND* a3dOpen(const PlatformInterface* pi, const GraphicsDesc* gd, A3D_WND* share)
```

**Purpose:** Create new X11 window, initialize GLX context, setup input methods.

**Called by:** Application initialization code.

**Calls:** `XOpenDisplay()`, `glXChooseFBConfig()`, `glXGetVisualFromFBConfig()`, `XCreateColormap()`, `XCreateWindow()`, `glXCreateContextAttribsARB()`, `glXMakeCurrent()`, `glGetString()`, `XOpenIM()`, `XCreateIC()`, `XGetWindowAttributes()`, `malloc()`

**Globals read:** `dpy` (if not null)
**Globals mutated:** `dpy` (opened if null), `wnd_head`, `wnd_tail` (window appended to list)

**Side effects:**
- Opens X11 display connection (once)
- Creates X11 window
- Creates GLX context
- Sets up X Input Method for international keyboard input (i18n)
- Adds window to linked list
- Calls `platform_api.init()` callback
- Calls `platform_api.resize()` with initial dimensions

**Notes:**
- RAII PUSH context saves/restores GL state during init
- Framebuffer config selection computes error score (line 748–766)
- XIM setup fails gracefully; ASCII-only input used as fallback (line 916–955)
- `share` parameter unused (commented out on line 823)
- Initial window rect: 800x600 centered on screen if `gd->wnd_xywh` null (line 816–821)
- Window class name set to "A3D" for desktop integration

---

### `a3dClose` (x11.cpp:1426-1482)

**Signature:**
```c
void a3dClose(A3D_WND* wnd)
```

**Purpose:** Destroy window, cleanup GLX context, unlink from window list.

**Called by:** Application shutdown, event loop when WM_DELETE_WINDOW received.

**Calls:** `XSync()`, `glXGetCurrentContext()`, `glXGetCurrentDrawable()`, `glXMakeCurrent()`, `XDestroyIC()`, `XCloseIM()`, `glXDestroyContext()`, `XDestroyWindow()`, `free()`

**Globals read:** `dpy` (if not null)
**Globals mutated:** `wnd_head`, `wnd_tail` (window removed from list)

**Side effects:**
- Destroys X11 window
- Destroys GLX context
- Destroys input method handles
- Unlinks window from global doubly-linked list
- Frees window memory

**Notes:**
- RAII PUSH protects GL state in cleanup path
- Avoids making null context/drawable current (lines 1455–1458)
- Does not close `dpy` (that happens in `a3dLoop()` when list empty)

---

### `a3dLoop` (x11.cpp:1025-1424)

**Signature:**
```c
void a3dLoop()
```

**Purpose:** Main event loop. Processes X11 events, dispatches callbacks, renders all windows.

**Called by:** Application main loop.

**Calls:** `XSync()`, `XPending()`, `XNextEvent()`, `XFilterEvent()`, `glXMakeCurrent()`, `XGetInputFocus()`, `glXSwapBuffers()`, `XCloseDisplay()`

**Globals read:** `dpy` (if not null)
**Globals mutated:** `dpy` (closed when last window closes), `wnd_head`, `wnd_tail`

**Side effects:**
- Processes all pending X11 events
- Dispatches callbacks: `keyb_key()`, `keyb_char()`, `mouse()`, `resize()`, `render()`, `keyb_focus()`, `close()`
- Swaps OpenGL buffers for all mapped windows
- Exits when `wnd_head` becomes null

**Notes:**
- Event poll phase (line 1053–1386): Drain X11 event queue, route to window, dispatch callbacks
- Render phase (line 1388–1419): Iterate windows, call render callback, swap buffers
- ClientMessage (WM_DELETE_WINDOW) unlinks window or calls close callback
- MappingNotify refreshes keyboard state
- ConfigureNotify triggers resize callback
- FocusIn/Out calls keyb_focus callback
- KeyPress dispatches keyb_key (physical key), then keyb_char (text via XIM/XIC or XLookupString)
- UTF-8 decoding in KeyPress handler (lines 1199–1239)
- ButtonPress/Release maps X11 button codes to MouseInfo enum
- MotionNotify dispatches mouse move
- EnterNotify/LeaveNotify dispatch mouse enter/leave

---

## Timing

### `a3dGetTime` (x11.cpp:1496-1501)

**Signature:**
```c
uint64_t a3dGetTime()
```

**Purpose:** Return monotonic microsecond clock.

**Called by:** Game engine for frame timing.

**Calls:** `clock_gettime(CLOCK_MONOTONIC, ...)`

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:**
- Returns uint64_t microseconds (wraps every 584542 years)
- Uses CLOCK_MONOTONIC (never goes backward, unaffected by NTP)
- Static timespec variable for efficiency

---

## Keyboard Input

### `a3dGetKeyb` (x11.cpp:1510-1533)

**Signature:**
```c
bool a3dGetKeyb(A3D_WND* wnd, KeyInfo ki)
```

**Purpose:** Poll physical key state without waiting for events.

**Called by:** Application to check current key state.

**Calls:** `XSync()`, `XQueryKeymap()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** None

**Notes:**
- Returns true if key is currently held
- Fast-path: checks `wnd->force_key` for key being reported by KeyPress event (line 1520–1521)
- Uses `ki_to_kc[]` to map KeyInfo → X11 keycode
- Returns false if keycode unmapped (kc == 0)

---

## Window Properties

### `a3dSetTitle` (x11.cpp:1535-1550)

**Signature:**
```c
void a3dSetTitle(A3D_WND* wnd, const char* name)
```

**Purpose:** Set window title using UTF-8 string.

**Called by:** Application to update title bar.

**Calls:** `strlen()`, `XInternAtom()`, `XChangeProperty()`, `XSync()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Updates _NET_WM_NAME and _NET_WM_ICON_NAME properties.

**Notes:** Sets both icon name and WM name for desktop integration.

---

### `a3dGetTitle` (x11.cpp:1552-1577)

**Signature:**
```c
int a3dGetTitle(A3D_WND* wnd, char* utf8_name, int size)
```

**Purpose:** Retrieve window title into buffer.

**Called by:** Application to read current title.

**Calls:** `XSync()`, `XInternAtom()`, `XGetWindowProperty()`, `strlen()`, `memcpy()`, `XFree()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Copies null-terminated UTF-8 string to buffer.

**Notes:**
- Returns length copied (excluding null terminator)
- Returns 0 on failure
- Reads _NET_WM_NAME property

---

## Window Visibility

### `a3dSetVisible` (x11.cpp:1579-1594)

**Signature:**
```c
void a3dSetVisible(A3D_WND* wnd, bool visible)
```

**Purpose:** Show or hide window.

**Called by:** Application to toggle window visibility.

**Calls:** `XMapWindow()`, `XUnmapWindow()`, `a3dSetRect()`, `XSync()`

**Globals read:** `dpy`

**Globals mutated:** `wnd->mapped`

**Side effects:** Maps/unmaps X11 window.

**Notes:**
- If mapping and wnddirty flag set, applies stored rect
- Updates `wnd->mapped` flag for render pass filtering

---

### `a3dGetVisible` (x11.cpp:1596-1600)

**Signature:**
```c
bool a3dGetVisible(A3D_WND* wnd)
```

**Purpose:** Query window visibility.

**Called by:** Application.

**Calls:** `XSync()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** None

**Notes:** Returns `wnd->mapped` flag.

---

### `a3dIsMaximized` (x11.cpp:1602-1605)

**Signature:**
```c
bool a3dIsMaximized(A3D_WND* wnd)
```

**Purpose:** Query maximized state.

**Called by:** Application.

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Stub; always returns false. Not implemented.

---

## Window Geometry

### `a3dGetRect` (x11.cpp:1608-1677)

**Signature:**
```c
WndMode a3dGetRect(A3D_WND* wnd, int* xywh, int* client_wh)
```

**Purpose:** Query window position and size, accounting for frame decorations.

**Called by:** Application to retrieve current geometry.

**Calls:** `XSync()`, `XInternAtom()`, `XGetWindowProperty()`, `XGetGeometry()`, `XTranslateCoordinates()`, `XFree()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Fills output buffers.

**Notes:**
- `xywh`: [x, y, w, h] including frame decorations
- `client_wh`: [w, h] client area only
- Queries _NET_FRAME_EXTENTS to account for WM decorations (line 1624–1646)
- Returns current window mode (NORMAL, FULLSCREEN, FRAMELESS)

---

### `a3dSetRect` (x11.cpp:1679-1891)

**Signature:**
```c
bool a3dSetRect(A3D_WND* wnd, const int* xywh, WndMode wnd_mode)
```

**Purpose:** Set window position, size, and mode (normal/fullscreen/frameless).

**Called by:** Application to resize/move window or toggle fullscreen.

**Calls:** `a3dGetRect()`, `XineramaQueryScreens()`, `XClientMessageEvent`, `XSendEvent()`, `XMoveResizeWindow()`, `XChangeProperty()`, `XFree()`, `XSync()`

**Globals read:** `wnd->wndmode`, `wnd->wndrect`, `wnd->wnddirty`
**Globals mutated:** `wnd->wndmode`, `wnd->wndrect`, `wnd->wnddirty`

**Side effects:**
- Updates window geometry and mode
- Sends _NET_WM_STATE ClientMessage for fullscreen/unfullscreen
- Sets _MOTIF_WM_HINTS for frame decoration toggling

**Notes:**
- If window unmapped, stores rect/mode and defers application (line 1684–1701)
- Fullscreen mode (line 1709–1778): Uses Xinerama to find monitor, sends _NET_WM_FULLSCREEN_MONITORS and _NET_WM_STATE ClientMessages
- Normal/Frameless mode (line 1780–1887): Toggles frame decorations via _MOTIF_WM_HINTS
- On exit from fullscreen to normal, re-applies stored rect accounting for frame extents

---

## Mouse Input

### `a3dGetMouse` (x11.cpp:1894-1919)

**Signature:**
```c
MouseInfo a3dGetMouse(A3D_WND* wnd, int* x, int* y)
```

**Purpose:** Poll current mouse position and button state.

**Called by:** Application to read mouse state without waiting for events.

**Calls:** `XQueryPointer()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Fills output pointers.

**Notes:**
- Returns MouseInfo with MOVE flag and button state
- Returns 0 if XQueryPointer fails

---

## Window Cookies

### `a3dSetCookie` (x11.cpp:1921-1924)

**Signature:**
```c
void a3dSetCookie(A3D_WND* wnd, void* cookie)
```

**Purpose:** Store opaque user pointer in window.

**Called by:** Application to attach data to window.

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Modifies `wnd->cookie`.

**Notes:** Simple setter for window user data.

---

### `a3dGetCookie` (x11.cpp:1926-1929)

**Signature:**
```c
void* a3dGetCookie(A3D_WND* wnd)
```

**Purpose:** Retrieve opaque user pointer from window.

**Called by:** Application callback handlers to access window data.

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple getter for window user data.

---

## Input Focus

### `a3dSetFocus` (x11.cpp:1931-1934)

**Signature:**
```c
void a3dSetFocus(A3D_WND* wnd)
```

**Purpose:** Give keyboard focus to window.

**Called by:** Application to focus window.

**Calls:** `XSetInputFocus()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Changes X11 input focus.

**Notes:** Uses RevertToNone, CurrentTime.

---

### `a3dGetFocus` (x11.cpp:1936-1942)

**Signature:**
```c
bool a3dGetFocus(A3D_WND* wnd)
```

**Purpose:** Check if window has keyboard focus.

**Called by:** Application to test focus state.

**Calls:** `XGetInputFocus()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** None

**Notes:** Compares window ID to current focus.

---

## Text Input

### `a3dCharSync` (x11.cpp:1944-1948)

**Signature:**
```c
void a3dCharSync(A3D_WND* wnd)
```

**Purpose:** Reset X Input Context (clear preedit buffer for IME).

**Called by:** Application when text input complete.

**Calls:** `Xutf8ResetIC()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Resets XIC input state.

**Notes:** Calls no-op if `wnd->ic` null.

---

## File & Image I/O

### `a3dLoadImage` (x11.cpp:1952-1988)

**Signature:**
```c
bool a3dLoadImage(const char* path, void* cookie, void(*cb)(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf))
```

**Purpose:** Load PNG/image file asynchronously via callback.

**Called by:** Application to load icon, texture, etc.

**Calls:** `upng_new_from_file()`, `upng_get_error()`, `upng_decode()`, `upng_get_format()`, `upng_get_width()`, `upng_get_height()`, `upng_get_bpp()`, `upng_get_buffer()`, `upng_get_pal_buffer()`, `upng_get_pal_size()`, `upng_free()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Calls callback immediately with loaded image data.

**Notes:**
- Uses uPNG library for image decoding
- Callback called synchronously (not queued)
- Returns false on decode error

---

### `_a3dSetIconData` (x11.cpp:1990-2010)

**Signature:**
```c
void _a3dSetIconData(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)
```

**Purpose:** Set window icon from decoded image data (callback for `a3dLoadImage()`).

**Called by:** `a3dLoadImage()` as completion callback, or `a3dSetIconData()`.

**Calls:** `XInternAtom()`, `malloc()`, `Convert_UL_AARRGGBB()`, `XChangeProperty()`, `free()`

**Globals read:** `dpy`

**Globals mutated:** None

**Side effects:** Sets _NET_WM_ICON property on window.

**Notes:**
- Allocates temporary buffer [w, h, pixel_data]
- Converts image to 0xAARRGGBB unsigned long format
- Uses XChangeProperty to set icon

---

### `a3dSetIconData` (x11.cpp:2012-2016)

**Signature:**
```c
bool a3dSetIconData(A3D_WND* wnd, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)
```

**Purpose:** Set window icon from pre-decoded image buffer.

**Called by:** Application to set icon directly.

**Calls:** `_a3dSetIconData()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Sets _NET_WM_ICON property.

**Notes:** Wrapper around `_a3dSetIconData()`.

---

### `a3dSetIcon` (x11.cpp:2018-2021)

**Signature:**
```c
bool a3dSetIcon(A3D_WND* wnd, const char* path)
```

**Purpose:** Load window icon from PNG file.

**Called by:** Application to set icon from file.

**Calls:** `a3dLoadImage()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Sets window icon.

**Notes:** Delegates to `a3dLoadImage()` with `_a3dSetIconData()` callback.

---

## Directory Listing

### `a3dListDir` (x11.cpp:2023-2070)

**Signature:**
```c
int a3dListDir(const char* dir_path, bool (*cb)(A3D_DirItem item, const char* name, void* cookie), void* cookie)
```

**Purpose:** List directory contents, invoking callback for each entry.

**Called by:** Application file browser, asset loader, etc.

**Calls:** `opendir()`, `readdir()`, `lstat()`, `closedir()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Calls callback for each directory/file entry.

**Notes:**
- Returns count of items visited, or -1 on opendir failure
- Callback can return false to stop iteration
- Handles DT_UNKNOWN by stat'ing file to determine type
- Skips symlinks, devices, etc. (only FILES and DIRECTORIES)

---

## Working Directory

### `a3dSetCurDir` (x11.cpp:2072-2075)

**Signature:**
```c
bool a3dSetCurDir(const char* dir_path)
```

**Purpose:** Change working directory.

**Called by:** Application to change cwd.

**Calls:** `chdir()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Changes process working directory.

**Notes:** Returns true on success.

---

### `a3dGetCurDir` (x11.cpp:2077-2091)

**Signature:**
```c
bool a3dGetCurDir(char* dir_path, int size)
```

**Purpose:** Retrieve current working directory into buffer.

**Called by:** Application to query cwd.

**Calls:** `getcwd()`, `strlen()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Fills output buffer with cwd path + trailing slash.

**Notes:**
- Appends trailing slash to path (line 2086–2087)
- Returns false if buffer null or getcwd fails

---

## Threading

### `a3dCreateThread` (x11.cpp:2098-2108)

**Signature:**
```c
A3D_THREAD* a3dCreateThread(void* (*entry)(void*), void* arg)
```

**Purpose:** Create worker thread.

**Called by:** Application to spawn background tasks.

**Calls:** `pthread_create()`, `malloc()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Allocates thread handle, starts pthread.

**Notes:**
- Returns null if pthread_create fails
- Thread runs immediately

---

### `a3dWaitForThread` (x11.cpp:2110-2116)

**Signature:**
```c
void* a3dWaitForThread(A3D_THREAD* thread)
```

**Purpose:** Wait for thread completion and retrieve exit value.

**Called by:** Application to join thread.

**Calls:** `pthread_join()`, `free()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Blocks until thread exits, frees thread handle.

**Notes:** Returns void* exit status from thread entry function.

---

## Mutexes

### `a3dCreateMutex` (x11.cpp:2123-2128)

**Signature:**
```c
A3D_MUTEX* a3dCreateMutex()
```

**Purpose:** Create mutual exclusion lock.

**Called by:** Application for thread synchronization.

**Calls:** `malloc()`, `pthread_mutex_init()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Allocates and initializes mutex.

**Notes:** Returns heap-allocated mutex.

---

### `a3dDeleteMutex` (x11.cpp:2130-2134)

**Signature:**
```c
void a3dDeleteMutex(A3D_MUTEX* mutex)
```

**Purpose:** Destroy mutex.

**Called by:** Application cleanup.

**Calls:** `pthread_mutex_destroy()`, `free()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Destroys mutex, frees memory.

**Notes:** Behavior undefined if mutex held.

---

### `a3dMutexLock` (x11.cpp:2136-2139)

**Signature:**
```c
void a3dMutexLock(A3D_MUTEX* mutex)
```

**Purpose:** Acquire lock (blocking).

**Called by:** Application thread synchronization.

**Calls:** `pthread_mutex_lock()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Blocks until lock acquired.

**Notes:** Deadlock if thread already holds lock (depends on pthread_mutexattr).

---

### `a3dMutexUnlock` (x11.cpp:2141-2144)

**Signature:**
```c
void a3dMutexUnlock(A3D_MUTEX* mutex)
```

**Purpose:** Release lock.

**Called by:** Application thread synchronization.

**Calls:** `pthread_mutex_unlock()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Wakes blocked thread if any.

**Notes:** Behavior undefined if thread does not hold lock.

---

## Pseudo-Terminal Management

### `a3dSetPtyVT` (x11.cpp:2146-2149)

**Signature:**
```c
void a3dSetPtyVT(A3D_PTY* pty, A3D_VT* vt)
```

**Purpose:** Attach terminal emulator to PTY.

**Called by:** Application to link terminal to pty.

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Modifies `pty->vt`.

**Notes:** Simple setter for terminal association.

---

### `a3dGetPtyVT` (x11.cpp:2151-2154)

**Signature:**
```c
A3D_VT* a3dGetPtyVT(A3D_PTY* pty)
```

**Purpose:** Retrieve terminal emulator from PTY.

**Called by:** Application to get associated terminal.

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple getter.

---

### `a3dOpenPty` (x11.cpp:2229-2296)

**Signature:**
```c
A3D_PTY* a3dOpenPty(int w, int h, const char* path, char* const argv[], char* const envp[])
```

**Purpose:** Create pseudo-terminal and fork child process.

**Called by:** Application to start embedded terminal or shell.

**Calls:** `malloc()`, `pipe()`, `forkpty()`, `ioctl()`, `execvpe()`, `close()`

**Globals read:** None
**Globals mutated:** `head_pty`, `tail_pty` (pty appended to list)

**Side effects:**
- Allocates PTY handle
- Forks child process attached to pty
- Child executes `execvpe(path, argv, envp)`
- Parent returns PTY handle

**Notes:**
- PTY dimensions set to w×h (line 2243–2246)
- Child process replaces itself via execvpe (line 2263)
- Uses `forkpty()` (single call, Unix only)
- Parent appends PTY to global list (line 2282–2288)
- Returns null on fork/pty failure

---

### `a3dReadPTY` (x11.cpp:2299-2312)

**Signature:**
```c
int a3dReadPTY(A3D_PTY* pty, void* buf, size_t size)
```

**Purpose:** Read data from PTY master (non-blocking with notification support).

**Called by:** Application to receive terminal output.

**Calls:** `select()`, `read()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Blocks until data available or notification pipe signaled.

**Notes:**
- Waits on both `pty->fd` (terminal) and `pty->pd[0]` (notification pipe)
- Returns -1 if notification pipe signaled (interrupt)
- Returns bytes read from pty, or 0 on EOF

---

### `a3dWritePTY` (x11.cpp:2314-2317)

**Signature:**
```c
int a3dWritePTY(A3D_PTY* pty, const void* buf, size_t size)
```

**Purpose:** Write data to PTY master (to child process stdin).

**Called by:** Application to send input to terminal.

**Calls:** `write()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to child process.

**Notes:** Returns bytes written.

---

### `a3dResizePTY` (x11.cpp:2319-2335)

**Signature:**
```c
void a3dResizePTY(A3D_PTY* pty, int w, int h)
```

**Purpose:** Notify child process of PTY size change (TIOCSWINSZ).

**Called by:** Application when terminal window resized.

**Calls:** `ioctl(TIOCSWINSZ)`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Updates PTY dimensions, child receives SIGWINCH.

**Notes:**
- Retries ioctl on EINTR (line 2328–2331)
- Commented out SIGKILL at line 2334

---

### `a3dUnblockPTY` (x11.cpp:2337-2344)

**Signature:**
```c
void a3dUnblockPTY(A3D_PTY* pty)
```

**Purpose:** Signal PTY read to unblock (for interrupt).

**Called by:** `a3dClosePTY()` to wake blocked read.

**Calls:** `write()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes to notification pipe.

**Notes:** Unblocks pending select() in `a3dReadPTY()`.

---

### `a3dClosePTY` (x11.cpp:2346-2370)

**Signature:**
```c
void a3dClosePTY(A3D_PTY* pty)
```

**Purpose:** Close PTY, wait for child, cleanup.

**Called by:** Application to terminate embedded terminal.

**Calls:** `a3dUnblockPTY()`, `close()`, `waitpid()`, `free()`

**Globals read:** None
**Globals mutated:** `head_pty`, `tail_pty` (pty removed from list)

**Side effects:**
- Signals unblock
- Closes master fd
- Waits for child process
- Closes notification pipe
- Unlinks PTY from global list
- Frees PTY memory

**Notes:**
- Commented out SIGKILL, explicit write (line 2351–2352)
- PTY removed from list before free

---

## End of File Guard

Conditional compilation (lines 2372–2373):
```c
#endif // USE_SDL
#endif // __linux__ or __APPLE__
```

Entire file is X11-only. Compiled only on Linux/macOS when SDL backend not selected.

