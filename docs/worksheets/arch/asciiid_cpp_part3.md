# asciiid.cpp Architecture Documentation - Part 3 (Lines 8001-11584)

**SCOPE:** Lines 8001-11584 ONLY  
**VALIDATION:** All line numbers verified to be within range 8001-11584

This document covers the final section of asciiid.cpp, including:
- Main rendering loop continuation (ImGui UI panels)
- Mouse and keyboard input handling
- Editor initialization and shutdown
- Testing framework
- Main entry point

---

## Functions Documented

### `DebugProbe` (asciiid.cpp:10744-10804)

**Signature:** `static void DebugProbe()`  
**Purpose:** Diagnostic raycasting tool that outputs detailed hit information to console

**Called by:**
- asciiid.cpp:9204 (ImGui button in INFO window)

**Calls:**
- HitWorld() - ray-mesh intersection
- GetInstName() - instance name lookup
- printf() - console output

**Globals read:**
- ImGui::GetIO() (io)
- font_size
- rot_pitch, rot_yaw
- pos_x, pos_y, pos_z
- world

**Globals mutated:** None

**Side effects:**
- Console printf output with ray origin, direction, hit point, normal, color

**Notes:**
- Reconstructs camera transform matrix manually (duplicates my_render logic)
- Uses same ray construction as main hover/selection logic
- Passes `false, true, false, true` to HitWorld (arg 8 = false allows hitting transparent meshes)
- Interpolates vertex colors at hit point (color[3])
- WHY separate from main render: Debug-only path, not performance critical

---

### `my_mouse` (asciiid.cpp:10806-10894)

**Signature:** `void my_mouse(A3D_WND* wnd, int x, int y, MouseInfo mi)`  
**Purpose:** Platform callback for all mouse events (move, click, wheel, enter/leave)

**Called by:**
- a3dLoop() platform layer (registered via pi.mouse in main)

**Calls:**
- ImGui::GetIO() - updates io.MousePos, io.MouseDown[], io.MouseWheel
- printf() - debug probe logging

**Globals read:**
- mouse_queue, mouse_queue_len, mouse_queue_size (if MOUSE_QUEUE defined)
- io (ImGuiIO)

**Globals mutated:**
- mouse_in (1 on ENTER, 0 on LEAVE)
- io.MousePos, io.MouseDown[0..2], io.MouseWheel
- mouse_queue (if MOUSE_QUEUE defined)
- zoom_wheel

**Side effects:**
- Updates ImGui input state
- Queues mouse events (if MOUSE_QUEUE defined)
- Console printf when Ctrl+click in probe mode

**Notes:**
- MOUSE_QUEUE code path is legacy (disabled by default)
- Without MOUSE_QUEUE: direct ImGui state update
- Mouse button mapping: 0=left, 1=right, 2=middle
- Wheel events: +1.0/-1.0 to io.MouseWheel, zoom_wheel incremented/decremented
- Debug logging: prints probe click info when io.KeyCtrl && !painting

---

### `my_resize` (asciiid.cpp:10896-10912)

**Signature:** `void my_resize(A3D_WND* wnd, int w, int h)`  
**Purpose:** Platform callback for window resize events, updates ImGui display size

**Called by:**
- a3dLoop() platform layer (registered via pi.resize in main)

**Calls:**
- a3dGetRect() - queries window rectangle
- ImGui::GetIO()

**Globals read:** None

**Globals mutated:**
- io.DisplaySize
- io.DisplayFramebufferScale

**Side effects:**
- Updates ImGui viewport dimensions
- Calculates framebuffer scale (for Retina/HiDPI displays)

**Notes:**
- Handles degenerate case: if window rect query fails (win_w/win_h <= 0), uses framebuffer size directly
- DisplayFramebufferScale = framebuffer / window (1.0 for non-Retina, 2.0 for Retina)
- WHY separate from render: Resize events are async, decoupled from frame timing

---

### `my_init` (asciiid.cpp:10923-11141)

**Signature:** `void my_init(A3D_WND* wnd)`  
**Purpose:** Initialize editor on startup: OpenGL, ImGui, world, assets, terrain

