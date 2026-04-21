use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderDebugCell {
    pub flags: u32,
    pub sample_spares: [u8; 4],
    pub sample_heights: [f32; 4],
    pub dominant_visual: u16,
    pub material_lane: u8,
    pub diffuse_index: u8,
    pub shape_distance: f32,
    pub resolve_glyph: u16,
    pub final_glyph: u16,
}

pub mod debug_flags {
    pub const CLEAR: u32 = 1 << 0;
    pub const MESH_PATH: u32 = 1 << 1;
    pub const MATERIAL_PATH: u32 = 1 << 2;
    pub const MIXED_MESH_TERRAIN: u32 = 1 << 3;
    pub const HAS_REFLECTION: u32 = 1 << 4;
    pub const HAS_NORMAL_TERRAIN: u32 = 1 << 5;
    pub const ALL_UNDERWATER: u32 = 1 << 6;
    pub const USED_AUTO_MAT: u32 = 1 << 7;
    pub const APPLIED_RIPPLE: u32 = 1 << 8;
    pub const APPLIED_GRID_OVERLAY: u32 = 1 << 9;
    pub const APPLIED_LINECASE_OVERLAY: u32 = 1 << 10;
    pub const SHAPE_VECTOR_OVERRIDE: u32 = 1 << 11;
    pub const APPLIED_SILHOUETTE_OVERLAY: u32 = 1 << 12;
    pub const SHAPE_SKIP_CLEAR: u32 = 1 << 13;
    pub const SHAPE_SKIP_UNDERWATER: u32 = 1 << 14;
    pub const SHAPE_SKIP_THRESHOLD: u32 = 1 << 15;
    pub const SHAPE_FALLBACK_SPACE: u32 = 1 << 16;
    pub const SHAPE_FALLBACK_STRUCTURAL: u32 = 1 << 17;
    pub const SHAPE_COLORED_SPACE: u32 = 1 << 18;
    pub const SHAPE_PRESERVED_RESOLVE: u32 = 1 << 19;
    pub const SHAPE_GATED_SEMANTIC: u32 = 1 << 20;
}

#[derive(Resource, Debug, Clone, Default)]
pub struct RenderDebugGrid {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<RenderDebugCell>,
}

impl RenderDebugGrid {
    pub fn resize(&mut self, width: u32, height: u32) {
        let len = (width * height) as usize;
        self.width = width;
        self.height = height;
        self.cells.resize(len, RenderDebugCell::default());
        self.clear();
    }

    pub fn clear(&mut self) {
        self.cells.fill(RenderDebugCell::default());
    }

    pub fn ensure_size(&mut self, width: u32, height: u32) {
        if self.width != width
            || self.height != height
            || self.cells.len() != (width * height) as usize
        {
            self.resize(width, height);
        } else {
            self.clear();
        }
    }
}
