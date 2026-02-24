//! Mesh shader: rasterizes mesh instance triangles into SampleBuffer.
//!
//! Ports the C++ `RenderFace` logic. MeshShader implements `RasterShader`
//! and writes RGB555 colors with `spare = MESH_FLAG` so that the resolve
//! stage takes the auto_mat path.

use crate::asset_loader::akm_mesh::AkmMesh;
use crate::asset_loader::constants::HEIGHT_SCALE;
use crate::render::camera::GameCamera;
use crate::render::math::transform_vertex_perspective;
use crate::render::quantize::{pack_rgb555, rgb8_to_rgb5};
use crate::render::rasterizer::{RasterShader, rasterize};
use crate::render::sample_buffer::{Sample, spare_bits};

/// Mesh shader implementing `RasterShader` for mesh face triangles.
///
/// Writes packed RGB555 color into `sample.visual` and sets
/// `spare = MESH_FLAG`, directing the resolve stage to the auto_mat path.
pub struct MeshShader {
    /// Per-face color packed as RGB555.
    pub rgb555: u16,
    /// Lighting intensity.
    pub diffuse: u8,
}

impl RasterShader for MeshShader {
    fn blend(&self, sample: &mut Sample, z: f32, _bc: [f32; 3]) {
        // Depth test: LARGER z = closer/on top (Z-up world; higher objects occlude lower).
        // C++ render.cpp uses `if(sam.z < z)` — write if new fragment is higher.
        if sample.height < z || sample.height == Sample::CLEAR_HEIGHT {
            sample.visual = self.rgb555;
            sample.diffuse = self.diffuse;
            sample.spare = spare_bits::MESH_FLAG;
            sample.height = z;
        }
    }
}

/// Compute per-face diffuse lighting intensity (0-255).
///
/// Ports C++ render.cpp:1115-1146:
/// 1. Cross product of model-space edges → face normal
/// 2. Transform normal by instance matrix (rotation part)
/// 3. Divide z by HEIGHT_SCALE (world z is scaled)
/// 4. Normalize
/// 5. Lambertian n·l
/// 6. Ambient blend + 0.5 bias, clamp [0,1]
fn compute_face_diffuse(
    v0: [f64; 3],
    v1: [f64; 3],
    v2: [f64; 3],
    instance_tm: &[f64; 16],
    light_dir: [f32; 3],
    light_ambient: f32,
) -> u8 {
    // Edge vectors in model space
    let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    // Cross product → model-space normal
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];

    // Transform normal by instance matrix (3x3 rotation part, w=0)
    // C++ uses Product(r->inst_tm, n, inst_n) with n[3]=0
    let inx = instance_tm[0] * nx + instance_tm[4] * ny + instance_tm[8] * nz;
    let iny = instance_tm[1] * nx + instance_tm[5] * ny + instance_tm[9] * nz;
    let mut inz = instance_tm[2] * nx + instance_tm[6] * ny + instance_tm[10] * nz;

    // C++ render.cpp:1130 — scale z component by HEIGHT_SCALE
    inz /= HEIGHT_SCALE as f64;

    // Normalize
    let len = (inx * inx + iny * iny + inz * inz).sqrt();
    if len < 1e-12 {
        return 128; // degenerate normal → neutral lighting
    }
    let inv_len = 1.0 / len;

    // Lambertian n·l (C++ render.cpp:1134)
    let mut df = inv_len
        * (inx * light_dir[0] as f64
            + iny * light_dir[1] as f64
            + inz * light_dir[2] as f64);

    // Ambient blend (C++ render.cpp:1138)
    let amb = light_ambient as f64;
    df = df * (1.0 - 0.5 * amb) + 0.5 * amb;

    // Bias (C++ render.cpp:1139)
    df += 0.5;

    // Clamp [0, 1] (C++ render.cpp:1141-1144)
    df = df.clamp(0.0, 1.0);

    // Convert to u8 (C++ render.cpp:1146)
    (df * 255.0) as u8
}

/// Multiply a model-space point by an instance transform matrix (row-major).
///
/// Returns world-space `[f64; 3]`.
fn apply_instance_tm(model: [f64; 3], instance_tm: &[f64; 16]) -> [f64; 3] {
    let mx = model[0];
    let my = model[1];
    let mz = model[2];

    let wx = instance_tm[0] * mx + instance_tm[4] * my + instance_tm[8] * mz + instance_tm[12];
    let wy = instance_tm[1] * mx + instance_tm[5] * my + instance_tm[9] * mz + instance_tm[13];
    let wz = instance_tm[2] * mx + instance_tm[6] * my + instance_tm[10] * mz + instance_tm[14];

    [wx, wy, wz]
}

