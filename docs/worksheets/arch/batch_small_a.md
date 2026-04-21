# Batch Small A Function Analysis

Complete function documentation for: weather.cpp, weather.h, font1.cpp, font1.h, sprite_validate.cpp, enemygen.cpp, enemygen.h, screen.cpp

---

## weather.cpp (449 lines)

### `InitMatTransitionRate` (weather.cpp:44-55)

**Signature:** `static void InitMatTransitionRate()`
**Purpose:** Initialize material-to-snow transition rate lookup table for terrain accumulation.
**Called by:** CreateWeather (weather.cpp:79)
**Calls:** memset
**Globals read:** None
**Globals mutated:** mat_transition_rate[256] (static array)
**Side effects:** Initializes global static array with hardcoded material rates (grass=1.0, dirt=1.0, sand=1.0, stone=0.3, mud=0.8, cobble=0.5, gravel=0.5, water/snow=0)
**Notes:** Material IDs 0-8 are known constants from terrain.h; array uses 256 slots for full uint8_t range. Transition rate 0=immune (water, snow), 1.0=full rate (grass, dirt), fractional=partial (stone 0.3, mud 0.8). Called once at weather creation.

### `CreateWeather` (weather.cpp:60-81)

**Signature:** `Weather* CreateWeather()`
**Purpose:** Allocate and initialize Weather system with default clear weather state.
**Called by:** game.cpp:5547 (lazy init in Game::Render), asciiid.cpp:6689, asciiid.cpp:7764 (editor lazy init)
**Calls:** calloc, InitMatTransitionRate
**Globals read:** None
**Globals mutated:** None (returns allocated struct)
**Side effects:** Heap allocation (sizeof(Weather)), initializes siv::PerlinNoise via default constructor (random seed)
**Notes:** Sets state=CLEAR, intensity=0.0, snow_line=10000.0 (above all terrain), accum_rate=1.0, transition_speed=0.1, all counters to 0. Backups array is NULL until first GetOrCreateBackup call. Static guard in callers prevents double-init.

### `DeleteWeather` (weather.cpp:83-90)

**Signature:** `void DeleteWeather(Weather* w)`
**Purpose:** Deallocate Weather system and free terrain patch backup array.
**Called by:** No callers found (cleanup path not implemented)
**Calls:** free
**Globals read:** None
**Globals mutated:** None
**Side effects:** Frees w->backups dynamic array (if allocated), frees Weather struct
**Notes:** NULL-safe (returns early if w==NULL). PatchBackup array is dynamically grown via realloc in GetOrCreateBackup, must be freed here. Missing cleanup in game shutdown path.

### `SetWeather` (weather.cpp:95-103)

**Signature:** `void SetWeather(int state)`
**Purpose:** Transition weather to new state by updating target intensity.
**Called by:** game.cpp:7944 (debug key W), asciiid.cpp:6690 (ImGui combo), asciiid.cpp:7765 (MCP SET_WEATHER)
**Calls:** None
**Globals read:** weather (global pointer), state_intensity[4] (static table)
**Globals mutated:** weather->state, weather->target_intensity
**Side effects:** None (state change only, UpdateWeather performs lerp to target)
**Notes:** Clamps state to [0,3] (CLEAR, LIGHT_SNOW, HEAVY_SNOW, BLIZZARD). state_intensity maps: 0→0.0, 1→0.3, 2→0.7, 3→1.0. Intensity lerp happens in UpdateWeather via transition_speed.

### `GetWeather` (weather.cpp:105-110)

**Signature:** `int GetWeather()`
**Purpose:** Query current weather state enum as integer.
**Called by:** game.cpp:7941 (debug key W cycle), asciiid.cpp:6702 (ImGui display), MCP GET_WEATHER
**Calls:** None
**Globals read:** weather (global pointer), weather->state
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns 0 if weather==NULL (uninitialized). Returns WeatherState enum cast to int (0-3).

### `SpawnParticle` (weather.cpp:115-155)

