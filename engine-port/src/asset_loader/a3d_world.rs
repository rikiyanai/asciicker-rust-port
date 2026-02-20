//! A3D world section parser.
//!
//! Parses the world instance list from an .a3d file. The world section
//! contains mesh, sprite, and item instances that populate the game world.
//!
//! Format: The first i32 encodes the format version. If negative,
//! `format_version = -value` and the next i32 is the instance count.
//! If non-negative, `format_version = 0` and the value is the count.
//!
//! Three instance variants are discriminated by `mesh_id_len`:
//! - `>= 0`: mesh instance
//! - `-1`: sprite instance
//! - `-2`: item instance

use super::error::AssetError;

/// A single world instance: mesh, sprite, or item.
#[derive(Debug, Clone)]
pub enum WorldInstance {
    Mesh {
        mesh_id: String,
        inst_name: String,
        tm: Vec<f64>,
        flags: i32,
        story_id: i32,
    },
    Sprite {
        sprite_name: String,
        pos: [f32; 3],
        yaw: f32,
        anim: i32,
        frame: i32,
        reps: [i32; 4],
        flags: i32,
        story_id: i32,
    },
    Item {
        item_proto_index: i32,
        count: i32,
        pos: [f32; 3],
        yaw: f32,
        flags: i32,
        story_id: i32,
    },
}

/// Parsed world section from an .a3d file.
#[derive(Debug, Clone, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct A3dWorld {
    pub format_version: u32,
    pub instances: Vec<WorldInstance>,
}

// --- Binary read helpers (little-endian, cursor-advancing) ---

fn read_i32(data: &[u8], cursor: &mut usize) -> Result<i32, AssetError> {
    let end = *cursor + 4;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(*cursor));
    }
    let val = i32::from_le_bytes(
        data[*cursor..end]
            .try_into()
            .map_err(|_| AssetError::UnexpectedEof(*cursor))?,
    );
    *cursor = end;
    Ok(val)
}

fn read_f32(data: &[u8], cursor: &mut usize) -> Result<f32, AssetError> {
    let end = *cursor + 4;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(*cursor));
    }
    let val = f32::from_le_bytes(
        data[*cursor..end]
            .try_into()
            .map_err(|_| AssetError::UnexpectedEof(*cursor))?,
    );
    *cursor = end;
    Ok(val)
}

fn read_f64(data: &[u8], cursor: &mut usize) -> Result<f64, AssetError> {
    let end = *cursor + 8;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(*cursor));
    }
    let val = f64::from_le_bytes(
        data[*cursor..end]
            .try_into()
            .map_err(|_| AssetError::UnexpectedEof(*cursor))?,
    );
    *cursor = end;
    Ok(val)
}

fn read_string(data: &[u8], cursor: &mut usize, len: usize) -> Result<String, AssetError> {
    let end = *cursor + len;
    if end > data.len() {
        return Err(AssetError::UnexpectedEof(*cursor));
    }
    let s = String::from_utf8_lossy(&data[*cursor..end]).into_owned();
    *cursor = end;
    Ok(s)
}

fn read_len_prefixed_string(data: &[u8], cursor: &mut usize) -> Result<String, AssetError> {
    let len = read_i32(data, cursor)? as usize;
    read_string(data, cursor, len)
}

fn read_f32x3(data: &[u8], cursor: &mut usize) -> Result<[f32; 3], AssetError> {
    Ok([
        read_f32(data, cursor)?,
        read_f32(data, cursor)?,
        read_f32(data, cursor)?,
    ])
}

fn read_i32x4(data: &[u8], cursor: &mut usize) -> Result<[i32; 4], AssetError> {
    Ok([
        read_i32(data, cursor)?,
        read_i32(data, cursor)?,
        read_i32(data, cursor)?,
        read_i32(data, cursor)?,
    ])
}

fn read_f64x16(data: &[u8], cursor: &mut usize) -> Result<Vec<f64>, AssetError> {
    let mut vals = Vec::with_capacity(16);
    for _ in 0..16 {
        vals.push(read_f64(data, cursor)?);
    }
    Ok(vals)
}

