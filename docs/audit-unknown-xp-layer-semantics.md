# XP Sprite Layer Semantics Audit

**Source:** `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp`  
**Lines Analyzed:** 550-600 (layer loading), plus related sections  
**Purpose:** Document layer semantics for Rust port of XP sprite loader

---

## Overview

The XP sprite format (REXPaint-based) uses a multi-layer structure where each layer has specific semantic meaning. This document details what each layer represents and how they are used in rendering.

---

## Layer Structure

### Layer 0: Background / Color Key

**Purpose:** Transparency key and metadata carrier

**Details (sprite.cpp:561):**
```cpp
XPCell* layer0 = (XPCell*)((int*)out + 4); // bg specifies color key
```

- The **background (bk) color** in Layer 0 acts as the transparency/colorkey
- Any cell matching this color is considered transparent
- Used as reference for transparency detection in swoosh merging

**Metadata Encoding (sprite.cpp:779-799):**
Layer 0 also encodes sprite atlas layout metadata in specific cells:
- `layer0[0].glyph`: Number of view angles (digit at top-left)
- `layer0[height*a].glyph`: Animation frame counts for each animation column
- `layer0[1].glyph`: Y projection reference offset (in half-blocks)
- `layer0[1+height].glyph`: Y reflection reference offset
- `layer0[2].glyph`: Z projection offset (negated)
- `layer0[2+height].glyph`: Z reflection offset (negated)

---

### Layer 1: Glyph Data / Height Map

**Purpose:** Encodes per-cell height for 2.5D rendering

**Details (sprite.cpp:562):**
```cpp
XPCell* layer1 = (XPCell*)((int*)(layer0 + cells) + 2); // glyph specifies height + '0'
```

- The **glyph value** in Layer 1 encodes height information
- Glyphs '0'-'9' represent hex digits (0-9)
- Glyphs 'A'-'Z' represent hex digits (10-35)
- This is used for depth/height mapping in 2.5D rendering
- Background is typically set to a solid color (not used for transparency)

---

### Layer 2: Primary Visual Data

**Purpose:** Main sprite rendering layer

**Details (sprite.cpp:563):**
```cpp
XPCell* layer2 = (XPCell*)((int*)(layer1 + cells) + 2); // image map
```

- Contains the actual **glyph + foreground color + background color** for rendering
- This is the layer that gets displayed
- Uses half-block glyphs (220-223) for pseudo-3D effects:
  - 220: Lower half-block
  - 221: Left half-block  
  - 222: Right half-block
  - 223: Upper half-block

---

### Layer 3+: Swoosh Overlay Layers

**Purpose:** Highlight and motion effects

**Details (sprite.cpp:579-777):**

Layers above 2 are "swoosh" overlays that add highlight/motion effects. They are merged onto Layer 2 during loading.

**Swoosh Rules:**
- The **last layer** (layers-1) has special swoosh semantics
- Swoosh is identified by **cyan foreground** (0, 255, 255) = RGB(0, 255, 255)
- Magenta background (255, 0, 255) = REXPaint transparency convention

**Half-block Swoosh (glyphs 220-223):**
- If swoosh background is transparent: average the underlying cell's coverage under the swoosh mask, lighten the foreground, preserve underlying background
- If swoosh background is opaque: average only the fg portion, set bk to swoosh's bk

**Full-block Swoosh:**
- If underlying is fully transparent: replace entirely with swoosh cell
- Otherwise: lighten each non-transparent component by +51 per channel

---

## Data Structure: XPCell

**Binary format (sprite.cpp:520-536):**
```cpp
#pragma pack(push,1)
struct XPCell
{
    uint32_t glyph;   // 4 bytes — CP437 code point
    uint8_t fg[3];    // 3 bytes — foreground RGB888
    uint8_t bk[3];    // 3 bytes — background RGB888
};                     // Total: 10 bytes per cell, packed
```

---

## Atlas Concept

### What is the Atlas?

The **atlas** is the assembled grid of sprite frames extracted from the XP layers. It's the final in-memory representation that gets passed to the renderer.

**Atlas Layout (sprite.cpp:779-882):**

The XP file's pixel grid is subdivided into individual sprite frames:
- **fr_num_x columns** = projections * total animation frames
- **fr_num_y rows** = angles (viewing directions)

Each sub-rectangle becomes one `Sprite::Frame` with its own cell buffer.

### Grid Layout Encoding

From Layer 0 metadata:
- `layer0[0].glyph` (top-left): Number of view angles
- If > 0: sprite has multiple viewing directions and projs=2 (proj + reflection)
- If 0 or non-digit: treat as single-angle (angles=1, projs=1)
- Subsequent column headers: Animation frame counts (digits scanned until non-digit or zero)

### Frame Extraction (sprite.cpp:913-949)

```cpp
for (int fr_y = 0; fr_y < fr_num_y; fr_y++)
{
    for (int fr_x = 0; fr_x < fr_num_x; fr_x++)
    {
        Sprite::Frame* frame = atlas + fr_x + fr_y * fr_num_x;
        // Extract cells for this frame from the layer data
        // ...
    }
}
```

---

## Color Constants (from sprite_constants.h)

> **WARNING:** The table below contains garbled/corrupted data from document generation. For correct color constant values, see audit-unknown-xp-format.md section 9.

| Constant | Value | Usage |
|----------|-------|-------|
| SPRITE_CYAN_R/G0, 255/B | (, 255) | Swoosh marker |
| SPRITE_MAGENTA_R/G/B | (, 255)255, 0 | REXPaintITE_SWOOSH transparency |
| SPR_INDEX | 254 | Palette index for SPRITE_TRANSPAR swoosh |
|ENT_INDEX | 255 | Palette index for transparent |

---

## Rendering Usage

The atlas is consumed by `render.cpp` via:
- `BlitSprite()` - draws sprite frames to screen
- `RenderSprite()` - main rendering entry point

**Swoosh handling in rendering (sprite.cpp:996-1024):**
```cpp
// Special palette values after quantization:
// SPRITE_TRANSPARENT_INDEX = transparent
// SPRITE_SWOOSH_INDEX = swoosh
bool fg_swoosh = (c2->fg[0] == SPRITE_CYAN_R && ...);
bool bk_swoosh = (c2->bk[0] == SPRITE_CYAN_R && ...);
// Handle swoosh transparency during render
```

---

## Summary for Rust Port

1. **Layer 0**: Background colorkey + metadata (angles, animation counts, projection offsets)
2. **Layer 1**: Height map (glyph encodes depth via hex digits)
3. **Layer 2**: Visual layer (glyph + fg color + bg color) — THIS IS THE MAIN RENDER LAYER
4. **Layer 3+**: Swoosh overlays merged onto Layer 2 during loading

**Key Insight:** The "atlas" is the assembled collection of `Sprite::Frame` structs extracted from the layered XP data, organized as a grid where columns = (projections * frames) and rows = angles.

---

## References

- `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` lines 1-57 (header documentation)
- `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` lines 520-600 (layer loading)
- `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` lines 779-882 (atlas assembly)
- `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp` lines 1445+ (swoosh rendering)
