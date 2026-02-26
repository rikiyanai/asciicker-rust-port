# rgba8.cpp Function Analysis

Pixel format conversion library for translating between various image formats (luminance, indexed palette, RGB, RGBA) and platform-specific 32-bit packed pixel representations.

---

## Macro Expansion System (Internal Templates)

The file defines 10 parameterized macros that expand at compile time, each instantiated with specific bit-shift parameters (R, G, B, A) for platform-specific byte ordering. This compile-time template approach eliminates per-pixel dispatch overhead.

### `L_UNPACK1` (rgba8.cpp:36-71)
**Signature:** `#define L_UNPACK1(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 1-bit luminance pixels (8 pixels per byte) from packed bitfield, expand each bit to 0 or 255, replicate across R, G, B channels with full alpha.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 1 bit per pixel, packed MSB-first (bit 7 = leftmost pixel). Output: Replicated grayscale (bit shifted by R, G, B parameters). Alpha: Always 0xFF (fully opaque). Row alignment: Byte-aligned (input_row_bytes = ceil(width/8)). Algorithm: Calculate `in_row = (w+7)>>3` for byte-aligned row stride. For each row, iterate through bytes; extract bits MSB-first (bit 7 down to 0). Each bit → 255 if set, 0 if clear. Pack via: `(l<<R) | (l<<G) | (l<<B) | (0xFF<<A)` where l ∈ {0, 255}. Partial final byte: process bits 7 down to `wr` (remainder shift count).

### `L_UNPACK2` (rgba8.cpp:73-104)
**Signature:** `#define L_UNPACK2(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 2-bit luminance (4 pixels per byte), scale [0,3] to [0, 85, 170, 255].

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Intensity levels: 0×85=0, 1×85=85, 2×85=170, 3×85=255. Packing: MSB-first, 2 bits per pixel. Row alignment: Byte-aligned (input_row_bytes = ceil(width/4)). Algorithm: Similar to L_UNPACK1; extract 2-bit values from bits [7:6], [5:4], [3:2], [1:0]. Multiply by 85 to scale 4-level grayscale to 8-bit range.

### `L_UNPACK4` (rgba8.cpp:106-132)
**Signature:** `#define L_UNPACK4(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 4-bit luminance (2 pixels per byte), scale [0,15] to [0, 17, 34, ..., 255].

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Scale factor: 17 (converts 4-bit to 8-bit: 0xF × 17 = 255). Packing: MSB-first, high and low nibbles per byte. Row alignment: Byte-aligned (input_row_bytes = ceil(width/2)).

### `L_UNPACK8` (rgba8.cpp:134-144)
**Signature:** `#define L_UNPACK8(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 8-bit luminance (1 byte per pixel), direct passthrough to output.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: uint8_t grayscale values [0,255]. Processing: Single loop over w×h pixels (no per-row overhead). Output: Each RGB channel receives identical value. Simplest macro; no bit extraction required.

### `L_UNPACK16` (rgba8.cpp:146-156)
**Signature:** `#define L_UNPACK16(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 16-bit luminance (MSB only), discard LSB.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 2 bytes per pixel. Extraction: byte index 2×i (MSB). LSB discarded (byte index 2×i+1).

### `I_UNPACK1` (rgba8.cpp:158-195)
**Signature:** `#define I_UNPACK1(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 1-bit palette indices (8 indices per byte), look up RGBA quads from palette.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer; out-of-bounds palette indices produce 0

**Notes:** Index range: [0, 1]. Palette size computed as: `palsize *= 4` (convert count to byte offset). Out-of-bounds check: `l >= palsize` → return 0 (black transparent). Palette structure: RGBA quads, 4 bytes each; index i maps to bytes 4×i+0:3. Algorithm: Extract bits MSB-first: bits 7,6,5,4,3,2,1,0 → pixel indices 0–7. Multiply index by 4 to get byte offset into palette. Bounds check; if valid, lookup `pal[l+0:3]` and pack per shift parameters. Partial final byte: process bits 7 down to `wr`. 

### `I_UNPACK2` (rgba8.cpp:197-230)
**Signature:** `#define I_UNPACK2(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 2-bit indices (4 indices per byte), look up RGBA quads.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer; out-of-bounds palette indices produce 0

**Notes:** Index Range: [0, 3]. Algorithm: Extract 2-bit values from bits [7:6], [5:4], [3:2], [1:0]. Multiply by 4 for byte offset; bounds check; lookup and pack.

### `I_UNPACK4` (rgba8.cpp:232-260)
**Signature:** `#define I_UNPACK4(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 4-bit indices (2 indices per byte), look up RGBA quads.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer; out-of-bounds palette indices produce 0

