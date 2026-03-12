//! AKM mesh parser (ASCII PLY format).
//!
//! Parses `.akm` files exported from Blender via the `io_asciicker` addon.
//! The format is standard ASCII PLY with Asciicker-specific conventions:
//! - Vertex properties: x, y, z, r, g, b, alpha (with normals/UVs skipped)
//! - Face lines: `count v0 v1 v2 [visual]`
//! - Negative vertex count marks a face as freestyle (wireframe)
//! - Two-vertex faces are parsed as edges

use super::error::AssetError;

/// A single vertex in an AKM mesh.
#[derive(Debug, Clone)]
pub struct AkmVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub alpha: u8,
}

/// A triangular face in an AKM mesh.
#[derive(Debug, Clone)]
pub struct AkmFace {
    pub indices: [u32; 3],
    pub visual: u32,
    pub freestyle: bool,
}

/// A freestyle edge (2-vertex line) in an AKM mesh.
#[derive(Debug, Clone)]
pub struct AkmEdge {
    pub v0: u32,
    pub v1: u32,
}

/// Parsed AKM mesh (ASCII PLY format from Blender export).
#[derive(Debug, Clone, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct AkmMesh {
    pub vertices: Vec<AkmVertex>,
    pub faces: Vec<AkmFace>,
    pub edges: Vec<AkmEdge>,
}

/// Property type codes for the flexible vertex property mapper.
/// Unknown properties get code 0 (skip).
const PROP_SKIP: u8 = 0;
const PROP_X: u8 = 1;
const PROP_Y: u8 = 2;
const PROP_Z: u8 = 3;
const PROP_RED: u8 = 4;
const PROP_GREEN: u8 = 5;
const PROP_BLUE: u8 = 6;
const PROP_ALPHA: u8 = 7;

/// Map a PLY vertex property name to a property code.
fn property_code(name: &str) -> u8 {
    match name {
        "x" => PROP_X,
        "y" => PROP_Y,
        "z" => PROP_Z,
        "red" | "diffuse_red" => PROP_RED,
        "green" | "diffuse_green" => PROP_GREEN,
        "blue" | "diffuse_blue" => PROP_BLUE,
        "alpha" => PROP_ALPHA,
        _ => PROP_SKIP,
    }
}

