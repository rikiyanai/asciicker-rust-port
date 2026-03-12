use asciicker_engine::asset_loader::a3d_world::{WorldInstance, parse_world_section};
use asciicker_engine::asset_loader::constants::{
    A3D_HEADER_SIZE, FILE_PATCH_SIZE, MATERIAL_TABLE_SIZE,
};

/// Helper: given a full .a3d file, return the world section slice.
/// World section starts after: header (16) + patches * 188 + material table (131072).
fn world_section_from_a3d(data: &[u8]) -> &[u8] {
    let num_patches = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
    let offset = A3D_HEADER_SIZE as usize + num_patches * FILE_PATCH_SIZE + MATERIAL_TABLE_SIZE;
    &data[offset..]
}

#[test]
fn test_parse_test_map_world() {
    let data = include_bytes!("golden/a3d/test_map.a3d");
    let world_data = world_section_from_a3d(data);
    let world = parse_world_section(world_data).expect("should parse test_map world section");

    // test_map.a3d has 3 mesh instances
    assert_eq!(world.instances.len(), 3);
    for inst in &world.instances {
        assert!(
            matches!(inst, WorldInstance::Mesh { .. }),
            "all instances in test_map should be Mesh variants"
        );
    }
}

#[test]
fn test_parse_test_map_no_terrain_world() {
    let data = include_bytes!("golden/a3d/test_map_no_terrain.a3d");
    let world_data = world_section_from_a3d(data);
    let world =
        parse_world_section(world_data).expect("should parse test_map_no_terrain world section");

    // test_map_no_terrain.a3d has 19 mesh instances, format v1 with story_id
    assert_eq!(world.instances.len(), 19);
    for inst in &world.instances {
        assert!(
            matches!(inst, WorldInstance::Mesh { .. }),
            "all instances in test_map_no_terrain should be Mesh variants"
        );
    }
}

#[test]
fn test_format_version_detection() {
    let data = include_bytes!("golden/a3d/test_map_no_terrain.a3d");
    let world_data = world_section_from_a3d(data);
    let world =
        parse_world_section(world_data).expect("should parse test_map_no_terrain world section");

    // test_map_no_terrain.a3d is a newer file with format_version > 0
    assert!(
        world.format_version > 0,
        "test_map_no_terrain should have format_version > 0, got {}",
        world.format_version
    );
    assert_eq!(world.format_version, 1);
}

#[test]
fn test_mesh_instance_fields() {
    let data = include_bytes!("golden/a3d/test_map.a3d");
    let world_data = world_section_from_a3d(data);
    let world = parse_world_section(world_data).expect("should parse test_map world section");

    // Verify first mesh instance has expected fields
    match &world.instances[0] {
        WorldInstance::Mesh {
            mesh_id,
            inst_name,
            tm,
            flags,
            story_id,
        } => {
            assert_eq!(mesh_id, "umbilic_torus_with_a_twist.akm");
            assert_eq!(inst_name, "umbilic_torus_with_a_twist");
            assert_eq!(tm.len(), 16);
            // tm[15] should be 1.0 (homogeneous coordinate)
            assert!((tm[15] - 1.0).abs() < f64::EPSILON);
            assert_eq!(*flags, 3);
            assert_eq!(*story_id, -1);
        }
        _ => panic!("expected Mesh variant"),
    }
}

#[test]
fn test_ply_to_akm_conversion() {
    // Construct a synthetic world section with a mesh whose mesh_id ends in ".ply"
    let mut buf: Vec<u8> = Vec::new();

    // format_version: -1 (version 1)
    buf.extend_from_slice(&(-1i32).to_le_bytes());
    // count: 1
    buf.extend_from_slice(&1i32.to_le_bytes());

    // mesh_id_len
    let mesh_id = b"test_mesh.ply";
    buf.extend_from_slice(&(mesh_id.len() as i32).to_le_bytes());
    buf.extend_from_slice(mesh_id);

    // inst_name with length prefix
    let inst_name = b"test_inst";
    buf.extend_from_slice(&(inst_name.len() as i32).to_le_bytes());
    buf.extend_from_slice(inst_name);

    // tm: 16 f64 values (identity-ish)
    for i in 0..16u32 {
        let val: f64 = if i % 5 == 0 { 1.0 } else { 0.0 };
        buf.extend_from_slice(&val.to_le_bytes());
    }

    // flags
    buf.extend_from_slice(&0i32.to_le_bytes());
    // story_id (format_version > 0)
    buf.extend_from_slice(&42i32.to_le_bytes());

    let world = parse_world_section(&buf).expect("should parse synthetic world");

    match &world.instances[0] {
        WorldInstance::Mesh { mesh_id, .. } => {
            assert!(
                mesh_id.ends_with(".akm"),
                "mesh_id should have .ply converted to .akm, got: {}",
                mesh_id
            );
            assert_eq!(mesh_id, "test_mesh.akm");
        }
        _ => panic!("expected Mesh variant"),
    }
}

#[test]
fn test_unknown_instance_type_error() {
    // Construct bytes with mesh_id_len = -3 (unknown)
    let mut buf: Vec<u8> = Vec::new();

    // format_version = 0 (legacy), count = 1
    buf.extend_from_slice(&1i32.to_le_bytes());

    // mesh_id_len = -3 (unknown)
    buf.extend_from_slice(&(-3i32).to_le_bytes());

    let result = parse_world_section(&buf);
    assert!(
        result.is_err(),
        "should return error for unknown instance type"
    );

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("unknown instance type"),
        "error should mention unknown instance type, got: {}",
        err_msg
    );
}
