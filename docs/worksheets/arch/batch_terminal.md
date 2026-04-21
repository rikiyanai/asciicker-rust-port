# Terminal Batch Analysis — term.cpp + terminal.cpp + term.h

**Files analyzed:** `term.cpp` (920 lines), `terminal.cpp` (441 lines), `term.h` (75 lines)  
**Total functions documented:** 22  
**Purpose:** OpenGL terminal emulator (term.cpp) + PTY-based pure terminal mode (terminal.cpp)

---

## Architecture Overview

### Two Rendering Modes

**term.cpp (OpenGL Terminal Emulator)**
- Renders AnsiCell buffer to GPU texture using OpenGL shaders
- Modern graphics pipeline: vertex shader + fragment shader with palette mapping
- 6-stage rendering: Game→AnsiCell→GPU texture→quad rendering→palette lookup→blend
- Multi-window support via linked list (TERM_LIST)
- Platform integration via PlatformInterface callbacks
- Used by: asciiid.cpp (editor), game_app.cpp (standalone game)

**terminal.cpp (PTY Pure Terminal)**
- Renders directly to native terminal via ANSI escape sequences
- No OpenGL/X11 dependencies (SSH-friendly)
- 3-thread architecture: main coordinator + read pump (PTY→terminal) + write pump (keyboard→PTY)
- Spawns bash shell in child process via forkpty()
- Standalone program (makefile_game_term)

---

## term.h — Public API (75 lines)

### `TermOpen` (term.h:71, term.cpp:756-811)

**Signature:** `Game* TermOpen(A3D_WND* share, float yaw, float pos[3], void(*close)() = 0)`  
**Purpose:** Create new OpenGL terminal window with independent game instance  
**Called by:** `asciiid.cpp:7571, 7592, 11138`, `game_app.cpp:2214`  
**Calls:** `a3dOpen()`, `a3dGetCookie()`, `a3dSetCookie()`, `term_init()` (via PlatformInterface.init callback)  
**Globals read:** None  
**Globals mutated:** `term_head`, `term_tail` (via term_init adding TERM_LIST node)  
**Side effects:** 
- Allocates TERM_LIST node (malloc)
- Creates OpenGL window via a3dOpen()
- Registers PlatformInterface callbacks (term_render, term_resize, term_keyb_*, term_mouse)
- Initializes OpenGL resources (textures, shaders, VAO/VBO) via term_init callback
- Adds window to global linked list (term_head/term_tail)

**Notes:** 
- `share` parameter enables OpenGL context sharing (texture/buffer sharing between windows), can be NULL
- `close` callback is optional cleanup hook invoked when window closes
- Creates independent Game instance per window (no shared state)
- Window rect defaults to 800×600 at (1920+800, 300) — hardcoded in gd.wnd_xywh
- Returns Game* pointer for caller to manipulate game state

---

### `TermCloseAll` (term.h:72, term.cpp:813-832)

**Signature:** `void TermCloseAll()`  
**Purpose:** Close all terminal windows and cleanup OpenGL resources  
**Called by:** `asciiid.cpp:5900, 11216`  
**Calls:** `a3dPushContext()`, `a3dSwitchContext()`, `term_close()`, `a3dPopContext()`  
**Globals read:** `term_head`  
**Globals mutated:** `term_head`, `term_tail` (resets to NULL)  
**Side effects:**
- Traverses linked list of TERM_LIST nodes (term_head → term_tail)
- For each window: deletes OpenGL textures/shaders/buffers, frees Game instance, closes window
- Resets linked list to empty (term_head = term_tail = 0)
- Context switching ensures correct OpenGL context for resource deletion

**Notes:**
- Called on program exit (main shutdown sequence)
- Uses A3D_PUSH_CONTEXT to preserve caller's OpenGL context
- Iterates via `next` pointer (not prev) to traverse forward

---

### `TermResizeAll` (term.h:73, term.cpp:834-845)

**Signature:** `void TermResizeAll()`  
**Purpose:** Resize all terminal windows to match current viewport dimensions  
**Called by:** `asciiid.cpp:1706, 1718, 8810, 8819`, `game_app.cpp:1060, 1117`  
**Calls:** `a3dGetRect()`, `term_resize()`  
**Globals read:** `term_head`  
**Globals mutated:** None (term_resize mutates per-window state)  
**Side effects:**
- Traverses linked list of TERM_LIST nodes
- For each window: calls term_resize() to recompute cell dimensions and notify Game::OnSize()

**Notes:**
- Called when font size changes (NextGLFont/PrevGLFont) or display scaling changes
- Does not require context switching (term_resize uses window cookie, not active context)

---

