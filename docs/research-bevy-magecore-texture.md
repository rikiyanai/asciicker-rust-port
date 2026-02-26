> **STATUS: ACTIVE REFERENCE WITH CORRECTIONS** — Texture pipeline analysis is valid. CORRECTION: Lines 285/309 use textureSampleLevel on texture_2d<u32> — this is INVALID WGSL. Integer textures must use textureLoad, not textureSampleLevel. Verify shader code against actual Mage Core shaders at `/Users/r/Projects/ascii research/Mage-core/src/shader.wgsl`.

# Bevy Texture Management for Mage-Core Triple-Buffer Implementation

This document details how to implement a Mage-core style triple-buffer system in Bevy, covering texture creation, frame updates, shader bindings, and font atlas handling.

## Overview: Mage-Core vs Bevy Texture Mapping

| Mage-Core Texture | Bevy Equivalent | Purpose |
|------------------|----------------|---------|
| `fg_texture` | `Image` asset (RGBA8UnormSrgb) | Foreground color per cell |
| `bg_texture` | `Image` asset (RGBA8UnormSrgb) | Background color per cell |
| `chars_texture` | `Image` asset (R8Unorm) | Character codes (0-255) |
| `font_texture` | `FontAtlas` + `Image` | Font atlas (16x16 glyph grid) |

---

## 1. Creating and Managing Textures (Image Asset)

### Core Structure

Bevy uses the `Image` struct from `bevy_image` crate as the primary texture type:

```rust
use bevy_image::{Image, ImageSampler};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

// Key fields in Image struct
pub struct Image {
    pub data: Option<Vec<u8>>,           // Raw pixel data (CPU side)
    pub texture_descriptor: TextureDescriptor<...>,
    pub sampler: ImageSampler,
    pub asset_usage: RenderAssetUsages,
    pub copy_on_resize: bool,
}
```

### Creating Textures

**Method 1: From raw pixel data**

```rust
// Create a new RGBA texture from raw bytes
let size = Extent3d { width: 80, height: 24, depth_or_array_layers: 1 };
let data: Vec<u8> = vec![0u8; 80 * 24 * 4]; // RGBA = 4 bytes per pixel

let image = Image::new(
    size,
    TextureDimension::D2,
    data,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
);

// Add to asset system
let handle = images.add(image);
```

**Method 2: Filled texture (zero-initialized)**

```rust
// Create a zero-filled texture - useful for buffers
let image = Image::new_fill(
    Extent3d { width: 80, height: 24, depth_or_array_layers: 1 },
    TextureDimension::D2,
    &[0, 0, 0, 0],  // Zero RGBA
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
);
```

**Method 3: Render target texture**

```rust
// For textures that will be rendered to (like double-buffering)
let image = Image::new_target_texture(
    width, 
    height,
    TextureFormat::Rgba8UnormSrgb,  // Or Rgba16Float for HDR
    None,  // view_format
);

// This sets: TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT
```

### Asset Usage Flags

Critical for understanding CPU/GPU data flow:

```rust
// Main flag combinations
RenderAssetUsages::MAIN_WORLD       // Keep data on CPU (for dynamic updates)
RenderAssetUsages::RENDER_WORLD     // Extract to GPU
RenderAssetUsages::default()        // MAIN_WORLD | RENDER_WORLD
RenderAssetUsages::RENDER_WORLD     // GPU-only (no CPU access after extraction)
```

**Key insight**: For Mage-core's triple-buffer where you update from CPU each frame, use `MAIN_WORLD | RENDER_WORLD`.

### Texture Storage and Access

```rust
// In a system
fn update_buffers(
    mut images: ResMut<Assets<Image>>,
    buffer_handles: Res<BufferHandles>,  // Your resource with handles
) {
    // Get mutable access to texture data
    if let Some(image) = images.get_mut(&buffer_handles.fg_texture) {
        if let Some(ref mut data) = image.data {
            // data is Vec<u8> - directly writable
            // Format: RGBA = 4 bytes per pixel
            data[pixel_index * 4 + 0] = r;  // R
            data[pixel_index * 4 + 1] = g;  // G
            data[pixel_index * 4 + 2] = b;  // B
            data[pixel_index * 4 + 3] = 255; // A
        }
    }
}
```

