//! RESOLVE stage: downsample 2x2 sample blocks into AnsiCell grid.
//!
//! Port of C++ render.cpp:3412-3938 (simplified for Phase 4).
//!
//! Two code paths:
//! - **Material path** (spare & MESH_FLAG == 0): terrain samples use
//!   `matlib[visual].shade[elevation][diffuse/17]` -> MatCell -> rgb2pal
//! - **Mesh path** (spare & MESH_FLAG != 0): mesh samples use
//!   `auto_mat_lookup(rgb555)` -> (bg, fg, dither_glyph)
//!
//! Grid/wireframe spare bits override glyph selection.

use super::material::{Material, auto_mat_lookup};
use super::quantize::{rgb2pal, rgb8_to_rgb5, rgb555_to_rgb888};
use super::sample_buffer::{Sample, spare_bits};
use super::types::AnsiCell;


/// Resolve the 2x-supersampled SampleBuffer into an AnsiCell grid.
///
/// Reads 2x2 sample blocks, determines terrain vs mesh path, applies
/// elevation-based glyph selection and grid/wireframe overlays.
///
/// # Arguments
/// * `samples` - Flat sample buffer slice (row-major, dw * dh elements)
/// * `dw` - Sample buffer width (`2 * ascii_w + 4`)
/// * `dh` - Sample buffer height (`2 * ascii_h + 4`)
/// * `ascii_w` - Output grid width
/// * `ascii_h` - Output grid height
/// * `materials` - Material library for terrain shade lookups
/// * `output` - Output slice of `ascii_w * ascii_h` AnsiCells
pub fn resolve(
    samples: &[Sample],
    dw: i32,
    dh: i32,
    ascii_w: i32,
    ascii_h: i32,
    materials: &[Material],
    output: &mut [AnsiCell],
) {
    debug_assert_eq!(samples.len(), (dw * dh) as usize);
    debug_assert_eq!(output.len(), (ascii_w * ascii_h) as usize);
    debug_assert!(dw >= 2 * ascii_w + 4);
    debug_assert!(dh >= 2 * ascii_h + 4);

    // Precompute sky palette index (C++ render.cpp:2884 clear color)
    // Sky blue: RGB555(12,12,27) → RGB888(100,100,220)
    let sky_pal = rgb2pal([100, 100, 220]);

    for cy in 0..ascii_h {
        for cx in 0..ascii_w {
            let out_idx = (cy * ascii_w + cx) as usize;

            // 2x2 block position in sample buffer (skip +2 border)
            let sx = 2 + 2 * cx;
            let sy = 2 + 2 * cy;

            let i00 = (sy * dw + sx) as usize;
            let i10 = i00 + 1;
            let i01 = ((sy + 1) * dw + sx) as usize;
            let i11 = i01 + 1;

            let s00 = &samples[i00];
            let s10 = &samples[i10];
            let s01 = &samples[i01];
            let s11 = &samples[i11];

            // Check if all 4 samples are clear — sky color
            if is_clear(s00) && is_clear(s10) && is_clear(s01) && is_clear(s11) {
                output[out_idx] = AnsiCell {
                    fg: sky_pal,
                    bk: sky_pal,
                    gl: b' ',
                    spare: 0,
                };
                continue;
            }

            // Average height (ignoring clear samples) — used for future shadow/reflection
            let (_avg_height, _height_count) = average_height(s00, s10, s01, s11);

            // Sum of all 4 diffuse values (C++ render.cpp:3449 keeps individual dif[4])
            let diffuse_sum =
                s00.diffuse as u32 + s10.diffuse as u32 + s01.diffuse as u32 + s11.diffuse as u32;

            // Combined spare flags (OR of all 4)
            let combined_spare = s00.spare | s10.spare | s01.spare | s11.spare;

            // Dominant visual: first non-clear sample's visual
            let dominant = dominant_sample(s00, s10, s01, s11);

            // Determine if reflection (spare & PARITY_MASK == REFLECTION and not mesh)
            let is_reflection = (combined_spare & spare_bits::PARITY_MASK)
                == spare_bits::REFLECTION
                && (combined_spare & spare_bits::MESH_FLAG) == 0;

            // Diffuse divisor: reflections use 400 (darker), normal uses 255
            let diffuse_divisor: u32 = if is_reflection { 400 } else { 255 };

            let cell = if combined_spare & spare_bits::MESH_FLAG != 0 {
                // Mesh path: auto_mat lookup (uses average diffuse)
                let avg_diffuse = diffuse_sum / 4;
                resolve_mesh(dominant.visual, avg_diffuse, diffuse_divisor)
            } else {
                // Material path: shade table lookup (uses C++ sum-based rounding)
                let ctx = MaterialResolveCtx {
                    samples,
                    dw,
                    sx,
                    sy,
                    materials,
                };
                resolve_material(dominant.visual, diffuse_sum, &ctx, is_reflection)
            };

            // Apply grid/wireframe overlay
            let gl = apply_overlay(cell.gl, combined_spare, cx, cy);

            output[out_idx] = AnsiCell {
                fg: cell.fg,
                bk: cell.bk,
                gl,
                spare: 0xFF,
            };
        }
    }
}

