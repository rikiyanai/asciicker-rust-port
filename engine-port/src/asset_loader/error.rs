use thiserror::Error;

/// Shared error type for all asset loaders.
///
/// Covers XP sprite, A3D terrain, A3D world, and AKM mesh parsing errors.
#[derive(Debug, Error)]
pub enum AssetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("too few layers: {0} (minimum 3 required)")]
    TooFewLayers(usize),

    #[error("invalid dimensions: {0}x{1}")]
    InvalidDimensions(usize, usize),

    #[error("bad magic number: 0x{0:08X}")]
    BadMagic(u32),

    #[error("bad header size: {0}")]
    BadHeaderSize(u32),

    #[error("unknown instance type: {0}")]
    UnknownInstanceType(i32),

    #[error("not a PLY file")]
    NotPly,

    #[error("unsupported PLY format (only ASCII 1.0 supported)")]
    UnsupportedPlyFormat,

    #[error("empty file")]
    EmptyFile,

    #[error("unexpected end of file at offset {0}")]
    UnexpectedEof(usize),

    #[error("parse error: {0}")]
    Parse(String),
}
