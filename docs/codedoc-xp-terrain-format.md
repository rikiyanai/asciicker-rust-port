# Asciicker XP/Terrain File Format Documentation

## IMPORTANT: Clarification on File Formats

**There is NO ".xp terrain format" in Asciicker.** The codebase uses two completely separate file formats:

| Asset Type | File Extension | Format | Loader Location |
|------------|---------------|--------|-----------------|
| Terrain/World | `.a3d` | Binary "AS3D" | `terrain.cpp` |
| Sprites | `.xp` | Gzip REXPaint | `sprite.cpp` |

This document covers both formats for completeness.

---

## Part 1: Terrain .a3d Format

**Source:** `/Users/r/Downloads/asciicker-Y9-2/terrain.cpp`

### 1.1 Terrain Loader Function

**Function:** `LoadTerrain(FILE* f, PatchIndex** idx)`

**Location:** `terrain.cpp` lines 3165-3266

```cpp
Terrain* LoadTerrain(FILE* f, PatchIndex** idx)
```

**Purpose:** Reads terrain from .a3d binary format file, optionally builds a patch index for editor use.

**Key Operations:**
1. Reads 16-byte FileHeader
2. Validates "AS3D" magic number
3. Creates empty terrain with `CreateTerrain()`
4. Iterates `num_patches` times, reading each FilePatch (188 bytes)
5. Calls `AddTerrainPatch()` for each patch to insert into quadtree
6. Updates visual and height maps via `UpdateTerrainVisualMap()` / `UpdateTerrainHeightMap()`

### 1.2 Header Layout (Exact Bytes)

**FileHeader struct** (16 bytes total):

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0x00 | 4 | uint32_t | `file_sign` | Magic: `"AS3D"` (0x33445341) |
| 0x04 | 4 | uint32_t | `header_size` | sizeof(FileHeader) = 16 |
| 0x08 | 4 | uint32_t | `num_patches` | Count of terrain patches |
| 0x0C | 4 | uint32_t | `reserved` | Reserved for future use |

**C Declaration** (terrain.cpp:3083-3089):
```cpp
struct FileHeader
{
    uint32_t file_sign;    // "AS3D"
    uint32_t header_size;  // 16
    uint32_t num_patches;  // patch count
    uint32_t reserved;     // 0
};
```

### 1.3 Compression Format

**No compression.** The .a3d format is plain binary little-endian data.

### 1.4 Height Data Layout

**Per-Patch Data** (FilePatch struct - 188 bytes):

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0x00 | 4 | int32_t | `x` | Patch world X coordinate |
| 0x04 | 4 | int32_t | `y` | Patch world Y coordinate |
| 0x08 | 128 | uint16_t[8][8] | `visual` | Material/visual data |
| 0x88 | 50 | uint16_t[5][5] | `height` | Vertex heights (5x5 grid) |
| 0xBA | 2 | uint16_t | `diag` | Triangle diagonal orientation |

**Constants** (terrain.h:50-66):
```cpp
#define HEIGHT_SCALE 16   // z-steps per visual cell
#define HEIGHT_CELLS 4    // vertices-1 along each axis (5x5 grid)
#define VISUAL_CELLS 8    // visual cells (8x8 grid)
```

**Height Data:** 
- 5x5 grid of uint16_t values (25 × 2 = 50 bytes)
- Each value represents vertex height in world units
- Shared between adjacent patches (vertices on boundaries)

### 1.5 Visual/Material Data Layout

**Visual Data:**
- 8x8 grid of uint16_t values (64 × 2 = 128 bytes)
- Each uint16_t encodes: 1-bit elevation flag + 6-bit material index

**Material Encoding** (from terrain.cpp comments):
```
visual[y][x] contains:
  - Bit 15: elevation flag (1 = has elevation change)
  - Bits 14-8: reserved
  - Bits 7-0: material ID (0-255)
```

**Diag Bitfield:**
- 16-bit value for 4x4 cell diagonal orientation
- Each bit controls which diagonal divides the quad into triangles
- Used for collision/rendering tessellation

---

## Part 2: Sprite .xp Format (REXPaint)

**Source:** `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp`, `/Users/r/Downloads/asciicker-Y9-2/scripts/asset_gen/xp_core.py`