/// Check if a sample is at clear height (sky).
#[inline]
fn is_clear(s: &Sample) -> bool {
    s.height == Sample::CLEAR_HEIGHT
}

/// Average height of non-clear samples in a 2x2 block.
/// Returns (average_height, count_of_non_clear).
#[inline]
fn average_height(s00: &Sample, s10: &Sample, s01: &Sample, s11: &Sample) -> (f32, u32) {
    let mut sum = 0.0_f32;
    let mut count = 0u32;
    for s in [s00, s10, s01, s11] {
        if !is_clear(s) {
            sum += s.height;
            count += 1;
        }
    }
    if count > 0 {
        (sum / count as f32, count)
    } else {
        (Sample::CLEAR_HEIGHT, 0)
    }
}

/// Get the dominant sample (first non-clear, or fallback to s00).
#[inline]
fn dominant_sample<'a>(
    s00: &'a Sample,
    s10: &'a Sample,
    s01: &'a Sample,
    s11: &'a Sample,
) -> &'a Sample {
    for s in [s00, s10, s01, s11] {
        if !is_clear(s) {
            return s;
        }
    }
    s00
}

/// Intermediate resolve result before overlay.
struct ResolvedCell {
    fg: u8,
    bk: u8,
    gl: u8,
}

/// Resolve a mesh sample via auto_mat lookup.
///
/// Expands RGB555 to RGB888, applies diffuse scaling, converts back to RGB555,
/// then looks up the auto_mat LUT.
fn resolve_mesh(visual_rgb555: u16, avg_diffuse: u32, diffuse_divisor: u32) -> ResolvedCell {
    // Expand RGB555 to RGB888
    let (r8, g8, b8) = rgb555_to_rgb888(visual_rgb555);

    // Apply diffuse scaling
    let r_scaled = ((r8 as u32) * avg_diffuse / diffuse_divisor).min(255) as u8;
    let g_scaled = ((g8 as u32) * avg_diffuse / diffuse_divisor).min(255) as u8;
    let b_scaled = ((b8 as u32) * avg_diffuse / diffuse_divisor).min(255) as u8;

    // Convert back to RGB555
    let scaled_rgb555 = (rgb8_to_rgb5(r_scaled) as u16)
        | ((rgb8_to_rgb5(g_scaled) as u16) << 5)
        | ((rgb8_to_rgb5(b_scaled) as u16) << 10);

    // auto_mat lookup
    let (bg, fg, dither_glyph) = auto_mat_lookup(scaled_rgb555);

    ResolvedCell {
        fg,
        bk: bg,
        gl: dither_glyph,
    }
}

/// Context for resolving a terrain material sample.
struct MaterialResolveCtx<'a> {
    samples: &'a [Sample],
    dw: i32,
    sx: i32,
    sy: i32,
    materials: &'a [Material],
}

