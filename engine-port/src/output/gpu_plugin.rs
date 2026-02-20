//! Bevy render pipeline for ASCII grid GPU output.
//!
//! Implements the Mage Core 4-texture approach using a custom ViewNode:
//! - Extract: copies AsciiCellGrid data from Main World to Render World
//! - Prepare: uploads grid data to GPU textures, creates bind groups
//! - Render: draws a fullscreen triangle with the ASCII compositing shader

use bevy::asset::{AssetServer, embedded_asset, load_embedded_asset};
use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_graph::RenderGraphExt;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderStartup, RenderSystems,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner},
    render_resource::{
        BindGroup, BindGroupEntries, BindGroupEntry, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, Buffer, BufferInitDescriptor, BufferUsages, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FragmentState, PipelineCache,
        RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, TexelCopyBufferLayout,
        Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
        TextureUsages, TextureView, TextureViewDescriptor,
        binding_types::{texture_2d, uniform_buffer_sized},
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
    view::ViewTarget,
};
use bevy::shader::Shader;

use super::ascii_cell_grid::AsciiCellGrid;
use super::gpu_types::{AsciiRenderConfig, AsciiUniforms, ExtractedAsciiGrid, extract_grid_data};

/// Render graph label for the ASCII output node.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct AsciiNodeLabel;

/// Plugin that registers the GPU render pipeline for ASCII grid display.
///
/// Adds extract/prepare systems and a ViewNode to the RenderApp that
/// draws AsciiCellGrid data as colored CP437 glyphs using a fullscreen shader.
pub struct AsciiGpuPlugin;

impl Plugin for AsciiGpuPlugin {
    fn build(&self, app: &mut App) {
        // Guard: only register GPU pipeline when RenderApp and AssetPlugin exist.
        // MinimalPlugins (used in tests) have neither, so we skip gracefully.
        if app.get_sub_app(RenderApp).is_none() {
            return;
        }

        // Embed the WGSL shader so it can be loaded from the render app.
        embedded_asset!(app, "shader.wgsl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<AsciiGpuTextures>()
            .add_systems(ExtractSchedule, extract_ascii_grid)
            .add_systems(
                Render,
                prepare_ascii_textures.in_set(RenderSystems::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<AsciiNode>>(Core2d, AsciiNodeLabel)
            .add_render_graph_edge(Core2d, Node2d::EndMainPass, AsciiNodeLabel);

        render_app.add_systems(RenderStartup, init_ascii_pipeline);
    }
}

// ---------------------------------------------------------------------------
// Pipeline resource (created once in RenderStartup)
// ---------------------------------------------------------------------------

/// Cached render pipeline and bind group layout descriptors for the ASCII shader.
#[derive(Resource)]
struct AsciiPipeline {
    pipeline_id: CachedRenderPipelineId,
    texture_layout: BindGroupLayoutDescriptor,
    uniform_layout: BindGroupLayoutDescriptor,
}

/// System that initializes the AsciiPipeline resource in the render world.
fn init_ascii_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let texture_layout = BindGroupLayoutDescriptor::new(
        "ascii_texture_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // binding 0: t_fore (foreground color per cell)
                texture_2d(TextureSampleType::Float { filterable: false }),
                // binding 1: t_back (background color per cell)
                texture_2d(TextureSampleType::Float { filterable: false }),
                // binding 2: t_text (character index per cell)
                texture_2d(TextureSampleType::Float { filterable: false }),
                // binding 3: t_font (CP437 font atlas)
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        ),
    );

    let uniform_layout = BindGroupLayoutDescriptor::new(
        "ascii_uniform_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (uniform_buffer_sized(
                false,
                Some(std::num::NonZero::new(16).unwrap()),
            ),),
        ),
    );

    let shader_handle: Handle<Shader> = load_embedded_asset!(asset_server.as_ref(), "shader.wgsl");

    let pipeline_descriptor = RenderPipelineDescriptor {
        label: Some("ascii_render_pipeline".into()),
        layout: vec![texture_layout.clone(), uniform_layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader_handle,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..Default::default()
        }),
        ..Default::default()
    };

    let pipeline_id = pipeline_cache.queue_render_pipeline(pipeline_descriptor);

    commands.insert_resource(AsciiPipeline {
        pipeline_id,
        texture_layout,
        uniform_layout,
    });
}

