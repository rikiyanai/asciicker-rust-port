# Manual Sprite Creation Workbook

Complete guide to creating XP sprites by hand for Asciicker

---

# SET 1: INFORMATION PAGES

## 1.1 XP File Format Overview

An .xp file is a gzip-compressed REXPaint format with 3+ layers:

```
+-------------------------------------------------------------------------+
| .xp FILE STRUCTURE                                                       |
+-------------------------------------------------------------------------+
|                                                                         |
| GZIP COMPRESSED:                                                        |
|   +-----------------------------------------------------------------+  |
|   | Global Header (16 bytes):                                        |  |
|   |   - version: i32 (always -1)                                   |  |
|   |   - num_layers: i32 (must be >=3)                             |  |
|   |   - width: i32 (cells across)                                  |  |
|   |   - height: i32 (cells tall)                                   |  |
|   +-----------------------------------------------------------------+  |
|                                                                         |
|   Per Layer:                                                            |
|   +-----------------------------------------------------------------+  |
|   | Layer Header (8 bytes): width, height                           |  |
|   | Cells: width x height x 10 bytes each                          |  |
|   |   Each cell = glyph(4) + fg_rgb(3) + bk_rgb(3) = 10 bytes   |  |
|   +-----------------------------------------------------------------+  |
|                                                                         |
| LAYOUT: Column-major order (x changes slowest)                         |
|   index = col x height + row                                           |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 1.2 Layer Semantics

```
+-------------------------------------------------------------------------+
| LAYER 0: COLOR KEY + ATLAS METADATA                                    |
+-------------------------------------------------------------------------+
|                                                                         |
| EVERY CELL contains:                                                    |
|   - bk (background RGB) = COLOR KEY for transparency                   |
|     (Any cell in L2 matching this = transparent)                      |
|                                                                         |
| METADATA CELLS (only first few rows/columns, rest are just color key): |
|   +-----------------------------------------------------------------+  |
|   | [0,0]      -> ANGLE COUNT (digit '1'-'8')                      |  |
|   |             If > 0: multi-angle sprite, projs=2                |  |
|   |             If 0 or non-digit: single-angle, projs=1          |  |
|   +-----------------------------------------------------------------+  |
|   | [1,0]      -> Y projection offset (in half-blocks)              |  |
|   | [1,height] -> Y reflection offset                             |  |
|   +-----------------------------------------------------------------+  |
|   | [2,0]      -> Z projection offset (negated)                    |  |
|   | [2,height] -> Z reflection offset                             |  |
|   +-----------------------------------------------------------------+  |
|   | [heightx1, 0] -> Animation 1 frame count (digit)              |  |
|   | [heightx2, 0] -> Animation 2 frame count (digit)              |  |
|   | [heightx3, 0] -> Animation 3 frame count (digit)              |  |
|   | ...                                                             |  |
|   +-----------------------------------------------------------------+  |
|                                                                         |
| EXAMPLE: player sprite with 8 angles, 2 animations (4 frames each)    |
|   - layer0[0] = '8' (8 angles)                                       |
|   - layer0[heightx1] = '4' (anim1 has 4 frames)                      |
|   - layer0[heightx2] = '4' (anim2 has 4 frames)                      |
|   - All other cells = your chosen color key (e.g., magenta)           |
|                                                                         |
+-------------------------------------------------------------------------+

+-------------------------------------------------------------------------+
| LAYER 1: HEIGHT ENCODING                                               |
+-------------------------------------------------------------------------+
|                                                                         |
| EVERY CELL contains:                                                    |
|   +-----------------------------------------------------------------+  |
|   | glyph = encodes Z-height for depth sorting                       |  |
|   |   '0'-'9' (ASCII 48-57)  -> heights 0-9                      |  |
|   |   'A'-'Z' (ASCII 65-90)  -> heights 10-35                     |  |
|   |   'a'-'z' (ASCII 97-122) -> heights 10-35 (case-insensitive) |  |
|   |   Any other glyph       -> undefined height (0xFF)              |  |
|   +-----------------------------------------------------------------+  |
|                                                                         |
| VISUAL HEIGHT REFERENCE:                                                |
|                                                                         |
|   '0' = ground level (feet)                                           |
|   '1'-'9' = increasingly tall                                          |
|   'A' = 10 units (ankles)                                            |
|   'B'-'I' = 11-18 (shin to head)                                    |
|   'J'-'Z' = 19-35 (very tall)                                        |
|                                                                         |
|   Example human standing:                                               |
|     Row 0 (top):    'H' (head, ~17)                                  |
|     Row 1:          'F' (shoulders, ~15)                              |
|     Row 2:          'D' (waist, ~13)                                 |
|     Row 3:          'B' (hips, ~11)                                   |
|     Row 4-5:        '9' (thighs, ~9)                                 |
|     Row 6-7:        '0' (feet, ~0)                                   |
|                                                                         |
+-------------------------------------------------------------------------+

