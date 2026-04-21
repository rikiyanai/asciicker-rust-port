> **STATUS: ACTIVE REFERENCE WITH CORRECTIONS** — Bevy render pipeline analysis. CORRECTION: TextureUsage should be TextureUsages (plural). Verify against Bevy 0.18 render APIs.

# Bevy Custom Render Pipeline Architecture Research

## Overview

Bevy uses a GPU-centric rendering architecture built on top of **wgpu**, a Rust-based cross-platform graphics API that supports WebGPU, Vulkan, Metal, DirectX, and OpenGL. The rendering system is designed around the concept of a **Render Graph** - a directed acyclic graph (DAG) of render nodes that define the rendering pipeline.

---

## 1. How to Create Custom Shaders in Bevy

### Shader Language: WGSL

Bevy uses **WGSL (WebGPU Shading Language)** as its primary shader language. GLSL is also supported via compilation to WGSL.

### Shader File Structure

Shaders are stored as `.wgsl` files in the `assets/shaders/` directory and loaded via the `AssetServer`:

```rust
const SHADER_ASSET_PATH: &str = "shaders/custom_shader.wgsl";

// Load shader
let shader_handle: Handle<Shader> = asset_server.load(SHADER_ASSET_PATH);
```

### Shader Imports

Bevy provides built-in shader imports for common functionality:

```wgsl
// Import from bevy_pbr for 3D
#import bevy_pbr::{mesh_functions, view_transformations::position_world_to_clip}

// Import from bevy_sprite for 2D
#import bevy_sprite::{mesh2d_functions, view_transformations::position_world_to_clip}

// Import fullscreen vertex shader for post-processing
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
```

### Creating Custom Materials

Materials in Bevy are created by implementing the `Material` trait:

```rust
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef};

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct CustomMaterial {
    #[uniform(0)]
    pub color: Color,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }
    
    fn blend_mode(&self) -> Option<BlendMode> {
        Some(BlendMode::Alpha)
    }
}
```

### Key Examples

- `examples/shader/shader_material.rs` - Basic custom material
- `examples/shader/shader_material_2d.rs` - 2D custom material
- `examples/shader/extended_material.rs` - Extending standard PBR material

---

## 2. How to Add Custom Render Passes

### Render Phases

Bevy uses **render phases** to organize rendering into logical groups. The main phases are:
- **Opaque** - Rendered front-to-back for efficiency
- **Transparent** - Rendered back-to-front for correct blending
- **Post-processing** - Screen-space effects

### Creating a Custom Phase

To create a custom render phase, implement `SortedRenderPhasePlugin` or `BinnedRenderPhasePlugin`:

```rust
use bevy::render::render_phase::{SortedRenderPhasePlugin, ViewSortedRenderPhases};
use bevy::render::render_resource::SpecializedMeshPipelines;
use bevy::render::RenderApp;

// Define phase item
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Stencil3d;

// In your plugin's build method:
app.add_plugins((
    ExtractComponentPlugin::<DrawStencil>::default(),
    SortedRenderPhasePlugin::<Stencil3d, MeshPipeline>::new(RenderDebugFlags::default()),
));

// Get render app and configure
let render_app = app.get_sub_app_mut(RenderApp).unwrap();
render_app.init_resource::<SpecializedMeshPipelines<StencilPipeline>>();
```

### Custom Render Pass (Post-Processing)

Create a custom post-processing effect by implementing a render graph node:

```rust
use bevy::render::render_graph::{ViewNode, ViewNodeRunner, RenderGraphApp, RenderLabel};
use bevy::render::render_resource::*;
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};

struct PostProcessNode;
impl ViewNode for PostProcessNode {
    type ViewQuery = ();
    
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        _view: (),
    ) -> Result<(), NodeRunError> {
        // Render pass logic here
        Ok(())
    }
}

// Register in plugin
render_app.add_render_node::<PostProcessNode>(PostProcessLabel);
render_app.add_render_graph_edges(Core3d, (Node3d::MainOpaquePass, PostProcessLabel));
```

### Key Examples

- `examples/shader_advanced/custom_render_phase.rs` - Custom render phase
- `examples/shader_advanced/custom_post_processing.rs` - Custom post-processing pass
- `examples/shader_advanced/custom_phase_item.rs` - Custom phase items

---