/// Parse an AKM file (ASCII PLY format).
///
/// # Errors
///
/// - `AssetError::EmptyFile` if input is empty
/// - `AssetError::NotPly` if first line is not "ply"
/// - `AssetError::UnsupportedPlyFormat` if format is not "ascii 1.0"
/// - `AssetError::Parse` for malformed data lines
pub fn parse_akm(text: &str) -> Result<AkmMesh, AssetError> {
    if text.is_empty() {
        return Err(AssetError::EmptyFile);
    }

    let mut lines = text.lines();

    // Validate "ply" header
    let first = lines.next().ok_or(AssetError::EmptyFile)?;
    if first.trim() != "ply" {
        return Err(AssetError::NotPly);
    }

    // Validate "format ascii 1.0"
    let format_line = lines
        .next()
        .ok_or(AssetError::Parse("missing format line".to_string()))?;
    if format_line.trim() != "format ascii 1.0" {
        return Err(AssetError::UnsupportedPlyFormat);
    }

    // Parse element/property declarations
    let mut prop_types: Vec<u8> = Vec::new();
    let mut num_verts: usize = 0;
    let mut num_faces: usize = 0;
    let mut current_element = ' ';
    let mut has_alpha = false;

    for line in &mut lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") {
            continue;
        }
        if line == "end_header" {
            break;
        }

        if let Some(rest) = line.strip_prefix("element vertex ") {
            num_verts = rest
                .trim()
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad vertex count: {e}")))?;
            current_element = 'V';
        } else if let Some(rest) = line.strip_prefix("element face ") {
            num_faces = rest
                .trim()
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad face count: {e}")))?;
            current_element = 'F';
        } else if let Some(rest) = line.strip_prefix("property ")
            && current_element == 'V'
        {
            // Property lines: "property <type> <name>"
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let code = property_code(parts[1]);
                if code == PROP_ALPHA {
                    has_alpha = true;
                }
                prop_types.push(code);
            }
        }
    }

    // Parse vertex data
    let mut vertices = Vec::with_capacity(num_verts);
    for _ in 0..num_verts {
        let line = lines.next().ok_or(AssetError::Parse(
            "unexpected end of vertex data".to_string(),
        ))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut z: f32 = 0.0;
        let mut r: u8 = 0;
        let mut g: u8 = 0;
        let mut b: u8 = 0;
        let mut alpha: u8 = if has_alpha { 0 } else { 255 };

        for (i, &prop) in prop_types.iter().enumerate() {
            if i >= tokens.len() {
                break;
            }
            match prop {
                PROP_X => {
                    x = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad x: {e}")))?;
                }
                PROP_Y => {
                    y = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad y: {e}")))?;
                }
                PROP_Z => {
                    z = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad z: {e}")))?;
                }
                PROP_RED => {
                    r = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad red: {e}")))?;
                }
                PROP_GREEN => {
                    g = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad green: {e}")))?;
                }
                PROP_BLUE => {
                    b = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad blue: {e}")))?;
                }
                PROP_ALPHA => {
                    alpha = tokens[i]
                        .parse()
                        .map_err(|e| AssetError::Parse(format!("bad alpha: {e}")))?;
                }
                _ => {} // PROP_SKIP: ignore normals, UVs, etc.
            }
        }

        vertices.push(AkmVertex {
            x,
            y,
            z,
            r,
            g,
            b,
            alpha,
        });
    }

    // Parse face data
    let mut faces = Vec::with_capacity(num_faces);
    let mut edges = Vec::new();

    for _ in 0..num_faces {
        let line = lines
            .next()
            .ok_or(AssetError::Parse("unexpected end of face data".to_string()))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let raw_count: i32 = tokens[0]
            .parse()
            .map_err(|e| AssetError::Parse(format!("bad face vertex count: {e}")))?;

        let freestyle = raw_count < 0;
        let abs_count = raw_count.unsigned_abs();

        if abs_count == 2 && tokens.len() >= 3 {
            // Edge (2-vertex line)
            let v0: u32 = tokens[1]
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad edge v0: {e}")))?;
            let v1: u32 = tokens[2]
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad edge v1: {e}")))?;
            edges.push(AkmEdge { v0, v1 });
        } else if abs_count >= 3 && tokens.len() >= 4 {
            // Triangle face
            let i0: u32 = tokens[1]
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad face i0: {e}")))?;
            let i1: u32 = tokens[2]
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad face i1: {e}")))?;
            let i2: u32 = tokens[3]
                .parse()
                .map_err(|e| AssetError::Parse(format!("bad face i2: {e}")))?;

            let visual: u32 = if tokens.len() >= 5 {
                tokens[4].parse().unwrap_or(0)
            } else {
                0
            };

            faces.push(AkmFace {
                indices: [i0, i1, i2],
                visual,
                freestyle,
            });
        }
    }

    Ok(AkmMesh {
        vertices,
        faces,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_code_mapping() {
        assert_eq!(property_code("x"), PROP_X);
        assert_eq!(property_code("y"), PROP_Y);
        assert_eq!(property_code("z"), PROP_Z);
        assert_eq!(property_code("red"), PROP_RED);
        assert_eq!(property_code("diffuse_red"), PROP_RED);
        assert_eq!(property_code("green"), PROP_GREEN);
        assert_eq!(property_code("diffuse_green"), PROP_GREEN);
        assert_eq!(property_code("blue"), PROP_BLUE);
        assert_eq!(property_code("diffuse_blue"), PROP_BLUE);
        assert_eq!(property_code("alpha"), PROP_ALPHA);
        assert_eq!(property_code("nx"), PROP_SKIP);
        assert_eq!(property_code("s"), PROP_SKIP);
    }

    #[test]
    fn test_empty_file() {
        assert!(parse_akm("").is_err());
    }
}
