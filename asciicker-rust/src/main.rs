use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Asciicker - Rust Port".into(),
                    resolution: (800., 600.).into(),
                    ..default()
                }),
                ..default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, print_frame_time)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    info!("Asciicker initialized");
}

fn print_frame_time(time: Res<Time>) {
    if time.delta_seconds() > 0.0 {
        // Print frame time every 60 frames
        static FRAME_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let count = FRAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % 60 == 0 {
            info!("Frame time: {:.2}ms ({:.0} fps)", 
                time.delta_seconds() * 1000.0, 
                1.0 / time.delta_seconds());
        }
    }
}
