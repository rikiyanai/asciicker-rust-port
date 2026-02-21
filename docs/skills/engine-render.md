# Skill Pack: Engine Render Subsystem

Software rasterizer producing ASCII/CP437 terminal output. 6-stage pipeline on CPU:
Clear -> Terrain -> World -> Shadow -> Reflection -> Resolve.

**Key files:** `render.cpp` (~4400 lines), `render.h`, `sprite.cpp`, `sprite.h`, `font1.cpp`, `screen.cpp`, `rgba8.cpp`

**Cross-references:** [Render Pipeline Part 1](../arch/render_cpp_part1.md), [Render Pipeline Part 2](../arch/render_cpp_part2.md), [Engine Architecture](../../ENGINE_ARCHITECTURE.md)

---

## 1. Entrypoints

### Renderer Lifecycle

```cpp
// render.h — opaque handle pattern (full definition in render.cpp)
Renderer* CreateRenderer(uint64_t stamp);   // Allocate + init Perlin noise
void DeleteRenderer(Renderer* r);           // Free SampleBuffer + sprite queue
```

### Main Frame Render

```cpp
void Render(Renderer* r, uint64_t stamp,
    Terrain* t, World* w, float water,          // scene
    float zoom, float yaw, const float pos[3],
    const float lt[4],                          // view (lt = 3 direction + 1 ambient)
    int width, int height, AnsiCell* ptr,       // output buffer
    Inst* player,                               // hidden during render, re-shown after
    const int scene_shift[2],                   // screen shake (multiplied by 2 internally)
    bool perspective);                          // isometric vs. architectural
```

Output: fills `ptr[width*height]` with final AnsiCell grid. No return value.

### Projection / Unprojection

```cpp
bool ProjectCoords(Renderer* r, const float pos[3], int view[3]);   // world -> screen
bool UnprojectCoords2D(Renderer* r, const int xy[2], float pos[3]); // screen -> world (auto depth from buffer)
bool UnprojectCoords3D(Renderer* r, const int xyz[3], float pos[3]);// screen -> world (explicit Z plane)
```

### Picking Results (populated during Render)

```cpp
Item** GetNearbyItems(Renderer* r);       // max 9, null-terminated
Inst** GetNearbyCharacters(Renderer* r);  // max 3, null-terminated
```

### Sprite API (sprite.h)

```cpp
Sprite* LoadSprite(const char* path, const char* name,
                   const uint8_t* recolor = 0, bool detached = false);
void FreeSprite(Sprite* spr);
Sprite* LoadPlayer(const char* path);

// Linked list traversal
Sprite* GetFirstSprite(bool all=true);
Sprite* GetNextSprite(Sprite* s, bool all=true);

// Blitting (used by render resolve + font system)
void BlitSprite(AnsiCell* ptr, int w, int h, const Sprite::Frame* sf,
                int x, int y, const int clip[4]=0, bool src_clip=true, AnsiCell* bk=0);
```

### Font System (font1.cpp)

```cpp
void LoadFont1();
void Font1Paint(AnsiCell* ptr, int w, int h, int x, int y,
                const char* str, int skin = 0, bool underline = false);
void Font1Size(const char* str, int* w, int* h);
```

3 skins (grey=0, gold=1, pink=2). Y-axis inverted (sprite atlas is bottom-up).

---

## 2. Invariants & Data Contracts

### AnsiCell (Final Output Format)

```cpp
struct AnsiCell {           // render.h:37
    uint8_t fg;    // xterm-256 palette index (16-231 = 6x6x6 RGB cube)
    uint8_t bk;    // xterm-256 palette index
    uint8_t gl;    // CP437 glyph code (0-255). Index 255 = transparent
    uint8_t spare; // flags (0xFF = rendered cell during debug)
};
```

Grid: `width * height` cells, row-major. Palette: `16 + 36*r + 6*g + b` (r,g,b in 0-5).

### Material System