---

## 2. Updating Texture Data Each Frame

### Automatic GPU Sync

Bevy's asset system automatically syncs CPU-side `Image` data to GPU. The flow is:

1. **CPU writes**: Modify `image.data` directly via `Assets<Image>::get_mut()`
2. **Automatic extraction**: Bevy's `RenderAssetPlugin<GpuImage>` extracts during render prep
3. **GPU upload**: `GpuImage::prepare_asset()` uploads to GPU via `RenderQueue::write_texture()`

### Texture Reuse Optimization

Bevy automatically reuses GPU textures when possible (same size/format):

```rust
// From gpu_image.rs - Bevy automatically:
// 1. Checks if previous GPU texture has same descriptor
// 2. Uses COPY_DST usage to allow updates
// 3. Reuses texture object instead of recreating

// Key code from GpuImage::prepare_asset:
if let Some(ref data) = image.data {
    render_queue.write_texture(
        prev.texture.as_image_copy(),
        data,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width() / block_width * block_bytes),
            rows_per_image: Some(image.height() / block_height),
        },
        image.texture_descriptor.size,
    );
}
```

### Manual GPU Write (Advanced)

For direct GPU-to-GPU transfers:

```rust
// Using RenderQueue directly (rarely needed - automatic path preferred)
fn direct_gpu_write(
    render_queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let data: Vec<u8> = /* your buffer data */;
    
    render_queue.write_texture(
        texture.as_image_copy(),
        &data,
        TexelCopyBufferLayout { /* row pitch settings */ },
        size,
    );
}
```

### TextureCache for Frame-Local Textures

For temporary textures (like ping-pong buffers):

```rust
// From texture_cache.rs
fn get_frame_texture(
    texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
) -> CachedTexture {
    let descriptor = TextureDescriptor {
        size: Extent3d { width: 80, height: 24, depth_or_array_layers: 1 },
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        ..Default::default()
    };
    
    texture_cache.get(&render_device, descriptor)
}

// TextureCache automatically:
// - Caches textures by descriptor
// - Reuses textures within 3 frames
// - Cleans up unused textures
```

---

## 3. Texture Bind Groups in Custom Shaders

### Using AsBindGroup Derive

The recommended approach is to derive `AsBindGroup`:

```rust
use bevy::{prelude::*, render::render_resource::AsBindGroup};

// Mage-core style material with 4 textures
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct AsciiMaterial {
    #[uniform(0)]
    pub font_size: Vec2,  // (char_width, char_height)
    
    #[texture(1)]
    #[sampler(2)]
    pub fg_texture: Option<Handle<Image>>,
    
    #[texture(3)]
    #[sampler(4)]
    pub bg_texture: Option<Handle<Image>>,
    
    #[texture(5)]
    #[sampler(6)]
    pub chars_texture: Option<Handle<Image>>,
    
    #[texture(7)]
    #[sampler(8)]
    pub font_texture: Option<Handle<Image>>,
}

impl Material for AsciiMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ascii_material.wgsl".into()
    }
}
```

### Binding Indices Summary

| Index | Type | Description |
|-------|------|-------------|
| 0 | uniform | font_size: vec2 |
| 1 | texture | fg_texture |
| 2 | sampler | fg_sampler |
| 3 | texture | bg_texture |
| 4 | sampler | bg_sampler |
| 5 | texture | chars_texture |
| 6 | sampler | chars_sampler |
| 7 | texture | font_texture |
| 8 | sampler | font_sampler |

### WGSL Shader Example