## 3. The bevy_render and bevy_core Crate Structure

### bevy_render Crate (`crates/bevy_render/src/`)

The main rendering crate with the following key modules:

| Module | Purpose |
|--------|---------|
| `lib.rs` | Main entry point, `RenderPlugin`, render system sets |
| `renderer/` | Low-level wgpu integration, device, queue |
| `render_resource/` | Pipelines, buffers, bind groups, textures |
| `render_phase/` | Phase items, draw functions, sorting |
| `extract_component.rs` | Data extraction from main world to render world |
| `extract_plugin.rs` | Extract schedule and main world |
| `mesh/` | Mesh handling and allocation |
| `texture/` | Texture management and caching |
| `view/` | View and camera handling |
| `batching/` | GPU preprocessing and batching |
| `camera.rs` | Camera extraction and handling |

### RenderPlugin Structure

```rust
pub struct RenderPlugin {
    pub render_creation: RenderCreation,
    pub synchronous_pipeline_compilation: bool,
    pub debug_flags: RenderDebugFlags,
}
```

### Render Systems (SystemSets)

The render schedule is organized into these main systems:

1. **Extract** - Extract data from main world to render world
2. **Prepare** - Prepare GPU resources (buffers, textures)
3. **Queue** - Queue entities into render phases
4. **PhaseSort** - Sort phase items
5. **PrepareBindGroups** - Create bind groups
6. **Render** - Execute the render graph
7. **Cleanup** - Clean up resources

### bevy_core_pipeline Crate (`crates/bevy_core_pipeline/`)

Note: Bevy 0.13+ uses `bevy_core_pipeline` instead of the deprecated `bevy_core`. Contains:
- Core 2D and 3D render pipelines
- Main pass definitions
- Deferred rendering support
- Fullscreen shader utilities

### Render Graph

The render graph is a DAG where:
- **Nodes** = Render passes/workloads
- **Edges** = Execution order dependencies  
- **Slots** = Input/output resources (textures, buffers)

```rust
// Render graph schedule
pub enum RenderGraphSystems {
    Begin,    // Per-frame setup
    Render,   // Main rendering
    Submit,   // Submit command buffers
    Finish,   // Per-frame finalization
}
```

---

## 4. How to Render to Texture/Target

### Creating a Render Target Texture

Use `TextureDescriptor` to create a render target:

```rust
use bevy::render::render_resource::*;
use bevy::render::texture::CachedTexture;

let texture = render_device.create_texture(&TextureDescriptor {
    label: Some("render_target"),
    size: Extent3d {
        width: 1920,
        height: 1080,
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2,
    format: TextureFormat::Bgra8Unorm,
    usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::TEXTURE_BINDING,
    view_formats: &[],
});
```

### Using ViewTarget

For rendering to a texture that can be displayed, use `ViewTarget`:

```rust
use bevy::view::ViewTarget;

fn render_to_texture(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // Create a new image asset
    let image_handle = images.add(Image::new_fill(
        Extent3d::new(512, 512),
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Bgra8Unorm,
    ));
    
    // Use in entity with a camera
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(image_handle),
        Transform::default(),
    ));
}
```

### Render to Texture in 3D

See `examples/3d/render_to_texture.rs` - demonstrates rendering a camera to a texture for mirrors/UI.

### Key Types

- `ViewTarget` - Represents what a camera renders to
- `RenderTarget` - Can be `Window` or `Image`
- `TextureAttachment` - For creating texture render attachments
- `CachedTexture` - Cached texture with automatic management

---

## 5. WGPU Integration in Bevy

### Accessing WGPU Types

Bevy re-exports wgpu types through `bevy_render::render_resource`:

```rust
use bevy::render::render_resource::{
    RenderDevice,    // Wraps wgpu::Device
    RenderQueue,     // Wraps wgpu::Queue
    PipelineCache,
    BindGroup,
    BindGroupLayout,
    Buffer,
    Texture,
};
```

### Direct WGPU Access

```rust
fn access_wgpu(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Get underlying wgpu types
    let wgpu_device = render_device.wgpu_device();
    let wgpu_queue = render_queue.wgpu_queue();
    
    // Direct wgpu operations
    let buffer = wgpu_device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("my_buffer"),
        size: 1024,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
}
```

### Pipeline Creation