### 2.1 Sprite Loader Function

**Function:** `LoadSprite(const char* path, const char* name, const uint8_t* recolor, bool detached)`

**Location:** `sprite.cpp` lines 293-1191

**Loading Pipeline** (from sprite.cpp comments lines 6-39):
1. Open .xp file (gzip-compressed REXPaint format)
2. Parse gzip header (ID1=31, ID2=139, CM=8)
3. Decompress deflate payload via `tinfl_decompress_mem_to_heap`
4. Parse decompressed header: version (int32), num_layers (int32)
5. Parse per-layer header: width (int32), height (int32)
6. Read cells in column-major order: glyph + fg RGB + bg RGB
7. Interpret layer semantics (metadata, height, visual, swoosh)
8. Apply swoosh merging for half-block glyphs
9. Quantize RGB888 colors to 216-color palette indices

### 2.2 XP Header Layout (Exact Bytes)

**File Header** (after gzip decompression):

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0x00 | 4 | int32_t | `version` (always -1 for REXPaint) |
| 0x04 | 4 | uint32_t | `num_layers` |

**Per-Layer Header** (repeated for each layer):

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0x00 | 4 | uint32_t | `width` in cells |
| 0x04 | 4 | uint32_t | `height` in cells |

> **CORRECTION:** The global XP header is 16 bytes, not 8. It contains: version (4 bytes) + num_layers (4 bytes) + width (4 bytes) + height (4 bytes). Width and height are global (apply to ALL layers), not per-layer. Per-layer sections may have additional width/height fields that the loader skips. See sprite.cpp:306-309.

### 2.3 Compression Format

**Gzip (RFC 1952)**:
- Standard gzip compression with DEFLATE interior
- Header: ID1=0x1F, ID2=0x8B, CM=0x08
- Optional mtime field (pinned to 0 for deterministic output)

### 2.4 Cell Data Layout

**Cell Format** (10 bytes per cell, column-major order):

| Offset | Size | Type | Description |
|--------|------|------|-------------|
| 0x00 | 4 | uint32_t | `glyph` (CP437 character code) |
| 0x04 | 3 | uint8_t[3] | `fg` (foreground RGB) |
| 0x07 | 3 | uint8_t[3] | `bg` (background RGB) |

**Storage Order:** Column-major (x changes fastest), then rows

### 2.5 Layer Semantics

The XP format supports multiple layers with specific meanings in Asciicker:

| Layer Index | Name | Purpose |
|-------------|------|---------|
| 0 | Metadata/Colorkey | Background transparency key |
| 1 | Height Map | Encodes cell height/ID via glyphs ('0'-'9','A'-'Z') |
| 2 | Visual | Primary sprite content (glyphs + colors) |
| 3+ | Swoosh | Overlay layers merged onto Layer 2 |

**Layer Interpretation** (from sprite.cpp lines 17-24):
- Layer 0: Background - color key for transparency
- Layer 1: Glyph data - encodes height/ID per cell
- Layer 2: Primary visual data
- Layer 3+: Swoosh overlays - half-block glyphs (220-223) with cyan foreground

---

## Summary Comparison

| Aspect | Terrain (.a3d) | Sprite (.xp) |
|--------|---------------|--------------|
| Magic/Signature | "AS3D" (0x33445341) | None (gzip container) |
| Compression | None | Gzip/DEFLATE |
| Header Size | 16 bytes | 8 bytes (+gzip overhead) |
| Data Layout | Row-major | Column-major |
| Cell Size | 2 bytes (uint16_t) | 10 bytes (glyph + 2×RGB) |
| Multi-layer | No | Yes (3-4 layers typical) |
| Coordinates | World space (x,y ints) | Grid space (layer,width,height) |

---

## Rust Port Considerations

1. **Endianness:** Both formats use little-endian. Use `byteorder` crate with `LittleEndian`.

2. **.a3d Terrain:**
   - Simple binary read, no decompression needed
   - Quadtree structure must be reconstructed from flat patch list
   - Height map is 5×5, visual is 8×8 per patch

3. **.x p Sprites:**
   - Must handle gzip decompression (use `flate2` crate)
   - Column-major cell iteration
   - Color quantization from RGB888 to palette indices for engine rendering
   - Layer semantics must be preserved for metadata extraction
