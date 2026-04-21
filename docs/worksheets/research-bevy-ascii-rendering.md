> **STATUS: ACTIVE REFERENCE** — Bevy ASCII rendering ecosystem analysis. Note: bevy_ascii_terminal version compatibility table may have minor formatting issues.

# Bevy ASCII/Terminal Rendering Research

## Executive Summary

Bevy has a mature ecosystem for ASCII and terminal rendering. The primary crate is **bevy_ascii_terminal** (141 stars, actively maintained), which provides a roguelike-focused terminal rendering system. For custom rendering pipelines, Bevy provides extensive APIs through `bevy_render` with support for custom render phases, shaders (WGSL), and material systems.

---

## 1. Existing ASCII/Terminal Rendering Crates

### 1.1 bevy_ascii_terminal (RECOMMENDED)

**Repository:** https://github.com/sarkahn/bevy_ascii_terminal
**Stars:** 141 | **License:** MIT

The most mature and widely-used ASCII terminal crate for Bevy. Designed for roguelikes but suitable for any terminal-style rendering.

**Key Features:**
- Tile-based ASCII rendering with per-character color
- Integrated into Bevy's ECS framework
- Supports font atlases for character rendering
- Active development (last push: Feb 2026)

**Basic Usage:**
```rust
use bevy::prelude::*;
use bevy_ascii_terminal::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TerminalPlugins))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Terminal::new([12, 1]).with_string([0, 0], "Hello world!"),
        TerminalBundle::from(Terminal::new([12, 1])),
    ));
}
```

### 1.2 bevy-ascii-effect

**Repository:** https://github.com/mrvintage710/bevy-ascii-effect
**Stars:** 2

A post-processing shader effect that converts any rendered scene into ASCII art. Uses WGSL shaders to sample the framebuffer and render as characters.

**Use Case:** Converting 3D/2D graphics to ASCII post-process

### 1.3 ascii-rust

**Repository:** https://github.com/JamesHDuffield/ascii-rust

A complete ASCII art space shooter game built with Bevy. Demonstrates practical ASCII game development.

### 1.4 bevy_ratatui_render

**Repository:** https://github.com/cxreiff/bevy_ratatui_render
**Downloads:** 21,770+

Renders Bevy scenes to terminal using ratatui (Rust TUI library). Uses unicode halfblocks for improved visuals.

**Key Features:**
- Bevy app renders to terminal window
- Supports keyboard/mouse input via ratatui
- Uses bevy_headless_render for windowless rendering

### 1.5 bevy_terminal_display

**Repository:** https://github.com/soaosdev/bevy_terminal_display

Experimental plugin for rendering Bevy to terminal with kitty terminal protocol support.

**Features:**
- Dithering to black/white, rendered as braille characters
- TUI widget integration via ratatui
- Terminal input handling

---

## 2. Text/Character Rendering in Bevy

### 2.1 Built-in Text Rendering (bevy_text)

Bevy has a mature text rendering system built on `cosmic_text` for text layout.

**Key Components:**
- `Text2d` - For rendering text in 2D world space
- `TextFont` - Font face, size, and style
- `TextColor` - Per-section text coloring
- `TextPipeline` - Handles glyph generation and layout
- `FontAtlasSet` - Caches rasterized glyphs in texture atlases

**Example (from examples/2d/text2d.rs):**
```rust
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_font = TextFont {
        font: font.clone().into(),
        font_size: FontSize::Px(50.0),
        ..default()
    };
    
    commands.spawn((
        Text2d::new("Hello world"),
        text_font,
        TextLayout::new_with_justify(Justify::Center),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));
}
```

### 2.2 Sprite-Based Character Rendering

Characters can be rendered as sprites using texture atlases:

**Architecture (from bevy_sprite):**
- `Sprite` component with optional `TextureAtlas`
- Per-sprite color tinting
- Custom size support
- Texture coordinate (UV) manipulation

**Pattern for ASCII:**
1. Create a texture atlas with all ASCII characters
2. Spawn a sprite for each character position
3. Use `texture_atlas.index` to select the character
4. Apply per-sprite color for colored ASCII

---

## 3. Custom Render Pipelines in Bevy

### 3.1 Render Pipeline Architecture

Bevy's rendering is built on wgpu. The key modules in `bevy_render`:

```
crates/bevy_render/src/
├── render_phase/     # Render phases (opaque, transparent, custom)
├── render_resource/  # Pipelines, buffers, bind groups
├── renderer/         # Render device, context, graph
├── mesh/             # Mesh handling
├── texture/          # Texture management
├── camera/           # Camera systems
└── view/             # Viewport management
```

### 3.2 Custom Render Phase

**Example:** `examples/shader_advanced/custom_render_phase.rs`

Shows how to create custom render phases for specialized rendering.

**Key Concepts:**
- `RenderPhase<T>` - A collection of items to render in a specific order
- `PhaseItem` - Individual items within a phase
- `RenderCommand` - Instructions for rendering a phase item

```rust
// Define a custom phase
#[derive(Component, ExtractComponent, Clone, Copy, Default)]
struct DrawStencil;

// Create the plugin
struct MeshStencilPhasePlugin;
impl Plugin for MeshStencilPhasePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderPhasePlugin::<DrawStencil>::default());
    }
}
```

### 3.3 Custom Materials and Shaders

**Files:**
- `examples/shader/shader_material.rs` - Basic custom shader
- `examples/shader_advanced/extended_material.rs` - Extending built-in materials

**WGSL Shader Example:**
```wgsl
#import bevy_sprite::mesh2d_types
#import bevy_sprite::mesh2d_vertex_output

struct CustomMaterial {
    color: vec4<f32>,
}

@fragment
fn fragment(
    in: Mesh2dVertexOutput,
    @uniform material: CustomMaterial,
) -> @location(0) vec4<f32> {
    return material.color;
}
```