```rust
use bevy::render::render_resource::*;

fn create_pipeline(
    pipeline_cache: &PipelineCache,
    shader: Handle<Shader>,
) -> CachedRenderPipelineId {
    pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("custom_pipeline".into()),
        layout: vec![bind_group_layout],
        vertex: VertexState {
            shader: shader.clone(),
            entry_point: "vertex".into(),
            buffers: vec![vertex_buffer_layout],
        },
        fragment: Some(FragmentState {
            shader,
            entry_point: "fragment".into(),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::GreaterEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
    })
}
```

### Key WGPU Integration Points

| Bevy Type | Wraps | Purpose |
|-----------|-------|---------|
| `RenderDevice` | `wgpu::Device` | GPU device access |
| `RenderQueue` | `wgpu::Queue` | Command submission |
| `PipelineCache` | N/A | Pipeline caching/specialization |
| `BindGroupLayout` | `wgpu::BindGroupLayout` | Bind group layouts |
| `RenderPipeline` | `wgpu::RenderPipeline` | Render pipelines |
| `ComputePipeline` | `wgpu::ComputePipeline` | Compute pipelines |
| `Sampler` | `wgpu::Sampler` | Texture sampling |

### Environment Variables

- `WGPU_DEBUG=1` - Enable debug labels
- `WGPU_VALIDATION=0` - Disable validation
- `WGPU_FORCE_FALLBACK_ADAPTER=1` - Force software rendering
- `WGPU_ADAPTER_NAME` - Select specific GPU
- `WGPU_SETTINGS_PRIO=webgl2` - WebGL2 limits

---

## Key Architecture Patterns

### 1. SubApp Architecture

Bevy uses **SubApps** for rendering - a separate ECS world that runs in parallel:

```
Main App (Simulation)
    ↓ Extract
Render App (Rendering)
    ↓ Execute
    GPU
```

### 2. Extraction Pattern

Data flows from main world to render world via extraction:

```rust
// Component to extract
#[derive(Component, ExtractComponent)]
struct MyComponent {
    value: f32,
}

// Extraction runs in RenderSystems::ExtractCommands
fn extract_my_component(
    mut commands: Commands,
    query: Extract<Query<&MyComponent>>,
) {
    for entity in query.iter() {
        commands.get_or_spawn(entity).insert(*query.get(entity).unwrap());
    }
}
```

### 3. Phase Item Pattern

Entities are added to render phases during the Queue system:

```rust
fn queue_my_items(
    mut opaque_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    query: Query<(Entity, &MyComponent, &Mesh3d)>,
) {
    for (entity, my_comp, mesh) in &query {
        // Add to phase
        opaque_phases.add(Opaque3dBatchSetKey {
            draw_function: draw_function_id,
            pipeline: pipeline_id,
            // ...
        }, /* ... */);
    }
}
```

---

## Relevant Examples from Bevy Repository

| Example | Path | Description |
|---------|------|-------------|
| Custom Render Phase | `examples/shader_advanced/custom_render_phase.rs` | Complete custom phase implementation |
| Post Processing | `examples/shader_advanced/custom_post_processing.rs` | Custom post-processing effect |
| Specialized Mesh Pipeline | `examples/shader_advanced/specialized_mesh_pipeline.rs` | Custom mesh pipeline |
| Custom Phase Item | `examples/shader_advanced/custom_phase_item.rs` | Custom draw commands |
| Render to Texture | `examples/3d/render_to_texture.rs` | Rendering to texture |
| Material | `examples/shader/shader_material.rs` | Basic custom material |
| Compute Shader | `examples/shader/compute_shader_game_of_life.rs` | GPU compute |

---

## Summary

Bevy's render pipeline architecture consists of:

1. **Custom Shaders**: WGSL files loaded via `AssetServer`, using `#import` for built-in functions
2. **Custom Render Passes**: Implemented via `SortedRenderPhasePlugin` or render graph nodes
3. **Crate Structure**: `bevy_render` for core rendering, `bevy_core_pipeline` for core pipelines
4. **Render to Texture**: Use `RenderTarget::Image` with `Image` assets
5. **WGPU Integration**: Direct access via `RenderDevice` and `RenderQueue` types

The architecture follows a data-driven approach where systems in the render schedule prepare and queue draw calls, which are then executed by the render graph.
