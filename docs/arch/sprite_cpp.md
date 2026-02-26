# sprite.cpp Analysis

**File Location:** `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp`

**Overview:** Sprite loading, management, and rendering. This file is the engine's bridge to the .xp asset pipeline, handling decompression of gzip-compressed REXPaint files, multi-layer interpretation, swoosh merging, palette quantization, and frame atlas assembly.

---

## Global Variables

### `static Sprite* sprite_head` (line 73)
**Purpose:** Head pointer of the global sprite linked list.
**Access Pattern:** Read by GetFirstSprite(); written by LoadSprite/FreeSprite.

### `static Sprite* sprite_tail` (line 74)
**Purpose:** Tail pointer of the global sprite linked list.
**Access Pattern:** Read by LoadSprite/FreeSprite; written by both functions during list management.

### `static int sprite_dither` (line 1239)
**Purpose:** Dither level (0-8) controlling transparency effect on sprite rendering.
**Access Pattern:** Read by BlitSprite/DitherSprite; written by SetSpriteDither().

---

## Function Reference (8 Required Fields per Function)

### `LoadPlayer` (sprite.cpp:96-132)

**Signature:** `Sprite* LoadPlayer(const char* path)`

**Purpose:** Convenience wrapper that loads a player sprite with default recolor table and detached=true (not added to global list).

**Called by:** No callers found via grep.

**Calls:** `LoadSprite(path, "player", recolor, true)` (line 109).

**Globals read:** None directly; LoadSprite reads sprite_head, sprite_tail, sprite_dither.

**Globals mutated:** None directly; LoadSprite mutates sprite_head, sprite_tail (indirectly via LoadSprite call).

**Side effects:** File I/O via LoadSprite; allocates Sprite struct and frame buffers on heap; gzip decompression; palette quantization. Returns either a new Sprite or null on failure.

**Notes:** Wrapper around LoadSprite with hardcoded detached=true, meaning the returned sprite is not inserted into the global linked list. The recolor table is a single uint8_t (0) indicating no color mapping. This is specifically for player sprites that are lifecycle-managed differently from non-player entities.

---

### `FreeSprite` (sprite.cpp:137-171)

**Signature:** `void FreeSprite(Sprite* spr)`

**Purpose:** Reference-counted sprite deallocation. Decrements refs and only frees when the last reference is released. Removes from the global sprite linked list if the sprite was linked (not detached).

**Called by:** mainmenu.cpp:FreeSprite, font1.cpp:FreeSprite(font1_sprite[0]/[1]/[2]), gamepad.cpp:FreeSprite, sprite.cpp (internal).

**Calls:** No external function calls; direct memory operations and pointer manipulation.

**Globals read:** sprite_head, sprite_tail (to check if sprite is linked list head/tail).

**Globals mutated:** sprite_head, sprite_tail (lines 151, 152, 156, 157, 182 when unlinking).

