//! Mesh shader: rasterizes mesh instance triangles into SampleBuffer.
//!
//! Ports the C++ `RenderFace` logic. MeshShader implements `RasterShader`
//! and writes RGB555 colors with `spare = MESH_FLAG` so that the resolve
//! stage takes the auto_mat path.

use crate::asset_loader::akm_mesh::AkmMesh;
use crate::render::math::transform_vertex;
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
        // Depth test: SMALLER z = closer to camera (wins depth test).
        // Inline pattern -- do NOT use depth_test_ro() which has semantic inversion.
        if sample.height > z || sample.height == Sample::CLEAR_HEIGHT {
            sample.visual = self.rgb555;
            sample.diffuse = self.diffuse;
            sample.spare = spare_bits::MESH_FLAG;
            sample.height = z;
        }
    }
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
/// space (via `instance_tm`), then to screen space (via `view_tm`), and
/// calls `rasterize()`.
///
/// # Arguments
/// * `buf` - Flat sample buffer slice
/// * `buf_w` - SAMPLE buffer width
/// * `buf_h` - SAMPLE buffer height
/// * `mesh` - Parsed AKM mesh geometry
/// * `instance_tm` - Instance transform matrix (model -> world)
/// * `view_tm` - Camera view matrix (world -> screen)
pub fn render_mesh(
    buf: &mut [Sample],
    buf_w: i32,
    buf_h: i32,
    mesh: &AkmMesh,
    instance_tm: &[f64; 16],
    view_tm: &[f64; 16],
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

        // Model -> World (instance_tm) -> Screen (view_tm)
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

        let sv0 = transform_vertex(w0, view_tm);
        let sv1 = transform_vertex(w1, view_tm);
        let sv2 = transform_vertex(w2, view_tm);

        // Average vertex colors for face color
        let avg_r = ((vert0.r as u16 + vert1.r as u16 + vert2.r as u16) / 3) as u8;
        let avg_g = ((vert0.g as u16 + vert1.g as u16 + vert2.g as u16) / 3) as u8;
        let avg_b = ((vert0.b as u16 + vert1.b as u16 + vert2.b as u16) / 3) as u8;

        let rgb555 = pack_rgb555(rgb8_to_rgb5(avg_r), rgb8_to_rgb5(avg_g), rgb8_to_rgb5(avg_b));

        // Simplified diffuse: full brightness (0xFF).
        // A proper n.l lighting computation would use face normals.
        let diffuse = 0xFF_u8;

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
        // Write a near fragment
        let mut sample = Sample::clear_state();
        let shader_near = MeshShader {
            rgb555: 0x1111,
            diffuse: 200,
        };
        shader_near.blend(&mut sample, 50.0, [0.33, 0.33, 0.34]);
        assert_eq!(sample.visual, 0x1111);

        // Try to write a farther fragment -- should NOT overwrite
        let shader_far = MeshShader {
            rgb555: 0x2222,
            diffuse: 100,
        };
        shader_far.blend(&mut sample, 200.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 0x1111,
            "Farther fragment should not overwrite closer"
        );

        // Write a closer fragment -- SHOULD overwrite
        let shader_closer = MeshShader {
            rgb555: 0x3333,
            diffuse: 255,
        };
        shader_closer.blend(&mut sample, 25.0, [0.33, 0.33, 0.34]);
        assert_eq!(
            sample.visual, 0x3333,
            "Closer fragment should overwrite farther"
        );
    }
}
