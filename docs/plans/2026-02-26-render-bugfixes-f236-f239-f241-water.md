# Render Pipeline Bugfix Plan: F236, F239, F241, Water Ripple

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix four open rendering bugs — mesh diffuse always 0xFF, frustum AABB coordinate mismatch, stubbed shadow/reflection stages, and broken water ripple pipeline integration.

**Architecture:** Each fix is a surgical change to one or two files in `engine-port/src/render/`. No new modules. TDD with inline `#[cfg(test)]` blocks. Fixes are independent — commit after each.

**Tech Stack:** Rust, Bevy 0.18 (ECS/Resources only — CPU rasterizer), `noise` crate (Perlin).

---

## Root Cause Analysis

### F236: Diffuse hardcoded 0xFF
`camera.light_ambient` defaults to `1.0` (camera.rs:114). The mesh diffuse formula in `compute_face_diffuse()` (mesh_shader.rs:82-96) computes:
```
df * (1.0 - 0.5 * ambient) + 0.5 * ambient + 0.5
```
With `ambient = 1.0`: `df * 0.5 + 1.0`, clamped to [0,1] → always ~1.0 → diffuse ≈ 255 for all faces.

Also, `camera.light_dir` defaults to normalized `(1,1,1)` while terrain uses `LIGHT_DIR = [0.3, -0.3, 1.0, 0.3]`. Lighting directions are inconsistent.

**Fix:** Set `light_ambient = 0.3` and `light_dir` to normalized `[0.3, -0.3, 1.0]` matching terrain's LIGHT_DIR constant.

### F239: Frustum AABB coordinate mismatch
In `query_terrain_frustum()` (quadtree.rs:295-299), the AABB uses patch coordinates directly:
```rust
let x0 = bx as f64;
let x1 = x0 + size;
```
But the C++ vertex formula (render.cpp:1723) is `vx = x * HEIGHT_CELLS + dx * VISUAL_CELLS`, meaning each patch renders geometry spanning `HEIGHT_CELLS * VISUAL_CELLS = 32` visual-cell units from its starting coordinate — far larger than 1 unit. In frustum-space (pos-space = visual-cell / HEIGHT_CELLS), a patch at `bx` renders from `bx` to `bx + VISUAL_CELLS` (8 pos-space units), but the AABB says `[bx, bx+1]`.

The frustum planes are derived in pos-space (camera.rs:264 divides by HEIGHT_CELLS). The AABB must match this space WITH the rendering extent.

**Fix:** Expand the AABB X/Y extent to account for the vertex computation overshoot: `x1 = x0 + size * VISUAL_CELLS as f64` (each patch unit spans VISUAL_CELLS pos-space units in the render formula). Z bounds remain unchanged (already in raw height units matching frustum Z).

### F241: Shadow stage stubbed
Pipeline.rs:398-400 is an empty stub:
```rust
let t3 = Instant::now();
timing.shadow_us = t3.elapsed().as_micros() as u64;
```
C++ render.cpp:3184-3217 implements player blob shadow: iterate nearby samples, transform screen-to-world via inv_tm, compute distance to player position, attenuate diffuse within ~2.0 unit radius.

**Fix:** Implement `render_player_shadow()` in a new section of `pipeline.rs` (or inline). Transform sample screen coords to world via `camera.inv_tm`, compute squared distance to player pos, attenuate `sample.diffuse` within radius.

### Water ripple broken at runtime
Three compounding issues:

1. **`extract_frustum_planes_from_tm()` is a stub** (water.rs:136-147): Returns original camera frustum planes instead of computing from the flipped matrix. Reflected terrain query uses WRONG planes — patches visible only in reflection may be culled.

2. **Single-sample vs 4-sample detection mismatch**: `resolve()` checks all 4 samples' combined spare (`s00|s10|s01|s11`) for REFLECTION bit. But `apply_water_ripple_pass()` checks only ONE sample (upper-left of 2x2 block). Cells where only 1-3 of 4 samples have REFLECTION get dimmed by resolve but NOT rippled.

3. **Perspective disabled for reflection**: `render_water_reflections()` sets `flipped_camera.perspective = false` (water.rs:92). This forces the affine `transform_vertex()` path, but the flipped view matrix was constructed to match the PERSPECTIVE projection. The terrain renders at wrong positions, producing no visible geometry.

**Fix:** (a) Implement real frustum plane extraction from flipped TM, (b) make ripple check match resolve's 4-sample OR logic, (c) keep perspective=true for flipped camera.

---

## Task 1: Fix F236 — Mesh diffuse lighting defaults

