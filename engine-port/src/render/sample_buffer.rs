use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use super::config::RenderConfig;

/// Spare-byte bit constants matching the C++ engine's sample flags.
pub mod spare_bits {
    /// Parity mask (2 LSBs) -- used for grid-phase alternation.
    pub const PARITY_MASK: u8 = 0x03;
    /// Grid overlay flag.
    pub const GRID: u8 = 0x04;
    /// Set when this sample came from a mesh (vs terrain/sky).
    pub const MESH_FLAG: u8 = 0x08;
    /// Wireframe rendering flag.
    pub const WIREFRAME: u8 = 0x40;
    /// Reflection bits (same mask as parity; context-dependent).
    pub const REFLECTION: u8 = 0x03;
}

/// A single sample in the rasterizer's output buffer.
///
/// Matches the C++ engine layout: `visual(u16) | diffuse(u8) | spare(u8) | height(f32)`.
/// Total size is 8 bytes with `#[repr(C)]` for stable, Pod-safe layout.
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Sample {
    /// Material index OR packed RGB555 color value.
    pub visual: u16,
    /// Lighting intensity 0-255 (0xFF = fully lit).
    pub diffuse: u8,
    /// Bit flags -- see [`spare_bits`] module.
    pub spare: u8,
    /// Depth value for Z-buffering. `-1_000_000.0` = cleared (sky).
    pub height: f32,
}

impl Sample {
    /// Depth value used to represent a cleared (empty/sky) sample.
    pub const CLEAR_HEIGHT: f32 = -1_000_000.0;

    /// Half of the C++ HEIGHT_SCALE constant, derived from the canonical constant.
    /// Used as the depth-test epsilon.
    const HALF_HEIGHT_SCALE: f32 = (crate::asset_loader::constants::HEIGHT_SCALE as f32) / 2.0;

    /// Returns the canonical clear state for a sample.
    ///
    /// Sky-blue RGB555 color, full diffuse, MESH_FLAG set, height = CLEAR_HEIGHT.
    /// The RGB555 value is `(0x0C | (0x0C << 5) | (0x1B << 10))` = `0x6D8C`.
    pub fn clear_state() -> Self {
        Self {
            visual: 0x0C | (0x0C << 5) | (0x1B << 10),
            diffuse: 0xFF,
            spare: spare_bits::MESH_FLAG,
            height: Self::CLEAR_HEIGHT,
        }
    }

    /// Read-only depth test: returns `true` if this sample is behind or at `z`
    /// (within half-height-scale tolerance), meaning a new fragment at depth `z`
    /// should be written here.
    #[inline]
    pub fn depth_test_ro(&self, z: f32) -> bool {
        self.height <= z + Self::HALF_HEIGHT_SCALE
    }

    /// Returns `true` if this sample came from a mesh (MESH_FLAG set).
    #[inline]
    pub fn is_mesh(&self) -> bool {
        self.spare & spare_bits::MESH_FLAG != 0
    }
}

/// 2x supersampled depth/color buffer for the CPU rasterizer.
///
/// Uses a double-allocation pattern: `clear_state` holds a cached template
/// of cleared samples, and `clear()` copies it into `samples` via
/// `copy_from_slice` (compiles to memcpy because `Sample` is `Copy + Pod`).
///
/// Dimensions are `(2 * ascii_width + 4) x (2 * ascii_height + 4)`.
#[derive(Resource)]
pub struct SampleBuffer {
    /// Width in samples.
    pub width: u32,
    /// Height in samples.
    pub height: u32,
    /// Working sample storage (row-major: index = y * width + x).
    pub samples: Vec<Sample>,
    /// Cached cleared template -- same size as `samples`.
    clear_state: Vec<Sample>,
}

impl FromWorld for SampleBuffer {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<RenderConfig>();
        Self::new(config.ascii_width, config.ascii_height)
    }
}

impl SampleBuffer {
    /// Create a new SampleBuffer with dimensions `(2*ascii_width+4) x (2*ascii_height+4)`.
    pub fn new(ascii_width: u32, ascii_height: u32) -> Self {
        let w = 2 * ascii_width + 4;
        let h = 2 * ascii_height + 4;
        let size = (w * h) as usize;
        let clear_sample = Sample::clear_state();
        let clear_state = vec![clear_sample; size];
        let samples = clear_state.clone();
        Self {
            width: w,
            height: h,
            samples,
            clear_state,
        }
    }

    /// Clear all samples by copying the cached clear template.
    ///
    /// Because `Sample` is `Copy + Pod`, this compiles to a single `memcpy`.
    #[inline]
    pub fn clear(&mut self) {
        self.samples.copy_from_slice(&self.clear_state);
    }

    /// Compute the flat index for coordinates `(x, y)`.
    #[inline]
    pub fn flat_index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Get a reference to the sample at `(x, y)`.
    ///
    /// # Panics
    /// Panics if `x >= width` or `y >= height`.
    #[inline]
    pub fn sample_at(&self, x: u32, y: u32) -> &Sample {
        let idx = self.flat_index(x, y);
        &self.samples[idx]
    }

    /// Get a mutable reference to the sample at `(x, y)`.
    ///
    /// # Panics
    /// Panics if `x >= width` or `y >= height`.
    #[inline]
    pub fn sample_at_mut(&mut self, x: u32, y: u32) -> &mut Sample {
        let idx = self.flat_index(x, y);
        &mut self.samples[idx]
    }
}
