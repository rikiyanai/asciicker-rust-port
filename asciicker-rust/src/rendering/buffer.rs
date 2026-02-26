// Rendering buffer - ASCII sample buffer
use bevy::prelude::*;

/// Sample buffer - 2x supersampled rendering buffer
#[derive(Clone, Debug)]
pub struct SampleBuffer {
    pub width: i32,
    pub height: i32,
    pub samples: Vec<Sample>,
}

impl SampleBuffer {
    pub fn new(width: i32, height: i32) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            samples: vec![Sample::default(); size as usize],
        }
    }
}

/// Single sample in buffer - matches C++ Sample struct
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub height: f32,    // Depth (negative = closer)
    pub visual: u16,   // RGB555 color
    pub diffuse: u8,  // Lighting
    pub spare: u8,     // Flags (bit 2=grid, bit3=mesh)
}

/// RGB555 packing/unpacking
impl Sample {
    pub fn pack_rgb555(r: u8, g: u8, b: u8) -> u16 {
        ((r as u16 & 0x1F) << 10) | ((g as u16 & 0x1F) << 5) | (b as u16 & 0x1F)
    }
    
    pub fn unpack_rgb555(value: u16) -> (u8, u8, u8) {
        let r = ((value >> 10) & 0x1F) as u8;
        let g = ((value >> 5) & 0x1F) as u8;
        let b = (value & 0x1F) as u8;
        (r, g, b)
    }
}
