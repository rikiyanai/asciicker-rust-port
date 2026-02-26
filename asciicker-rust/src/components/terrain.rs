use bevy::prelude::*;

/// Terrain patch - 8x8 visual cells, 9x9 vertices
#[derive(Component, Clone, Debug)]
pub struct TerrainPatch {
    pub x: i32,
    pub y: i32,
    pub visual: [[u8; 8]; 8],  // Material/elevation
    pub height: [[f32; 5]; 5], // 5x5 vertex heights
    pub diag: u16,               // Diagonal orientation bits
}

/// Terrain quadtree node
#[derive(Clone, Debug)]
pub struct QuadNode {
    pub x: i32,
    pub y: i32,
    pub level: u32,
    pub patch: Option<TerrainPatch>,
    pub children: [Option<Box<QuadNode>>; 4],
}

impl Default for TerrainPatch {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            visual: [[0; 8]; 8],
            height: [[0.0; 5]; 5],
            diag: 0,
        }
    }
}
