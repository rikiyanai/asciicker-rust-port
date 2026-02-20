use bevy::math::Vec3;

/// Asciicker uses a Z-up coordinate system.
/// Bevy uses Y-up internally.
/// All game logic operates in Z-up space; conversion happens at the Bevy rendering boundary.
pub const UP: Vec3 = Vec3::Z;

/// Forward direction in Z-up game space (+Y axis).
pub const FORWARD: Vec3 = Vec3::Y;

/// Right direction in Z-up game space (+X axis).
pub const RIGHT: Vec3 = Vec3::X;

/// Type alias marking a Vec3 as being in game space (Z-up coordinate system).
/// Provides documentation intent; same underlying type as Vec3.
pub type GameVec3 = Vec3;

/// Convert from game space (Z-up) to Bevy render space (Y-up).
///
/// Game: +X right, +Y forward, +Z up
/// Bevy: +X right, +Y up, -Z forward
#[inline]
pub fn game_to_bevy(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.z, -v.y)
}

/// Convert from Bevy render space (Y-up) to game space (Z-up).
///
/// Bevy: +X right, +Y up, -Z forward
/// Game: +X right, +Y forward, +Z up
#[inline]
pub fn bevy_to_game(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_is_z() {
        assert_eq!(UP, Vec3::Z);
    }

    #[test]
    fn forward_is_y() {
        assert_eq!(FORWARD, Vec3::Y);
    }

    #[test]
    fn right_is_x() {
        assert_eq!(RIGHT, Vec3::X);
    }

    #[test]
    fn game_up_maps_to_bevy_up() {
        let result = game_to_bevy(UP);
        assert_eq!(result, Vec3::Y);
    }

    #[test]
    fn game_forward_maps_to_bevy_negative_z() {
        let result = game_to_bevy(FORWARD);
        assert_eq!(result, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn roundtrip_identity() {
        let vectors = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-5.0, 0.0, 10.0),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(0.5, -0.3, 7.7),
        ];
        for v in vectors {
            let roundtrip = bevy_to_game(game_to_bevy(v));
            assert!(
                (roundtrip - v).length() < f32::EPSILON,
                "Roundtrip failed for {v}: got {roundtrip}"
            );
        }
    }

    #[test]
    fn inverse_roundtrip_identity() {
        let vectors = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-5.0, 0.0, 10.0),
            Vec3::ZERO,
        ];
        for v in vectors {
            let roundtrip = game_to_bevy(bevy_to_game(v));
            assert!(
                (roundtrip - v).length() < f32::EPSILON,
                "Inverse roundtrip failed for {v}: got {roundtrip}"
            );
        }
    }
}
