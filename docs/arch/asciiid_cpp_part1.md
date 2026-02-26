# asciiid.cpp Function Analysis (Lines 1-4000)

Complete analysis of all functions with definitions starting in lines 1-4000 of asciiid.cpp.
This file contains the editor's OpenGL rendering pipeline, material system, and core UI infrastructure.

---

## Global Functions (Non-static)

### `IsStdinReady` (asciiid.cpp:223-233)

**Signature:** `bool IsStdinReady()`

**Purpose:** Check if stdin has data available for reading without blocking

**Called by:** MCP command loop (stdin select check for non-blocking reads)

**Calls:** `select()` (POSIX system call)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Performs file descriptor polling via `select()`; platform-specific (disabled on Windows)

**Notes:** Uses POSIX `select()` with zero timeout for non-blocking stdin check. Required for MCP editor mode to process commands while maintaining event loop responsiveness. Windows build returns false (unimplemented).

---

### `akAPI_Exec` (asciiid.cpp:263-265)

**Signature:** `void akAPI_Exec(const char* str, int len, bool root)`

**Purpose:** Stub function for API command execution

**Called by:** No callers found via grep

**Calls:** None (empty stub)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Placeholder stub with no implementation.  reserved for future API command routing. Present to satisfy external linkage requirements from other modules.

---

### `Buzz` (asciiid.cpp:267-269)

**Signature:** `void Buzz()`

**Purpose:** Stub function for audio feedback

**Called by:** No callers found via grep

**Calls:** None (empty stub)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Empty placeholder stub. Intended for audio notification but not implemented. Satisfies external linkage requirements.

---

### `SyncConf` (asciiid.cpp:271-273)

**Signature:** `void SyncConf()`

**Purpose:** Stub function for configuration synchronization

**Called by:** No callers found via grep

**Calls:** None (empty stub)

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Empty stub. Intended for saving/synchronizing editor configuration state but not implemented.

---

### `GetConfPath` (asciiid.cpp:275-279)

**Signature:** `const char* GetConfPath()`

**Purpose:** Return path to editor configuration file

**Called by:** No callers found via grep

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Returns pointer to static string

**Notes:** Returns hardcoded "asciicker.cfg" path. TODO comment suggests USER_DIR should be used instead of relative path. Config file location is currently relative to working directory.

---

### `MergeCancel` (asciiid.cpp:618-627)

**Signature:** `void MergeCancel()`

**Purpose:** Cancel and cleanup pending map merge operation

**Called by:** ImGui UI (merge dialog cancel button)

**Calls:** `DeleteTerrain()`, `DeleteWorld()`

**Globals read:** `merge._terrain`, `merge._world`

**Globals mutated:** `merge._terrain` (nulled), `merge._world` (nulled)

**Side effects:** Deallocates terrain/world structs loaded by MergeOpen()

**Notes:** Cleans up state after merge dialog dismissed without committing. Prevents dangling pointers.

---

### `MergeOpen` (asciiid.cpp:637-697)

**Signature:** `void MergeOpen(const char* path)`

**Purpose:** Load external .a3d map file for merging into current map

**Called by:** ImGui file dialog callback for merge operation

**Calls:** `fopen()`, `LoadTerrain()`, `fread()`, `LoadWorld()`, `GetFirstMesh()`, `GetMeshName()`, `sprintf()`, `UpdateMesh()`, `malloc()`, `memset()`, `SetMeshCookie()`, `GetNextMesh()`, `fclose()`, `CreateTerrain()`, `CreateWorld()`, `RebuildWorld()`

**Globals read:** `merge._terrain`, `merge._world`, `base_path`

**Globals mutated:** `merge._terrain`, `merge._world`

**Side effects:** Opens file, reads binary terrain/world data, reloads mesh geometry from .akm files, allocates MeshPrefs cookies

**Notes:** WHY skip materials on read: Current map's materials are preserved; read-in materials discarded. Reloads meshes from .akm files to ensure merge instances reference valid geometry. Creates default terrain/world if file load fails.

---

### `MergeCommit` (asciiid.cpp:704-732)

**Signature:** `void MergeCommit()`

**Purpose:** Apply loaded merge data to current terrain and world with offset positioning

**Called by:** ImGui merge dialog confirm button

**Calls:** `URDO_Open()`, `floor()`, `GetTerrainBase()`, `SetTerrainBase()`, `QueryTerrain()`, `Merge::CommitPatch()`, `QueryWorld()`, `RebuildWorld()`, `URDO_Close()`, `MergeCancel()`

