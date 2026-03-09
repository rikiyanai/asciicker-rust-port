use super::sample_buffer::Sample;

// ---------------------------------------------------------------------------
// RasterShader trait
// ---------------------------------------------------------------------------

/// Trait for shader dispatch during triangle rasterization.
///
/// Uses `impl RasterShader` (static dispatch / monomorphization) rather than
/// `dyn RasterShader` to match C++ template inlining for zero-cost dispatch.
///
/// Matches C++ `Shader::Blend(Sample* s, float z, float nbc[3])` from
/// render.cpp:404-557.
pub trait RasterShader {
    /// Blend a fragment into the sample at the given depth and barycentric
    /// coordinates. Called for every pixel inside the triangle.
    ///
    /// # Arguments
    /// * `sample` - Mutable reference to the sample buffer cell
    /// * `z` - Interpolated depth at this pixel
    /// * `bc` - Normalized barycentric coordinates [w0, w1, w2], summing to ~1.0
    fn blend(&self, sample: &mut Sample, z: f32, bc: [f32; 3]);
}

// ---------------------------------------------------------------------------
// Edge function helpers (private)
// ---------------------------------------------------------------------------

/// Compute 2 * signed area of triangle (a, b, c).
///
/// Matches C++ `BC_A(a, b, c)` macro from render.cpp:424.
#[inline]
fn bc_a(a: &[i32; 4], b: &[i32; 4], c: &[i32; 4]) -> i32 {
    2 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]))
}

/// Evaluate edge function at pixel center (cx + 0.5, cy + 0.5).
///
/// Matches C++ `BC_P(a, b, c)` macro from render.cpp:431.
/// The `2*c+1` terms sample at pixel centers rather than corners.
#[inline]
fn bc_p(a: &[i32; 4], b: &[i32; 4], cx: i32, cy: i32) -> i32 {
    (b[0] - a[0]) * (2 * cy + 1 - 2 * a[1]) - (b[1] - a[1]) * (2 * cx + 1 - 2 * a[0])
}

// ---------------------------------------------------------------------------
// Triangle rasterizer
// ---------------------------------------------------------------------------

/// Barycentric triangle rasterizer with shader-driven blending.
///
/// Accepts any `impl RasterShader` for zero-cost static dispatch.
/// Handles both CCW and CW (double-sided) winding. Matches C++
/// render.cpp:404-557.
///
/// # Arguments
/// * `buf` - Flat sample buffer slice (row-major, w*h elements)
/// * `w` - Buffer width in samples
/// * `h` - Buffer height in samples
/// * `shader` - Shader providing the `blend` method for each pixel
/// * `v` - Three vertices, each `[x, y, z, cull_flags]` in sample-buffer coords
/// * `double_sided` - If true, CW winding triangles are also rasterized
pub fn rasterize(
    buf: &mut [Sample],
    w: i32,
    h: i32,
    shader: &impl RasterShader,
    v: [&[i32; 4]; 3],
    double_sided: bool,
) {
    // Cull check: if all 3 vertices share a common cull bit, skip.
    if v[0][3] & v[1][3] & v[2][3] != 0 {
        return;
    }

    let area = bc_a(v[0], v[1], v[2]);

    if area > 0 {
        // CCW winding
        if area >= 0x10000 {
            return; // degenerate (too large)
        }
        rasterize_ccw(buf, w, h, shader, v, area);
    } else if area < 0 && double_sided {
        // CW winding (only when double-sided)
        if area <= -0x10000 {
            return; // degenerate
        }
        rasterize_cw(buf, w, h, shader, v, area);
    }
    // area == 0: degenerate (zero-area), skip
}

/// Rasterize a CCW-wound triangle (area > 0).
///
/// All three edge functions must be positive for interior pixels.
/// Tie-breaking: if bc[i] == 0, skip if edge goes left-to-right
/// (matching C++ render.cpp:478-483).
fn rasterize_ccw(
    buf: &mut [Sample],
    w: i32,
    h: i32,
    shader: &impl RasterShader,
    v: [&[i32; 4]; 3],
    area: i32,
) {
    let normalizer = (1.0_f32 - f32::EPSILON) / area as f32;

    // Bounding box clamped to buffer
    let left = 0.max(v[0][0].min(v[1][0]).min(v[2][0]));
    let right = w.min(v[0][0].max(v[1][0]).max(v[2][0]));
    let bottom = 0.max(v[0][1].min(v[1][1]).min(v[2][1]));
    let top = h.min(v[0][1].max(v[1][1]).max(v[2][1]));

    for y in bottom..top {
        for x in left..right {
            let bc0 = bc_p(v[1], v[2], x, y);
            let bc1 = bc_p(v[2], v[0], x, y);
            let bc2 = bc_p(v[0], v[1], x, y);

            // All must be non-negative for CCW interior
            if bc0 < 0 || bc1 < 0 || bc2 < 0 {
                continue;
            }

            // Tie-breaking: skip if pixel lies exactly on an edge and the
            // edge direction goes left-to-right (matching C++ render.cpp:478-483).
            if (bc0 == 0 && v[1][0] <= v[2][0])
                || (bc1 == 0 && v[2][0] <= v[0][0])
                || (bc2 == 0 && v[0][0] <= v[1][0])
            {
                continue;
            }

            // Normalize barycentrics to [0, 1]
            let nbc = [
                bc0 as f32 * normalizer,
                bc1 as f32 * normalizer,
                bc2 as f32 * normalizer,
            ];

            // Interpolate depth
            let z = nbc[0] * v[0][2] as f32 + nbc[1] * v[1][2] as f32 + nbc[2] * v[2][2] as f32;

            let idx = (w * y + x) as usize;
            shader.blend(&mut buf[idx], z, nbc);
        }
    }
}