```wgsl
// shaders/ascii_material.wgsl

struct AsciiMaterial {
    font_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> material: AsciiMaterial;

@group(0) @binding(1)
var fg_texture: texture_2d<f32>;

@group(0) @binding(2)
var fg_sampler: sampler;

@group(0) @binding(3)
var bg_texture: texture_2d<f32>;

@group(0) @binding(4)
var bg_sampler: sampler;

@group(0) @binding(5)
var chars_texture: texture_2d<u32>;  // R8Unorm for char codes

@group(0) @binding(6)
var chars_sampler: sampler;

@group(0) @binding(7)
var font_texture: texture_2d<f32>;

@group(0) @binding(8)
var font_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate cell coordinates
    let cell_size = material.font_size;
    let cell = floor(input.uv / cell_size);
    let cell_index = cell.y * (1.0 / cell_size.x) + cell.x;  // Simplified
    
    // Sample character code
    let char_code = textureSampleLevel(chars_texture, chars_sampler, cell, 0.0).r;
    
    // Calculate font atlas position (16x16 grid)
    let char_x = f32(char_code % 16u) / 16.0;
    let char_y = f32(char_code / 16u) / 16.0;
    
    // Sample font atlas
    let font_uv = vec2<f32>(char_x, char_y) + fract(input.uv / cell_size) / 16.0;
    let font_sample = textureSampleLevel(font_texture, font_sampler, font_uv, 0.0).r;
    
    // Sample colors
    let fg = textureSampleLevel(fg_texture, fg_sampler, input.uv, 0.0);
    let bg = textureSampleLevel(bg_texture, bg_sampler, input.uv, 0.0);
    
    // Threshold for character rendering
    let ink = select(bg, fg, font_sample > 0.5);
    
    return vec4<f32>(ink.rgb, 1.0);
}
```

### Non-Derived Approach (Manual)

For more control, implement `AsBindGroup` manually:

```rust
use bevy_render::render_resource::{
    AsBindGroup, AsBindGroupError, BindGroup, BindGroupId, 
    BindGroupLayoutDescriptor, BindGroupLayoutEntries, BindingResources,
    PreparedBindGroup, RenderAssetUsages, RenderAssets, UnpreparedBindGroup,
};

impl AsBindGroup for AsciiMaterial {
    // ... see bevy source: crates/bevy_render/src/render_resource/mod.rs
}
```

---

## 4. Font Atlas Handling (bevy_text)

### FontAtlas Structure

From `bevy_text/src/font_atlas.rs`:

```rust
pub struct FontAtlas {
    pub dynamic_texture_atlas_builder: DynamicTextureAtlasBuilder,
    pub glyph_to_atlas_index: HashMap<GlyphCacheKey, GlyphAtlasLocation>,
    pub texture_atlas: TextureAtlasLayout,
    pub texture: Handle<Image>,  // The actual texture asset
}
```

### Creating a Font Atlas

```rust
use bevy_text::{FontAtlas, FontSmoothing};

fn create_font_atlas(
    textures: &mut Assets<Image>,
    size: UVec2,  // e.g., UVec2::splat(512)
    font_smoothing: FontSmoothing,
) -> FontAtlas {
    FontAtlas::new(textures, size, font_smoothing)
}

// FontAtlas::new does:
// 1. Create Image::new_fill with RGBA8UnormSrgb, filled with transparent black
// 2. Set asset_usage to MAIN_WORLD | RENDER_WORLD (for dynamic updates)
// 3. Create empty TextureAtlasLayout
// 4. Return handle to the texture asset
```

### Adding Glyphs to Atlas

```rust
use bevy_text::{GlyphCacheKey, GlyphAtlasLocation};

fn add_glyph_to_atlas(
    atlas: &mut FontAtlas,
    textures: &mut Assets<Image>,
    glyph_texture: &Image,  // Pre-rasterized glyph
    glyph_id: u16,
    offset: Vec2,
) -> Result<(), TextError> {
    atlas.add_glyph(
        textures,
        GlyphCacheKey { glyph_id },
        glyph_texture,
        offset,
    )
}

// add_glyph does:
// 1. Uses DynamicTextureAtlasBuilder to find space in atlas
// 2. Copies glyph pixels into atlas texture (modifies image.data)
// 3. Updates TextureAtlasLayout with glyph position
// 4. Stores mapping: GlyphCacheKey -> GlyphAtlasLocation
```

