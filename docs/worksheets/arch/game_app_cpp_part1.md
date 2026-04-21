# game_app.cpp Analysis (Lines 1-1882)

## File Header Documentation (Lines 1-74)

**Purpose:** Native desktop entry point with V8 JavaScript engine for NPC scripting. Initializes platform backends (SDL/X11/Win32), V8 isolate, OpenGL rendering, and runs the main game loop with native filesystem and input.

**Platform-Specific Features:**
- V8 JavaScript Engine (embedded, not available on web)
- Native Filesystem (direct OS access vs web's IndexedDB)
- Native OpenGL (direct GPU vs WebGL)
- Native Main Loop (preemptive while(1) vs web's cooperative emscripten_set_main_loop)

**Initialization Order:**
1. Parse command-line arguments
2. Initialize V8 JavaScript engine
3. Initialize platform backends
4. Load world data (.a3d, .xp sprites, .akm meshes)
5. Initialize rendering (OpenGL context, shaders, textures)
6. Initialize physics
7. Run main game loop (input → physics → render → present @ 60 FPS)

---

## Platform Comparison Table (Lines 35-51)

Comprehensive comparison of game_app.cpp (Native Desktop) vs game_web.cpp (WebAssembly) vs game_svr.cpp (Headless Server):
- JavaScript: V8 embedded vs Browser JS vs None
- Filesystem: Native OS vs Virtual IndexedDB vs Native OS
- Rendering: Native OpenGL vs WebGL vs None (headless)
- Main Loop: while(1) blocking vs emscripten_set_main_loop vs tick-based network
- Input: SDL/X11/Win32 vs Browser events vs Network only
- Config Storage: ./asciicker.cfg (native) vs /data/asciicker.cfg (IndexedDB) vs ./asciicker.cfg (native)

---

## Global Variables (Lines 164-923)

### `tty` (game_app.cpp:164)
**Type:** `int`
**Initial Value:** `-1`
**Purpose:** File descriptor for Linux virtual console terminal (ttyN)
**Mutated by:** `find_tty()`, `exit_handler()`
**Read by:** `Print()`, `exit_handler()`, `PrevGLFont()`, `NextGLFont()`

### `base_path` (game_app.cpp:175)
**Type:** `char[1024]`
**Initial Value:** `"./"`
**Purpose:** Base directory path for executable (used for asset loading)
**Mutated by:** `main()`
**Read by:** `GetConfPath()`, `MyFont::Scan()`, `PrevGLFont()`, `NextGLFont()`

### `conf_path` (game_app.cpp:184)
**Type:** `char[1024+20]`
**Initial Value:** `""`
**Purpose:** Cached path to configuration file (asciicker.cfg)
**Mutated by:** `GetConfPath()`
**Read by:** `GetConfPath()`

### `xterm_kitty` (game_app.cpp:284)
**Type:** `bool`
**Initial Value:** `false`
**Purpose:** Flag for kitty terminal detection (enables bracketed paste mode)
**Mutated by:** `SetScreen()`

### `mouse_x`, `mouse_y`, `mouse_down` (game_app.cpp:285-287)
**Type:** `int`, `int`, `int`
**Initial Value:** `-1`, `-1`, `0`
**Purpose:** Mouse cursor position and button state for GPM (general purpose mouse)
**Mutated by:** Input handling (not in this range)
**Read by:** `Print()`

### `gpm` (game_app.cpp:288)
**Type:** `int`
**Initial Value:** `-1`
**Purpose:** GPM (general purpose mouse) file descriptor
**Read by:** `Print()`

### `pal_16` (game_app.cpp:376)

**Signature:** `uint8_t pal_16[256]`

**Purpose:** 16-color palette mapping for Linux virtual console

**Called by:** `Print()` (terminal output)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Read-only palette data

**Notes:** Used for Linux virtual console terminal output in PURE_TERM mode.

### `pal_rgba` (game_app.cpp:378-427)

**Signature:** `const uint8_t pal_rgba[256][3]`

**Purpose:** 256-color RGB palette (first 16 are CGA colors, rest are 6x6x6 color cube)

**Called by:** `Print()` (terminal output)

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Read-only palette data

**Notes:** First 16 entries are CGA colors, remaining entries are 6x6x6 color cube (216 colors) plus grays.

### `running` (game_app.cpp:536)
**Type:** `bool`
**Initial Value:** `false`
**Purpose:** Main game loop flag
**Mutated by:** `exit_handler()`, `main()`
**Read by:** Main loop (not in this range)

### `mat` (game_app.cpp:577)
**Type:** `Material[256]`
**Purpose:** Material definitions array (terrain surface types with glyphs, colors, shading)
**Mutated by:** `InitMaterials()`
**Read by:** `GetMaterialArr()`, rendering code

### `fonts_loaded` (game_app.cpp:792)
**Type:** `int`
**Initial Value:** `0`
**Purpose:** Count of loaded font textures
**Mutated by:** `MyFont::Load()`, `MyFont::Free()`
**Read by:** `MyFont::Free()`, `FindFont()`, `GetGLFont()`, `NextGLFont()`

### `font` (game_app.cpp:908)
**Type:** `MyFont[256]`
**Purpose:** Array of loaded font textures with dimensions
**Mutated by:** `MyFont::Load()`, `MyFont::Free()`
**Read by:** `GetFontArr()`, `FindFont()`, `GetGLFont()`, `PrevGLFont()`, `NextGLFont()`

### `pos_x`, `pos_y`, `pos_z`, `rot_yaw` (game_app.cpp:916-917)
**Type:** `float`
**Purpose:** Camera/player position and rotation (used by term.h compatibility layer)

### `probe_z` (game_app.cpp:918)
**Type:** `int`
**Purpose:** Z-axis probe value (terrain query)

### `global_lt` (game_app.cpp:919)
**Type:** `float[4]`
**Purpose:** Global lighting vector

### `world` (game_app.cpp:920)
**Type:** `World*`
**Initial Value:** `0`
**Purpose:** Pointer to world data structure (BSP tree, meshes, instances)

### `terrain` (game_app.cpp:921)
**Type:** `Terrain*`
**Initial Value:** `0`
**Purpose:** Pointer to terrain quadtree heightmap

### `font_zoom` (game_app.cpp:923)
**Type:** `int`
**Initial Value:** `0`
**Purpose:** Font zoom offset from auto-detected font size

### `tty_font` (game_app.cpp:981)
**Type:** `static int`
**Initial Value:** `4`
**Purpose:** Index into tty_fonts array for current Linux console font size

### `tty_fonts` (game_app.cpp:982)
**Type:** `static const int[]`
**Value:** `{6,8,10,12,14,16,18,20,24,28,32,-1}`
**Purpose:** Available font sizes for Linux virtual console

### `xterm_fullscreen` (game_app.cpp:988)
**Type:** `static bool`
**Initial Value:** `false`
**Purpose:** Fullscreen state for xterm (PURE_TERM mode only)

### `server` (game_app.cpp:1124)
**Type:** `Server*`
**Initial Value:** `0`
**Purpose:** Pointer to server connection (multiplayer client mode)

### `game` (game_app.cpp:1688)
**Type:** `Game*`
**Initial Value:** `0`
**Purpose:** Pointer to main game state object

### `MakeStamp` (game_app.cpp:1693)
**Type:** `uint64_t (*)()`
**Initial Value:** `0`
**Purpose:** Function pointer for timestamp generation (V8 callback)

---

## Template Functions

### `UTF8` (game_app.cpp:227-246)
**Signature:** `template <uint16_t C> static int UTF8(char* buf)`
**Purpose:** Convert CP437 code point to UTF-8 byte sequence
**Called by:** CP437 array initialization (lines 248-282)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** Writes 1-3 bytes to buf
**Notes:** Template metaprogramming - generates compile-time UTF-8 encoders for each CP437 character

---

## Static Data Structures

### `CP437` (game_app.cpp:248-282)

**Signature:** `static int (* const CP437[256])(char*)`

**Purpose:** Function pointer array mapping CP437 code points to UTF-8 encoders

**Called by:** `Print()` (terminal output)

**Calls:** None (read-only data)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Read-only data structure

**Notes:** Used for terminal output in PURE_TERM mode. Each entry is a function pointer to UTF8<N> for encoding specific CP437 characters to UTF-8.

---

## Functions

### `Buzz` (game_app.cpp:171-173)
**Signature:** `void Buzz()`
**Purpose:** Trigger haptic feedback (gamepad rumble) - stub on native, implemented elsewhere
**Called by:** Game logic (external)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None (stub implementation)
**Notes:** Platform-specific stub. Native platforms use SDL gamepad API, web uses Vibration API, server has no haptic output.

### `SyncConf` (game_app.cpp:180-182)
**Signature:** `void SyncConf()`
**Purpose:** Synchronize configuration to persistent storage - stub on native, web requires FS.syncfs()
**Called by:** Configuration save code (external)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None (stub implementation)
**Notes:** Native platforms write synchronously to filesystem, web platforms use FS.syncfs() to flush IndexedDB.

### `GetConfPath` (game_app.cpp:188-217)
**Signature:** `const char* GetConfPath()`
**Purpose:** Get platform-specific path to configuration file (asciicker.cfg)
**Called by:** Configuration loading/saving (external)
**Calls:** `getenv()`, `sprintf()`
**Globals read:** `conf_path`, `base_path`
**Globals mutated:** `conf_path`
**Side effects:** Reads environment variables (SNAP_USER_DATA, HOME on Linux/macOS; APPDATA on Windows)
**Notes:** Lazy initialization - builds path on first call. Linux/macOS checks SNAP_USER_DATA → HOME → base_path. Windows checks APPDATA → base_path.

### `GetWH` (game_app.cpp:290-307)
**Signature:** `bool GetWH(int wh[2])`
**Purpose:** Get terminal window dimensions via ioctl (PURE_TERM mode)
**Called by:** `SetScreen()`, terminal input handling
**Calls:** `ioctl()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Queries terminal via TIOCGWINSZ ioctl, clamps to 160x90 max
**Notes:** Linux/macOS only. Returns false if ioctl fails.

### `SetScreen` (game_app.cpp:310-356)
**Signature:** `void SetScreen(bool alt)`
**Purpose:** Switch terminal between alternate screen buffer (game mode) and normal mode
**Called by:** `exit_handler()`, main loop init
**Calls:** `getenv()`, `strcmp()`, `write()`, `tcgetattr()`, `tcsetattr()`, `GetWH()`, `sprintf()`
**Globals read:** `xterm_kitty`, `tty`
**Globals mutated:** `xterm_kitty`
**Side effects:** Enables/disables alternate screen buffer, mouse tracking, bracketed paste, canonical mode. Restores palette on exit.
**Notes:** ANSI escape sequences: `\x1B[?1049h` (alt screen), `\x1B[?1002h` (mouse), `\x1B[?1006h` (SGR mouse encoding), `\x1B[?2017h` (kitty bracketed paste).

### `Print` (game_app.cpp:429-534)
**Signature:** `void Print(AnsiCell* buf, int w, int h, const char utf[256][4])`
**Purpose:** Render framebuffer to terminal using ANSI escape codes
**Called by:** Main loop (not in this range)
**Calls:** `write()`, `WRITE()` macro, `FLUSH()` macro
**Globals read:** `tty`, `gpm`, `mouse_x`, `mouse_y`, `pal_rgba`
**Globals mutated:** None
**Side effects:** Writes ANSI escape sequences to stdout (2.3MB output buffer)
**Notes:** Two rendering paths: Linux virtual console (tty>=0) uses palette escapes `\e]PXRRGGBB`, xterm uses 256-color codes `\x1B[38;5;N;48;5;Mm`. Bakes mouse cursor into buffer for GPM. Top-to-bottom scanline order.

### `exit_handler` (game_app.cpp:537-555)
**Signature:** `void exit_handler(int signum)`
**Purpose:** Signal handler for clean shutdown
**Called by:** OS signal delivery (SIGINT, SIGTERM)
**Calls:** `SetScreen()`, `FreeAudio()`, `getenv()`, `sprintf()`, `system()`, `exit()`
**Globals read:** `running`, `tty`
**Globals mutated:** `running`
**Side effects:** Restores terminal state, restores original font via setfont, exits process
**Notes:** Restores font from /tmp/asciicker.<tty>.psf (saved at startup). SNAP_USER_DATA fallback to /tmp.

### `GetTime` (game_app.cpp:564-569)
**Signature:** `uint64_t GetTime()`
**Purpose:** Get high-resolution monotonic timestamp in microseconds
**Called by:** Frame timing, animation, profiling
**Calls:** `clock_gettime(CLOCK_MONOTONIC, &ts)`
**Globals read:** None
**Globals mutated:** `static timespec ts`
**Side effects:** None (pure query)
**Notes:** POSIX clock_gettime() on Linux/macOS. Windows/web use different implementations (a3dGetTime).

### `GetMaterialArr` (game_app.cpp:578-581)
**Signature:** `void* GetMaterialArr()`
**Purpose:** Accessor for material array (used by rendering code)
**Called by:** Rendering code (external)
**Calls:** None
**Globals read:** `mat`
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns void* to avoid header dependencies

### `InitMaterials` (game_app.cpp:584-789)
**Signature:** `void InitMaterials()`
**Purpose:** Initialize 256 material definitions with glyphs, colors, and shading
**Called by:** `main()` or early init
**Calls:** Fast random number generator (lambda)
**Globals read:** None
**Globals mutated:** `mat`
**Side effects:** Fills material array
**Notes:** First 9 materials are defined (WATER=0, GRASS=1, DIRT=2, STONE=3, SAND=4, SNOW=5, MUD=6, COBBLESTONE=7, GRAVEL=8). Materials 9-255 are random placeholders. Each material has 4 roughness levels × 16 shade levels.

**Material Definitions:**
- **WATER (0):** Glyphs `{',', ' ', '!', ' '}`, blue-to-dark fg, bright blue bg
- **GRASS (1):** Glyphs `{'"', '\'', '"', '`'}`, green shades
- **DIRT (2):** Glyphs `{'.', ':', ',', '\''}`, brown shades
- **STONE (3):** Glyphs `{'#', 'O', '8', '@'}`, gray shades
- **SAND (4):** Glyphs `{' ', '.', ':', ','}`, pale yellow shades
- **SNOW (5):** Glyphs `{'*', '+', '.', ' '}`, white-blue shades
- **MUD (6):** Glyphs `{'~', '=', '-', '.'}`, dark brown shades
- **COBBLESTONE (7):** Glyphs `{'o', 'O', '0', '@'}`, steel blue shades
- **GRAVEL (8):** Glyphs `{'.', ':', ';', ','}`, medium gray shades

### `GetFontArr` (game_app.cpp:910-913)
**Signature:** `void* GetFontArr()`
**Purpose:** Accessor for font array (used by font loading/rendering)
**Called by:** `MyFont::Scan()`, `MyFont::Load()`, `MyFont::Free()`, `qsort()`
**Calls:** None
**Globals read:** `font`
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns void* to avoid header dependencies

### `FindFont` (game_app.cpp:925-946)
**Signature:** `static int FindFont(const int wnd_wh[2])`
**Purpose:** Find best-fit font for window dimensions (targeting 120x75 cells)
**Called by:** `GetGLFont()`, `PrevGLFont()`, `NextGLFont()`
**Calls:** `fabsf()`
**Globals read:** `fonts_loaded`, `font`
**Globals mutated:** None
**Side effects:** None
**Notes:** Heuristic: minimizes error from target cell count (120×75). Iterates all loaded fonts.

### `GetGLFont` (game_app.cpp:948-979)
**Signature:** `int GetGLFont(int wh[2], const int wnd_wh[2], int* id)`
**Purpose:** Get OpenGL font texture for window dimensions (with zoom offset and size clamping)
**Called by:** Rendering code (external), `PrevGLFont()`, `NextGLFont()`
**Calls:** `FindFont()`
**Globals read:** `font_zoom`, `fonts_loaded`, `font`
**Globals mutated:** None
**Side effects:** None
**Notes:** Applies font_zoom offset, clamps to [0, fonts_loaded-1], ensures minimum 45×36 cells. Returns texture ID, optionally writes font dimensions and index.

### `ToggleFullscreen` (game_app.cpp:989-1002)
**Signature:** `void ToggleFullscreen(Game* g)`
**Purpose:** Toggle fullscreen mode in xterm (PURE_TERM mode only)
**Called by:** Input handling (external)
**Calls:** `getenv()`, `strcmp()`, `write()`
**Globals read:** `xterm_fullscreen`
**Globals mutated:** `xterm_fullscreen`
**Side effects:** Sends xterm escape codes `\033[9;1t` (fullscreen) or `\033[9;0t` (windowed)
**Notes:** Only in PURE_TERM mode. Ignores Linux virtual console (term != "linux").

### `IsFullscreen` (game_app.cpp:1004-1007)
**Signature:** `bool IsFullscreen(Game* g)`
**Purpose:** Query fullscreen state (PURE_TERM mode only)
**Called by:** Input handling (external)
**Calls:** None
**Globals read:** `xterm_fullscreen`
**Globals mutated:** None
**Side effects:** None
**Notes:** Only in PURE_TERM mode.

### `PrevGLFont` (game_app.cpp:1010-1065)
**Signature:** `bool PrevGLFont()`
**Purpose:** Decrease font size (zoom out)
**Called by:** Input handling (F11 key)
**Calls:** `sprintf()`, `system()`, `write()`, `a3dGetRect()`, `GetGLFont()`, `FindFont()`, `TermResizeAll()`
**Globals read:** `tty`, `base_path`, `tty_fonts`, `tty_font`, `xterm_fullscreen`, `font_zoom`, `fonts_loaded`
**Globals mutated:** `tty_font`, `font_zoom`
**Side effects:** Linux console: changes font via setfont system call. Xterm: sends `\033]50;#-1\a` escape code. GL mode: updates font_zoom and resizes terminals.
**Notes:** Returns false if already at minimum. PURE_TERM mode uses tty_fonts array or xterm font ops. GL mode adjusts font_zoom and ensures font doesn't get clamped (f2 != f).

### `NextGLFont` (game_app.cpp:1067-1122)
**Signature:** `bool NextGLFont()`
**Purpose:** Increase font size (zoom in)
**Called by:** Input handling (F12 key)
**Calls:** `sprintf()`, `system()`, `write()`, `a3dGetRect()`, `GetGLFont()`, `FindFont()`, `TermResizeAll()`
**Globals read:** `tty`, `base_path`, `tty_fonts`, `tty_font`, `xterm_fullscreen`, `font_zoom`, `fonts_loaded`
**Globals mutated:** `tty_font`, `font_zoom`
**Side effects:** Linux console: changes font via setfont. Xterm: sends `\033]50;#+1\a`. GL mode: updates font_zoom and resizes terminals.
**Notes:** Returns false if already at maximum. Symmetric to PrevGLFont().

---

## Structures

### `GameServer` (game_app.cpp:1126-1222)

**Signature:** `struct GameServer : Server`

**Purpose:** Native platform server connection (multiplayer client mode)

**Called by:** Server connection initialization

**Calls:** Method entries listed below

**Globals read:** None

**Globals mutated:** Instance fields (see Notes)

**Side effects:** Manages network I/O and message queue

**Notes:** Extends Server. Fields: `TCP_SOCKET server_socket` (line 1128) - WebSocket connection to game server, `uint8_t buf[buf_size]` (line 1131) - Circular receive buffer (64KB), `int buf_ofs` (line 1132) - Current write offset in receive buffer, `MSG_FIFO msg[msg_size]` (line 1142) - Message queue (256 entries), `int msg_read` (line 1144) - Read index (main thread, wraps at 256), `int msg_write` (line 1145) - Write index (network thread, wraps at 256), `volatile unsigned int msg_num` (line 1147) - Message count (interlocked operations). Nested Struct: `MSG_FIFO` (lines 1134-1138) with `uint8_t* ptr` (line 1136) - Pointer into buf array and `int size` (line 1137) - Message size in bytes.

**Fields:**
- `TCP_SOCKET server_socket` (line 1128): WebSocket connection to game server
- `uint8_t buf[buf_size]` (line 1131): Circular receive buffer (64KB)
- `int buf_ofs` (line 1132): Current write offset in receive buffer
- `MSG_FIFO msg[msg_size]` (line 1142): Message queue (256 entries)
- `int msg_read` (line 1144): Read index (main thread, wraps at 256)
- `int msg_write` (line 1145): Write index (network thread, wraps at 256)
- `volatile unsigned int msg_num` (line 1147): Message count (interlocked operations)



---

### `MSG_FIFO` (game_app.cpp:1134-1138)

**Signature:** `struct MSG_FIFO`

**Purpose:** Message queue entry structure

**Called by:** GameServer message queue

**Calls:** None (data structure)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (data structure)

**Notes:** Fields: `uint8_t* ptr` (line 1136) - Pointer into buf array, `int size` (line 1137) - Message size in bytes.

**Methods:**

### `GameServer::Start` (game_app.cpp:1149-1171)

**Signature:** `bool Start()`

**Purpose:** Start background receive thread

**Called by:** `Connect()` (not in this range)

**Calls:** `malloc()`, `THREAD_CREATE_DETACHED()`, `GetTime()`

**Globals read:** None

**Globals mutated:** `this->head`, `this->tail`, `this->others`, `this->buf_ofs`, `this->msg_read`, `this->msg_write`, `this->msg_num`, `this->stamp`

**Side effects:** Allocates Human array, spawns network thread

**Notes:** Initializes message queue, allocates `max_clients` Human structs. Returns false if thread creation fails.

### `GameServer::Recv` (game_app.cpp:1173-1197)
**Signature:** `void Recv()`
**Purpose:** Background thread receive loop
**Called by:** `GameServer::Entry()`
**Calls:** `WS_READ()`, `THREAD_SLEEP()`, `INTERLOCKED_INC()`
**Globals read:** None
**Globals mutated:** `this->buf`, `this->buf_ofs`, `this->msg`, `this->msg_write`, `this->msg_num`
**Side effects:** Blocks on WebSocket read, sleeps when queue full, wraps buffer at 64KB
**Notes:** Runs in separate thread. Exits loop on read error (r<=0). Circular buffer wraps when <256 bytes remain.

### `GameServer::Entry` (game_app.cpp:1199-1204)
**Signature:** `static void* Entry(void* arg)`
**Purpose:** Thread entry point
**Called by:** OS thread creation
**Calls:** `GameServer::Recv()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Runs Recv() loop
**Notes:** Static method, casts arg to GameServer* and calls Recv().

### `GameServer::Stop` (game_app.cpp:1206-1221)
**Signature:** `void Stop()`
**Purpose:** Clean shutdown (close socket, free resources)
**Called by:** `Server::Send()` on error, `Server::Proc()` on error
**Calls:** `TCP_CLOSE()`, `TCP_CLEANUP()`, `free()`
**Globals read:** `server`
**Globals mutated:** `this->server_socket`, `this->others`
**Side effects:** Closes TCP socket, terminates background thread, frees others array
**Notes:** Does not join thread (detached thread will exit when socket closes).

### `MyFont` (game_app.cpp:793-908)

**Signature:** `struct MyFont`

**Purpose:** Font texture loader and manager

**Called by:** Font loading pipeline

**Calls:** Method entries listed below

**Globals read:** None

**Globals mutated:** None

**Side effects:** Loads fonts, manages OpenGL textures

**Notes:** Fields: `int width` (line 904) - Font texture width in pixels, `int height` (line 905) - Font texture height in pixels, `GLuint tex` (line 907) - OpenGL texture ID.

### `MyFont::Scan` (game_app.cpp:795-806)
**Signature:** `static bool Scan(A3D_DirItem item, const char* name, void* cookie)`
**Purpose:** Callback for directory scanning (loads font images)
**Called by:** Directory scanner (external)
**Calls:** `snprintf()`, `a3dLoadImage()`
**Globals read:** `base_path`
**Globals mutated:** None
**Side effects:** Loads image file via a3dLoadImage()
**Notes:** Cookie parameter is base directory path. Ignores directories. Calls MyFont::Load() as image loader callback.

### `MyFont::Sort` (game_app.cpp:808-817)
**Signature:** `static int Sort(const void* a, const void* b)`
**Purpose:** qsort comparator (sort by font area: width×height)
**Called by:** `qsort()` in `MyFont::Load()`
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Ascending order (smallest fonts first).

### `MyFont::Free` (game_app.cpp:819-826)
**Signature:** `static void Free()`
**Purpose:** Delete all loaded font textures
**Called by:** Cleanup code (not in this range)
**Calls:** `GetFontArr()`, `glDeleteTextures()`
**Globals read:** `fonts_loaded`
**Globals mutated:** None (textures deleted, but array not cleared)
**Side effects:** Deletes OpenGL textures
**Notes:** Does not reset fonts_loaded counter.

### `MyFont::Load` (game_app.cpp:828-889)
**Signature:** `static void Load(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)`
**Purpose:** Image loader callback (converts to RGBA8 texture, uploads to GPU)
**Called by:** `a3dLoadImage()` via `MyFont::Scan()`
**Calls:** `malloc()`, `ConvertLuminance_UI32_LLZZYYXX()`, `gl3CreateTextures()`, `gl3TextureStorage2D()`, `glPixelStorei()`, `gl3TextureSubImage2D()`, `gl3TextureParameteri2D()`, `gl3TextureParameterfv2D()`, `free()`, `qsort()`, `GetFontArr()`
**Globals read:** `fonts_loaded`, `font`
**Globals mutated:** `fonts_loaded`, `font`
**Side effects:** Allocates temp buffer, uploads texture to GPU, sorts font array
**Notes:** Stops at 256 fonts. Converts luminance to white RGB + luminance alpha. Uses GL_NEAREST filtering, GL_CLAMP_TO_BORDER with transparent white border. Sorts entire font array after each load (inefficient but preserves size order).

### `MyFont::SetTexel` (game_app.cpp:891-895)
**Signature:** `void SetTexel(int x, int y, uint8_t val)`
**Purpose:** Write single texel alpha value
**Called by:** Font editing code (not in this range)
**Calls:** `gl3TextureSubImage2D()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Modifies GPU texture (white RGB, val alpha)
**Notes:** 1×1 pixel write.

### `MyFont::GetTexel` (game_app.cpp:897-902)
**Signature:** `uint8_t GetTexel(int x, int y)`
**Purpose:** Read single texel alpha value
**Called by:** Font editing code (not in this range)
**Calls:** `gl3GetTextureSubImage()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** GPU readback (slow)
**Notes:** 1×1 pixel read. Returns alpha channel.

---

## Server Methods

### `Server::Send` (game_app.cpp:1224-1237)
**Signature:** `bool Send(const uint8_t* data, int size)`
**Purpose:** Send data to server over WebSocket
**Called by:** Game networking code (external)
**Calls:** `WS_WRITE()`, `GameServer::Stop()`, `free()`
**Globals read:** `server`
**Globals mutated:** `server` (set to 0 on failure)
**Side effects:** Writes to socket, may close connection and free server
**Notes:** Binary WebSocket frame (0x2). On error, calls Stop(), frees others, frees server, nulls global.

### `Server::Proc` (game_app.cpp:1239-1258)
**Signature:** `void Proc()`
**Purpose:** Process received messages from network thread
**Called by:** Main loop (external)
**Calls:** `Server::Proc(uint8_t*, int)`, `free()`, `INTERLOCKED_SUB()`
**Globals read:** `server`
**Globals mutated:** `server` (set to 0 on error)
**Side effects:** Processes message queue, may close connection
**Notes:** Batch-processes msg_num messages, calls overloaded Proc() for each. Exits and frees server on size<=0 (connection closed).

### `Server::Log` (game_app.cpp:1260-1263)
**Signature:** `void Log(const char* str)`
**Purpose:** Log server message (stub, commented out)
**Called by:** Server code (external)
**Calls:** None (printf commented out)
**Globals read:** None
**Globals mutated:** None
**Side effects:** None (stub)
**Notes:** Would print to stdout if enabled.

---

## Networking Functions

### `Connect` (game_app.cpp:1265-1435)
**Signature:** `GameServer* Connect(const char* addr, const char* port, const char* path, const char* user)`
**Purpose:** Connect to game server via WebSocket (HTTP upgrade handshake, JOIN protocol)
**Called by:** Main loop on --server flag (not in this range)
**Calls:** `TCP_INIT()`, `memset()`, `getaddrinfo()`, `socket()`, `connect()`, `setsockopt()`, `freeaddrinfo()`, `sprintf()`, `strlen()`, `TCP_WRITE()`, `HTTP_READ()`, `TCP_READ()`, `WS_WRITE()`, `WS_READ()`, `strncpy()`, `malloc()`, `TCP_CLOSE()`, `TCP_CLEANUP()`, `printf()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Creates TCP socket, performs HTTP->WebSocket upgrade, sends JOIN request, receives JOIN response
**Notes:** Full connection sequence:
1. Resolve hostname via getaddrinfo()
2. Create TCP socket
3. Connect to server
4. Enable SO_KEEPALIVE, TCP_NODELAY
5. Send HTTP GET request with WebSocket upgrade headers
6. Read HTTP response headers (check Content-Length)
7. Read response body
8. Send JOIN message with username (WebSocket binary frame)
9. Receive JOIN response with client ID
10. Allocate GameServer, start background thread
Returns NULL on any failure. User-Agent: "native-asciicker-windows" or "native-asciicker-linux". Hardcoded WebSocket key: "btsPdKGunHdaTPnSSDlfow==".

---

## Platform-Specific Functions (Linux/macOS)

### `find_tty` (game_app.cpp:1441-1474)
**Signature:** `static int find_tty()`
**Purpose:** Find Linux virtual console TTY number by walking /proc parent chain
**Called by:** `main()` (not in this range)
**Calls:** `getpid()`, `sprintf()`, `fopen()`, `fread()`, `fclose()`, `strchr()`, `sscanf()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Reads /proc filesystem
**Notes:** Linux only. Walks parent process chain via /proc/<pid>/stat. Looks for tty in range 1025-1087 (virtual consoles 1-63). Returns 0 if not in virtual console.

### `scan_js` (game_app.cpp:1513-1648)
**Signature:** `int scan_js(char* gamepad_name, int* gamepad_axes, int* gamepad_buttons, uint8_t* gamepad_mapping)`
**Purpose:** Scan for gamepad devices and build SDL-compatible button/axis mapping
**Called by:** Input init (not in this range)
**Calls:** `sprintf()`, `open()`, `ioctl()`, `strcpy()`, `memset()`
**Globals read:** None
**Globals mutated:** `static int index`, `static int skip`
**Side effects:** Opens /dev/input/js<N> device, queries capabilities, builds mapping
**Notes:** Linux only. Round-robin scans js0-js15. On failure, skips next 10 iterations. Uses Linux joystick API (JSIOCGVERSION, JSIOCGAXES, JSIOCGBUTTONS, JSIOCGNAME, JSIOCGBTNMAP, JSIOCGAXMAP). Maps Linux button/axis codes to SDL-style indices. Returns fd on success, -1 on failure.

**Mapping format (gamepad_mapping array):**
- Bit 7: 1=button, 0=axis
- Bit 6: 1=negative direction, 0=positive direction
- Bits 0-5: SDL axis/button index

**Button mappings:**
- BTN_A → 0x80|0
- BTN_B → 0x80|1
- BTN_X → 0x80|2
- BTN_Y → 0x80|3
- BTN_SELECT → 0x80|4 (back_button)
- BTN_MODE → 0x80|5 (guide_button)
- BTN_START → 0x80|6 (start_button)
- BTN_THUMBL → 0x80|7 (left_stick_button)
- BTN_THUMBR → 0x80|8 (right_stick_button)
- BTN_TL → 0x80|9 (left_shoulder_button)
- BTN_TR → 0x80|10 (right_shoulder_button)
- BTN_TL2 → 0x00|2 (left_trigger_axis)
- BTN_TR2 → 0x00|5 (right_trigger_axis)

**Axis mappings (signed):**
- Axis 0 (left-x): neg=0x40|0, pos=0x00|0
- Axis 1 (left-y): neg=0x40|1, pos=0x00|1
- Axis 2 (left-z): both=0x00|4 (unsigned trigger)
- Axis 3 (right-x): neg=0x40|2, pos=0x00|2
- Axis 4 (right-y): neg=0x40|3, pos=0x00|3
- Axis 5 (right-z): both=0x00|5 (unsigned trigger)
- Axis 16 (dirpad-x): neg=0x80|13, pos=0x80|14
- Axis 17 (dirpad-y): neg=0x80|11, pos=0x80|12

### `read_js` (game_app.cpp:1651-1686)
**Signature:** `bool read_js(int fd)`
**Purpose:** Read gamepad events and dispatch to GamePadButton/GamePadAxis
**Called by:** Input loop (not in this range)
**Calls:** `read()`, `GamePadButton()`, `GamePadAxis()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Reads from joystick device fd, dispatches button/axis events
**Notes:** Linux only. Reads up to 64 js_event structs in one read(). Ignores JS_EVENT_INIT flag. Calls GamePadButton() for button events, GamePadAxis() for axis events. Clamps axis -32768 to -32767.

---

## V8 JavaScript Engine Functions (Declared)

### `init_v8` (game_app.cpp:1690)
**Signature:** `void init_v8()`
**Purpose:** Initialize V8 JavaScript engine (isolate, global context, callbacks)
**Called by:** `main()`
**Implementation:** Not in this range

### `free_v8` (game_app.cpp:1691)
**Signature:** `void free_v8()`
**Purpose:** Shutdown V8 JavaScript engine
**Called by:** `main()` or exit handler
**Implementation:** Not in this range

---

## Main Function (Partial)

### `main` (game_app.cpp:1698-1882)
**Signature:** `int main(int argc, char* argv[])`
**Purpose:** Entry point - V8 init, module setup, base_path extraction
**Called by:** OS
**Calls:** `init_v8()`, `akAPI_Exec()`, `akAPI_Init()`, `strcpy()`, `strlen()`, `realpath()`, `GetFullPathNameA()`, `strrchr()`, `memcpy()`, `strstr()`
**Globals read:** `base_path`
**Globals mutated:** `base_path`
**Side effects:** Initializes V8, exposes Module object to JavaScript (HEAP views, UTF8 conversions)
**Notes:** Lines 1698-1882 only (first half of main). Initializes V8, injects JavaScript Module object with typed array heap views (HEAPF64, HEAPF32, HEAPU32, HEAP32, HEAPU16, HEAP16, HEAPU8, HEAP8). Defines UTF8ToString() and stringToUTF8() helpers (emscripten compatibility). Extracts base_path from argv[0] (handles realpath on Unix, GetFullPathNameA on Windows). Detects .run/ directory in path (build output directory).

**V8 JavaScript Module Setup (lines 1702-1814):**
- `akAPI_V8AB`: Shared ArrayBuffer for C++ ↔ JS data exchange
- `akAPI_Buff`: Pointer to buffer in C++ heap (exposed to JS)
- `akAPI_This`: Object for JS context
- `Module.HEAP*`: Typed array views over akAPI_V8AB
- `UTF8ToString(ptr, len)`: Convert C++ UTF-8 string to JS string
- `stringToUTF8(str, ptr, len)`: Convert JS string to C++ UTF-8 buffer

**base_path Extraction Logic (lines 1840-1882):**
- Unix: `realpath(argv[0])` → find last '/' → copy directory portion
- Windows: `GetFullPathNameA(argv[0])` → copy directory portion
- Detects `/.run/`, `\.run\`, `\.run/`, `/.run\` in path (build artifact directories)

---

## Summary Statistics

**Lines analyzed:** 1-1882 (first half of game_app.cpp)

**Functions documented:** 28
- Platform functions: 2 (Buzz, SyncConf)
- Configuration: 1 (GetConfPath)
- Terminal I/O: 4 (GetWH, SetScreen, Print, exit_handler)
- Time: 1 (GetTime)
- Materials: 2 (GetMaterialArr, InitMaterials)
- Fonts: 9 (GetFontArr, FindFont, GetGLFont, ToggleFullscreen, IsFullscreen, PrevGLFont, NextGLFont, MyFont::*)
- Networking: 4 (Server::Send, Server::Proc, Server::Log, Connect)
- Linux-specific: 3 (find_tty, scan_js, read_js)
- V8: 2 (init_v8, free_v8 - stubs)
- Entry: 1 (main - partial)

**Structures documented:** 2
- GameServer (with 4 methods)
- MyFont (with 6 methods)

**Global variables documented:** 24
- tty, base_path, conf_path, xterm_kitty
- mouse_x, mouse_y, mouse_down, gpm
- pal_16, pal_rgba
- running, mat, fonts_loaded, font
- pos_x, pos_y, pos_z, rot_yaw, probe_z, global_lt
- world, terrain, font_zoom
- tty_font, tty_fonts, xterm_fullscreen
- server, game, MakeStamp

**Static data structures:** 1 (CP437 function pointer array)

**All line numbers verified in range [1, 1882].**

