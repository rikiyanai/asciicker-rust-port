# Audit: Unknown .xp File Format (Asciicker Sprite Format)

**Date:** 2026-02-20  
**Source:** Analysis of `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` and `sprite_constants.h`  
**Status:** COMPLETE - Format fully documented

---

## Executive Summary

The `.xp` file format used by Asciicker for sprite assets is a **gzip-compressed REXPaint format** with game-specific layer semantics layered on top. The format stores multi-layer ASCII art with truecolor (RGB888) foreground and background colors, suitable for 2.5D isometric rendering.

**Key Findings:**
- Format is gzip-compressed (standard RFC 1952 container)
- Decompressed payload is little-endian binary with REXPaint-compatible structure
- Minimum 3 layers required: colorkey (L0), height (L1), visual (L2)
- Cells are 10 bytes each: 4-byte glyph + 3-byte fg RGB + 3-byte bk RGB
- Colors are quantized to 216-color xterm-256 palette for rendering
- Atlas layout (angles, animations, projections) encoded in Layer 0 metadata

---

## 1. File Format Overview

### 1.1 Container: Gzip Compression

The `.xp` file is wrapped in a standard gzip container (RFC 1952):

```
On-disk structure:
  [GZip header: ID1=31, ID2=139, CM=8, FLG, MTIME, XFL, OS]
  [Optional: FEXTRA, FNAME, FCOMMENT, FHCRC fields]
  [Deflate-compressed payload]
  [CRC32 + ISIZE trailer (8 bytes)]
```

**Gzip Header Validation** (from sprite.cpp lines 401-406):
- `id1` must equal 31 (0x1F)
- `id2` must equal 139 (0x8B)  
- `cm` must equal 8 (deflate method)

Optional gzip header fields are handled:
- `FEXTRA` - skipped via `fseek(f, len, SEEK_CUR)`
- `FNAME` - read until null terminator
- `FCOMMENT` - read until null terminator
- `FHCRC` - 2-byte CRC is read but not validated

Decompression uses `tinfl_decompress_mem_to_heap` (from the miniz/tinfl library).

### 1.2 Decompressed Payload: REXPaint-Compatible Binary

After gzip decompression, the payload is a little-endian binary structure:

```
Offset 0:   int32 version      (currently unused/skipped by loader)
Offset 4:   int32 num_layers   (number of layers)
Offset 8:   int32 width        (applies to ALL layers)
Offset 12:  int32 height       (applies to ALL layers)
Offset 16+: Per-layer data blocks
```

---

## 2. Header Structure

### 2.1 Global Header (16 bytes)

| Offset | Size   | Type   | Name       | Description                              |
|--------|--------|--------|------------|------------------------------------------|
| 0      | 4      | int32  | version    | File format version (currently unused)   |
| 4      | 4      | int32  | num_layers | Number of layers in the file            |
| 8      | 4      | int32  | width      | Width in cells (applies to all layers)  |
| 12     | 4      | int32  | height     | Height in cells (applies to all layers)  |

**Validation (from sprite.cpp lines 494-518):**
- `num_layers` must be >= 3 (defined as `SPRITE_MIN_LAYERS`)
- `width` and `height` must be >= 1

### 2.2 Per-Layer Header (8 bytes)

Each layer is preceded by its own width/height pair (though the loader primarily uses the global header values):

```
Per layer:
  int32: layer_width
  int32: layer_height
  [cells follow here]
```

**Note:** The loader skips these per-layer dimensions via pointer arithmetic: `((int*)(layer0 + cells) + 2)` advances past the 2 int32 values.

---

## 3. Cell Structure (XPCell)

### 3.1 Binary Layout (10 bytes per cell, packed)

Each cell contains glyph and color information, stored in a packed struct:

```cpp
#pragma pack(push,1)
struct XPCell
{
    uint32_t glyph;   // 4 bytes - CP437 code point (0-255 typical)
    uint8_t  fg[3];   // 3 bytes - foreground RGB888
    uint8_t  bk[3];   // 3 bytes - background RGB888
};
#pragma pack(pop)
// Total: 10 bytes per cell
```

