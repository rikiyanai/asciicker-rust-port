pub mod config;
pub mod material;
pub mod quantize;
pub mod rasterizer;
pub mod resolve;
pub mod sample_buffer;
pub mod types;

use bevy::prelude::*;

use config::RenderConfig;
use sample_buffer::SampleBuffer;

pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderConfig>()
            .init_resource::<SampleBuffer>();
        info!("CpuRasterizerPlugin registered");
    }
}