**Signature:** `static void SpawnParticle(Weather* w, uint64_t stamp)`
**Purpose:** Allocate and initialize snow particle in ring buffer with randomized position/velocity.
**Called by:** UpdateWeather (weather.cpp:217, spawn loop)
**Calls:** fast_rand
**Globals read:** fast_rand state, snow_glyphs[4] (static table)
**Globals mutated:** w->pool.particles[], w->pool.head, w->pool.count
**Side effects:** Updates ring buffer head pointer, overwrites oldest particle if at capacity (512)
**Notes:** Spawns within 25-unit radius of cached player position. Velocity: wind + random horizontal, gravity -2 to -3 z. Lifetime 3-5 seconds. Glyph random from {0x2A, 0x2E, 0x27, 0x2C} (CP437 * . ' ,). Color 50% white (255,255,255), 50% light blue (200,220,255). Ring buffer CAPACITY=512.

### `UpdateWeather` (weather.cpp:160-230)

**Signature:** `void UpdateWeather(uint64_t stamp, float player_x, float player_y)`
**Purpose:** Per-frame weather simulation: intensity lerp, wind variation, snow line, particle spawn/update.
**Called by:** game.cpp:6668 (Game::Render, every frame)
**Calls:** SpawnParticle, fast_rand
**Globals read:** weather (global pointer), spawn_rate[4] (static table)
**Globals mutated:** w->stamp, w->_player_x, w->_player_y, w->intensity, w->pn_time, w->wind[2], w->snow_line, w->pool.particles[]
**Side effects:** Updates particle positions via velocity integration, spawns new particles probabilistically
**Notes:** Skips if dt<=0 or dt>1.0 (first frame or time gap). Intensity lerp speed 0.1/sec via w->transition_speed. Perlin wind at 0.3 time scale, amplitude 2.0*intensity. Snow line lerp from 10000 to 1000 (9000 unit drop). Fractional spawn via probability (spawn_count_f - floor(spawn_count_f)).

### `RgbToXterm256` (weather.cpp:237-243)

**Signature:** `static uint8_t RgbToXterm256(uint8_t r, uint8_t g, uint8_t b)`
**Purpose:** Convert RGB to xterm-256 color cube index (16-231 range).
**Called by:** CompositeSnowParticles (weather.cpp:292)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Maps RGB [0,255] to 6x6x6 color cube: 16 + (r*5/255)*36 + (g*5/255)*6 + (b*5/255). Integer division rounds down. Standard xterm-256 palette formula.

### `CompositeSnowParticles` (weather.cpp:245-301)

**Signature:** `void CompositeSnowParticles(Weather* w, AnsiCell* buf, int width, int height, Renderer* r, uint64_t stamp)`
**Purpose:** Overlay snow particles onto AnsiCell buffer with fade-out and color mapping.
**Called by:** game.cpp:6684 (Game::Render, after terrain/entity render)
**Calls:** ProjectCoords, RgbToXterm256
**Globals read:** None
**Globals mutated:** buf[] (AnsiCell array, overlay only)
**Side effects:** Modifies AnsiCell buffer (sets glyph and fg color, preserves bg)
**Notes:** Skips dead particles (age > lifetime). Projects 3D pos to screen via ProjectCoords. Fade last 20% of lifetime to dark (life_frac > 0.8). Brightness threshold: skip if avg(r,g,b) < 8. Color mapping: bright (>200) uses RgbToXterm256, dim uses grayscale ramp (232-255). Overlay: sets cell.gl and cell.fg only, preserves cell.bk.

### `FindBackup` (weather.cpp:306-314)

**Signature:** `static PatchBackup* FindBackup(Weather* w, Patch* patch)`
**Purpose:** Linear search for existing terrain patch backup by pointer identity.
**Called by:** GetOrCreateBackup (weather.cpp:318), SnowAccumCB (weather.cpp:405)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** O(n) scan of w->backups array. Returns NULL if not found. Compares Patch* pointers for identity (not contents).

### `GetOrCreateBackup` (weather.cpp:316-343)

**Signature:** `static PatchBackup* GetOrCreateBackup(Weather* w, Patch* patch)`
**Purpose:** Ensure terrain patch backup exists, creating and snapshotting if needed.
**Called by:** SnowAccumCB (weather.cpp:392, before snow accumulation)
**Calls:** FindBackup, realloc, GetTerrainVisualMap
**Globals read:** None
**Globals mutated:** w->backups (realloc growth), w->backup_count, w->backup_alloc
**Side effects:** Heap allocation/realloc of w->backups array, grows 64→128→256 etc. Snapshots current visual map material IDs.
**Notes:** Dynamic array growth: initial 64, doubles on overflow. Stores original_mat[64] (8x8 VISUAL_CELLS). Extracts material ID from vis[i] & 0x00FF (low byte). Returns NULL on realloc failure (no error handling).

### `SnowAccumCB` (weather.cpp:354-426)

**Signature:** `static void SnowAccumCB(Patch* p, int x, int y, int view_flags, void* cookie)`
**Purpose:** QueryTerrain callback for probabilistic snow accumulation and melt on terrain patches.
**Called by:** QueryTerrain (called from UpdateSnowAccumulation)
**Calls:** GetTerrainVisualMap, GetTerrainHeightMap, GetOrCreateBackup, FindBackup, UpdateTerrainVisualMap, fast_rand
**Globals read:** mat_transition_rate[256] (static table)
**Globals mutated:** vis[] (visual map material IDs via UpdateTerrainVisualMap)
**Side effects:** Modifies terrain visual map (material ID changes), creates patch backups
**Notes:** Dual mode: (1) accumulate if intensity>0.01 AND height>=snow_line, (2) melt if intensity<0.01 AND height<snow_line. Material 5=snow, 0=water. Transition probability: intensity*accum_rate*mat_transition_rate*0.01. Melt probability: fixed 0.02. Visual map encoding: (vis[vi] & ~0x00FF) | matid preserves elevation bits. VISUAL_CELLS=8, HEIGHT_CELLS unknown (not in file). Unused params: x, y, view_flags.

### `UpdateSnowAccumulation` (weather.cpp:433-449)

**Signature:** `void UpdateSnowAccumulation(Weather* w, Terrain* t, uint64_t stamp)`
**Purpose:** Throttled terrain modification via snow accumulation/melt callback.
**Called by:** game.cpp:6671 (Game::Render, after UpdateWeather)
**Calls:** QueryTerrain
**Globals read:** last_accum_stamp (static uint64_t)
**Globals mutated:** last_accum_stamp (static throttle timer)
**Side effects:** Modifies terrain visual maps via SnowAccumCB callback
**Notes:** Throttled to 200ms intervals (200000 microseconds) via static timer. QueryTerrain radius 40.0 units around player, view_flags=0. SnowAccumCookie passes w and stamp to callback.

---

## weather.h (99 lines)

### Structs and Enums

**`struct Particle`** (lines 16-24): 3D snow particle with position, velocity, lifetime, glyph, RGB foreground color. Used in ParticlePool ring buffer.

**`struct ParticlePool`** (lines 29-35): Fixed-capacity ring buffer (512 particles). Fields: particles[CAPACITY], count (active count), head (insertion index).

**`enum WeatherState`** (lines 40-46): Discrete weather intensities - CLEAR(0), LIGHT_SNOW(1), HEAVY_SNOW(2), BLIZZARD(3).

**`struct PatchBackup`** (lines 51-55): Stores original material IDs for terrain patch melt restoration. Fields: patch pointer, original_mat[64] (8x8 VISUAL_CELLS).

**`struct Weather`** (lines 60-82): Top-level weather system state. Contains: WeatherState, intensity/target_intensity, wind[2], snow_line, accum_rate, ParticlePool, siv::PerlinNoise, PatchBackup array. Cached player position (_player_x, _player_y) set by UpdateWeather.

### Global Variables

**`extern Weather* weather`** (line 84): Global weather system pointer. NULL until CreateWeather called. Shared by game.cpp and asciiid.cpp.

### Public API (lines 89-98)

All functions declared in weather.h, documented above in weather.cpp section.

---

## font1.cpp (389 lines)

### `LoadFont1` (font1.cpp:229-251)

**Signature:** `void LoadFont1()`
**Purpose:** Load CP437 font atlas with three color skins from sprites/font-1.xp.
**Called by:** game.cpp:3260 (InitGame)
**Calls:** LoadSprite, sprintf
**Globals read:** base_path (extern char[1024])
**Globals mutated:** font1_sprite[3] (static Sprite* array)
**Side effects:** Loads .xp file from disk 3 times (grey, gold, pink), heap allocation via LoadSprite
**Notes:** Recolor format: {count, old_r, old_g, old_b, new_r, new_g, new_b, ..., 0, 0}. Gold skin (selected): grey→yellow/gold. Pink skin (disabled): grey→magenta. Path: base_path + "sprites/font-1.xp". 5x5 pixel glyphs, 4x13 atlas (52 glyphs).

### `FreeFont1` (font1.cpp:253-258)

**Signature:** `void FreeFont1()`
**Purpose:** Unload font atlas and free memory for all three skins.
**Called by:** game.cpp:3666 (cleanup path)
**Calls:** FreeSprite
**Globals read:** None
**Globals mutated:** font1_sprite[3] (via FreeSprite)
**Side effects:** Deallocates sprite data (3 atlas textures)
**Notes:** Calls FreeSprite for all 3 skins. Safe to call if LoadFont1 was never called (FreeSprite handles NULL).

### `Font1Size` (font1.cpp:263-292)

**Signature:** `void Font1Size(const char* str, int* w, int* h)`
**Purpose:** Measure string bounding box dimensions for layout calculations.
**Called by:** mainmenu.cpp:415,473,487 (menu layout), game.cpp:11117,11132,11167,11177 (HUD layout)
**Calls:** None
**Globals read:** font1_cmap[96] (static table), font1_xadv[44] (static table), font1_yadv (static uint8_t)
**Globals mutated:** None
**Side effects:** None (read-only string measurement)
**Notes:** Accumulates horizontal advance via font1_xadv[glyph]. Counts newlines for vertical dimension (line_count * font1_yadv=4). Returns width=max line width, height=total vertical span. Handles unmapped characters (font1_cmap[ch]==99) by skipping. Output via w and h pointers (NULL-safe).

### `Font1UnderLine` (font1.cpp:294-318)

**Signature:** `void Font1UnderLine(AnsiCell* ptr, int width, int height, int dx, int dy, int w, int skin)`
**Purpose:** Render horizontal line decoration below text.
**Called by:** mainmenu.cpp:475 (title underline)
**Calls:** BlitSprite
**Globals read:** font1_sprite[3] (static array)
**Globals mutated:** ptr[] (AnsiCell buffer, overlay)
**Side effects:** Modifies AnsiCell buffer via BlitSprite
**Notes:** Uses last glyph in atlas (col 12, row 0) as horizontal line segment. Clip rect: (font1_cols-1)*font1_cell_w to font1_cols*font1_cell_w, height 1 pixel. Repeats BlitSprite across width w. Position: (dx, dy-1). Skin bounds check: returns early if skin not in [0,2].

### `Font1Paint` (font1.cpp:324-388)

**Signature:** `void Font1Paint(AnsiCell* ptr, int width, int height, int dx, int dy, const char* str, int skin, bool underline)`
**Purpose:** Render string to AnsiCell buffer with specified skin and optional underline.
**Called by:** mainmenu.cpp:474,490,512 (menu items), game.cpp:7796,7797,7833,11168,11180,11190 (HUD text)
**Calls:** BlitSprite
**Globals read:** font1_sprite[3], font1_cmap[96], font1_xadv[44], font1_yadv, font1_rows, font1_cols, font1_cell_w, font1_cell_h (all static)
**Globals mutated:** ptr[] (AnsiCell buffer, overlay)
**Side effects:** Modifies AnsiCell buffer via BlitSprite (text and optional underline)
**Notes:** Y-inversion: up_row = font1_rows - 1 - row (sprite atlas Y-up, text Y-down). Newline handling: y -= font1_yadv, x = dx. Underline mode: renders horizontal line glyph at y-1 for each character. Clip rect per glyph: [col*cell_w, up_row*cell_h, (col+1)*cell_w, (up_row+1)*cell_h]. Skin bounds check: returns early if skin not in [0,2]. Debug code: int a=0 when *str=='?' (line 346, unused).

---

## font1.h (82 lines)

### Defines and Constants

**`FONT1_GREY_SKIN`** (line 73): Skin constant 0 (normal text, grey tones).
**`FONT1_GOLD_SKIN`** (line 74): Skin constant 1 (selected text, yellow/gold).
**`FONT1_PINK_SKIN`** (line 75): Skin constant 2 (disabled text, magenta/pink).

### Public API (lines 70-79)

All functions declared in font1.h, documented above in font1.cpp section.

---

## sprite_validate.cpp (341 lines)

### `XPCell::GetDigit` (sprite_validate.cpp:39-48)

**Signature:** `int GetDigit() const`
**Purpose:** Extract digit value from CP437 glyph code (0-9, A-Z=10-35, a-z=10-35).
**Called by:** ValidateXPFile (sprite_validate.cpp:251, 258, 278, atlas metadata parsing)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None
**Notes:** Returns -1 if not a digit. Maps: '0'-'9' → 0-9, 'A'-'Z' → 10-35, 'a'-'z' → 10-35 (case insensitive for letters). Used to parse animation frame counts from XP layer 0 metadata row.

### `ValidateXPFile` (sprite_validate.cpp:63-313)

**Signature:** `bool ValidateXPFile(const char* path)`
**Purpose:** Comprehensive XP sprite file validation (gzip, layer count, dimensions, glyphs, frame alignment).
**Called by:** main (sprite_validate.cpp:334)
**Calls:** fopen, fread, fseek, ftell, fclose, malloc, free, tinfl_decompress_mem_to_heap, fprintf, XPCell::GetDigit
**Globals read:** SPRITE_MIN_LAYERS (from sprite_constants.h)
**Globals mutated:** None
**Side effects:** File I/O (read entire .xp file), heap allocation (decompressed buffer), stderr output (error messages)
**Notes:** 6 validation stages: (1) gzip header (id1=31, id2=139, cm=8), (2) skip optional fields (FEXTRA, FNAME, FCOMMENT, FHCRC), (3) decompress and check size, (4) layer count >= SPRITE_MIN_LAYERS, (5) dimensions > 0, (6) glyph range 0-255 on layers 0-2, (7) frame alignment (width % fr_num_x == 0, height % fr_num_y == 0). Atlas metadata parsing: angles from layer0[0], animation lengths from layer0[height*a]. Error format: "[SPRITE] path: error\n". Returns true on valid, false on failure.

### `main` (sprite_validate.cpp:315-340)

**Signature:** `int main(int argc, char* argv[])`
**Purpose:** CLI entry point for standalone XP validation binary.
**Called by:** Shell (executable invocation)
**Calls:** fprintf, printf, ValidateXPFile
**Globals read:** None
**Globals mutated:** None
**Side effects:** Stdout/stderr output, process exit
**Notes:** Exit codes: 0=valid, 1=validation failure, 2=usage error. Usage: sprite_validate <sprite.xp>. Prints validation criteria to stderr on usage error. Prints "OK: path passed validation" on success.

---

## enemygen.cpp (293 lines)

### `HitEnemyGen` (enemygen.cpp:116-148)

**Signature:** `EnemyGen* HitEnemyGen(double* p, double* v)`
**Purpose:** Editor raycast selection of spawn points in 3D view.
**Called by:** asciiid.cpp:10361 (editor raycast on click)
**Calls:** HitSprite, printf
**Globals read:** enemygen_sprite (extern Sprite*), enemygen_head (global linked list)
**Globals mutated:** None
**Side effects:** Stdout print "EG-HIT\n" when spawn point selected
**Notes:** O(n) scan of enemygen_head linked list. For each spawn point, tests HitSprite with enemygen_sprite. Calculates dot product proj = v·(r-p) to find closest hit (most negative proj). Returns best EnemyGen* or NULL. Debug print on success. EDITOR-only (#ifdef EDITOR).

### `DeleteEnemyGen` (enemygen.cpp:160-176)

**Signature:** `void DeleteEnemyGen(EnemyGen* eg)`
**Purpose:** Unlink spawn point from doubly-linked list (does not free).
**Called by:** asciiid.cpp:10371 (editor delete), asciiid.cpp:11284 (cleanup loop)
**Calls:** None
**Globals read:** None
**Globals mutated:** enemygen_head, enemygen_tail (if eg is head/tail)
**Side effects:** Updates global linked list pointers
**Notes:** O(1) removal. Updates prev->next or enemygen_head if first node. Updates next->prev or enemygen_tail if last node. Caller must free(eg) after unlinking. NULL-safe (no-op if eg==NULL). EDITOR-only (#ifdef EDITOR).

### `FreeEnemyGens` (enemygen.cpp:184-196)

**Signature:** `void FreeEnemyGens()`
**Purpose:** Deallocate entire spawn point linked list before world reload.
**Called by:** LoadEnemyGens (enemygen.cpp:221, before loading), asciiid.cpp:11237 (cleanup), world.cpp (inferred from LoadA3D/SaveA3D pattern)
**Calls:** free
**Globals read:** enemygen_head
**Globals mutated:** enemygen_head, enemygen_tail (reset to NULL)
**Side effects:** Heap deallocation of all EnemyGen nodes
**Notes:** Walk-and-free pattern. Resets head/tail to NULL to prevent dangling pointers. Called before LoadEnemyGens to clear stale data.

### `LoadEnemyGens` (enemygen.cpp:219-256)

**Signature:** `void LoadEnemyGens(FILE* f)`
**Purpose:** Load spawn points from .a3d binary format into global linked list.
**Called by:** asciiid.cpp:5966 (editor load), game.cpp:5454 (game load), world.cpp LoadA3D (inferred)
**Calls:** FreeEnemyGens, fread, malloc
**Globals read:** None
**Globals mutated:** enemygen_head, enemygen_tail
**Side effects:** File I/O (read spawn point data), heap allocation (malloc per spawn point)
**Notes:** Binary format: count (4 bytes), then count * 44-byte records (pos[3] float, 8 int fields). Insert-at-head O(1) insertion. TODO comment line 252: enemygen_tail = 0 should be enemygen_tail = eg when list empty (semantic bug, doesn't break current usage). Read order: pos[3], alive_max, revive_min, revive_max, armor, helmet, shield, sword, crossbow.

### `SaveEnemyGens` (enemygen.cpp:264-292)

**Signature:** `void SaveEnemyGens(FILE* f)`
**Purpose:** Write spawn points to .a3d binary format from global linked list.
**Called by:** asciiid.cpp:7803,7901 (editor save), world.cpp SaveA3D (inferred)
**Calls:** fwrite
**Globals read:** enemygen_head
**Globals mutated:** None
**Side effects:** File I/O (write spawn point data)
**Notes:** Two-pass: (1) count nodes, (2) write data. Write order matches LoadEnemyGens read order exactly. Binary format: count prefix (4 bytes), then count * 44-byte records.

---

## enemygen.h (71 lines)

### Structs

**`struct EnemyGen`** (lines 11-53): Enemy spawn point with linked list pointers, world position, population parameters (alive_max, revive_min/max), equipment probabilities (armor/helmet/shield 0-10 scale, sword/crossbow weights). Contains future extension comment for story_id (line 48).

### Global Variables (lines 58-59)

**`extern EnemyGen* enemygen_head`**: Global linked list head (first spawn point, NULL if empty).
**`extern EnemyGen* enemygen_tail`**: Global linked list tail (last spawn point, NULL if empty).

### Public API (lines 62-70)

All functions declared in enemygen.h, documented above in enemygen.cpp section. HitEnemyGen and DeleteEnemyGen are EDITOR-only (#ifdef EDITOR).

---

## screen.cpp (94 lines)

### Status: UNIMPLEMENTED

**Entire file wrapped in `#if 0`** (line 31). Contains skeleton architecture for multi-layer UI compositing but was never completed. No active code.

### `Screen::Merge` (screen.cpp:66-76, UNUSED)

**Signature:** `void Screen::Merge(AnsiCell* buf, int width, int height)`
**Purpose:** (INTENDED) Composite child layers onto parent buffer in z-order.
**Called by:** None (code disabled)
**Calls:** (INTENDED) Layer::Merge in loop
**Globals read:** None
**Globals mutated:** buf[] (INTENDED)
**Side effects:** None (code disabled)
**Notes:** INCOMPLETE. Syntax error line 72: `max(0, lay->x, )` missing third argument. Intended to traverse head→next linked list of child layers, clipping and blitting each. ClipRect struct undefined. Would enable game-view + UI-overlay composition.

### `Layer::Merge` (screen.cpp:78-84, UNUSED)

**Signature:** `void Layer::Merge(AnsiCell* buf, int width, int height, int src_x, int src_y, int dst_x, int dst_y, int )`
**Purpose:** (INTENDED) Blit layer contents with offset and clipping.
**Called by:** None (code disabled)
**Calls:** None
**Globals read:** None
**Globals mutated:** None (code disabled)
**Side effects:** None (code disabled)
**Notes:** INCOMPLETE. Signature missing final parameter type. Lines 80-83 perform ClipRect offset adjustment (cr->x1/y1/x2/y2 += x) but logic appears wrong (all add x, not x/y respectively).

### `HitTest` (screen.cpp:87-91, UNUSED)

**Signature:** `Screen* HitTest(Screen* root, int x, int y)`
**Purpose:** (INTENDED) Find topmost opaque layer under coordinate.
**Called by:** None (code disabled)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None (code disabled)
**Notes:** STUB. Comment describes intent: (1) locate topmost screen, (2) traverse down until non-transparent cell found. Not implemented. Would enable mouse/touch input dispatch to correct layer.

### Structs (screen.cpp:33-63, UNUSED)

**`struct ScreenCB`**: (INTENDED) Callback table for touch, mouse, keyboard, gamepad input. Unused.

**`struct Screen`**: (INTENDED) Base compositing layer with parent pointer, cookie (user data), child linked list (head/tail). Contains Merge method.

**`struct Layer : Screen`**: (INTENDED) Inherits Screen, adds visibility flag and sibling pointers (prev/next). Forms doubly-linked list of sibling layers.

---

## Summary

**Total entries documented:** 32 functions across 8 files

**Key findings:**
- weather.cpp: 13 functions, fully implemented Phase 17 snow weather system
- font1.cpp: 4 functions, CP437 font atlas with 3 color skins
- sprite_validate.cpp: 3 functions, standalone XP validation CLI
- enemygen.cpp: 5 functions, spawn point management with linked list
- screen.cpp: 7 functions, ALL UNUSED (wrapped in #if 0, incomplete layer compositing system)

**Caller analysis:**
- Weather functions: Called by game.cpp (render loop) and asciiid.cpp (editor/MCP)
- Font1 functions: Called by mainmenu.cpp (menu rendering) and game.cpp (HUD)
- ValidateXPFile: Self-contained CLI binary (no external callers)
- EnemyGen functions: Called by asciiid.cpp (editor), game.cpp (spawn), world.cpp (persistence)
- Screen functions: No callers (dead code)

**Notable gaps:**
- DeleteWeather has no callers (missing cleanup path)
- screen.cpp is entirely disabled (layer compositing never finished)
- LoadEnemyGens line 252 has semantic bug (tail pointer not set correctly on empty list)
