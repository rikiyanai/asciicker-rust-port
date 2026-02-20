pub mod ascii_cell_grid;

use bevy::prelude::*;

use ascii_cell_grid::AsciiCellGrid;

pub struct AsciiOutputPlugin;

impl Plugin for AsciiOutputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AsciiCellGrid>();
        info!("AsciiOutputPlugin registered");
    }
}
