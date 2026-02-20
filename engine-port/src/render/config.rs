use bevy::prelude::*;

/// Configuration for the rendering pipeline dimensions.
///
/// Controls the ASCII output resolution. SampleBuffer dimensions are derived
/// as `2 * ascii + 4` to provide 2x supersampling plus a 2-pixel border on each side.
#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    /// Width of the ASCII output grid in cells.
    pub ascii_width: u32,
    /// Height of the ASCII output grid in cells.
    pub ascii_height: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            ascii_width: 240,
            ascii_height: 135,
        }
    }
}

impl RenderConfig {
    /// Width of the sample buffer: `2 * ascii_width + 4`.
    ///
    /// The factor of 2 provides 2x supersampling; the +4 adds a 2-sample
    /// border on each side so that filter kernels never read out of bounds.
    pub fn sample_width(&self) -> u32 {
        2 * self.ascii_width + 4
    }

    /// Height of the sample buffer: `2 * ascii_height + 4`.
    pub fn sample_height(&self) -> u32 {
        2 * self.ascii_height + 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ascii_dimensions() {
        let config = RenderConfig::default();
        assert_eq!(config.ascii_width, 240);
        assert_eq!(config.ascii_height, 135);
    }

    #[test]
    fn default_sample_dimensions_include_border() {
        let config = RenderConfig::default();
        assert_eq!(config.sample_width(), 484);
        assert_eq!(config.sample_height(), 274);
    }

    #[test]
    fn custom_config() {
        let config = RenderConfig {
            ascii_width: 120,
            ascii_height: 67,
        };
        assert_eq!(config.sample_width(), 244);
        assert_eq!(config.sample_height(), 138);
    }
}
