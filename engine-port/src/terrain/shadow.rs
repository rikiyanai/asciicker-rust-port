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
        let Some(terrain_z) = terrain.interpolate_height(wx, wy) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_loader::a3d_terrain::{A3dTerrain, TerrainPatch};
    use crate::asset_loader::constants::{HEIGHT_CELLS_PLUS_ONE, VISUAL_CELLS};
    use crate::terrain::RuntimeTerrain;

    fn make_patch(x: i32, y: i32, base_height: u16) -> TerrainPatch {
        TerrainPatch {
            x,
            y,
            height: [[base_height; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
            visual: [[1u16; VISUAL_CELLS]; VISUAL_CELLS],
            diag: 0,
        }
    }

    fn make_runtime_terrain(patches: &[TerrainPatch]) -> RuntimeTerrain {
        let terrain = A3dTerrain {
            patches: patches.to_vec(),
        };
        RuntimeTerrain::build_from_parsed(&terrain)
    }

    #[test]
    fn test_flat_terrain_no_shadows() {
        // All-zero heights: no height difference means no shadows.
        let patches = vec![make_patch(0, 0, 0), make_patch(1, 0, 0)];
        let mut rt = make_runtime_terrain(&patches);
        let light_dir = default_light_dir();

        update_terrain_dark(&mut rt, light_dir);

        rt.for_each_patch(|p| {
            assert_eq!(
                p.dark, 0,
                "Flat terrain at height=0 should have no shadows, got dark={:#018x} at ({},{})",
                p.dark, p.x, p.y
            );
        });
    }

    #[test]
    fn test_tall_peak_casts_shadow() {
        // Light direction [-1, 1, 2] means light source is at (-X, +Y, +Z).
        // Shadow rays trace from cell centers TOWARD the light ([-1, +1, +2]).
        // A tall patch at x=0 blocks light for cells on the OPPOSITE side (+X).
        // So flat patch at x=1 should be in shadow of tall patch at x=0.
        let mut tall_patch = make_patch(0, 0, 0);
        for row in tall_patch.height.iter_mut() {
            for h in row.iter_mut() {
                *h = 200;
            }
        }
        let flat_patch = make_patch(1, 0, 0);

        let patches = vec![tall_patch, flat_patch];
        let mut rt = make_runtime_terrain(&patches);
        let light_dir = default_light_dir();

        update_terrain_dark(&mut rt, light_dir);

        // The flat patch (x=1) should have SOME shadowed cells.
        // Rays from flat cells trace toward the light ([-1, +1, +2]) and hit
        // the tall patch at x=0, which is at height 200*16=3200.
        let flat = rt.get_patch_at(1, 0).expect("flat patch must exist");
        assert!(
            flat.dark != 0,
            "Flat patch on shadow side of tall peak should have some shadow, got dark=0"
        );
    }

    #[test]
    fn test_shadow_bitmask_layout() {
        // Verify bit indexing: cell (u=3, v=5) maps to bit u + v * VISUAL_CELLS = 3 + 5 * 8 = 43
        let cell_bit = 3 + 5 * VISUAL_CELLS;
        assert_eq!(cell_bit, 43, "Cell (3,5) should map to bit 43");

        // Verify a dark bitmask with just that bit set
        let dark: u64 = 1u64 << cell_bit;
        assert_eq!(dark, 1u64 << 43);

        // Verify extraction
        assert_ne!(dark & (1u64 << 43), 0, "Bit 43 should be set");
        assert_eq!(dark & (1u64 << 42), 0, "Bit 42 should NOT be set");
        assert_eq!(dark & (1u64 << 44), 0, "Bit 44 should NOT be set");
    }

    #[test]
    fn test_shadow_computation_is_deterministic() {
        // P5-125 FIX: Calling update_terrain_dark TWICE with identical inputs
        // produces identical dark bitmasks.
        let patches = vec![
            make_patch(0, 0, 0),
            make_patch(1, 0, 50),
            make_patch(0, 1, 100),
        ];

        let light_dir = default_light_dir();

        // First run
        let mut rt1 = make_runtime_terrain(&patches);
        update_terrain_dark(&mut rt1, light_dir);
        let mut darks_1 = Vec::new();
        rt1.for_each_patch(|p| darks_1.push(p.dark));

        // Second run
        let mut rt2 = make_runtime_terrain(&patches);
        update_terrain_dark(&mut rt2, light_dir);
        let mut darks_2 = Vec::new();
        rt2.for_each_patch(|p| darks_2.push(p.dark));

        assert_eq!(
            darks_1, darks_2,
            "Shadow computation must be deterministic: run1={:?}, run2={:?}",
            darks_1, darks_2
        );
    }

    #[test]
    fn test_shadow_known_answer() {
        // R16-F192 FIX: Non-degenerate known-answer test.
        // Light direction [-1, 0, 1] (normalized): light source at (-X, 0, +Z).
        // Shadow rays trace toward [-1, 0, +1]. A tall patch at x=0 blocks
        // light for flat patch at x=1 (on the +X shadow side).
        let mut tall = make_patch(0, 0, 0);
        for row in tall.height.iter_mut() {
            for h in row.iter_mut() {
                *h = 64;
            }
        }
        let flat_east = make_patch(1, 0, 0);

        let patches = vec![tall, flat_east];
        let mut rt = make_runtime_terrain(&patches);

        // Light direction: [-1, 0, 1] normalized -- light source at (-X, 0, +Z)
        let mag = (1.0f64 * 1.0 + 0.0 + 1.0 * 1.0).sqrt();
        let light_dir = [-1.0 / mag, 0.0, 1.0 / mag];

        update_terrain_dark(&mut rt, light_dir);

        // The flat east patch should have shadowed cells from the tall neighbor.
        // Rays from flat east cells trace toward light ([-1, 0, +1]) and encounter
        // the tall patch at x=0 (height=64*16=1024) which occludes them.
        let east_patch = rt.get_patch_at(1, 0).expect("east patch must exist");
        assert!(
            east_patch.dark != 0,
            "Flat patch east of tall patch should be shadowed"
        );

        // The tall patch itself should NOT be shadowed (it is the tallest,
        // and rays go toward -X where there is no terrain).
        let tall_patch = rt.get_patch_at(0, 0).expect("tall patch must exist");
        let east_shadow_count = east_patch.dark.count_ones();
        let tall_shadow_count = tall_patch.dark.count_ones();
        assert!(
            east_shadow_count > tall_shadow_count,
            "East (flat) patch should have more shadow ({}) than tall patch ({})",
            east_shadow_count,
            tall_shadow_count
        );
    }
}
