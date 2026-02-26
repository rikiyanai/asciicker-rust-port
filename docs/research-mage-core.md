> **STATUS: ACTIVE REFERENCE** — Mage Core is a real Rust ASCII rendering engine (local at `/Users/r/Projects/ascii research/Mage-core`). Its 4-texture GPU rendering approach (char, fg, bg, font atlas via WGPU/WGSL shader) is a candidate pattern for the Bevy render plugin. Engine decision D001 chose Bevy as the game framework; Mage Core's rendering approach can be integrated as a Bevy render plugin.

# Mage-core Rust Engine Research

## 1. Architecture and Module Structure

The Mage-core engine is a modern GPU-accelerated ASCII game engine built with Rust and WGPU. It follows a clean modular architecture:

### Module Overview

| File | Purpose |
|------|---------|
| **lib.rs** | Entry point, async `run()` function, event loop management |
| **app.rs** | `App` trait defining the game interface (tick/present pattern) |
| **render.rs** | WGPU rendering pipeline, texture management |
| **input.rs** | Keyboard modifier state tracking (Shift/Ctrl/Alt) |
| **colour.rs** | 16-color ANSI palette + RGB support |
| **image.rs** | 2D image primitives (Point, Rect, Char, Image) |
| **present.rs** | Blitting operations for sprite rendering |
| **config.rs** | Configuration, font loading from PNG |
| **error.rs** | Error types using `thiserror` |

### Core Design Patterns

1. **Trait-based Application Interface**: The `App` trait is the main interface - games implement `tick()` for logic and `present()` for rendering.

2. **Triple Buffer System**: Three separate RGBA textures for foreground color, background color, and character codes.

3. **Font Atlas System**: Fonts are loaded from PNG images arranged as a 16x16 grid (256 characters).

---

## 2. ASCII/Terminal Rendering Pipeline

### Rendering Architecture

The engine uses a **4-texture GPU pipeline**:

```
┌─────────────────────────────────────────────────────────────┐
│                    WGPU Shader Pipeline                       │
├─────────────────────────────────────────────────────────────┤
│  Bind Group 0:                                              │
│    [0] fg_texture  - Foreground color (RGBA8)               │
│    [1] bg_texture  - Background color (RGBA8)               │
│    [2] chars_texture - Character codes (8-bit values)        │
│    [3] font_texture  - Font atlas (RGBA8)                   │
├─────────────────────────────────────────────────────────────┤
│  Bind Group 1:                                              │
│    [0] uniforms   - font_width, font_height                 │
└─────────────────────────────────────────────────────────────┘
```

### How It Works

1. **Character Data**: Each cell stores a single byte (0-255) representing the ASCII/CP437 character code
2. **Color Data**: RGBA values stored per-cell (not per-pixel)
3. **Shader Logic** (from shader.wgsl):
   - Calculate which character cell the current pixel falls into
   - Look up the character code from `chars_texture`
   - Use character code to compute position in font atlas (16x16 grid)
   - Sample the font atlas pixel - if darker than 0.5, output background color; otherwise output foreground color

### Window Management

- Window dimensions snap to character boundaries
- Minimum window: 20x20 characters
- Supports fullscreen toggle (Alt+Enter)
- Handles resize with automatic surface recreation

---

## 3. Key Data Structures

### Buffer Formats (32-bit RGBA)

```
Foreground Color:  [B][G][R][A]  (little-endian: 0xAABBGGRR)
Background Color:  [B][G][R][A]  (same format)
Character Code:    [unused][unused][unused][char_byte]
```

### Core Types

```rust
// app.rs
struct TickInput { dt: Duration, width: u32, height: u32 }
struct PresentInput<'a> { 
    width: u32, height: u32,
    fore_image: &'a mut [u32],
    back_image: &'a mut [u32], 
    text_image: &'a mut [u32]
}

// image.rs
struct Image { width, height, fore_image, back_image, text_image }
struct Point { x: i32, y: i32 }
struct Rect { x: i32, y: i32, width: u32, height: u32 }
struct Char { ch: u32, ink: u32, paper: u32 }

// colour.rs
enum Colour { Black, Blue, Green, ..., Rgb(u8, u8, u8) }
```