**Notes:** Index Range: [0, 15]. Algorithm: Extract high nibble and low nibble from each byte. Multiply by 4; bounds check; lookup and pack.

### `I_UNPACK8` (rgba8.cpp:262-274)
**Signature:** `#define I_UNPACK8(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 8-bit indices (1 index per byte), look up RGBA quads.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer; out-of-bounds palette indices produce 0

**Notes:** Index Range: [0, 255]. Algorithm: Direct index: `l = 4 × src[i]`. Bounds check; lookup `pal[l:l+3]` and pack.

### `RGB_UNPACK8` (rgba8.cpp:276-286)
**Signature:** `#define RGB_UNPACK8(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 24-bit RGB (3 bytes per pixel), replicate across all channel shifts, set alpha to 0xFF.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input bytes: 3×i+0=R, 3×i+1=G, 3×i+2=B. Output: `(src[j+0]<<R) | (src[j+1]<<G) | (src[j+2]<<B) | (0xFF<<A)`. Alpha: Fixed 0xFF.

### `RGB_UNPACK16` (rgba8.cpp:288-298)
**Signature:** `#define RGB_UNPACK16(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 48-bit RGB (6 bytes per pixel), use MSBs only (bytes 0, 2, 4).

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input bytes: 6×i+0=R_msb, 6×i+2=G_msb, 6×i+4=B_msb. LSBs (1, 3, 5) discarded. Output: `(src[j+0]<<R) | (src[j+2]<<G) | (src[j+4]<<B) | (0xFF<<A)`. 

### `RGBA_UNPACK8` (rgba8.cpp:300-310)
**Signature:** `#define RGBA_UNPACK8(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 32-bit RGBA (4 bytes per pixel), direct passthrough.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 4×i+0=R, 4×i+1=G, 4×i+2=B, 4×i+3=A. No scaling or recombination. Output: `(src[j+0]<<R) | (src[j+1]<<G) | (src[j+2]<<B) | (src[j+3]<<A)`. 

### `RGBA_UNPACK16` (rgba8.cpp:312-322)
**Signature:** `#define RGBA_UNPACK16(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 64-bit RGBA (8 bytes per pixel), use MSBs only (bytes 0, 2, 4, 6).

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 8×i+0=R_msb, 8×i+2=G_msb, 8×i+4=B_msb, 8×i+6=A_msb. Output: `(src[j+0]<<R) | (src[j+2]<<G) | (src[j+4]<<B) | (src[j+6]<<A)`. 

### `LA_UNPACK8` (rgba8.cpp:324-336)
**Signature:** `#define LA_UNPACK8(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 16-bit luminance+alpha (2 bytes per pixel), replicate L across RGB, output A in alpha channel.

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 2×i+0=L, 2×i+1=A. Output RGB: All channels = L. Output A: Fixed at bit 24 (hardcoded shift, not parameterized). Alpha parameter `A` is unused; alpha always packed at bit 24 (line 334: `(a<<24)`).

### `LA_UNPACK16` (rgba8.cpp:338-350)
**Signature:** `#define LA_UNPACK16(w,h,data,buf,type,R,G,B,A)`

**Purpose:** Extract 32-bit luminance+alpha (4 bytes per pixel), use MSBs (bytes 0 and 2).

**Called by:** Macro expansion in `Convert_UI32_AABBGGRR()`, `Convert_UI32_AARRGGBB()`, `Convert_UL_AARRGGBB()`

**Calls:** None (macro code block)

**Globals read:** None

**Globals mutated:** None

**Side effects:** Writes pixel data to output buffer

**Notes:** Input: 4×i+0=L_msb, 4×i+2=A_msb. Output A: Fixed at bit 24 (hardcoded, line 348).

## Public Functions

### `Convert_UI32_AABBGGRR` (rgba8.cpp:358-388)