**Globals read:** `merge._terrain`, `merge._world`, `pos_x`, `pos_y`, `VISUAL_CELLS`, `terrain`, `world`

**Globals mutated:** `terrain` (patches added), `world` (instances added)

**Side effects:** Modifies terrain height map (max-merge strategy) and world instances; wraps entire operation in undo unit

**Notes:** WHY URDO_Open/Close wraps operation: Multi-patch/instance operations become single undo unit. dx/dy computed from world position with VISUAL_CELLS scaling. Terrain base offset adjusted for patch coordinate space.

---

### `GetMaterialArr` (asciiid.cpp:1649-1652)

**Signature:** `void* GetMaterialArr()`

**Purpose:** Return pointer to material array for external access

**Called by:** MyMaterial::Init(), rendering code

**Calls:** None

**Globals read:** `mat[256]`

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple getter function. Returns cast to void* to avoid circular include issues.

---

### `GetPaletteArr` (asciiid.cpp:1654-1657)

**Signature:** `void* GetPaletteArr()`

**Purpose:** Return pointer to palette array for external access

**Called by:** MyPalette initialization, rendering code

**Calls:** None

**Globals read:** `pal[256]`

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple getter function. Returns cast to void*.

---

### `GetFontArr` (asciiid.cpp:1659-1662)

**Signature:** `void* GetFontArr()`

**Purpose:** Return pointer to font array for external access

**Called by:** MyFont initialization, rendering code

**Calls:** None

**Globals read:** `font[256]`

**Globals mutated:** None

**Side effects:** None

**Notes:** Simple getter function. Returns cast to void*.

---

### `GetGLFont` (asciiid.cpp:1684-1696)

**Signature:** `int GetGLFont(int wh[2], const int wnd_wh[2], int* id)`

**Purpose:** Retrieve current font texture handle and dimensions

**Called by:** Term++ game engine (terminal rendering subsystem)

**Calls:** None

**Globals read:** `font[active_font].width`, `font[active_font].height`, `font[active_font].tex`, `active_font`

**Globals mutated:** None

**Side effects:** Writes dimensions and ID to output parameters

**Notes:** Provides GL texture handle for game rendering. Used by terminal UI overlay. wnd_wh parameter currently unused.

---

### `PrevGLFont` (asciiid.cpp:1698-1708)

**Signature:** `bool PrevGLFont()`

**Purpose:** Cycle to previous loaded font

**Called by:** ImGui editor UI (font selector hotkey)

**Calls:** `TermResizeAll()`

**Globals read:** `active_font`, `fonts_loaded`

**Globals mutated:** `active_font`

**Side effects:** Updates active font and resizes terminal

**Notes:** Decrements active_font. Returns false if already at first font (0). Triggers terminal resize for font dimension change.

---

### `NextGLFont` (asciiid.cpp:1710-1720)

**Signature:** `bool NextGLFont()`

**Purpose:** Cycle to next loaded font

**Called by:** ImGui editor UI (font selector hotkey)

**Calls:** `TermResizeAll()`

**Globals read:** `active_font`, `fonts_loaded`

**Globals mutated:** `active_font`

**Side effects:** Updates active font and resizes terminal

**Notes:** Increments active_font. Returns false if at last font. Triggers terminal resize.

---

### `glDebugCall` (asciiid.cpp:3772-3821)

**Signature:** `void GL_APIENTRY glDebugCall(GLenum source, GLenum type, GLuint id, GLenum severity, GLsizei length, const GLchar *message, const void *userParam)`

**Purpose:** OpenGL debug callback for capturing and formatting shader/API errors

**Called by:** OpenGL driver via glDebugMessageCallback

**Calls:** `printf()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Prints formatted GL error messages to console

**Notes:** Decodes GL enum values to human-readable strings. Maps source/type/severity codes. Skips GL_DEBUG_SEVERITY_NOTIFICATION (0x826B) with early return. Used to debug GL shader compilation and state errors.

---

## Static Functions (File-scoped)

### `InitSpritePrefs` (asciiid.cpp:373-410)

**Signature:** `static void InitSpritePrefs(Sprite* s)`

**Purpose:** Initialize sprite placement preferences struct if not already initialized

**Called by:** `ApplyActiveSpriteAsQuickSkin()` (line 440)

**Calls:** `GetSpriteCookie()`, `malloc()`, `memset()`, `SetSpriteCookie()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** Allocates and attaches SpritePrefs struct to sprite cookie

