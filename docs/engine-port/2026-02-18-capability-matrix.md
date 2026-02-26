> **STATUS: REFERENCE MATERIAL** — Written 2026-02-18 before the Bevy engine decision (D001, 2026-02-19). The Mage Core rendering analysis and ASCII architecture patterns described here remain valid as technical reference. Integration approach has been updated: Mage Core's 4-texture GPU rendering approach will be implemented within Bevy's render pipeline rather than as a standalone engine. See DECISION_LOG.md D001 for engine decision.

---
title: "Capability Matrix: Mage Core vs Asciicker C++"
type: research
status: REFERENCE
date: 2026-02-18
# blocked_by: docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md (not applicable to Rust port)
---

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

# Capability Matrix: Mage Core vs Asciicker C++

## Status: REFERENCE (originally deferred in C++ project)

---

## Rendering Approach Comparison

| Aspect | Asciicker C++ | Mage Core (Rust) |
|--------|---------------|------------------|
| **Renderer Type** | CPU software rasterizer | GPU-accelerated (WGPU) |
| **Architecture** | 6-stage pipeline (render.cpp:1-55) | 4-texture shader approach |
| **Target Output** | AnsiCell grid (fg, bk, gl) | Same (fore_image, back_image, text_image) |
| **Supersampling** | 2x SampleBuffer | None (direct cell rendering) |
| **Depth Buffer** | Per-sample height field | N/A (2D grid) |
| **Lighting** | Per-sample diffuse (0-255) | Per-cell color only |

### Asciicker Rendering Pipeline (render.cpp)

```
Stage 1 — Clear:      memcpy cached clean buffer
Stage 2 — Terrain:    QueryTerrain -> RenderPatch -> Rasterize
Stage 3 — World:      QueryWorld -> RenderMesh + RenderSprite
Stage 4 — Shadow:     Player blob shadow projection
Stage 5 — Reflection: Flip Z, re-render for water
Stage 6 — Resolve:    2x2 downsample + material lookup
```

**Evidence:** `render.cpp:1-55`

### Mage Core Rendering Pipeline

```
1. User writes to PresentInput buffers (fore, back, text)
2. CPU buffers uploaded to GPU textures
3. Fragment shader samples 4 textures
4. Output to screen
```

**Evidence:** `Mage-core/src/render.rs:330-375`, `Mage-core/README.md:28-74`

---

## Format Support

| Format | Asciicker | Mage Core | Port Complexity |
|--------|-----------|-----------|-----------------|
| **XP (Gzip REXPaint)** | Full support | None | High - need parser |
| **Font Atlas (PNG)** | Custom CP437 subset | Standard 16x16 grid | Medium - different layout |
| **AKM (Mesh)** | PLY-based geometry | None | High - new module |
| **A3D (World)** | Binary serialization | None | High - new module |

### XP Format Details (Asciicker)

**Source:** `sprite.cpp:293-332` (DATA-CONTRACT:SPRITE)

```
On-disk: gzip container (RFC 1952)
Decompressed:
  int32: version (offset 0)
  int32: num_layers (offset 4)
  int32: width (offset 8)
  int32: height (offset 12)
  Per layer:
    int32: layer_width
    int32: layer_height
    Per cell (column-major):
      uint32: glyph (4 bytes)
      uint8: fg_r, fg_g, fg_b (3 bytes)
      uint8: bk_r, bk_g, bk_b (3 bytes)
```

**Layer semantics:**
- Layer 0: Background / Color Key
- Layer 1: Glyph Data (Height/ID)
- Layer 2: Primary visual data
- Layer 3+: Swoosh overlays

### Font Atlas Details

**Asciicker:** `font1.cpp`
- Subset: 52 glyphs (A-Z, 0-9, punctuation)
- Layout: 4 rows x 13 columns
- Cell size: 5x5 pixels
- Variable-width advance (2-4 pixels)

**Mage Core:** `config.rs:65-86`
- Full: 256 glyphs in 16x16 grid
- Cell size: derived from image / 16
- Monospace

---

## Performance Characteristics