/// Convert `.ply` extension to `.akm` in mesh IDs.
///
/// Old .a3d files store mesh names with `.ply` extension but the
/// actual files on disk use `.akm`. The C++ code does this conversion
/// inline during loading.
fn convert_ply_to_akm(mesh_id: String) -> String {
    if mesh_id.ends_with(".ply") {
        let mut result = mesh_id;
        let new_len = result.len() - 4;
        result.truncate(new_len);
        result.push_str(".akm");
        result
    } else {
        mesh_id
    }
}

/// Parse the world section of an .a3d file.
///
/// The world section starts after the terrain patches and material table.
/// Format: first i32 determines version (negative = versioned, positive = legacy count).
///
/// # Errors
///
/// Returns `AssetError::UnexpectedEof` if the data is truncated.
/// Returns `AssetError::UnknownInstanceType` for unrecognized discriminants.
pub fn parse_world_section(data: &[u8]) -> Result<A3dWorld, AssetError> {
    if data.len() < 4 {
        return Err(AssetError::UnexpectedEof(0));
    }

    let mut cursor = 0;

    let first_int = read_i32(data, &mut cursor)?;

    let (format_version, num_instances) = if first_int < 0 {
        let version = (-first_int) as u32;
        let count = read_i32(data, &mut cursor)?;
        (version, count as usize)
    } else {
        (0u32, first_int as usize)
    };

    let mut instances = Vec::with_capacity(num_instances);

    for _ in 0..num_instances {
        let mesh_id_len = read_i32(data, &mut cursor)?;

        match mesh_id_len {
            len if len >= 0 => {
                let mesh_id = read_string(data, &mut cursor, len as usize)?;
                let inst_name = read_len_prefixed_string(data, &mut cursor)?;
                let tm = read_f64x16(data, &mut cursor)?;
                let flags = read_i32(data, &mut cursor)?;
                let story_id = if format_version > 0 {
                    read_i32(data, &mut cursor)?
                } else {
                    -1
                };

                let mesh_id = convert_ply_to_akm(mesh_id);

                instances.push(WorldInstance::Mesh {
                    mesh_id,
                    inst_name,
                    tm,
                    flags,
                    story_id,
                });
            }
            -1 => {
                let sprite_name = read_len_prefixed_string(data, &mut cursor)?;
                let pos = read_f32x3(data, &mut cursor)?;
                let yaw = read_f32(data, &mut cursor)?;
                let anim = read_i32(data, &mut cursor)?;
                let frame = read_i32(data, &mut cursor)?;
                let reps = read_i32x4(data, &mut cursor)?;
                let flags = read_i32(data, &mut cursor)?;
                let story_id = if format_version > 0 {
                    read_i32(data, &mut cursor)?
                } else {
                    -1
                };

                instances.push(WorldInstance::Sprite {
                    sprite_name,
                    pos,
                    yaw,
                    anim,
                    frame,
                    reps,
                    flags,
                    story_id,
                });
            }
            -2 => {
                let item_proto_index = read_i32(data, &mut cursor)?;
                let count = read_i32(data, &mut cursor)?;
                let pos = read_f32x3(data, &mut cursor)?;
                let yaw = read_f32(data, &mut cursor)?;
                let flags = read_i32(data, &mut cursor)?;
                let story_id = if format_version > 0 {
                    read_i32(data, &mut cursor)?
                } else {
                    -1
                };

                instances.push(WorldInstance::Item {
                    item_proto_index,
                    count,
                    pos,
                    yaw,
                    flags,
                    story_id,
                });
            }
            _ => return Err(AssetError::UnknownInstanceType(mesh_id_len)),
        }
    }

    Ok(A3dWorld {
        format_version,
        instances,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_ply_to_akm() {
        assert_eq!(convert_ply_to_akm("foo.ply".to_string()), "foo.akm");
        assert_eq!(convert_ply_to_akm("bar.akm".to_string()), "bar.akm");
        assert_eq!(convert_ply_to_akm("baz".to_string()), "baz");
    }

    #[test]
    fn test_read_helpers_eof() {
        let empty: &[u8] = &[];
        let mut cursor = 0;
        assert!(read_i32(empty, &mut cursor).is_err());
    }
}