**Notes:** WHY: Sprite placement tools use SpritePrefs to control animation/frame selection. Cookie pattern avoids modifying Sprite struct. Sets default animation timing reps: loops use [0,4,0,0], ping-pong uses [20,2,10,4].

---

### `FindSpriteByName` (asciiid.cpp:413-429)

**Signature:** `static Sprite* FindSpriteByName(const char* name)`

**Purpose:** Find a loaded sprite by filename

**Called by:** Sprite utility (name-based lookup)

**Calls:** `GetFirstSprite()`, `GetSpriteName()`, `strcmp()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** None (read-only sprite traversal)

**Notes:** Linear search through sprite linked list. Returns first match or NULL. Used by asset management tools to cross-reference sprites by name string.

---

### `ApplyActiveSpriteAsQuickSkin` (asciiid.cpp:434-466)

**Signature:** `static bool ApplyActiveSpriteAsQuickSkin()`

**Purpose:** Apply currently selected sprite to all player skin slots for quick testing

**Called by:** ImGui editor UI (mode selector, hotkey handler)

**Calls:** `InitSpritePrefs()`, `printf()`

**Globals read:** `active_sprite`, `player[][][][][]`, `player_fall[][][][][]`, `player_attack[][][][][]`, `wolfie[][][][][]`, `wolfie_fall[][][][][]`, `wolfie_attack[][][][][]`, `bigbee[][][][][]`, `bigbee_attack[][][][][]`, `bigbee_fall[][][][][]`, `player_nude`

**Globals mutated:** All above player/mount sprite arrays

**Side effects:** Replaces all player/mount sprite lookups with active_sprite; prints warning if sprite lacks walk animation

**Notes:** WHY: Enables editor workflow of picking sprite → applying as skin → launching game without file operations. Fills all 5D mount array slots with same sprite. Warns if sprite.anims < 2 (walk anim needed).

---

### `LoadMaterialsFromA3D` (asciiid.cpp:761-805)

**Signature:** `static bool LoadMaterialsFromA3D(const char* path, Material* mats)`

**Purpose:** Load material definitions from a .a3d map file into array

**Called by:** `LoadMaterialDefaults()` (line 822)

**Calls:** `fopen()`, `fread()`, `memcmp()`, `fclose()`, `fseek()`

**Globals read:** None

**Globals mutated:** None (materials written via parameter)

**Side effects:** Reads 256 materials from disk, verifies "AS3D" signature, skips to materials section

**Notes:** Reads file header to locate materials section. Skips terrain patches using computed offset: header_size + num_patches * patch_size. Returns false if file invalid or read fails.

---

### `LoadMaterialDefaults` (asciiid.cpp:807-829)

**Signature:** `static bool LoadMaterialDefaults(Material* mats)`

**Purpose:** Load material definitions from one of several candidate .a3d files

**Called by:** `MyMaterial::Init()` (line 857)

**Calls:** `snprintf()`, `printf()`, `LoadMaterialsFromA3D()`

**Globals read:** `base_path`

**Globals mutated:** None

**Side effects:** Attempts multiple file paths in sequence; prints diagnostic messages

**Notes:** Tries game_map_y8_original_game_map.a3d first, falls back to game_map_y8.a3d or game_map_y7.a3d. Prints success/failure messages for debugging.

---

### `SampleHeightBilinear` (asciiid.cpp:3927-3959)

**Signature:** `static double SampleHeightBilinear(const uint16_t* map, double fx, double fy)`

**Purpose:** Sample terrain height at non-integer coordinates using bilinear interpolation

**Called by:** `SampleSlopeMagnitude()`, `HasElevationDelta()`, auto-material slope checks

**Calls:** `floor()`, `std::min()`

**Globals read:** `HEIGHT_CELLS`

**Globals mutated:** None

**Side effects:** None (read-only sampling)

**Notes:** WHY bilinear interpolation: Provides smooth height values for ray casting, slope calculation avoiding staircase artifacts. Clamps coordinates to grid bounds. Uses standard bilinear formula: lerp(lerp(h00, h10, tx), lerp(h01, h11, tx), ty).

---

### `SampleSlopeMagnitude` (asciiid.cpp:3961-3970)

**Signature:** `static double SampleSlopeMagnitude(const uint16_t* map, double fx, double fy, double step)`

**Purpose:** Compute terrain slope magnitude at a point using central differences

**Called by:** Auto-material elevation slope checks

**Calls:** `SampleHeightBilinear()`, `sqrt()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Uses 4-point central difference stencil (step in each cardinal direction) to compute height gradient. Magnitude = sqrt(dx^2 + dy^2). Used to classify terrain as flat/gentle/steep.

