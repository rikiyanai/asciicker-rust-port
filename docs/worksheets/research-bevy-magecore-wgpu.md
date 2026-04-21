> **STATUS: ACTIVE REFERENCE** — WGPU integration analysis is valid. Note: Line 4 references "Bevy 0.19.0-dev" which conflicts with the project target of Bevy 0.18+. Use Bevy 0.18 stable APIs.

# Bevy WGPU Integration Research - Mage-Core Adaptation

Research date: 2026-02-19
Bevy version: 0.19.0-dev
WGPU version: 28

---

## Executive Summary

This document investigates how WGPU is integrated into the Bevy engine to inform adaptation of Mage-Core's WGPU rendering approach. Bevy wraps wgpu extensively with its own abstractions while providing direct access to underlying wgpu types when needed.

---

## 1. Does Bevy Use WGPU Internally?

### Yes - WGPU is Bevy's Default Renderer

Bevy uses **wgpu** (version 28) as its primary rendering backend. The integration is deep and pervasive:

- All rendering (2D, 3D, UI) flows through wgpu
- Bevy provides high-level abstractions over wgpu concepts
- Direct wgpu access is available through Bevy types

### WGPU Version

From `crates/bevy_render/Cargo.toml`:
```toml
wgpu = { version = "28", default-features = false, features = [
  "wgsl",
  "dx12",
  "metal",
  "vulkan",
  "naga-ir",
  "fragile-send-sync-non-atomic-wasm",
] }
```

---

## 2. Accessing wgpu::Device and wgpu::Queue

### Through Bevy's RenderDevice (Recommended)

Bevy provides `RenderDevice` as the primary way to access GPU functionality:

```rust
use bevy::render::renderer::RenderDevice;

// In a system:
fn my_system(render_device: Res<RenderDevice>) {
    // Get the underlying wgpu::Device
    let wgpu_device: &wgpu::Device = render_device.wgpu_device();
    
    // Create wgpu resources using the device
    let buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("My Buffer"),
        size: 1024,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
}
```

### Accessing the Queue

The `RenderQueue` resource provides access to wgpu's Queue:

```rust
use bevy::render::renderer::RenderQueue;

fn my_system(render_queue: Res<RenderQueue>) {
    // Access the underlying wgpu::Queue
    let queue: &wgpu::Queue = &*render_queue;
    
    // Write directly to buffer
    queue.write_buffer(&buffer, 0, data);
}
```

### Key Files

| File | Purpose |
|------|---------|
| `crates/bevy_render/src/renderer/render_device.rs` | RenderDevice implementation - wraps wgpu::Device |
| `crates/bevy_render/src/renderer/mod.rs` | RenderQueue, RenderAdapter, RenderInstance resources |

### Available Resources in Render App

```rust
// These resources are available in the render sub-app:
RenderDevice      // &wgpu::Device (wrapped)
RenderQueue       // &wgpu::Queue (wrapped)  
RenderAdapter     // &wgpu::Adapter (wrapped)
RenderInstance   // &wgpu::Instance (wrapped)
RenderAdapterInfo // AdapterInfo metadata
```

### Accessing from Main App

For accessing from the main app (not render sub-app), use world scope:

```rust
app.world().resource_scope(|world, render_device: Mut<RenderDevice>| {
    let device = render_device.wgpu_device();
    // use device
});
```

---

## 3. Creating Custom Textures in Bevy

### Method 1: Using Bevy's Image Asset System (Recommended)

Bevy provides the `Image` type in `bevy_image`:

```rust
use bevy_image::Image;
use wgpu::TextureFormat;

// Create a new image with raw pixel data
let image = Image::new(
    // Raw pixel data
    vec![0u8; width * height * 4],
    // Dimensions
    Extent3d { width, height, depth_or_array_layers: 1 },
    // Texture dimension (2D, 3D, Cube)
    TextureDimension::D2,
    // Format (e.g., RGBA8UnormSrgb)
    TextureFormat::Rgba8UnormSrgb,
);

// Configure as render target (no CPU data needed)
let render_target_image = Image::new(
    None, // No data - will be rendered to
    Extent3d { width: 512, height: 512, depth_or_array_layers: 1 },
    TextureDimension::D2,
    TextureFormat::Rgba8Unorm,
);
// Important: Add RENDER_ATTACHMENT usage
render_target_image.texture_descriptor.usage = wgpu::TextureUsages::RENDER_ATTACHMENT 
    | wgpu::TextureUsages::TEXTURE_BINDING;
```

### Method 2: Direct Texture Creation via RenderDevice

For low-level control:

```rust
fn create_custom_texture(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    width: u32,
    height: u32,
) -> Texture {
    let descriptor = wgpu::TextureDescriptor {
        label: Some("Custom Texture"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING 
            | wgpu::TextureUsages::COPY_DST 
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };
    
    render_device.create_texture(&descriptor)
}
```

### Creating Texture with Initial Data

```rust
fn create_texture_with_data(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    data: &[u8],
    width: u32,
    height: u32,
) -> Texture {
    let descriptor = wgpu::TextureDescriptor {
        label: Some("Data Texture"),
        size: Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };
    
    render_device.create_texture_with_data(
        &render_queue,
        &descriptor,
        wgpu::util::TextureDataOrder::default(),
        data,
    )
}
```

### Key Files

| File | Purpose |
|------|---------|
| `crates/bevy_render/src/texture/gpu_image.rs` | GpuImage - GPU representation of Image |
| `crates/bevy_image/src/image.rs` | Image struct definition |
| `crates/bevy_render/src/texture/mod.rs` | Texture plugin and cache |

