//! Collision soup: triangle items and sphere-space transformation.
//!
//! `SoupItem` holds a single collision triangle with its plane normal and
//! material index. `to_sphere_space` transforms a position into the scaled
//! coordinate system where the physics sphere has unit radius.

/// A single collision triangle in sphere space.
///
/// Collected from terrain and world geometry for collision testing.
/// The `nrm` field is `[nx, ny, nz, w]` defining the triangle's plane
/// equation: `nx*x + ny*y + nz*z + w = 0`.
#[derive(Debug, Clone)]
pub struct SoupItem {
    /// Triangle vertices `[[x,y,z]; 3]` in sphere space.
    pub tri: [[f32; 3]; 3],
    /// Material index (terrain vmap material during collect; mesh material = 0).
    pub material: i32,
    /// Plane equation `[nx, ny, nz, w]` where `dot(nrm, p) + w = 0`.
    pub nrm: [f32; 4],
}

/// Transform a world-space position to sphere space.
///
/// TRAP-P03: XY components are scaled by `mul_xy`, Z is scaled by `mul_z`.
/// The result is relative to `center` (sphere center in world space).
///
/// In sphere space, the physics sphere has unit radius (1.0).
pub fn to_sphere_space(pos: &[f32; 3], center: &[f32; 3], mul_xy: f32, mul_z: f32) -> [f32; 3] {
    [
        (pos[0] - center[0]) * mul_xy,
        (pos[1] - center[1]) * mul_xy,
        (pos[2] - center[2]) * mul_z,
    ]
}