---

### `HasElevationDelta` (asciiid.cpp:3972-3989)

**Signature:** `static bool HasElevationDelta(const uint16_t* map, double fx, double fy, double step, double threshold)`

**Purpose:** Check if terrain has elevation change above threshold within a radius

**Called by:** Auto-material application (mode 1 = slope-based material selection)

**Calls:** `SampleHeightBilinear()`, `std::min()`

**Globals read:** None

**Globals mutated:** None

**Side effects:** None

**Notes:** Compares center height against 4 neighbors. Returns true if max elevation delta exceeds threshold. Used to detect slope transitions for material boundaries.

---

## Struct Member Functions (Merge, MyMaterial, MyPalette, MyFont, RenderContext)

### `Merge::CommitPatch` (asciiid.cpp:519-565)

**Signature:** `static void CommitPatch(Patch* p, int x, int y, int view_flags, void* cookie)`

**Purpose:** Merge callback to integrate a source patch into destination terrain

**Called by:** `QueryTerrain()` during MergeCommit

**Calls:** `GetTerrainPatch()`, `URDO_Create()`, `URDO_Patch()`, `GetTerrainVisualMap()`, `memcpy()`, `UpdateTerrainVisualMap()`, `UpdateTerrainHeightMap()`, `URDO_Diag()`, `GetTerrainDiag()`, `SetTerrainDiag()`

**Globals read:** `terrain`

**Globals mutated:** `terrain` (patches created/updated)

**Side effects:** Creates new destination patches if needed, merges height maps using max-height strategy

**Notes:** WHY max-height merge: Preserves tallest terrain from both maps; prevents merge from lowering terrain. Copies diagonal flag only for newly created patches. Uses undo system to track all changes.

---

### `Merge::CommitMesh` (asciiid.cpp:578-605)

**Signature:** `static void CommitMesh(Inst* i, Mesh* m, double tm[16], void* cookie)`

**Purpose:** Merge callback to copy mesh instances from source to destination map

**Called by:** `QueryWorldCB` during MergeCommit

**Calls:** `memcpy()`, `GetMeshName()`, `GetFirstMesh()`, `strcmp()`, `GetNextMesh()`, `URDO_Create()`

**Globals read:** `world`, `VISUAL_CELLS`

**Globals mutated:** `world` (instances added via URDO_Create)

**Side effects:** Adds new mesh instances to world; transforms instances by offset

**Notes:** WHY name-based matching: Allows map sections created in separate files to be recombined. Searches dest world for mesh matching source mesh name. Transform matrix translated by (dx, dy) patch coords converted to world coords via VISUAL_CELLS scaling.

---

### `MyMaterial::Free` (asciiid.cpp:837-840)

**Signature:** `static void Free()`
**Purpose:** Delete material GPU texture on shutdown
**Called by:** `my_close()` (line 11242)
**Calls:** `glDeleteTextures()`
**Globals read:** `tex` (GPU texture handle)
**Globals mutated:** None (deletes GPU texture)
**Side effects:** Releases GPU memory for 256×128 material texture
**Notes:** Deletes the single material texture that contains all 256 materials × 64 variations (4 ramps × 16 shades).

---

### `MyMaterial::Init` (asciiid.cpp:853-1162)

**Signature:** `static void Init()`
**Purpose:** Initialize all 256 material definitions with default colors, glyphs, and textures

**Called by:** Editor startup via my_init()

**Calls:** `GetMaterialArr()`, `printf()`, `LoadMaterialDefaults()`, GL texture functions

**Globals read:** None

**Globals mutated:** `mat[256]` (materials), GPU material texture

**Side effects:** Populates material array with RGB colors, glyphs, shade levels; uploads material texture to GPU

**Notes:** Explicitly defines 9 materials (Water, Grass, Dirt, Stone, Sand, Snow, Mud, Cobblestone, Gravel). 4 elevation ramps × 16 shade levels per material = 64 variations. Materials 9-255 left black (unused).

---

### `MyMaterial::Update` (asciiid.cpp:1164-1172)

**Signature:** `void MyMaterial::Update()`

**Purpose:** Update single material's GPU texture slice after color modification

**Called by:** Material painting UI

**Calls:** `glPixelStorei()`, `gl3TextureSubImage2D()`

**Globals read:** `tex` (GPU texture handle)

**Globals mutated:** `tex` (one material row updated)

**Side effects:** Uploads single row of GPU texture

