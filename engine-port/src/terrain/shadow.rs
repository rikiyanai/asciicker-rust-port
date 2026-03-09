//! Terrain shadow computation via raycasting.
//!
//! Ports the C++ `DarkUpdater` from terrain.cpp:1714-1765.
//! Computes a 64-bit shadow bitmask per terrain patch by raycasting from each
//! visual cell center toward a light direction and testing terrain self-shadowing.
//!
//! This is a LOAD-TIME precomputation, NOT per-frame. Called once after terrain
//! assembly completes (from `a3d_assembly_system` in assembly.rs).
//!
//! World geometry shadowing (HitWorld) is deferred to Phase 6. Phase 5 implements
//! terrain self-shadowing only. The dark bitmask format supports future world
//! shadow additions (just OR in more bits).

use crate::asset_loader::constants::{HEIGHT_SCALE, VISUAL_CELLS};

use super::RuntimeTerrain;

/// Default light direction (NOT normalized): direction FROM surface TOWARD light source.
///
/// Light source is at upper-right, above the terrain: `[-1.0, 1.0, 2.0]`.
/// The positive Z component means the light is above the terrain plane.
/// We normalize at usage site for clarity.
///
/// Note: The C++ engine computes this from `light_pos - patch_center`. With
/// `light_pos` above the scene, the Z component is positive. Phase 5 hardcodes
/// a reasonable default; Phase 7 may make this configurable.
const LIGHT_DIR_DEFAULT_RAW: [f64; 3] = [-1.0, 1.0, 2.0];

/// Normalized light direction derived from `LIGHT_DIR_DEFAULT_RAW`.
fn normalized_light_dir() -> [f64; 3] {
    let [x, y, z] = LIGHT_DIR_DEFAULT_RAW;
    let mag = (x * x + y * y + z * z).sqrt();
    [x / mag, y / mag, z / mag]
}

/// Public constant for the default light direction (normalized).
/// Callers (e.g., assembly.rs) pass this to `update_terrain_dark`.
pub fn default_light_dir() -> [f64; 3] {
    normalized_light_dir()
}

/// Maximum raycast steps for terrain self-shadow testing.
const MAX_SHADOW_STEPS: usize = 32;

/// Compute terrain self-shadows and write 64-bit dark bitmasks.
///
/// Two-pass approach to satisfy the borrow checker:
/// 1. **Immutable pass**: For each patch, iterate all 64 visual cells, raycast
///    from cell center along `light_dir`, test terrain height via
///    `interpolate_height`. Collect `(patch_index, dark_bitmask)` pairs.
/// 2. **Mutable pass**: Write collected dark values back via `for_each_patch_mut`.
///
/// # Arguments
/// * `terrain` - Mutable reference to RuntimeTerrain
/// * `light_dir` - Normalized light direction vector `[x, y, z]`
pub fn update_terrain_dark(terrain: &mut RuntimeTerrain, light_dir: [f64; 3]) {
    // Pass 1: Collect shadow results (immutable borrow of terrain)
    let shadow_results = compute_shadow_bitmasks(terrain, light_dir);

    // Pass 2: Write dark values back (mutable borrow)
    let mut result_idx = 0;
    terrain.for_each_patch_mut(|patch| {
        if result_idx < shadow_results.len() {
            patch.dark = shadow_results[result_idx];
            result_idx += 1;
        }
    });
}

/// Immutable pass: compute dark bitmasks for all patches.
///
/// Returns a Vec of dark bitmasks in the same order as `for_each_patch` traversal.
fn compute_shadow_bitmasks(terrain: &RuntimeTerrain, light_dir: [f64; 3]) -> Vec<u64> {
    let mut results = Vec::new();

    // Scale light_dir Z by HEIGHT_SCALE for proper height comparison.
    // P5-330 FIX: Both interpolate_height return values and ray Z are in scaled
    // height units (raw height * HEIGHT_SCALE). The light direction Z must be
    // scaled accordingly.
    let light_dir_scaled = [
        light_dir[0],
        light_dir[1],
        light_dir[2] * HEIGHT_SCALE as f64,
    ];

    terrain.for_each_patch(|patch| {
        let mut dark: u64 = 0;
        let px = patch.x * VISUAL_CELLS as i32;
        let py = patch.y * VISUAL_CELLS as i32;

        for v in 0..VISUAL_CELLS {
            for u in 0..VISUAL_CELLS {
                // P5-315 FIX: Use patch.x and patch.y for px/py
                let cell_center = patch.sample_cell_center(u, v, px, py);

                if terrain_raycast_height(
                    terrain,
                    &cell_center,
                    &light_dir_scaled,
                    MAX_SHADOW_STEPS,
                ) {
                    // R7-003 FIX: bit layout = u + v * VISUAL_CELLS
                    let cell_bit = u + v * VISUAL_CELLS;
                    dark |= 1u64 << cell_bit;
                }
            }
        }

        results.push(dark);
    });

    results
}

/// Raycast along terrain to test for self-shadowing.
///
/// Steps from `origin` along `dir` (with Z already scaled by HEIGHT_SCALE),
/// interpolating terrain height at each step. Returns `true` if the terrain
/// is higher than the ray at any step (shadowed).
///
/// # Arguments
/// * `terrain` - Immutable reference to RuntimeTerrain
/// * `origin` - World-space ray origin `[x, y, z]` (z already scaled)
/// * `dir` - Light direction with Z scaled by HEIGHT_SCALE
/// * `max_steps` - Maximum number of steps along the ray
fn terrain_raycast_height(
    terrain: &RuntimeTerrain,
    origin: &[f64; 3],
    dir: &[f64; 3],
    max_steps: usize,
) -> bool {
    let tolerance = HEIGHT_SCALE as f64 / 4.0;

    // Step along ray from origin. Start at t=1.0 to skip self-intersection.
    // P5-311 FIX: step size 1.0 = one visual-cell unit in X/Y, correct for
    // shadow resolution (over-samples 2x relative to height grid).
    for step in 1..=max_steps {
        let t = step as f64;
        let wx = origin[0] + dir[0] * t;
        let wy = origin[1] + dir[1] * t;
        let ray_z = origin[2] + dir[2] * t;

        // P5-120 FIX: interpolate_height returns Option<f64>; None means
        // outside terrain bounds -- skip this step (no terrain to shadow from).
        // F238 FIX: interpolate_height returns world units (raw / HEIGHT_SCALE).
        // Shadow coordinates use raw * HEIGHT_SCALE (matching sample_cell_center).
        // Convert: (raw / HS) * HS² = raw * HS.
        let hs_sq = HEIGHT_SCALE as f64 * HEIGHT_SCALE as f64;
        let Some(terrain_z) = terrain.interpolate_height(wx, wy).map(|z| z * hs_sq) else {
            continue;
        };

        // If terrain height exceeds ray height (with tolerance), this cell is shadowed
        if terrain_z > ray_z + tolerance {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