**Called by:**
- a3dOpen() platform layer (registered via pi.init in main)

**Calls:**
- glGetString() - OpenGL info
- CreateWorld()
- a3dListDir() - scans meshes/, sprites/, fonts/, palettes/
- GetFirstMesh(), GetFirstSprite()
- RebuildWorld()
- gl3CreateTextures(), gl3TextureStorage3D(), gl3TextureParameteri3D() - 3D palette texture
- Palettize()
- MyMaterial::Init()
- MyFont::Scan(), MyPalette::Init(), MyPalette::Scan()
- a3dGetTime()
- RenderContext::Create()
- glDebugMessageCallback() (non-GL3 builds)
- ImGui::CreateContext(), ImGui_ImplOpenGL3_Init()
- ImGui::StyleColorsDark()
- io.Fonts->AddFontFromFileTTF(), io.Fonts->Build()
- CreateTerrain(), AddTerrainPatch()
- GetTerrainPatch(), GetTerrainVisualMap(), UpdateTerrainVisualMap()
- a3dSetTitle(), a3dSetIcon(), a3dSetVisible()

**Globals read:**
- base_path (for asset directories)

**Globals mutated:**
- world
- active_mesh, active_sprite
- pal_tex
- fonts_loaded, palettes_loaded
- g_Time
- render_context
- io (ImGuiIO struct)
- ini_path
- pFont
- terrain
- pos_x, pos_y, pos_z

**Side effects:**
- Console printf (OpenGL version, terrain stats)
- Filesystem: scans meshes/, sprites/, fonts/, palettes/
- OpenGL: creates textures, uploads data
- ImGui: creates context, loads font from disk
- Window: sets title, icon, visibility

**Notes:**
- **DEPENDENCY ORDER IS CRITICAL** (see lines 10915-10922 comment block)
  1. World creation (scene graph root)
  2. Mesh/sprite library scans
  3. Material system init
  4. OpenGL state setup
  5. ImGui context creation
- Terrain initialization: 16x16 patches at height 0xA000
  - Water border: outer 2 patches (material 0)
  - Grass playable area: inner 12x12 patches (material 1)
- Random patch selection: shuffles 0-255 indices, creates first 256 patches
- Camera position: centered on terrain (32x32 visual cells / 2 = 16,16)
- ImGui key mapping: A3D_* keycodes to ImGuiKey_* enum
- Font: Roboto-Medium.ttf, 16px, range 0x0020-0x03FF

---

### `my_keyb_char` (asciiid.cpp:11143-11147)

**Signature:** `void my_keyb_char(A3D_WND* wnd, wchar_t chr)`  
**Purpose:** Platform callback for text input (after key-to-char translation)

**Called by:**
- a3dLoop() platform layer (registered via pi.keyb_char in main)

**Calls:**
- ImGui::GetIO()
- io.AddInputCharacter()

**Globals read:** None

**Globals mutated:**
- io (ImGuiIO input buffer)

**Side effects:**
- Adds character to ImGui input queue for text widgets

**Notes:**
- Separate from my_keyb_key (which handles raw key presses)
- Handles composed characters, IME input, etc.
- WHY split from key events: Text input vs control input separation

---

### `my_keyb_key` (asciiid.cpp:11149-11206)

**Signature:** `void my_keyb_key(A3D_WND* wnd, KeyInfo ki, bool down)`  
**Purpose:** Platform callback for raw keyboard events, handles modifiers and special keys

**Called by:**
- a3dLoop() platform layer (registered via pi.keyb_key in main)

**Calls:**
- ImGui::GetIO()
- a3dGetKeyb() - queries current modifier key state
- GetInstTM(), SetInstTM()
- DetachInst(), AttachInst()
- printf() - debug nudge logging

**Globals read:**
- selected_inst
- edit_mode
- io (ImGuiIO)

**Globals mutated:**
- io.KeysDown[ki]
- io.KeysDown[A3D_ENTER] (special case: numpad+main enter)
- io.KeyAlt, io.KeyCtrl, io.KeyShift
- reload_sprites_requested (on F5)
- selected_inst transform matrix (arrow key nudge)