**Notes:** Used for live editing of material properties without re-uploading entire 256×128 texture.

---

### `MyPalette::Load` (asciiid.cpp:1261-1290)

**Signature:** `static void MyPalette::Load(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)`

**Purpose:** Load palette colors from image by sampling grid centers

**Called by:** Image loader callback

**Calls:** `malloc()`, `Convert_UI32_AABBGGRR()`, `free()`

**Globals read:** `palettes_loaded`

**Globals mutated:** `palettes_loaded`, `pal[].rgb[]`

**Side effects:** Extracts 16×16=256 palette entries by sampling image at grid centers

**Notes:** Samples centers of 16×16 grid patch from w×h image. Stops when 256 palettes loaded.

---

### `MyFont::Load` (asciiid.cpp:1560-1628)

**Signature:** `static void MyFont::Load(void* cookie, A3D_ImageFormat f, int w, int h, const void* data, int palsize, const void* palbuf)`

**Purpose:** Load font image, generate GPU texture, and export PSF/BDF files

**Called by:** Image loader callback for each font file

**Calls:** GL texture functions, image conversion, file I/O (WriteBDF, WritePSF), qsort

**Globals read:** `fonts_loaded`

**Globals mutated:** `fonts_loaded`, `font[]` (width, height, tex), fonts array sorted

**Side effects:** Creates GL texture, exports font to PSF/BDF files, sorts font array

**Notes:** Converts image to grayscale, uploads to GPU with NEAREST filter. Generates both PSF2 (Linux) and BDF (X11) font files. Sorts fonts by area after loading.

---

### `RenderContext::Create` (asciiid.cpp:1826-3019)

**Signature:** `void RenderContext::Create()`

**Purpose:** Initialize all OpenGL rendering state: shaders, VAOs, VBOs, textures, uniforms

**Called by:** Editor startup

**Calls:** Extensive GL API calls (glCreateProgram, glCreateShader, glCompileShader, etc.)

**Globals read:** None

**Globals mutated:** `render_context` (shader programs, VAOs, VBOs, textures)

**Side effects:** Compiles all shader programs; creates GPU buffers; enables GL capabilities

**Notes:** WHY multiple shader programs: Different geometry types require different fragment processing. Includes 5 major pipelines: ANSI/terminal, mesh rendering, BSP, terrain with height/material/shade, sprite billboard. ~1200 lines of GLSL embedded inline.

---

### `RenderContext::Delete` (asciiid.cpp:3021-3044)

**Signature:** `void RenderContext::Delete()`

**Purpose:** Cleanup all OpenGL resources: delete programs, VAOs, VBOs, textures

**Called by:** Editor shutdown

**Calls:** GL deletion functions, free()

**Globals read:** `render_context.*`

**Globals mutated:** None

**Side effects:** Releases all GL resources; frees CPU ansi_buf

**Notes:** Inverse of Create(). Deletes all 5 shader programs and associated GPU state.

---

### `RenderContext::BeginPatches` (asciiid.cpp:3513-3597)

**Signature:** `void RenderContext::BeginPatches(const double* tm, const float* lt, const float* br, const float* qd, const float* pr)`

**Purpose:** Setup OpenGL state for terrain patch rendering

**Called by:** Main render loop

**Calls:** GL state setting functions, sqrt(), a3dGetTime()

**Globals read:** `render_context`, `MyMaterial::tex`, `font`, `pal_tex`

**Globals mutated:** `render_context.head`, `render_context.patches`, `render_context.draws`, `render_context.changes`, `render_context.page_tex`, `render_context.render_time`

**Side effects:** Binds terrain shader and 5 texture units; sets 8+ uniforms

**Notes:** WHY parameters: Transform matrix (tm), light direction (lt), brush state (br), quad mode (qd for probing), projection info (pr for height probing). Initializes patch batching structures.

---

### `RenderContext::EndPatches` (asciiid.cpp:3658-3694)

**Signature:** `void RenderContext::EndPatches()`

**Purpose:** Finalize terrain rendering, flush remaining batches, unbind GL state

**Called by:** Main render loop after terrain patches done

**Calls:** GL buffer/draw/bind functions

**Globals read:** `render_context.head`, `render_context.page_tex`

**Globals mutated:** `render_context.page_tex`, `render_context.head`, `render_context.render_time`

**Side effects:** Flushes all remaining TexPageBuffers; unbinds textures; records elapsed render time

**Notes:** Walks linked list of TexPageBuffers and issues final draws. Clears all texture bindings.

---

## Summary

