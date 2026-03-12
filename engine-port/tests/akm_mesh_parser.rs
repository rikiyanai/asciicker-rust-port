use asciicker_engine::asset_loader::akm_mesh::parse_akm;

#[test]
fn test_parse_cube_header() {
    let text = include_str!("golden/akm/Cube.akm");
    let mesh = parse_akm(text).expect("should parse Cube.akm");

    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.faces.len(), 12);
    assert!(mesh.edges.is_empty());
}

#[test]
fn test_vertex_properties() {
    let text = include_str!("golden/akm/Cube.akm");
    let mesh = parse_akm(text).expect("should parse Cube.akm");

    // First vertex: -1.000 1.000 1.000 ... 231 231 231 0
    let v0 = &mesh.vertices[0];
    assert!((v0.x - (-1.0)).abs() < f32::EPSILON);
    assert!((v0.y - 1.0).abs() < f32::EPSILON);
    assert!((v0.z - 1.0).abs() < f32::EPSILON);
    assert_eq!(v0.r, 231);
    assert_eq!(v0.g, 231);
    assert_eq!(v0.b, 231);
    assert_eq!(v0.alpha, 0);

    // Property skipping works: normals and UVs present in Cube.akm are ignored.
    // If they were NOT skipped, vertex data would be garbled.
    let v1 = &mesh.vertices[1];
    assert!((v1.x - 1.0).abs() < f32::EPSILON);
    assert!((v1.y - (-1.0)).abs() < f32::EPSILON);
    assert!((v1.z - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_face_indices() {
    let text = include_str!("golden/akm/Cube.akm");
    let mesh = parse_akm(text).expect("should parse Cube.akm");

    // All faces should have 3 indices each, all in range [0, 23]
    for (i, face) in mesh.faces.iter().enumerate() {
        for idx in face.indices {
            assert!(
                idx < 24,
                "face {} has out-of-range index {}, max is 23",
                i,
                idx
            );
        }
        assert!(!face.freestyle, "Cube.akm faces should not be freestyle");
    }

    // First face: 3 0 1 2
    assert_eq!(mesh.faces[0].indices, [0, 1, 2]);
    // Second face: 3 3 4 5
    assert_eq!(mesh.faces[1].indices, [3, 4, 5]);
}

#[test]
fn test_not_ply_error() {
    let result = parse_akm("not a ply file\nsome data\n");
    assert!(result.is_err(), "should return error for non-PLY text");

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("not a PLY file"),
        "error should mention 'not a PLY file', got: {}",
        err_msg
    );
}

#[test]
fn test_freestyle_negative_count() {
    let ply_text = "\
ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
property uchar red
property uchar green
property uchar blue
property uchar alpha
element face 1
property list uchar uint vertex_indices
end_header
0.0 0.0 0.0 255 0 0 255
1.0 0.0 0.0 0 255 0 255
0.0 1.0 0.0 0 0 255 255
-3 0 1 2
";
    let mesh = parse_akm(ply_text).expect("should parse freestyle PLY");

    assert_eq!(mesh.faces.len(), 1);
    assert!(mesh.faces[0].freestyle, "face should be marked freestyle");
    assert_eq!(mesh.faces[0].indices, [0, 1, 2]);
}

#[test]
fn test_two_vertex_edge() {
    let ply_text = "\
ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
property uchar red
property uchar green
property uchar blue
property uchar alpha
element face 1
property list uchar uint vertex_indices
end_header
0.0 0.0 0.0 255 0 0 255
1.0 0.0 0.0 0 255 0 255
2 0 1
";
    let mesh = parse_akm(ply_text).expect("should parse edge PLY");

    assert!(
        mesh.faces.is_empty(),
        "2-vertex lines should not produce faces"
    );
    assert_eq!(mesh.edges.len(), 1);
    assert_eq!(mesh.edges[0].v0, 0);
    assert_eq!(mesh.edges[0].v1, 1);
}