---

## 4. Render Target Management in Bevy

### Understanding View and Target System

Bevy manages render targets through the `ViewTarget` system:

```rust
use bevy::render::view::ViewTarget;
use bevy::camera::RenderTarget;

// A camera renders to a ViewTarget
// The ViewTarget contains:
// - Main texture (what gets displayed)
// - Depth texture (for depth buffering)
// - Resolve texture (for MSAA)
```

### Creating a Render-to-Texture (Framebuffer)

Bevy uses `Image` assets as render targets:

```rust
fn setup_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    // Create a new image to render into
    let render_target = images.add(Image::new(
        vec![],
        Extent3d { width: 512, height: 512, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::Rgba8Unorm,
    ));
    
    // Configure for rendering
    let image = images.get_mut(&render_target).unwrap();
    image.texture_descriptor.usage = wgpu::TextureUsages::RENDER_ATTACHMENT 
        | wgpu::TextureUsages::TEXTURE_BINDING 
        | wgpu::TextureUsages::COPY_DST;
    
    // Use as render target via camera
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(render_target),
    ));
}
```

### Color and Depth Attachments

From `crates/bevy_render/src/texture/texture_attachment.rs`:

```rust
// ColorAttachment for render targets
pub struct ColorAttachment {
    pub texture: CachedTexture,
    pub resolve_target: Option<CachedTexture>,
    pub previous_frame_texture: Option<CachedTexture>,
    // ...
}

// DepthAttachment for depth buffers
pub struct DepthAttachment {
    pub view: TextureView,
    clear_value: Option<f32>,
    // ...
}
```

### Using as Input Texture

Once rendered, use the texture as a sampled texture in materials:

```rust
// The rendered Image asset can be used directly
let texture_handle: Handle<Image> = /* your render target */;

// Use in a material
material.base_color_texture = Some(texture_handle);
```

---

## 5. bevy_wgpu_utils Crate

### Does NOT Exist

**There is no `bevy_wgpu_utils` crate.** Bevy does not provide a separate utility crate for wgpu helpers.

Instead, Bevy integrates wgpu utilities directly:
- Uses `wgpu::util::*` for buffer/texture helpers
- Provides its own abstractions in `bevy_render`

---

## 6. Mage-Core Adaptation Recommendations

### Mapping Mage-Core Patterns to Bevy

| Mage-Core Concept | Bevy Equivalent |
|------------------|----------------|
| `Device` | `RenderDevice::wgpu_device()` |
| `Queue` | `RenderQueue` |
| Custom texture creation | `RenderDevice::create_texture()` |
| Render targets | `Image` with `RENDER_ATTACHMENT` usage |
| Texture views | `TextureView` from `Texture::create_view()` |
| Pipeline creation | `RenderDevice::create_render_pipeline()` |

### Integration Strategy

1. **Access WGPU resources through Bevy's ECS**
   ```rust
   // Get device/queue in a render system
   fn process(
       render_device: Res<RenderDevice>,
       render_queue: Res<RenderQueue>,
   ) {
       // Your wgpu code here
   }
   ```

2. **Create custom textures via RenderDevice**
   ```rust
   let texture = render_device.create_texture(&descriptor);
   let view = texture.create_view(&view_descriptor);
   ```

3. **Use as Bevy assets when possible**
   - For textures that need to be sampled: use `Image` assets
   - For internal render targets: use raw `Texture`/`TextureView`

4. **Pipeline management**
   - Create pipelines through `RenderDevice`
   - Store in Bevy's `PipelineCache` for async compilation

### Example: Adapting Mage-Core's 4-Texture Approach

```rust
fn setup_ascii_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut images: ResMut<Assets<Image>>,
) {
    // Create 4 texture slots (like Mage-Core)
    let texture_formats = [
        TextureFormat::Rgba8Unorm,
        TextureFormat::Rg16Float,
        TextureFormat::R32Float,
        TextureFormat::Rg32Float,
    ];
    
    let mut texture_views = Vec::new();
    
    for (i, format) in texture_formats.iter().enumerate() {
        // Create render target texture
        let desc = wgpu::TextureDescriptor {
            label: Some(format!("ascii_slot_{}", i)),
            size: Extent3d { width: 1920, height: 1080, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: *format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT 
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        
        let texture = render_device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        texture_views.push(view);
    }
    
    // Store for later use
    commands.insert_resource(AsciiTextureSlots(texture_views));
}
```

---

## 7. Key Bevy Files for WGPU Integration

| File | Description |
|------|-------------|
| `crates/bevy_render/src/renderer/render_device.rs` | Main device abstraction |
| `crates/bevy_render/src/renderer/mod.rs` | Renderer initialization, Queue, Instance |
| `crates/bevy_render/src/texture/gpu_image.rs` | GPU texture representation |
| `crates/bevy_render/src/texture/texture_attachment.rs` | Render pass attachments |
| `crates/bevy_render/src/render_resource/mod.rs` | Bevy's render resource types |
| `crates/bevy_image/src/image.rs` | CPU-side Image type |

---

## References

- Bevy Render Crate: `crates/bevy_render/`
- WGPU Crate: https://github.com/gfx-rs/wgpu
- Bevy Render to Texture Example: https://bevy.org/examples/3d-rendering/render-to-texture/
- Bevy WgpuSettings: https://docs.rs/bevy/latest/bevy/render/settings/struct.WgpuSettings.html
