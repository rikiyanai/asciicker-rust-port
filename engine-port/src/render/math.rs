//! Shared math utilities for the render pipeline.
//!
//! Contains `transform_vertex` (base affine matrix multiply) and
//! `transform_vertex_perspective` (architectural perspective with 1/dist scaling)
//! used by terrain and mesh shaders to project world-space points into
//! sample-buffer coordinates.

use super::camera::GameCamera;

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

/// Transform a world-space point with architectural perspective projection.
///
/// Ports C++ render.cpp:1804-1846. Uses the camera's view direction and focal
/// length for depth-dependent scaling (closer = larger, farther = smaller).
///
/// Returns `None` if the vertex is behind the camera.
/// Returns `Some([x, y, z, cull_flags])` in sample-buffer integer coordinates.
pub fn transform_vertex_perspective(
    world: [f64; 3],
    camera: &GameCamera,
    buf_w: i32,
    buf_h: i32,
) -> Option<[i32; 4]> {
    let wx = world[0];
    let wy = world[1];
    let wz = world[2];

    // Eye-to-vertex vector in world units
    let eye_x = wx as f32 - camera.view_pos[0];
    let eye_y = wy as f32 - camera.view_pos[1];
    let eye_z = wz as f32 - camera.view_pos[2];

    // Distance along view direction (view_dir is normalized by focal)
    let viewer_dist =
        eye_x * camera.view_dir[0] + eye_y * camera.view_dir[1] + eye_z * camera.view_dir[2];

    if viewer_dist <= 0.0 {
        return None; // behind camera
    }

    let recp_dist = 1.0 / viewer_dist;

    // Base screen position WITHOUT translation (from affine view matrix components)
    let fx = (camera.mul[0] * wx + camera.mul[2] * wy) as f32 * recp_dist;
    let fy = (camera.mul[1] * wx + camera.mul[3] * wy + camera.mul[5] * wz) as f32 * recp_dist;

    // Apply translated offset with perspective
    let qx = (camera.add[0] as f32 - camera.view_ofs[0]) * recp_dist + camera.view_ofs[0];
    let qy = (camera.add[1] as f32 - camera.view_ofs[1]) * recp_dist + camera.view_ofs[1];

    let sx = fx + qx;
    let sy = fy + qy;

    let ix = (sx + 0.5).floor() as i32;
    let iy = (sy + 0.5).floor() as i32;
    let iz = wz.floor() as i32;

    let mut cull = 0i32;
    if ix < 0 {
        cull |= 1;
    }
    if ix > buf_w {
        cull |= 2;
    }
    if iy < 0 {
        cull |= 4;
    }
    if iy > buf_h {
        cull |= 8;
    }

    Some([ix, iy, iz, cull])
}