+-------------------------------------------------------------------------+
| LAYER 2: VISUAL (PRIMARY DRAWING)                                      |
+-------------------------------------------------------------------------+
|                                                                         |
| EVERY CELL contains:                                                    |
|   +-----------------------------------------------------------------+  |
|   | glyph = CP437 character (0-255)                                 |  |
|   | fg    = Foreground RGB (the "ink" color)                     |  |
|   | bk    = Background RGB (the "paper" color)                   |  |
|   +-----------------------------------------------------------------+  |
|                                                                         |
| TRANSPARENCY RULES:                                                     |
|   - If fg exactly matches Layer 0's bk color -> foreground transparent |
|   - If bk exactly matches Layer 0's bk color -> background transparent|
|   - If bk = magenta (255, 0, 255) -> fully transparent (REXPaint std)|
|                                                                         |
| SWOOSH EFFECTS (Layer 3+):                                             |
|   - If last layer has cyan fg (0,255,255) + half-block glyph         |
|     (220-223) -> swoosh effect (lightens underlying colors)           |
|   - Used for motion highlights                                         |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 1.3 Atlas Layout (Sprite Sheet Grid)

```
+-------------------------------------------------------------------------+
| ATLAS LAYOUT (How multiple frames/angles are arranged in one .xp file) |
+-------------------------------------------------------------------------+
|                                                                         |
| METADATA IN LAYER 0 determines:                                        |
|   - angles = number of view directions (1 = single, 8 = 8-way)        |
|   - projs  = 1 if angles=1, else 2 (projection + reflection halves) |
|   - anims  = list of frame counts per animation sequence              |
|                                                                         |
| TOTAL DIMENSIONS:                                                      |
|   width  = projs x sum(anims) x frame_width                          |
|   height = angles x frame_height                                      |
|                                                                         |
| EXAMPLE: Player sprite                                                  |
|   - frame_size = 7 x 10 cells                                         |
|   - angles = 8 (N, NE, E, SE, S, SW, W, NW)                       |
|   - anims = [4, 4] (idle=4 frames, walk=4 frames)                   |
|   - projs = 2 (because angles > 0)                                   |
|                                                                         |
|   -> Total size = 2 x 8 x 7 = 112 cells wide                         |
|                  x 8 = 80 cells tall                                 |
|                                                                         |
| GRID LAYOUT (16 columns x 8 rows):                                    |
|                                                                         |
|         Anim 1 (4 frames)    Anim 2 (4 frames)                        |
|         -------------------     -------------------                    |
|  Ang 0  | [F0][F1][F2][F3]  [F0][F1][F2][F3]| <- projection half   |
|  Ang 1  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 2  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 3  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 4  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 5  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 6  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|  Ang 7  | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                      |
|         +-----------------------+-----------------------+                |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]| <- reflection half    |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         | [F0][F1][F2][F3]  [F0][F1][F2][F3]|                        |
|         +-----------------------+-----------------------+                |
|              Columns 0-7              Columns 8-15                     |
|                                                                         |
| EACH [Fx] = ONE COMPLETE SPRITE FRAME (7x10 cells = 70 characters)   |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 1.4 Equipment Variations

Equipment variations are **separate .xp files** named with encoded flags:

```
+-------------------------------------------------------------------------+
| FILENAME ENCODING: {kind}-{ABCD}.xp                                    |
+-------------------------------------------------------------------------+
|                                                                         |
|   A = mount    (0=none, 1=wolf, 2=bee)                              |
|   B = armor    (0=none, 1=regular)                                   |
|   C = helmet   (0=none, 1=regular)                                   |
|   D = weapon   (0=none, 1=sword, 2=crossbow)                        |
|                                                                         |
| EXAMPLES:                                                               |
|   player-0000.xp  = human, no equipment                               |
|   player-1000.xp  = wolf mount                                        |
|   player-1100.xp  = wolf + armor                                       |
|   player-1110.xp  = wolf + armor + helmet                             |
|   player-1111.xp  = wolf + armor + helmet + sword                     |
|   player-1112.xp  = wolf + armor + helmet + crossbow                  |
|                                                                         |
| SPRITEKIND:                                                             |
|   Human = player-xxxx                                                 |
|   Wolf  = wolfie-xxxx                                                |
|   Bee   = bigbee-xxxx                                                |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 1.5 Color Palette & Quantization