**Cell Order:** Column-major (x varies slowest, i.e., row-by-row scanning)

### 3.2 Glyph Field

- **Type:** `uint32_t` (4 bytes, little-endian)
- **Range:** 0-255 for standard CP437 characters
- **Special values:**
  - 0 (`SPRITE_GLYPH_NULL`) - null/empty cell
  - 32 (`SPRITE_GLYPH_SPACE`) - space character
  - 219 (`SPRITE_GLYPH_FULL_BLOCK`) - solid block
  - 220-223 (`SPRITE_GLYPH_HALF_*`) - half-block variants for 2.5D effects

**Height encoding (Layer 1 semantics):**
- `'0'-'9'` = heights 0-9
- `'A'-'Z'` = heights 10-35
- `'a'-'z'` = heights 10-35 (lowercase also valid)

### 3.3 Color Fields (RGB888)

Each color component is 8-bit (0-255):
- `fg[0]` = foreground red (0-255)
- `fg[1]` = foreground green (0-255)
- `fg[2]` = foreground blue (0-255)
- `bk[0]` = background red (0-255)
- `bk[1]` = background green (0-255)
- `bk[2]` = background blue (0-255)

---

## 4. Layer Semantics

### 4.1 Layer 0: Colorkey / Background / Metadata

Layer 0 serves multiple purposes:

1. **Transparency color key:** The background color (`bk`) defines what is considered transparent. Any cell in Layer 2 whose foreground or background matches Layer 0's background color is treated as transparent.

2. **Atlas layout metadata:** Specific cells encode sprite atlas geometry:
   - `layer0[0].glyph` (top-left): Number of view angles (digit '1'-'9')
   - `layer0[height*a].glyph` (first cell of column a): Animation frame count for angle a
   - `layer0[1].glyph`: Y projection reference offset (in half-blocks)
   - `layer0[1+height].glyph`: Y reflection reference offset
   - `layer0[2].glyph`: Z projection offset (negated)
   - `layer0[2+height].glyph`: Z reflection offset (negated)

3. **Color key for transparency:**
   ```
   Transparency detection:
   - fg_transp = (layer2.fg == layer0.bk)
   - bk_transp = (layer2.bk == layer0.bk)
   - REXPaint transparency: if bk == magenta (255,0,255), both fg and bk are transparent
   ```

### 4.2 Layer 1: Height / Depth Encoding

Layer 1 encodes Z-height information for each cell, used by the 2.5D rendering system:

- Glyph values '0'-'9' map to heights 0-9
- Glyph values 'A'-'Z' map to heights 10-35
- This is stored in the `spare` field of `AnsiCell` during loading
- `SPRITE_HEIGHT_UNDEFINED` (0xFF) for non-digit glyphs

### 4.3 Layer 2: Primary Visual Data

Layer 2 contains the actual rendered appearance:
- Glyph to display
- Foreground color
- Background color
- This is the primary layer used for sprite rendering

### 4.4 Layer 3+: Swoosh Overlay Layers

Additional layers (3+) are "swoosh" overlay layers merged onto Layer 2 with special rules:

**Swoosh detection:** Cells with foreground color cyan (0, 255, 255) are swoosh indicators.

**Swoosh merging rules:**
- Half-block glyphs (220-223) with transparent background: average coverage under swoosh
- Half-block glyphs with opaque background: average only fg portion, use swoosh bk
- Full-block glyphs: lighten underlying colors by +51 per RGB channel

**Swoosh color constants:**
```cpp
SPRITE_CYAN_R = 0, SPRITE_CYAN_G = 255, SPRITE_CYAN_B = 255
SPRITE_MAGENTA_R = 255, SPRITE_MAGENTA_G = 0, SPRITE_MAGENTA_B = 255
```

---

## 5. Atlas Layout Encoding

### 5.1 Grid Structure

The .xp file encodes multiple frames in a grid layout:

- **Columns:** `projs * animation_frames` (projections × total animation frames)
- **Rows:** `angles` (view angles)

