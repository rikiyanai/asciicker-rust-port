> **STATUS: ACTIVE REFERENCE** — Core integration analysis is valid. Mage Core's rendering patterns (4-texture GPU approach) will be implemented within Bevy's WGPU pipeline. Note: Some code examples in this document contain fabricated API types (FontAssets, FontAtlasSize, FontLoadRequest) and corrupted code — verify against actual Mage Core source at `/Users/r/Projects/ascii research/Mage-core/src/`.

# Mage-core → Bevy Implementation Strategy

## Executive Summary

This document outlines how to implement Mage-core's ASCII rendering approach within Bevy's ECS framework. **Mage-core features can be adapted to Bevy** without losing their functionality.

---

## Architecture Comparison

### Mage-core Standalone
```
┌─────────────────────────────────────────┐
│         Mage-core (Standalone)          │
├─────────────────────────────────────────┤
│  lib.rs:    async run(), event loop    │
│  app.rs:    App trait (tick/present)   │
│  render.rs: WGPU pipeline              │
│  input.rs:  Keyboard modifiers only    │
│  colour.rs: ANSI + RGB                │
│  image.rs:  Point, Rect, Char, Image  │
│  present.rs: Blitting                  │
└─────────────────────────────────────────┘
```

### Mage-core → Bevy Adaptation
```
┌─────────────────────────────────────────────────────────────┐
│                      Bevy Engine                            │
│  ECS | Input (full) | Audio | UI | State Machines         │
├─────────────────────────────────────────────────────────────┤
│  Mage-core Modules Adapted to Bevy:                        │
│  • render.rs → Custom render phase + WGPU textures        │
│  • input.rs  → bevy_input (full keyboard/mouse/gamepad)  │
│  • colour.rs → bevy_color (or custom)                     │
│  • image.rs  → bevy_ecs + custom components               │
│  • present.rs→ Custom system for blitting                  │
│  • app.rs    → Bevy App (already has tick/present)       │
└─────────────────────────────────────────────────────────────┘
```

---

## Module Mapping

| Mage-core Module | Bevy Equivalent | Adaptation Notes |
|-----------------|-----------------|------------------|
| `lib.rs` (run) | `bevy::prelude::App` | Use `add_systems(Update, ...)` |
| `app.rs` (App trait) | Built-in | Bevy has tick/present via systems |
| `render.rs` (WGPU) | `RenderDevice`, `RenderQueue` | Access wgpu directly |
| `input.rs` (modifiers) | `bevy_input` | Full keyboard/mouse/gamepad |
| `colour.rs` (ANSI) | `bevy_color` or custom | 16-color + RGB mapping |
| `image.rs` (primitives) | `bevy_ecs` components | Create `#[derive(Component)]` |
| `present.rs` (blit) | Custom system | Update `Image` assets each frame |

---

## WGPU Integration

### Accessing WGPU Device

```rust
fn my_render_system(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Access underlying wgpu types
    let wgpu_device = render_device.wgpu_device();
    let wgpu_queue = &*render_queue;
    
    // Create custom textures
    let texture = render_device.create_texture(&wgpu::TextureDescriptor {
        size: Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        ..default()
    });
}
```

### Creating Triple-Buffer Textures

```rust
// In a startup system
fn setup_ascii_buffers(
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let width = 80u32;
    let height = 24u32;
    
    // Foreground color buffer
    let mut fg_image = Image::new_fill(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],  // Default: black
        TextureFormat::Rgba8Unorm,
    );
    
    // Background color buffer
    let mut bg_image = Image::new_fill(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    
    // Character code buffer (R8 = 1 byte per cell)
    let mut chars_image = Image::new_fill(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[32],  // Default: space
        TextureFormat::R8Unorm,
    );
    
    // Insert as assets
    let fg_handle = images.add(fg_image);
    let bg_handle = images.add(bg_image);
    let chars_handle = images.add(chars_image);
    
    // Store as resource
    commands.insert_resource(AsciiBuffers {
        fg: fg_handle,
        bg: bg_handle,
        chars: chars_handle,
    });
}
```

---

## Input System

### Mage-core (Minimal)
```rust
// Only tracked modifier keys
struct ShiftState { shift: bool, ctrl: bool, alt: bool }
```

### Bevy (Full)
```rust
// Full keyboard input
fn keyboard_input(mut keys: ResMut<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyW) { /* handle W */ }
    if keys.pressed(KeyCode::ShiftLeft) { /* handle shift */ }
    if keys.just_released(KeyCode::KeyQ) { /* handle release */ }
}

// Mouse input
fn mouse_input(
    mut motion: EventReader<MouseMotion>,
    mut buttons: ResMut<ButtonInput<MouseButton>>,
) {
    for event in motion.read() {
        println!("Mouse moved: {:?}", event.delta);
    }
    if buttons.just_pressed(MouseButton::Left) { /* click */ }
}

// Gamepad input
fn gamepad_input(
    gamepads: Query<&Gamepad>,
    buttons: Res<ButtonInput<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
) {
    // Full gamepad support
}
```

---

## Color System

### Mage-core ANSI + RGB
```rust
// 16-color ANSI palette + RGB
struct Color { r: u8, g: u8, b: u8, ansi: Option<u8> }
```