```
+-------------------------------------------------------------------------+
| COLOR QUANTIZATION (RGB888 -> 216-color xterm cube)                   |
+-------------------------------------------------------------------------+
|                                                                         |
| The engine quantizes RGB888 to 6x6x6 color cube:                      |
|                                                                         |
|   level_r = (r + 25) / 51  (0-5)                                    |
|   level_g = (g + 25) / 51  (0-5)                                    |
|   level_b = (b + 25) / 51  (0-5)                                    |
|                                                                         |
|   palette_index = 16 + 36*level_r + 6*level_g + level_b              |
|                                                                         |
| SPECIAL PALETTE INDICES:                                                |
|   254 = swoosh marker                                                  |
|   255 = transparent                                                    |
|                                                                         |
| COMMON COLORS (pre-calculated):                                        |
|   +----------+-----+-----+-----+                                       |
|   | Name     | R   | G   | B   | Palette                             |
|   +----------+-----+-----+-----+                                     |
|   | Black    |  0  |  0  |  0  | 16                                  |
|   | White    |255  |255  |255  | 231                                 |
|   | Red      |255  |  0  |  0  | 196                                 |
|   | Green    |  0  |255  |  0  | 46                                  |
|   | Blue     |  0  |  0  |255  | 21                                  |
|   | Yellow   |255  |255  |  0  | 226                                 |
|   | Cyan     |  0  |255  |255  | 50                                  |
|   | Magenta  |255  |  0  |255  | 201  <- transparency key           |
|   | Gray     |128  |128  |128  | 102                                 |
|   +----------+-----+-----+-----+                                     |
|                                                                         |
+-------------------------------------------------------------------------+
```

---

# SET 2: BASE CHARACTER CREATION WORKFLOW

## 2.1 Single Frame Template (One Character Cell = One ASCII)

```
+-------------------------------------------------------------------------+
| SINGLE FRAME TEMPLATE (8x8 or variable)                               |
+-------------------------------------------------------------------------+
|                                                                         |
| Frame Size: ________ x ________ cells (typical: 7x10 for player)       |
|                                                                         |
| For EACH cell in the frame, define:                                    |
|   - CP437 glyph (what character to draw)                              |
|   - Foreground color (RGB)                                            |
|   - Background color (RGB)                                             |
|                                                                         |
|   +---+---+---+---+---+---+---+---+                                   |
| 0 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 1 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 2 |   |   | # | # |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 3 |   |   | # | # |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 4 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 5 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 6 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
| 7 |   |   |   |   |   |   |   |   |                                   |
|   +---+---+---+---+---+---+---+---+                                   |
|     0   1   2   3   4   5   6   7                                 |
|                                                                         |
| CP437 REFERENCE (common glyphs):                                       |
|   219 = # full block    220 = _ lower half    221 = | left half       |
|   222 = | right half   223 = ~ upper half    176 = . light shade    |
|   177 = : med shade    178 = ; dark shade    1-255 = all CP437 chars |
|                                                                         |
| PER-CELL FORM:                                                          |
|   Cell [_,_]: Glyph [___] FG [R__|G__|B__] BG [R__|G__|B__]         |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 2.2 Complete Sprite Sheet Template (All Angles + Animations)

```
+-------------------------------------------------------------------------+
| COMPLETE SPRITE SHEET TEMPLATE                                          |
+-------------------------------------------------------------------------+
|                                                                         |
| DEFINITIONS:                                                            |
|   Frame Size: __x__ cells                                              |
|   Angles: __ (1 or 8)                                                 |
|   Animations: __                                                        |
|   Frames per animation: __, __, __                                      |
|   Projections: __ (1 or 2)                                             |
|                                                                         |
| TOTAL GRID SIZE:                                                        |
|   Width  = proj x sum(frames) x frame_width = ______ cells            |
|   Height = angles x frame_height = ______ cells                         |
|                                                                         |
| ========================================================================= |
|                                                                         |
| LAYER 0 (Color Key + Metadata):                                        |
|                                                                         |
| Color Key: [R__|G__|B__] (typically 255,0,255 magenta)               |
|                                                                         |
| Metadata cells (fill these with digits):                                 |
|   [0,0] = __ (angle count, typically 8)                               |
|   [heightx1, 0] = __ (anim1 frames)                                   |
|   [heightx2, 0] = __ (anim2 frames)                                   |
|   [heightx3, 0] = __ (anim3 frames)                                   |
|                                                                         |
| All other cells = Color Key                                             |
|                                                                         |
| ========================================================================= |
|                                                                         |
| LAYER 1 (Height):                                                      |
|                                                                         |
| Same grid as visual. For each cell, mark height (0-35):               |
|                                                                         |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 0 |  0  |  0  |  A  |  A  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 1 |  0  |  0  |  5  |  5  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 2 |  0  |  0  |  5  |  5  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 3 |  0  |  0  |  2  |  2  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 4 |  0  |  0  |  0  |  0  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 5 |  0  |  0  |  0  |  0  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 6 |  0  |  0  |  0  |  0  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 7 |  0  |  0  |  0  |  0  |  0  |  0  |  0  |  0  |               |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
|                                                                         |
| ========================================================================= |
|                                                                         |
| LAYER 2 (Visual):                                                      |
|                                                                         |
| Draw your sprite here. Same grid structure. Each cell needs:            |
|   - Glyph (CP437)                                                      |
|   - Foreground color                                                   |
|   - Background color                                                   |
|                                                                         |
| (Use separate per-frame forms from 2.1)                                |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 2.3 Bubble Sheet (OMR) for Scanning