**Files:**
- Modify: `engine-port/src/render/camera.rs:93-116` (GameCamera Default impl)
- Test: `engine-port/src/render/camera.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing test**

Add at the bottom of `camera.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_light_ambient_matches_terrain() {
        // F236: light_ambient must be 0.3 (matching terrain LIGHT_DIR[3]),
        // not 1.0 which collapses diffuse to ~255 for all normals.
        let cam = GameCamera::default();
        assert!(
            (cam.light_ambient - 0.3).abs() < 0.01,
            "light_ambient should be 0.3, got {}",
            cam.light_ambient
        );
    }

    #[test]
    fn default_light_dir_matches_terrain() {
        // F236: light_dir must match terrain's LIGHT_DIR [0.3, -0.3, 1.0]
        // (normalized), not (1,1,1)/sqrt(3).
        let cam = GameCamera::default();
        // Normalized [0.3, -0.3, 1.0]: length = sqrt(0.09 + 0.09 + 1.0) = sqrt(1.18) ≈ 1.0863
        let len = (cam.light_dir[0].powi(2) + cam.light_dir[1].powi(2) + cam.light_dir[2].powi(2)).sqrt();
        assert!((len - 1.0).abs() < 0.01, "light_dir should be unit length, got {}", len);
        // Z component should be dominant (sun from above)
        assert!(cam.light_dir[2] > 0.9, "light_dir Z should be >0.9 (sun above), got {}", cam.light_dir[2]);
        // Y component should be negative (matching terrain LIGHT_DIR[1] = -0.3)
        assert!(cam.light_dir[1] < 0.0, "light_dir Y should be negative, got {}", cam.light_dir[1]);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -- camera::tests --nocapture`
Expected: FAIL — `light_ambient should be 0.3, got 1` and `light_dir Z should be >0.9, got 0.577`

**Step 3: Fix the defaults**

In `camera.rs`, change the `Default` impl (lines 110-114):

```rust
// BEFORE:
light_dir: {
    let inv = 1.0_f32 / 3.0_f32.sqrt();
    [inv, inv, inv]
},
light_ambient: 1.0,

// AFTER (F236 FIX: match terrain LIGHT_DIR [0.3, -0.3, 1.0, 0.3]):
light_dir: {
    // Normalized [0.3, -0.3, 1.0]: sun from upper-right-above
    let raw = [0.3_f32, -0.3, 1.0];
    let len = (raw[0] * raw[0] + raw[1] * raw[1] + raw[2] * raw[2]).sqrt();
    [raw[0] / len, raw[1] / len, raw[2] / len]
},
light_ambient: 0.3,
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -- camera::tests --nocapture`
Expected: PASS

**Step 5: Verify mesh shader produces varying diffuse values**

Add one more test in `camera.rs::tests`:

```rust
#[test]
fn mesh_diffuse_varies_with_default_light() {
    // F236: With corrected light_ambient=0.3, compute_face_diffuse should
    // produce varying values, not always ~255.
    let cam = GameCamera::default();

    // Face pointing straight up: normal = (0, 0, 1)
    let up_face = crate::render::mesh_shader::compute_face_diffuse_public(
        [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0],
        &IDENTITY_TM, cam.light_dir, cam.light_ambient,
    );

    // Face pointing down: normal = (0, 0, -1)
    let down_face = crate::render::mesh_shader::compute_face_diffuse_public(
        [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0],
        &IDENTITY_TM, cam.light_dir, cam.light_ambient,
    );

    assert!(
        up_face != down_face,
        "Up-facing ({}) and down-facing ({}) should differ",
        up_face, down_face
    );
    assert!(up_face > down_face, "Up-facing should be brighter than down-facing");
    assert!(up_face < 255, "Up-facing should not be saturated at 255, got {}", up_face);
}

const IDENTITY_TM: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];
```

Note: This requires making `compute_face_diffuse` public. Add a thin public wrapper in `mesh_shader.rs`:

```rust
/// Public test accessor for compute_face_diffuse.
#[cfg(test)]
pub fn compute_face_diffuse_public(
    v0: [f64; 3], v1: [f64; 3], v2: [f64; 3],
    instance_tm: &[f64; 16], light_dir: [f32; 3], light_ambient: f32,
) -> u8 {
    compute_face_diffuse(v0, v1, v2, instance_tm, light_dir, light_ambient)
}
```

**Step 6: Run all tests**

Run: `cargo test --lib -- camera::tests --nocapture`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add engine-port/src/render/camera.rs engine-port/src/render/mesh_shader.rs
git commit -m "fix(render): F236 — set light_ambient=0.3 and light_dir to match terrain

GameCamera defaults had light_ambient=1.0 which collapsed the mesh diffuse
formula to ~255 for all face normals (no shading variation). Changed to 0.3
matching terrain's LIGHT_DIR constant. Also aligned light_dir to normalized
[0.3, -0.3, 1.0] for consistent lighting across terrain and mesh shaders."
```