### Bevy Integration
```rust
use bevy_color::Color;

// Use Bevy's built-in color types
let ansi_red = Color::rgb(1.0, 0.0, 0.0);
let custom = Color::rgba(0.5, 0.25, 0.75, 1.0);

// Or create ANSI mapping component
#[derive(Component)]
struct AnsiColor(pub u8);  // 0-15 for ANSI colors
```

---

## Image Primitives

### Mage-core Primitives
```rust
struct Point { x: i32, y: i32 }
struct Rect { x: i32, y: i32, w: i32, h: i32 }
struct Char { c: char, fg: Color, bg: Color }
struct Image { width: u32, height: u32, cells: Vec<Char> }
```

### Bevy ECS Components
```rust
use bevy_ecs::prelude::*;

// Define as components
#[derive(Component)]
struct AsciiPosition(pub i32, pub i32);

#[derive(Component)]
struct AsciiRect {
    x: i32, y: i32, width: u32, height: u32,
}

#[derive(Component)]
struct AsciiCell {
    char: u8,
    fg_color: [f32; 4],
    bg_color: [f32; 4],
}

// Resource for the full screen buffer
#[derive(Resource)]
struct AsciiScreen {
    width: u32,
    height: u32,
    cells: Vec<AsciiCell>,  // width * height
}
```

---

## Blitting / Present System

### Mage-core Approach
```rust
// present.rs - blit sprite to buffer
fn blit(sprite: &Sprite, dest: &mut Image, x: i32, y: i32) {
    for py in 0..sprite.h {
        for px in 0..sprite.w {
            let[py * sprite.w + px];
 char = sprite.data            dest.cells[(y + py) * dest.width + (x + px)] = char;
        }
    }
}
```

### Bevy System
```rust
fn present_ascii_buffer(
    mut images: ResMut<Assets<Image>>,
    buffers: Res<AsciiBuffers>,
    screen: Res<AsciiScreen>,
) {
    // Get mutable access to textures
    let fg = images.get_mut(&buffers.fg).unwrap();
    let bg = images.get_mut(&buffers.bg).unwrap();
    let chars = images.get_mut(&buffers.chars).unwrap();
    
    // Blit screen cells to textures
    for y in 0..screen.height {
        for x in 0..screen.width {
            let cell = &screen.cells[(y * screen.width + x) as usize];
            
            // Update character texture
            chars.data[(y * screen.width + x) as usize] = cell.char;
            
            // Update fg color texture (RGBA)
            let fg_idx = ((y * screen.width + x) * 4) as usize;
            fg.data[fg_idx..fg_idx+4].copy_from_slice(&cell.fg_color);
            
            // Update bg color texture
            let bg_idx = ((y * screen.width + x) * 4) as usize;
            bg.data[bg_idx..bg_idx+4].copy_from_slice(&cell.bg_color);
        }
    }
    // Bevy auto-syncs to GPU
}
```

---

## Font Atlas

### Mage-core
```rust
// Load PNG as 16x16 font atlas
let font = load_png("font.png");  // 16 cols, 16 rows = 256 chars
```

### Bevy
```rust
use bevy_text::prelude::*;

// Option 1: Use Bevy's FontAtlas system
fn setup_font(
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut textures: ResMut<Assets<Image>>,
    mut chars: ResMut<AsciiCharToQuad>,
) {
    // Add font to atlas
    let font_handle = /* load your font */;
    let (atlas_info, _) = font_atlas_set.get_or_insert_collection(
        FontAssets::Default,
        FontAtlasSize::SIZE_32,
        &FontLoadRequest {
            fonts: vec![font_handle],
            scale: 1.0,
        },
    );
}

// Option 2: Custom font texture (like Mage-core)
fn load_custom_font(
    mut images: ResMut<Assets<Image>>,
) -> Handle<Image> {
    let font_png = /* load your PNG */;
    images.add(font_png)
}
```

---

## Complete Integration Example

```rust
// main.rs
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(RenderPlugin {
            wgpu_settings: WgpuSettings {
                backends: Some(WgpuBackend::Primary),
                ..default()
            },
        }))
        // ASCII rendering resources
        .init_resource::<AsciiBuffers>()
        .init_resource::<AsciiScreen>()
        .init_resource::<AsciiCharToQuad>()
        // Startup systems
        .add_systems(Startup, (
            setup_ascii_buffers,
            load_font_atlas,
        ))
        // Game systems
        .add_systems(Update, (
            handle_input,
            update_game_logic,
            render_sprites,
            present_ascii_buffer,
        ))
        // Custom render phase
        .add_plugins(AsciiRenderPlugin)
        .run();
}

// Custom render plugin for ASCII
struct AsciiRenderPlugin;

impl Plugin for AsciiRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, render_ascii_to_screen);
    }
}
```

---

## Summary: Bevy Advantages Over Standalone Mage-core

| Feature | Mage-core | Bevy + Mage-core |
|---------|-----------|------------------|
| **Input** | Modifiers only | Full K/M/G |
| **ECS** | None | Full ECS |
| **Audio** | None | Built-in |
| **UI** | None | Built-in |
| **States** | Manual | Built-in |
| **Hot Reload** | None | Built-in |
| **Cross-platform** | Manual | Built-in |
| **Asset Mgmt** | Manual | Built-in |

**Conclusion**: Implement Mage-core's rendering approach within Bevy, not as a standalone engine.

---

*Document Version: 1.0*
*Created: 2026-02-19*
