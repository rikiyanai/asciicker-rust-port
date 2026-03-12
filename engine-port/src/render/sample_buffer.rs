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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_is_8_bytes() {
        assert_eq!(std::mem::size_of::<Sample>(), 8);
    }

    #[test]
    fn clear_state_has_correct_fields() {
        let s = Sample::clear_state();
        // Sky-blue: r5=0x0C, g5=0x0C, b5=0x1B => packed = 0x6D8C
        assert_eq!(s.visual, 0x0C | (0x0C << 5) | (0x1B << 10));
        assert_eq!(s.diffuse, 0xFF);
        assert_eq!(s.spare, spare_bits::MESH_FLAG);
        assert_eq!(s.height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn depth_test_ro_passes_when_behind() {
        let s = Sample {
            height: 10.0,
            ..Sample::clear_state()
        };
        // z = 10.0: height(10) <= 10 + 8 = true
        assert!(s.depth_test_ro(10.0));
        // z = 20.0: height(10) <= 20 + 8 = true
        assert!(s.depth_test_ro(20.0));
    }

    #[test]
    fn depth_test_ro_fails_when_in_front() {
        let s = Sample {
            height: 30.0,
            ..Sample::clear_state()
        };
        // z = 10.0: height(30) <= 10 + 8 = 18 => false
        assert!(!s.depth_test_ro(10.0));
    }

    #[test]
    fn depth_test_ro_boundary() {
        let s = Sample {
            height: 18.0,
            ..Sample::clear_state()
        };
        // z = 10.0: height(18) <= 10 + 8 = 18 => true (equal)
        assert!(s.depth_test_ro(10.0));

        let s2 = Sample {
            height: 18.01,
            ..Sample::clear_state()
        };
        // z = 10.0: height(18.01) <= 18 => false
        assert!(!s2.depth_test_ro(10.0));
    }

    #[test]
    fn is_mesh_flag() {
        let with_mesh = Sample::clear_state(); // clear_state sets MESH_FLAG
        assert!(with_mesh.is_mesh());

        let without_mesh = Sample {
            spare: 0,
            ..Sample::clear_state()
        };
        assert!(!without_mesh.is_mesh());
    }

    #[test]
    fn buffer_default_dimensions() {
        let buf = SampleBuffer::new(240, 135);
        assert_eq!(buf.width, 484);
        assert_eq!(buf.height, 274);
        assert_eq!(buf.samples.len(), 484 * 274);
    }

    #[test]
    fn buffer_clear_restores_all_samples() {
        let mut buf = SampleBuffer::new(240, 135);
        // Mutate a sample
        buf.sample_at_mut(10, 20).visual = 0xBEEF;
        buf.sample_at_mut(10, 20).height = 999.0;
        assert_eq!(buf.sample_at(10, 20).visual, 0xBEEF);

        // Clear should restore
        buf.clear();
        let s = buf.sample_at(10, 20);
        assert_eq!(s.visual, Sample::clear_state().visual);
        assert_eq!(s.height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn buffer_clear_uses_copy_from_slice_semantics() {
        // Verify the double-allocation pattern works:
        // mutate -> clear -> verify restored
        let mut buf = SampleBuffer::new(4, 4);
        let original_visual = buf.sample_at(0, 0).visual;

        buf.sample_at_mut(0, 0).visual = 0x1234;
        buf.sample_at_mut(3, 3).height = 42.0;
        assert_ne!(buf.sample_at(0, 0).visual, original_visual);

        buf.clear();
        assert_eq!(buf.sample_at(0, 0).visual, original_visual);
        assert_eq!(buf.sample_at(3, 3).height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn corner_access_does_not_panic() {
        let buf = SampleBuffer::new(240, 135);
        let _ = buf.sample_at(0, 0);
        let _ = buf.sample_at(483, 273);
    }

    #[test]
    fn sample_at_indexing() {
        let mut buf = SampleBuffer::new(240, 135);
        buf.sample_at_mut(10, 20).height = 42.0;
        assert_eq!(buf.sample_at(10, 20).height, 42.0);
        // Adjacent samples are unaffected (still clear_state)
        assert_eq!(buf.sample_at(11, 20).height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn flat_index_matches_row_major() {
        let buf = SampleBuffer::new(240, 135);
        assert_eq!(buf.flat_index(0, 0), 0);
        assert_eq!(buf.flat_index(1, 0), 1);
        assert_eq!(buf.flat_index(0, 1), 484);
        assert_eq!(buf.flat_index(483, 273), (273 * 484 + 483) as usize);
    }

    // --- GAP-10 (R43): Boundary tests ---

    #[test]
    fn test_sample_buffer_zero_size() {
        // SampleBuffer::new(0, 0) produces dimensions (4, 4) due to
        // the 2*ascii+4 formula. Verify it doesn't panic.
        let buf = SampleBuffer::new(0, 0);
        assert_eq!(buf.width, 4);
        assert_eq!(buf.height, 4);
        assert_eq!(buf.samples.len(), 16);
        // Can access the last valid index
        let _ = buf.sample_at(3, 3);
    }

    #[test]
    fn test_sample_buffer_border_pixels() {
        // Border is +2 on each side of the 2x-supersampled area.
        // For ascii 4x4: w=2*4+4=12, h=2*4+4=12
        let buf = SampleBuffer::new(4, 4);
        assert_eq!(buf.width, 12);
        assert_eq!(buf.height, 12);

        // Border pixel (0,0) should be accessible and cleared
        let s = buf.sample_at(0, 0);
        assert_eq!(s.height, Sample::CLEAR_HEIGHT);

        // Border pixel (1,0), (0,1) should also be clear
        assert_eq!(buf.sample_at(1, 0).height, Sample::CLEAR_HEIGHT);
        assert_eq!(buf.sample_at(0, 1).height, Sample::CLEAR_HEIGHT);

        // Last border pixel (11,11)
        assert_eq!(buf.sample_at(11, 11).height, Sample::CLEAR_HEIGHT);
    }

    #[test]
    fn test_sample_buffer_last_valid_index() {
        // For ascii 4x4: w=12, h=12. Last valid index is (11, 11).
        let mut buf = SampleBuffer::new(4, 4);
        let last_x = buf.width - 1; // 11
        let last_y = buf.height - 1; // 11

        // Write to last valid sample
        buf.sample_at_mut(last_x, last_y).visual = 0xABCD;
        buf.sample_at_mut(last_x, last_y).height = 42.0;

        // Read back
        let s = buf.sample_at(last_x, last_y);
        assert_eq!(s.visual, 0xABCD);
        assert_eq!(s.height, 42.0);
    }
}