### 3.4 Specialized Mesh Pipeline

**Example:** `examples/shader_advanced/specialized_mesh_pipeline.rs`

For full control over the rendering pipeline, use `SpecializedMeshPipeline`:

```rust
struct MyPipeline {
    mesh_pipeline: MeshPipeline,
    shader_handle: Handle<Shader>,
}

impl SpecializedMeshPipeline for MyPipeline {
    type Key = MeshPipelineKey;
    
    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        // Full pipeline customization
    }
}
```

### 3.5 Post-Processing Effects

**Example:** `examples/shader_advanced/custom_post_processing.rs`

For ASCII effects that process the entire frame:

```rust
// Create a post-processing effect
struct AsciiPostProcess {
    character_density: f32,
    // ... parameters
}

// Implement AsBindGroup for your effect
impl AsBindGroup for AsciiPostProcess {
    // ...
}
```

---

## 4. Non-Standard Rendering Examples

### 4.1 Mesh2d (2D Meshes)

**Location:** `examples/2d/mesh2d.rs`

Renders arbitrary meshes in 2D, useful for character-shaped geometry:

```rust
commands.spawn((
    Mesh2d(meshes.add(Rectangle::new(50.0, 50.0))),
    MeshMaterial2d(materials.add(ColorMaterial::default())),
    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
));
```

### 4.2 CPU Draw

**Location:** `examples/2d/cpu_draw.rs`

Allows direct CPU-based 2D rendering when GPU isn't suitable.

### 4.3 Texture Atlas

**Location:** `examples/2d/texture_atlas.rs`

Essential for ASCII rendering - shows how to use texture atlases for tile-based rendering.

### 4.4 Tilemap

**Location:** `examples/2d/tilemap_chunk.rs`

Bevy's tilemap system, directly applicable to ASCII grids.

---

## 5. Architecture Recommendations for Asciicker

### 5.1 Recommended Approach: Extend bevy_ascii_terminal

The existing `bevy_ascii_terminal` provides:
- Terminal data structure (`Terminal` component)
- Character buffer management
- ECS integration

**To extend for Asciicker:**
1. Fork or depend on `bevy_ascii_terminal`
2. Add custom components for your features (layers, animations)
3. Implement custom systems for your game logic
4. Use standard Bevy patterns for new rendering features

### 5.2 Alternative: Custom Sprite-Based System

If you need full control:

1. **Character Atlas:** Create a texture atlas with all ASCII characters (similar to font atlases)

2. **Terminal Component:**
```rust
#[derive(Component)]
struct AsciiTerminal {
    width: u32,
    height: u32,
    buffer: Vec<CharCell>,  // Character + color per cell
}

#[derive(Clone)]
struct CharCell {
    char: char,
    foreground: Color,
    background: Color,
}
```

3. **Rendering System:**
- Use `Sprite` components for each visible cell
- Update texture atlas index based on character
- Apply colors via sprite color

4. **Batch Rendering:** Use Bevy's GPU-driven rendering with instancing for performance

### 5.3 Key Bevy Patterns

**ECS Components:**
```rust
#[derive(Component, Clone)]
struct TerminalLayer {
    z_index: i32,
    visible: bool,
}

#[derive(Component)]
struct CharPosition {
    x: u32,
    y: u32,
}
```

**Systems:**
```rust
fn update_terminal(
    mut query: Query<(&mut Sprite, &TerminalLayer)>,
) {
    // Update rendering each frame
}
```

**Resources:**
```rust
#[derive(Resource)]
struct TerminalSettings {
    font_path: String,
    char_size: f32,
    default_foreground: Color,
    default_background: Color,
}
```

---

## 6. External Resources

### Crates
- **bevy_ascii_terminal:** https://github.com/sarkahn/bevy_ascii_terminal
- **bevy-ascii-effect:** https://github.com/mrvintage710/bevy-ascii-effect
- **bevy_ratatui_render:** https://github.com/cxreiff/bevy_ratatui_render
- **bevy_image_font:** Image-based pixel fonts

### Documentation
- **Bevy Rendering Guide:** https://taintedcoders.com/bevy/rendering
- **Bevy Text Docs:** https://docs.rs/bevy_text/latest/bevy_text/
- **WGPU Text:** https://github.com/Blatko1/wgpu-text

### Examples (in bevy repo)
- `examples/2d/text2d.rs` - Text rendering
- `examples/2d/texture_atlas.rs` - Texture atlases
- `examples/2d/mesh2d.rs` - 2D meshes
- `examples/shader_advanced/custom_render_phase.rs` - Custom rendering
- `examples/shader_advanced/custom_post_processing.rs` - Post-processing

---

## 7. Version Compatibility

| Crate | Bevy Version |
|-------|--------------|
| bevy_ascii_terminal 0.18.1 | 0.17 |
| bevy_ascii_terminal (latest) | 0.18+ |
| bevy_ratatui_render | 0.15+ |

**Note:** Bevy 0.18 introduced significant rendering changes. Ensure crate compatibility before use.

---

## Conclusion

For implementing ASCII rendering in Asciicker:

1. **Primary recommendation:** Use `bevy_ascii_terminal` as a foundation
2. **For custom graphics:** Extend with sprite-based character rendering using texture atlases
3. **For advanced effects:** Use Bevy's custom render pipeline APIs (render phases, materials, shaders)
4. **For terminal output:** Consider `bevy_ratatui_render` for terminal-based display

The Bevy ecosystem has mature solutions for all ASCII rendering use cases. The key is leveraging the existing `bevy_ascii_terminal` crate while extending it with custom components and systems for game-specific features.
