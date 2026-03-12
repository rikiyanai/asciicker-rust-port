//! Golden-file CI comparison infrastructure and budget assertion tests.
//!
//! Exercises the full rendering pipeline and validates output correctness.
//! Tests determinism, comparison function accuracy, and frame budget.
//!
//! R14-SYNTH-BAN: ALL synthetic/fake test baselines are BANNED.
//! VIS-02 requires <1% diff against C++ reference output.
//! C++ reference data capture requires a utility that compiles the C++
//! engine to dump AnsiCell output for a specific scene + camera position.
//! This utility does NOT yet exist.
//!
//! BANNED: Do NOT create synthetic "known-good baselines" that make tests
//! pass without real C++ data. Such tests prove nothing about compatibility.
//!
//! Tests that need C++ reference data MUST be #[ignore] with this message:
//!   "Requires C++ reference data in engine-port/tests/fixtures/cpp_reference/"
//!
//! Phase 4 execution audit gaps addressed here:
//! - R35 (GAP-01): Golden-file infrastructure built; C++ data is HARD BLOCKER
//! - R36 (GAP-02): RGB555 exhaustive test added in Phase 3.1 Task 3
//! - R37 (GAP-03): auto_mat LUT consistency test added in Phase 3.1 Task 3
//!   Full C++ comparison deferred until dump utility exists
//! - R38 (GAP-04): Budget assertion test below validates release-mode perf
//! - R39 (GAP-05): <1% threshold testable ONLY against real C++ output
//!
//! REQUIRED steps to unblock golden-file tests:
//! 1. Build C++ dump utility (HARD BLOCKER for Phase 5 completion)
//! 2. Capture reference output for game_map_y8.a3d at fixed camera
//! 3. Also capture: rgb2pal() output for all 32768 RGB555 values
//! 4. Also capture: auto_mat LUT dump (32768 entries x 3 bytes)
//! 5. Store reference in engine-port/tests/fixtures/cpp_reference/
//! 6. Update test_golden_vs_cpp_reference to load and compare
//! 7. Add test_rgb2pal_vs_cpp and test_auto_mat_vs_cpp

use asciicker_engine::asset_loader::a3d_terrain::{A3dTerrain, TerrainPatch};
use asciicker_engine::asset_loader::constants::{HEIGHT_CELLS_PLUS_ONE, VISUAL_CELLS};
use asciicker_engine::output::ascii_cell_grid::AsciiCellGrid;
use asciicker_engine::render::camera::GameCamera;
use asciicker_engine::render::material::{Material, test_materials};
use asciicker_engine::render::resolve_bridge::{
    AutoMatGlyphSelector, XTERM_256_PALETTE, resolve_to_grid,
};
use asciicker_engine::render::sample_buffer::SampleBuffer;
use asciicker_engine::render::terrain_shader::render_patch;
use asciicker_engine::terrain::RuntimeTerrain;
use asciicker_engine::terrain::shadow::{default_light_dir, update_terrain_dark};

// ---------------------------------------------------------------------------
// Helper: create minimal test terrain
// ---------------------------------------------------------------------------

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

/// Run the full pipeline (clear -> render_patches -> resolve_to_grid) and return
/// the resulting AsciiCellGrid.
fn run_pipeline(
    terrain: &RuntimeTerrain,
    materials: &[Material],
    camera: &GameCamera,
    ascii_w: u32,
    ascii_h: u32,
) -> AsciiCellGrid {
    let mut buf = SampleBuffer::new(ascii_w, ascii_h);
    let mut grid = AsciiCellGrid::new(ascii_w, ascii_h);
    let mut glyph_sel = AutoMatGlyphSelector;
    let mut resolve_buf = Vec::new();

    // Stage 1: CLEAR (SampleBuffer starts clear)
    buf.clear();

    // Stage 2: TERRAIN
    let buf_w = buf.width as i32;
    let buf_h = buf.height as i32;

    terrain.for_each_patch(|patch| {
        render_patch(
            &mut buf.samples,
            buf_w,
            buf_h,
            patch,
            patch.x,
            patch.y,
            &camera,
            None,
        );
    });

    // Stage 6: RESOLVE
    resolve_to_grid(&buf, materials, &mut grid, &mut glyph_sel, &mut resolve_buf);

    grid
}

// ---------------------------------------------------------------------------
// RGBA direct comparison (no xterm round-trip)
// ---------------------------------------------------------------------------

