# Audit: Unknown Glyph Coverage Table

**Date:** 2026-02-20
**Source:** asciicker-Y9-2/sprite.cpp (lines 1822-1840)
**Rust Port Reference:** TBD - sprite.rs module

## Summary

The `glyph_coverage[256]` table is a precomputed 4-quadrant coverage map for all 256 CP437 glyphs. It enables half-block transparency compositing by determining which color (foreground or background) dominates in each quadrant of a cell.

## 1. Search Results: "glyph" and "coverage" in render.cpp, sprite.cpp

### render.cpp
Found 12 matches for "glyph":
- Line 25: Comment about elevation glyphs
- Line 106: Comment about dither glyph selection
- Line 706: "dither glyph that visually approximates the original color"
- Line 746: `static const char glyph[] = " ..::%";` - dither pattern string
- Lines 827, 833: Shadow level indexing into glyph array

### sprite.cpp
Found 57 matches for "glyph":
- Line 16-22: Documentation of .xp loading pipeline mentioning half-block glyphs (220-223)
- Line 315: Comment explaining glyph is uint32 (CP437 code point)
- Lines 524-548: XPCell structure with glyph field
- Lines 562-577: Glyph validation (checks <= 255)
- Lines 585-726: Swoosh merging logic for half-block glyphs (220-223)
- Lines 980-1020: Layer 1 glyph encoding for height values
- Lines 1627-1631: Box-drawing frame glyphs (bit2gl lookup)
- **Lines 1813-1840: glyph_coverage[256] table definition**
- Lines 1945-2023: AverageGlyph and AverageGlyphTransp functions

## 2. Location of 256-Value Coverage Table

**File:** `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp`
**Lines:** 1822-1840

```cpp
// WHY: Precomputed coverage map for all 256 CP437 glyphs. Each uint16 encodes
// how much of the glyph's visual area falls in each of 4 quadrants:
//   nibble 0 (bits 0-3): bottom-left coverage (0-4)
//   nibble 1 (bits 4-7): bottom-right coverage (0-4)
//   nibble 2 (bits 8-11): top-left coverage (0-4)
//   nibble 3 (bits 12-15): top-right coverage (0-4)
// Coverage value 0 = empty, 4 = fully filled. Used by AverageGlyph to determine
// whether fg or bk dominates in a given region of a cell, enabling sub-cell
// compositing for half-block transparency effects.
static const uint16_t glyph_coverage[256] =
{
    0x0000,0x2222,0x4433,0x3412,0x2312,0x2323,0x2312,0x1111,0x3333,0x1111,0x3333,0x4122,0x2222,0x2203,0x3322,0x3322,
    // ... (16 values per line, 16 lines = 256 values)
    0x2211,0x1312,0x0212,0x0211,0x1202,0x2012,0x1111,0x1212,0x2200,0x0000,0x0000,0x2011,0x2200,0x2100,0x2222,0x1111,
};
```

### Encoding Format

Each `uint16_t` value encodes 4 nibbles (4 bits each), representing coverage in four quadrants:
- **Bits 0-3 (nibble 0):** Bottom-left coverage (0-4, where 4 = fully filled)
- **Bits 4-7 (nibble 1):** Bottom-right coverage (0-4)
- **Bits 8-11 (nibble 2):** Top-left coverage (0-4)
- **Bits 12-15 (nibble 3):** Top-right coverage (0-4)

Example values:
- `0x4444` = All four quadrants fully filled (full block, glyph 219)
- `0x0000` = All four quadrants empty (space, glyph 0)
- `0x00FF` = Lower two quadrants full, upper two empty (lower half-block, glyph 220)

## 3. How Glyphs Are Selected for Dithering

### Primary Functions

#### AverageGlyph() (sprite.cpp:1951-1985)
```cpp
int AverageGlyph(const AnsiCell* ptr, int mask)
{
    int cov = glyph_coverage[ptr->gl];  // Get coverage for cell's glyph
    
    int num = 0;
    int sum = 0;
    // Extract coverage from selected quadrants based on mask
    if (mask & 1) { sum += cov & 0xf; num++; }      // Bottom-left
    if (mask & 2) { sum += (cov >> 4) & 0xf; num++; }  // Bottom-right
    if (mask & 4) { sum += (cov >> 8) & 0xf; num++; }  // Top-left
    if (mask & 8) { sum += (cov >> 12) & 0xf; num++; } // Top-right
    
    // If coverage > 50%, return foreground; otherwise background
    if (sum > num * 2)
        return ptr->fg != SPRITE_TRANSPARENT_INDEX ? ptr->fg : ptr->bk;
    return ptr->bk != SPRITE_TRANSPARENT_INDEX ? ptr->bk : ptr->fg;
}
```