/// Resolve a terrain material sample via shade table lookup.
///
/// Computes elevation (0-3) from bit 15 of visual values in surrounding rows,
/// masks off bit 15 to get the material index, looks up the material shade
/// table, and converts MatCell colors to xterm-256 palette indices.
///
/// `diffuse_sum` is the raw sum of all 4 samples' diffuse values (0-1020).
/// When `is_reflection` is true, applies water tinting (darker, blue-shifted)
/// matching C++ render.cpp reflection behaviour for terrain.
fn resolve_material(
    visual: u16,
    diffuse_sum: u32,
    ctx: &MaterialResolveCtx<'_>,
    is_reflection: bool,
) -> ResolvedCell {
    // C++ render.cpp:3448: mat[i] = src[i].visual & 0x00FF (8-bit material index).
    // Upper bits may be used for visual shade / animation in future.
    let mat_idx = (visual & 0x00FF) as usize;

    // Compute elevation from bit 15 of surrounding samples (C++ render.cpp:3456-3474)
    let elevation = compute_elevation(ctx.samples, ctx.dw, ctx.sx, ctx.sy);

    // C++ render.cpp:3493: shd = (dif[0]+dif[1]+dif[2]+dif[3] + 17*2) / (17*4)
    // Rounding bias (+34) rounds to nearest instead of truncating.
    let shade_idx = ((diffuse_sum + 34) / 68).min(15) as usize;

    if mat_idx < ctx.materials.len() {
        let mat_cell = &ctx.materials[mat_idx].shade[elevation as usize][shade_idx];

        if is_reflection {
            // C++ render.cpp:3622-3627: uniform brightness dimming for reflected
            // terrain cells — multiply each channel by 255/400 (~64%).
            // NO blue tint — the C++ engine dims all channels equally.
            let dim = |rgb: [u8; 3]| -> [u8; 3] {
                [
                    ((rgb[0] as u32) * 255 / 400).min(255) as u8,
                    ((rgb[1] as u32) * 255 / 400).min(255) as u8,
                    ((rgb[2] as u32) * 255 / 400).min(255) as u8,
                ]
            };

            ResolvedCell {
                fg: rgb2pal(dim(mat_cell.fg)),
                bk: rgb2pal(dim(mat_cell.bg)),
                gl: mat_cell.gl,
            }
        } else {
            let fg_pal = rgb2pal(mat_cell.fg);
            let bg_pal = rgb2pal(mat_cell.bg);
            ResolvedCell {
                fg: fg_pal,
                bk: bg_pal,
                gl: mat_cell.gl,
            }
        }
    } else {
        // Fallback for invalid material index
        ResolvedCell {
            fg: 0,
            bk: 0,
            gl: b'?',
        }
    }
}

/// Compute elevation (0-3) from bit 15 of visual values in surrounding rows.
///
/// Ports C++ render.cpp:3456-3474. Reads bit 15 (elevation flag) from the
/// row above the 2x2 block and the bottom row of the block. The pattern
/// of elevated vs non-elevated rows determines the elevation index:
/// - 0 = lowering edge (above elevated, below not)
/// - 1 = high flat (both elevated)
/// - 2 = raising edge (above not elevated, below elevated)
/// - 3 = low flat (neither elevated)
///
/// When no samples have bit 15 set (common in terrain data without elevation
/// flags), all pairs are 0 → elevation=3. This matches C++ exactly.
fn compute_elevation(samples: &[Sample], dw: i32, sx: i32, sy: i32) -> u8 {
    let bit15 = |row: i32, col: i32| -> i32 {
        if row < 0 || col < 0 {
            return 0;
        }
        let idx = (row * dw + col) as usize;
        samples
            .get(idx)
            .map_or(0, |s| ((s.visual >> 15) & 1) as i32)
    };

    // Row above the 2x2 block (C++ src[-dw] and src[-dw+1])
    let e_lo = bit15(sy - 1, sx) + bit15(sy - 1, sx + 1);

    // Bottom row of the 2x2 block (C++ src[dw] and src[dw+1])
    let e_hi = bit15(sy + 1, sx) + bit15(sy + 1, sx + 1);

    // C++ render.cpp:3461-3474: elevation from bit-15 pattern.
    // When no bit-15 flags exist (e_lo=0, e_hi=0), both are <=1 → elevation=3.
    // This IS correct C++ behavior — material shade[3] is the "low flat" entry.
    if e_lo <= 1 {
        if e_hi <= 1 { 3 } else { 2 }
    } else if e_hi <= 1 {
        0
    } else {
        1
    }
}

/// Apply grid/wireframe overlay to the resolved glyph.
///
/// Grid lines (spare & 0x04) override with grid characters.
/// Wireframe (spare & 0x40) overrides with wireframe characters.
fn apply_overlay(base_glyph: u8, combined_spare: u8, cx: i32, cy: i32) -> u8 {
    if combined_spare & spare_bits::WIREFRAME != 0 {
        return b'/';
    }

    if combined_spare & spare_bits::GRID != 0 {
        // Determine grid character based on which directions have grid bits
        // Simplified for Phase 4: use '+' at intersections, '-' for horizontal, '|' for vertical
        let is_h_edge = cx % 2 == 0;
        let is_v_edge = cy % 2 == 0;
        return if is_h_edge && is_v_edge {
            b'+'
        } else if is_h_edge {
            b'-'
        } else {
            b'|'
        };
    }

    base_glyph
}
