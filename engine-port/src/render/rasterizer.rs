use super::sample_buffer::Sample;

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
pub fn bresenham(
    buf: &mut [Sample],
    w: i32,
    h: i32,
    from: [i32; 3],
    to: [i32; 3],
    or_bits: u8,
) {
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

        bresenham(
            &mut buf,
            w,
            h,
            [0, 0, 100],
            [20, 20, 100],
            spare_bits::GRID,
        );

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

        bresenham(
            &mut buf,
            w,
            h,
            [5, 5, 100],
            [5, 5, 100],
            spare_bits::GRID,
        );

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

        // In horizontal mode, only even-x positions (and x+1) are written.
        // Odd-x positions that aren't x+1 of an even position should remain clear.
        // The stepping is: x=0,2,4,6,8,10,12,14,16,18 and each writes x and x+1.
        // So positions 0,1,2,3,...,19 are all written. Let's verify the stepping
        // by checking that the writes happen at x0 aligned to even.
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
}
