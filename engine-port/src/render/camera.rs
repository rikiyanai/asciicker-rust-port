//! Perspective camera system for the CPU rasterizer.
//!
//! Ports the view matrix construction from C++ render.cpp:2966-3034.
//! Provides `GameCamera` resource with view matrix, frustum planes, and input systems.
//!
//! # Cast Boundary (P5-126)
//! `GameCamera.pos` is `[f32; 3]` for input/physics compatibility.
//! When passing `camera_pos` to `RuntimeWorld::query_visible` or `query_world_frustum`
//! (which require `[f64; 3]`), cast at the call site:
//! ```ignore
//! let camera_pos: [f64; 3] = camera.pos.map(|x| x as f64);
//! ```

use bevy::prelude::*;

use crate::asset_loader::constants::{HEIGHT_CELLS, HEIGHT_SCALE, VISUAL_CELLS};

use super::config::RenderConfig;

/// Scale factor for DBL mode (always active — we use 2x supersampling).
/// In C++: `#ifdef DBL float scale = 3.0; #else float scale = 1.5; #endif`
const DBL_SCALE: f32 = 3.0;

/// sin(30 degrees) = 0.5, used for isometric projection tilt.
const SIN30: f64 = 0.5;

/// cos(30 degrees), used for isometric height scaling.
const COS30: f64 = 0.866_025_403_784_438_6;

/// Camera resource encapsulating position, orientation, and derived view state.
///
/// The view matrix and frustum planes are recomputed each frame by `camera_update_system`.
#[derive(Resource, Debug, Clone)]
pub struct GameCamera {
    // --- Input state (set by input systems or physics) ---
    /// World-space position `[x, y, z]`.
    /// Note: `[f32; 3]` for physics compatibility. Cast to `[f64; 3]` at query call sites.
    pub pos: [f32; 3],

    /// Yaw angle in degrees. 0 = north, increases counter-clockwise.
    pub yaw: f32,

    /// Zoom level. 1.0 = default. Higher = closer.
    pub zoom: f32,

    /// Whether perspective (architectural) mode is active.
    /// If false, uses isometric projection.
    pub perspective: bool,

    /// Scene shift in ASCII cells (screen shake). Multiplied by 2 in sample space (TRAP-R06).
    pub scene_shift: [i32; 2],

    // --- Derived state (recomputed each frame by `update`) ---
    /// Focal length for perspective mode. `max(dw, dh) * 2.0`.
    pub focal: f32,

    /// 4x4 column-major view matrix (world-to-screen transform).
    /// This is a 2D affine transform, NOT a clip-space projection.
    pub view_tm: [f64; 16],

    /// Inverse of `view_tm` for screen-to-world unprojection.
    pub inv_tm: [f64; 16],

    /// View direction (unit-ish vector, normalized by focal). For perspective mode.
    /// Architectural projection: horizontal only (view_dir[2] == 0).
    pub view_dir: [f32; 3],

    /// View position in world-space (height-cell units). For perspective mode.
    pub view_pos: [f32; 3],

    /// View offset: screen center + scene_shift*2. For perspective mode.
    pub view_ofs: [f32; 2],

    /// Extracted frustum planes `[a, b, c, d]` where `ax + by + cz + d >= 0` is inside.
    /// At least 4 planes (left, right, top, bottom). No near/far for isometric.
    pub frustum_planes: Vec<[f64; 4]>,

    /// The `mul[6]` array from C++ (3x2 rotation part), stored for use by terrain/world queries.
    pub mul: [f64; 6],

    /// The `add[3]` translation offset from C++.
    pub add: [f64; 3],
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            yaw: 0.0,
            zoom: 1.0,
            perspective: true,
            scene_shift: [0, 0],
            focal: 0.0,
            view_tm: [0.0; 16],
            inv_tm: [0.0; 16],
            view_dir: [0.0, 0.0, 0.0],
            view_pos: [0.0, 0.0, 0.0],
            view_ofs: [0.0, 0.0],
            frustum_planes: Vec::new(),
            mul: [0.0; 6],
            add: [0.0; 3],
        }
    }
}

