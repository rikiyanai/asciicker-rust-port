use bevy::prelude::*;

/// Camera component - maps to render.cpp camera
#[derive(Component, Clone, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,       // Rotation in radians
    pub zoom: f32,      // Default 1.0, range 0.2-5.0
    pub focal: f32,     // Calculated from screen size
    pub perspective: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 15.0, 0.0),
            yaw: std::f32::consts::FRAC_PI_4, // 45 degrees
            zoom: 1.0,
            focal: 224.0, // max(800, 600) * 2.0
            perspective: true,
        }
    }
}

/// Scene shift for inventory sliding
#[derive(Component, Default)]
pub struct SceneShift(pub i32);

/// Camera shift for I/X keys
#[derive(Component, Default)]
pub struct CamShift(pub i32);
