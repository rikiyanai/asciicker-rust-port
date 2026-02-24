//! ECS integration tests for character module.
//!
//! These tests verify character systems work correctly when wired through Bevy's
//! scheduler, catching silent scheduling failures that unit tests miss.

use bevy::prelude::*;

use asciicker_engine::character::equipment::Shield;
use asciicker_engine::character::{ActionState, AnimationState, Character, SpriteReq};
use asciicker_engine::physics::PhysicsIO;
use asciicker_engine::render::camera::GameCamera;
use asciicker_engine::render::sprite_blit::SpriteQueue;

/// Helper to create a minimal app with character-related resources
/// (without full plugin registration which requires terrain/world/assets).
fn make_character_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<GameCamera>();
    app.insert_resource(PhysicsIO::default());
    app.init_resource::<SpriteQueue>();

    // Register character systems manually (cannot use CharacterPlugin because
    // it calls spawn_player which requires RuntimeTerrain)
    app.add_systems(
        PreUpdate,
        (asciicker_engine::character::input::accumulate_player_input,),
    );
    app.add_systems(
        PostUpdate,
        asciicker_engine::character::sprite_query::query_character_sprites,
    );
    app
}

#[test]
fn test_spawn_player_produces_sprite_entry() {
    let mut app = make_character_test_app();

    // Initialize camera at a position where characters are in front
    {
        let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
        cam.pos = [0.0, 0.0, 0.0];
        cam.yaw = 0.0;
        cam.update(484.0, 274.0);
    }

    // Spawn a character entity (simulating what spawn_player does)
    app.world_mut().spawn((
        Character,
        SpriteReq::default(),
        AnimationState::default(),
        ActionState::None,
        Transform::from_xyz(5.0, 5.0, 0.0),
    ));

    app.update();

    let queue = app.world().get_resource::<SpriteQueue>().unwrap();
    assert!(
        queue.len() >= 1,
        "SpriteQueue should have at least 1 entry after character spawned, got {}",
        queue.len()
    );
}

#[test]
fn test_decoy_entity_excluded_from_character_queries() {
    let mut app = make_character_test_app();

    {
        let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
        cam.pos = [0.0, 0.0, 0.0];
        cam.yaw = 0.0;
        cam.update(484.0, 274.0);
    }

    // Spawn real character
    app.world_mut().spawn((
        Character,
        SpriteReq::default(),
        AnimationState::default(),
        ActionState::None,
        Transform::from_xyz(5.0, 5.0, 0.0),
    ));

    // Spawn decoy with Transform only (no Character marker)
    app.world_mut().spawn(Transform::from_xyz(10.0, 10.0, 0.0));

    app.update();

    let queue = app.world().get_resource::<SpriteQueue>().unwrap();
    assert_eq!(
        queue.len(),
        1,
        "Only character entity should produce sprite entry, decoy should be excluded. Got {}",
        queue.len()
    );
}

#[test]
fn test_input_system_modifies_physics_io() {
    let mut app = make_character_test_app();

    // Spawn character for block input query
    app.world_mut().spawn((
        Character,
        SpriteReq {
            shield: Shield::RegularShield,
            ..Default::default()
        },
        AnimationState::default(),
        ActionState::None,
        Transform::default(),
    ));

    // Press W to set y_force
    {
        let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
        cam.yaw = 0.0;
    }
    {
        let mut input = app
            .world_mut()
            .get_resource_mut::<ButtonInput<KeyCode>>()
            .unwrap();
        input.press(KeyCode::KeyW);
    }

    app.update();

    let io = app.world().get_resource::<PhysicsIO>().unwrap();
    assert!(
        io.y_force.abs() > 0.01,
        "PhysicsIO.y_force should be non-zero after W press, got {}",
        io.y_force
    );
}