### `Server::Send` (asciiid.cpp:282-285)

**Signature:** `bool Server::Send(const uint8_t* ptr, int size)`
**Purpose:** Send data over network (stub implementation for editor)
**Called by:** No callers found via grep
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Stub implementation always returns false. Exists only to satisfy external linkage requirements from game.cpp. Editor doesn't use server networking.

---

### `Server::Proc` (asciiid.cpp:287-289)

**Signature:** `void Server::Proc()`
**Purpose:** Process server events (stub implementation)
**Called by:** No callers found via grep
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Empty stub. placeholder for future server event processing in editor.

---

### `Server::Log` (asciiid.cpp:291-293)

**Signature:** `void Server::Log(const char* str)`
**Purpose:** Server logging interface (stub implementation)
**Called by:** No callers found via grep
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Empty stub. Exists to satisfy external linkage from game.cpp server code.

---

### `Merge::CommitSprite` (asciiid.cpp:567-570)

**Signature:** `static void CommitSprite(Inst* inst, Sprite* s, float pos[3], float yaw, int anim, int frame, int reps[4], void* cookie)`
**Purpose:** Stub callback for merging sprite instances (not implemented)
**Called by:** `QueryWorldCB struct` initialisation
**Calls:** None (assert(0))
**Globals read:** None
**Globals mutated:** None
**Side effects:** Terminates program via assert
**Notes:** Currently a stub with assert(0). Merge operations only support meshes, not sprites. Future implementation point for sprite merging.

---

### `MyPalette::Init` (asciiid.cpp:1240-1246)

**Signature:** `static void Init()`
**Purpose:** Initialize all 256 palettes with random RGB values
**Called by:** `my_init()` (line 10963)
**Calls:** `GetPaletteArr()`, `fast_rand()`
**Globals read:** None
**Globals mutated:** `pal[].rgb[768]` (all 256 palettes × 768 bytes)
**Side effects:** Fills palette array with random colors
**Notes:** Generates random palettes as fallback/default. Real palettes loaded via MyPalette::Scan during asset scanning.

---

### `MyPalette::Scan` (asciiid.cpp:1248-1259)

**Signature:** `static bool Scan(A3D_DirItem item, const char* name, void* cookie)`
**Purpose:** Directory scan callback to load palette image files
**Called by:** `a3dListDir()` in `my_init()`
**Calls:** `snprintf()`, `a3dLoadImage()`
**Globals read:** None
**Globals mutated:** None directly (palette loading via MyPalette::Load callback)
**Side effects:** Triggers image loading for .palette files
**Notes:** Passed to a3dListDir as callback. Only processes files (not directories). Calls MyPalette::Load as image load callback.

---

### `MyFont::Scan` (asciiid.cpp:1333-1344)

**Signature:** `static bool Scan(A3D_DirItem item, const char* name, void* cookie)`
**Purpose:** Directory scan callback to load font image files
**Called by:** `a3dListDir()` in `my_init()`
**Calls:** `snprintf()`, `a3dLoadImage()`
**Globals read:** None
**Globals mutated:** None directly (font loading via MyFont::Load callback)
**Side effects:** Triggers image loading for font files
**Notes:** Passed to a3dListDir as callback. Only processes files. Path cookie passed through to Load callback for BDF/PSF export.

---

### `MyFont::Sort` (asciiid.cpp:1346-1355)

**Signature:** `static int Sort(const void* a, const void* b)`
**Purpose:** qsort comparison function to sort fonts by area (width×height)
**Called by:** `qsort()` in `MyFont::Load()` (line 1627)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** Returns comparison result (-1, 0, 1)
**Notes:** Callback for C qsort(). Compares font area for ascending order: smaller fonts sorted first.

---

### `MyFont::Free` (asciiid.cpp:1357-1364)

**Signature:** `static void Free()`
**Purpose:** Delete all OpenGL font textures
**Called by:** `my_close()` (line 11241)
**Calls:** `glDeleteTextures()`, `GetFontArr()`
**Globals read:** `fonts_loaded`
**Globals mutated:** None (deletes GPU textures)
**Side effects:** Releases GPU memory for all loaded font textures
**Notes:** Iterates through all loaded fonts and calls glDeleteTextures on each texture handle.

---

### `MyFont::WritePSF` (asciiid.cpp:1366-1470)

