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
#[derive(Debug, Clone)]
pub struct AkmMesh {
    pub vertices: Vec<AkmVertex>,
    pub faces: Vec<AkmFace>,
    pub edges: Vec<AkmEdge>,
}

/// Parse an AKM file (ASCII PLY format).
pub fn parse_akm(_text: &str) -> Result<AkmMesh, AssetError> {
    todo!("implement AKM parsing")
}