```cpp
struct Material {           // render.h:82
    MatCell shade[4][16];   // [elevation 0-3][diffuse/17 = 0-15]
    int mode;               // animation flags
};

struct MatCell {            // render.h:53
    uint8_t fg[3];          // RGB888 (quantized to palette in resolve)
    uint8_t gl;             // CP437 glyph
    uint8_t bg[3];          // RGB888
    uint8_t flags;          // bits: 0x03=fg_blend, 0x04=gl_mask, 0x18=bg_blend
};
```

Terrain shader indexes `shade[elevation][diffuse/17]` -> MatCell -> palette quantize at resolve.

### Sample Buffer (2x Supersampling)

```cpp
struct Sample {             // render.cpp:567
    uint16_t visual;        // material index OR RGB555 (when spare & 0x8)
    uint8_t diffuse;        // lighting 0-255
    uint8_t spare;          // bit 0-1: parity, bit 2: grid, bit 3: mesh/auto-mat, bit 6: wireframe
    float height;           // depth (-1000000 = clear)
};
```

Buffer is `(2*width+4) * (2*height+4)` samples. **Double-allocated**: upper half is cached clear state; `memcpy` clears each frame (faster than memset).

### Sprite Atlas Layout

```cpp
struct Sprite {             // sprite.h:55
    int projs;              // 1=no reflection, 2=projection+reflection
    int anims;              // 0 for still sprites
    int frames;             // 1 for still sprites
    int angles;             // e.g. 8 for octagonal
    Frame* atlas;           // indexed: atlas[frame * angles * 2 + angle * 2 + proj]
    float proj_bbox[6];     // AABB for frustum culling
};

struct Sprite::Frame {
    int width, height;
    int ref[3];             // origin (x,y in HALF-cell units for sub-cell precision)
    int meta_xy[2];         // attachment point (e.g. arrow tip)
    AnsiCell* cell;         // [width * height], spare encodes cell height
};
```

**Atlas contract**: `projs=2` when `angles > 0` (reflection always present with directional sprites).

### XP File Format (On-Disk)

Gzip container -> decompressed payload (little-endian):
- `int32 version` (skipped), `int32 num_layers`, `int32 width`, `int32 height`
- Per layer, column-major: `uint32 glyph` + `uint8[3] fg_rgb` + `uint8[3] bk_rgb` (10 bytes/cell)

Layer semantics: L0=color key/metadata, L1=height encoding, L2=primary visual, L3+=swoosh overlays.

**Minimum 3 layers required** or load fails.

### Key Constants

| Constant | Value | Contract |
|----------|-------|----------|
| `HEIGHT_SCALE` | 16 | Z-units per visual cell. Changing breaks .a3d files |
| `DBL` macro | defined | Enables 2x supersampling (always active) |
| Max items | 9 | Picking array hard limit |
| Max NPCs | 3 | Picking array hard limit |
| Palette formula | `16 + 36*r + 6*g + b` | r,g,b each 0-5: `(component + 25) / 51` |

### Global State (read by renderer)

```cpp
extern int render_break_point[2];  // debug breakpoint for resolve loop
// Player characters hidden via HideInst()/ShowInst() during render
```

---

## 3. Known Traps

### TRAP-R01: Sample.visual Overloading (Material vs RGB555)
`Sample.visual` stores **material indices** for terrain (spare bit 3 = 0) but **RGB555 direct color** for meshes (spare bit 3 = 1). Confusing these produces corrupt colors. The resolve pass branches on `spare & 0x8`.

### TRAP-R02: SampleBuffer Double-Allocation
Allocation is `2 * dw * dh * sizeof(Sample)`. Lower half = working target, upper half = cached clean state. Resizing during a frame loses the clean state — always check dimensions at frame start.

### TRAP-R03: Projection vs Reflection Palette Scaling
LoadSprite uses different quantization: projection = `(c*5+128)/255`, reflection = `(c*5+128)/400` (darker). Two divergent code paths exist (RGB2PAL vs inline). Verify which path is active when debugging color issues.

### TRAP-R04: Swoosh Merging — LAST Layer Only
Only `layer[num_layers-1]` triggers special swoosh logic (cyan fg + half-block glyphs 220-223 -> lighten). Layers 3 to N-2 are simple overwrites. Reordering layers breaks swoosh effects silently.

