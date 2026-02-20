use bevy::prelude::*;

/// Configuration for the rendering pipeline dimensions.
///
/// Controls the ASCII output resolution and supersampling factor.
/// SampleBuffer dimensions are derived from this config.
#[derive(Resource, Debug, Clone)]
pub struct RenderConfig {
    /// Width of the ASCII output grid in cells.
    pub ascii_width: u32,
    /// Height of the ASCII output grid in cells.
    pub ascii_height: u32,
    /// Supersampling factor (samples per ASCII cell per axis).
    pub supersample_factor: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            ascii_width: 240,
            ascii_height: 135,
            supersample_factor: 2,
        }
    }
}

impl RenderConfig {
    /// Width of the sample buffer (ascii_width * supersample_factor).
    pub fn sample_width(&self) -> u32 {
        self.ascii_width * self.supersample_factor
    }

    /// Height of the sample buffer (ascii_height * supersample_factor).
    pub fn sample_height(&self) -> u32 {
        self.ascii_height * self.supersample_factor
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
        assert_eq!(config.supersample_factor, 2);
    }

    #[test]
    fn sample_dimensions_are_supersampled() {
        let config = RenderConfig::default();
        assert_eq!(config.sample_width(), 480);
        assert_eq!(config.sample_height(), 270);
    }

    #[test]
    fn custom_config() {
        let config = RenderConfig {
            ascii_width: 120,
            ascii_height: 67,
            supersample_factor: 4,
        };
        assert_eq!(config.sample_width(), 480);
        assert_eq!(config.sample_height(), 268);
    }
}