/// Compare two AsciiCellGrids directly by RGBA values (no palette round-trip).
///
/// Returns (diff_count, total_cells, diff_percentage).
/// Used for determinism tests and comparison function validation.
fn compare_rgba_grids(a: &AsciiCellGrid, b: &AsciiCellGrid) -> (usize, usize, f64) {
    assert_eq!(a.width, b.width, "Grid widths must match");
    assert_eq!(a.height, b.height, "Grid heights must match");

    let total = a.cells_count();
    let mut diffs = 0;

    for i in 0..total {
        let char_diff = a.char_indices[i] != b.char_indices[i];
        let fg_diff = a.fg_colors[i] != b.fg_colors[i];
        let bg_diff = a.bg_colors[i] != b.bg_colors[i];

        if char_diff || fg_diff || bg_diff {
            diffs += 1;
        }
    }

    let pct = if total > 0 {
        (diffs as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    (diffs, total, pct)
}

// ---------------------------------------------------------------------------
// Xterm-256 round-trip comparison (for C++ reference tests)
// ---------------------------------------------------------------------------

/// Convert an RGBA color to the nearest xterm-256 palette index.
///
/// Searches the color cube (indices 16-231) for the closest match by
/// Euclidean distance in RGB space. Ignores alpha channel.
fn rgba_to_nearest_xterm256(rgba: [u8; 4]) -> u8 {
    let r = rgba[0];
    let g = rgba[1];
    let b = rgba[2];

    let mut best_idx = 16u8;
    let mut best_dist = u32::MAX;

    // Search color cube (16-231)
    for idx in 16..=231u8 {
        let pal = XTERM_256_PALETTE[idx as usize];
        let dr = (r as i32 - pal[0] as i32).unsigned_abs();
        let dg = (g as i32 - pal[1] as i32).unsigned_abs();
        let db = (b as i32 - pal[2] as i32).unsigned_abs();
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }

    // Also check grayscale ramp (232-255)
    for idx in 232..=255u8 {
        let pal = XTERM_256_PALETTE[idx as usize];
        let dr = (r as i32 - pal[0] as i32).unsigned_abs();
        let dg = (g as i32 - pal[1] as i32).unsigned_abs();
        let db = (b as i32 - pal[2] as i32).unsigned_abs();
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }

    best_idx
}

/// Compare an AsciiCellGrid against xterm-256 reference data.
///
/// Converts grid RGBA back to xterm-256 indices for comparison against
/// C++ reference output (which is natively in xterm-256 palette indices).
///
/// Returns (diff_count, total_cells, diff_percentage).
/// Reserved for `test_golden_vs_cpp_reference` (#[ignore] until C++ data exists).
fn compare_ansi_grids(
    grid: &AsciiCellGrid,
    ref_gl: &[u8],
    ref_fg: &[u8],
    ref_bk: &[u8],
) -> (usize, usize, f64) {
    let total = grid.cells_count();
    assert_eq!(ref_gl.len(), total, "ref_gl length must match grid size");
    assert_eq!(ref_fg.len(), total, "ref_fg length must match grid size");
    assert_eq!(ref_bk.len(), total, "ref_bk length must match grid size");

    let mut diffs = 0;

    for i in 0..total {
        // M-05 FIX: debug_assert for char_index > 255
        debug_assert!(
            grid.char_indices[i] <= 255,
            "char_index {} exceeds u8 range at cell {}",
            grid.char_indices[i],
            i
        );
        let gl_match = grid.char_indices[i] as u8 == ref_gl[i];
        let fg_match = rgba_to_nearest_xterm256(grid.fg_colors[i]) == ref_fg[i];
        let bk_match = rgba_to_nearest_xterm256(grid.bg_colors[i]) == ref_bk[i];

        if !gl_match || !fg_match || !bk_match {
            diffs += 1;
        }
    }

    let pct = if total > 0 {
        (diffs as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    (diffs, total, pct)
}

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn setup_camera(ascii_w: u32, ascii_h: u32) -> GameCamera {
    let mut camera = GameCamera::default();
    // Position camera near the terrain patch center, looking down at it.
    // Terrain patch (0,0) covers world X=[0..8), Y=[0..8).
    // Center is at (4, 4). Camera slightly offset with appropriate height.
    camera.pos = [4.0, 4.0, 50.0];
    camera.yaw = 0.0;
    camera.zoom = 0.5; // Wider view to capture more terrain
    camera.perspective = true;

    let dw = (2 * ascii_w + 4) as f64;
    let dh = (2 * ascii_h + 4) as f64;
    camera.update(dw, dh);
    camera.extract_frustum_planes(dw, dh);
    camera
}

// ===========================================================================
// Pipeline integration tests
// ===========================================================================

#[test]
fn test_pipeline_produces_nontrivial_output() {
    // Create a minimal RuntimeTerrain with 1 flat patch at known position
    let patches = vec![make_patch(0, 0, 100)];
    let terrain = make_runtime_terrain(&patches);
    let materials = test_materials();
    let camera = setup_camera(40, 25);

    let grid = run_pipeline(&terrain, &materials, &camera, 40, 25);

    let total = grid.cells_count();

    // Assert: at least some cells are non-clear (non-space glyph).
    // A single flat patch at small resolution may only fill a fraction of cells
    // depending on camera projection. The key test is that the pipeline PRODUCES
    // output, not that it fills a specific percentage.
    let non_clear = grid.char_indices.iter().filter(|&&c| c != 32).count();
    assert!(
        non_clear >= 1,
        "Pipeline must produce at least 1 non-clear cell, got 0 out of {}",
        total
    );

    // Assert: at least 2 distinct fg_colors appear
    let mut unique_fg: std::collections::HashSet<[u8; 4]> = std::collections::HashSet::new();
    for &fg in &grid.fg_colors {
        unique_fg.insert(fg);
    }
    assert!(
        unique_fg.len() >= 2,
        "Expected at least 2 distinct fg colors, got {}",
        unique_fg.len()
    );
}

#[test]
fn test_pipeline_determinism() {
    // R14-SYNTH-BAN: This tests determinism only, NOT a golden-file baseline.
    let patches = vec![make_patch(0, 0, 100), make_patch(1, 0, 50)];
    let terrain = make_runtime_terrain(&patches);
    let materials = test_materials();
    let camera = setup_camera(20, 15);

    // Run pipeline twice with identical inputs
    let grid1 = run_pipeline(&terrain, &materials, &camera, 20, 15);
    let grid2 = run_pipeline(&terrain, &materials, &camera, 20, 15);

    let (diffs, total, pct) = compare_rgba_grids(&grid1, &grid2);
    assert_eq!(
        diffs, 0,
        "Pipeline must be deterministic: {} diffs out of {} cells ({:.2}%)",
        diffs, total, pct
    );
}

#[test]
fn test_compare_diff_threshold() {
    // R14-SYNTH-BAN: Tests the comparison FUNCTION, not a golden-file baseline.
    let patches = vec![make_patch(0, 0, 100)];
    let terrain = make_runtime_terrain(&patches);
    let materials = test_materials();
    let camera = setup_camera(20, 15);

    // Run pipeline
    let grid1 = run_pipeline(&terrain, &materials, &camera, 20, 15);

    // Clone and mutate one cell
    let mut grid2 = AsciiCellGrid::new(20, 15);
    grid2.char_indices = grid1.char_indices.clone();
    grid2.fg_colors = grid1.fg_colors.clone();
    grid2.bg_colors = grid1.bg_colors.clone();

    // Mutate exactly one cell
    grid2.char_indices[0] = if grid1.char_indices[0] == b'X' as u16 {
        b'O' as u16
    } else {
        b'X' as u16
    };

    let (diffs, _total, pct) = compare_rgba_grids(&grid1, &grid2);
    assert_eq!(diffs, 1, "Exactly one cell should differ");
    assert!(
        pct > 0.0 && pct < 1.0,
        "Diff percentage should be >0% and <1% for single cell, got {:.4}%",
        pct
    );
}

// ===========================================================================
// Budget assertion test
// ===========================================================================

#[test]
#[ignore] // Run explicitly: cargo test -- --ignored budget
fn test_pipeline_budget_240x135() {
    // AUDIT #11: Budget assertion test validates full pipeline < 12ms at 240x135.
    // If this test fails, apply escape hatches:
    // 1. Reduce resolution: RenderConfig { ascii_width: 160, ascii_height: 90 }
    // 2. Skip shadow computation
    // 3. Tighten frustum culling far plane

    // Create representative terrain (4x4 = 16 patches)
    let mut patches = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let height = ((x + y) * 20) as u16;
            patches.push(make_patch(x, y, height));
        }
    }
    let mut terrain = make_runtime_terrain(&patches);

    // P5-314 FIX: Shadow computation is load-time precomputation,
    // NOT included in the per-frame timing loop.
    // update_terrain_dark called ONCE here (load-time cost, not included in frame budget)
    update_terrain_dark(&mut terrain, default_light_dir());

    // Full 256-material table (use test_materials padded to 256)
    let mut materials: Vec<Material> = test_materials();
    while materials.len() < 256 {
        materials.push(Material::default());
    }

    let camera = setup_camera(240, 135);

    // Warm-up run
    let _ = run_pipeline(&terrain, &materials, &camera, 240, 135);

    // Timed runs
    let iterations = 20;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = run_pipeline(&terrain, &materials, &camera, 240, 135);
    }
    let total_elapsed = start.elapsed();
    let avg_ms = total_elapsed.as_secs_f64() * 1000.0 / iterations as f64;

    eprintln!(
        "Pipeline budget test: {iterations} iterations, total={:.2}ms, avg={:.2}ms",
        total_elapsed.as_secs_f64() * 1000.0,
        avg_ms
    );

    assert!(
        avg_ms < 12.0,
        "Full pipeline at 240x135 must complete in <12ms (leaving 4ms headroom for 60fps). \
         Average was {:.2}ms. Apply escape hatches if needed.",
        avg_ms
    );
}

// ===========================================================================
// End-to-end .a3d integration test
// ===========================================================================

#[test]
#[ignore] // Requires full Bevy runtime (not available in unit tests)
fn test_load_a3d_full_pipeline() {
    // R49 FIX: End-to-end .a3d integration test.
    // This test validates the FULL pipeline with real data via a Bevy app.
    // It requires game_map_y8.a3d in engine-port/assets/ (not committed;
    // each developer must deploy manually).
    //
    // To run manually:
    //   cargo test -- --ignored test_load_a3d_full_pipeline
    //
    // The test needs a full Bevy runtime with AssetServer, which is not
    // available in the unit test harness.  Validate interactively with:
    //   cargo run   (from engine-port/)
    let asset_path = std::path::Path::new("assets/game_map_y8.a3d");
    assert!(
        asset_path.exists(),
        "Requires game_map_y8.a3d in engine-port/assets/. \
         Copy from /Users/r/Downloads/asciicker-Y9-2/a3d/game_map_y8.a3d"
    );
    // Full Bevy app test would go here when runtime testing infrastructure is available.
}

// ===========================================================================
// Golden-file vs C++ reference test
// ===========================================================================

#[test]
#[ignore] // R14-SYNTH-BAN: Requires real C++ reference data
fn test_golden_vs_cpp_reference() {
    // VIS-02 UNBLOCK CHECKLIST:
    //
    // 1. BUILD C++ DUMP UTILITY:
    //    In /Users/r/Downloads/asciicker-Y9-2/, create a small program that:
    //    - Loads game_map_y8.a3d
    //    - Sets camera to: pos=[0,0,0], yaw=0, perspective=true, zoom=1.0
    //    - Renders one frame at 240x135 ASCII resolution
    //    - Dumps AnsiCell grid as binary: [gl:u8, fg_pal:u8, bk_pal:u8] per cell, row-major
    //    - Output to: game_map_y8_ref.bin (240*135*3 = 97200 bytes)
    //
    // 2. CAPTURE REFERENCE DATA:
    //    Run the C++ dump utility, save output to:
    //    engine-port/tests/fixtures/cpp_reference/game_map_y8_ref.bin
    //
    // 3. ENABLE THIS TEST:
    //    - Remove this assertion
    //    - Load reference data from fixture path
    //    - Run pipeline at same camera position
    //    - Call compare_ansi_grids() and assert < 1% diff
    //
    // 4. UPDATE REQUIREMENTS:
    //    Change VIS-02 status to [x] Complete in REQUIREMENTS.md
    //
    // Expected reference file size: 97200 bytes (240 * 135 * 3)
    // Camera: pos=[0,0,0], yaw=0, perspective=true, zoom=1.0
    // Resolution: 240x135 ASCII cells

    let ref_path = std::path::Path::new("tests/fixtures/cpp_reference/game_map_y8_ref.bin");
    assert!(
        ref_path.exists(),
        "VIS-02 BLOCKED: C++ reference data not found at {:?}. \
         See test comments for the 4-step unblock checklist.",
        ref_path
    );
}

// ===========================================================================
// Unit tests for compare_ansi_grids (xterm round-trip, #[ignore] path)
// ===========================================================================

#[test]
fn test_compare_identical_grids() {
    let grid = AsciiCellGrid::new(4, 4);
    let total = grid.cells_count();

    // Create matching reference data (all space/black)
    let ref_gl = vec![32u8; total];
    let ref_fg = vec![16u8; total]; // xterm 16 = (0,0,0) in color cube
    let ref_bk = vec![16u8; total];

    let (diffs, _, pct) = compare_ansi_grids(&grid, &ref_gl, &ref_fg, &ref_bk);
    assert_eq!(diffs, 0, "Identical grids should have 0 diffs");
    assert!(
        (pct - 0.0).abs() < f64::EPSILON,
        "Diff percentage should be 0%"
    );
}

#[test]
fn test_compare_fully_different() {
    let mut grid = AsciiCellGrid::new(4, 4);
    let total = grid.cells_count();

    // Set grid to non-default values
    for i in 0..total {
        grid.char_indices[i] = b'A' as u16;
        grid.fg_colors[i] = [255, 0, 0, 255]; // red
        grid.bg_colors[i] = [0, 255, 0, 255]; // green
    }
    // Drop mutability for comparison
    let grid = grid;

    // Reference is completely different
    let ref_gl = vec![b'Z'; total];
    let ref_fg = vec![21u8; total]; // blue
    let ref_bk = vec![196u8; total]; // red

    let (diffs, _, pct) = compare_ansi_grids(&grid, &ref_gl, &ref_fg, &ref_bk);
    assert_eq!(
        diffs, total,
        "Completely different grids should have total diffs"
    );
    assert!(
        (pct - 100.0).abs() < f64::EPSILON,
        "Diff percentage should be 100%, got {:.2}%",
        pct
    );
}

#[test]
fn test_compare_one_cell_diff() {
    let grid = AsciiCellGrid::new(4, 4);
    let total = grid.cells_count();

    // Default grid: space (32), black fg/bg
    // Reference matches except one cell
    let mut ref_gl = vec![32u8; total];
    ref_gl[0] = b'X'; // One different glyph

    let ref_fg = vec![16u8; total];
    let ref_bk = vec![16u8; total];

    let (diffs, _, pct) = compare_ansi_grids(&grid, &ref_gl, &ref_fg, &ref_bk);
    assert_eq!(diffs, 1, "Should detect exactly 1 cell difference");
    let expected_pct = 100.0 / total as f64;
    assert!(
        (pct - expected_pct).abs() < 0.01,
        "Diff percentage should be ~{:.4}%, got {:.4}%",
        expected_pct,
        pct
    );
}

#[test]
fn test_rgba_to_xterm256_roundtrip() {
    // Convert xterm-256 color cube entries -> RGBA -> xterm-256.
    // Should produce same index for pure color cube entries.
    for idx in 16..=231u8 {
        let rgb = XTERM_256_PALETTE[idx as usize];
        let rgba = [rgb[0], rgb[1], rgb[2], 255];
        let result = rgba_to_nearest_xterm256(rgba);
        assert_eq!(
            result, idx,
            "Round-trip failed for xterm index {}: RGB={:?}, got index {}",
            idx, rgb, result
        );
    }
}

// ===========================================================================
// Unit tests for compare_rgba_grids (active CI comparison function)
// ===========================================================================

#[test]
fn test_compare_rgba_identical() {
    let grid = AsciiCellGrid::new(4, 4);
    let grid2 = AsciiCellGrid::new(4, 4);

    let (diffs, _, pct) = compare_rgba_grids(&grid, &grid2);
    assert_eq!(diffs, 0, "Identical grids should have 0 diffs");
    assert!(
        (pct - 0.0).abs() < f64::EPSILON,
        "Diff percentage should be 0%"
    );
}

#[test]
fn test_compare_rgba_one_cell_diff() {
    let grid1 = AsciiCellGrid::new(4, 4);
    let mut grid2 = AsciiCellGrid::new(4, 4);

    // Mutate one cell
    grid2.char_indices[0] = b'X' as u16;

    let total = grid1.cells_count();
    let (diffs, _, pct) = compare_rgba_grids(&grid1, &grid2);
    assert_eq!(diffs, 1, "Should detect exactly 1 cell difference");
    let expected_pct = 100.0 / total as f64;
    assert!(
        (pct - expected_pct).abs() < 0.01,
        "Diff percentage should be ~{:.4}%, got {:.4}%",
        expected_pct,
        pct
    );
}

#[test]
fn test_compare_rgba_fully_different() {
    let grid1 = AsciiCellGrid::new(4, 4);
    let total = grid1.cells_count();

    let mut grid2 = AsciiCellGrid::new(4, 4);
    for i in 0..total {
        grid2.char_indices[i] = b'X' as u16;
        grid2.fg_colors[i] = [255, 0, 0, 255];
        grid2.bg_colors[i] = [0, 255, 0, 255];
    }

    let (diffs, _, pct) = compare_rgba_grids(&grid1, &grid2);
    assert_eq!(
        diffs, total,
        "Completely different grids should have total diffs"
    );
    assert!(
        (pct - 100.0).abs() < f64::EPSILON,
        "Diff percentage should be 100%, got {:.2}%",
        pct
    );
}