impl GameCamera {
    /// Recompute the view matrix and perspective parameters from current input state.
    ///
    /// `dw` and `dh` are SAMPLE buffer dimensions (e.g., 484x274), NOT ASCII dimensions.
    /// This ports C++ render.cpp:2966-3034.
    pub fn update(&mut self, dw: f64, dh: f64) {
        let zoom_scaled = self.zoom * DBL_SCALE;
        let ds = 2.0 * zoom_scaled as f64 / VISUAL_CELLS as f64;

        let a = (self.yaw as f64) * std::f64::consts::PI / 180.0;
        let sinyaw = a.sin();
        let cosyaw = a.cos();

        // Build isometric view matrix (C++ render.cpp:2971-2988)
        let mut tm = [0.0f64; 16];
        tm[0] = cosyaw * ds;
        tm[1] = -sinyaw * SIN30 * ds;
        tm[2] = 0.0;
        tm[3] = 0.0;
        tm[4] = sinyaw * ds;
        tm[5] = cosyaw * SIN30 * ds;
        tm[6] = 0.0;
        tm[7] = 0.0;
        tm[8] = 0.0;
        tm[9] = COS30 / HEIGHT_SCALE as f64 * ds * HEIGHT_CELLS as f64;
        tm[10] = 1.0;
        tm[11] = 0.0;

        // Translation: center on camera position with scene_shift * 2 (TRAP-R06)
        let hc = HEIGHT_CELLS as f64;
        tm[12] = dw * 0.5
            - (self.pos[0] as f64 * tm[0] * hc
                + self.pos[1] as f64 * tm[4] * hc
                + self.pos[2] as f64 * tm[8])
            + self.scene_shift[0] as f64 * 2.0;
        tm[13] = dh * 0.5
            - (self.pos[0] as f64 * tm[1] * hc
                + self.pos[1] as f64 * tm[5] * hc
                + self.pos[2] as f64 * tm[9])
            + self.scene_shift[1] as f64 * 2.0;
        tm[14] = 0.0;
        tm[15] = 1.0;

        // Store mul/add for terrain/world query compatibility
        self.mul[0] = tm[0];
        self.mul[1] = tm[1];
        self.mul[2] = tm[4];
        self.mul[3] = tm[5];
        self.mul[4] = 0.0;
        self.mul[5] = tm[9];

        self.add[0] = tm[12];
        self.add[1] = tm[13] + 0.5; // C++ adds 0.5 rounding offset
        self.add[2] = tm[14];

        self.view_tm = tm;

        // Compute inverse view matrix
        self.inv_tm = invert_4x4(&tm);

        // Perspective parameters (C++ render.cpp:3021-3034)
        // "sin/cos 30 are commented out to achieve 'architectural' perspective"
        self.focal = (dw.max(dh) as f32) * 2.0;

        // view_dir: horizontal only (architectural — no vertical tilt)
        self.view_dir[0] = -sinyaw as f32;
        self.view_dir[1] = cosyaw as f32;
        self.view_dir[2] = 0.0;

        // view_pos: camera position backed up along view_dir by focal length
        self.view_pos[0] = HEIGHT_CELLS as f32 * self.pos[0] - self.view_dir[0] * self.focal;
        self.view_pos[1] = HEIGHT_CELLS as f32 * self.pos[1] - self.view_dir[1] * self.focal;
        self.view_pos[2] = self.pos[2];

        // Normalize view_dir by focal (C++ divides after computing view_pos)
        self.view_dir[0] /= self.focal;
        self.view_dir[1] /= self.focal;

        // view_ofs: screen center with scene_shift * 2 (TRAP-R06)
        self.view_ofs[0] = (dw as f32) / 2.0 + self.scene_shift[0] as f32 * 2.0;
        self.view_ofs[1] = (dh as f32) / 2.0 + self.scene_shift[1] as f32 * 2.0;
    }

    /// Extract frustum planes from the current camera state.
    ///
    /// For perspective mode: derives planes from focus_node and screen corners
    /// transformed through inv(view_tm), matching C++ render.cpp:3065-3136.
    ///
    /// For isometric mode: derives planes from a normalized clip-space transform,
    /// matching C++ render.cpp:3137-3163.
    ///
    /// Each plane `[a, b, c, d]` satisfies: `ax + by + cz + d >= 0` means inside.
    pub fn extract_frustum_planes(&mut self, dw: f64, dh: f64) {
        self.frustum_planes.clear();

        let a = (self.yaw as f64) * std::f64::consts::PI / 180.0;
        let sinyaw = a.sin();
        let cosyaw = a.cos();

        if self.perspective {
            self.extract_perspective_frustum(dw, dh, sinyaw, cosyaw);
        } else {
            self.extract_isometric_frustum(dw, dh, sinyaw, cosyaw);
        }
    }

