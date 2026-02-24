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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_sphere_space_identity() {
        // Same position as center with 1.0 multipliers -> origin
        let pos = [10.0, 20.0, 30.0];
        let center = [10.0, 20.0, 30.0];
        let result = to_sphere_space(&pos, &center, 1.0, 1.0);
        assert_eq!(result, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_to_sphere_space_offset() {
        let pos = [12.0, 25.0, 35.0];
        let center = [10.0, 20.0, 30.0];
        let result = to_sphere_space(&pos, &center, 1.0, 1.0);
        assert!((result[0] - 2.0).abs() < 1e-6);
        assert!((result[1] - 5.0).abs() < 1e-6);
        assert!((result[2] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_to_sphere_space_scaling() {
        // TRAP-P03: XY scaled differently from Z
        let pos = [12.0, 25.0, 35.0];
        let center = [10.0, 20.0, 30.0];
        let mul_xy = 0.5;
        let mul_z = 0.25;
        let result = to_sphere_space(&pos, &center, mul_xy, mul_z);
        assert!((result[0] - 1.0).abs() < 1e-6); // 2.0 * 0.5
        assert!((result[1] - 2.5).abs() < 1e-6); // 5.0 * 0.5
        assert!((result[2] - 1.25).abs() < 1e-6); // 5.0 * 0.25
    }

    #[test]
    fn test_to_sphere_space_human_entity() {
        // Realistic: world_radius=1.333, world_height=86.2
        // mul_xy = 1/1.333 ~= 0.75, mul_z = 2/86.2 ~= 0.0232
        let pos = [100.0, 200.0, 10.0];
        let center = [99.0, 200.0, 10.0];
        let mul_xy = 1.0 / 1.333;
        let mul_z = 2.0 / 86.2;
        let result = to_sphere_space(&pos, &center, mul_xy, mul_z);
        assert!((result[0] - 0.75019).abs() < 0.001);
        assert!(result[1].abs() < 1e-6);
        assert!(result[2].abs() < 1e-6);
    }

    #[test]
    fn test_soup_item_creation() {
        let item = SoupItem {
            tri: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            material: 3,
            nrm: [0.0, 0.0, 1.0, 0.0],
        };
        assert_eq!(item.material, 3);
        assert_eq!(item.nrm[2], 1.0);
    }
}
