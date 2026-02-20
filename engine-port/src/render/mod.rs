use bevy::prelude::*;

pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, _app: &mut App) {
        info!("CpuRasterizerPlugin registered");
    }
}