```
+-------------------------------------------------------------------------+
| SPRITE IDENTIFICATION BUBBLE SHEET                                      |
+-------------------------------------------------------------------------+
|                                                                         |
| SHEET ID: [X][X][X][X][X][X][X][X] (001-255)                        |
|                                                                         |
| ========================================================================= |
|                                                                         |
| SPRITE INFO:                                                            |
|                                                                         |
| Sprite Name: _________________________________________________          |
|                                                                         |
| Kind:          ( ) Human    ( ) Wolf    ( ) Bee                       |
|                                                                         |
| Frame Size:    Width [___] cells  Height [___] cells                 |
|                                                                         |
| ========================================================================= |
|                                                                         |
| ANGLES (view directions):                                              |
|   ( ) 1 (single view)                                                  |
|   ( ) 8 (8-way rotation)                                              |
|                                                                         |
| ANIMATIONS:                                                             |
|   Number of animations: [___]                                           |
|                                                                         |
|   Animation 1: [___] frames                                            |
|   Animation 2: [___] frames                                            |
|   Animation 3: [___] frames                                            |
|   Animation 4: [___] frames                                            |
|                                                                         |
| ========================================================================= |
|                                                                         |
| LAYER 0:                                                               |
|                                                                         |
| Color Key (transparency):                                               |
|   ( ) Magenta (255, 0, 255) - REXPaint standard                       |
|   ( ) Custom: R[ ][ ][ ] G[ ][ ][ ] B[ ][ ][ ]                      |
|                                                                         |
| ========================================================================= |
|                                                                         |
| FOR SCANNING:                                                          |
|                                                                         |
|   Mark registration corners:                                             |
|     +--------------------------------------------------------------+   |
|     | ##                                                      ## |   |
|     |                                                          ## |   |
|     | ##                                                      ## |   |
|     |                                                          ## |   |
|     | ##                                                      ## |   |
|     |                                                          ## |   |
|     | ##                                                      ## |   |
|     +--------------------------------------------------------------+   |
|                                                                         |
+-------------------------------------------------------------------------+
```

---

# SET 3: EQUIPMENT VARIATION WORKFLOW

## 3.1 Equipment Overlay Template

```
+-------------------------------------------------------------------------+
| EQUIPMENT VARIATION TEMPLATE                                           |
+-------------------------------------------------------------------------+
|                                                                         |
| BASE SPRITE: _________________________________________________         |
|                                                                         |
| EQUIPMENT TO ADD:                                                      |
|   Armor:    ( ) None    ( ) Regular                                   |
|   Helmet:   ( ) None    ( ) Regular                                   |
|   Shield:   ( ) None    ( ) Regular                                   |
|   Weapon:   ( ) None    ( ) Sword    ( ) Crossbow                    |
|                                                                         |
| ========================================================================= |
|                                                                         |
| INSTRUCTIONS:                                                           |
|   1. Start with base sprite (e.g., player-0000.xp)                   |
|   2. Overlay equipment additions on specific cells                     |
|   3. Only draw cells that CHANGE from base                            |
|   4. Leave unchanged areas BLANK (transparent)                        |
|                                                                         |
| ========================================================================= |
|                                                                         |
| EQUIPMENT OVERLAY GRID (only mark changed cells):                     |
|                                                                         |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 0 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 1 |     |     | [X] | [X] |     |     |     |     |  <- Helmet area  |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 2 |     |     | [X] | [X] |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 3 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 4 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 5 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 6 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
| 7 |     |     |     |     |     |     |     |     |                   |
|   +-----+-----+-----+-----+-----+-----+-----+-----+                   |
|                                                                         |
|   [X] = Mark cells that have equipment drawn on them                    |
|                                                                         |
| FOR EACH MARKED CELL:                                                   |
|   Cell [_,_]: New Glyph [___] FG [R__|G__|B__] BG [R__|G__|B__]     |
|                                                                         |
| ========================================================================= |
|                                                                         |
| OUTPUT: Save as {kind}-{ABCD}.xp where:                              |
|   A = mount (0=none, 1=wolf, 2=bee)                                 |
|   B = armor (0=none, 1=regular)                                      |
|   C = helmet (0=none, 1=regular)                                      |
|   D = weapon (0=none, 1=sword, 2=crossbow)                           |
|                                                                         |
| Example: player-1101.xp = human + armor + helmet + sword              |
|                                                                         |
+-------------------------------------------------------------------------+
```

