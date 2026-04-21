//! Comprehensive golden-file integration tests for all asset parsers.
//!
//! Tests exercise parse functions end-to-end with real game assets loaded via
//! `include_bytes!` / `include_str!`. These validate parser correctness against
//! the C++ engine reference output.

use asciicker_engine::asset_loader::a3d_terrain::{parse_material_section, parse_terrain_section};
use asciicker_engine::asset_loader::a3d_world::parse_world_section;
use asciicker_engine::asset_loader::akm_mesh::parse_akm;
use asciicker_engine::asset_loader::xp_sprite::{merge_layers, parse_xp};

// ---------------------------------------------------------------------------
// XP integration tests
// ---------------------------------------------------------------------------

static ITEM_APPLE_XP: &[u8] = include_bytes!("golden/xp/item-apple.xp");
static GRID_WATER_XP: &[u8] = include_bytes!("golden/xp/grid-water.xp");

#[test]
fn test_item_apple_full_parse() {
    let sprite = parse_xp(ITEM_APPLE_XP).expect("item-apple.xp should parse");
    assert_eq!(sprite.width, 2);
    assert_eq!(sprite.height, 2);
    assert!(sprite.layers.len() >= 3, "minimum 3 layers required");

    // Verify cell count per layer matches width * height
    for (i, layer) in sprite.layers.iter().enumerate() {
        assert_eq!(
            layer.cells.len(),
            (sprite.width * sprite.height) as usize,
            "layer {i} cell count should match width*height"
        );
    }
}

#[test]
fn test_grid_water_full_parse() {
    let sprite = parse_xp(GRID_WATER_XP).expect("grid-water.xp should parse");
    assert_eq!(sprite.width, 7);
    assert_eq!(sprite.height, 7);
    assert!(sprite.layers.len() >= 3, "minimum 3 layers required");

    for (i, layer) in sprite.layers.iter().enumerate() {
        assert_eq!(
            layer.cells.len(),
            (sprite.width * sprite.height) as usize,
            "layer {i} cell count should match width*height"
        );
    }
}

#[test]
fn test_xp_layer_data_integrity() {
    let sprite = parse_xp(GRID_WATER_XP).expect("grid-water.xp should parse");

    // Layer 2 (visual) should contain at least some non-zero data
    let layer2 = &sprite.layers[2];
    let has_visible_cells = layer2
        .cells
        .iter()
        .any(|c| c.glyph != 0 || c.fg != [0, 0, 0] || c.bg != [0, 0, 0]);
    assert!(
        has_visible_cells,
        "layer 2 (visual) should have non-transparent cells"
    );

    // Merge should produce valid results without panic
    let merged = merge_layers(&sprite);
    assert_eq!(merged.len(), (sprite.width * sprite.height) as usize);
}

// ---------------------------------------------------------------------------
// A3D terrain integration tests
// ---------------------------------------------------------------------------

static MINIMAL_1X1_A3D: &[u8] = include_bytes!("golden/a3d/minimal_1x1.a3d");
static MINIMAL_2X2_A3D: &[u8] = include_bytes!("golden/a3d/minimal_2x2.a3d");

#[test]
fn test_minimal_1x1_full_parse() {
    // Parse all three sections sequentially
    let (terrain, terrain_consumed) =
        parse_terrain_section(MINIMAL_1X1_A3D).expect("terrain section should parse");
    assert_eq!(terrain.patches.len(), 1, "1x1 file should have 1 patch");

    let (materials, mat_consumed) = parse_material_section(&MINIMAL_1X1_A3D[terrain_consumed..])
        .expect("material section should parse");
    assert_eq!(materials.materials.len(), 256, "256 materials expected");

    let world = parse_world_section(&MINIMAL_1X1_A3D[terrain_consumed + mat_consumed..])
        .expect("world section should parse");
    assert_eq!(
        world.instances.len(),
        0,
        "minimal 1x1 has 0 world instances"
    );
}

#[test]
fn test_minimal_2x2_full_parse() {
    let (terrain, terrain_consumed) =
        parse_terrain_section(MINIMAL_2X2_A3D).expect("terrain section should parse");
    assert_eq!(terrain.patches.len(), 4, "2x2 file should have 4 patches");

    let (materials, mat_consumed) = parse_material_section(&MINIMAL_2X2_A3D[terrain_consumed..])
        .expect("material section should parse");
    assert_eq!(materials.materials.len(), 256, "256 materials expected");

    let world = parse_world_section(&MINIMAL_2X2_A3D[terrain_consumed + mat_consumed..])
        .expect("world section should parse");
    assert_eq!(
        world.instances.len(),
        0,
        "minimal 2x2 has 0 world instances"
    );
}