    /// Perspective frustum: transform screen corners through inv(view_tm),
    /// intersect with neutral plane, build planes from focus_node + corners.
    /// Ports C++ render.cpp:3065-3136.
    fn extract_perspective_frustum(&mut self, dw: f64, dh: f64, sinyaw: f64, cosyaw: f64) {
        let hc = HEIGHT_CELLS as f64;

        // Focus node (C++ render.cpp:3057-3062)
        let focus_node = [
            self.pos[0] as f64 + sinyaw * self.focal as f64 / hc,
            self.pos[1] as f64 - cosyaw * self.focal as f64 / hc,
            self.pos[2] as f64 + SIN30 * self.focal as f64 / hc * HEIGHT_SCALE as f64,
        ];

        // Neutral plane: the camera's horizontal plane through pos
        let neutral_plane = [
            -sinyaw,
            cosyaw,
            0.0,
            sinyaw * self.pos[0] as f64 - cosyaw * self.pos[1] as f64,
        ];

        // Screen corners at z=0 and z=10
        let screen_corners_0 = [
            [0.0, 0.0, 0.0, 1.0],
            [dw, 0.0, 0.0, 1.0],
            [0.0, dh, 0.0, 1.0],
            [dw, dh, 0.0, 1.0],
        ];
        let screen_corners_1 = [
            [0.0, 0.0, 10.0, 1.0],
            [dw, 0.0, 10.0, 1.0],
            [0.0, dh, 10.0, 1.0],
            [dw, dh, 10.0, 1.0],
        ];

        let clip_tm = invert_4x4(&self.view_tm);

        let mut world_corners = [[0.0f64; 3]; 4];
        for c in 0..4 {
            // Transform corners from screen to world
            let mut w0 = mat4_mul_vec4(&clip_tm, &screen_corners_0[c]);
            let mut w1 = mat4_mul_vec4(&clip_tm, &screen_corners_1[c]);

            // Divide by HEIGHT_CELLS (premultiplied -> world)
            w0[0] /= hc;
            w0[1] /= hc;
            w1[0] /= hc;
            w1[1] /= hc;

            // Compute ray direction
            let dir = [w1[0] - w0[0], w1[1] - w0[1], w1[2] - w0[2]];

            // Intersect with neutral_plane
            let dot_origin = neutral_plane[0] * w0[0]
                + neutral_plane[1] * w0[1]
                + neutral_plane[2] * w0[2]
                + neutral_plane[3];
            let dot_dir =
                neutral_plane[0] * dir[0] + neutral_plane[1] * dir[1] + neutral_plane[2] * dir[2];

            let t = if dot_dir.abs() > 1e-12 {
                -dot_origin / dot_dir
            } else {
                0.0
            };

            world_corners[c] = [w0[0] + t * dir[0], w0[1] + t * dir[1], w0[2] + t * dir[2]];
        }

        // Build frustum planes from focus_node and corners
        // C++ naming: ll=0, lr=1, ul=2, ur=3
        let corner_ll = world_corners[0];
        let corner_lr = world_corners[1];
        let corner_ul = world_corners[2];
        let corner_ur = world_corners[3];

        // left  (focus, ll, ul)
        self.frustum_planes
            .push(plane_from_points(&focus_node, &corner_ll, &corner_ul));
        // right (focus, ur, lr)
        self.frustum_planes
            .push(plane_from_points(&focus_node, &corner_ur, &corner_lr));
        // top   (focus, ul, ur)
        self.frustum_planes
            .push(plane_from_points(&focus_node, &corner_ul, &corner_ur));
        // bottom(focus, lr, ll)
        self.frustum_planes
            .push(plane_from_points(&focus_node, &corner_lr, &corner_ll));
    }