### Font Loading

```rust
struct FontData {
    data: Vec<u32>,        // RGBA pixel data
    char_width: u32,       // e.g., 12 pixels
    char_height: u32,      // e.g., 12 pixels
}
```

---

## 4. Comparison with Asciicker C++ Engine

### Mage-core Advantages (for porting)

| Aspect | Mage-core | Asciicker (inferred from docs) |
|--------|-----------|-------------------------------|
| **Rendering** | GPU-accelerated via WGPU | Software/OpenGL (stb_truetype) |
| **Language** | Safe Rust | C++ (memory unsafe) |
| **Font Format** | PNG atlas | PSF/TTF (system fonts) |
| **Color Depth** | Full RGBA per cell | Limited palette |
| **Windowing** | winit (cross-platform) | Platform-specific |
| **Threading** | Async/await | Custom main loop |

### Key Differences

1. **Font Rendering**: Mage-core uses a pre-baked PNG font atlas; Asciicker uses runtime font loading (stb_truetype)
2. **Coordinate System**: Mage-core uses top-left origin; Asciicker may differ
3. **Buffer Strategy**: Mage-core gives mutable slice access per-frame; Asciicker likely uses double-buffering
4. **Event Handling**: Mage-core uses winit's event loop; Asciicker has custom input handling

### What's Missing in Mage-core (for full game engine)

Based on the capability matrix and typical game engine needs:

| Feature | Status | Implementation Notes |
|---------|--------|---------------------|
| Sprite loading (XP format) | Missing | Need .xp parser |
| Audio | Missing | stb_vorbis vendored in Asciicker |
| Physics | Missing | Custom sweep/prune in Asciicker |
| Network/Multiplayer | Missing | WebSocket in Asciicker |
| Gamepad support | Missing | SDL2 in Asciicker |
| World/Map system | Missing | BSP/Quadtree in Asciicker |

---

## 5. Porting Recommendations

### For Matching Asciicker Functionality

To create a full game engine matching Asciicker, you would need to implement:

#### High Priority

1. **XP Sprite Format Parser** - Parse the binary XP format used by Asciicker
2. **Image/Blit Operations** - Already have basic blit; need rotation/flip support
3. **Animation System** - Frame-based sprite animation
4. **Z-Ordering/Layering** - Multiple render layers

#### Medium Priority

5. **Font Rendering Alternative** - Add TTF/OTF support via stb_truetype or rusttype
6. **Input System** - Keyboard, mouse, gamepad support
7. **Configuration System** - Save/load game configs

#### Lower Priority

8. **Audio** - Audio playback via rodio or cpal
9. **Physics** - AABB collision, sweep/prune
10. **Networking** - WebSocket client/server

### Architecture for Game State

```rust
// Example: How to structure a game
struct GameState {
    sprites: HashMap<String, Sprite>,
    player: Entity,
    world: World,
}

impl App for GameState {
    fn tick(&mut self, input: TickInput) -> TickResult {
        // Update game logic
        // Handle input
        // Run physics
        TickResult::Continue
    }
    
    fn present(&mut self, input: PresentInput) -> PresentResult {
        // Clear screen
        // Blit sprites to back buffer
        // Copy to present textures
        PresentResult::Changed
    }
}
```

---

## 6. Dependencies

From Cargo.toml:
- **wgpu** (22.1) - GPU rendering
- **winit** (0.29) - Window management
- **image** (0.24) - PNG loading
- **bytemuck** (1.13) - Zero-copy buffer casting
- **thiserror** (1.0) - Error types
- **chrono** (0.4) - Time handling
- **tracing** (0.1) - Logging
- **tokio** (1.28) - Async runtime
- **winit-fullscreen** (1.0) - Fullscreen toggle

---

## Summary

Mage-core provides a solid foundation for GPU-accelerated ASCII rendering in Rust. Its clean architecture, safe memory model, and modern async patterns make it an excellent target for porting Asciicker functionality. The main gaps are in asset pipeline (XP format, TTF fonts), audio, physics, and higher-level game systems.

For a complete port, you would build game-specific systems on top of the `App` trait, using the triple-buffer rendering approach with direct pixel manipulation through the `PresentInput` buffers.
