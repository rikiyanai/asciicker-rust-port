use bevy::prelude::*;

use super::config::RenderConfig;

/// A single sample in the rasterizer's output buffer.
///
/// Matches the C++ engine's sample layout for performance.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Depth value for Z-buffering.
    pub depth: f32,
    /// Color in RGB555 format (15-bit color).
    pub color_rgb555: u16,
    /// CP437 glyph index.
    pub glyph: u16,
    /// Material identifier for shade table lookup.
    pub material_id: u8,
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            depth: f32::MAX,
            color_rgb555: 0,
            glyph: 0,
            material_id: 0,
        }
    }
}

/// 2x supersampled depth/color buffer for the CPU rasterizer.
///
/// Flat Vec<Sample> layout with index methods matching C++ for performance.
/// Dimensions derived from RenderConfig at initialization.
#[derive(Resource)]
pub struct SampleBuffer {
    /// Width in samples (ascii_width * supersample_factor).
    pub width: u32,
    /// Height in samples (ascii_height * supersample_factor).
    pub height: u32,
    /// Flat sample storage (row-major: index = y * width + x).
    pub samples: Vec<Sample>,
}

impl FromWorld for SampleBuffer {
    fn from_world(world: &mut World) -> Self {
        let config = world.resource::<RenderConfig>();
        let w = config.sample_width();
        let h = config.sample_height();
        Self {
            width: w,
            height: h,
            samples: vec![Sample::default(); (w * h) as usize],
        }
    }
}

impl SampleBuffer {
    /// Get a reference to the sample at (x, y).
    ///
    /// # Panics
    /// Panics if x >= width or y >= height.
    pub fn sample_at(&self, x: u32, y: u32) -> &Sample {
        let idx = (y * self.width + x) as usize;
        &self.samples[idx]
    }

    /// Get a mutable reference to the sample at (x, y).
    ///
    /// # Panics
    /// Panics if x >= width or y >= height.
    pub fn sample_at_mut(&mut self, x: u32, y: u32) -> &mut Sample {
        let idx = (y * self.width + x) as usize;
        &mut self.samples[idx]
    }

    /// Clear all samples to default values (depth = MAX, color = 0, glyph = 0).
    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = Sample::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(width: u32, height: u32) -> SampleBuffer {
        SampleBuffer {
            width,
            height,
            samples: vec![Sample::default(); (width * height) as usize],
        }
    }

    #[test]
    fn default_sample_has_max_depth() {
        let sample = Sample::default();
        assert_eq!(sample.depth, f32::MAX);
        assert_eq!(sample.color_rgb555, 0);
        assert_eq!(sample.glyph, 0);
        assert_eq!(sample.material_id, 0);
    }

    #[test]
    fn buffer_dimensions() {
        let buf = make_buffer(480, 270);
        assert_eq!(buf.width, 480);
        assert_eq!(buf.height, 270);
        assert_eq!(buf.samples.len(), 480 * 270);
    }

    #[test]
    fn sample_at_indexing() {
        let mut buf = make_buffer(480, 270);
        buf.sample_at_mut(10, 20).depth = 42.0;
        assert_eq!(buf.sample_at(10, 20).depth, 42.0);
        // Adjacent samples are unaffected
        assert_eq!(buf.sample_at(11, 20).depth, f32::MAX);
    }

    #[test]
    fn clear_resets_all_samples() {
        let mut buf = make_buffer(4, 4);
        buf.sample_at_mut(1, 1).depth = 5.0;
        buf.sample_at_mut(2, 3).glyph = 65;
        buf.clear();
        assert_eq!(buf.sample_at(1, 1).depth, f32::MAX);
        assert_eq!(buf.sample_at(2, 3).glyph, 0);
    }

    #[test]
    fn corner_access() {
        let buf = make_buffer(480, 270);
        // Should not panic
        let _ = buf.sample_at(0, 0);
        let _ = buf.sample_at(479, 269);
    }
}
