# Phase 3: GPU Output - Research

**Researched:** 2026-02-20
**Domain:** Bevy render pipeline, WGSL shaders, GPU texture management, CP437 font atlas rendering
**Confidence:** HIGH

## Summary

Phase 3 creates a Bevy render plugin that displays an `AsciiCellGrid` as colored CP437 glyphs in a window. The approach adapts the Mage Core 4-texture pattern (font atlas, char index texture, fg color texture, bg color texture) to Bevy's render architecture. The CPU rasterizer is NOT involved -- this phase uses synthetic test data to prove the GPU output path independently.

The existing `AsciiCellGrid` resource (already in `src/output/ascii_cell_grid.rs`) stores separate `char_indices`, `fg_colors`, and `bg_colors` arrays designed exactly for this 4-texture approach. The planner should structure work as: (1) font atlas loading, (2) WGSL shader + render pipeline, (3) Extract/Prepare GPU resource management, (4) window resize handling + test pattern system.

**Primary recommendation:** Use a low-level custom render node (ViewNode pattern from Bevy's `custom_post_processing` example), NOT the new `FullscreenMaterial` trait. FullscreenMaterial requires `ShaderType + Copy` which limits it to uniform-buffer data; our shader needs 4 texture bindings.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| GPU-01 | Bevy render plugin displays AsciiCellGrid using Mage Core 4-texture approach (char index, fg, bg, font atlas) | Mage Core reference implementation analyzed; 4-texture WGSL shader pattern verified; ViewNode render pipeline approach identified |
| GPU-02 | WGSL fullscreen shader composites glyphs with correct fg/bg colors | Mage Core shader.wgsl analyzed (83 lines); font lookup math verified; adaptation to Bevy's fullscreen vertex shader documented |
| GPU-03 | Font atlas loaded as Bevy PNG asset (CP437 16x16 glyph grid) | Two reference atlases found (Mage Core 10x16px/glyph, Godot 12x12px/glyph); Bevy AssetServer PNG loading verified; font dimensions passed as uniforms |
| GPU-04 | Correct Extract/Prepare/Render world pipeline with unconditional extraction | Bevy Extract/Prepare/Queue/Render stage architecture researched; ExtractResource pattern for AsciiCellGrid documented; unconditional extraction strategy defined |
| GPU-05 | Window resize handled correctly (AsciiCellGrid dimensions update) | WindowResized event + EventReader pattern documented; texture recreation on resize (from Mage Core resize()) analyzed; grid dimension recalculation approach defined |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy | 0.18.0 | Game engine, render pipeline, asset loading, windowing | Project decision D001; already pinned in Cargo.toml |
| bytemuck | 1.x | Pod/Zeroable derives for GPU uniform structs | Already in Cargo.toml; required for casting Rust structs to GPU-compatible byte layouts |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| bevy::render | (part of bevy) | Low-level render node, pipeline, bind group APIs | Building the custom render plugin |
| bevy::core_pipeline | (part of bevy) | fullscreen_vertex_shader, graph labels (Node2d/Node3d) | Fullscreen triangle vertex shader, render graph integration |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Low-level ViewNode | FullscreenMaterial (Bevy 0.18) | FullscreenMaterial requires ShaderType+Copy, cannot bind textures; our shader needs 4 texture bindings so FullscreenMaterial is NOT viable |
| Bevy Image assets for dynamic textures | Direct wgpu texture management | Bevy Image + RenderAssets<GpuImage> handles GPU upload automatically; direct wgpu violates project constraint (no direct wgpu dependency) |
| Custom render graph subgraph | Inserting into Core2d/Core3d graph | Using Core2d graph is simpler and correct for 2D fullscreen rendering |

### No Additional Dependencies Needed

The existing Cargo.toml already has everything needed. The `bevy_render` and `bevy_core_pipeline` features are already enabled. No new crate dependencies required.

## Architecture Patterns

### Recommended Module Structure
```
src/output/
  mod.rs                    # AsciiOutputPlugin (expand existing)
  ascii_cell_grid.rs        # AsciiCellGrid resource (EXISTS, keep as-is)
  gpu_plugin.rs             # AsciiGpuPlugin: render node, pipeline, bind groups
  shader.wgsl               # WGSL fragment shader (adapted from Mage Core)
  test_pattern.rs           # Synthetic test pattern system for Phase 3 validation
```

### Pattern 1: Mage Core 4-Texture Approach
**What:** Encode the ASCII grid as 3 GPU textures (char index, foreground color, background color) plus a font atlas texture. The fragment shader samples all 4 textures per screen pixel to determine the final color.
**When to use:** Always -- this is the core rendering pattern for the entire game.

**Data flow:**
```
AsciiCellGrid (Main World Resource)
  -> Extract to Render World (every frame, unconditional)
    -> Prepare: upload char_indices/fg/bg to GPU textures
      -> Render: fullscreen triangle + fragment shader
        -> Screen output
```

**Texture layout (from Mage Core reference):**
- `t_text` (char index): width x height, Rgba8Unorm, R channel = glyph index (0-255)
- `t_fore` (foreground): width x height, Rgba8Unorm, RGBA = fg color per cell
- `t_back` (background): width x height, Rgba8Unorm, RGBA = bg color per cell
- `t_font` (font atlas): (16 * glyph_w) x (16 * glyph_h), Rgba8Unorm, loaded once from PNG

### Pattern 2: ViewNode Render Plugin
**What:** A custom render node that runs in Bevy's render graph, drawing a fullscreen triangle with the ASCII shader.
**When to use:** This is the implementation pattern for GPU-01 through GPU-04.

**Key components:**
1. **AsciiGpuPlugin** -- registers extract/prepare systems and render node
2. **ExtractedAsciiGrid** -- render-world copy of AsciiCellGrid data
3. **AsciiPipeline** -- cached render pipeline + bind group layout + sampler
4. **AsciiBindGroup** -- per-frame bind group with current textures
5. **AsciiNode** -- ViewNode that draws the fullscreen triangle

### Pattern 3: Unconditional Extraction
**What:** Copy AsciiCellGrid data from Main World to Render World every frame, regardless of whether it changed.
**When to use:** Always, per GPU-04 requirement. The CPU rasterizer updates the grid every frame, so change detection is unnecessary overhead.

**Implementation:** Use `ExtractResource` derive on a wrapper, or write a manual extract system that clones the grid data into a render-world resource.

### Pattern 4: Font Atlas as Bevy Asset
**What:** Load the CP437 font atlas PNG through Bevy's AssetServer, not embedded bytes.
**When to use:** For the font atlas texture (GPU-03).

**Why AssetServer, not include_bytes!():** Bevy's asset system handles GPU upload via RenderAssets<GpuImage> automatically. The font atlas does not change, so it can use `RenderAssetUsages::RENDER_WORLD` to free the CPU copy after upload.

### Anti-Patterns to Avoid
- **Direct wgpu usage:** Project constraint says access GPU through bevy_render only. No `wgpu::Device` or `wgpu::Queue` directly.
- **FullscreenMaterial for texture-heavy rendering:** It requires ShaderType (uniform data only), not texture bindings. Use the low-level ViewNode instead.
- **Recreating the render pipeline every frame:** Cache the pipeline in a Resource. Only recreate bind groups when textures resize.
- **Extracting only when changed:** The requirement says "unconditional extraction." Even if AsciiCellGrid hasn't changed, extract it. This avoids stale-frame bugs when the CPU rasterizer skips a frame.
- **Using Rgba8UnormSrgb for data textures:** The char index, fg, and bg textures carry raw data, not display colors. Use `Rgba8Unorm` (linear) to avoid sRGB gamma correction artifacts. The font atlas should also be `Rgba8Unorm` since its pixel values are masks (0 or 1), not colors.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fullscreen triangle vertex shader | Custom vertex buffer + triangle | `bevy::core_pipeline::fullscreen_vertex_shader::FullscreenShader` | Built-in Bevy utility; generates a single triangle covering the screen with correct UVs |
| GPU texture creation from CPU data | Manual wgpu::Texture + write_texture | `Bevy Image` + `Assets<Image>` + `RenderAssets<GpuImage>` | Bevy's asset system handles format conversion, GPU upload, and lifecycle |
| Render pipeline caching | Manual HashMap of pipelines | `bevy::render::render_resource::CachedRenderPipelineId` + `PipelineCache` | Bevy's pipeline cache handles async compilation and caching |
| Font atlas PNG decoding | Manual image crate usage | `AssetServer::load::<Image>("fonts/cp437.png")` | Bevy's image loader handles PNG decoding with correct settings |
| Window size change detection | Manual size tracking | `Option<MessageReader<WindowResized>>` (Bevy 0.18 API; see P3-007/P3-H03 FIX in 03-03-PLAN.md) | Bevy's event system provides reliable resize notifications |

**Key insight:** Bevy provides all the GPU resource management primitives needed. The custom work is limited to: (1) the WGSL shader, (2) the extract/prepare systems, (3) the render node, and (4) the bind group layout.

## Common Pitfalls

### Pitfall 1: sRGB vs Linear Texture Formats
**What goes wrong:** Using `Rgba8UnormSrgb` for data textures causes the shader to read gamma-corrected values instead of raw bytes. Char indices get mangled, colors shift.
**Why it happens:** Bevy defaults to sRGB for loaded images. The font atlas and data textures need linear (`Rgba8Unorm`).
**How to avoid:** When creating `Image` objects, explicitly set `texture_descriptor.format = TextureFormat::Rgba8Unorm`. For the font atlas loaded via AssetServer, set `is_srgb: false` in the image sampler settings or use `ImageLoaderSettings`.
**Warning signs:** Colors appear washed out or too dark; glyph indices are wrong (showing wrong characters).

### Pitfall 2: Texture Size Mismatch After Resize
**What goes wrong:** Window resizes change the grid dimensions but the GPU textures retain old dimensions, causing shader to read out of bounds or display stretched content.
**Why it happens:** The bind group references old textures. Need to recreate textures AND bind group on resize.
**How to avoid:** In the prepare system, check if grid dimensions changed since last frame. If so, recreate all 3 data textures (char, fg, bg) and the bind group. The font atlas texture never changes size.
**Warning signs:** Artifacts after resize, wrong aspect ratio, panic from texture size mismatch.

### Pitfall 3: Render World Cleanup
**What goes wrong:** Entities in the render world are automatically cleaned up each frame, but Resources persist. Using entities for GPU state causes them to vanish.
**Why it happens:** Bevy's render world has a Cleanup stage that removes all entities.
**How to avoid:** Store GPU state (textures, bind groups, pipeline) as Resources in the render world, NOT as entity components.
**Warning signs:** Textures disappear after first frame, bind groups become invalid.

### Pitfall 4: Font Atlas Not Ready on First Frame
**What goes wrong:** The font atlas is loaded via AssetServer, which is async. On the first frame(s), the font GpuImage may not exist yet, causing the render node to panic or skip.
**Why it happens:** Asset loading is asynchronous; the image may not be prepared as a GpuImage until a few frames after load.
**How to avoid:** In the prepare system, check if the font atlas GpuImage exists in `RenderAssets<GpuImage>`. If not, skip bind group creation. In the render node, check if the bind group exists before drawing.
**Warning signs:** Panic on first frame, black screen for first few frames then correct rendering.

### Pitfall 5: Char Index Encoding
**What goes wrong:** The shader reads the wrong glyph because char indices are stored with incorrect encoding in the texture.
**Why it happens:** Mage Core stores char index as the R channel of an Rgba8 texture (`text.x * 255.0` in shader). Our `AsciiCellGrid` stores `u16` char_indices. Must convert u16 -> u8 and store in R channel.
**How to avoid:** When uploading char_indices to the texture, cast each u16 to u8 (CP437 is 0-255, fits in u8). Store in the R channel of Rgba8Unorm. The shader reads `textureLoad(t_text, cp, 0).x * 255.0` to recover the index.
**Warning signs:** Wrong glyphs displayed, glyphs beyond index 255 wrap around.

### Pitfall 6: Pixel-to-Cell Coordinate Math
**What goes wrong:** Fragment shader computes wrong cell coordinates, causing glyphs to be offset or repeated.
**Why it happens:** Integer division rounding or off-by-one errors in the pixel-to-character-coordinate calculation.
**How to avoid:** Follow Mage Core's exact math: `cp = vec2(i32(p.x) / i32(font_width), i32(p.y) / i32(font_height))` and `lp = vec2(i32(p.x) % i32(font_width), i32(p.y) % i32(font_height))`. The `p.x - 0.5` adjustment handles pixel center vs corner convention.
**Warning signs:** Glyphs shifted by half a character, visible seams between cells.

## Code Examples

### WGSL Shader (adapted from Mage Core)

```wgsl
// Source: Mage Core shader.wgsl, adapted for Bevy
// Bindings: group(0) = textures, group(1) = uniforms

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@group(0) @binding(0) var t_fore: texture_2d<f32>;
@group(0) @binding(1) var t_back: texture_2d<f32>;
@group(0) @binding(2) var t_text: texture_2d<f32>;
@group(0) @binding(3) var t_font: texture_2d<f32>;

struct Uniforms {
    font_width: u32,
    font_height: u32,
}
// R6-M01 NOTE: Rust-side AsciiUniforms includes `_padding: [u32; 2]` for 16-byte GPU alignment.
// WGSL does NOT need matching padding fields — it only reads the first 8 bytes (font_width + font_height).
// The padding ensures the uniform buffer meets GPU alignment requirements on the Rust side.

@group(1) @binding(0) var<uniform> uniforms: Uniforms;

@fragment
fn fragment(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<f32>(pos.x - 0.5, pos.y - 0.5);

    // Cell coordinates and local pixel within cell
    let cp = vec2(i32(p.x) / i32(uniforms.font_width), i32(p.y) / i32(uniforms.font_height));
    let lp = vec2(i32(p.x) % i32(uniforms.font_width), i32(p.y) % i32(uniforms.font_height));

    // Look up cell data
    let fore = textureLoad(t_fore, cp, 0);
    let back = textureLoad(t_back, cp, 0);
    let text = textureLoad(t_text, cp, 0);

    // Glyph index from R channel (0-255)
    let c = i32(text.r * 255.0);

    // Font atlas position (16x16 grid)
    let fx = c % 16;
    let fy = c / 16;
    let lx = fx * i32(uniforms.font_width) + lp.x;
    let ly = fy * i32(uniforms.font_height) + lp.y;

    // Sample font atlas
    let font_pixel = textureLoad(t_font, vec2<i32>(lx, ly), 0);

    // Foreground where glyph is lit, background elsewhere
    if font_pixel.r < 0.5 {
        return back;
    } else {
        return fore;
    }
}
```

### Uniform Struct (Rust side)
```rust
// Source: Mage Core render.rs RenderUniforms
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AsciiUniforms {
    font_width: u32,
    font_height: u32,
    _padding: [u32; 2],  // Align to 16 bytes for GPU
}
```

### AsciiCellGrid to GPU Texture Conversion
```rust
// Convert AsciiCellGrid arrays to Rgba8Unorm texture data
fn grid_to_char_texture(grid: &AsciiCellGrid) -> Vec<u8> {
    // Each cell -> 4 bytes (R=char_index, G=0, B=0, A=255)
    let mut data = Vec::with_capacity(grid.cells_count() * 4);
    for &idx in &grid.char_indices {
        data.push(idx as u8);  // R = glyph index
        data.push(0);          // G unused
        data.push(0);          // B unused
        data.push(255);        // A opaque
    }
    data
}

fn grid_to_color_texture(colors: &[[u8; 4]]) -> Vec<u8> {
    // Colors are already RGBA, flatten
    let mut data = Vec::with_capacity(colors.len() * 4);
    for color in colors {
        data.extend_from_slice(color);
    }
    data
}
```

### Bevy Image Creation from Raw Data
```rust
// Source: Bevy docs for Image::new
fn create_data_texture(width: u32, height: u32, data: &[u8]) -> Image {
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data.to_vec(),
        TextureFormat::Rgba8Unorm,  // NOT sRGB for data textures
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();  // No filtering for pixel data
    image
}
```

### Resize Handling Pattern
```rust
// Source: Mage Core render.rs resize(), adapted for Bevy
// R6-H01 FIX: CORRECTED to Bevy 0.18 API (was EventReader, now MessageReader wrapped in Option)
fn handle_window_resize(
    mut resize_events: Option<MessageReader<WindowResized>>,
    mut grid: ResMut<AsciiCellGrid>,
    config: Res<AsciiRenderConfig>,
    windows: Query<&Window>,
) {
    // R7-M01 FIX: Must unwrap the Option before calling .read() — Option has no .read() method.
    if let Some(mut reader) = resize_events {
        for _event in reader.read() {
            if let Ok(window) = windows.get_single() {
                let new_w = window.physical_width() / config.font_width;
                let new_h = window.physical_height() / config.font_height;
                if new_w != grid.width || new_h != grid.height {
                    // Resize the grid (allocate new arrays)
                    let cell_count = (new_w * new_h) as usize;
                    grid.width = new_w;
                    grid.height = new_h;
                    grid.char_indices = vec![0; cell_count];
                    grid.fg_colors = vec![[0, 0, 0, 255]; cell_count];
                    grid.bg_colors = vec![[0, 0, 0, 255]; cell_count];
                }
            }
        }
    }
}
```

### Test Pattern System (for Phase 3 validation)
```rust
// Generates a checkerboard of CP437 glyphs with varying colors
fn generate_test_pattern(mut grid: ResMut<AsciiCellGrid>) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let checker = (x + y) % 2 == 0;
            let glyph = if checker { 0xDB } else { 0xB1 }; // full block vs medium shade
            let fg = if checker {
                [255, 128, 0, 255]  // orange
            } else {
                [0, 255, 128, 255]  // green
            };
            let bg = if checker {
                [0, 0, 64, 255]     // dark blue
            } else {
                [64, 0, 0, 255]     // dark red
            };
            // R8-M01 FIX: AsciiCellGrid uses separate flat arrays, not a cells[] struct array.
            let idx = (y * grid.width + x) as usize;
            grid.char_indices[idx] = glyph as u16;
            grid.fg_colors[idx] = fg;
            grid.bg_colors[idx] = bg;
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Low-level custom render feature | FullscreenMaterial trait | Bevy 0.18 (Jan 2026) | Simplifies post-processing but NOT applicable to our 4-texture case |
| Manual render graph manipulation | RenderGraphApp helper methods | Bevy 0.11+ | add_render_graph_node/edges simplifies graph setup |
| Manual entity-based render state | Resource-based render state | Bevy convention | Resources persist across frames; entities are cleaned up |
| Direct wgpu texture management | Bevy Image + RenderAssets<GpuImage> | Bevy 0.8+ | Automatic GPU upload, lifecycle management |

**Why FullscreenMaterial does NOT work for this phase:**
- FullscreenMaterial requires `ShaderType + Copy + WriteInto` (uniform buffer data only)
- Our shader needs 4 `texture_2d` bindings, not uniform data
- FullscreenMaterial does not support `AsBindGroup`-style texture bindings
- The low-level ViewNode approach (same as `custom_post_processing` example) is required

## Key Design Decisions for Planner

### 1. Font Atlas Source
Use the Mage Core font1.png (160x256, 10x16 per glyph) as a starting point. It is a standard CP437 16x16 glyph grid with white-on-black rendering. The Godot project also has a 192x192 font (12x12 per glyph). Either works, but 8x8 glyphs are most common for ASCII games and produce the best grid density at 1080p (240x135 grid = 1920/8 x 1080/8). **The planner should use or create an 8x8 CP437 atlas** to match the default RenderConfig (240x135 ASCII resolution at 1080p).

**P3-004 FIX:** OVERRIDDEN — Implementation uses the 10x16 Mage Core font (cp437_10x16.png), not an 8x8 atlas. Grid resolution is font-dependent: at 1080p with 10x16 glyphs the grid is 192x67 (1920/10 x 1080/16), not 240x135. The 8x8 recommendation above was not followed.

### 2. Camera Requirement
The ViewNode pattern requires a Camera entity to function (it runs per-view). A 2D camera with no clear color (or black clear) should be spawned. The ASCII output replaces whatever the camera would normally show.

### 3. Texture Update Strategy
Three approaches exist for updating CPU data textures every frame:
1. **Bevy Image mutation** -- Use `Assets<Image>::get_mut()` to modify the Image data, Bevy re-uploads automatically. Simple but has a known issue with render targets.
2. **Extract + manual queue.write_texture** -- Extract grid data, write to GPU textures directly in prepare. More control, avoids the get_mut issue.
3. **Recreate Image handles each frame** -- Very wasteful, do not use.

**Recommendation:** Approach 2 (Extract + manual write). Create the GPU textures once in the render world. Each frame, the extract system copies grid data into a render-world resource, and the prepare system calls `write_texture` on existing GPU textures. This avoids the `get_mut` frame-skip issue and gives full control over the GPU upload path.

### 4. Render Graph Placement
[WRONG — see I-01 NOTE below] ~~Insert the ASCII render node in the `Core2d` subgraph, after `Node2d::MainPass` and before `Node2d::EndMainPass`. This runs after the 2D camera clears the screen and before UI, giving clean fullscreen control.~~

**I-01 NOTE (Round 4) — SUPERSEDED:** Implementation uses edge AFTER `Node2d::EndMainPass`, not between `MainPass` and `EndMainPass`. See P3-009 FIX in 03-02-PLAN.md for the corrected graph edge. The placement advice above was not followed at implementation time.

## Open Questions

1. **Font atlas glyph size for 1080p**
   - What we know: RenderConfig defaults to 240x135 (=1920/8 x 1080/8), implying 8x8 glyphs
   - What's unclear: The Mage Core font is 10x16 and Godot font is 12x12. Neither is 8x8.
   - Recommendation: Bundle an 8x8 CP437 font atlas PNG with the project, or compute grid dimensions from window size and font atlas dimensions (window_width/glyph_width x window_height/glyph_height) rather than hardcoding 240x135. The Mage Core approach (compute from font) is more flexible.

2. **Bevy 0.18 render graph node label API stability**
   - What we know: Bevy 0.18 uses `InternedRenderLabel` and `Node2d`/`Node3d` enums
   - What's unclear: Exact API for inserting nodes between existing graph edges may have changed from 0.17
   - Recommendation: Follow the `custom_post_processing` example pattern from Bevy 0.18 release branch; verify at implementation time.

## Sources

### Primary (HIGH confidence)
- **Mage Core source** (`/Users/r/Projects/ascii research/Mage-core/src/`) - Complete reference implementation of 4-texture ASCII GPU rendering (shader.wgsl, render.rs, image.rs, config.rs, app.rs). ~2000 lines. Directly inspected.
- **Existing AsciiCellGrid** (`engine-port/src/output/ascii_cell_grid.rs`) - Already implements the 3-array layout (char_indices, fg_colors, bg_colors) matching the 4-texture approach. Directly inspected.
- **Existing RenderConfig** (`engine-port/src/render/config.rs`) - Default 240x135 ASCII resolution, 2x supersample factor. Directly inspected.
- **Bevy 0.18 Release Notes** (https://bevy.org/news/bevy-0-18/) - FullscreenMaterial introduction, render pipeline updates.

### Secondary (MEDIUM confidence)
- **Bevy FullscreenMaterial docs** (https://docs.rs/bevy/0.18.0/bevy/core_pipeline/fullscreen_material/trait.FullscreenMaterial.html) - Trait bounds confirmed (Component + ExtractComponent + Clone + Copy + ShaderType + WriteInto + Default).
- **Bevy Render Stages** (https://bevy-cheatbook.github.io/gpu/stages.html) - Extract/Prepare/Queue/Render architecture overview.
- **Bevy RenderAssetUsages** (https://docs.rs/bevy/latest/bevy/asset/struct.RenderAssetUsages.html) - MAIN_WORLD vs RENDER_WORLD asset lifecycle.
- **Bevy WindowResized** (https://docs.rs/bevy/latest/bevy/window/struct.WindowResized.html) - Event for detecting window resize.
- **Bevy custom_post_processing example** (https://bevy.org/examples/shaders/custom-post-processing/) - ViewNode pattern documentation.

### Tertiary (LOW confidence)
- **Bevy 0.18 render graph API specifics** - Could not access raw source code for the fullscreen_material example on the release-0.18.0 branch. API details inferred from docs and release notes. Needs validation at implementation time.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Bevy 0.18 already in project, Mage Core reference fully analyzed
- Architecture: HIGH - Mage Core provides a complete working implementation of the exact pattern; Bevy render pipeline well-documented
- WGSL shader: HIGH - Mage Core shader.wgsl is 83 lines and directly applicable with minor adaptation
- Pitfalls: HIGH - Mage Core source reveals exact patterns for resize, texture format, and coordinate math
- Bevy 0.18 render graph API: MEDIUM - Could not verify exact method signatures; may differ slightly from older versions

**Research date:** 2026-02-20
**Valid until:** 2026-03-20 (stable - Bevy 0.18 is pinned, Mage Core is local reference)