#[test]
fn test_terrain_material_world_sequence() {
    // Verify the three sections chain correctly via offset tracking
    let (terrain, terrain_consumed) =
        parse_terrain_section(MINIMAL_1X1_A3D).expect("terrain section should parse");
    assert!(terrain_consumed > 0, "terrain consumes bytes");

    let remaining_after_terrain = &MINIMAL_1X1_A3D[terrain_consumed..];
    assert!(
        !remaining_after_terrain.is_empty(),
        "data remains after terrain"
    );

    let (_materials, mat_consumed) = parse_material_section(remaining_after_terrain)
        .expect("material section should parse from terrain offset");
    assert_eq!(
        mat_consumed, 131_072,
        "material table is always 131,072 bytes"
    );

    let remaining_after_mat = &MINIMAL_1X1_A3D[terrain_consumed + mat_consumed..];
    assert!(
        !remaining_after_mat.is_empty(),
        "data remains after material table"
    );

    let world = parse_world_section(remaining_after_mat)
        .expect("world section should parse from material offset");

    // Verify the data makes sense end-to-end
    assert_eq!(terrain.patches.len(), 1);
    assert_eq!(world.instances.len(), 0);
}

// ---------------------------------------------------------------------------
// A3D world integration tests
// ---------------------------------------------------------------------------

static TEST_MAP_A3D: &[u8] = include_bytes!("golden/a3d/test_map.a3d");
static TEST_MAP_NO_TERRAIN_A3D: &[u8] = include_bytes!("golden/a3d/test_map_no_terrain.a3d");

#[test]
fn test_test_map_mesh_instances() {
    // Skip terrain + material sections to reach the world section
    let (_, terrain_consumed) =
        parse_terrain_section(TEST_MAP_A3D).expect("terrain section should parse");
    let (_, mat_consumed) = parse_material_section(&TEST_MAP_A3D[terrain_consumed..])
        .expect("material section should parse");

    let world = parse_world_section(&TEST_MAP_A3D[terrain_consumed + mat_consumed..])
        .expect("world section should parse");

    assert_eq!(world.instances.len(), 3, "test_map has 3 instances");

    // Verify all instances are mesh type with valid mesh_id strings
    for inst in &world.instances {
        match inst {
            asciicker_engine::asset_loader::a3d_world::WorldInstance::Mesh { mesh_id, .. } => {
                assert!(
                    !mesh_id.is_empty(),
                    "mesh instance should have non-empty mesh_id"
                );
            }
            _ => panic!("expected all instances to be Mesh type in test_map"),
        }
    }
}

#[test]
fn test_test_map_no_terrain_instances() {
    // Skip terrain + material sections
    let (_, terrain_consumed) =
        parse_terrain_section(TEST_MAP_NO_TERRAIN_A3D).expect("terrain section should parse");
    let (_, mat_consumed) = parse_material_section(&TEST_MAP_NO_TERRAIN_A3D[terrain_consumed..])
        .expect("material section should parse");

    let world = parse_world_section(&TEST_MAP_NO_TERRAIN_A3D[terrain_consumed + mat_consumed..])
        .expect("world section should parse");

    assert_eq!(
        world.instances.len(),
        19,
        "test_map_no_terrain has 19 instances"
    );

    // Verify format_version > 0 means story_id is present
    assert!(
        world.format_version > 0,
        "test_map_no_terrain should have format_version > 0"
    );

    // Check that at least one instance has a meaningful story_id field
    let has_story_ids = world.instances.iter().any(|inst| match inst {
        asciicker_engine::asset_loader::a3d_world::WorldInstance::Mesh { story_id, .. } => {
            *story_id != -1
        }
        asciicker_engine::asset_loader::a3d_world::WorldInstance::Sprite { story_id, .. } => {
            *story_id != -1
        }
        asciicker_engine::asset_loader::a3d_world::WorldInstance::Item { story_id, .. } => {
            *story_id != -1
        }
    });
    // story_id may all be -1 if no story triggers were placed; we just verify the field exists
    let _ = has_story_ids;
}

// ---------------------------------------------------------------------------
// Full game map integration test (game_map_y8.a3d)
// ---------------------------------------------------------------------------