/// Rasterize a CW-wound triangle (area < 0, double-sided only).
///
/// All three edge functions must be non-positive for interior pixels.
/// Tie-breaking mirrors CCW case (matching C++ render.cpp:534-537).
fn rasterize_cw(
    buf: &mut [Sample],
    w: i32,
    h: i32,
    shader: &impl RasterShader,
    v: [&[i32; 4]; 3],
    area: i32,
) {
    let normalizer = (1.0_f32 - f32::EPSILON) / area as f32;

    // Bounding box clamped to buffer
    let left = 0.max(v[0][0].min(v[1][0]).min(v[2][0]));
    let right = w.min(v[0][0].max(v[1][0]).max(v[2][0]));
    let bottom = 0.max(v[0][1].min(v[1][1]).min(v[2][1]));
    let top = h.min(v[0][1].max(v[1][1]).max(v[2][1]));

    for y in bottom..top {
        for x in left..right {
            let bc0 = bc_p(v[1], v[2], x, y);
            let bc1 = bc_p(v[2], v[0], x, y);
            let bc2 = bc_p(v[0], v[1], x, y);

            // All must be non-positive for CW interior
            if bc0 > 0 || bc1 > 0 || bc2 > 0 {
                continue;
            }

            // Tie-breaking (same rule as CCW, matching C++ render.cpp:534-537)
            if (bc0 == 0 && v[1][0] <= v[2][0])
                || (bc1 == 0 && v[2][0] <= v[0][0])
                || (bc2 == 0 && v[0][0] <= v[1][0])
            {
                continue;
            }

            // Normalize barycentrics (area is negative, so nbc values are positive)
            let nbc = [
                bc0 as f32 * normalizer,
                bc1 as f32 * normalizer,
                bc2 as f32 * normalizer,
            ];

            // Interpolate depth
            let z = nbc[0] * v[0][2] as f32 + nbc[1] * v[1][2] as f32 + nbc[2] * v[2][2] as f32;

            let idx = (w * y + x) as usize;
            shader.blend(&mut buf[idx], z, nbc);
        }
    }
}

// ---------------------------------------------------------------------------
// Bresenham line rasterization
// ---------------------------------------------------------------------------

/// Bresenham line rasterization in sample-buffer space.
///
/// Writes spare bit flags (e.g., 0x04 for grid lines) via OR operation
/// at depth-tested positions. Steps by 2 in horizontal domain due to
/// 2x supersampling. Matches C++ render.cpp:111-184.
///
/// # Arguments
/// * `buf` - Flat sample buffer slice (row-major, w*h elements)
/// * `w` - Buffer width in samples
/// * `h` - Buffer height in samples
/// * `from` - Start point [x, y, z] in sample-buffer integer coordinates
/// * `to` - End point [x, y, z] in sample-buffer integer coordinates
/// * `or_bits` - Spare bits to OR into each touched sample (e.g., 0x04 for grid)
pub fn bresenham(buf: &mut [Sample], w: i32, h: i32, from: [i32; 3], to: [i32; 3], or_bits: u8) {
    let sx = to[0] - from[0];
    let sy = to[1] - from[1];

    if sx == 0 && sy == 0 {
        return;
    }

    let sz = to[2] - from[2];

    let ax = sx.abs();
    let ay = sy.abs();

    // Swap so traversal goes in positive major-axis direction.
    let (from, to) = if ax >= ay {
        if from[0] > to[0] {
            (to, from)
        } else {
            (from, to)
        }
    } else if from[1] > to[1] {
        (to, from)
    } else {
        (from, to)
    };

    if ax >= ay {
        // Horizontal domain, step by 2
        let n = 1.0_f32 / sx as f32;

        // Round up start x to even alignment (align to 2x supersampling grid).
        let x0 = (0.max(from[0]) + 1) & !1;
        let x1 = w.min(to[0]);

        let mut x = x0;
        while x < x1 {
            let a = (x - from[0]) as f32 + 0.5;
            let y = (a * sy as f32 * n + from[1] as f32 + 0.5).floor() as i32;
            if y >= 0 && y < h {
                let z = a * sz as f32 * n + from[2] as f32;
                let idx = (w * y + x) as usize;
                if buf[idx].depth_test_ro(z) {
                    buf[idx].spare |= or_bits;
                }
                if buf[idx + 1].depth_test_ro(z) {
                    buf[idx + 1].spare |= or_bits;
                }
            }
            x += 2;
        }
    } else {
        // Vertical domain, step by 1
        let n = 1.0_f32 / sy as f32;

        let y0 = 0.max(from[1]);
        let y1 = h.min(to[1]);

        for y in y0..y1 {
            let a = (y - from[1]) as f32;
            let x = (a * sx as f32 * n + from[0] as f32 + 0.5).floor() as i32;
            if x >= 0 && x < w {
                let z = a * sz as f32 * n + from[2] as f32;
                let idx = (w * y + x) as usize;
                if buf[idx].depth_test_ro(z) {
                    buf[idx].spare |= or_bits;
                }
            }
        }
    }
}
