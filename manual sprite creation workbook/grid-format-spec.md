# Printable Sprite Coloring Workbook — Grid Format Specification

> **Status:** Draft for review — 2026-06-09

## 1. Overview

**Purpose:** Printable sheets for hand-drawing Asciicker XP sprites.
Each page is continuous grid paper with faint grayscale underdrawings of base sprite
frames. User prints, colors over the underdrawings by hand, scans back into the pipeline.

**Output:** Letter-size pages (8.5×11″), inkjet/laser printable.
**Scan-back target:** 300 DPI flatbed scanner or 12MP+ phone camera.

---

## 2. Page Geometry

### 2.1 Physical Dimensions

| Parameter | Value | Notes |
|---|---|---|
| Paper size | US Letter (8.5 × 11.0″) | A4 variant later |
| Print margin | 0.50″ all sides | Keeps grid away from unprintable edges |
| Printable area | 7.50 × 10.00″ (190.5 × 254.0 mm) | Grid lives entirely within this |
| Registration zone | 0.25″ band inside printable edge | For fiducials + metadata |

### 2.2 Primary Format: 5/16″ Cells

**Cell size: 5/16″ (0.3125″, 7.94 mm) square.**

Yields a **24 × 32** cell grid within the printable area:
- 24 columns × 0.3125″ = 7.500″ ✓
- 32 rows × 0.3125″ = 10.000″ ✓

### 2.3 Frame Dimensions

A **frame** = one complete sprite pose: **7 columns × 9 rows** of XP cells
(derived from player-nude.xp: 126÷18=7 wide, 72÷8=9 tall).

At 5/16″ cells:
- Frame size: 2.188 × 2.813″ (55.6 × 71.4 mm)
- 3 frames across × 3 frames tall = **9 frames per page**

This covers all 8 rotation angles (S, SW, W, NW, N, NE, E, SE) plus 1 spare
frame for notes or equipment overlay testing.

### 2.4 Frame Placement Map

```
      Col: 0───6 │ 7───13 │ 14───20 │ 21  22  23
   ┌─────────────┬────────┬─────────┬─────────────┐
 0 │ Frame 0     │Frame 1 │Frame 2  │ Reference   │
   │ (S)         │(SW)    │(W)      │ Strip       │
 9 │ Frame 3     │Frame 4 │Frame 5  │ (palette,   │
   │ (NW)        │(N)     │(NE)     │  glyphs,    │
18 │ Frame 6     │Frame 7 │Frame 8  │  heights)   │
   │ (E)         │(SE)    │(FREE)   │             │
   └─────────────┴────────┴─────────┴─────────────┘
27─31: Metadata footer
```

### 2.5 Alternate Format: 3/8″ Cells (Precision Mode)

For sprites needing more detailed hand work:

| Parameter | Value |
|---|---|
| Cell size | 3/8″ (0.375″, 9.53mm) |
| Grid | 20 × 26 cells |
| Frame size | 2.625 × 3.375″ (66.7 × 85.7mm) |
| Layout | 2 across × 2 tall = 4 frames/page |
| Coverage | 2 pages for all 8 angles |

---

## 3. Grid Line Hierarchy

Three levels, rendered as CSS borders on table cells:

| Level | Weight | Color | Purpose |
|---|---|---|---|
| **Frame border** | 1.5 pt, `#000` | Surrounds each 7×9 frame region |
| **Cell line** | 0.4 pt, `#888` | Individual XP cell boundaries |
| **Half-cell guide** | 0.2 pt, `#ccc`, dotted | 2×2 quadrant guides inside each cell |

The half-cell guides are critical: CP437 block glyphs (▄▀▌▐) use half-cell
boundaries, and the guides help users draw to those boundaries.

```
┌───────┬───────┐
│ · · · │ · · · │
│ · · · │ · · · │   ← 0.2pt dotted quadrant guides
├·······┼·······┤
│ · · · │ · · · │
│ · · · │ · · · │
└───────┴───────┘
    7.94 mm square
```

Grid lines render on top of the underdrawing (CSS border over background-image).

---

## 4. Underdrawing Specification

### 4.1 Source

Underdrawing = Layer 2 (visual) from the source .xp file.
Extracted as a specific frame at a specific angle.
Source: `player-nude.xp` (and other base sprites as needed).

### 4.2 Grayscale Rendering

Each cell's glyph is rendered as a grayscale pattern — the **shape** of the
glyph without its color:

| Glyph | Pattern | Description |
|---|---|---|
| 219 (█) | Solid dark gray | Full block — entire cell filled |
| 223 (▀) | Top half dark, bottom light | Upper half block |
| 220 (▄) | Top half light, bottom dark | Lower half block |
| 221 (▌) | Left half dark, right light | Left half block |
| 222 (▐) | Left half light, right dark | Right half block |
| 176 (░) | 2×2 dither, 25% dark | Light shade |
| 177 (▒) | Checkerboard, 50% dark | Medium shade |
| 178 (▓) | Inverted 2×2, 75% dark | Dark shade |
| 32 (space) | Fully light/white | Empty cell |