### `TermApplyPlayerSkin` (term.h:74, term.cpp:847-882)

**Signature:** `int TermApplyPlayerSkin(Sprite* fallback)`  
**Purpose:** Update player sprite across all terminal windows (hot skin swap)  
**Called by:** `asciiid.cpp:7582`  
**Calls:** `GetSprite()`, `UpdateSpriteInst()`, `GetInstWorld()`  
**Globals read:** `term_head`  
**Globals mutated:** `g->player.sprite` for each Game instance  
**Side effects:**
- Traverses linked list of TERM_LIST nodes
- For each game: resolves sprite from player.req + player.clr, falls back to fallback parameter
- Updates `g->player.sprite` and `g->player_inst` (world sprite instance)
- Returns count of updated windows

**Notes:**
- Enables live sprite replacement without restarting game instances
- Used when editor loads new sprite (e.g., user selects different player character)
- Returns 0 if no windows updated (all windows closed or sprite resolution failed)

---

## term.cpp — OpenGL Terminal Implementation (920 lines)

### `term_render` (term.cpp:223-367)

**Signature:** `void term_render(A3D_WND* wnd)`  
**Purpose:** Main render loop — upload AnsiCell buffer to GPU, draw fullscreen quad with palette mapping  
**Called by:** PlatformInterface.render callback (registered in TermOpen)  
**Calls:** `a3dGetCookie()`, `a3dGetRect()`, `GetGLFont()`, `server->Proc()`, `a3dGetTime()`, `Game::Render()`, `glClearColor()`, `glClear()`, `sprintf()`, `a3dSetTitle()`, `gl3TextureSubImage2D()`, `glViewport()`, `glUseProgram()`, `glUniform*()`, `glActiveTexture()`, `glBindTexture()`, `glBindVertexArray()`, `glDrawArrays()`, `fprintf()`, `fflush()`  
**Globals read:** `server` (optional MCP server), `render_break_point` (debug)  
**Globals mutated:** `perf_*` static locals (profiling state)  
**Side effects:**
- Reads TERM_LIST from window cookie
- Processes server messages if server != NULL
- Renders AnsiCell buffer via 6-stage pipeline:
  1. CPU: Game::Render() fills term->buf with AnsiCell structs
  2. CPU→GPU: Upload buffer to GL_RGBA8 texture via gl3TextureSubImage2D
  3. GPU Vertex Shader: Transform fullscreen quad UV → cell coordinates
  4. GPU Fragment Shader: Sample AnsiCell texture at cell center, extract fg/bk/glyph
  5. GPU Fragment Shader: Map palette indices to RGB via Pal() function (6×6×6 cube)
  6. GPU Fragment Shader: Blend FG/BG colors using glyph alpha (mix)
- Writes profiling data to stderr or ASCIICKER_PROFILE_LOG if enabled
- Updates window title with viewport dimensions