    /// Isometric frustum: build a normalized clip transform, then extract planes
    /// via TransposeProduct. Ports C++ render.cpp:3137-3163.
    fn extract_isometric_frustum(&mut self, dw: f64, dh: f64, sinyaw: f64, cosyaw: f64) {
        let zoom_scaled = self.zoom * DBL_SCALE;
        let ds = 2.0 * zoom_scaled as f64 / VISUAL_CELLS as f64;
        let hc = HEIGHT_CELLS as f64;
        let half_dw = 0.5 * dw;
        let half_dh = 0.5 * dh;

        // Build the normalized clip-space transform (C++ render.cpp:3142-3157)
        let mut clip_tm = [0.0f64; 16];
        clip_tm[0] = cosyaw / half_dw * ds * hc;
        clip_tm[1] = -sinyaw * SIN30 / half_dh * ds * hc;
        clip_tm[2] = 0.0;
        clip_tm[3] = 0.0;
        clip_tm[4] = sinyaw / half_dw * ds * hc;
        clip_tm[5] = cosyaw * SIN30 / half_dh * ds * hc;
        clip_tm[6] = 0.0;
        clip_tm[7] = 0.0;
        clip_tm[8] = 0.0;
        clip_tm[9] = COS30 / HEIGHT_SCALE as f64 / half_dh * ds * hc;
        clip_tm[10] = 2.0 / 65535.0;
        clip_tm[11] = 0.0;
        clip_tm[12] = -(self.pos[0] as f64 * clip_tm[0]
            + self.pos[1] as f64 * clip_tm[4]
            + self.pos[2] as f64 * clip_tm[8]
            - self.scene_shift[0] as f64 * 2.0 / (dw / 2.0)); // width in C++ is ASCII width = dw/2 approx
        clip_tm[13] = -(self.pos[0] as f64 * clip_tm[1]
            + self.pos[1] as f64 * clip_tm[5]
            + self.pos[2] as f64 * clip_tm[9]
            - self.scene_shift[1] as f64 * 2.0 / (dh / 2.0));
        clip_tm[14] = -1.0;
        clip_tm[15] = 1.0;

        // Clip planes in normalized device coords
        let clip_left = [1.0, 0.0, 0.0, 1.0];
        let clip_right = [-1.0, 0.0, 0.0, 1.0];
        let clip_bottom = [0.0, 1.0, 0.0, 1.0];
        let clip_top = [0.0, -1.0, 0.0, 1.0];

        // TransposeProduct: clip_world = clip_tm^T * clip_plane
        self.frustum_planes
            .push(normalize_plane(transpose_product_4x4(&clip_tm, &clip_left)));
        self.frustum_planes
            .push(normalize_plane(transpose_product_4x4(
                &clip_tm,
                &clip_right,
            )));
        self.frustum_planes
            .push(normalize_plane(transpose_product_4x4(
                &clip_tm,
                &clip_bottom,
            )));
        self.frustum_planes
            .push(normalize_plane(transpose_product_4x4(&clip_tm, &clip_top)));
    }
}

// --- Bevy Systems ---

/// Temporary input system for camera navigation.
///
/// TODO(Phase 6): Q/E will be replaced with physics-driven rotation via PhysicsIO.torque.
/// These direct yaw modifications are TEMPORARY for Phase 5 camera navigation only.
pub fn camera_input_system(mut camera: ResMut<GameCamera>, keyboard: Res<ButtonInput<KeyCode>>) {
    // Q/E rotation (temporary — Phase 6 replaces with PhysicsIO.torque)
    if keyboard.just_pressed(KeyCode::KeyQ) {
        camera.yaw -= 45.0;
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        camera.yaw += 45.0;
    }

    // Basic WASD movement (speed = 0.5 per frame)
    let speed = 0.5f32;
    if keyboard.pressed(KeyCode::KeyW) {
        camera.pos[1] += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        camera.pos[1] -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        camera.pos[0] -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        camera.pos[0] += speed;
    }
}

/// System that recomputes the view matrix and frustum planes each frame.
pub fn camera_update_system(mut camera: ResMut<GameCamera>, config: Res<RenderConfig>) {
    let dw = config.sample_width() as f64;
    let dh = config.sample_height() as f64;
    camera.update(dw, dh);
    camera.extract_frustum_planes(dw, dh);
}

// --- Linear Algebra Helpers ---

