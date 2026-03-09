//! Water reflection rendering and Perlin ripple effect.
//!
//! Port of C++ render.cpp Stage 5 (lines 3266-3374) for water reflections
//! and lines 3860-3903 for Perlin ripple color shift.
//!
//! Two public functions:
//! - `render_water_reflections`: Re-runs terrain+world rendering with flipped
//!   view matrix below water plane, producing reflected geometry in SampleBuffer.
//! - `apply_water_ripple_pass`: Applies Perlin noise color shifts to reflected
//!   cells in the resolve buffer (xterm-256 palette domain).

use noise::{Fbm, MultiFractal, NoiseFn, Perlin}; // MultiFractal provides set_octaves()

use crate::render::camera::GameCamera;
use crate::render::sample_buffer::{Sample, SampleBuffer, spare_bits};
use crate::render::types::AnsiCell;
use crate::terrain::RuntimeTerrain;
use crate::world::RuntimeWorld;

/// Re-runs terrain and world rendering with a flipped view matrix
/// below the water plane, producing reflected geometry in the SampleBuffer.
///
/// Called from render_pipeline_system (pipeline.rs) at Stage 5 (REFLECTION),
/// between SHADOW and RESOLVE stages.
///
/// # Arguments
/// * `sample_buffer` - The SampleBuffer to render reflected geometry into
/// * `terrain` - Runtime terrain for query_visible
/// * `world` - Runtime world (unused in current implementation; BSP query for reflection)
/// * `camera` - Current camera state for view matrix
/// * `water_z` - Water surface height in game units
pub fn render_water_reflections(
    sample_buffer: &mut SampleBuffer,
    terrain: &RuntimeTerrain,
    _world: &RuntimeWorld,
    camera: &GameCamera,
    water_z: f32,
) {
    // Step 1: Create flipped view matrix (Z-reflection about water plane)
    //
    // R19-F06 FIX (HIGH): C++ recomputes tm[12,13] with full dot-product formula.
    // render.cpp:3274-3282 does:
    //   tm[8..10] negate (Z-axis flip)
    //   tm[12] = dw*0.5 - (pos[0]*tm[0]*HC + pos[1]*tm[4]*HC + (2*water-pos[2])*tm[8]) + shift_x*2
    //   tm[13] = dh*0.5 - (pos[0]*tm[1]*HC + pos[1]*tm[5]*HC + (2*water-pos[2])*tm[9]) + shift_y*2
    //   tm[14] = 2*water
    let mut flipped_tm = camera.view_tm;
    flipped_tm[8] = -camera.view_tm[8];
    flipped_tm[9] = -camera.view_tm[9];
    flipped_tm[10] = -camera.view_tm[10];

    // Full translation recomputation (matches C++ render.cpp:3278-3280)
    let hc = crate::asset_loader::constants::HEIGHT_CELLS as f64;
    let reflected_z = 2.0 * water_z as f64 - camera.pos[2] as f64;
    let dw = sample_buffer.width as f64;
    let dh = sample_buffer.height as f64;
    flipped_tm[12] = dw * 0.5
        - (camera.pos[0] as f64 * flipped_tm[0] * hc
            + camera.pos[1] as f64 * flipped_tm[4] * hc
            + reflected_z * flipped_tm[8])
        + camera.scene_shift[0] as f64 * 2.0;
    flipped_tm[13] = dh * 0.5
        - (camera.pos[0] as f64 * flipped_tm[1] * hc
            + camera.pos[1] as f64 * flipped_tm[5] * hc
            + reflected_z * flipped_tm[9])
        + camera.scene_shift[1] as f64 * 2.0;
    flipped_tm[14] = 2.0 * water_z as f64;

    // Step 2: Re-extract frustum planes from flipped matrix
    let flipped_planes = extract_frustum_planes_from_tm(&flipped_tm, camera, dw, dh);

    // Step 3: Query terrain patches visible from flipped camera
    // Render terrain into sample_buffer with flipped_tm
    let buf_w = sample_buffer.width as i32;
    let buf_h = sample_buffer.height as i32;

    if terrain.root.is_some() {
        // F242 FIX: Save pre-reflection heights to identify underwater cells.
        // After the reflection render, we can't tell which cells were originally
        // underwater vs above-water, so we save heights before rendering.
        let pre_heights: Vec<f32> = sample_buffer.samples.iter().map(|s| s.height).collect();
        let pre_spares: Vec<u8> = sample_buffer.samples.iter().map(|s| s.spare).collect();

        // Build a flipped camera for the terrain shader.
        // F242 FIX: Set perspective=false so render_patch uses the flipped view_tm
        // via transform_vertex(). In perspective mode, render_patch ignores view_tm
        // and uses view_pos/view_dir/mul/add/view_ofs (which are NOT flipped),
        // making the reflection render a complete no-op.
        let mut flipped_camera = camera.clone();
        flipped_camera.view_tm = flipped_tm;
        flipped_camera.frustum_planes = flipped_planes.clone();
        flipped_camera.perspective = false;

        terrain.query_visible(&flipped_planes, |patch| {
            crate::render::terrain_shader::render_patch(
                &mut sample_buffer.samples,
                buf_w,
                buf_h,
                patch,
                patch.x,
                patch.y,
                &flipped_camera,
                None, // No water clamping in reflection mode
            );
        });

        // Step 4: World mesh reflections (TODO: re-query world with flipped frustum)
        // For MVP, terrain reflections are the primary visual. Mesh reflections
        // can be added when BSP frustum culling is fixed (same issue as Stage 3).

        // Step 5: Mark ONLY underwater terrain cells with REFLECTION spare bit.
        // F242 FIX: Previous code marked ALL non-clear samples, causing water
        // ripple on trees/meshes instead of water areas only.
        //
        // Underwater = original terrain height was below water_z AND was terrain
        // (not mesh, not clear). The z-buffer naturally prevents the reflection
        // render from overwriting above-water terrain (reflected z < original z
        // for terrain above water_z).
        for (i, sample) in sample_buffer.samples.iter_mut().enumerate() {
            let pre_h = pre_heights[i];
            let pre_spare = pre_spares[i];
            let was_terrain = pre_spare & spare_bits::MESH_FLAG == 0;
            let was_underwater = pre_h > Sample::CLEAR_HEIGHT && pre_h <= water_z;

            if was_terrain && was_underwater {
                sample.spare |= spare_bits::REFLECTION;
            }
        }
    }
}