### 4.3 Opacity

Glyph "dark" pixels: **12% black** (K=12, rgb(225,225,225))
Glyph "light" (bg) pixels: **3% black** (K=3, rgb(247,247,247))

This produces a ghost image: visible enough to trace, light enough that
hand-applied marker/pencil/colored-pencil fully covers it.

**Opacity must be calibrated per printer.** Each sheet includes a test strip
with K100, K50%, K10% patches for verification (§6.2).

---

## 5. Per-Page Elements

### 5.1 Frame Labels

Each frame block's top-left corner: angle abbreviation (S, SW, W, NW, N, NE, E, SE)
in 8pt monospace, printed inside the frame border.

### 5.2 Reference Strip (cols 21-23, all rows)

Rightmost 3 columns serve as a permanent quick-reference:

| Rows | Content |
|---|---|
| 0 | Blank test cell |
| 1-2 | RGB and CMY primary color swatches (`rgb(255,0,0)` etc. as printed fills) |
| 3 | Grayscale calibration: K100, K50%, K10% |
| 4-8 | CP437 glyph reference — most common block glyphs with code + visual |
| 9-17 | Height legend — cells labeled 0-9, A-F for height encoding reference |
| 18-26 | Equipment overlay notes area |

Reference strip has same grid lines, no underdrawing, printed labels inside cells.

### 5.3 Metadata Footer (rows 27-31, cols 0-20)

```
Sheet:  ○○○○ ○○○○    Sprite: player-nude    Anim: Idle F0
Date: _____________    Artist: __________________________
Layer:  □ Visual (L2)  □ Height (L1)  □ Color Key (L0)
```

- **OMR bubbles:** 8 bubbles for sheet ID (0-255), filled by user after printing
- **Sprite/anim/frame:** pre-printed per sheet type
- **Layer checkboxes:** for height-annotation pages

---

## 6. Registration & Scanning Infrastructure

### 6.1 Corner Fiducials

Four **3/16″ (0.1875″) solid black squares** at the four corners of the
printable area (0.50″ from each paper edge).

Serves as affine transform anchors for scan alignment.

### 6.2 Color Calibration Strip

Six 0.25″ squares of known colors, printed below the grid (rows 28-31 area):

```
[K100] [K50%] [K10%] [R255] [G255] [B255]
```

Scan pipeline reads these → computes color correction matrix to compensate
for printer+scanner color shift.

### 6.3 Sheet ID

8 OMR bubbles in metadata footer. Each bubble = 0.15″ diameter circle,
fillable with standard pen/pencil. MSB first, binary encoding.
Range: 0-255 sheets.

---

## 7. Page Type Variants

| Type | Underdrawing | User Action | Result |
|---|---|---|---|
| **A — Visual** | L2 glyph shapes in grayscale | Color each cell's fg+bg | Colored L2 data |
| **B — Height** | L1 digits as faint numbers per cell | Write height code (0-9, A-Z) per cell | L1 height data |
| **C — Equipment** | No underdrawing (blank grid) | Draw equipment additions | Overlay L2 data |
| **D — Cover** | No underdrawing | Reference only | Index/legend page |

---

## 8. Generation Pipeline

1. `xp_core.XPFile` loads source .xp
2. Parse Layer 0 metadata → atlas layout (angles, anims, frame size)
3. For each frame: extract 7×9 cell region from Layer 2
4. Render each cell → grayscale pattern array
5. Generate HTML: full-grid `<table>` with CSS borders
6. Frame cells: underdrawing as `background-image` with `opacity`
7. Non-frame cells: printed labels in 6pt monospace
8. Add fiducials, calibration strip, metadata footer
9. `@page { size: letter; margin: 0.5in; }` for print CSS

---

## 9. Open Questions

1. **Should underdrawing distinguish fg vs bg regions?**
   Currently shows only glyph shape (glyph=dark, non-glyph=light).
   Alternative: two-tone grayscale (glyph area darker, bg area lighter).
   Two-tone is more information but possibly confusing. Test both.

2. **Print glyph code number inside each cell?**
   e.g., "219" in 5pt type in corner. Helps manual reference, clutters drawing area.
   Could be an alternate page variant.

3. **Color the 9th frame or leave blank?**
   Could duplicate frame 0 for testing, or leave blank for equipment overlays.
   Recommend: blank with label "Extra / Notes".

4. **Multi-page booklet vs. loose sheets?**
   Loose sheets scan better (no binding shadow). Booklet easier to organize.
   Start with loose sheets.

5. **Non-player sprites (wolfie, bigbee)?**
   Different frame sizes. Each sprite type gets its own page template with
   appropriate grid dimensions.

---

*Version: 1.0-draft*