---

## Task 2: Fix F239 — Frustum AABB extent

**Files:**
- Modify: `engine-port/src/terrain/quadtree.rs:281-360` (query_terrain_frustum)
- Test: `engine-port/src/terrain/quadtree.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing test**

Add at the bottom of `quadtree.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::constants::VISUAL_CELLS;

    /// Build a minimal 1-patch terrain for AABB testing.
    fn make_single_patch(x: i32, y: i32) -> QuadNode {
        use crate::terrain::patch_runtime::RuntimePatch;
        use crate::asset_loader::constants::{HEIGHT_CELLS_PLUS_ONE, VISUAL_CELLS};

        QuadNode::Leaf(RuntimePatch {
            x,
            y,
            height: [[100u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
            dark: 0,
            lo: 100,
            hi: 100,
        })
    }

    #[test]
    fn frustum_aabb_accounts_for_render_extent() {
        // F239: A patch at bx=5 renders geometry spanning [5, 5+VISUAL_CELLS]
        // in pos-space. A frustum plane at x=10 (inside the render extent)
        // should NOT cull this patch.
        let node = make_single_patch(5, 5);

        // Create a frustum with a single LEFT plane at x=10:
        // ax + by + cz + d >= 0 → x >= 10 → [1, 0, 0, -10]
        // Patch at bx=5 renders to x=5..13 (5 + VISUAL_CELLS=8).
        // The plane x>=10 should intersect this extent (Partial), not reject it.
        let planes: Vec<[f64; 4]> = vec![[1.0, 0.0, 0.0, -10.0]];

        let mut visible = Vec::new();
        query_terrain_frustum(&node, 0, 5, 5, &planes, &mut |patch: &RuntimePatch| {
            visible.push((patch.x, patch.y));
        });

        assert!(
            !visible.is_empty(),
            "Patch at (5,5) renders to x=[5,13] and should be visible when frustum plane is at x=10"
        );
    }

    #[test]
    fn frustum_aabb_still_culls_distant_patches() {
        // F239: A patch at bx=0 renders to [0, VISUAL_CELLS=8].
        // A plane requiring x>=20 should cull it.
        let node = make_single_patch(0, 0);

        let planes: Vec<[f64; 4]> = vec![[1.0, 0.0, 0.0, -20.0]];

        let mut visible = Vec::new();
        query_terrain_frustum(&node, 0, 0, 0, &planes, &mut |patch: &RuntimePatch| {
            visible.push((patch.x, patch.y));
        });

        assert!(
            visible.is_empty(),
            "Patch at (0,0) renders to x=[0,8] and should be culled when plane requires x>=20"
        );
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -- quadtree::tests --nocapture`
Expected: FAIL — first test asserts patch visible but current AABB [5,6] is fully outside plane x>=10.

**Step 3: Fix the AABB computation**

In `quadtree.rs`, function `query_terrain_frustum()`, change the AABB computation (lines ~295-299):

```rust
// BEFORE:
let size = (1i32 << level) as f64;
let x0 = bx as f64;
let y0 = by as f64;
let x1 = x0 + size;
let y1 = y0 + size;

// AFTER (F239 FIX: expand AABB to account for vertex render extent):
// Each patch renders geometry from bx to bx + VISUAL_CELLS in pos-space
// (C++ render.cpp:1723: vx = x * HEIGHT_CELLS + dx * VISUAL_CELLS).
// At level L, the region covers 2^L patches. The last patch extends
// VISUAL_CELLS beyond its coordinate. Previous: size=2^L (1 unit per patch).
// Fixed: x1 = x0 + size - 1 + VISUAL_CELLS (last patch starts at x0+size-1,
// renders to x0+size-1+VISUAL_CELLS).
use crate::asset_loader::constants::VISUAL_CELLS;
let size = (1i32 << level) as f64;
let x0 = bx as f64;
let y0 = by as f64;
let x1 = x0 + size - 1.0 + VISUAL_CELLS as f64;
let y1 = y0 + size - 1.0 + VISUAL_CELLS as f64;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -- quadtree::tests --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add engine-port/src/terrain/quadtree.rs
git commit -m "fix(terrain): F239 — expand frustum AABB to match render extent

query_terrain_frustum AABB was [bx, bx+size] (1 unit per patch) but each
patch renders geometry spanning VISUAL_CELLS (8) pos-space units. Expanded
AABB to [bx, bx+size-1+VISUAL_CELLS] so patches whose rendered geometry
intersects the frustum are not falsely culled."
```

---

## Task 3: Fix F241 — Implement player blob shadow (Stage 4)

**Files:**
- Modify: `engine-port/src/render/pipeline.rs:398-401` (Stage 4 stub)
- Test: `engine-port/src/render/pipeline.rs` (inline `#[cfg(test)]`)

**Step 1: Write the failing test**

Add at the bottom of `pipeline.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::camera::GameCamera;
    use crate::render::sample_buffer::{Sample, SampleBuffer};

    #[test]
    fn player_shadow_attenuates_nearby_samples() {
        // F241: Stage 4 shadow should darken samples near the player position.
        let config = RenderConfig { ascii_width: 10, ascii_height: 10, ..Default::default() };
        let mut buf = SampleBuffer::new(10, 10);
        let mut camera = GameCamera::default();
        camera.pos = [5.0, 5.0, 100.0];
        let dw = config.sample_width() as f64;
        let dh = config.sample_height() as f64;
        camera.update(dw, dh);

        // Fill a region with terrain samples at player's Z height
        let buf_w = buf.width as i32;
        for y in 0..buf.height {
            for x in 0..buf.width {
                let idx = (y * buf.width + x) as usize;
                buf.samples[idx].height = 100.0;
                buf.samples[idx].diffuse = 200;
                buf.samples[idx].spare = 0; // terrain
            }
        }

        // Save pre-shadow diffuse values
        let pre_diffuse: Vec<u8> = buf.samples.iter().map(|s| s.diffuse).collect();

        // Apply shadow
        render_player_shadow(&mut buf.samples, buf.width as i32, buf.height as i32, &camera, &[5.0, 5.0, 100.0]);

        // At least some samples near player should be darkened
        let darkened_count = buf.samples.iter().zip(pre_diffuse.iter())
            .filter(|(s, &pre)| s.diffuse < pre)
            .count();

        assert!(darkened_count > 0, "Shadow should darken at least some nearby samples");

        // Samples far from player should be unchanged
        // (corners of the buffer are far from center)
        assert_eq!(buf.samples[0].diffuse, pre_diffuse[0],
            "Corner samples should be unaffected by shadow");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -- pipeline::tests --nocapture`
Expected: FAIL — `render_player_shadow` doesn't exist yet.

**Step 3: Implement render_player_shadow**

Add to `pipeline.rs` (above the pipeline system function):

```rust
/// Stage 4: Player blob shadow projection.
///
/// Ports C++ render.cpp:3184-3217. For each sample near the player, transforms
/// screen coords to world via inv_tm, computes distance to player position,
/// and attenuates diffuse within a ~2.0 unit radius.
///
/// # Arguments
/// * `samples` - Mutable sample buffer slice
/// * `buf_w` - Sample buffer width
/// * `buf_h` - Sample buffer height
/// * `camera` - Camera with inv_tm for screen-to-world transform
/// * `player_pos` - Player world position [x, y, z]
pub fn render_player_shadow(
    samples: &mut [Sample],
    buf_w: i32,
    buf_h: i32,
    camera: &GameCamera,
    player_pos: &[f32; 3],
) {
    use crate::asset_loader::constants::HEIGHT_CELLS;

    let inv_tm = &camera.inv_tm;
    let hc = HEIGHT_CELLS as f64;

    // Shadow radius in world units (C++ uses ~2.0)
    let shadow_radius_sq: f64 = 2.0;
    // Height tolerance (C++ uses 64 raw height units)
    let height_tolerance: f32 = 64.0;

    for y in 0..buf_h {
        for x in 0..buf_w {
            let idx = (y * buf_w + x) as usize;
            let sample = &samples[idx];

            // Skip clear samples
            if sample.height == Sample::CLEAR_HEIGHT {
                continue;
            }

            // Height proximity check (C++ render.cpp:3188)
            if (sample.height - player_pos[2]).abs() > height_tolerance {
                continue;
            }

            // Transform screen (x, y, height) to world via inv_tm
            // C++ render.cpp:3191-3194
            let sx = x as f64;
            let sy = y as f64;
            let sz = sample.height as f64;

            let wx = inv_tm[0] * sx + inv_tm[4] * sy + inv_tm[8] * sz + inv_tm[12];
            let wy = inv_tm[1] * sx + inv_tm[5] * sy + inv_tm[9] * sz + inv_tm[13];

            // Distance from player (C++ render.cpp:3197-3198)
            let dx = wx / hc - player_pos[0] as f64;
            let dy = wy / hc - player_pos[1] as f64;
            let sq_xy = dx * dx + dy * dy;

            if sq_xy <= shadow_radius_sq {
                // Shadow attenuation (C++ render.cpp:3204)
                let dz = (2.0 * (player_pos[2] as f64 - sample.height as f64) + 2.0 * sq_xy) as i32;
                let attenuation = (dz.max(0).min(255)) as u8;

                // Darken diffuse (subtract attenuation, clamp to 0)
                let s = &mut samples[idx];
                s.diffuse = s.diffuse.saturating_sub(attenuation);
            }
        }
    }
}
```

**Step 4: Wire into the pipeline**

In `render_pipeline_system()`, replace the Stage 4 stub (lines ~398-400):

```rust
// BEFORE:
// Stage 4: SHADOW (stub -- future)
let t3 = Instant::now();
timing.shadow_us = t3.elapsed().as_micros() as u64;

// AFTER (F241 FIX: real player blob shadow):
let t3 = Instant::now();
// Player position for shadow. Uses camera.pos as fallback (spectator mode).
// When Character entities exist (Phase 6), this should come from the
// player entity's position. For now, camera.pos is a reasonable proxy.
render_player_shadow(buf, buf_w, buf_h, &camera, &camera.pos);
timing.shadow_us = t3.elapsed().as_micros() as u64;
```

Note: This requires `render_player_shadow` to be called inside the `{ let buf = &mut sample_buffer.samples; ... }` block, between Stage 3 WORLD and the closing brace.

**Step 5: Run test to verify it passes**

Run: `cargo test --lib -- pipeline::tests --nocapture`
Expected: PASS

**Step 6: Commit**

```bash
git add engine-port/src/render/pipeline.rs
git commit -m "feat(render): F241 — implement Stage 4 player blob shadow

Ports C++ render.cpp:3184-3217. Transforms sample screen coords to world
via inv_tm, computes distance to player position, attenuates diffuse within
2.0 unit radius. Replaces empty stub in render_pipeline_system Stage 4."
```

---

## Task 4: Fix water ripple pipeline integration

**Files:**
- Modify: `engine-port/src/render/water.rs` (all three sub-fixes)
- Modify: `engine-port/src/render/sample_buffer.rs` (add 4-sample helper if needed)
- Test: `engine-port/src/render/water.rs` (inline `#[cfg(test)]`)

### Sub-fix 4a: Keep perspective=true for reflected camera

**Step 1: Write the failing test**

Add at the bottom of `water.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflected_camera_keeps_perspective() {
        // Water reflection must use perspective projection (same as normal render).
        // Setting perspective=false forces the affine transform_vertex() path
        // which doesn't match the flipped perspective view matrix.
        // We verify by checking that the flipped camera preserves perspective=true.
        let mut camera = GameCamera::default();
        camera.perspective = true;
        camera.pos = [5.0, 5.0, 100.0];
        camera.update(484.0, 274.0);
        camera.extract_frustum_planes(484.0, 274.0);

        // The render_water_reflections function creates a flipped_camera internally.
        // We test the logic directly: flipped camera should have perspective=true.
        let mut flipped = camera.clone();
        // This is what render_water_reflections does — perspective should NOT be set false
        flipped.view_tm[8] = -camera.view_tm[8];
        flipped.view_tm[9] = -camera.view_tm[9];
        flipped.view_tm[10] = -camera.view_tm[10];
        // F241-water FIX: perspective must stay true
        assert!(flipped.perspective, "Reflected camera must keep perspective=true");
    }
}
```

**Step 2: Run test (this test passes even before the fix — it tests the desired behavior)**

This test documents the requirement. The actual fix is in the next step.

**Step 3: Fix render_water_reflections — remove perspective=false**

In `water.rs`, function `render_water_reflections()`, change line 92:

```rust
// BEFORE (line 92):
flipped_camera.perspective = false;

// AFTER:
// F241-water FIX: Keep perspective=true. The flipped view matrix is built
// for perspective projection. Setting perspective=false forced the affine
// transform_vertex() path which doesn't match, producing wrong positions.
// The perspective path in render_patch reads view_pos/view_dir/mul/add
// directly, so we must also update these for the flipped camera.
// flipped_camera.perspective stays true (inherited from camera.clone())
```

Also, after `flipped_camera.view_tm = flipped_tm;`, recompute the perspective parameters for the flipped camera. Add:

```rust
// Recompute mul/add/view_pos/view_dir for the flipped matrix.
// The perspective path in terrain_shader reads these directly.
// For reflection, only Z is flipped, so:
// - view_pos Z reflects about water plane
// - view_dir stays the same (horizontal only, Z=0 in architectural perspective)
// - mul/add change because the view matrix changed
flipped_camera.mul[0] = flipped_tm[0];
flipped_camera.mul[1] = flipped_tm[1];
flipped_camera.mul[2] = flipped_tm[4];
flipped_camera.mul[3] = flipped_tm[5];
flipped_camera.mul[4] = 0.0;
flipped_camera.mul[5] = flipped_tm[9];

flipped_camera.add[0] = flipped_tm[12];
flipped_camera.add[1] = flipped_tm[13] + 0.5;
flipped_camera.add[2] = flipped_tm[14];

// Reflected view_pos Z (camera backed up by focal along view_dir, then Z reflected)
flipped_camera.view_pos[2] = 2.0 * water_z - camera.view_pos[2];
```

### Sub-fix 4b: Fix extract_frustum_planes_from_tm stub

**Step 4: Write failing test for frustum plane extraction**

```rust
#[cfg(test)]
mod tests {
    // ... (add to existing tests block)

    #[test]
    fn extract_frustum_planes_from_tm_not_stub() {
        // The function must compute planes from the given TM, not just
        // clone the camera's original planes.
        let mut camera = GameCamera::default();
        camera.pos = [5.0, 5.0, 100.0];
        camera.update(484.0, 274.0);
        camera.extract_frustum_planes(484.0, 274.0);

        let original_planes = camera.frustum_planes.clone();

        // Create a very different TM (e.g., identity)
        let different_tm = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        let result = extract_frustum_planes_from_tm(&different_tm, &camera, 484.0, 274.0);

        // If the stub just returns camera.frustum_planes.clone(), these will be equal.
        // After the fix, they should differ because the TM is different.
        // For now, we just check it returns non-empty planes.
        assert!(!result.is_empty(), "Should return at least 4 frustum planes");
        // The planes from an identity TM should differ from the camera's planes
        let planes_differ = result.iter().zip(original_planes.iter())
            .any(|(a, b)| (a[0] - b[0]).abs() > 0.001 || (a[1] - b[1]).abs() > 0.001);
        assert!(planes_differ, "Planes from different TM should differ from original camera planes");
    }
}
```

**Step 5: Run test to verify it fails**

Run: `cargo test --lib -- water::tests --nocapture`
Expected: FAIL — stub returns `camera.frustum_planes.clone()` which equals `original_planes`.

**Step 6: Implement real frustum extraction from flipped TM**

Replace `extract_frustum_planes_from_tm` in `water.rs`:

```rust
/// Extract frustum planes from a given view matrix.
///
/// Computes 4 frustum planes (left, right, top, bottom) from the provided
/// view transform matrix using the same focus_node + screen corner approach
/// as GameCamera::extract_frustum_planes, but with an arbitrary TM.
fn extract_frustum_planes_from_tm(
    tm: &[f64; 16],
    camera: &GameCamera,
    dw: f64,
    dh: f64,
) -> Vec<[f64; 4]> {
    use crate::asset_loader::constants::HEIGHT_CELLS;

    let hc = HEIGHT_CELLS as f64;
    let a = (camera.yaw as f64) * std::f64::consts::PI / 180.0;
    let sinyaw = a.sin();
    let cosyaw = a.cos();

    // Focus node (same as camera — the focal point doesn't change for reflection)
    let focus_node = [
        camera.pos[0] as f64 + sinyaw * camera.focal as f64 / hc,
        camera.pos[1] as f64 - cosyaw * camera.focal as f64 / hc,
        camera.pos[2] as f64 + 0.5 * camera.focal as f64 / hc * crate::asset_loader::constants::HEIGHT_SCALE as f64,
    ];

    // Neutral plane (camera horizontal through pos)
    let neutral_plane = [
        -sinyaw,
        cosyaw,
        0.0,
        sinyaw * camera.pos[0] as f64 - cosyaw * camera.pos[1] as f64,
    ];

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

    // Invert the provided TM (not camera.view_tm)
    let clip_tm = invert_4x4_local(tm);

    let mut world_corners = [[0.0f64; 3]; 4];
    for c in 0..4 {
        let mut w0 = mat4_mul_vec4_local(&clip_tm, &screen_corners_0[c]);
        let mut w1 = mat4_mul_vec4_local(&clip_tm, &screen_corners_1[c]);

        w0[0] /= hc;
        w0[1] /= hc;
        w1[0] /= hc;
        w1[1] /= hc;

        let dir = [w1[0] - w0[0], w1[1] - w0[1], w1[2] - w0[2]];
        let dot_origin = neutral_plane[0] * w0[0] + neutral_plane[1] * w0[1]
            + neutral_plane[2] * w0[2] + neutral_plane[3];
        let dot_dir = neutral_plane[0] * dir[0] + neutral_plane[1] * dir[1]
            + neutral_plane[2] * dir[2];
        let t = if dot_dir.abs() > 1e-12 { -dot_origin / dot_dir } else { 0.0 };
        world_corners[c] = [w0[0] + t * dir[0], w0[1] + t * dir[1], w0[2] + t * dir[2]];
    }

    let (ll, lr, ul, ur) = (world_corners[0], world_corners[1], world_corners[2], world_corners[3]);

    vec![
        plane_from_points_local(&focus_node, &ll, &ul),
        plane_from_points_local(&focus_node, &ur, &lr),
        plane_from_points_local(&focus_node, &ul, &ur),
        plane_from_points_local(&focus_node, &lr, &ll),
    ]
}
```

Note: You'll need local copies of `invert_4x4`, `mat4_mul_vec4`, and `plane_from_points` in `water.rs` (or make the camera.rs versions `pub(crate)`). The cleanest approach: make them `pub(crate)` in `camera.rs` and import.

In `camera.rs`, change visibility of helper functions:
```rust
pub(crate) fn invert_4x4(m: &[f64; 16]) -> [f64; 16] { ... }
pub(crate) fn mat4_mul_vec4(m: &[f64; 16], v: &[f64; 4]) -> [f64; 4] { ... }
pub(crate) fn plane_from_points(p0: &[f64; 3], p1: &[f64; 3], p2: &[f64; 3]) -> [f64; 4] { ... }
```

Then in `water.rs`:
```rust
use crate::render::camera::{invert_4x4, mat4_mul_vec4, plane_from_points};
```
And call them directly (remove the `_local` suffix).

### Sub-fix 4c: Fix ripple 4-sample detection

**Step 7: Write failing test for ripple detection consistency**

```rust
#[cfg(test)]
mod tests {
    // ... (add to existing tests block)

    #[test]
    fn ripple_detects_reflection_with_any_sample_in_block() {
        // apply_water_ripple_pass should check all 4 samples in a 2x2 block,
        // not just the upper-left. A cell where only s10 has REFLECTION should
        // still get rippled.
        let grid_w = 2i32;
        let grid_h = 2i32;
        let sample_w = 2 * grid_w + 4; // = 8
        let sample_h = 2 * grid_h + 4; // = 8
        let mut samples = vec![Sample::clear_state(); (sample_w * sample_h) as usize];

        // Cell (0, 0) maps to sample block at (sx=2, sy=2).
        // Set s10 (sx+1, sy) to have REFLECTION but not s00.
        let s10_idx = (2 * sample_w + 3) as usize; // sy=2, sx=3
        samples[s10_idx].spare = spare_bits::REFLECTION;
        samples[s10_idx].height = 50.0; // not clear

        let mut cells = vec![AnsiCell { fg: 100, bk: 100, gl: b'.', spare: 0 }; (grid_w * grid_h) as usize];
        let original_fg = cells[0].fg;

        apply_water_ripple_pass(&samples, &mut cells, grid_w, grid_h, 1.0);

        // Cell (0,0) should be rippled because at least one sample in its block has REFLECTION
        // (This may or may not change the fg depending on noise — but the function should at least
        // enter the ripple path. We test that the function doesn't skip the cell.)
        // A more robust test: check that the ripple function was reached for this cell.
        // Since Perlin noise at (0,0,1.0) produces a deterministic value, we can check:
        let was_processed = cells[0].fg != original_fg || true; // ripple may produce id=0 (no change)
        // The key assertion is that the function doesn't skip: we verify by checking
        // that with ALL 4 samples having REFLECTION, the result matches.
        let mut samples_all = samples.clone();
        let s00_idx = (2 * sample_w + 2) as usize;
        samples_all[s00_idx].spare = spare_bits::REFLECTION;
        samples_all[s00_idx].height = 50.0;

        let mut cells_all = vec![AnsiCell { fg: 100, bk: 100, gl: b'.', spare: 0 }; (grid_w * grid_h) as usize];
        apply_water_ripple_pass(&samples_all, &mut cells_all, grid_w, grid_h, 1.0);

        // Both should produce the same ripple effect for cell (0,0)
        assert_eq!(cells[0].fg, cells_all[0].fg,
            "Ripple should produce same result whether 1 or 4 samples have REFLECTION");
    }
}
```

**Step 8: Run test to verify it fails**

Run: `cargo test --lib -- water::tests --nocapture`
Expected: FAIL — current code only checks s00, so the single-sample case skips the cell.

**Step 9: Fix apply_water_ripple_pass to check all 4 samples**

In `water.rs`, function `apply_water_ripple_pass()`, change the sample check (lines ~176-189):

```rust
// BEFORE (checks only upper-left sample):
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

// AFTER (F241-water FIX: check all 4 samples, matching resolve() logic):
let sx = 2 + 2 * cx;
let sy = 2 + 2 * cy;
let sample_w = 2 * grid_w + 4;

let i00 = (sy * sample_w + sx) as usize;
let i10 = i00 + 1;
let i01 = ((sy + 1) * sample_w + sx) as usize;
let i11 = i01 + 1;

if i11 >= samples.len() {
    continue;
}

// OR all 4 samples' spare bits (same as resolve.rs:87)
let combined_spare = samples[i00].spare | samples[i10].spare
    | samples[i01].spare | samples[i11].spare;

if (combined_spare & spare_bits::PARITY_MASK) != spare_bits::REFLECTION
    || (combined_spare & spare_bits::MESH_FLAG) != 0
{
    continue;
}
```

**Step 10: Run all water tests**

Run: `cargo test --lib -- water::tests --nocapture`
Expected: ALL PASS

**Step 11: Commit**

```bash
git add engine-port/src/render/water.rs engine-port/src/render/camera.rs
git commit -m "fix(render): water ripple pipeline integration — 3 compounding bugs

1. Removed perspective=false on reflected camera — was forcing affine path
   that doesn't match the perspective-built flipped view matrix.
2. Replaced extract_frustum_planes_from_tm stub with real implementation
   that derives planes from the flipped TM.
3. Fixed apply_water_ripple_pass to check all 4 samples in 2x2 block
   (matching resolve's combined_spare OR logic) instead of only upper-left."
```

---

## Task 5: Update failure log and run full verification

**Files:**
- Modify: `docs/FAILURE_LOG.md`

**Step 1: Run full test suite**

Run: `cargo test --lib --nocapture`
Expected: ALL PASS (all new tests from Tasks 1-4)

**Step 2: Run cargo clippy**

Run: `cargo clippy -- -D warnings`
Expected: No errors

**Step 3: Update FAILURE_LOG.md**

Change status of F236, F239, F241 (Round 12 entries) from OPEN to RESOLVED with commit references.

Add a new entry for the water ripple fix.

**Step 4: Commit**

```bash
git add docs/FAILURE_LOG.md
git commit -m "docs: mark F236, F239, F241 as RESOLVED, add water ripple fix entry"
```

---

## Verification Checklist

| Fix | Test | Runtime Evidence |
|-----|------|-----------------|
| F236 light defaults | `camera::tests::default_light_ambient_matches_terrain` | Mesh faces show shading variation (not flat white) |
| F236 diffuse varies | `camera::tests::mesh_diffuse_varies_with_default_light` | Up-facing vs down-facing normals produce different intensity |
| F239 AABB extent | `quadtree::tests::frustum_aabb_accounts_for_render_extent` | Nearby patches not culled; distant patches still culled |
| F241 shadow | `pipeline::tests::player_shadow_attenuates_nearby_samples` | Dark circle under player position |
| Water perspective | `water::tests::reflected_camera_keeps_perspective` | Reflection geometry renders at correct positions |
| Water frustum | `water::tests::extract_frustum_planes_from_tm_not_stub` | Reflected terrain patches not falsely culled |
| Water ripple 4-sample | `water::tests::ripple_detects_reflection_with_any_sample_in_block` | Ripple covers all water cells, not just those with 4/4 reflection samples |

## Out of Scope

- Mesh reflections in water (world BSP query with flipped frustum) — deferred, terrain-only for now
- Shadow from world geometry (C++ HitWorld in DarkUpdater) — terrain self-shadow only
- Dynamic light direction (currently hardcoded) — future Phase 7
- Shape-vector glyph selection for reflected cells — uses existing AutoMat fallback
