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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample_buffer::spare_bits;

    /// Create a cleared buffer of given dimensions for testing.
    fn make_buf(w: i32, h: i32) -> Vec<Sample> {
        vec![Sample::clear_state(); (w * h) as usize]
    }

    /// Test shader that writes flat color at depth-tested positions.
    struct FlatShader {
        visual: u16,
        diffuse: u8,
        spare: u8,
    }

    impl RasterShader for FlatShader {
        fn blend(&self, sample: &mut Sample, z: f32, _bc: [f32; 3]) {
            if sample.height > z || sample.height == Sample::CLEAR_HEIGHT {
                sample.visual = self.visual;
                sample.diffuse = self.diffuse;
                sample.spare = self.spare;
                sample.height = z;
            }
        }
    }

    // ---- Bresenham tests ----

    #[test]
    fn bresenham_horizontal_line() {
        let w = 24;
        let h = 10;
        let mut buf = make_buf(w, h);

        bresenham(&mut buf, w, h, [0, 5, 100], [20, 5, 100], spare_bits::GRID);

        // Samples at even x positions along y=5 should have GRID bit set
        for x in (0..20).step_by(2) {
            let idx = (w * 5 + x) as usize;
            assert!(
                buf[idx].spare & spare_bits::GRID != 0,
                "Expected GRID bit at x={x}, y=5"
            );
        }
    }

    #[test]
    fn bresenham_vertical_line() {
        let w = 10;
        let h = 24;
        let mut buf = make_buf(w, h);

        bresenham(&mut buf, w, h, [5, 0, 100], [5, 20, 100], spare_bits::GRID);

        // Samples along x=5 should have GRID bit set
        for y in 0..20 {
            let idx = (w * y + 5) as usize;
            assert!(
                buf[idx].spare & spare_bits::GRID != 0,
                "Expected GRID bit at x=5, y={y}"
            );
        }
    }

    #[test]
    fn bresenham_diagonal_line() {
        let w = 30;
        let h = 30;
        let mut buf = make_buf(w, h);

        bresenham(&mut buf, w, h, [0, 0, 100], [20, 20, 100], spare_bits::GRID);

        // At least some samples along the diagonal should have GRID bit set.
        // Since ax == ay, it goes horizontal (ax >= ay), stepping by 2.
        let mut count = 0;
        for x in (0..20).step_by(2) {
            for y in 0..20 {
                let idx = (w * y + x) as usize;
                if buf[idx].spare & spare_bits::GRID != 0 {
                    count += 1;
                }
            }
        }
        assert!(count > 0, "Expected some diagonal samples to be set");
    }

    #[test]
    fn bresenham_outside_buffer_no_panic() {
        let w = 10;
        let h = 10;
        let mut buf = make_buf(w, h);

        // Line entirely below (negative y)
        bresenham(
            &mut buf,
            w,
            h,
            [0, -10, 100],
            [10, -5, 100],
            spare_bits::GRID,
        );

        // No writes should have occurred
        for s in &buf {
            assert_eq!(
                s.spare,
                Sample::clear_state().spare,
                "No writes expected for out-of-bounds line"
            );
        }
    }

    #[test]
    fn bresenham_depth_behind_existing() {
        let w = 24;
        let h = 10;
        let mut buf = make_buf(w, h);

        // Set existing geometry at high depth (closer to camera = higher z)
        for x in 0..20 {
            let idx = (w * 5 + x) as usize;
            buf[idx].height = 5000.0;
        }

        // Draw line at z=100, which is behind z=5000 geometry
        // depth_test_ro: height(5000) <= z(100) + 8 = 108 => false
        bresenham(&mut buf, w, h, [0, 5, 100], [20, 5, 100], spare_bits::GRID);

        // Spare bits should NOT be set (depth test fails)
        for x in (0..20).step_by(2) {
            let idx = (w * 5 + x) as usize;
            assert_eq!(
                buf[idx].spare & spare_bits::GRID,
                0,
                "GRID bit should NOT be set when depth test fails at x={x}"
            );
        }
    }

    #[test]
    fn bresenham_zero_length_no_writes() {
        let w = 10;
        let h = 10;
        let mut buf = make_buf(w, h);

        bresenham(&mut buf, w, h, [5, 5, 100], [5, 5, 100], spare_bits::GRID);

        // No writes for zero-length line
        for s in &buf {
            assert_eq!(s.spare, Sample::clear_state().spare);
        }
    }

    #[test]
    fn bresenham_step_by_2_horizontal() {
        let w = 24;
        let h = 10;
        let mut buf = make_buf(w, h);

        bresenham(&mut buf, w, h, [0, 5, 100], [20, 5, 100], spare_bits::GRID);

        let clear_spare = Sample::clear_state().spare;
        for x in (0..20).step_by(2) {
            let idx = (w * 5 + x) as usize;
            assert!(
                buf[idx].spare & spare_bits::GRID != 0,
                "Even x={x} should have GRID bit"
            );
            // x+1 also gets written in horizontal mode
            assert!(
                buf[idx + 1].spare & spare_bits::GRID != 0,
                "x+1={} should have GRID bit",
                x + 1
            );
        }
        // Past the line endpoint, samples should be clear
        for x in 20..w {
            let idx = (w * 5 + x) as usize;
            assert_eq!(
                buf[idx].spare, clear_spare,
                "x={x} past endpoint should be clear"
            );
        }
    }

    // ---- Triangle rasterizer tests ----

    #[test]
    fn rasterize_small_triangle_fills_interior() {
        // A small CCW triangle in the upper-left of the buffer.
        // Vertices: (2,2), (10,2), (6,8) -- a clear triangle shape.
        let w = 16;
        let h = 16;
        let mut buf = make_buf(w, h);

        let shader = FlatShader {
            visual: 0x1234,
            diffuse: 200,
            spare: 0,
        };
        let v0: [i32; 4] = [2, 2, 100, 0];
        let v1: [i32; 4] = [10, 2, 100, 0];
        let v2: [i32; 4] = [6, 8, 100, 0];

        rasterize(&mut buf, w, h, &shader, [&v0, &v1, &v2], false);

        // Center of triangle (6, 4) should be filled
        let center = (w * 4 + 6) as usize;
        assert_eq!(buf[center].visual, 0x1234, "Center pixel should be filled");
        assert_ne!(
            buf[center].height,
            Sample::CLEAR_HEIGHT,
            "Center depth should be set"
        );
    }

    #[test]
    fn rasterize_outside_pixels_remain_clear() {
        let w = 16;
        let h = 16;
        let mut buf = make_buf(w, h);

        let shader = FlatShader {
            visual: 0x1234,
            diffuse: 200,
            spare: 0,
        };
        let v0: [i32; 4] = [2, 2, 100, 0];
        let v1: [i32; 4] = [10, 2, 100, 0];
        let v2: [i32; 4] = [6, 8, 100, 0];

        rasterize(&mut buf, w, h, &shader, [&v0, &v1, &v2], false);

        // Pixel far outside triangle (0, 15) should remain at clear state
        let outside = (w * 15 + 0) as usize;
        assert_eq!(
            buf[outside].visual,
            Sample::clear_state().visual,
            "Outside pixel should remain clear"
        );
        assert_eq!(buf[outside].height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn rasterize_double_sided_cw_winding() {
        // CW-wound triangle (vertices in clockwise order).
        // Without double_sided, nothing should be drawn.
        // With double_sided, pixels should be filled.
        let w = 16;
        let h = 16;

        let shader = FlatShader {
            visual: 0xABCD,
            diffuse: 128,
            spare: 0,
        };
        // CW: swap v1 and v2 to reverse winding
        let v0: [i32; 4] = [2, 2, 100, 0];
        let v1: [i32; 4] = [6, 8, 100, 0];
        let v2: [i32; 4] = [10, 2, 100, 0];

        // Single-sided: should NOT draw
        let mut buf_single = make_buf(w, h);
        rasterize(&mut buf_single, w, h, &shader, [&v0, &v1, &v2], false);
        let center_single = (w * 4 + 6) as usize;
        assert_eq!(
            buf_single[center_single].visual,
            Sample::clear_state().visual,
            "Single-sided CW should NOT draw"
        );

        // Double-sided: should draw
        let mut buf_double = make_buf(w, h);
        rasterize(&mut buf_double, w, h, &shader, [&v0, &v1, &v2], true);
        let center_double = (w * 4 + 6) as usize;
        assert_eq!(
            buf_double[center_double].visual, 0xABCD,
            "Double-sided CW should draw"
        );
    }

    #[test]
    fn rasterize_single_sided_cw_is_culled() {
        let w = 16;
        let h = 16;
        let mut buf = make_buf(w, h);

        let shader = FlatShader {
            visual: 0xABCD,
            diffuse: 128,
            spare: 0,
        };
        // CW winding
        let v0: [i32; 4] = [2, 2, 100, 0];
        let v1: [i32; 4] = [6, 8, 100, 0];
        let v2: [i32; 4] = [10, 2, 100, 0];

        rasterize(&mut buf, w, h, &shader, [&v0, &v1, &v2], false);

        // No pixels should be written
        for s in &buf {
            assert_eq!(s.visual, Sample::clear_state().visual);
        }
    }

    #[test]
    fn rasterize_degenerate_collinear_no_draw() {
        // Three collinear vertices: area = 0, degenerate.
        let w = 16;
        let h = 16;
        let mut buf = make_buf(w, h);

        let shader = FlatShader {
            visual: 0x1234,
            diffuse: 200,
            spare: 0,
        };
        let v0: [i32; 4] = [2, 2, 100, 0];
        let v1: [i32; 4] = [6, 6, 100, 0];
        let v2: [i32; 4] = [10, 10, 100, 0];

        rasterize(&mut buf, w, h, &shader, [&v0, &v1, &v2], true);

        // No pixels should be written (degenerate)
        for s in &buf {
            assert_eq!(s.visual, Sample::clear_state().visual);
        }
    }

    #[test]
    fn rasterize_frustum_culled_all_share_cull_bit() {
        // All 3 vertices have cull_flags bit 1 set -> frustum culled.
        let w = 16;
        let h = 16;
        let mut buf = make_buf(w, h);

        let shader = FlatShader {
            visual: 0x1234,
            diffuse: 200,
            spare: 0,
        };
        let v0: [i32; 4] = [2, 2, 100, 0x01];
        let v1: [i32; 4] = [10, 2, 100, 0x01];
        let v2: [i32; 4] = [6, 8, 100, 0x01];

        rasterize(&mut buf, w, h, &shader, [&v0, &v1, &v2], false);

        // No pixels should be written (frustum culled)
        for s in &buf {
            assert_eq!(s.visual, Sample::clear_state().visual);
        }
    }

    #[test]
    fn rasterize_depth_test_closer_overwrites() {
        let w = 16;
        let h = 16;

        // First: draw a far triangle at z=200
        let far_shader = FlatShader {
            visual: 0x1111,
            diffuse: 100,
            spare: 0,
        };
        let v0: [i32; 4] = [2, 2, 200, 0];
        let v1: [i32; 4] = [10, 2, 200, 0];
        let v2: [i32; 4] = [6, 8, 200, 0];

        let mut buf = make_buf(w, h);
        rasterize(&mut buf, w, h, &far_shader, [&v0, &v1, &v2], false);

        // Then: draw a closer triangle at z=50 (same shape)
        let near_shader = FlatShader {
            visual: 0x2222,
            diffuse: 200,
            spare: 0,
        };
        let n0: [i32; 4] = [2, 2, 50, 0];
        let n1: [i32; 4] = [10, 2, 50, 0];
        let n2: [i32; 4] = [6, 8, 50, 0];

        rasterize(&mut buf, w, h, &near_shader, [&n0, &n1, &n2], false);

        // Center pixel should show the closer (near) triangle
        let center = (w * 4 + 6) as usize;
        assert_eq!(
            buf[center].visual, 0x2222,
            "Closer triangle should overwrite farther one"
        );
    }

    #[test]
    fn rasterize_adjacent_triangles_no_double_draw() {
        // Two triangles sharing edge from (5,2) to (5,8).
        // Tie-breaking should ensure each shared-edge pixel is drawn by exactly
        // one triangle.
        let w = 16;
        let h = 16;

        // Counter to track how many times each pixel is blended.
        // We'll use visual field as a counter.
        struct CountShader;
        impl RasterShader for CountShader {
            fn blend(&self, sample: &mut Sample, _z: f32, _bc: [f32; 3]) {
                // Increment visual as a draw counter
                sample.visual = sample.visual.wrapping_add(1);
            }
        }

        let mut buf = make_buf(w, h);
        // Reset visual to 0 for counting
        for s in buf.iter_mut() {
            s.visual = 0;
        }

        let shader = CountShader;

        // Left triangle (CCW): (1,2), (5,2), (5,8)
        let l0: [i32; 4] = [1, 2, 100, 0];
        let l1: [i32; 4] = [5, 2, 100, 0];
        let l2: [i32; 4] = [5, 8, 100, 0];

        // Right triangle (CCW): (5,2), (9,2), (5,8)
        let r0: [i32; 4] = [5, 2, 100, 0];
        let r1: [i32; 4] = [9, 2, 100, 0];
        let r2: [i32; 4] = [5, 8, 100, 0];

        rasterize(&mut buf, w, h, &shader, [&l0, &l1, &l2], false);
        rasterize(&mut buf, w, h, &shader, [&r0, &r1, &r2], false);

        // Check shared edge pixels (x=5, y in range): each should be drawn
        // exactly once (not 0, not 2)
        let mut any_on_edge_drawn = false;
        for y in 3..8 {
            let idx = (w * y + 5) as usize;
            let count = buf[idx].visual;
            assert!(
                count <= 1,
                "Shared edge pixel at (5,{y}) drawn {count} times (expected 0 or 1)"
            );
            if count == 1 {
                any_on_edge_drawn = true;
            }
        }
        // At least one pixel on/near the shared edge should be drawn by one triangle
        assert!(
            any_on_edge_drawn,
            "Expected at least one pixel near shared edge to be drawn"
        );
    }
}