**Notes:**
- **WHY 160×90 max dimensions:** Typical retro terminal size, constraint enforced at lines 242-245
- **WHY palette mapping on GPU:** RGB cube formula (lines 532-542) computes 256-color xterm palette in parallel for all pixels
  - Indices 16-231: 6×6×6 RGB cube (216 colors)
  - Formula: p' = p - 16, blue = p'/36, green = (p' mod 36)/6, red = p' mod 6, scale by 0.2
  - WHY 0.2 scale: 6 discrete steps (0-5) map to [0.0, 1.0], so 5 × 0.2 = 1.0
- **Profiling:** ASCIICKER_PROFILE=1 enables timing, measures render (CPU) vs present (GPU) time + FPS
- **Viewport centering:** Calculates vp_xy to center cell grid in window (lines 292-300)

---

### `term_mouse` (term.cpp:369-406)

**Signature:** `void term_mouse(A3D_WND* wnd, int x, int y, MouseInfo mi)`  
**Purpose:** Route mouse events from platform layer to Game::OnMouse()  
**Called by:** PlatformInterface.mouse callback (registered in TermOpen)  
**Calls:** `a3dGetCookie()`, `Game::ScreenToCell()`, `Game::OnMouse()`  
**Globals read:** None  
**Globals mutated:** `render_break_point` (debug, on MIDDLE_DN)  
**Side effects:**
- Reads TERM_LIST from window cookie
- Dispatches MouseInfo events to game:
  - MOVE → MOUSE_MOVE
  - LEFT_DN/UP → MOUSE_LEFT_BUT_DOWN/UP
  - RIGHT_DN/UP → MOUSE_RIGHT_BUT_DOWN/UP
  - WHEEL_UP/DN → MOUSE_WHEEL_UP/DOWN
- Special: MIDDLE_DN sets render_break_point for debugging

**Notes:**
- ScreenToCell() converts pixel coordinates to cell grid coordinates
- render_break_point is debug global (render.cpp) for pixel-level breakpoints

---

### `term_resize` (term.cpp:408-417)

**Signature:** `void term_resize(A3D_WND* wnd, int w, int h)`  
**Purpose:** Notify game of viewport size change (recompute cell dimensions)  
**Called by:** PlatformInterface.resize callback (registered in TermOpen), TermResizeAll()  
**Calls:** `a3dGetCookie()`, `GetGLFont()`, `Game::OnSize()`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:**
- Reads TERM_LIST from window cookie
- Calls Game::OnSize(w, h, cell_w, cell_h) to notify game of viewport change

**Notes:**
- cell_w/cell_h are computed from font dimensions (fnt_wh[0]/fnt_wh[1] >> 4)
- Game::OnSize() typically updates UI layout, camera aspect ratio, etc.

---

### `term_init` (term.cpp:419-675)

**Signature:** `void term_init(A3D_WND* wnd)`  
**Purpose:** Initialize OpenGL resources (textures, shaders, VAO/VBO) for new terminal window  
**Called by:** PlatformInterface.init callback (registered in TermOpen, invoked by a3dOpen)  
**Calls:** `malloc()`, `a3dGetTime()`, `CreateGame()`, `InitGame()`, `gl3CreateTextures()`, `gl3TextureStorage2D()`, `glCreateProgram()`, `glCreateShader()`, `glShaderSource()`, `glCompileShader()`, `glGetShaderInfoLog()`, `glAttachShader()`, `glLinkProgram()`, `glDeleteShader()`, `glGetProgramInfoLog()`, `glGetUniformLocation()`, `glGetAttribLocation()`, `glGetFragDataLocation()`, `glGenBuffers()`, `glBindBuffer()`, `glBufferData()`, `glGenVertexArrays()`, `glBindVertexArray()`, `glVertexAttribPointer()`, `glEnableVertexAttribArray()`, `a3dSetCookie()`, `a3dSetIcon()`, `a3dSetVisible()`  
**Globals read:** `terrain`, `world`, `pos_x`, `pos_y`, `pos_z`, `rot_yaw`, `global_lt`, `probe_z` (ifdef EDITOR), `term_tail`  
**Globals mutated:** `term_head`, `term_tail` (adds new TERM_LIST node)  
**Side effects:**
- Allocates TERM_LIST node (malloc, sizeof(TERM_LIST) = ~200 bytes)
- Creates OpenGL texture (GL_RGBA8, 160×90 cells = 14400 pixels = 57600 bytes)
- Compiles vertex + fragment shaders, links program
- Creates VBO (4 vec2 vertices = 32 bytes) and VAO
- Creates Game instance via CreateGame() + InitGame()
- Adds TERM_LIST node to linked list (term_head/term_tail)
- Sets window cookie, icon, visibility

**Notes:**
- **WHY runtime binding:** Uses glGetUniformLocation/glGetAttribLocation instead of layout(location=N) for OpenGL 3.3 compatibility
- **Shader source:** Vertex shader (lines 453-462) transforms UV → cell_coord, fragment shader (lines 505-568) implements palette mapping
- **VBO data:** Fullscreen quad {0,0, 1,0, 1,1, 0,1} in UV space
- **Game initialization:** InitGame() uses editor globals (pos_x, rot_yaw, etc.) if EDITOR defined
- **Linked list:** Adds to tail (term_tail->next = term, term_tail = term)

---

### `term_keyb_char` (term.cpp:677-681)

**Signature:** `void term_keyb_char(A3D_WND* wnd, wchar_t chr)`  
**Purpose:** Route character input to Game::OnKeyb(KEYB_CHAR)  
**Called by:** PlatformInterface.keyb_char callback (registered in TermOpen)  
**Calls:** `a3dGetCookie()`, `Game::OnKeyb()`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Calls Game::OnKeyb(KEYB_CHAR, chr)

**Notes:** Handles text input (Unicode characters), not key events (use term_keyb_key for key presses)

---

### `term_keyb_key` (term.cpp:683-715)

**Signature:** `void term_keyb_key(A3D_WND* wnd, KeyInfo ki, bool down)`  
**Purpose:** Route keyboard events to Game::OnKeyb() or handle special keys (F11, numpad +/-)  
**Called by:** PlatformInterface.keyb_key callback (registered in TermOpen)  
**Calls:** `a3dGetCookie()`, `NextGLFont()`, `PrevGLFont()`, `ToggleFullscreen()`, `Game::OnKeyb()`  
**Globals read:** None  
**Globals mutated:** None (NextGLFont/PrevGLFont/ToggleFullscreen mutate their own state)  
**Side effects:**
- Handles special keys:
  - A3D_NUMPAD_ADD → NextGLFont() (increase font size)
  - A3D_NUMPAD_SUBTRACT → PrevGLFont() (decrease font size)
  - A3D_F5..F8 → Game::OnKeyb(KEYB_PRESS, ki) (if not auto-repeat)
  - A3D_F11 → ToggleFullscreen(term->game)
  - Others → Game::OnKeyb(KEYB_DOWN/KEYB_UP, ki)

**Notes:**
- Auto-repeat filtering: F5-F8 only send KEYB_PRESS if !(ki & A3D_AUTO_REPEAT)
- F11 does not send KEYB_UP event (handled specially)

---

### `term_keyb_focus` (term.cpp:717-721)

**Signature:** `void term_keyb_focus(A3D_WND* wnd, bool set)`  
**Purpose:** Notify game of focus change (window activation/deactivation)  
**Called by:** PlatformInterface.keyb_focus callback (registered in TermOpen)  
**Calls:** `a3dGetCookie()`, `Game::OnFocus()`  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** Calls Game::OnFocus(set)

**Notes:** Game::OnFocus() typically pauses/unpauses game, stops sound, etc.

---

### `term_close` (term.cpp:723-754)

**Signature:** `void term_close(A3D_WND* wnd)`  
**Purpose:** Cleanup OpenGL resources and free game instance when window closes  
**Called by:** PlatformInterface.close callback (registered in TermOpen), TermCloseAll()  
**Calls:** `a3dGetCookie()`, `FreeGame()`, `DeleteGame()`, `glDeleteTextures()`, `glDeleteVertexArrays()`, `glDeleteBuffers()`, `glDeleteProgram()`, `a3dClose()`, `free()`  
**Globals read:** None  
**Globals mutated:** `term_head`, `term_tail` (removes TERM_LIST node from linked list)  
**Side effects:**
- Frees game instance via FreeGame() + DeleteGame()
- Deletes OpenGL resources (texture, VAO, VBO, program)
- Invokes term->close callback if set (custom cleanup hook)
- Closes platform window via a3dClose()
- Removes TERM_LIST node from linked list (updates prev/next pointers)
- Frees TERM_LIST node (free)

**Notes:**
- Linked list removal: Updates head/tail if removing first/last node
- close callback is optional (NULL check before invoke)

---

### `ToggleFullscreen` (term.cpp:884-901)

**Signature:** `void ToggleFullscreen(Game* g)`  
**Purpose:** Toggle fullscreen mode for window containing game instance  
**Called by:** `term_keyb_key()` (F11 press), `mainmenu.cpp:2026`, `game.cpp` (via game_app.cpp/game_web.cpp)  
**Calls:** `a3dGetRect()`, `a3dSetRect()`  
**Globals read:** `term_head`  
**Globals mutated:** None (a3dSetRect mutates platform window state)  
**Side effects:**
- Searches linked list for TERM_LIST node with matching game instance
- Toggles window mode: A3D_WND_FULLSCREEN ↔ A3D_WND_NORMAL

**Notes:**
- Returns early if game not found in linked list
- Platform layer handles actual fullscreen transition (a3dSetRect)

---

### `IsFullscreen` (term.cpp:903-919)

**Signature:** `bool IsFullscreen(Game* g)`  
**Purpose:** Query fullscreen state of window containing game instance  
**Called by:** `mainmenu.cpp:2025, 2031, 2041` (mainmenu fullscreen toggle logic)  
**Calls:** `a3dGetRect()`  
**Globals read:** `term_head`  
**Globals mutated:** None  
**Side effects:** None (read-only query)

**Notes:**
- Returns false if game not found in linked list
- Mainmenu warns: "on web IsFullscreen can be late" (async fullscreen API on web)

---

### TERM_LIST::IsKeyDown (term.cpp:178-181)

**Signature:** `bool IsKeyDown(int key)`  
**Purpose:** Check if key is currently pressed (bitfield lookup)  
**Called by:** No callers found via grep  
**Calls:** None  
**Globals read:** None  
**Globals mutated:** None  
**Side effects:** None (read-only bitfield check)

**Notes:**
- Bitfield storage: 32 bytes × 8 bits = 256 keys max
- Formula: `keys[key >> 3] & (1 << (key & 0x7))` extracts bit for key index
- WHY bitfield: Compact storage (256 keys in 32 bytes vs 256 bytes for bool array)

---

## terminal.cpp — PTY Pure Terminal (441 lines)

### `main` (terminal.cpp:340-440)

**Signature:** `int main(int argc, char** argv)`  
**Purpose:** Setup PTY, spawn bash shell, coordinate read/write threads, cleanup on exit  
**Called by:** OS (entry point for terminal executable)  
**Calls:** `ioctl()`, `forkpty()`, `execl()`, `open()`, `tcgetattr()`, `cfmakeraw()`, `tcsetattr()`, `signal()`, `pthread_create()`, `pthread_join()`, `close()`, `waitpid()`  
**Globals read:** None  
**Globals mutated:** `pty_fd`, `log_fd`, `stop`  
**Side effects:**
- Creates PTY master/slave pair via forkpty()
- Forks child process, execs /bin/bash in PTY slave
- Puts parent terminal in raw mode (no echo, no line buffering)
- Registers SIGWINCH handler for terminal resize
- Creates read thread (PTY→terminal pump) and write thread (keyboard→PTY pump)
- Waits for child process exit, then stops write thread via TIOCSTI wakeup
- Restores parent terminal to original settings
- Opens log file if argv[1] provided (O_WRONLY | O_CREAT | O_TRUNC)

**Notes:**
- **Return codes:**
  - `-2`: Not attached to parent terminal (ioctl TIOCGWINSZ failed)
  - `-1`: forkpty failed or pty_fd < 0
  - `0`: Normal exit
- **WHY forkpty:** Creates PTY master/slave pair AND forks child in single atomic call (vs openpty + fork)
- **WHY raw mode:** Disable echo, line buffering, signal generation to enable direct keyboard passthrough
- **WHY TIOCSTI wakeup:** Write thread blocked in read(STDIN) must be woken cleanly (pthread_cancel is unsafe)
- **Thread coordination:** Read thread exits naturally when child closes stdout, main sets stop flag to wake write thread

---

### `Read` (terminal.cpp:268-284)

**Signature:** `void* Read(void* arg)`  
**Purpose:** PTY → terminal pump (child output to user's screen)  
**Called by:** pthread_create in main()  
**Calls:** `read()`, `write()`, `Escape()`  
**Globals read:** `pty_fd`, `log_fd`, `stop`  
**Globals mutated:** None  
**Side effects:**
- Continuously reads from pty_fd (PTY master)
- Writes to STDOUT_FILENO (user's terminal)
- Logs output to log_fd if logging enabled (via Escape)
- Returns when child closes stdout (EOF) or stop flag set

**Notes:**
- **WHY read thread:** Pumps child process output through PTY master to parent terminal's STDOUT
- **Normal exit:** Child process terminates, closes stdout, read() returns 0 (EOF)
- **Buffer size:** 1024 bytes (const siz)

---

### `Write` (terminal.cpp:291-308)

**Signature:** `void* Write(void* arg)`  
**Purpose:** Keyboard → PTY pump (user input to child process)  
**Called by:** pthread_create in main()  
**Calls:** `read()`, `write()`, `Escape()`  
**Globals read:** `log_fd`, `stop`  
**Globals mutated:** `pty_fd`  
**Side effects:**
- Continuously reads from STDIN_FILENO (keyboard)
- Writes to pty_fd (PTY master → child stdin)
- Logs input to log_fd if logging enabled (via Escape)
- Returns when EOF or stop flag set

**Notes:**
- **WHY write thread:** Pumps keyboard input from parent terminal's STDIN through PTY master to child process
- **Wakeup mechanism:** Main thread uses ioctl(STDIN, TIOCSTI, "!") to inject fake input and unblock read(STDIN)
- **Buffer size:** 1024 bytes (const siz)

---

### `Escape` (terminal.cpp:163-262)

**Signature:** `int Escape(int fd, const char* hdr, int hdrlen, const char* buf, int buflen)`  
**Purpose:** Convert non-printable characters to readable format for logging  
**Called by:** Read(), Write()  
**Calls:** `pthread_mutex_lock()`, `memcpy()`, `write()`, `pthread_mutex_unlock()`  
**Globals read:** None  
**Globals mutated:** `last_hdr` (static local), `log_mutex`  
**Side effects:**
- Writes to fd (log file)
- Converts control characters to escaped format:
  - Printable (>0x20 or <0) → pass through
  - `\r` → `\\r`
  - `\n` → `\\n` + actual newline + hdr
  - `\t` → `\\t`
  - Others → `\\xHH` (hex format)
- Mutex protects log file writes from race conditions (read + write threads)

**Notes:**
- **WHY escape logging:** Debug ANSI escape sequences by making non-printable characters visible
- **Buffer size:** 4096 bytes (const siz)
- **Header logic:** Only prints header once per direction (I:/O:), adds newline when direction changes
- **Thread safety:** pthread_mutex_lock/unlock protects log_fd writes

---

### `SignalHandler` (terminal.cpp:314-338)

**Signature:** `void SignalHandler(int s)`  
**Purpose:** Propagate terminal resize events from parent to PTY slave  
**Called by:** signal(SIGWINCH, SignalHandler) in main()  
**Calls:** `ioctl()`, `sprintf()`, `Escape()`  
**Globals read:** `log_fd`  
**Globals mutated:** None (ioctl mutates PTY slave terminal size)  
**Side effects:**
- Receives SIGWINCH signal when parent terminal resizes
- Reads parent terminal size via ioctl(STDIN, TIOCGWINSZ)
- Logs new size to log_fd if logging enabled
- NOTE: Commented out ioctl(pty_fd, TIOCSWINSZ) — see notes

**Notes:**
- **WHY SIGWINCH handler:** Terminal resize events must be propagated from parent terminal to PTY slave so child process can adjust output formatting
- **Commented propagation:** Lines 329 shows ioctl(pty_fd, TIOCSWINSZ) is commented out
- **Explanation (lines 333-337):** Parent terminal sends SIGWINCH when window resizes, but child process (stty) can also call ioctl(TIOCSWINSZ) to modify PTY size silently — neither we nor parent can respond to child-initiated resize
- **Buffer size:** 64 bytes for size logging

---

## Data Structures

### TERM_LIST (term.cpp:164-205)

**Fields:**
- `TERM_LIST* prev, *next`: Linked list pointers (doubly-linked)
- `A3D_WND* wnd`: Platform window handle
- `void (*close)()`: Optional cleanup callback
- `Game* game`: Game instance for this window
- `float yaw`: Initial camera rotation (unused, commented in TermOpen)
- `uint8_t keys[32]`: Keyboard bitfield (256 keys, 32 bytes)
- `static const int max_width = 160, max_height = 90`: Max terminal dimensions (const)
- `AnsiCell buf[max_width × max_height]`: AnsiCell buffer (14400 cells = 57600 bytes)
- `GLuint tex, prg, vbo, vao`: OpenGL resource handles
- `GLint uni_*, att_*, out_*`: Shader uniform/attribute/output locations

**Size:** ~200 bytes struct + 57600 bytes buffer = ~57.8 KB per window

**Notes:**
- WHY 160×90 max: Typical retro terminal size, large enough for detailed ASCII art while maintaining ~60 FPS on integrated GPUs
- WHY doubly-linked: Enables removal from middle of list (term_close updates prev/next)

---

## Global State

### term.cpp Globals

- `TERM_LIST* term_head, *term_tail` (lines 214-215): Linked list of terminal windows
- `extern Terrain* terrain, World* world` (lines 208-209): Editor subsystems (HACK comment)
- `extern float pos_x, pos_y, pos_z, rot_yaw, global_lt[], int probe_z` (lines 218-221): Editor camera/light state (SUPER_HACK LIVE VIEW comment)
- `extern int render_break_point[]` (line 379): Debug breakpoint (set by MIDDLE_DN mouse)
- `extern Server* server` (line 247): Optional MCP server (processed in term_render)

### terminal.cpp Globals

- `volatile int stop` (line 153): Thread coordination flag (main → write thread)
- `int pty_fd` (line 154): PTY master file descriptor
- `int log_fd` (line 155): Log file descriptor (-1 if disabled)
- `pthread_mutex_t log_mutex` (line 157): Protects log file writes

---

## External Dependencies

### term.cpp Calls Out To

- **Platform layer:** `a3dOpen()`, `a3dClose()`, `a3dGetCookie()`, `a3dSetCookie()`, `a3dGetRect()`, `a3dSetRect()`, `a3dGetTime()`, `a3dSetTitle()`, `a3dSetIcon()`, `a3dSetVisible()`, `a3dPushContext()`, `a3dSwitchContext()`, `a3dPopContext()`
- **OpenGL:** All `gl*` functions (texture, shader, buffer, VAO, program management)
- **Game lifecycle:** `CreateGame()`, `InitGame()`, `FreeGame()`, `DeleteGame()`, `Game::Render()`, `Game::OnSize()`, `Game::OnKeyb()`, `Game::OnMouse()`, `Game::OnFocus()`, `Game::ScreenToCell()`
- **Font management:** `GetGLFont()`, `NextGLFont()`, `PrevGLFont()` (asciiid.cpp/game_app.cpp)
- **Sprite system:** `GetSprite()`, `UpdateSpriteInst()`, `GetInstWorld()`
- **Server:** `server->Proc()` (optional, if server != NULL)

### terminal.cpp Calls Out To

- **POSIX:** `ioctl()`, `forkpty()`, `execl()`, `open()`, `close()`, `read()`, `write()`, `tcgetattr()`, `tcsetattr()`, `cfmakeraw()`, `signal()`, `waitpid()`
- **Pthread:** `pthread_create()`, `pthread_join()`, `pthread_mutex_lock()`, `pthread_mutex_unlock()`
- **Stdlib:** `malloc()`, `free()`, `sprintf()`, `memcpy()`

---

## Key Algorithms

### 6×6×6 RGB Cube Palette Mapping (term.cpp:532-542)

**Formula:** Maps xterm 256-color palette indices to RGB [0,1]³

```glsl
vec3 Pal(float p) {
    p = clamp(floor(p - 16.0 + 0.5), 0.0, 215.0);  // Normalize to [0, 215]
    
    float blue  = floor(p / 36.0);    // Extract blue [0-5] (36 = 6×6)
    p -= 36.0 * blue;                 // Remove blue contribution
    
    float green = floor(p / 6.0);     // Extract green [0-5]
    float red   = p - 6.0 * green;    // Extract red [0-5] (remainder)
    
    return vec3(blue, green, red) * 0.2;  // Scale to [0, 1]
}
```

**WHY 0.2 scale factor:** 6 discrete steps (0-5) need to map to [0.0, 1.0], so step size = 1 / 5 = 0.2  
**Verification:**
- Index 16 (p'=0): blue=0, green=0, red=0 → RGB(0,0,0)×0.2 = (0.0, 0.0, 0.0) ✓ black
- Index 231 (p'=215): blue=5, green=5, red=5 → RGB(5,5,5)×0.2 = (1.0, 1.0, 1.0) ✓ white
- Index 196 (xterm red, p'=180): blue=5, green=0, red=0 → RGB(5,0,0)×0.2 = (1.0, 0.0, 0.0) ✓ red

**Edge cases:**
- Indices 0-15 (system colors): p - 16 < 0 → clamp to 0 → RGB(0,0,0) (should use lookup table)
- Indices 232-255 (grayscale): p - 16 > 215 → clamp to 215 → RGB(5,5,5) (should use linear ramp)

---

### PTY 3-Thread Architecture (terminal.cpp)

**Thread 1: Main Coordinator**
1. ioctl TIOCGWINSZ → get parent terminal size
2. forkpty() → create PTY master/slave + fork child
3. execl bash → child becomes shell
4. cfmakeraw → put parent terminal in raw mode
5. signal SIGWINCH → setup resize handler
6. pthread_create read/write threads → start pumps
7. pthread_join read → wait for child exit
8. stop flag + TIOCSTI → wake write thread
9. pthread_join write → wait for write thread
10. tcsetattr restore → restore terminal
11. waitpid → reap child

**Thread 2: Read (PTY→terminal)**
- Loop: read(pty_fd) → write(STDOUT) → Escape(log_fd)
- Exit: Child closes stdout OR stop flag set

**Thread 3: Write (keyboard→PTY)**
- Loop: read(STDIN) → write(pty_fd) → Escape(log_fd)
- Exit: EOF OR stop flag set
- Wakeup: ioctl(STDIN, TIOCSTI, "!") injects fake input to unblock

**Shutdown sequence:** Child exits → read thread detects EOF → main sets stop → TIOCSTI wakeup → write thread exits → cleanup

---

## Build Targets

### term.cpp
- **Target:** OpenGL terminal emulator (shared library or linked into asciiid/game_app)
- **Defines:** `USE_GL3` (OpenGL 3.3) or `USE_GL45` (OpenGL 4.5)
- **Links:** OpenGL, platform.h backend (SDL/X11/Win32), game.cpp, render.cpp
- **Used by:** asciiid.cpp (map editor), game_app.cpp (standalone game)

### terminal.cpp
- **Target:** Pure terminal mode executable (makefile_game_term)
- **Defines:** `-DPURE_TERM -DUSE_GPM` (optional GPM mouse support)
- **Links:** `-lutil` (forkpty), `-lpthread` (threads), `-lgpm` (optional)
- **Standalone:** Yes (independent executable, not linked into other targets)
- **Build:** `g++ -pthread -o .run/terminal terminal.cpp -lutil`

---

## WHY Two Modes

### OpenGL Mode (term.cpp)
- ✓ GPU acceleration (parallel palette mapping, millions of pixels/frame)
- ✓ Scalable fonts (bilinear filtering, no pixelation)
- ✓ Multi-window support (linked list, shared contexts)
- ✓ Mouse support (Game::OnMouse routing)
- ✗ Requires X11/Wayland/Win32 + OpenGL
- ✗ Not SSH-friendly

### PTY Mode (terminal.cpp)
- ✓ No GUI dependencies (SSH sessions, Linux console, Docker)
- ✓ Native terminal features (scrollback, copy/paste, Unicode)
- ✓ Headless server support (testing/automation)
- ✗ Fixed-width font (terminal emulator's choice)
- ✗ Keyboard-only input (no mouse unless GPM enabled)
- ✗ Single window (no multi-window support)

---

## Performance Notes

### term.cpp Profiling

**Environment:** `ASCIICKER_PROFILE=1` enables timing  
**Output:** stderr or `ASCIICKER_PROFILE_LOG` file  
**Metrics:**
- Render time: Game::Render() CPU work (fill AnsiCell buffer)
- Present time: OpenGL GPU work (upload, shader, draw)
- FPS: Frames per second over 1-second windows

**Example output:** `[perf] render 2.34ms present 0.87ms fps 60.0 (160x90)`

**Implementation:**
- Static locals: `perf_window_start`, `perf_render_sum`, `perf_present_sum`, `perf_frames`
- t0 = before Game::Render, t1 = after, t2 = after glDrawArrays
- Window reset: Every 1000000 µs (1 second)

---

## Thread Safety

### term.cpp
- **NOT thread-safe:** All functions assume single-threaded OpenGL context
- **Linked list:** term_head/term_tail modified without locking (expects single main thread)
- **OpenGL contexts:** Per-thread, TERM_LIST operations assume main thread only

### terminal.cpp
- **Thread-safe logging:** pthread_mutex_lock/unlock protects log_fd writes
- **Atomic stop flag:** `volatile int stop` coordinates thread shutdown
- **POSIX assumptions:** forkpty, ioctl, signal handlers are standard POSIX (Linux/macOS/BSD)

---

## Call Graph Summary

### Public API → Internal

- `TermOpen` → `a3dOpen` (PlatformInterface.init callback) → `term_init` → `CreateGame`, `InitGame`, OpenGL setup
- `TermCloseAll` → `term_close` (per window) → `FreeGame`, `DeleteGame`, OpenGL cleanup
- `TermResizeAll` → `term_resize` (per window) → `Game::OnSize`
- `TermApplyPlayerSkin` → `GetSprite`, `UpdateSpriteInst` (per window)

### PlatformInterface Callbacks

- `pi.render` = `term_render` → `Game::Render`, OpenGL draw
- `pi.resize` = `term_resize` → `Game::OnSize`
- `pi.mouse` = `term_mouse` → `Game::OnMouse`
- `pi.keyb_char` = `term_keyb_char` → `Game::OnKeyb(KEYB_CHAR)`
- `pi.keyb_key` = `term_keyb_key` → `NextGLFont`/`PrevGLFont`/`ToggleFullscreen`/`Game::OnKeyb`
- `pi.keyb_focus` = `term_keyb_focus` → `Game::OnFocus`
- `pi.close` = `term_close` → cleanup

### terminal.cpp Threads

- `main` → `pthread_create(Read)`, `pthread_create(Write)` → `read`/`write` syscalls
- `Read` → `Escape` (logging)
- `Write` → `Escape` (logging)
- `SignalHandler` → `ioctl`, `Escape` (logging)

---

## Known Issues / TODOs

### term.cpp

1. **Incomplete palette mapping** (lines 500-503)
   - System colors (0-15) default to black (should use lookup table)
   - Grayscale ramp (232-255) defaults to white (should use linear grayscale)
   - Current Pal() only handles RGB cube (16-231)

2. **Hardcoded window rect** (line 787)
   - `int rc[] = {1920 + 800, 300, 800, 600}` — should save in settings on clean exit
   - TODO comment at line 786

3. **Editor globals as HACK** (lines 207-221)
   - `extern Terrain* terrain, World* world` — "HACK: get it from editor"
   - `extern float pos_x, pos_y, pos_z, rot_yaw, global_lt[], int probe_z` — "SUPER_HACK LIVE VIEW"
   - Should use proper dependency injection instead of extern globals

### terminal.cpp

1. **Commented SIGWINCH propagation** (line 329)
   - `ioctl(pty_fd, TIOCSWINSZ, &ws)` is commented out
   - Child process can silently resize PTY, parent cannot respond
   - No clean solution (PTY limitation)

2. **No error handling**
   - read()/write() return values checked for <=0 but not errno
   - forkpty() failure returns -1 but doesn't log reason
   - open() failure for log file silently fails (log_fd = -1)

---