### TRAP-R05: Depth Test is Read-Only
`DepthTest_RO()` tests but does NOT write depth. The Shader's `Blend()` callback must write depth. A new shader that forgets this will be Z-culled by subsequent draws.

### TRAP-R06: Scene Shift Scale Factor
`scene_shift[2]` is multiplied by 2 in transform (sample-buffer space). Shift of 1 cell = 2 samples. Large values move viewport off-screen.

### TRAP-R07: Template Rasterizer Duck Typing
`Rasterize<Sample, Shader>()` requires `Shader` to have `Blend()`, `Fill()`, `Diffuse()` methods. This is compile-time duck typing — no virtual interface. Missing method = cryptic template error.

### TRAP-R08: Auto-Material LUT (32KB, static init)
`auto_mat[32*32*32*3]` maps RGB555 -> {bg palette, fg palette, dither glyph}. Computed once at first use. If triggered lazily during a frame, can cause stutter. Currently fine in practice.

### TRAP-R09: Y-Axis Inversion in Font Rendering
Sprite atlas is bottom-up (OpenGL Y), but text renders top-down. `Font1Paint` inverts with `up_row = font1_rows - 1 - row`. Editing font atlas without this in mind = upside-down text.

### TRAP-R10: spare Byte Bit Layout is Fragile
Bits 0-1 = parity, bit 2 = grid, bit 3 = mesh flag, bit 6 = wireframe. Multiple shaders write these independently. Reusing a bit field causes cross-shader visual corruption.

### TRAP-R11: Water Boundary Clamping
Meshes within `HEIGHT_SCALE/8` of water plane get special-cased. Z-coordinates exactly at the boundary may render in wrong layer. Use Z-offset of `HEIGHT_SCALE/4` for clean separation.

### TRAP-R12: Sprite Deferred Blitting Sort
Sprites are queued during world query, sorted far-to-near, blitted AFTER resolve. This is painter's algorithm — correct for semi-transparent cells but means sprite->cell writes can overwrite resolved terrain.

---

## 4. Callgraph

### Frame Pipeline (Linear Stages)

```
Render() [render.cpp:2838]
  |
  +-- HideInst(player)
  +-- SampleBuffer realloc if dimensions changed
  +-- memcpy(clean_state) ........................ [Stage 1: Clear]
  +-- View matrix + clip plane construction
  |
  +-- QueryTerrain(cb=RenderPatch) ............... [Stage 2: Terrain]
  |     +-- per visible patch:
  |           +-- Rasterize<Sample, TerrainShader>()
  |                 +-- TerrainShader.Blend() writes material+diffuse
  |
  +-- QueryWorld(cb={RenderMesh, RenderSprite}) .. [Stage 3: World]
  |     +-- RenderMesh:
  |     |     +-- QueryMesh(cb=RenderFace)
  |     |           +-- Rasterize<Sample, MeshShader>() writes RGB555+diffuse
  |     +-- RenderSprite:
  |           +-- queue SpriteRenderBuf (deferred)
  |
  +-- RenderPlayerShadow() ...................... [Stage 4: Shadow]
  |
  +-- global_refl_mode=true ..................... [Stage 5: Reflection]
  |     +-- re-run QueryTerrain + QueryWorld (below water only)
  |
  +-- 2x2 downsample loop ...................... [Stage 6: Resolve]
  |     +-- material: shade[elev][diffuse] -> MatCell -> palette quantize
  |     +-- mesh: auto_mat[rgb555] -> dither glyph + fg/bg
  |     +-- Perlin water ripple Z-offset
  |     +-- Write AnsiCell grid
  |
  +-- qsort sprites far-to-near
  +-- BlitSprite per sorted sprite (deferred blit)
  +-- ShowInst(player)
```

### Sprite Loading

```
LoadSprite(path, name)
  +-- Check sprite cache (linked list walk)
  +-- fopen -> gzip header parse -> tinfl_decompress
  +-- Validate: layers >= 3, dimensions > 0, glyphs <= 255
  +-- Swoosh merge: layers 3..N onto layer 2 (cyan fg = special)
  +-- Palette quantize: RGB888 -> xterm-256 per cell
  +-- Parse metadata: layer0[0,0].glyph = angles, layer0[col,0] = anim lengths
  +-- Allocate Sprite + Frame array
  +-- Fill atlas[frame * angles * 2 + angle * 2 + proj]
  +-- Link into global sprite list (unless detached=true)
```