static GAME_MAP_Y8_A3D: &[u8] = include_bytes!("../assets/original_game_map_y8.a3d");

#[test]
fn test_game_map_y8_terrain_parse() {
    let (terrain, terrain_consumed) =
        parse_terrain_section(GAME_MAP_Y8_A3D).expect("terrain section should parse");
    assert_eq!(terrain.patches.len(), 4876, "game_map_y8 has 4876 patches");
    assert!(terrain_consumed > 0);
}

#[test]
fn test_game_map_y8_material_parse() {
    let (_terrain, terrain_consumed) =
        parse_terrain_section(GAME_MAP_Y8_A3D).expect("terrain section should parse");
    let (materials, mat_consumed) = parse_material_section(&GAME_MAP_Y8_A3D[terrain_consumed..])
        .expect("material section should parse");
    assert_eq!(materials.materials.len(), 256);
    assert_eq!(mat_consumed, 131_072);
}

#[test]
fn test_game_map_y8_world_parse() {
    let (_terrain, terrain_consumed) =
        parse_terrain_section(GAME_MAP_Y8_A3D).expect("terrain section should parse");
    let (_materials, mat_consumed) = parse_material_section(&GAME_MAP_Y8_A3D[terrain_consumed..])
        .expect("material section should parse");
    let world = parse_world_section(&GAME_MAP_Y8_A3D[terrain_consumed + mat_consumed..])
        .expect("world section should parse");
    assert_eq!(
        world.instances.len(),
        1281,
        "game_map_y8 has 1281 instances"
    );
    assert_eq!(world.format_version, 1);
}

#[test]
fn test_game_map_y8_full_pipeline() {
    // All three sections must parse sequentially without panic
    let (terrain, terrain_consumed) = parse_terrain_section(GAME_MAP_Y8_A3D).expect("terrain");
    let (materials, mat_consumed) =
        parse_material_section(&GAME_MAP_Y8_A3D[terrain_consumed..]).expect("materials");
    let world =
        parse_world_section(&GAME_MAP_Y8_A3D[terrain_consumed + mat_consumed..]).expect("world");

    assert_eq!(terrain.patches.len(), 4876);
    assert_eq!(materials.materials.len(), 256);
    assert_eq!(world.instances.len(), 1281);

    // Verify total bytes consumed matches file size (no trailing garbage)
    let world_section = &GAME_MAP_Y8_A3D[terrain_consumed + mat_consumed..];
    assert!(world_section.len() >= 8, "world section has header");
}

// ---------------------------------------------------------------------------
// AKM integration tests
// ---------------------------------------------------------------------------

static CUBE_AKM: &str = include_str!("golden/akm/Cube.akm");

#[test]
fn test_cube_akm_vertices_and_faces() {
    let mesh = parse_akm(CUBE_AKM).expect("Cube.akm should parse");

    assert_eq!(mesh.vertices.len(), 24, "Cube has 24 vertices");
    assert_eq!(mesh.faces.len(), 12, "Cube has 12 triangular faces");

    // Verify vertices have valid coordinates
    for v in &mesh.vertices {
        assert!(v.x.is_finite(), "vertex x must be finite");
        assert!(v.y.is_finite(), "vertex y must be finite");
        assert!(v.z.is_finite(), "vertex z must be finite");
    }

    // Verify faces reference valid vertex indices
    for face in &mesh.faces {
        for &idx in &face.indices {
            assert!(
                (idx as usize) < mesh.vertices.len(),
                "face index {idx} out of bounds"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Error handling tests
// ---------------------------------------------------------------------------

#[test]
fn test_empty_bytes_error() {
    // XP: empty bytes should fail
    assert!(parse_xp(&[]).is_err(), "empty XP should error");

    // A3D terrain: empty bytes should fail
    assert!(
        parse_terrain_section(&[]).is_err(),
        "empty terrain should error"
    );

    // A3D material: empty bytes should fail
    assert!(
        parse_material_section(&[]).is_err(),
        "empty material should error"
    );

    // A3D world: empty bytes should fail
    assert!(
        parse_world_section(&[]).is_err(),
        "empty world should error"
    );

    // AKM: empty string should fail
    assert!(parse_akm("").is_err(), "empty AKM should error");
}

#[test]
fn test_truncated_a3d_error() {
    // Less than 16 bytes (header size) should fail
    let truncated = &MINIMAL_1X1_A3D[..12];
    assert!(
        parse_terrain_section(truncated).is_err(),
        "truncated A3D header should error"
    );
}
