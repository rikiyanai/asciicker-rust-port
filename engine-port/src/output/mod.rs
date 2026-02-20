use bevy::prelude::*;

pub struct AsciiOutputPlugin;

impl Plugin for AsciiOutputPlugin {
    fn build(&self, _app: &mut App) {
        info!("AsciiOutputPlugin registered");
    }
}