### DynamicTextureAtlasBuilder

From `bevy_image/src/dynamic_texture_atlas_builder.rs`:

```rust
pub struct DynamicTextureAtlasBuilder {
    atlas_allocator: AtlasAllocator,  // Guillotiere algorithm
    padding: u32,
}

impl DynamicTextureAtlasBuilder {
    pub fn new(size: UVec2, padding: u32) -> Self;
    
    pub fn add_texture(
        &mut self,
        atlas_layout: &mut TextureAtlasLayout,
        texture: &Image,
        atlas_texture: &mut Image,  // Destination - MUST have MAIN_WORLD usage
    ) -> Result<usize, DynamicTextureAtlasBuilderError>;
}
```

### FontAtlasSet for Multiple Sizes

```rust
// From font_atlas_set.rs
pub struct FontAtlasSet(HashMap<FontAtlasKey, Vec<FontAtlas>>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FontAtlasKey {
    pub id: u32,           // Font data ID
    pub index: u32,        // Font index (for font collections)
    pub font_size_bits: u32,  // Font size as f32 bits
    pub variations_hash: u64,
    pub hinting: FontHinting,
    pub font_smoothing: FontSmoothing,
}

// Different font sizes create separate FontAtlas entries
// Each atlas is a separate Image asset
```

### Loading Fonts

```rust
use bevy_text::{Font, FontLoader};

fn load_font(asset_server: &AssetServer) -> Handle<Font> {
    asset_server.load("fonts/terminus.png")  // PNG font atlas
    // Or for TTF/OTF:
    // asset_server.load("fonts/terminus.ttf")
}

// Font is also an Asset, stored in Assets<Font>
```

### Getting Glyph Info

```rust
use bevy_text::{GlyphAtlasInfo, GlyphCacheKey};

struct GlyphAtlasInfo {
    pub offset: Vec2,           // Glyph offset
    pub rect: Rect,              // Position in atlas texture
    pub texture: AssetId<Image>, // Which atlas texture
}

// To get glyph info:
fn get_glyph_info(
    font_atlases: &mut [FontAtlas],
    glyph_id: u16,
) -> Option<GlyphAtlasInfo> {
    get_glyph_atlas_info(font_atlases, GlyphCacheKey { glyph_id })
}
```

### Font Atlas as Texture for Mage-Core

For the `font_texture` in Mage-core's pipeline, you can either:

**Option A**: Use Bevy's FontAtlas directly (recommended)

```rust
fn get_font_texture_handle(font_atlas_set: &FontAtlasSet) -> Option<Handle<Image>> {
    // Get first available font atlas texture
    font_atlases
        .values()
        .next()?
        .first()
        .map(|atlas| atlas.texture.clone())
}
```

**Option B**: Create custom font atlas (Mage-core style 16x16 grid)

```rust
fn create_mage_font_atlas(
    textures: &mut Assets<Image>,
    font_data: &[u8],  // Raw RGBA pixels
    char_width: u32,
    char_height: u32,
) -> Handle<Image> {
    // Mage-core uses 16x16 = 256 character grid
    let atlas_width = char_width * 16;
    let atlas_height = char_height * 16;
    
    let mut image = Image::new_fill(
        Extent3d { width: atlas_width, height: atlas_height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        font_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    
    textures.add(image)
}
```

---

## 5. Implementation Strategy for Triple-Buffer

### Recommended Architecture