// ---------------------------------------------------------------------------
// Extract system
// ---------------------------------------------------------------------------

/// Extracts AsciiCellGrid and AsciiRenderConfig from Main World into
/// an ExtractedAsciiGrid resource in the Render World.
///
/// Runs unconditionally every frame (GPU-04 requirement).
fn extract_ascii_grid(
    grid: Extract<Res<AsciiCellGrid>>,
    config: Extract<Res<AsciiRenderConfig>>,
    mut commands: Commands,
) {
    let extracted = extract_grid_data(&grid, &config);
    commands.insert_resource(extracted);
    // Also pass the font atlas handle so prepare can look it up.
    commands.insert_resource(ExtractedFontAtlasHandle(config.font_atlas_handle.clone()));
}

/// Render-world resource carrying the font atlas handle from the main world.
#[derive(Resource)]
struct ExtractedFontAtlasHandle(Handle<Image>);

// ---------------------------------------------------------------------------
// GPU textures resource (persists across frames in render world)
// ---------------------------------------------------------------------------

/// Render-world resource holding GPU textures, uniform buffer, and bind groups.
///
/// Stored as a Resource (not an entity) so it persists across frames
/// despite Bevy's render-world entity cleanup.
#[derive(Resource, Default)]
struct AsciiGpuTextures {
    /// GPU texture for foreground colors (Rgba8Unorm).
    fore_texture: Option<Texture>,
    /// GPU texture for background colors (Rgba8Unorm).
    back_texture: Option<Texture>,
    /// GPU texture for character indices (Rgba8Unorm, R channel = glyph index).
    text_texture: Option<Texture>,
    /// Persistent TextureView for foreground texture (AUDIT-01: outlives BindGroup).
    fore_view: Option<TextureView>,
    /// Persistent TextureView for background texture (AUDIT-01: outlives BindGroup).
    back_view: Option<TextureView>,
    /// Persistent TextureView for text/glyph texture (AUDIT-01: outlives BindGroup).
    text_view: Option<TextureView>,
    /// Uniform buffer for AsciiUniforms.
    uniform_buffer: Option<Buffer>,
    /// Bind group for textures (group 0).
    texture_bind_group: Option<BindGroup>,
    /// Bind group for uniforms (group 1).
    uniform_bind_group: Option<BindGroup>,
    /// Last known grid width (to detect resize).
    last_width: u32,
    /// Last known grid height (to detect resize).
    last_height: u32,
}

// ---------------------------------------------------------------------------
// Prepare system
// ---------------------------------------------------------------------------