### 5.2 Projection/Reflection

If `layer0[0].glyph` is a digit (angles > 0), then:
- `projs = 2` (projection + reflection)
- Grid columns are split: first half = projections, second half = reflections

### 5.3 Animation Encoding

Animation frame counts are stored in the first cell of each column:
- `layer0[height * a].glyph` for column `a` (a = 1, 2, ...)
- Digit '1'-'9' indicates number of frames
- Scanning stops at first non-digit or zero

### 5.4 Frame Dimensions

```
fr_num_x = projs * anim_sum   // total columns
fr_num_y = angles             // total rows
fr_width = width / fr_num_x
fr_height = height / fr_num_y
```

**Validation:** Width must be evenly divisible by `fr_num_x`, height by `fr_num_y`.

---

## 6. Palette Quantization

### 6.1 RGB888 to xterm-256

The engine quantizes truecolor RGB888 to the 216-color xterm-256 palette (6×6×6 color cube):

**Quantization formula (RGB2PAL):**
```cpp
int r = (rgb[0] + 25) / 51;  // 51 = 255/5, +25 for rounding
int g = (rgb[1] + 25) / 51;
int b = (rgb[2] + 25) / 51;
return 16 + 36 * r + 6 * g + b;  // Palette index 16-231
```

**Palette index range:**
- 0-15: Standard ANSI colors (16 colors)
- 16-231: 6×6×6 = 216 color cube (primary range used)
- 232-255: Grayscale ramp (unused)

### 6.2 LoadSprite Inline Quantization

A slightly different formula is used in the sprite loader for projection vs. reflection:

```cpp
// Projection: rgb_div = 255
int r = (c2->bk[0] * 5 + 128) / 255;

// Reflection: rgb_div = 400 (produces darker result)
int r = (c2->bk[0] * 5 + 128) / 400;
```

**TODO noted in code:** These two quantization paths use different rounding strategies and should be unified.

### 6.3 Palette Constants

```cpp
SPRITE_PALETTE_STEP = 51       // 255/5 for 6-level RGB cube
SPRITE_PALETTE_ROUND = 25      // 51/2 for nearest-level rounding  
SPRITE_LIGHTEN_AMOUNT = 51     // RGB increment for swoosh lightening
```

---

## 7. Transparency Handling

### 7.1 Color Key Transparency

Transparency is determined by comparing Layer 2 colors against Layer 0's background color:

```cpp
bool bk_transp = (layer2.bk == layer0.bk);
bool fg_transp = (layer2.fg == layer0.bk);
```

### 7.2 REXPaint Native Transparency

Magenta background (255, 0, 255) forces both fg and bk transparent:

```cpp
if (c2->bk == (255, 0, 255)) {
    bk_transp = true;
    fg_transp = true;
}
```

### 7.3 Palette Sentinels

After quantization, special palette indices are used:

```cpp
SPRITE_SWOOSH_INDEX = 254       // Swoosh marker
SPRITE_TRANSPARENT_INDEX = 255  // Transparent
```

---

## 8. File Reading Code Flow

The complete loading flow (from sprite.cpp):

```
1. Open .xp file (gzip-compressed REXPaint format)
2. Parse gzip header (ID1=31, ID2=139, CM=8) and skip optional fields
3. Decompress deflate payload via tinfl_decompress_mem_to_heap
4. Parse decompressed header: version (int32), num_layers (int32)
5. Parse per-layer header: width (int32), height (int32)
6. Read cells in column-major order: glyph (uint32) + fg RGB (3 bytes) + bk RGB (3 bytes)
7. Interpret layer semantics:
   - Layer 0: Background / Color Key (bk color = transparency key)
   - Layer 1: Glyph Data — encodes height/ID per cell
   - Layer 2: Primary visual data (glyphs + colors)
   - Layer 3+: Swoosh overlay layers (merged onto Layer 2)
8. Apply swoush merging for half-block glyphs with cyan fg
9. Quantize RGB888 colors to 216-color palette indices
10. Assemble multi-angle/multi-frame sprite atlas from grid layout
```