/// Rasterize a mesh instance into the sample buffer.
///
/// For each face in the mesh, transforms vertices from model space to world
/// space (via `instance_tm`), then to screen space (via camera projection),
/// and calls `rasterize()`.
///
/// Uses architectural perspective projection matching C++ render.cpp:1804-1846.
///
/// # Arguments
/// * `buf` - Flat sample buffer slice
/// * `buf_w` - SAMPLE buffer width
/// * `buf_h` - SAMPLE buffer height
/// * `mesh` - Parsed AKM mesh geometry
/// * `instance_tm` - Instance transform matrix (model -> world)
/// * `camera` - Camera with view matrix and perspective parameters
pub fn render_mesh(
    buf: &mut [Sample],
    buf_w: i32,
    buf_h: i32,
    mesh: &AkmMesh,
    instance_tm: &[f64; 16],
    camera: &GameCamera,
) {
    for face in &mesh.faces {
        let i0 = face.indices[0] as usize;
        let i1 = face.indices[1] as usize;
        let i2 = face.indices[2] as usize;

        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue; // skip invalid face indices
        }

        let vert0 = &mesh.vertices[i0];
        let vert1 = &mesh.vertices[i1];
        let vert2 = &mesh.vertices[i2];

        // Model -> World (instance_tm)
        let w0 = apply_instance_tm(
            [vert0.x as f64, vert0.y as f64, vert0.z as f64],
            instance_tm,
        );
        let w1 = apply_instance_tm(
            [vert1.x as f64, vert1.y as f64, vert1.z as f64],
            instance_tm,
        );
        let w2 = apply_instance_tm(
            [vert2.x as f64, vert2.y as f64, vert2.z as f64],
            instance_tm,
        );

        // World -> Screen (architectural perspective)
        let sv0 = match transform_vertex_perspective(w0, camera, buf_w, buf_h) {
            Some(v) => v,
            None => continue,
        };
        let sv1 = match transform_vertex_perspective(w1, camera, buf_w, buf_h) {
            Some(v) => v,
            None => continue,
        };
        let sv2 = match transform_vertex_perspective(w2, camera, buf_w, buf_h) {
            Some(v) => v,
            None => continue,
        };

        // Average vertex colors for face color
        let avg_r = ((vert0.r as u16 + vert1.r as u16 + vert2.r as u16) / 3) as u8;
        let avg_g = ((vert0.g as u16 + vert1.g as u16 + vert2.g as u16) / 3) as u8;
        let avg_b = ((vert0.b as u16 + vert1.b as u16 + vert2.b as u16) / 3) as u8;

        let rgb555 = pack_rgb555(rgb8_to_rgb5(avg_r), rgb8_to_rgb5(avg_g), rgb8_to_rgb5(avg_b));

        // Face normal lighting (C++ render.cpp:1115-1146).
        let diffuse = compute_face_diffuse(
            [vert0.x as f64, vert0.y as f64, vert0.z as f64],
            [vert1.x as f64, vert1.y as f64, vert1.z as f64],
            [vert2.x as f64, vert2.y as f64, vert2.z as f64],
            instance_tm,
            camera.light_dir,
            camera.light_ambient,
        );

        let shader = MeshShader { rgb555, diffuse };

        // Double-sided for meshes
        rasterize(buf, buf_w, buf_h, &shader, [&sv0, &sv1, &sv2], true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sample_buffer::spare_bits;

    #[test]
    fn test_mesh_shader_writes_rgb555() {
        let mut sample = Sample::clear_state();
        let shader = MeshShader {
            rgb555: 0x1234,
            diffuse: 200,
        };
        shader.blend(&mut sample, 100.0, [0.33, 0.33, 0.34]);

        assert_eq!(sample.visual, 0x1234, "Should write RGB555 color");
        assert_eq!(sample.diffuse, 200, "Should write diffuse");
        assert_eq!(
            sample.spare,
            spare_bits::MESH_FLAG,
            "Mesh must have MESH_FLAG set"
        );
        assert_eq!(sample.height, 100.0, "Should write depth");
    }

    #[test]
    fn test_mesh_shader_depth_test() {
        // Write an initial fragment (via CLEAR_HEIGHT path)
        let mut sample = Sample::clear_state();
        let shader_low = MeshShader {
            rgb555: 0x1111,
            diffuse: 200,
        };
        shader_low.blend(&mut sample, 50.0, [0.33, 0.33, 0.34]);
        assert_eq!(sample.visual, 0x1111);

        // Write a HIGHER z fragment -- SHOULD overwrite (higher = on top in Z-up)
        let shader_high = MeshShader {
            rgb555: 0x2222,
            diffuse: 100,
        };
        shader_high.blend(&mut sample, 200.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 0x2222,
            "Higher z fragment should overwrite lower (on top in Z-up)"
        );

        // Write a LOWER z fragment -- should NOT overwrite
        let shader_below = MeshShader {
            rgb555: 0x3333,
            diffuse: 255,
        };
        shader_below.blend(&mut sample, 25.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 0x2222,
            "Lower z fragment should not overwrite higher (underneath in Z-up)"
        );
    }
}