**Side effects:** Decrements spr->refs; frees allocated frame cells (malloc'd in LoadSprite line 926); frees atlas pointer; frees anim[].frame_idx arrays; frees sprite->name string; frees the Sprite struct itself. Modifies global linked list pointers.

**Notes:** Reference counting allows multiple owners to hold the same Sprite pointer without conflicting free operations. Detached sprites (prev=0, next=0) are not removed from the global list. The assertion `spr->refs>=1` (line 139) ensures the sprite wasn't already freed. Freeing a sprite removes it from both the linked list and all indices that animation frames depend on.

---

### `GetFirstSprite` (sprite.cpp:173-181)

**Signature:** `Sprite* GetFirstSprite(bool all)`

**Purpose:** Returns the head of the global sprite linked list. If all=false, skips recolored sprites (used by editor to iterate only original sprites).

**Called by:** LoadSprite (line 342) to check for duplicates before loading.

**Calls:** No external calls; pointer dereferencing only.

**Globals read:** sprite_head (line 175).

**Globals mutated:** None.

**Side effects:** None; read-only access to global state.

**Notes:** When all=false, the function walks the list skipping sprites where s->recolored==true. This allows the editor to hide player-specific recolor variants from the sprite browser while keeping them in memory.

---

### `GetPrevSprite` (sprite.cpp:183-193)

**Signature:** `Sprite* GetPrevSprite(Sprite* s, bool all)`

**Purpose:** Returns the previous sprite in the linked list. If all=false, skips recolored sprites.

**Called by:** No callers found via grep

**Calls:** No external calls; pointer dereferencing.

**Globals read:** None directly; assumes s is a valid linked list node.

**Globals mutated:** None.

**Side effects:** None; read-only traversal.

**Notes:** Navigates backward in the doubly-linked sprite list. Returns null if s is null or if no previous sprite exists (or no non-recolored previous sprite if all=false).

---

### `GetNextSprite` (sprite.cpp:195-205)

**Signature:** `Sprite* GetNextSprite(Sprite* s, bool all)`

**Purpose:** Returns the next sprite in the linked list. If all=false, skips recolored sprites.

**Called by:** No callers found via grep

**Calls:** No external calls; pointer dereferencing.

**Globals read:** None directly; assumes s is a valid linked list node.

**Globals mutated:** None.

**Side effects:** None; read-only traversal.

**Notes:** Navigates forward in the doubly-linked sprite list. Mirror of GetPrevSprite.

---

### `GetSpriteName` (sprite.cpp:207-222)

**Signature:** `int GetSpriteName(Sprite* s, char* buf, int size)`

**Purpose:** Copies the sprite's name string into buf and returns the length (strlen + 1). Handles null sprites and buffer overflow gracefully.

**Called by:** No callers found via grep

**Calls:** `strlen()`, `strncpy()` (C standard library).

**Globals read:** None; reads from s->name only.

**Globals mutated:** None.

**Side effects:** Writes to buf up to size bytes; nul-terminates the string.

**Notes:** If s is null, sets buf[0]=0 (empty string) and returns 0. Returns len+1 even if buf is null, allowing callers to query name length without allocation.

---

### `SetSpriteCookie` (sprite.cpp:224-228)

**Signature:** `void SetSpriteCookie(Sprite* s, void* cookie)`

**Purpose:** Stores an opaque pointer (cookie) in the sprite for caller-specific data association.

**Called by:** No callers found via grepcpp for entity association.

**Calls:** None.

**Globals read:** None.

**Globals mutated:** s->cookie (line 227).

**Side effects:** Writes to sprite struct; no allocation.

**Notes:** Cookie is arbitrary void* with no lifetime management by sprite.cpp. Caller is responsible for ensuring the cookie remains valid as long as the sprite exists.

---

### `GetSpriteCookie` (sprite.cpp:230-235)

**Signature:** `void* GetSpriteCookie(Sprite* s)`

**Purpose:** Retrieves the opaque cookie pointer previously set by SetSpriteCookie.

**Called by:** No grep results found; game logic.

**Calls:** None.

**Globals read:** None.

**Globals mutated:** None.

**Side effects:** None; read-only.

**Notes:** Returns null if s is null.

---

### `RGB2PAL` (sprite.cpp:260-266)

**Signature:** `int RGB2PAL(const uint8_t* rgb)`

**Purpose:** Converts a truecolor RGB triplet (0-255 per channel) to the nearest 216-color xterm palette index (16-231).

**Called by:** LoadSprite (lines 659, 700); sprite.cpp internally during swoosh merging.

**Calls:** No external calls; arithmetic only.

**Globals read:** None.

**Globals mutated:** None.

**Side effects:** None; pure computation.

**Notes:** Uses formula: per-component level = (value + 25) / 51 (quantizes 0-255 to 0-5), then palette_index = 16 + 36*r + 6*g + b. The +25 offset provides rounding to nearest level. See TODO(PIPELINE-FIX) comment at line 256: LoadSprite uses a different inline quantization formula that applies rgb_div (255 or 400) for projection vs reflection darkening, diverging from RGB2PAL's symmetric rounding.

---

### `PAL2RGB` (sprite.cpp:273-284)

**Signature:** `void PAL2RGB(int pal, uint8_t* rgb)`

**Purpose:** Inverse of RGB2PAL—converts a 216-color palette index (16-231) back to RGB888 (0-255 per channel).

**Called by:** LoadSprite (lines 676, 689, 717) during swoosh merging when updating cells.

**Calls:** None.

**Globals read:** None.

**Globals mutated:** rgb[0], rgb[1], rgb[2] (the output array).

**Side effects:** Writes to rgb output buffer.

**Notes:** Extracts r,g,b components via division/modulo, then scales each level (0-5) back to 0-255 by multiplying by 51. NOT a perfect inverse of RGB2PAL due to rounding loss in RGB2PAL's +25 offset (comment at line 271). For palette indices outside 16-231 (e.g., 0-15 or 232-255), behavior is undefined (subtracts 16 from invalid pal values).

---

### `LoadSprite` (sprite.cpp:293-1191)

**Signature:** `Sprite* LoadSprite(const char* path, const char* name, const uint8_t* recolor, bool detached)`

**Purpose:** Main .xp loading entry point. Parses gzip-compressed REXPaint files, interprets multi-layer sprite data, applies swoosh merging, quantizes colors, and assembles a Sprite atlas. This is the sole path from .xp files into the engine.

**Called by:** LoadPlayer (line 109); game.cpp (not found in grep but mentioned in header comments as primary caller).

**Calls:** 
- `fopen()`, `fread()`, `fseek()`, `ftell()`, `fclose()` — file I/O (lines 354-468)
- `tinfl_decompress_mem_to_heap()` — gzip decompression (line 458)
- `malloc()`, `free()` — memory allocation (throughout)
- `fprintf()` — error logging (lines 357, 365, 368, 391, 403, 473, 478, 501, 509, 517, 572)
- `RGB2PAL()`, `PAL2RGB()`, `LightenColor()`, `AverageGlyph()`, `AverageGlyphTransp()` — palette and glyph operations
- `GetFirstSprite()` — check for duplicates (line 342)
- `strdup()`, `strcmp()` — string management (lines 345, 1186)

**Globals read:** sprite_head, sprite_tail (for duplicate checking and list insertion).

**Globals mutated:** sprite_head, sprite_tail (lines 1176-1182 when inserting into list if not detached).

**Side effects:** 
- File I/O (opens .xp, reads gzip header and payload, closes)
- Decompression (tinfl_decompress_mem_to_heap allocates heap memory)
- Dynamic allocation: Sprite struct, Frame atlas, AnsiCell buffers per frame, animation frame_idx arrays, name string
- Palette quantization (RGB to 216-color index)
- Global list mutation (inserts sprite into linked list unless detached=true)
- Logging to stderr on errors or staging directory warnings (lines 357, 365, 368, 391, 403, 473, 478, 501, 509, 517, 572)

**Notes:**

The [DATA-CONTRACT:SPRITE] tags throughout document format boundaries:
1. **Gzip header parsing** (lines 371-442): Validates ID1=31, ID2=139, CM=8 (deflate). Handles optional FEXTRA, FNAME, FCOMMENT, FHCRC fields per RFC 1952.
2. **Decompression** (line 458): Calls tinfl_decompress_mem_to_heap to inflate deflate payload.
3. **.xp header** (lines 484-512): Reads version (skipped), num_layers, width, height. Validates layers >= SPRITE_MIN_LAYERS (3) and dimensions > 0.
4. **Layer pointer arithmetic** (lines 555-563): Each layer is preceded by 2-int (8-byte) header. Cells are in column-major order. Layer 0 (color key), Layer 1 (height encoding), Layer 2 (visual data), Layers 3+ (swoosh overlays).
5. **Glyph validation** (lines 565-577): Checks all glyphs <= 255 (CP437 range) to prevent out-of-bounds in glyph_coverage[256].
6. **Swoosh merging** (lines 579-777): Layers 3+ are merged onto Layer 2. Special rules for:
   - Cyan (0,255,255) foreground = swoosh marker
   - Magenta (255,0,255) = REXPaint transparency
   - Half-block glyphs (220-223) with masked coverage averaging
   - Full-block glyphs with lightening (+51 per channel)
7. **Atlas layout** (lines 779-854): Layer 0 encodes sprite grid layout:
   - layer0[0].glyph = number of angles (if 0 or non-digit, treat as 1 angle)
   - If angles > 0, projs=2 (projection + reflection), else projs=1
   - layer0[height*a].glyph (a=1...) = animation frame counts (scanning stops at first non-digit)
8. **Frame extraction** (lines 856-1113): Divides .xp grid into (projs * anim_sum) x angles frames. Each frame is a subrectangle extracted into AnsiCell buffer. Per-frame processing:
   - Detects meta-pos cell (c0->glyph==2) to store frame reference point
   - Extracts height from Layer 1 (spare field, '0'-'9' = 0-9, 'A'-'Z' = 10-35)
   - Detects transparency (Layer 2 color matches Layer 0 bk, or Layer 2 bk is magenta)
   - Applies recolor table if provided (RGB matching and glyph remapping)
   - Quantizes RGB to palette index with divisor=255 (projection) or 400 (reflection, darkens)
9. **Animation mapping** (lines 1129-1149): Builds frame_idx[] arrays per animation per angle per reflection. Maps (refl, angle, anim, frame) tuples to atlas frame indices.
10. **Bounding box calculation** (lines 1151-1162): Computes proj_bbox using frame height, ref[0][1], HEIGHT_SCALE, and 2D isometric projection math.

**TODO(PIPELINE-FIX) items:**
- Line 329: Per-layer width/height fields are skipped without validation; differing layer dimensions would misparse.
- Line 489: layers < 3 silently returns null with no detailed error message.
- Line 528: glyph uint32 allows values >255, which would cause out-of-bounds access in glyph_coverage[256].
- Line 597: Swoosh merging only activates on the LAST layer (m == layers-1); all other layers above 2 are simple overwrites (undocumented convention).
- Line 795: max_anims hard-coded to 16; exceeding this causes anim_len[] buffer overrun.
- Line 796: No divisibility validation on fr_num_x and fr_num_y; misaligned frames cause visual artifacts.

---

### `FillRect` (sprite.cpp:1195-1226)

**Signature:** `void FillRect(AnsiCell* ptr, int width, int height, int x, int y, int w, int h, AnsiCell ac)`

**Purpose:** Fills an axis-aligned rectangle in the AnsiCell buffer with a uniform cell value. Clips against the buffer boundaries.

**Called by:** No callers found via grepcpp for UI backgrounds or clearing regions.

**Calls:** None; direct pointer arithmetic and memory writes.

**Globals read:** None.

**Globals mutated:** None.

**Side effects:** Writes to ptr buffer in the clipped rectangle region; modifies up to w*h cells.

**Notes:** Performs in-place clipping against [0, width) x [0, height) bounds. If the rectangle is entirely out of bounds after clipping, returns without writing. Simple row-major iteration (x varies fast, y varies slow).

---

### `SetSpriteDither` (sprite.cpp:1241-1244)

**Signature:** `void SetSpriteDither(int eighths)`

**Purpose:** Sets the dither level (0-8) controlling transparency on sprite rendering. 0 = solid, 8 = fully transparent.

**Called by:** No callers found via grepcpp for distance-based sprite fading.

**Calls:** None.

**Globals read:** None.

**Globals mutated:** sprite_dither (line 1243).

**Side effects:** Modifies global dither state; affects all subsequent BlitSprite calls.

**Notes:** Used for rendering sprites at distance or during transitions by selectively skipping cells based on a 4x4 ordered dither matrix (sprite_dither_matrix, line 1231-1237). Dither level 1-8 corresponds to skipping 1/8 to 8/8 of cells.

---

### `DitherSprite` (sprite.cpp:1251-1434)

**Signature:** `void DitherSprite(AnsiCell* ptr, int width, int height, const Sprite::Frame* sf, int x, int y, const int clip[4], bool src_clip, AnsiCell* bk)`

**Purpose:** Dithered sprite blit—same as BlitSprite but skips cells based on a 4x4 ordered dither matrix. Creates a fade/transparency effect.

**Called by:** BlitSprite (line 1451) when sprite_dither > 0.

**Calls:**
- No external function calls; direct memory access and cell comparison.

**Globals read:** sprite_dither (line 1253), sprite_dither_matrix (lines 1343, 1356).

**Globals mutated:** ptr (the target buffer, lines 1348, 1391-1392, 1424-1425, 1428).

**Side effects:** Writes to ptr (target AnsiCell buffer) at destination coordinates; applies clipping; selectively skips cells based on dither level and spatial position via sprite_dither_matrix.

**Notes:**

Clipping logic (lines 1256-1305):
- If clip is null, skips clipping.
- If src_clip=true, clip is interpreted as source rectangle bounds.
- If src_clip=false, clip is interpreted as destination rectangle bounds.
Both clips are then applied against buffer boundaries [0, width) x [0, height).

Transparency handling (lines 1338-1433):
- First fills destination with bk (background cell) if provided (lines 1338-1351).
- Then iterates over source and applies dither matrix check `sprite_dither_matrix[y&3][(x1+i)&3] <= dither`.
- If dither check fails, continues (skips cell).
- If source bk is transparent (SPRITE_TRANSPARENT_INDEX):
  - Averages underlying cell coverage via AverageGlyph() with half-block glyph masks.
  - Blends fg onto bk using mask.
- Else if source fg is transparent:
  - Averages destination coverage and blends.
- Else (both opaque):
  - Direct cell copy.

This creates a spatially-varying transparency effect where some cells are skipped (showing background) and others are blended based on glyph coverage.

---

### `BlitSprite` (sprite.cpp:1436-1625)

**Signature:** `void BlitSprite(AnsiCell* ptr, int width, int height, const Sprite::Frame* sf, int x, int y, const int clip[4], bool src_clip, AnsiCell* bk)`

**Purpose:** Software rasterizer for ANSI cells. Blits a sprite frame onto the target buffer (ptr) with clipping and transparency handling. Handles "swoosh" transparency where specific glyphs/colors indicate transparency.

**Called by:** mainmenu.cpp (lines for wolfie and player rendering), font1.cpp (glyph rendering), gamepad.cpp (7 calls for button rendering), render.cpp (not found in grep but mentioned in file header), game.cpp (mentioned in comments).

**Calls:**
- DitherSprite (line 1451) if sprite_dither > 0.
- No other external calls; direct memory and cell operations.

**Globals read:** sprite_dither (line 1447), sprite_dither_matrix (indirectly via DitherSprite).

**Globals mutated:** ptr (the target buffer).

**Side effects:** Writes to ptr (target AnsiCell buffer); applies clipping; composites sprite frame with transparency. If sprite_dither > 8, returns without drawing.

**Notes:**

Clipping and bounds checking (lines 1455-1533):
- Handles optional clip[4] = [x1, y1, x2, y2] clipping rectangle.
- src_clip=true means clip is source-space; src_clip=false means clip is destination-space.
- Also clips against buffer boundaries [0, width) x [0, height).
- Adjusts source (sx, sy) and destination (x, y) offsets when clipping.
- Returns early if width or height becomes zero or negative.

Rendering loop (lines 1538-1625):
- First fills destination with bk (background) if bk pointer is provided.
- Then iterates over destination (y1..y2) x (x1..x2) and source (sy..sy+h) x (sx..sx+w).
- Per-cell logic:
  - If source bk == SPRITE_TRANSPARENT_INDEX (255):
    - If both fg and bk transparent or gl is space (32), skip cell.
    - Else average glyph coverage (AverageGlyph) with mask based on glyph type (half-blocks 220-223 vs full-block).
    - Set destination bk to averaged color, preserve source fg and gl.
  - Else if source fg == SPRITE_TRANSPARENT_INDEX:
    - If gl is full-block (219), skip.
    - Else average glyph coverage, set destination fg to averaged color, preserve source bk and gl.
  - Else (both opaque):
    - Direct cell copy.

This compositing strategy allows half-transparent cells (transparency only in fg or bk) to blend with the destination, preserving the alpha channel information encoded in the glyph coverage table.

---

### `PaintFrame` (sprite.cpp:1632-1810)

**Signature:** `void PaintFrame(AnsiCell* ptr, int width, int height, int x, int y, int w, int h, const int dst_clip[4], uint8_t fg, uint8_t bk, bool dbl, bool combine)`

**Purpose:** Draws a box-drawing frame (UI border) using CP437 box-drawing glyphs. Uses a bit-encoded glyph lookup table (bit2gl[16]) where each bit represents a connection direction (bottom-left, bottom-right, top-left, top-right). The 'combine' flag enables merging with existing box-drawing characters in the buffer, so overlapping frames share junction glyphs.

**Called by:** No callers found via grepcpp or UI panels for dialog/window borders.

**Calls:** `memset()` (C standard library, line 1710).

**Globals read:** None directly; uses static lookup tables gl2bit_raw[] and gl2bit_cmb[] (lines 1706-1707).

**Globals mutated:** Modifies static gl2bit_cmb[] initialization flag (line 1708).

**Side effects:** Writes to ptr (target AnsiCell buffer) at frame border positions; initializes static lookup table on first call.

**Notes:**

Box-drawing bit encoding (bit2gl[16], line 1704):
- Bits: 0=BL, 1=BR, 2=TL, 3=TR (bottom-left, bottom-right, top-left, top-right connections)
- Glyph values map to CP437 box-drawing characters (176-206 range)
- Example: 0xC (1100 binary) = BL + BR = bottom horizontal = glyph 205

Combining logic (lines 1706-1719):
- If combine=false, uses gl2bit_raw[] (all zeros), forcing each cell to be drawn fresh.
- If combine=true, uses gl2bit_cmb[] (initialized on first call), allowing existing glyphs to be "OR"ed with new connections.
- This enables overlapping frames to merge at junctions intelligently.

Frame drawing (lines 1721-1809):
- Clips frame against dst_clip and buffer bounds.
- Draws 4 sides (bottom, top, left, right) with corner handling.
- For each edge cell, looks up existing glyph bits via gl2bit[], ORs with new direction bit(s), and looks up result in bit2gl[] to get the combined glyph.
- If fg or bk != SPRITE_TRANSPARENT_INDEX (255), also updates color.

---

### `AverageGlyph` (sprite.cpp:1951-1985)

**Signature:** `int AverageGlyph(const AnsiCell* ptr, int mask)`

**Purpose:** Determines the dominant color of a cell within a masked region (quadrants). Each CP437 glyph has precomputed 4-quadrant coverage in glyph_coverage[256]. If the glyph's coverage exceeds 50% in the masked area, returns fg; otherwise bk. Falls back to the other color if the primary is transparent (255).

**Called by:** BlitSprite (7 calls, lines 1563-1579), DitherSprite (7 calls, lines 1372-1388), render.cpp (28+ calls for lighting/shadow effects), sprite.cpp internally (LightenColor, line 1933).

**Calls:** None; arithmetic only.

**Globals read:** glyph_coverage[] (line 1957).

**Globals mutated:** None.

**Side effects:** None; pure computation.

**Notes:**

Coverage encoding (bits per quadrant, 0-4 per nibble):
- Bits 0-3: BL (bottom-left), Bits 4-7: BR (bottom-right), Bits 8-11: TL (top-left), Bits 12-15: TR (top-right)
- Each quadrant value 0-4 represents fill percentage (0=empty, 4=fully filled)

Mask parameter (bits 1-8):
- 0x1 = BL, 0x2 = BR, 0x4 = TL, 0x8 = TR
- Caller selects which quadrants to average over

Threshold (line 1982):
- Averages coverage sum over masked quadrants
- If average > 50% (sum > num * 2), returns fg
- Otherwise returns bk
- Falls back to the other color if primary is SPRITE_TRANSPARENT_INDEX (255)

Used for half-block transparency: when one channel (fg or bk) is transparent, AverageGlyph determines which quadrants are filled by the glyph, then returns the non-transparent color for those regions and the transparent color for empty regions.

---

### `AverageGlyphTransp` (sprite.cpp:1991-2023)

**Signature:** `int AverageGlyphTransp(const AnsiCell* ptr, int mask)`

**Purpose:** Same coverage-based averaging as AverageGlyph but does NOT fall back to the other color when the result is transparent (255). Returns the raw fg or bk even if transparent.

**Called by:** LoadSprite during swoosh merging (lines 663, 664, 704) to preserve transparency information.

**Calls:** None; arithmetic only.

**Globals read:** glyph_coverage[] (line 1995).

**Globals mutated:** None.

**Side effects:** None; pure computation.

**Notes:** Identical logic to AverageGlyph except lines 2020-2022 return fg or bk directly without checking for SPRITE_TRANSPARENT_INDEX. This preserves transparency during swoosh merging where the result is transparent (e.g., averaging a masked region where all quadrants are empty, or the underlying color is transparent). Used to compute separate fg and bk averages under swoosh overlays (lines 663-664 in LoadSprite).

---

## Data Structures

### `struct XPCell` (lines 532-552)

**Binary layout:** 10 bytes, packed (no padding).
- `uint32_t glyph` (4 bytes): CP437 code point (0-255 typical)
- `uint8_t fg[3]` (3 bytes): Foreground RGB888
- `uint8_t bk[3]` (3 bytes): Background RGB888

**Method:** `GetDigit()` (lines 538-551) — extracts digit value from glyph ('0'-'9' = 0-9, 'A'-'Z' = 10-35, 'a'-'z' = 10-35). Used for parsing animation frame counts and angle metadata from Layer 0.

### `struct GZ` (lines 379-384)

**Binary layout:** 10 bytes, gzip header structure.
- `uint8_t id1, id2, cm, flg` (4 bytes)
- `uint8_t mtime[4]` (4 bytes)
- `uint8_t xfl, os` (2 bytes)

### `struct SpriteInst` (lines 76-81)

**Purpose:** Unused;  be leftover from earlier design.
- `Sprite* sprite`
- `int pos[3]`
- `int anim`

---

## Lookup Tables

### `glyph_coverage[256]` (lines 1822-1840)

**Purpose:** Precomputed 4-quadrant coverage map for all 256 CP437 glyphs. Each uint16 encodes coverage per nibble (bits 0-3=BL, 4-7=BR, 8-11=TL, 12-15=TR). Values 0-4 indicate fill percentage per quadrant. Used by AverageGlyph/AverageGlyphTransp for half-transparent compositing.

### `sprite_dither_matrix[4][4]` (lines 1231-1237)

**Purpose:** 4x4 ordered dither pattern. Values 1-8 distributed spatially so that cells where matrix[y%4][x%4] <= dither_level are skipped, creating N/8 transparency. Used by DitherSprite for distance-based fading.

### `palette_rgb[256]` (lines 1847-1897)

**Purpose:** Maps each palette index (0-255) to a packed RGB value where each nibble is a level (0-5). Used by DarkenGlyph to decompose palette colors into per-component levels for darkening arithmetic. Indices 0-15 and 232-255 are undefined (0x000); indices 16-231 encode the 216-color cube.

### `bit2gl[16]` (line 1704)

**Purpose:** Maps 4-bit direction encoding (BL/BR/TL/TR) to CP437 box-drawing glyphs. Used by PaintFrame to render frame borders.

### `gl2bit_raw[256]` and `gl2bit_cmb[256]` (lines 1706-1707)

**Purpose:** Reverse mapping from glyph to direction bits. gl2bit_raw is all zeros (non-combining); gl2bit_cmb is initialized on first use to allow merging overlapping frames. Used by PaintFrame with combine flag.

---

## Constants

**Defined in sprite_constants.h (not shown, referenced):**
- `SPRITE_CYAN_R/G/B` — RGB marker for swoosh overlays
- `SPRITE_MAGENTA_R/G/B` — RGB marker for REXPaint transparency
- `SPRITE_SWOOSH_INDEX` — Palette index for swoosh (254)
- `SPRITE_TRANSPARENT_INDEX` — Palette index for transparency (255)
- `SPRITE_GLYPH_FULL_BLOCK` — CP437 code for full block (219)
- `SPRITE_GLYPH_HALF_*` — Half-block glyphs (220-223)
- `SPRITE_MASK_*` — Quadrant masks for half-blocks
- `SPRITE_HEIGHT_UNDEFINED` — Sentinel for invalid height encoding
- `SPRITE_LIGHTEN_AMOUNT` — RGB increment for swoosh brightening (+51)
- `SPRITE_MIN_LAYERS` — Minimum layer count (3)
- `HEIGHT_SCALE` — Isometric projection scale factor

---

## Error Handling

File I/O errors (LoadSprite):
- Line 357: File not found → fprintf + return null
- Line 391: Gzip header read failed → fprintf + return null
- Line 403: Invalid gzip signature → fprintf + return null
- Line 473: Decompression failed → fprintf + free + return null
- Line 478: Decompressed size mismatch → fprintf + free + return null

Validation errors (LoadSprite):
- Line 501: Layer count < SPRITE_MIN_LAYERS → fprintf + free + return null
- Line 509: Invalid dimensions (width/height < 1) → fprintf + free + return null
- Line 572: Glyph out of range (>255) → fprintf + free + return null
- Line 868: Frame width not divisible by column count → fprintf + free + return null
- Line 875: Frame height not divisible by angle count → fprintf + free + return null

All errors log to stderr with context (path, expected vs actual values). On failure, LoadSprite cleans up decompressed buffer (free(out)) and returns null.

---

## Performance Characteristics

**LoadSprite complexity:**
- Gzip decompression: O(input_size) (tinfl_decompress_mem_to_heap)
- Layer parsing: O(width * height * num_layers) — quadratic in sprite resolution
- Frame extraction: O(frames * frame_width * frame_height) — per-cell quantization
- Animation mapping: O(angles * anims * animation_frames)
- Overall: O(width * height * num_layers) dominated by frame extraction

**BlitSprite complexity:**
- Per-destination-cell: O(1) glyph lookup + palette arithmetic
- Overall: O(clip_width * clip_height) — linear in rendered area

**Memory usage:**
- Sprite struct: sizeof(Sprite) + sizeof(Sprite::Anim) * num_anims
- Atlas frames: num_frames * frame_width * frame_height * sizeof(AnsiCell)
- Animation indices: num_anims * 2 * angles * anim_length[i] * sizeof(int)
- Total: roughly (width * height * 8 bytes) + overhead

---

### `LightenColor` (sprite.cpp:1903-1929)

**Signature:** `int LightenColor(int c)`

**Purpose:** Lighten a palette color by one level (increment RGB components in 6-level cube). Maps palette index to RGB levels, adds 1 to each component (clamped to 5), then maps back to palette index.

**Called by:**
- `BlitSprite()` (lines 676, 717) — lighten foreground colors for highlighted cells
- `sprite.cpp` internally (line 1933) — referenced in DarkenGlyph comment

**Calls:**
- Arithmetic only (palette index ↔ RGB level conversion)

**Globals read:**
- `palette_rgb[256]` (lines 1847-1897) — palette index to RGB level mapping

**Globals mutated:** None

**Side effects:** None (pure function)

**Notes:**
- Uses 6×6×6 = 216-color cube (palette indices 16-231)
- Each RGB component is a level 0-5
- Clamping: min(level + 1, 5) prevents overflow
- Returns modified palette index
- Inverse operation of DarkenGlyph's color darkening

---

### `DarkenGlyph` (sprite.cpp:1931-1948)

**Signature:** `int DarkenGlyph(const AnsiCell* ptr)`

**Purpose:** Darken a cell's foreground and background by one level (decrement RGB components). Modifies both fg and bk colors in-place, clamping to level 0.

**Called by:**
- `render.cpp` — lighting and shadow effects (multiple calls)
- Used for ambient occlusion and shadow darkening

**Calls:**
- `AverageGlyph()` — determine dominant color in cell quadrants (for glyph preservation)

**Globals read:**
- `palette_rgb[256]` — palette to RGB level mapping

**Globals mutated:**
- Input `AnsiCell* ptr` (modifies fg and bk fields in-place)

**Side effects:**
- Modifies cell colors destructively
- Preserves glyph character

**Notes:**
- **TODO (line 1905):** "make lookup table" — currently recalculates RGB decomposition on every call; could precompute like LightenColor
- Decrements each RGB component: max(level - 1, 0)
- Operates on 216-color cube (indices 16-231)
- Inverse operation of LightenColor

---

## Known Issues & TODOs

1. **TODO(PIPELINE-FIX) line 256:** Quantization divergence — RGB2PAL vs LoadSprite inline formulas use different rounding. Should unify or justify.

2. **TODO(PIPELINE-FIX) line 329:** Per-layer width/height skipped without validation. Heterogeneous layer dimensions would cause misparse.

3. **TODO(PIPELINE-FIX) line 489:** No error message for layers < 3. Silently returns null.

4. **TODO(PIPELINE-FIX) line 528:** glyph >255 not validated on load. Would cause out-of-bounds in glyph_coverage[256].

5. **TODO(PIPELINE-FIX) line 597:** Swoosh activation only on last layer. Undocumented convention; other layers above 2 are simple overwrites.

6. **TODO(PIPELINE-FIX) line 795:** max_anims hard-coded to 16. Exceeding this causes anim_len[] buffer overrun.

7. **TODO(PIPELINE-FIX) line 796:** No divisibility validation on frame alignment. Misaligned sprites cause visual artifacts.

8. **Line 1905:** DarkenGlyph comment "todo make lookup table". DarkenGlyph recalculates RGB decomposition on every call; could precompute like LightenColor.

---

## Summary Table

| Function | Lines | Purpose | Callers (grep-verified) |
|----------|-------|---------|------------------------|
| LoadPlayer | 96-132 | Wrapper for player sprite loading | None found in main codebase |
| FreeSprite | 137-171 | Reference-counted sprite deallocation | mainmenu.cpp, font1.cpp, gamepad.cpp |
| GetFirstSprite | 173-181 | Return head of global sprite list | LoadSprite (duplicate check) |
| GetPrevSprite | 183-193 | Navigate to previous sprite in list | None found |
| GetNextSprite | 195-205 | Navigate to next sprite in list | None found |
| GetSpriteName | 207-222 | Query sprite name | None found |
| SetSpriteCookie | 224-228 | Store opaque pointer in sprite | None found |
| GetSpriteCookie | 230-235 | Retrieve opaque pointer from sprite | None found |
| RGB2PAL | 260-266 | RGB888 to 216-color palette index | LoadSprite (lines 659, 700) |
| PAL2RGB | 273-284 | 216-color palette index to RGB888 | LoadSprite (lines 676, 689, 717) |
| LoadSprite | 293-1191 | Main .xp file loader | LoadPlayer, game.cpp (mentioned) |
| FillRect | 1195-1226 | Fill rectangle with uniform cell | None found |
| SetSpriteDither | 1241-1244 | Set dither transparency level | None found |
| DitherSprite | 1251-1434 | Dithered sprite blit | BlitSprite (line 1451) |
| BlitSprite | 1436-1625 | Main sprite rendering function | mainmenu.cpp, font1.cpp, gamepad.cpp, render.cpp |
| PaintFrame | 1632-1810 | Draw box-drawing frame borders | None found |
| AverageGlyph | 1951-1985 | Determine dominant color in quadrants | BlitSprite, DitherSprite, render.cpp, DarkenGlyph |
| AverageGlyphTransp | 1991-2023 | Average glyph coverage preserving transparency | LoadSprite (swoosh merging) |