---

## 9. Key Constants (from sprite_constants.h)

```cpp
// Layer Indices
SPRITE_LAYER_COLORKEY = 0   // Background / transparency key
SPRITE_LAYER_HEIGHT = 1     // Z-height encoding  
SPRITE_LAYER_VISUAL = 2     // Primary visual data
SPRITE_MIN_LAYERS = 3       // Minimum required layers

// Palette Sentinels
SPRITE_SWOOSH_INDEX = 254       // Swoosh marker
SPRITE_TRANSPARENT_INDEX = 255  // Transparent

// Special Colors
SPRITE_CYAN_R = 0, SPRITE_CYAN_G = 255, SPRITE_CYAN_B = 255
SPRITE_MAGENTA_R = 255, SPRITE_MAGENTA_G = 0, SPRITE_MAGENTA_B = 255

// CP437 Glyphs
SPRITE_GLYPH_NULL = 0
SPRITE_GLYPH_SPACE = 32
SPRITE_GLYPH_FULL_BLOCK = 219
SPRITE_GLYPH_HALF_LOWER = 220
SPRITE_GLYPH_HALF_LEFT = 221
SPRITE_GLYPH_HALF_RIGHT = 222
SPRITE_GLYPH_HALF_UPPER = 223

// Half-Block Quadrant Masks
SPRITE_MASK_LOWER = 0x3   // Bottom two quadrants
SPRITE_MASK_LEFT = 0x5    // Left two quadrants
SPRITE_MASK_RIGHT = 0xA   // Right two quadrants
SPRITE_MASK_UPPER = 0xC   // Top two quadrants
SPRITE_MASK_FULL = 0xF    // All four quadrants

// Height Encoding
SPRITE_HEIGHT_UNDEFINED = 0xFF
```

---

## 10. Implementation Notes

### 10.1 Byte Order

All multi-byte values are stored in **little-endian** order (least significant byte first).

### 10.2 Pointer Arithmetic for Layer Access

The loader uses pointer arithmetic to navigate layers:

```cpp
int cells = width * height;
XPCell* layer0 = (XPCell*)((int*)out + 4);  // Starts at offset 16 (after 4 ints)
XPCell* layer1 = (XPCell*)((int*)(layer0 + cells) + 2);  // +2 skips per-layer w/h
XPCell* layer2 = (XPCell*)((int*)(layer1 + cells) + 2);
```

### 10.3 Glyph Validation

The loader validates glyph values don't exceed 255 (to prevent buffer overruns in `glyph_coverage[256]` lookup):

```cpp
if (layer0[c].glyph > 255 || layer1[c].glyph > 255 || layer2[c].glyph > 255) {
    // Error: glyph out of range
}
```

### 10.4 Known TODOs in Code

1. **Quantization inconsistency:** Two different formulas for RGB→palette conversion (RGB2PAL vs inline in LoadSprite)
2. **Per-layer dimensions:** Not validated; assumes all layers have same dimensions as global header
3. **max_anims hardcoded:** Currently 16; no bounds checking if more animations encoded

---

## 11. References

- **Source file:** `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` (main loader)
- **Constants:** `/Users/r/Downloads/asciicker-Y9-2/sprite_constants.h`
- **Test files:** `/Users/r/Downloads/asciicker-Y9-2/staging/xp/*.xp`
- **Related spec:** See also `.planning/phases/35-png-xp-pipeline-fix/` for Python-side XP handling

---

## 12. Conclusion

The `.xp` format is a well-documented, gzip-compressed variant of the REXPaint format with game-specific layer semantics for 2.5D sprite rendering. The format supports:

- Multiple view angles and animation frames in a single atlas
- Truecolor (RGB888) foreground and background per cell
- Transparency via color key matching
- Height encoding for depth sorting
- Swoosh/overlay effects for motion highlights

The format is suitable for porting to Rust - the binary structure is straightforward, and the compression/decompression can be handled by standard crates (like `flate2` for gzip and `miniz`/`inflate` for deflate).

---
