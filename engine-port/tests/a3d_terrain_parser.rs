use asciicker_engine::asset_loader::a3d_terrain::{
    parse_material_section, parse_terrain_section, A3dTerrain, MaterialTable,
};
use asciicker_engine::asset_loader::constants::{
    A3D_MAGIC, FILE_PATCH_SIZE, HEIGHT_CELLS, MATERIAL_TABLE_SIZE, VISUAL_CELLS,
};
use asciicker_engine::asset_loader::AssetError;

/// Helper: read a golden .a3d file and return its bytes.
fn load_golden(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/golden/a3d/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// ---- Test 1: minimal_1x1 produces exactly 1 patch at (0,0) ----

#[test]
fn test_parse_minimal_1x1_terrain() {
    let data = load_golden("minimal_1x1.a3d");
    let (terrain, consumed) = parse_terrain_section(&data).expect("parse should succeed");

    assert_eq!(terrain.patches.len(), 1, "expected exactly 1 patch");
    assert_eq!(terrain.patches[0].x, 0);
    assert_eq!(terrain.patches[0].y, 0);

    // consumed = header (16) + 1 patch (188) = 204
    let expected_consumed = 16 + 1 * FILE_PATCH_SIZE;
    assert_eq!(consumed, expected_consumed);
}

// ---- Test 2: minimal_2x2 produces exactly 4 patches ----

#[test]
fn test_parse_minimal_2x2_terrain() {
    let data = load_golden("minimal_2x2.a3d");
    let (terrain, consumed) = parse_terrain_section(&data).expect("parse should succeed");

    assert_eq!(terrain.patches.len(), 4, "expected exactly 4 patches");

    // consumed = header (16) + 4 patches (4 * 188) = 768
    let expected_consumed = 16 + 4 * FILE_PATCH_SIZE;
    assert_eq!(consumed, expected_consumed);
}

// ---- Test 3: each patch has 5x5 height grid ----

#[test]
fn test_patch_height_grid_dimensions() {
    let data = load_golden("minimal_1x1.a3d");
    let (terrain, _) = parse_terrain_section(&data).expect("parse should succeed");

    let patch = &terrain.patches[0];
    // height is [[u16; HEIGHT_CELLS+1]; HEIGHT_CELLS+1] = [[u16; 5]; 5]
    assert_eq!(patch.height.len(), HEIGHT_CELLS + 1, "height rows");
    for row in &patch.height {
        assert_eq!(row.len(), HEIGHT_CELLS + 1, "height cols");
    }
}

// ---- Test 4: each patch has 8x8 visual grid ----

#[test]
fn test_patch_visual_grid_dimensions() {
    let data = load_golden("minimal_1x1.a3d");
    let (terrain, _) = parse_terrain_section(&data).expect("parse should succeed");

    let patch = &terrain.patches[0];
    // visual is [[u16; VISUAL_CELLS]; VISUAL_CELLS] = [[u16; 8]; 8]
    assert_eq!(patch.visual.len(), VISUAL_CELLS, "visual rows");
    for row in &patch.visual {
        assert_eq!(row.len(), VISUAL_CELLS, "visual cols");
    }
}

// ---- Test 5: wrong magic produces BadMagic error ----

#[test]
fn test_magic_validation() {
    // Create a 16-byte header with wrong magic
    let mut bad_data = vec![0u8; 16 + FILE_PATCH_SIZE];
    // Write a wrong magic (0xDEADBEEF) in little-endian
    bad_data[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
    // Write header_size = 16
    bad_data[4..8].copy_from_slice(&16u32.to_le_bytes());
    // Write num_patches = 1
    bad_data[8..12].copy_from_slice(&1u32.to_le_bytes());

    let result = parse_terrain_section(&bad_data);
    assert!(result.is_err(), "should fail on bad magic");

    let err = result.unwrap_err();
    let err_msg = format!("{err}");
    assert!(
        err_msg.contains("magic"),
        "error should mention magic: {err_msg}"
    );
}

// ---- Test 6: material table parses 256 entries ----

#[test]
fn test_material_table_parse() {
    let data = load_golden("minimal_1x1.a3d");
    let (_, terrain_consumed) = parse_terrain_section(&data).expect("terrain parse should succeed");

    let mat_data = &data[terrain_consumed..];
    let (table, mat_consumed) =
        parse_material_section(mat_data).expect("material parse should succeed");

    assert_eq!(mat_consumed, MATERIAL_TABLE_SIZE);
    assert_eq!(table.materials.len(), 256, "expected 256 material entries");

    // Each entry has 4 elevations x 16 diffuse levels
    for (i, entry) in table.materials.iter().enumerate() {
        assert_eq!(entry.len(), 4, "material {i}: expected 4 elevations");
        for (e, elev) in entry.iter().enumerate() {
            assert_eq!(
                elev.len(),
                16,
                "material {i} elevation {e}: expected 16 diffuse levels"
            );
        }
    }

    // Verify MatCell structure: fg [u8;3], gl u8, bg [u8;3], flags u8 = 8 bytes
    let cell = &table.materials[0][0][0];
    // Just verify we can access all fields (structural test)
    let _fg = cell.fg;
    let _gl = cell.gl;
    let _bg = cell.bg;
    let _flags = cell.flags;
}

// ---- Test 7: terrain consumed bytes allows correct material offset ----

#[test]
fn test_terrain_then_materials_offset() {
    let data = load_golden("minimal_1x1.a3d");
    let (_, terrain_consumed) = parse_terrain_section(&data).expect("terrain parse should succeed");

    // After terrain, material table should start at the right offset
    let remaining = &data[terrain_consumed..];
    assert!(
        remaining.len() >= MATERIAL_TABLE_SIZE,
        "not enough bytes for material table after terrain: {} < {}",
        remaining.len(),
        MATERIAL_TABLE_SIZE
    );

    // Parse material section from correct offset
    let (table, mat_consumed) =
        parse_material_section(remaining).expect("material parse should succeed");

    assert_eq!(mat_consumed, MATERIAL_TABLE_SIZE);
    assert_eq!(table.materials.len(), 256);

    // After both terrain and materials, remaining bytes are the world section
    let world_offset = terrain_consumed + mat_consumed;
    assert!(
        world_offset <= data.len(),
        "terrain + materials should not exceed file size"
    );

    // For minimal_1x1: 16 + 188 + 131072 = 131276, file is 131288, so 12 bytes remaining
    let world_bytes = data.len() - world_offset;
    assert!(world_bytes > 0, "should have world section bytes remaining");
}