**Side effects:**
- Updates ImGui key state
- Console printf for nudge operations
- Detaches/reattaches instance to BSP tree on nudge

**Notes:**
- Strips A3D_AUTO_REPEAT flag before processing
- Modifier key handling: macOS uses RALT (no LALT), other platforms use LALT
- Arrow key nudge (lines 11168-11198):
  - Only in edit_mode==2 (mesh mode)
  - Only when selected_inst exists
  - Respects io.WantCaptureKeyboard (don't nudge if ImGui has focus)
  - Step size: 1.0 normally, 0.1 with Shift
  - LEFT/RIGHT: X axis
  - UP/DOWN: Y axis (or Z with Ctrl)
  - Detach before transform update (BSP tree integrity)
- F5: sets reload_sprites_requested flag (processed in my_render)
- WHY detach/attach: BSP tree spatial index needs to be rebuilt when position changes

---

### `my_keyb_focus` (asciiid.cpp:11208-11212)

**Signature:** `void my_keyb_focus(A3D_WND* wnd, bool set)`  
**Purpose:** Platform callback for keyboard focus gain/loss

**Called by:**
- a3dLoop() platform layer (registered via pi.keyb_focus in main)

**Calls:** None (currently stub)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:**
- TODO comment: should clear modifiers, drags, etc.
- Currently no-op
- WHY needed: Prevents stuck modifier keys when focus is lost during drag/paint

---

### `my_close` (asciiid.cpp:11214-11265)

**Signature:** `void my_close(A3D_WND* wnd)`  
**Purpose:** Editor shutdown: cleanup all resources in reverse init order

**Called by:**
- a3dLoop() platform layer (registered via pi.close in main)

**Calls:**
- TermCloseAll()
- glDeleteTextures()
- GetFirstMesh(), GetNextMesh(), GetMeshCookie()
- URDO_Purge()
- DeleteWorld()
- DeleteTerrain()
- FreeEnemyGens()
- PurgeItemInstCache()
- MyFont::Free(), MyMaterial::Free()
- ImGui_ImplOpenGL3_Shutdown(), ImGui::DestroyContext()
- RenderContext::Delete()
- a3dClose()

**Globals read:**
- world
- pal_tex
- gather
- ipal

**Globals mutated:**
- pal_tex (set to 0)
- gather (freed)
- ipal (freed)

**Side effects:**
- Frees all heap memory
- Destroys OpenGL textures
- Closes window

**Notes:**
- **REVERSE ORDER OF my_init** (stack discipline)
- MeshPrefs cookie cleanup loop (lines 11223-11229)
- URDO undo/redo buffer purged before world deletion
- gather structure (palettization) freed if exists
- ipal (inverse palette) freed if exists
- WHY order matters: Child objects must be freed before parents (e.g., instances before world)

---

### `DeleteAllEnemyGens` (asciiid.cpp:11281-11285)

**Signature:** `void DeleteAllEnemyGens()`  
**Purpose:** Delete all enemy generators from the linked list

**Called by:**
- asciiid.cpp:8604 (ImGui button in ENEMY tab)

**Calls:**
- DeleteEnemyGen() (called on enemygen_head in loop)

**Globals read:**
- enemygen_head

**Globals mutated:**
- enemygen_head (indirectly via DeleteEnemyGen)

**Side effects:**
- Frees all EnemyGen structures
- Modifies linked list (enemygen_head becomes null)

**Notes:**
- Simple while loop: while(enemygen_head) DeleteEnemyGen(enemygen_head)
- DeleteEnemyGen updates the head pointer internally
- WHY not recursive: Linked list structure makes iteration cleaner

---

### `RunTestScript` (asciiid.cpp:11295-11406)

**Signature:** `extern "C" void RunTestScript(const char* script_path)`  
**Purpose:** Execute automated test script for headless integration testing

**Called by:**
- asciiid.cpp:11559 (main, if --test-script arg provided)

**Calls:**
- fopen(), fgets(), fclose()
- sscanf()
- GetTerrainPatch(), AddTerrainPatch()
- GetAllTerrainPatches()
- GetTerrainHeightMap(), UpdateTerrainHeightMap()
- LoadMesh()
- CreateInst()
- RebuildWorld()
- BakeMeshesToTerrain() (forward declared at line 11292)
- GetTerrainVisualMap()
- exit()

**Globals read:**
- terrain
- world

**Globals mutated:**
- terrain (height/visual maps modified)
- world (meshes added)

**Side effects:**
- Console printf (command logging)
- File I/O: reads script, writes export files
- Calls exit(0) when complete (terminates process)

**Notes:**
- Supported commands:
  - SET_TERRAIN_HEIGHT <h>: sets all patches to height h
  - PLACE_MESH <file> <x> <y> <z>: loads .akm and creates instance
  - BAKE_MESH_TO_TERRAIN: raycasts meshes onto terrain
  - EXPORT_TERRAIN_DATA <file>: writes visual map to binary file
  - EXPORT_HEIGHT_SAMPLES <file>: writes height map to CSV
- Script format: line-based, # for comments
- WHY exits at end: Headless testing, no interactive loop needed
- Scale matrix: Z axis scaled by HEIGHT_SCALE (matches editor placement)
- Ensures patch 0,0 exists before height setting

---

### `CMD_TestMeshBaking` (asciiid.cpp:11425-11427)

**Signature:** `extern "C" void CMD_TestMeshBaking(const char* args)`  
**Purpose:** MCP command entry point for mesh baking unit tests

**Called by:** No callers found via grep MCP command dispatcher (dynamic lookup)

**Calls:**
- MeshBakingTest::RunAllTests() (line 11418)

**Globals read:** None

**Globals mutated:** None

**Side effects:**
- Console printf (test output)

**Notes:**
- MeshBakingTest::RunAllTests() calls TestQuantization()
- TestQuantization(): prints rounding behavior for heights 0.0-24.0
- WHY extern "C": MCP command table uses C linkage
- Currently only tests quantization (height rounding to 16-unit grid)

---

### `main` (asciiid.cpp:11429-11583)

**Signature:** `int main(int argc, char *argv[])`  
**Purpose:** Entry point - parse args, initialize platform, enter main loop

**Called by:**
- Operating system (program entry point)

**Calls:**
- realpath() / GetFullPathNameA() - resolve executable path
- strrchr(), strstr() - path parsing
- LoadSprites()
- LoadSprite() - enemygen.xp
- a3dOpen() - creates window, calls my_init
- RunTestScript() - if --test-script arg
- a3dLoop() - main event loop
- GetFirstSprite(), GetNextSprite()
- GetSpriteCookie(), SetSpriteCookie()
- FreeSprites()
- DumpLeakCounter()
- _CrtDumpMemoryLeaks() (Windows debug builds)

**Globals read:**
- argc, argv

**Globals mutated:**
- base_path
- g_mcp_mode (if --mcp arg)
- enemygen_sprite
- pi (PlatformInterface callbacks)
- gd (GraphicsDesc settings)

**Side effects:**
- Console printf (paths, MCP mode)
- File I/O: loads sprites
- Window creation
- stdin made non-blocking (if --mcp on Unix)
- Memory leak reports (debug builds)

**Notes:**
- **Base path resolution** (lines 11431-11488):
  - Default: "./"
  - Resolves executable path via realpath/GetFullPathNameA
  - Strips /.run/ directory if present (build output dir)
  - Result: path to project root (where meshes/, sprites/, etc. live)
- **Command-line args:**
  - --test-script <path>: runs script and exits
  - --mcp: enables MCP stdin command mode, sets non-blocking stdin
- **Platform callbacks** (lines 11505-11513):
  - close: my_close
  - render: my_render
  - resize: my_resize
  - init: my_init
  - keyb_char: my_keyb_char
  - keyb_key: my_keyb_key
  - keyb_focus: my_keyb_focus
  - mouse: my_mouse
- **Graphics settings** (lines 11517-11533):
  - 32-bit color, 8-bit alpha
  - 24-bit depth, 8-bit stencil
  - OpenGL 3.3 (USE_GL3) or 4.5 (default)
  - Debug context + double buffer
- **Sprite cookie cleanup** (lines 11564-11572):
  - Frees SpritePrefs allocated in RenderContext::RenderSprite
  - WHY in main not my_close: Sprite list still needed for shutdown rendering
- Memory leak detection (Windows debug builds only)
- Returns 0 (success)

---

## Static Data Structures

### `ini_path` (asciiid.cpp global, written at line 10982)

**Type:** `static char[4096]`  
**Purpose:** Stores path to ImGui .ini file for persistent UI layout  
**Initialized:** `my_init()` (lines 10982-10984) - "./imgui.ini"  
**Modified:** `my_init()` only  
**Read by:** ImGui (io.IniFilename)  
**Notes:**
- Fixed buffer (4096 bytes)
- Always "./imgui.ini" in current directory
- WHY static: Must outlive ImGui context (ImGui stores pointer, not copy)

### `pFont` (asciiid.cpp global, written at line 11022)

**Type:** `ImFont*`  
**Purpose:** Roboto-Medium font for ImGui UI  
**Initialized:** `my_init()` (line 11022) - loaded from fonts/Roboto-Medium.ttf  
**Modified:** `my_init()` only  
**Read by:** my_render (ImGui::PushFont/PopFont, currently commented out)  
**Notes:**
- 16px size, Unicode range 0x0020-0x03FF
- WHY separate from ImGui default: Better readability, professional appearance

### `pal_tex` (asciiid.cpp global)

**Type:** `GLuint`  
**Purpose:** OpenGL 3D texture for palette lookup  
**Initialized:** `my_init()` (lines 10947-10954) - GL_TEXTURE_3D  
**Modified:** `my_init()`, `my_close()`  
**Read by:** terrain shader (implicit via binding)  
**Notes:**
- Dimensions: 256x256x256 (RGB -> palette index)
- Format: GL_RGBA8
- Filter: GL_NEAREST (no interpolation)
- Wrap: GL_CLAMP_TO_EDGE
- Deleted in my_close()
- WHY 3D texture: Fast palette quantization lookup on GPU

---

## Control Flow Summary

### Initialization Chain

1. **OS launches asciiid**
   - `main()` called
2. **Path resolution** (lines 11431-11491)
   - Resolve executable path
   - Strip /.run/ if present
   - Set base_path
3. **Sprite preload** (lines 11498-11503)
   - LoadSprites() - global sprite list
   - LoadSprite("enemygen.xp") - editor-specific sprite
4. **Platform setup** (lines 11505-11533)
   - Register callbacks (pi struct)
   - Configure graphics (gd struct)
5. **Window creation** (line 11555)
   - a3dOpen(&pi, &gd, 0)
   - Triggers `my_init()` callback
6. **my_init() sequence** (lines 10923-11141)
   - Print OpenGL info
   - CreateWorld()
   - Scan meshes/, sprites/, fonts/, palettes/
   - Initialize materials
   - Create render context
   - Initialize ImGui
   - Create terrain (16x16 patches)
   - Set window title/icon
7. **Test script check** (lines 11557-11560)
   - If --test-script: RunTestScript(), exit
8. **Main loop** (line 11562)
   - a3dLoop() - event loop
   - Calls my_render() per frame
   - Calls my_mouse(), my_keyb_*() on events

### Shutdown Chain

1. **User closes window**
   - Platform layer detects close request
2. **my_close() called** (lines 11214-11265)
   - TermCloseAll()
   - Free MeshPrefs cookies
   - URDO_Purge()
   - DeleteWorld()
   - DeleteTerrain()
   - FreeEnemyGens()
   - PurgeItemInstCache()
   - MyFont::Free(), MyMaterial::Free()
   - ImGui shutdown
   - RenderContext::Delete()
   - a3dClose()
3. **Control returns to main** (lines 11564-11583)
   - Free SpritePrefs cookies
   - FreeSprites()
   - DumpLeakCounter()
   - Memory leak check (Windows debug)
4. **Process exits** (return 0)

---

## Testing Infrastructure

### MeshBakingTest Class (lines 11408-11423)

**Purpose:** Unit tests for mesh-to-terrain baking quantization  
**Methods:**
- `TestQuantization()` - tests height rounding (0.0-24.0)
- `RunAllTests()` - executes all tests

**Notes:**
- Currently only tests quantization (round to nearest 16 units)
- Static class (no instances needed)
- WHY struct not class: C++ default public access

### Test Script Commands

| Command | Arguments | Effect |
|---------|-----------|--------|
| SET_TERRAIN_HEIGHT | h | Set all patches to height h |
| PLACE_MESH | file x y z | Load .akm, create instance at (x,y,z) |
| BAKE_MESH_TO_TERRAIN | - | Raycast meshes onto terrain |
| EXPORT_TERRAIN_DATA | file | Write visual map (binary) |
| EXPORT_HEIGHT_SAMPLES | file | Write height map (CSV) |

**Usage:**
```bash
./asciiid --test-script tests/bake_mesh.txt
```

**Example script:**
```
# Setup terrain
SET_TERRAIN_HEIGHT 32768
PLACE_MESH meshes/rock-1.akm 10.0 10.0 32768.0
BAKE_MESH_TO_TERRAIN
EXPORT_TERRAIN_DATA terrain_out.bin
```

---

## MCP Mode

**Enabled by:** `--mcp` command-line flag  
**Effect:**
- Sets `g_mcp_mode = true`
- Makes stdin non-blocking (Unix only, lines 11546-11548)
- Allows command input during rendering

**Integration:**
- MCP commands processed in my_render() loop
- Commands parsed from stdin
- Results written to stdout (JSON)

**Supported commands:** (defined elsewhere in asciiid.cpp, not in this range)

---

## Platform Abstraction (a3d layer)

### Callback Registration

```c
PlatformInterface pi;
pi.close = my_close;
pi.render = my_render;
pi.resize = my_resize;
pi.init = my_init;
pi.keyb_char = my_keyb_char;
pi.keyb_key = my_keyb_key;
pi.keyb_focus = my_keyb_focus;
pi.mouse = my_mouse;
```

**Notes:**
- a3d layer abstracts Win32/X11/Cocoa
- Callbacks invoked from a3dLoop() event loop
- Graphics context managed by a3d layer

---

## Memory Management Patterns

### Initialization Allocations

| Object | Allocated | Freed | Notes |
|--------|-----------|-------|-------|
| world | my_init() (CreateWorld) | my_close() (DeleteWorld) | Scene graph root |
| terrain | my_init() (CreateTerrain) | my_close() (DeleteTerrain) | Quadtree heightmap |
| MeshPrefs | RenderContext (per mesh) | my_close() (loop) | Mesh placement settings |
| SpritePrefs | RenderContext (per sprite) | main() (loop) | Sprite placement settings |
| pal_tex | my_init() (glCreateTextures) | my_close() (glDeleteTextures) | Palette lookup |
| gather | Palettize() | my_close() | Quantization temp data |
| ipal | Palettize() | my_close() | Inverse palette map |

**Notes:**
- MeshPrefs freed in my_close()
- SpritePrefs freed in main() (not my_close())
  - WHY: Sprite list still used during final frames
- gather/ipal freed only if allocated (null check)

---

## Critical Globals Modified by This Section

### Input State

| Global | Type | Modified by | Purpose |
|--------|------|-------------|---------|
| mouse_in | bool | my_mouse | Mouse inside window |
| zoom_wheel | int | my_mouse | Scroll accumulator |
| reload_sprites_requested | bool | my_keyb_key() (line 11204) | F5 reload flag |

### Editor State

| Global | Type | Modified by | Purpose |
|--------|------|-------------|---------|
| g_mcp_mode | bool | main() (--mcp arg) | MCP stdin mode |

### Resources

| Global | Type | Modified by | Purpose |
|--------|------|-------------|---------|
| world | World* | my_init, my_close | Scene graph |
| terrain | Terrain* | my_init, my_close | Heightmap |
| active_mesh | Mesh* | my_init | Current mesh for placement |
| active_sprite | Sprite* | my_init | Current sprite for placement |
| enemygen_sprite | Sprite* | main() | Enemygen marker sprite |
| pal_tex | GLuint | my_init, my_close | Palette texture |
| fonts_loaded | int | my_init | Font count |
| palettes_loaded | int | my_init | Palette count |
| g_Time | double | my_init | Frame timestamp |

---

## Function Call Graph

### my_init() is called by:
- a3dOpen() (platform layer)

### my_init() calls:
- CreateWorld()
- a3dListDir() (4x: meshes, sprites, fonts, palettes)
- GetFirstMesh(), GetFirstSprite()
- RebuildWorld()
- gl3CreateTextures(), gl3TextureStorage3D(), gl3TextureParameteri3D()
- Palettize()
- MyMaterial::Init()
- MyFont::Scan(), MyPalette::Init(), MyPalette::Scan()
- CreateTerrain(), AddTerrainPatch()
- GetTerrainPatch(), GetTerrainVisualMap(), UpdateTerrainVisualMap()
- ImGui::CreateContext(), ImGui_ImplOpenGL3_Init()
- a3dSetTitle(), a3dSetIcon(), a3dSetVisible()

### my_close() is called by:
- a3dLoop() (platform layer, on window close)

### my_close() calls:
- TermCloseAll()
- glDeleteTextures()
- GetFirstMesh(), GetNextMesh(), GetMeshCookie()
- URDO_Purge()
- DeleteWorld()
- DeleteTerrain()
- FreeEnemyGens()
- PurgeItemInstCache()
- MyFont::Free(), MyMaterial::Free()
- ImGui_ImplOpenGL3_Shutdown(), ImGui::DestroyContext()
- RenderContext::Delete()
- a3dClose()

### main() is called by:
- Operating system (entry point)

### main() calls:
- realpath() / GetFullPathNameA()
- LoadSprites(), LoadSprite()
- a3dOpen()
- RunTestScript() (if --test-script)
- a3dLoop()
- GetFirstSprite(), GetNextSprite()
- GetSpriteCookie(), SetSpriteCookie()
- FreeSprites()
- DumpLeakCounter()
- _CrtDumpMemoryLeaks() (Windows debug)

---

## Cross-Reference to Other Parts

### Depends on Part 1 (lines 1-4000):
- Global variable declarations
- Struct definitions (MeshPrefs, SpritePrefs, etc.)
- Forward declarations

### Depends on Part 2 (lines 4001-8000):
- my_render() (main rendering loop)
- URDO system functions
- QueryTerrain(), HitWorld()

### Referenced by Part 1:
- Forward declarations at top of file

### Referenced by Part 2:
- my_init() sets up resources used in my_render()

---

## Design Patterns

### Initialization Order Dependency

**Problem:** Resources must be initialized in specific order (world before meshes, OpenGL before ImGui).

**Solution:**
- my_init() uses strict sequence (lines 10915-10922 comment)
- Violation causes segfaults or OpenGL errors

**Example:**
```c
world = CreateWorld();        // MUST be first (scene graph root)
a3dListDir("meshes", ...);    // Scans .akm files, creates Mesh* (needs world)
RebuildWorld(world);          // Builds BSP tree (needs meshes loaded)
render_context.Create();      // OpenGL setup (needs materials)
ImGui::CreateContext();       // MUST be last (needs OpenGL context)
```

### Resource Cleanup (RAII)

**Problem:** Cleanup must mirror initialization (reverse order).

**Solution:**
- my_close() frees in reverse order of my_init()
- Stack discipline prevents use-after-free

**Example:**
```c
// my_init()
CreateWorld();
CreateTerrain();
ImGui::CreateContext();

// my_close() (reverse order)
ImGui::DestroyContext();
DeleteTerrain();
DeleteWorld();
```

### Event-Driven Input

**Problem:** Platform events (mouse, keyboard) need OS-agnostic handling.

**Solution:**
- Platform callbacks (my_mouse, my_keyb_*) translate OS events to ImGui state
- ImGui provides cross-platform input API

**Example:**
```c
// Windows: WM_MOUSEMOVE
// macOS: NSEvent
// Linux: XEvent
// All call: my_mouse(wnd, x, y, MOUSE_MOVE)
//   -> io.MousePos = ImVec2(x, y)
```

---

## Performance Notes

1. **Terrain initialization** (lines 11034-11053)
   - Allocates 256 patches (16x16)
   - Random shuffle to avoid allocation patterns
   - WHY: Exercises quadtree insertion, tests BSP balancing

2. **ImGui frame overhead** (my_resize, my_mouse)
   - Direct state updates (no buffering)
   - WHY: ImGui immediate mode requires fresh state each frame

3. **MCP mode stdin polling** (line 11547)
   - Non-blocking read prevents frame stalls
   - WHY: 60fps rendering must not block on input

---

## Bug-Prone Areas

1. **Base path resolution** (lines 11431-11488)
   - Complex platform-specific logic
   - PATH_MAX buffer overflow risk (uses strcpy, memcpy)
   - /.run/ stripping fragile (assumes directory structure)

2. **SpritePrefs cleanup split** (main vs my_close)
   - MeshPrefs freed in my_close()
   - SpritePrefs freed in main()
   - WHY inconsistent: Sprite list still used after my_close() returns
   - RISK: Easy to forget when adding new cookie types

3. **ImGui key mapping** (lines 10989-11009)
   - Hardcoded A3D_* to ImGuiKey_* mapping
   - No validation of array bounds
   - RISK: Out-of-bounds write if IM_ARRAYSIZE(io.KeysDown) < A3D_Z

4. **Test script error handling** (lines 11295-11406)
   - No validation of patch existence before height writes
   - File write failures not checked (fopen success assumed)
   - exit(0) even on errors (should exit(1))

---

## TODO Comments in This Section

1. **my_keyb_focus** (line 11210):
   ```c
   // TODO: clear all modifiers, drags etc...
   ```
   - WHY needed: Prevents stuck state when focus lost during interaction
   - RISK: Can't undo/pan/rotate if modifiers stuck

---

## External Dependencies

### Libraries
- **ImGui** (Dear ImGui)
  - Context management, rendering
  - Input state (io struct)
  - Font loading

- **OpenGL 3.3 / 4.5**
  - Texture creation (gl3CreateTextures, etc.)
  - Debug callback (glDebugMessageCallback)

- **a3d platform layer**
  - Window creation (a3dOpen)
  - Event loop (a3dLoop)
  - File enumeration (a3dListDir)
  - Path queries (a3dGetRect)

- **Standard library**
  - File I/O (fopen, fgets, fprintf)
  - Path resolution (realpath, GetFullPathNameA)
  - String parsing (sscanf, strrchr, strstr)

### Platform-Specific

**Unix/Linux/macOS:**
- `realpath()` - canonical path resolution
- `fcntl()` - non-blocking stdin (MCP mode)

**Windows:**
- `GetFullPathNameA()` - path resolution
- `_CrtDumpMemoryLeaks()` - debug leak detection

---

## Summary of Line Range 8001-11584

This section completes the asciiid.cpp editor, covering:

1. **Platform Integration**
   - Mouse/keyboard callbacks (my_mouse, my_keyb_*)
   - Window lifecycle (my_init, my_close, my_resize)
   - Event-driven architecture via a3d layer

2. **Resource Management**
   - World/terrain creation (16x16 patches, water+grass)
   - Asset scanning (meshes, sprites, fonts, palettes)
   - Strict initialization order (dependency chain)
   - Reverse cleanup order (stack discipline)

3. **Testing Support**
   - RunTestScript() for headless integration tests
   - MeshBakingTest unit tests
   - Console debug output (DebugProbe)

4. **Entry Point**
   - main() argument parsing (--test-script, --mcp)
   - Base path resolution (handles /.run/ build dir)
   - Platform setup and main loop

**Key architectural principles:**
- **Separation of concerns:** Platform layer (a3d) vs editor logic
- **Resource ownership:** Clear init/shutdown pairs
- **Testability:** Headless script execution, unit test hooks
- **Debuggability:** Console logging, probe tools

**Critical functions for developers:**
- **Initialization:** my_init() (line 10923)
- **Shutdown:** my_close() (line 11214)
- **Input handling:** my_mouse() (line 10806), my_keyb_key() (line 11149)
- **Testing:** RunTestScript() (line 11295)

---

**END OF PART 3 DOCUMENTATION**
