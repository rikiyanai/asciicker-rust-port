//! Shared math utilities for the render pipeline.
//!
//! Contains `transform_vertex` used by both terrain and mesh shaders
//! to project world-space points through a 4x4 row-major matrix.

/// Transform a world-space point through a 4x4 row-major matrix and return
/// integer sample-buffer coordinates `[x, y, z, cull_flags]`.
///
/// Matrix layout is ROW-MAJOR (matching C++ render.cpp):
///   `out[i*4+j] = sum_k(A[i*4+k] * B[k*4+j])`
/// Element access: `screen_x = tm[0]*wx + tm[4]*wy + tm[8]*wz + tm[12]`, etc.
///
/// # Arguments
/// * `world` - World-space position `[x, y, z]` as `f64`
/// * `view_tm` - 4x4 row-major view matrix
///
/// # Returns
/// `[x, y, z, cull_flags]` in sample-buffer integer coordinates.
/// `cull_flags`: bit 0 = left of buffer, bit 1 = right, bit 2 = below, bit 3 = above.
pub fn transform_vertex(world: [f64; 3], view_tm: &[f64; 16]) -> [i32; 4] {
    let wx = world[0];
    let wy = world[1];
    let wz = world[2];

    // Row-major multiply: column vectors are at indices [0,4,8,12], [1,5,9,13], etc.
    let sx = view_tm[0] * wx + view_tm[4] * wy + view_tm[8] * wz + view_tm[12];
    let sy = view_tm[1] * wx + view_tm[5] * wy + view_tm[9] * wz + view_tm[13];
    let sz = view_tm[2] * wx + view_tm[6] * wy + view_tm[10] * wz + view_tm[14];

    // Convert to integer sample-buffer coordinates
    let ix = sx.floor() as i32;
    let iy = sy.floor() as i32;
    let iz = sz.floor() as i32;

    // Cull flags: conservative screen-space rejection
    // Bit 0: left of screen (x < -256)
    // Bit 1: right of screen (x > some large value) - use generous margin
    // Bit 2: below screen (y < -256)
    // Bit 3: above screen (y > some large value)
    // We use large margins since the actual buffer dimensions aren't passed here;
    // the rasterizer's bounding box clamp handles precise clipping.
    let mut cull = 0i32;
    if ix < -256 {
        cull |= 1;
    }
    if ix > 4096 {
        cull |= 2;
    }
    if iy < -256 {
        cull |= 4;
    }
    if iy > 4096 {
        cull |= 8;
    }

    [ix, iy, iz, cull]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_identity_matrix() {
        let identity: [f64; 16] = [
            1.0, 0.0, 0.0, 0.0, // col 0 (row-major layout)
            0.0, 1.0, 0.0, 0.0, // col 1
            0.0, 0.0, 1.0, 0.0, // col 2
            0.0, 0.0, 0.0, 1.0, // col 3 (translation)
        ];
        let result = transform_vertex([10.0, 20.0, 30.0], &identity);
        assert_eq!(result[0], 10);
        assert_eq!(result[1], 20);
        assert_eq!(result[2], 30);
        assert_eq!(result[3], 0); // no cull flags
    }

    #[test]
    fn transform_with_translation() {
        let mut tm = [0.0f64; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = 100.0; // translate x by 100
        tm[13] = 200.0; // translate y by 200
        tm[14] = 50.0; // translate z by 50

        let result = transform_vertex([5.0, 10.0, 15.0], &tm);
        assert_eq!(result[0], 105);
        assert_eq!(result[1], 210);
        assert_eq!(result[2], 65);
        assert_eq!(result[3], 0);
    }

    #[test]
    fn transform_with_scale() {
        let mut tm = [0.0f64; 16];
        tm[0] = 2.0; // scale x by 2
        tm[5] = 3.0; // scale y by 3
        tm[10] = 1.0;
        tm[15] = 1.0;

        let result = transform_vertex([10.0, 20.0, 5.0], &tm);
        assert_eq!(result[0], 20);
        assert_eq!(result[1], 60);
        assert_eq!(result[2], 5);
    }

    #[test]
    fn transform_cull_flags_left() {
        let mut tm = [0.0f64; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = -500.0;

        let result = transform_vertex([0.0, 0.0, 0.0], &tm);
        assert!(result[3] & 1 != 0, "Should have left cull flag");
    }

    #[test]
    fn transform_cull_flags_right() {
        let mut tm = [0.0f64; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = 5000.0;

        let result = transform_vertex([0.0, 0.0, 0.0], &tm);
        assert!(result[3] & 2 != 0, "Should have right cull flag");
    }

    #[test]
    fn transform_no_cull_in_bounds() {
        let mut tm = [0.0f64; 16];
        tm[0] = 1.0;
        tm[5] = 1.0;
        tm[10] = 1.0;
        tm[15] = 1.0;
        tm[12] = 100.0;
        tm[13] = 100.0;

        let result = transform_vertex([0.0, 0.0, 0.0], &tm);
        assert_eq!(result[3], 0, "Should have no cull flags when in bounds");
    }
}