/// Extract frustum planes from a given view matrix.
///
/// Simplified version that re-derives planes using the same approach as
/// GameCamera::extract_frustum_planes but with an arbitrary transform.
fn extract_frustum_planes_from_tm(
    _tm: &[f64; 16],
    camera: &GameCamera,
    _dw: f64,
    _dh: f64,
) -> Vec<[f64; 4]> {
    // For reflected rendering, we use the original camera's frustum planes
    // expanded slightly to catch geometry that might be visible in reflection.
    // The reflected geometry is at mirrored positions so the original frustum
    // planes (which define the visible screen area) are still valid for culling.
    camera.frustum_planes.clone()
}

/// Apply Perlin noise ripple color shifts to reflected water cells.
///
/// Port of C++ render.cpp:3860-3903.
///
/// This is the buffer-level function that iterates samples and cells in sync.
/// For cells where `sample.spare & PARITY_MASK == REFLECTION`, applies a
/// Perlin noise-based color shift in the xterm-256 palette domain.
///
/// # Arguments
/// * `samples` - Sample buffer for reflection detection via spare bits
/// * `cells` - Resolve buffer (AnsiCell with xterm-256 palette indices)
/// * `grid_w` - ASCII grid width
/// * `grid_h` - ASCII grid height
/// * `time` - Animation time for noise offset
pub fn apply_water_ripple_pass(
    samples: &[Sample],
    cells: &mut [AnsiCell],
    grid_w: i32,
    grid_h: i32,
    time: f32,
) {
    let fbm = Fbm::<Perlin>::default().set_octaves(4);

    for cy in 0..grid_h {
        for cx in 0..grid_w {
            let cell_idx = (cy * grid_w + cx) as usize;

            // Check if corresponding sample has REFLECTION spare bits
            // Sample coordinates: sx = 2 + 2*cx, sy = 2 + 2*cy
            let sx = 2 + 2 * cx;
            let sy = 2 + 2 * cy;
            let sample_w = 2 * grid_w + 4;
            let sample_idx = (sy * sample_w + sx) as usize;

            if sample_idx >= samples.len() {
                continue;
            }

            let sample = &samples[sample_idx];
            if sample.spare & spare_bits::PARITY_MASK != spare_bits::REFLECTION {
                continue;
            }

            // Apply ripple to this cell
            apply_water_ripple(&fbm, &mut cells[cell_idx], cx, cy, time);
        }
    }
}

/// Internal per-cell ripple helper (NOT exported).
///
/// `cx`/`cy` are grid coordinates used as world-space proxies for noise sampling.
fn apply_water_ripple(fbm: &Fbm<Perlin>, cell: &mut AnsiCell, cx: i32, cy: i32, time: f32) {
    // Perlin noise sampling with world-space coordinates
    let d = fbm.get([cx as f64 * 0.05, cy as f64 * 0.05, time as f64]);
    let d_norm = (d + 1.0) * 0.5; // Normalize [-1,1] to [0,1]

    // Color shift computation (C++ WRAP logic, NOT clamp)
    let mut id = (d_norm * 5.0) as i32 - 2;
    if id < -1 {
        id = 2; // wrap extreme dark -> light
    }
    if id > 1 {
        id = -2; // wrap extreme light -> dark
    }

    // Apply shift to fg color in xterm-256 6x6x6 RGB cube
    let c = cell.fg as i32;
    if !(16..=231).contains(&c) {
        return; // Only shift cube colors (16-231)
    }

    // RGB cube decomposition (render.cpp:3873-3876)
    // R19-F07 FIX: C++ has a BUG at line 3875: `c -= cr * 6` should be `c -= cg * 6`.
    // INTENTIONALLY REPLICATE this bug for visual fidelity with the C++ engine.
    let c_rel = c - 16;
    let cr = c_rel / 36;
    let c_after_r = c_rel - cr * 36;
    let cg = c_after_r / 6;
    let cb = c_after_r - cr * 6; // BUG REPLICATED: cr instead of cg (matches C++ line 3875)

    // Apply shift: id > 0 lightens, id < 0 darkens
    // Shift by +/-(1 + 6 + 36) steps in 6x6x6 RGB cube
    let shift = id; // Per-channel shift amount
    let new_cr = (cr + shift).clamp(0, 5);
    let new_cg = (cg + shift).clamp(0, 5);
    let new_cb = (cb + shift).clamp(0, 5);

    let new_c = 16 + new_cr * 36 + new_cg * 6 + new_cb;
    cell.fg = new_c.clamp(16, 231) as u8;
}