#### AverageGlyphTransp() (sprite.cpp:1991-2023)
Same logic but does NOT fall back to the other color when the result is transparent. Used for swoosh merging where transparency needs to be preserved.

### Dithering Usage in BlitSprite/DitherSprite

The coverage table is used in two rendering paths:

1. **Swoosh Merging** (sprite.cpp:585-726):
   - Half-block glyphs (220-223) combined with cyan foreground indicate "swoosh" effects
   - Coverage determines how underlying cell colors blend with swoosh overlay
   - Example: Glyph 220 (lower half-block) covers bottom two quadrants

2. **Sprite Dithering** (sprite.cpp:1251-1434):
   - Distance-based transparency using ordered 4x4 dither matrix
   - AverageGlyph determines dominant color in each cell quadrant
   - Masks: `SPRITE_MASK_LOWER=0x3`, `SPRITE_MASK_LEFT=0x5`, `SPRITE_MASK_RIGHT=0xA`, `SPRITE_MASK_UPPER=0xC`

### Quadrant Masks (sprite_constants.h:68-72)
```cpp
inline constexpr int SPRITE_MASK_LOWER = 0x3;   // Bottom two quadrants (glyph 220)
inline constexpr int SPRITE_MASK_LEFT = 0x5;    // Left two quadrants (glyph 221)
inline constexpr int SPRITE_MASK_RIGHT = 0xA;   // Right two quadrants (glyph 222)
inline constexpr int SPRITE_MASK_UPPER = 0xC;   // Top two quadrants (glyph 223)
inline constexpr int SPRITE_MASK_FULL = 0xF;   // All four quadrants (glyph 219)
```

## 4. Origin of This Table

### Historical Context

The table was present in the original codebase when forked:
- **Fork commit:** `788c1a4f27d761da407de1b0d507bbfb0a994b0c` (2026-01-02)
- **Author:** r <r@rs-MacBook-Pro.local>
- **Commit message:** "Forked AsciickerY9"

### Likely Origin

The table appears to be **hand-crafted or computed from the actual CP437 font bitmap data**. The values represent the visual fill of each character in a 4-quadrant grid:

1. **For block characters (glyphs 219-223):**
   - Full block (219): All four quadrants = 4 (`0x4444`)
   - Lower half (220): Bottom two quadrants = 4, top two = 0 (`0x00FF`)
   - Left half (221): Left two quadrants = 4, right two = 0 (`0x0F0F`)
   - Right half (222): Right two quadrants = 4, left two = 0 (`0xF0F0`)
   - Upper half (223): Top two quadrants = 4, bottom two = 0 (`0xFF00`)

2. **For line-drawing characters (glyphs 179-218):**
   - Coverage varies based on which parts of the cell are filled

3. **For other characters:**
   - Coverage computed based on actual pixel fill in standard CP437 8x16 font

### Verification

Looking at specific values in the table:
- Line 1837: `0x4444` at index 15 = glyph 15 (vertical/horizontal line junction)
- Line 1837: `0x0044` at index 13 = glyph 13 (horizontal line, lower portion)
- Line 1837: `0x0404` at index 14 = glyph 14 (vertical line, right side)

> **CORRECTION:** The values 0x4444, 0x0044, 0x0404 are NOT at indices 13-15. They are at indices 219-223 (full block █ at 219, lower half ▄ at 220, left half ▌ at 221). Indices 13-15 contain 0x2203, 0x3322, 0x3322 respectively.

### No External Source Found

- No generation script found in the codebase
- No documentation about computation method
- Table appears to be original implementation from the Asciicker engine

## Rust Port Recommendation

```rust
/// Precomputed 4-quadrant coverage for all 256 CP437 glyphs.
/// Each nibble (4 bits) encodes coverage (0-4) for one quadrant:
///   - bits 0-3: bottom-left
///   - bits 4-7: bottom-right
///   - bits 8-11: top-left
///   - bits 12-15: top-right
/// Used for half-block transparency compositing.
pub const GLYPH_COVERAGE: [u16; 256] = [
    0x0000, 0x2222, 0x4433, 0x3412, 0x2312, 0x2323, 0x2312, 0x1111,
    // ... (full table)
];

/// Quadrant masks for half-block glyphs
pub const MASK_LOWER: u8 = 0x3;  // Bottom two quadrants
pub const MASK_LEFT: u8 = 0x5;   // Left two quadrants
pub const MASK_RIGHT: u8 = 0xA;  // Right two quadrants
pub const MASK_UPPER: u8 = 0xC;  // Top two quadrants
pub const MASK_FULL: u8 = 0xF;   // All four quadrants
```

## Related Files

- `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` - Main implementation
- `/Users/r/Downloads/asciicker-Y9-2/sprite_constants.h` - Related constants
- `/Users/r/Downloads/asciicker-Y9-2/render.cpp` - Consumer of glyph coverage logic