```rust
// Resource to hold all buffer handles
#[derive(Resource)]
struct AsciiBuffers {
    fg_texture: Handle<Image>,
    bg_texture: Handle<Image>,
    chars_texture: Handle<Image>,
    font_texture: Handle<Image>,
}

// Initialization system
fn setup_buffers(
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<AsciiBuffers>,
) {
    let size = (80, 24);  // Terminal dimensions
    
    // Foreground color buffer (RGBA)
    let fg = images.add(Image::new_fill(
        Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 255],  // Default white
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    
    // Background color buffer (RGBA)
    let bg = images.add(Image::new_fill(
        Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],  // Default black
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    
    // Character code buffer (R8Unorm)
    let chars = images.add(Image::new_fill(
        Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[32],  // Default space character
        TextureFormat::R8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    
    // Font atlas (loaded from PNG)
    let font = images.add(Image::from_buffer(
        /* font PNG data */,
        ImageType::Png,
        CompressedImageFormats::default(),
        true,  // is_srgb
        ImageSampler::default(),
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ).unwrap());
    
    *buffers = AsciiBuffers { fg_texture: fg, bg_texture: bg, chars_texture: chars, font_texture: font };
}

// Update system - runs each frame
fn update_buffers(
    mut images: ResMut<Assets<Image>>,
    buffers: Res<AsciiBuffers>,
    state: Res<GameState>,  // Your game state
) {
    let width = 80;
    let height = 24;
    
    // Update foreground
    if let Some(fg) = images.get_mut(&buffers.fg_texture) {
        if let Some(ref mut data) = fg.data {
            for y in 0..height {
                for x in 0..width {
                    let cell = state.get_cell(x, y);
                    let idx = (y * width + x) * 4;
                    data[idx + 0] = cell.fg_r;
                    data[idx + 1] = cell.fg_g;
                    data[idx + 2] = cell.fg_b;
                    data[idx + 3] = cell.fg_a;
                }
            }
        }
    }
    
    // Update background, chars similarly...
}

// Material setup
fn setup_material(
    mut materials: ResMut<Assets<AsciiMaterial>>,
    buffers: Res<AsciiBuffers>,
) {
    let mat = materials.add(AsciiMaterial {
        font_size: Vec2::new(12.0, 12.0),
        fg_texture: Some(buffers.fg_texture.clone()),
        bg_texture: Some(buffers.bg_texture.clone()),
        chars_texture: Some(buffers.chars_texture.clone()),
        font_texture: Some(buffers.font_texture.clone()),
    });
}
```

### Key Points

1. **Asset Usage is Critical**: Always use `MAIN_WORLD | RENDER_WORLD` for textures you update from CPU each frame
2. **Handle Mutability**: Use `ResMut<Assets<Image>>` and `get_mut()` for updating
3. **Texture Reuse**: Bevy automatically reuses GPU textures when size/format unchanged
4. **Bind Group Layout**: Use `AsBindGroup` derive for clean shader bindings
5. **Font Atlas**: Use `FontAtlas` for dynamic glyph loading, or load custom PNG for fixed glyph set

---

## Source Files Reference

| File | Purpose |
|------|---------|
| `crates/bevy_image/src/image.rs` | Image struct, creation methods |
| `crates/bevy_render/src/texture/gpu_image.rs` | GPU-side texture management |
| `crates/bevy_render/src/texture/texture_cache.rs` | Frame-cached textures |
| `crates/bevy_render/src/texture/texture_attachment.rs` | Render target attachments |
| `crates/bevy_text/src/font_atlas.rs` | Font atlas with dynamic glyphs |
| `crates/bevy_text/src/font_atlas_set.rs` | Multiple font/size management |
| `crates/bevy_image/src/dynamic_texture_atlas_builder.rs` | Atlas packing algorithm |

---

## Summary

Bevy provides a robust texture management system that can replicate Mage-core's triple-buffer approach:

1. **Create textures** using `Image::new()`, `Image::new_fill()`, or `Image::new_target_texture()`
2. **Update each frame** by modifying `image.data` via `Assets<Image>::get_mut()` - Bevy auto-syncs to GPU
3. **Bind in shaders** using `#[derive(AsBindGroup)]` with `#[texture(n)]` and `#[sampler(n)]` attributes
4. **Handle fonts** via `FontAtlas` (dynamic) or custom `Image` atlas (fixed grid)

The key difference from Mage-core: Bevy manages GPU resources automatically through its asset system, requiring less manual buffer management but offering similar performance through texture reuse.
