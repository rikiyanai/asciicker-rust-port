// ASCII glyph compositing shader.
//
// Uses the Mage Core 4-texture approach:
//   t_fore  - foreground color per cell (Rgba8Unorm)
//   t_back  - background color per cell (Rgba8Unorm)
//   t_text  - character index per cell (R channel = glyph 0-255)
//   t_font  - CP437 font atlas (16x16 glyph grid)
//
// The vertex stage is provided by Bevy's built-in fullscreen vertex shader.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

// Texture bindings (group 0)
@group(0) @binding(0) var t_fore: texture_2d<f32>;
@group(0) @binding(1) var t_back: texture_2d<f32>;
@group(0) @binding(2) var t_text: texture_2d<f32>;
@group(0) @binding(3) var t_font: texture_2d<f32>;

// Uniform bindings (group 1)
struct Uniforms {
    font_width: u32,
    font_height: u32,
}

@group(1) @binding(0) var<uniform> uniforms: Uniforms;

@fragment
fn fragment(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Pixel center adjustment (Bevy positions are at pixel center, offset by 0.5)
    let p = vec2<f32>(pos.x - 0.5, pos.y - 0.5);

    let fw = i32(uniforms.font_width);
    let fh = i32(uniforms.font_height);

    // Cell coordinates (which ASCII cell this pixel belongs to)
    let cp = vec2<i32>(i32(p.x) / fw, i32(p.y) / fh);

    // Local pixel coordinates within the cell
    let lp = vec2<i32>(i32(p.x) % fw, i32(p.y) % fh);

    // Load cell data from the per-cell textures
    let fore = textureLoad(t_fore, cp, 0);
    let back = textureLoad(t_back, cp, 0);
    let text = textureLoad(t_text, cp, 0);

    // Decode glyph index from the R channel (stored as normalized float)
    let c = i32(text.r * 255.0);

    // Font atlas lookup: 16x16 grid of glyphs
    let fx = c % 16;
    let fy = c / 16;
    let lx = fx * fw + lp.x;
    let ly = fy * fh + lp.y;

    // Sample the font atlas at the computed coordinates
    let font_pixel = textureLoad(t_font, vec2<i32>(lx, ly), 0);

    // Foreground where the glyph pixel is lit, background elsewhere
    if font_pixel.r < 0.5 {
        return back;
    } else {
        return fore;
    }
}