**Signature:**
```cpp
void Convert_UI32_AABBGGRR(uint32_t* buf, A3D_ImageFormat f, int w, int h,
                           const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Convert any image format to 32-bit AABBGGRR packed pixels (R at shift 0, G at 8, B at 16, A at 24).

**Called by:**
- asciiid.cpp:1269 (ImGui OpenGL texture loading)
- png2xp/png2xp.cpp:71 (PNG to XP sprite conversion)

**Calls:**
- Macro expansion: RGB_UNPACK8, RGB_UNPACK16, RGBA_UNPACK8, RGBA_UNPACK16, L_UNPACK1/2/4/8/16, LA_UNPACK8/16, I_UNPACK1/2/4/8

**Format Dispatch (switch on f):**
- RGB: A3D_RGB8, A3D_RGB16
- RGBA: A3D_RGBA8, A3D_RGBA16
- Luminance: A3D_LUMINANCE1, A3D_LUMINANCE2, A3D_LUMINANCE4, A3D_LUMINANCE8, A3D_LUMINANCE16
- Luminance+Alpha: A3D_LUMINANCE_ALPHA8, A3D_LUMINANCE_ALPHA16
- Indexed RGB: A3D_INDEX1_RGB, A3D_INDEX2_RGB, A3D_INDEX4_RGB, A3D_INDEX8_RGB
- Indexed RGBA: A3D_INDEX1_RGBA, A3D_INDEX2_RGBA, A3D_INDEX4_RGBA, A3D_INDEX8_RGBA

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Writes w×h 32-bit values to buf (caller-owned buffer)
- Out-of-bounds palette indices produce 0 (no error signal)
- No validation of palette size or buffer bounds

**Notes:**
- OpenGL format on little-endian: byte 0=R, byte 1=G, byte 2=B, byte 3=A
- Shift parameters (0,8,16,24) compiled into each macro instance at call site (line 362-386)
- All indexed format cases grouped (lines 376–386): A3D_INDEX*_RGB and A3D_INDEX*_RGBA aliases use same handler

---

### `Convert_UI32_AARRGGBB` (rgba8.cpp:393-423)

**Signature:**
```cpp
void Convert_UI32_AARRGGBB(uint32_t* buf, A3D_ImageFormat f, int w, int h,
                           const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Convert any image format to 32-bit AARRGGBB packed pixels (R at shift 16, G at 8, B at shift 0, A at 24).

**Called by:**
- mswin.cpp:1651 (Windows GDI DIB conversion)
- sdl.cpp:1155 (SDL texture loading)

**Calls:**
- Macro expansion: RGB_UNPACK8, RGB_UNPACK16, RGBA_UNPACK8, RGBA_UNPACK16, L_UNPACK1/2/4/8/16, LA_UNPACK8/16, I_UNPACK1/2/4/8

**Format Dispatch:**
Identical to Convert_UI32_AABBGGRR; only shift parameters differ (R=16, G=8, B=0, A=24).

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Writes w×h 32-bit values to buf
- Out-of-bounds palette indices produce 0

**Notes:**
- Native display format for Windows DIBs and macOS CGBitmapContext
- Byte order: byte 0=B, byte 1=G, byte 2=R, byte 3=A

---

### `Convert_UL_AARRGGBB` (rgba8.cpp:430-460)

**Signature:**
```cpp
void Convert_UL_AARRGGBB(unsigned long* buf, A3D_ImageFormat f, int w, int h,
                         const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Convert any image format to unsigned long array with AARRGGBB color (for X11 XImage compatibility on 64-bit LP64 systems).

**Called by:**
- x11.cpp:2004 (X11 XImage pixel buffer)

**Calls:**
- Macro expansion: RGB_UNPACK8, RGB_UNPACK16, RGBA_UNPACK8, RGBA_UNPACK16, L_UNPACK1/2/4/8/16, LA_UNPACK8/16, I_UNPACK1/2/4/8

**Format Dispatch:**
Identical to Convert_UI32_AARRGGBB; output type is unsigned long instead of uint32_t.

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Writes w×h unsigned long values to buf
- Stride between pixels: sizeof(unsigned long) (8 bytes on 64-bit LP64)
- Only low 32 bits used; high bits zeroed (implicit via macro)

**Notes:**
- X11 XImage requires pixel stride to match sizeof(unsigned long)
- Writing uint32_t directly into unsigned long array corrupts every other pixel on 64-bit systems
- Shift parameters (16,8,0,24) identical to native display variant

---

### `ConvertLuminance_UI32_LLZZYYXX` (rgba8.cpp:471-578)

**Signature:**
```cpp
void ConvertLuminance_UI32_LLZZYYXX(uint32_t* buf, const uint8_t rgb[3],
                                    A3D_ImageFormat f, int w, int h,
                                    const void* data, int palsize, const void* palbuf)
```

**Purpose:**
Convert any image format to luminance in high byte (bits 31-24) with constant RGB tint in low 24 bits. Used for directional lighting calculations and normal map storage where only brightness modulation needed.

**Called by:**
- asciiid.cpp:1577 (Editor luminance export with white tint)
- game_app.cpp:845 (Font texture loading for terminal renderer with white tint)

**Calls:**
- No macro expansion; explicit switch-case with inline loops per format

**Format Dispatch (switch on f):**
- A3D_RGB8, A3D_RGB16
- A3D_RGBA8, A3D_RGBA16
- A3D_LUMINANCE8
- A3D_LUMINANCE_ALPHA8
- A3D_INDEX8_RGB, A3D_INDEX8_RGBA
- Unimplemented stubs (break without body): A3D_LUMINANCE1/2/4/16, A3D_LUMINANCE_ALPHA16, A3D_INDEX1/2/4_RGB, A3D_INDEX1/2/4_RGBA

**Globals read:**
None

**Globals mutated:**
None

**Side effects:**
- Writes w×h 32-bit values to buf
- Output format: luminance in bits 31-24, RGB tint in bits 23-0
- Out-of-bounds palette indices produce 0xFFFFFF (white)

**Luminance Computation:**

For RGB/RGBA (lines 483-500, 492-509):
```cpp
(R * 2 + G * 3 + B + 3) / 6        // 8-bit
(R * 2 + G * 3 + B + 3*257) / (6*257)  // 16-bit (scale by 257 = 256+1)
```
Human vision weighting: green (3×) > red (2×) > blue (1×)

For LUMINANCE8/LUMINANCE_ALPHA8 (lines 522, 529):
```cpp
buf[i] = (src[i] << 24) | const_rgb  // Direct pass-through
```

For INDEX8_RGB/RGBA (lines 551, 573):
```cpp
const uint8_t* p = palbuf + 4*q;
buf[i] = (((p[0]*2 + p[1]*3 + p[2] + 3) / 6) << 24) | const_rgb
```

**Parameter `rgb`:**
- If NULL: `const_rgb = 0` (black tint)
- If non-NULL: `const_rgb = rgb[0] | (rgb[1]<<8) | (rgb[2]<<16)` (packed low 24 bits)

**Notes:**
- Luminance1/2/4 unsupported (stubs at lines 512–517, no conversion)
- Luminance16 unsupported (stub at line 533, no conversion)
- Luminance_Alpha16 unsupported (stub at line 532, no conversion)
- Index1/2/4_RGB/RGBA unsupported (stubs at lines 534–562, no conversion)
- INDEX8_RGBA case missing opening brace (line 563): code follows case label directly (valid C, unusual style)
- Out-of-bounds INDEX8 lookups: return 0xFFFFFF (white, line 547, 569) — differs from color convert functions (which return 0, black)

---

## Summary

**Platform-Specific Variants:**

| Function | Output Type | Byte Order | Use Case |
|----------|-------------|-----------|----------|
| Convert_UI32_AABBGGRR | uint32_t | R=0, G=8, B=16, A=24 | OpenGL on little-endian |
| Convert_UI32_AARRGGBB | uint32_t | R=16, G=8, B=0, A=24 | Windows GDI, macOS, SDL |
| Convert_UL_AARRGGBB | unsigned long | R=16, G=8, B=0, A=24 | X11 XImage (64-bit LP64) |
| ConvertLuminance_UI32_LLZZYYXX | uint32_t | L=24, RGB tint=0-23 | Lighting, normal maps |

**Supported Input Formats (20 total):**
- Luminance: 1, 2, 4, 8, 16 bits/pixel (luminance only)
- RGB: 8, 16 bits/channel (3 channels)
- RGBA: 8, 16 bits/channel (4 channels)
- Luminance+Alpha: 8, 16 bits/channel (2 channels)
- Indexed: 1, 2, 4, 8 bits/index with RGB or RGBA palette (20 entries)

**Unsupported by ConvertLuminance:**
- Luminance 1, 2, 4 bits (stubs only)
- Luminance+Alpha 16 bits (stub only)
- Indexed 1, 2, 4 bits (stubs only)
