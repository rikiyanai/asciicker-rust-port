//! A3D terrain and material table parser.
//!
//! Parses the terrain section (AS3D magic, FileHeader + FilePatch structs) and
//! the fixed-size material table (131,072 bytes = 256 materials x 4 elevations x
//! 16 diffuse levels x 8-byte MatCell) from .a3d binary files.

#[cfg(not(target_endian = "little"))]
compile_error!("A3D parsing requires little-endian platform");

use super::constants::{
    A3D_HEADER_SIZE, A3D_MAGIC, FILE_PATCH_SIZE, HEIGHT_CELLS_PLUS_ONE, MATERIAL_TABLE_SIZE,
    VISUAL_CELLS,
};
use super::error::AssetError;

// ---------------------------------------------------------------------------
// Wire types (zero-copy via bytemuck)
// ---------------------------------------------------------------------------

/// On-disk A3D file header (16 bytes).
///
/// Layout: file_sign(4) + header_size(4) + num_patches(4) + reserved(4).
#[repr(C, packed)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FileHeader {
    file_sign: u32,
    header_size: u32,
    num_patches: u32,
    reserved: u32,
}

const _ASSERT_FILE_HEADER_SIZE: () = assert!(
    std::mem::size_of::<FileHeader>() == 16,
    "FileHeader must be exactly 16 bytes"
);

/// On-disk terrain patch (188 bytes).
///
/// Layout: x(4) + y(4) + visual(128) + height(50) + diag(2) = 188.
/// `visual`: 8x8 grid of u16 material indices (128 bytes).
/// `height`: 5x5 grid of u16 height values (50 bytes).
/// `diag`:   u16 diagonal flag.
#[repr(C, packed)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FilePatch {
    x: i32,
    y: i32,
    visual: [[u16; VISUAL_CELLS]; VISUAL_CELLS],
    height: [[u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
    diag: u16,
}

const _ASSERT_FILE_PATCH_SIZE: () = assert!(
    std::mem::size_of::<FilePatch>() == FILE_PATCH_SIZE,
    "FilePatch must be exactly 188 bytes"
);

// ---------------------------------------------------------------------------
// Owned Rust types (returned to callers)
// ---------------------------------------------------------------------------

/// A single terrain patch with owned data.
#[derive(Debug, Clone)]
pub struct TerrainPatch {
    pub x: i32,
    pub y: i32,
    pub visual: [[u16; VISUAL_CELLS]; VISUAL_CELLS],
    pub height: [[u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],
    pub diag: u16,
}

/// Collection of all terrain patches from the A3D terrain section.
#[derive(Debug, Clone, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct A3dTerrain {
    pub patches: Vec<TerrainPatch>,
}

/// A single material cell (8 bytes): foreground RGB, glyph, background RGB, flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatCell {
    pub fg: [u8; 3],
    pub gl: u8,
    pub bg: [u8; 3],
    pub flags: u8,
}

const _ASSERT_MAT_CELL_SIZE: () = assert!(
    std::mem::size_of::<MatCell>() == 8,
    "MatCell must be exactly 8 bytes"
);

/// Material lookup table: 256 materials, each with 4 elevations x 16 diffuse levels.
#[derive(Debug, Clone, bevy::asset::Asset, bevy::reflect::TypePath)]
pub struct MaterialTable {
    pub materials: Vec<[[MatCell; 16]; 4]>,
}

// ---------------------------------------------------------------------------
// Parsing functions
// ---------------------------------------------------------------------------

/// Parse the terrain section from the start of `data`.
///
/// Returns `(A3dTerrain, bytes_consumed)` on success. The caller can use
/// `bytes_consumed` to find where the material table begins.
pub fn parse_terrain_section(data: &[u8]) -> Result<(A3dTerrain, usize), AssetError> {
    let header_size = std::mem::size_of::<FileHeader>();

    if data.len() < header_size {
        return Err(AssetError::UnexpectedEof(data.len()));
    }

    let header: &FileHeader = bytemuck::from_bytes(&data[..header_size]);

    // Validate magic number
    if header.file_sign != A3D_MAGIC {
        return Err(AssetError::BadMagic(header.file_sign));
    }

    // Validate header size
    if header.header_size != A3D_HEADER_SIZE {
        return Err(AssetError::BadHeaderSize(header.header_size));
    }

    let num_patches = header.num_patches as usize;
    let patches_size = num_patches * FILE_PATCH_SIZE;
    let total_terrain = header_size + patches_size;

    if data.len() < total_terrain {
        return Err(AssetError::UnexpectedEof(data.len()));
    }

    let mut patches = Vec::with_capacity(num_patches);

    for i in 0..num_patches {
        let start = header_size + i * FILE_PATCH_SIZE;
        let end = start + FILE_PATCH_SIZE;
        let file_patch: &FilePatch = bytemuck::from_bytes(&data[start..end]);

        // Copy data out of packed struct into owned TerrainPatch.
        // Reading from packed fields requires copying to avoid unaligned access UB.
        patches.push(TerrainPatch {
            x: { file_patch.x },
            y: { file_patch.y },
            visual: { file_patch.visual },
            height: { file_patch.height },
            diag: { file_patch.diag },
        });
    }

    Ok((A3dTerrain { patches }, total_terrain))
}

/// Parse the material table from the start of `data`.
///
/// Expects at least `MATERIAL_TABLE_SIZE` (131,072) bytes. Returns
/// `(MaterialTable, bytes_consumed)`.
///
/// Layout: 256 materials x 512 bytes each.
/// Each material: 4 elevations x 16 diffuse levels x 8-byte MatCell.
pub fn parse_material_section(data: &[u8]) -> Result<(MaterialTable, usize), AssetError> {
    if data.len() < MATERIAL_TABLE_SIZE {
        return Err(AssetError::UnexpectedEof(data.len()));
    }

    let mut materials: Vec<[[MatCell; 16]; 4]> = Vec::with_capacity(256);
    let mut offset = 0usize;

    for _mat_idx in 0..256 {
        let mut elevations = [[MatCell::default(); 16]; 4];

        for elevation in &mut elevations {
            for cell in elevation.iter_mut() {
                let slice = &data[offset..offset + 8];
                *cell = MatCell {
                    fg: [slice[0], slice[1], slice[2]],
                    gl: slice[3],
                    bg: [slice[4], slice[5], slice[6]],
                    flags: slice[7],
                };
                offset += 8;
            }
        }

        materials.push(elevations);
    }

    debug_assert_eq!(offset, MATERIAL_TABLE_SIZE);

    Ok((MaterialTable { materials }, MATERIAL_TABLE_SIZE))
}