| Metric | Asciicker C++ | Mage Core (Rust) |
|--------|---------------|------------------|
| **Target FPS** | 60 | 2000+ (README:11) |
| **Characters/frame** | ~3000 | 30000 |
| **GPU Utilization** | None (CPU-only) | Full GPU |
| **Memory Model** | Manual malloc/free | Safe Rust ownership |

**Mage Core benchmark:** "30000 characters at around 2000 fps on nVidia RTX2080" (`README.md:11-12`)

**Asciicker complexity:** ~4400 lines in render.cpp, ~1200 lines in sprite.cpp

---

## Data Structure Comparison

### AnsiCell

**Asciicker** (`render.h:37-45`):
```cpp
struct AnsiCell {
    uint8_t fg;    // 256-color xterm palette index
    uint8_t bk;    // 256-color xterm palette index
    uint8_t gl;    // CP437 glyph code
    uint8_t spare; // per-cell flags
};
```

**Mage Core** (`app.rs:140-166`):
```rust
pub struct PresentInput<'textures> {
    pub width: u32,
    pub height: u32,
    pub fore_image: &'textures mut [u32],  // ABGR foreground
    pub back_image: &'textures mut [u32],  // ABGR background
    pub text_image: &'textures mut [u32],  // char in low 8 bits
}
```

**Key difference:** Asciicker uses palette indices; Mage Core uses ABGR u32.

### Sprite Frame

**Asciicker** (`sprite.h:60-67`):
```cpp
struct Frame {
    int width, height;
    int ref[3];       // origin in half-cell units
    int meta_xy[2];   // attachment point
    AnsiCell* cell;   // glyph + fg/bk palette
};
```

**Mage Core** (`image.rs:3-18`):
```rust
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub fore_image: Vec<u32>,
    pub back_image: Vec<u32>,
    pub text_image: Vec<u32>,
}
```

**Key difference:** Asciicker stores palette indices; Mage Core stores ABGR. No ref/meta in Mage Core.

---

## Missing Capabilities in Mage Core

| Capability | Asciicker Has | Mage Core Missing | Port Priority |
|------------|---------------|-------------------|---------------|
| 3D rasterization | Yes (triangles) | No | Critical |
| Depth buffer | Yes (SampleBuffer) | No | Critical |
| Material system | Yes (4x16 shade tables) | No | High |
| Sprite animation | Yes (multi-angle/frame) | No | High |
| World BSP tree | Yes | No | Medium |
| Physics/collision | Yes | No | Medium |
| Audio | Yes | No | Low |
| Networking | Yes | No | Low |

---

## Port Compatibility Assessment

### Direct Reuse (Mage Core)

| Module | Reusable? | Notes |
|--------|-----------|-------|
| `app.rs` (App trait) | Yes | Core game loop interface |
| `config.rs` | Partial | Need XP font support |
| `render.rs` | No | GPU-only, need CPU fallback |
| `image.rs` | Partial | Need palette index support |
| `present.rs` | Yes | Blit operation useful |
| `input.rs` | Yes | Shift state tracking |

### New Modules Required

1. **xp_loader.rs** - Parse gzip, extract layers, build atlas
2. **rasterizer.rs** - CPU triangle rasterizer (template port)
3. **sample_buffer.rs** - 2x supersampled render target
4. **material.rs** - Shade tables, auto-material quantization
5. **world_bsp.rs** - Spatial partitioning
6. **palette.rs** - xterm-256 color handling

---

## References

- Asciicker render.cpp: `/Users/r/Downloads/asciicker-Y9-2/render.cpp`
- Asciicker sprite.cpp: `/Users/r/Downloads/asciicker-Y9-2/sprite.cpp`
- Asciicker render.h: `/Users/r/Downloads/asciicker-Y9-2/render.h`
- Asciicker sprite.h: `/Users/r/Downloads/asciicker-Y9-2/sprite.h`
- Mage Core README: `/Users/r/Projects/ascii research/Mage-core/README.md`
- Mage Core render.rs: `/Users/r/Projects/ascii research/Mage-core/src/render.rs`
- Mage Core app.rs: `/Users/r/Projects/ascii research/Mage-core/src/app.rs`