**Signature:** `static bool WritePSF(const char* path, int w, int h, uint32_t* buf, int shift)`
**Purpose:** Export font as PSF2 (Linux console) format file
**Called by:** `MyFont::Load()` (line 1584)
**Calls:** `fopen()`, `fwrite()`, `fclose()`
**Globals read:** `cp437[256]` (unicode mapping table)
**Globals mutated:** None
**Side effects:** Writes binary .psf file to disk
**Notes:** PSF2 format: header + 256 glyph bitmaps + Unicode table. Maps CP437 codes to UTF-8. Shift parameter selects which RGBA channel to use.

---

### `MyFont::WriteBDF` (asciiid.cpp:1472-1558)

**Signature:** `static bool WriteBDF(const char* path, int w, int h, uint32_t* buf, int shift)`
**Purpose:** Export font as BDF (Bitmap Distribution Format) for X11
**Called by:** `MyFont::Load()` (line 1582)
**Calls:** `fopen()`, `fprintf()`, `fclose()`
**Globals read:** `cp437[256]`
**Globals mutated:** None
**Side effects:** Writes text .bdf file
**Notes:** BDF is a human-readable text format used by X11. Contains full font metadata and hex-encoded glyph bitmaps.

---

### `MyFont::SetTexel` (asciiid.cpp:1630-1634)

**Signature:** `void SetTexel(int x, int y, uint8_t val)`
**Purpose:** Set single texel in font texture (debug/utility function)
**Called by:** No callers found via grep
**Calls:** `gl3TextureSubImage2D()`
**Globals read:** `tex` (GPU texture handle)
**Globals mutated:** `tex` (single texel updated)
**Side effects:** Uploads single texel to GPU texture
**Notes:** Helper for modifying font textures. Creates RGBA texel from alpha value. Currently unused.

---

### `MyFont::GetTexel` (asciiid.cpp:1636-1641)

**Signature:** `uint8_t GetTexel(int x, int y)`
**Purpose:** Read single texel alpha value from font texture
**Called by:** No callers found via grep
**Calls:** `gl3GetTextureSubImage()`
**Globals read:** `tex`
**Globals mutated:** None
**Side effects:** Performs GPU readback
**Notes:** Reads RGBA texel from GPU, returns alpha channel (component 3). Currently unused. Expensive operation due to GPU-CPU transfer.

---

### `RenderContext::PaintGhost` (asciiid.cpp:3046-3113)

**Signature:** `void PaintGhost(const double* tm, int px, int py, int pz, uint16_t ghost[4 * HEIGHT_CELLS])`
**Purpose:** Render ghost/preview outline for terrain patch editing
**Called by:** `asciiid.cpp:10731`
**Calls:** GL drawing functions, `gl3NamedBufferSubData()`
**Globals read:** `ghost_tm_loc`, `ghost_cl_loc`, `ghost_vao`, `ghost_vbo`
**Globals mutated:** None (binds state only)
**Side effects:** Renders wireframe outline + transparent fill
**Notes:** Two-pass rendering: first line loop (solid outline), then triangle fan with blend (filled overlay). Shows where terrain will be modified by brush. px/py are patch coordinates, ghost array contains height values.

---

### `RenderContext::BeginBSP` (asciiid.cpp:3116-3142)

**Signature:** `void BeginBSP(const double* tm)`
**Purpose:** Initialize BSP (Binary Space Partition) rendering pass
**Called by:** `asciiid.cpp:10654`
**Calls:** GL state setup functions, `glUseProgram()`
**Globals read:** `bsp_prg`, `bsp_tm_loc`, `mesh_vao`, `mesh_vbo`
**Globals mutated:** `mesh_faces` (reset to 0), GL state
**Side effects:** Sets up depth testing, blending, shader bindings
**Notes:** Legacy BSP rendering mode. Sets GL_GEQUAL depth func, disables depth write. Batches triangles into mesh_vbo for instanced rendering.

---

### `RenderContext::RenderBSP` (asciiid.cpp:3144-3164)

**Signature:** `static void RenderBSP(int level, const float bbox[6], void* cookie)`
**Purpose:** Callback to render BSP node bounding boxes
**Called by:** `asciiid.cpp:10655` (`QueryWorldBSP(..., RenderContext::RenderBSP, ...)`)
**Calls:** `glBufferSubData()`, `glDrawArrays()`
**Globals read:** `rc->mesh_faces`, `rc->mesh_map[]`, `mesh_vbo`
**Globals mutated:** `rc->mesh_faces`, `rc->mesh_map[]`
**Side effects:** Batches bbox vertices, flushes if buffer full
**Notes:** Converts bbox to 2 triangles (6 vertices). Auto-flush at 1024 faces to avoid buffer overflow. Static method passed as callback to BSP tree traversal.

