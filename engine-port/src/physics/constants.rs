//! Physics constants matching C++ physics.cpp values.

/// Physics simulation frequency in Hz.
pub const PHYSICS_HZ: f64 = 66.667;

/// Maximum substeps per FixedUpdate tick to prevent infinite loops.
pub const MAX_SUBSTEPS: u32 = 10;

/// Maximum velocity magnitude in air (world units/s).
pub const MAX_VEL_AIR: f32 = 27.0;

/// Maximum velocity magnitude in water (world units/s).
pub const MAX_VEL_WATER: f32 = 10.0;

/// Jump velocity impulse (vertical, world units/s).
pub const JUMP_VELOCITY: f32 = 10.0;

/// Velocity damping factor per second (applied as powf(dt)).
pub const VEL_DAMPING: f32 = 0.9;

/// Impulse drain rate per tick.
pub const IMPULSE_DRAIN: f32 = 0.5;

/// Grounded detection threshold: accum_contact must be >= this to be grounded.
pub const GROUNDED_THRESHOLD: f32 = 1.0;

/// Maximum accumulated contact normal value.
pub const GROUNDED_MAX_ACCUM: f32 = 5.0;

/// Grounded accumulator decay rate per tick.
pub const GROUNDED_DECAY: f32 = 0.9;

/// XY movement speed scaling factor.
pub const XY_SPEED: f32 = 0.13;

/// XY velocity threshold below which movement is considered idle.
pub const XY_THRESH: f32 = 0.002;

/// Z velocity threshold below which vertical movement is considered idle.
pub const Z_THRESH: f32 = 0.001;

/// Safe distance maintained from collision surfaces to prevent penetration.
pub const SAFE_DISTANCE: f32 = 0.01;

/// Step mask for player_stp animation counter (8*1024 - 1).
pub const STEP_MASK: i32 = 8 * 1024 - 1;