/// Invert a 4x4 column-major matrix. Returns the inverse.
/// Uses the adjugate/cofactor method.
fn invert_4x4(m: &[f64; 16]) -> [f64; 16] {
    // Compute cofactors for a general 4x4 matrix
    let s0 = m[0] * m[5] - m[4] * m[1];
    let s1 = m[0] * m[6] - m[4] * m[2];
    let s2 = m[0] * m[7] - m[4] * m[3];
    let s3 = m[1] * m[6] - m[5] * m[2];
    let s4 = m[1] * m[7] - m[5] * m[3];
    let s5 = m[2] * m[7] - m[6] * m[3];

    let c5 = m[10] * m[15] - m[14] * m[11];
    let c4 = m[9] * m[15] - m[13] * m[11];
    let c3 = m[9] * m[14] - m[13] * m[10];
    let c2 = m[8] * m[15] - m[12] * m[11];
    let c1 = m[8] * m[14] - m[12] * m[10];
    let c0 = m[8] * m[13] - m[12] * m[9];

    let det = s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0;
    if det.abs() < 1e-30 {
        return [0.0; 16]; // singular matrix
    }
    let inv_det = 1.0 / det;

    let mut inv = [0.0f64; 16];
    inv[0] = (m[5] * c5 - m[6] * c4 + m[7] * c3) * inv_det;
    inv[1] = (-m[1] * c5 + m[2] * c4 - m[3] * c3) * inv_det;
    inv[2] = (m[13] * s5 - m[14] * s4 + m[15] * s3) * inv_det;
    inv[3] = (-m[9] * s5 + m[10] * s4 - m[11] * s3) * inv_det;

    inv[4] = (-m[4] * c5 + m[6] * c2 - m[7] * c1) * inv_det;
    inv[5] = (m[0] * c5 - m[2] * c2 + m[3] * c1) * inv_det;
    inv[6] = (-m[12] * s5 + m[14] * s2 - m[15] * s1) * inv_det;
    inv[7] = (m[8] * s5 - m[10] * s2 + m[11] * s1) * inv_det;

    inv[8] = (m[4] * c4 - m[5] * c2 + m[7] * c0) * inv_det;
    inv[9] = (-m[0] * c4 + m[1] * c2 - m[3] * c0) * inv_det;
    inv[10] = (m[12] * s4 - m[13] * s2 + m[15] * s0) * inv_det;
    inv[11] = (-m[8] * s4 + m[9] * s2 - m[11] * s0) * inv_det;

    inv[12] = (-m[4] * c3 + m[5] * c1 - m[6] * c0) * inv_det;
    inv[13] = (m[0] * c3 - m[1] * c1 + m[2] * c0) * inv_det;
    inv[14] = (-m[12] * s3 + m[13] * s1 - m[14] * s0) * inv_det;
    inv[15] = (m[8] * s3 - m[9] * s1 + m[10] * s0) * inv_det;

    inv
}

/// Multiply a 4x4 column-major matrix by a 4-vector.
fn mat4_mul_vec4(m: &[f64; 16], v: &[f64; 4]) -> [f64; 4] {
    [
        m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12] * v[3],
        m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13] * v[3],
        m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14] * v[3],
        m[3] * v[0] + m[7] * v[1] + m[11] * v[2] + m[15] * v[3],
    ]
}

/// Compute `M^T * v` for a 4x4 column-major matrix and a 4-vector.
/// This is the TransposeProduct from C++: each output component is a row of M dotted with v.
fn transpose_product_4x4(m: &[f64; 16], v: &[f64; 4]) -> [f64; 4] {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2] + m[3] * v[3],
        m[4] * v[0] + m[5] * v[1] + m[6] * v[2] + m[7] * v[3],
        m[8] * v[0] + m[9] * v[1] + m[10] * v[2] + m[11] * v[3],
        m[12] * v[0] + m[13] * v[1] + m[14] * v[2] + m[15] * v[3],
    ]
}