---

### `RenderContext::EndBSP` (asciiid.cpp:3166-3186)

**Signature:** `void EndBSP()`
**Purpose:** Finalize BSP rendering, flush remaining batches, restore GL state
**Called by:** `asciiid.cpp:10656`
**Calls:** `glBufferSubData()`, `glDrawArrays()`, GL cleanup functions
**Globals read:** `mesh_faces`, `mesh_map[]`
**Globals mutated:** `mesh_faces` (reset to 0), GL state
**Side effects:** Flushes any remaining bbox batches, disables depth test/blend
**Notes:** Counterpart to BeginBSP. Ensures all bboxes are drawn. Restores normal GL state (depth write enabled, depth func default).

---

### `RenderContext::BeginMeshes` (asciiid.cpp:3188-3222)

**Signature:** `void BeginMeshes(const double* tm, const float* lt)`
**Purpose:** Initialize mesh rendering pass, bind shader and textures
**Called by:** `asciiid.cpp:10594`, `asciiid.cpp:10671`
**Calls:** `glUseProgram()`, `glUniform*()`, texture binding functions
**Globals read:** `mesh_prg`, `mesh_tm_loc`, `mesh_a_tex_loc`, active font, `pal_tex`, `ansi_tex`
**Globals mutated:** `mesh_faces` (reset to 0), GL bindings
**Side effects:** Binds 3D mesh shader, 3 texture units (ansi, font, palette)
**Notes:** Sets up material shader pipeline for .akm meshes. Binds active font texture and 3D palette quantization texture. Prepares mesh_vbo for vertex batch updates.

---

### `RenderContext::RenderFace` (asciiid.cpp:3224-3249)

**Signature:** `static void RenderFace(float coords[9], uint8_t colors[12], uint32_t visual, void* cookie)`
**Purpose:** Callback to render a single mesh face triangle
**Called by:** `asciiid.cpp:3480` (`QueryMesh(m, RenderFace, rc)`), `asciiid.cpp:7329` (`QueryMesh(active_mesh, RenderContext::RenderFace, rc)`)
**Calls:** `glBufferSubData()`, `glDrawArrays()`
**Globals read:** `rc->mesh_faces`, `rc->mesh_map[]`, `mesh_vbo`
**Globals mutated:** `rc->mesh_faces`, `rc->mesh_map[]`
**Side effects:** Batches triangle vertices into GPU buffer, auto-flushes
**Notes:** Converts mesh face (3 vertices) to RenderContext::Face format. Auto-flush at 2048 faces. Static callback passed to QueryMesh for mesh traversal.

---

### `RenderContext::RenderFrame` (asciiid.cpp:3255-3357)

**Signature:** `static void RenderFrame(Sprite::Frame* f, float pos[3], void* cookie)`
**Purpose:** Render single sprite animation frame
**Called by:** `asciiid.cpp:3412`, `asciiid.cpp:3450`
**Calls:** None (just populates Face buffer)
**Globals read:** None
**Globals mutated:** `rc->mesh_map[]` (writes 2 triangles = 4 sprites)
**Side effects:** Prepares AABB quad for sprite rendering
**Notes:** Converts sprite frame AABB to 2 triangles. Batches into mesh_map buffer. Called twice per sprite (foreground + background layers).

---

### `RenderContext::EndMeshes` (asciiid.cpp:3483-3506)

**Signature:** `void EndMeshes()`
**Purpose:** Finalize mesh rendering, flush remaining batches, restore GL state
**Called by:** `asciiid.cpp:10641`, `asciiid.cpp:10688`
**Calls:** `glBufferSubData()`, `glDrawArrays()`, GL restore functions
**Globals read:** `mesh_faces`, `mesh_map[]`
**Globals mutated:** `mesh_faces` (reset to 0), GL state
**Side effects:** Flushes remaining triangles, unbinds textures
**Notes:** Counterpart to BeginMeshes. Ensures all queued triangles are drawn. Resets mesh_faces counter for next frame.

---

**Total Functions Analyzed:** 70

**Key Architecture Patterns:**
- Static member functions as callbacks for heap queries
- Dual-rendering pipelines (OpenGL for 3D, software for ASCII)
- GPU texture page batching for efficient terrain rendering
- Separate shader programs for different geometry types
- Undo/redo integration via URDO_* calls in all editing operations
- Cookie pattern for attaching per-sprite/mesh preferences

**Line Range:** All functions defined from line 223 to line 3989 (within 1-4000 limit)