/// Creates/updates GPU textures from ExtractedAsciiGrid and builds bind groups.
///
/// Skips if the font atlas GpuImage is not ready yet (graceful first-frame handling).
#[allow(clippy::too_many_arguments)]
fn prepare_ascii_textures(
    extracted: Option<Res<ExtractedAsciiGrid>>,
    font_handle: Option<Res<ExtractedFontAtlasHandle>>,
    pipeline: Option<Res<AsciiPipeline>>,
    mut textures: ResMut<AsciiGpuTextures>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    pipeline_cache: Res<PipelineCache>,
) {
    let (Some(extracted), Some(font_handle), Some(pipeline)) =
        (extracted.as_ref(), font_handle.as_ref(), pipeline.as_ref())
    else {
        return;
    };

    // Check if font atlas GpuImage is ready (Pitfall 4: async loading).
    let Some(font_gpu_image) = gpu_images.get(&font_handle.0) else {
        warn!("ASCII GPU: font atlas not ready, skipping frame");
        return;
    };

    let width = extracted.width;
    let height = extracted.height;

    // Recreate data textures if grid dimensions changed.
    if width != textures.last_width || height != textures.last_height {
        let desc = TextureDescriptor {
            label: Some("ascii_data_texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let fore_tex = render_device.create_texture(&desc);
        let back_tex = render_device.create_texture(&desc);
        let text_tex = render_device.create_texture(&desc);
        let view_desc = TextureViewDescriptor::default();
        textures.fore_view = Some(fore_tex.create_view(&view_desc));
        textures.back_view = Some(back_tex.create_view(&view_desc));
        textures.text_view = Some(text_tex.create_view(&view_desc));
        textures.fore_texture = Some(fore_tex);
        textures.back_texture = Some(back_tex);
        textures.text_texture = Some(text_tex);
        textures.last_width = width;
        textures.last_height = height;
    }

    // Upload texture data via write_texture.
    let bytes_per_row = width * 4;
    let image_data_layout = TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: None,
    };
    let tex_size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    if let Some(ref fore_tex) = textures.fore_texture {
        render_queue.write_texture(
            fore_tex.as_image_copy(),
            &extracted.fg_data,
            image_data_layout,
            tex_size,
        );
    }
    if let Some(ref back_tex) = textures.back_texture {
        render_queue.write_texture(
            back_tex.as_image_copy(),
            &extracted.bg_data,
            image_data_layout,
            tex_size,
        );
    }
    if let Some(ref text_tex) = textures.text_texture {
        render_queue.write_texture(
            text_tex.as_image_copy(),
            &extracted.char_data,
            image_data_layout,
            tex_size,
        );
    }

    // Create/update uniform buffer.
    let uniforms = AsciiUniforms {
        font_width: extracted.font_width,
        font_height: extracted.font_height,
        _padding: [0; 2],
    };
    let uniform_bytes = bytemuck::bytes_of(&uniforms);
    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("ascii_uniform_buffer"),
        contents: uniform_bytes,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    textures.uniform_buffer = Some(uniform_buffer);

    // Build bind groups.
    let texture_layout = pipeline_cache.get_bind_group_layout(&pipeline.texture_layout);
    let uniform_layout = pipeline_cache.get_bind_group_layout(&pipeline.uniform_layout);

    if let (Some(fore_view), Some(back_view), Some(text_view)) = (
        &textures.fore_view,
        &textures.back_view,
        &textures.text_view,
    ) {
        let texture_bind_group = render_device.create_bind_group(
            Some("ascii_texture_bind_group"),
            &texture_layout,
            &BindGroupEntries::sequential((
                fore_view,
                back_view,
                text_view,
                &font_gpu_image.texture_view,
            )),
        );
        textures.texture_bind_group = Some(texture_bind_group);
    }

    if let Some(ref uniform_buf) = textures.uniform_buffer {
        let uniform_bind_group = render_device.create_bind_group(
            Some("ascii_uniform_bind_group"),
            &uniform_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        );
        textures.uniform_bind_group = Some(uniform_bind_group);
    }
}

// ---------------------------------------------------------------------------
// ViewNode (render node)
// ---------------------------------------------------------------------------

/// Render node that draws the ASCII grid as a fullscreen triangle.
#[derive(Default)]
pub struct AsciiNode;

impl ViewNode for AsciiNode {
    type ViewQuery = &'static ViewTarget;

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_target: bevy::ecs::query::QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(gpu_textures) = world.get_resource::<AsciiGpuTextures>() else {
            return Ok(());
        };
        let Some(pipeline_res) = world.get_resource::<AsciiPipeline>() else {
            return Ok(());
        };
        let pipeline_cache = world.resource::<PipelineCache>();

        // Skip if bind groups not ready (font atlas still loading).
        let (Some(texture_bg), Some(uniform_bg)) = (
            &gpu_textures.texture_bind_group,
            &gpu_textures.uniform_bind_group,
        ) else {
            return Ok(());
        };

        // Get cached render pipeline (may not be compiled yet).
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_res.pipeline_id) else {
            return Ok(());
        };

        let color_attachment = view_target.get_color_attachment();
        let pass_descriptor = RenderPassDescriptor {
            label: Some("ascii_output_pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, texture_bg, &[]);
        render_pass.set_bind_group(1, uniform_bg, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
