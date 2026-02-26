// Terrain quadtree - maps to terrain.cpp quadtree
use crate::components::TerrainPatch;

#[derive(Clone, Debug)]
pub struct Quadtree {
    pub root: Option<Box<QuadNode>>,
    pub min_level: i32,
}

#[derive(Clone, Debug)]
pub struct QuadNode {
    pub x: i32,
    pub y: i32,
    pub level: i32,
    pub patch: Option<TerrainPatch>,
    pub children: [Option<Box<QuadNode>>; 4],
}

impl Quadtree {
    pub fn new() -> Self {
        Self { root: None, min_level: 0 }
    }
    
    pub fn get_height(&self, x: f32, y: f32) -> f32 {
        // TODO: Implement height query
        0.0
    }
}

impl Default for Quadtree {
    fn default() -> Self { Self::new() }
}