### Resolve Pass (per output cell)

```
For each AnsiCell(x,y):
  +-- Read 2x2 Sample block from SampleBuffer
  +-- Average height, diffuse; collect spare flags
  +-- Branch on spare & 0x8:
  |     0 (material):
  |       +-- shade[elev][diffuse] -> MatCell -> RGB -> palette
  |     1 (mesh/RGB555):
  |       +-- auto_mat[rgb555] -> {bg, fg, dither_glyph}
  +-- Apply grid/wireframe glyphs (spare bits 2, 6)
  +-- Water: Perlin Z-perturbation if near water plane
  +-- Write AnsiCell{fg, bk, gl, spare}
```

### Palette Quantization

```
RGB2PAL(rgb[3]) -> int
  r = (rgb[0] + 25) / 51     // 0-255 -> 0-5
  g = (rgb[1] + 25) / 51
  b = (rgb[2] + 25) / 51
  return 16 + 36*r + 6*g + b  // xterm-256 indices 16-231
```

---

## 5. Bevy Mapping

### Monolithic Pipeline System

The 6-stage render pipeline is implemented as **ONE Bevy system** (`render_pipeline_system`), NOT as 6 separate systems. This is a deliberate architectural decision.

**Rationale:**
- All 6 stages write to the same `SampleBuffer` sequentially (Clear must finish before Terrain, Terrain before World, etc.)
- Splitting into 6 systems would require 6 `ResMut<SampleBuffer>` borrows — Bevy cannot schedule these in parallel anyway since they're all mutable
- Explicit `.before()`/`.after()` ordering across 6 systems is fragile and provides zero parallelism benefit
- The C++ equivalent is one function body (`Render()`) with sequential calls — the Rust port mirrors this directly
- One system = one scheduling overhead point vs six

### C++ to Bevy Mapping Table

| C++ Construct | Bevy Target | Rationale |
|---------------|-------------|-----------|
| `Renderer*` (opaque handle) | `Resource` (`RenderPipeline`) | Single instance, not per-entity |
| `SampleBuffer` (2x supersampled) | `Resource` (`SampleBuffer`) | Single buffer, `pub width: u32` field. 484x274 dimensions |
| `Sample` struct | Plain Rust struct inside `Vec<Sample>` | DOD layout preserved — flat array, sequential access |
| `AnsiCell` output grid | `Resource` (`OutputBuffer`) | Single output, copied to Bevy `Image` texture |
| `Material shade[4][16]` | `Resource` (`MaterialTable`) | Static lookup table, not per-entity |
| `auto_mat[32768*3]` LUT | `Resource` (32KB, computed once) | Static data, shared across all mesh resolve |
| `Render()` function | `render_pipeline_system` in `PostUpdate` | Monolithic system, takes `ResMut<SampleBuffer>` |
| `SpriteRenderBuf` queue | `Resource` (`SpriteQueue`) | Phase 6 writes sprite entries, pipeline reads/sorts/blits |
| `LoadSprite()` | Asset loader or startup system | Sprites loaded once, stored in `Assets<Sprite>` or `Resource` |
| `ProjectCoords` / `UnprojectCoords` | Methods on `RenderPipeline` Resource | View matrix stored in Resource, not per-entity |

### TRAP: Do NOT Split the Pipeline

A Bevy newcomer might see 6 pipeline stages and think "6 systems with ordering constraints." This is wrong:

1. There is no parallelism to exploit — stages are strictly sequential on one buffer
2. System ordering (`.before()`/`.after()`) adds complexity with zero performance benefit
3. Each system boundary adds Bevy scheduling overhead (~microseconds each, but 6x adds up at 60fps)
4. Debug breakpoints and profiling are harder across 6 system boundaries
5. The C++ code is one function body for a reason — it was already optimized this way

The pipeline runs in `PostUpdate` schedule, after all game logic has finalized positions and sprite queues.