## 3.2 Quick Equipment Reference Chart

```
+-------------------------------------------------------------------------+
| EQUIPMENT COMBINATION QUICK REFERENCE                                   |
+-------------------------------------------------------------------------+
|                                                                         |
| +---------+------+-------+--------+--------+-------------+             |
| | Sprite  | Mount| Armor | Helmet | Weapon | Filename    |             |
| +---------+------+-------+--------+--------+-------------+             |
| | Human   |  0   |   0   |   0    |   0    | player-0000 |             |
| | Human   |  0   |   0   |   0    |   1    | player-0001 |             |
| | Human   |  0   |   0   |   0    |   2    | player-0002 |             |
| | Human   |  0   |   1   |   0    |   0    | player-0100 |             |
| | Human   |  0   |   1   |   1    |   0    | player-0110 |             |
| | Human   |  0   |   1   |   1    |   1    | player-0111 |             |
| | Human   |  0   |   1   |   1    |   2    | player-0112 |             |
| | Human   |  1   |   0   |   0    |   0    | player-1000 |             |
| | Human   |  1   |   1   |   1    |   1    | player-1111 |             |
| +---------+------+-------+--------+--------+-------------+             |
| | Wolf    |  1   |   0   |   0    |   0    | wolfie-1000 |            |
| | Wolf    |  1   |   1   |   0    |   0    | wolfie-1100 |            |
| | Wolf    |  1   |   1   |   1    |   1    | wolfie-1111 |            |
| +---------+------+-------+--------+--------+-------------+             |
| | Bee     |  2   |   0   |   0    |   0    | bigbee-2000 |            |
| | Bee     |  2   |   1   |   1    |   1    | bigbee-2111 |            |
| +---------+------+-------+--------+--------+-------------+             |
|                                                                         |
| Note: This covers 32 unique equipment combinations per character type!  |
|                                                                         |
+-------------------------------------------------------------------------+
```

---

# APPENDIX: Common CP437 Glyphs Reference

```
+-------------------------------------------------------------------------+
| COMMON CP437 GLYPHS FOR SPRITE ART                                      |
+-------------------------------------------------------------------------+
|                                                                         |
| Block Characters:                                                       |
|   219 = # Full block        220 = _ Lower half block                  |
|   221 = | Left half block   222 = | Right half block                  |
|   223 = ~ Upper half block  176 = . Light shade (25%)                |
|   177 = : Medium shade (50%) 178 = ; Dark shade (75%)                |
|                                                                         |
| People/Faces:                                                          |
|   1   = :) Smiley        2   = :| Neutral     3   = <3 Heart         |
|   4   = <> Diamond        5   = %> Card       8   = BB Invisible      |
|   7   = :: Bullet         9   = o) Circle    10  = * Star            |
|                                                                         |
| Weather/Nature:                                                         |
|   11  = up arrow         12  = down arrow     16 = right arrow       |
|   24  = up double arrow  25  = down double    17 = left arrow        |
|   28  = sun             29  = full moon     30 = new moon            |
|   31  = lightning       36  = umbrella       37 = snowman            |
|                                                                         |
| Misc Symbols:                                                           |
|   15  = right arrow      17  = left arrow     30  = up triangle      |
|   31  = lightning        18  = down arrow     24  = up double arrow   |
|   25  = down double      26  = right double   27  = left double       |
|   158 = small bullet    249  = dot center     250 = dot              |
|                                                                         |
| For full reference: https://en.wikipedia.org/wiki/Code_page_437         |
|                                                                         |
+-------------------------------------------------------------------------+
```