/// Compute a plane `[a, b, c, d]` from three points using cross product.
/// The plane normal is `(p1 - p0) x (p2 - p0)`, normalized.
/// `d = -dot(normal, p0)`.
fn plane_from_points(p0: &[f64; 3], p1: &[f64; 3], p2: &[f64; 3]) -> [f64; 4] {
    let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

    let nx = u[1] * v[2] - u[2] * v[1];
    let ny = u[2] * v[0] - u[0] * v[2];
    let nz = u[0] * v[1] - u[1] * v[0];

    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-30 {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let a = nx / len;
    let b = ny / len;
    let c = nz / len;
    let d = -(a * p0[0] + b * p0[1] + c * p0[2]);

    [a, b, c, d]
}

/// Normalize a plane `[a, b, c, d]` so that `sqrt(a^2+b^2+c^2) == 1`.
fn normalize_plane(p: [f64; 4]) -> [f64; 4] {
    let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    if len < 1e-30 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    [p[0] / len, p[1] / len, p[2] / len, p[3] / len]
}

/// Test whether a point is inside all frustum planes.
/// Returns true if `dot(plane_normal, point) + d >= 0` for every plane.
pub fn point_inside_frustum(planes: &[[f64; 4]], point: &[f64; 3]) -> bool {
    planes
        .iter()
        .all(|p| p[0] * point[0] + p[1] * point[1] + p[2] * point[2] + p[3] >= 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a camera and update it at default config dimensions.
    fn make_camera(yaw: f32, pos: [f32; 3], perspective: bool) -> GameCamera {
        let mut cam = GameCamera {
            yaw,
            pos,
            perspective,
            ..Default::default()
        };
        // Default config: 240x135 ASCII -> 484x274 sample
        cam.update(484.0, 274.0);
        cam.extract_frustum_planes(484.0, 274.0);
        cam
    }

    #[test]
    fn test_yaw_zero_view_matrix() {
        let cam = make_camera(0.0, [0.0, 0.0, 0.0], false);
        // At yaw=0: cosyaw=1, sinyaw=0
        // zoom=1.0, scale=3.0, ds = 2*3.0/8 = 0.75
        let ds = 2.0 * 3.0 / 8.0;
        // tm[0] = cos(0)*ds = ds
        assert!(
            (cam.view_tm[0] - ds).abs() < 1e-10,
            "tm[0] should be ds={ds}"
        );
        // tm[1] = -sin(0)*sin30*ds = 0
        assert!(
            cam.view_tm[1].abs() < 1e-10,
            "tm[1] should be 0, got {}",
            cam.view_tm[1]
        );
        // tm[4] = sin(0)*ds = 0
        assert!(
            cam.view_tm[4].abs() < 1e-10,
            "tm[4] should be 0, got {}",
            cam.view_tm[4]
        );
        // tm[5] = cos(0)*sin30*ds = 0.5*ds
        let expected_5 = SIN30 * ds;
        assert!(
            (cam.view_tm[5] - expected_5).abs() < 1e-10,
            "tm[5] should be {expected_5}, got {}",
            cam.view_tm[5]
        );
    }

    #[test]
    fn test_scene_shift_doubled() {
        let mut cam = GameCamera {
            scene_shift: [5, 3],
            ..Default::default()
        };
        cam.update(484.0, 274.0);

        // view_tm[12] should include scene_shift[0]*2 = 10
        let mut cam_no_shift = GameCamera::default();
        cam_no_shift.update(484.0, 274.0);

        let diff_x = cam.view_tm[12] - cam_no_shift.view_tm[12];
        assert!(
            (diff_x - 10.0).abs() < 1e-10,
            "view_tm[12] shift diff should be 10.0, got {diff_x}"
        );

        let diff_y = cam.view_tm[13] - cam_no_shift.view_tm[13];
        assert!(
            (diff_y - 6.0).abs() < 1e-10,
            "view_tm[13] shift diff should be 6.0, got {diff_y}"
        );

        // view_ofs should also include scene_shift * 2
        assert!(
            (cam.view_ofs[0] - (484.0 / 2.0 + 10.0)).abs() < 1e-5,
            "view_ofs[0] should be 242+10=252"
        );
        assert!(
            (cam.view_ofs[1] - (274.0 / 2.0 + 6.0)).abs() < 1e-5,
            "view_ofs[1] should be 137+6=143"
        );
    }

    #[test]
    fn test_perspective_focal_length() {
        let cam = make_camera(0.0, [0.0, 0.0, 0.0], true);
        // focal = max(484, 274) * 2 = 968
        assert!(
            (cam.focal - 968.0).abs() < 1e-5,
            "focal should be 968, got {}",
            cam.focal
        );
    }

    #[test]
    fn test_perspective_view_dir_horizontal() {
        let cam = make_camera(45.0, [10.0, 20.0, 5.0], true);
        assert!(
            cam.view_dir[2].abs() < 1e-10,
            "view_dir[2] should be 0 (no vertical tilt), got {}",
            cam.view_dir[2]
        );
    }

    #[test]
    fn test_frustum_planes_count() {
        let cam_iso = make_camera(0.0, [0.0, 0.0, 0.0], false);
        assert!(
            cam_iso.frustum_planes.len() >= 4,
            "Isometric should have at least 4 planes, got {}",
            cam_iso.frustum_planes.len()
        );

        let cam_persp = make_camera(0.0, [0.0, 0.0, 0.0], true);
        assert!(
            cam_persp.frustum_planes.len() >= 4,
            "Perspective should have at least 4 planes, got {}",
            cam_persp.frustum_planes.len()
        );
    }

    #[test]
    fn test_frustum_planes_deterministic() {
        let cam1 = make_camera(30.0, [5.0, 5.0, 1.0], true);
        let cam2 = make_camera(30.0, [5.0, 5.0, 1.0], true);

        assert_eq!(
            cam1.frustum_planes.len(),
            cam2.frustum_planes.len(),
            "Plane count should be identical"
        );
        for (i, (p1, p2)) in cam1
            .frustum_planes
            .iter()
            .zip(cam2.frustum_planes.iter())
            .enumerate()
        {
            for j in 0..4 {
                assert!(
                    (p1[j] - p2[j]).abs() < 1e-15,
                    "Plane {i} component {j} differs: {} vs {}",
                    p1[j],
                    p2[j]
                );
            }
        }
    }

    #[test]
    fn test_frustum_planes_axis_aligned_camera() {
        // R6-007: At yaw=0, pos=[0,0,0], zoom=1.0, isometric mode:
        // left/right planes should have normal.x != 0,
        // top/bottom planes should have normal.y != 0.
        let cam = make_camera(0.0, [0.0, 0.0, 0.0], false);
        assert!(cam.frustum_planes.len() >= 4);

        // Planes 0,1 are left/right
        assert!(
            cam.frustum_planes[0][0].abs() > 0.01,
            "Left plane normal.x should be nonzero"
        );
        assert!(
            cam.frustum_planes[1][0].abs() > 0.01,
            "Right plane normal.x should be nonzero"
        );

        // Planes 2,3 are bottom/top
        assert!(
            cam.frustum_planes[2][1].abs() > 0.01,
            "Bottom plane normal.y should be nonzero"
        );
        assert!(
            cam.frustum_planes[3][1].abs() > 0.01,
            "Top plane normal.y should be nonzero"
        );
    }

    #[test]
    fn test_frustum_planes_known_point_inside() {
        // A point at camera origin should be inside all frustum planes
        let cam = make_camera(0.0, [5.0, 5.0, 0.0], true);
        let origin = [5.0, 5.0, 0.0];
        assert!(
            point_inside_frustum(&cam.frustum_planes, &origin),
            "Camera origin should be inside all frustum planes"
        );
    }

    #[test]
    fn test_frustum_planes_far_point_outside() {
        // A point far behind the camera should be outside at least one plane
        let cam = make_camera(0.0, [0.0, 0.0, 0.0], true);
        // At yaw=0, view_dir is (0, 1, 0). A point far in -Y should be behind.
        let behind = [0.0, -10000.0, 0.0];
        assert!(
            !point_inside_frustum(&cam.frustum_planes, &behind),
            "Point far behind camera should be outside frustum"
        );
    }

    #[test]
    fn test_q_decrements_yaw_by_45() {
        // R16-F187: Verify Q press decrements yaw by 45 degrees
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameCamera>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, camera_input_system);

        // Simulate Q key press
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyQ);
        }
        app.update();

        let camera = app.world().get_resource::<GameCamera>().unwrap();
        assert!(
            (camera.yaw - (-45.0)).abs() < 1e-5,
            "After Q press, yaw should be -45, got {}",
            camera.yaw
        );
    }

    #[test]
    fn test_e_increments_yaw_by_45() {
        // R16-F187: Verify E press increments yaw by 45 degrees
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameCamera>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, camera_input_system);

        // Simulate E key press
        {
            let mut input = app
                .world_mut()
                .get_resource_mut::<ButtonInput<KeyCode>>()
                .unwrap();
            input.press(KeyCode::KeyE);
        }
        app.update();

        let camera = app.world().get_resource::<GameCamera>().unwrap();
        assert!(
            (camera.yaw - 45.0).abs() < 1e-5,
            "After E press, yaw should be 45, got {}",
            camera.yaw
        );
    }

    #[test]
    fn test_invert_identity() {
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let inv = invert_4x4(&identity);
        for i in 0..16 {
            assert!(
                (inv[i] - identity[i]).abs() < 1e-10,
                "Identity inverse[{i}] should be identity, got {}",
                inv[i]
            );
        }
    }
}
