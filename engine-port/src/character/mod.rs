//! Character module: state machine, equipment, animation, input, sprite query.
//!
//! Port of C++ game.cpp/game.h character subsystem. Provides CharacterPlugin
//! that registers all character systems and components.
//!
//! CRITICAL: CharacterPlugin does NOT add PhysicsPlugin as a sub-plugin.
//! main.rs registers all plugins independently to avoid duplicate registration panic.

pub mod animation;
pub mod equipment;
pub mod input;
pub mod sprite_query;
pub mod state_machine;

use bevy::prelude::*;

pub use animation::AnimationState;
pub use equipment::SpriteReq;
pub use state_machine::{ActionState, Character};

use crate::physics::PhysicsIO;
use crate::render::sprite_blit::SpriteQueue;
use crate::system_sets::CharacterSet;
use crate::terrain::RuntimeTerrain;

use input::accumulate_player_input;
use sprite_query::query_character_sprites;

/// Clear the sprite queue each frame before new entries are pushed.
///
/// OWNERSHIP: Registered by CharacterPlugin only. CpuRasterizerPlugin and
/// GamePlugin do NOT register it. The render pipeline Stage 3 WORLD only
/// APPENDS world sprites, never clears.
fn clear_sprite_queue_system(mut sprite_queue: ResMut<SpriteQueue>) {
    sprite_queue.clear();
}

/// Update character state from physics output.
///
/// Handles animation-complete auto-transitions (Attack->None, Stand->None),
/// fall detection (not grounded + vel_z < threshold), and landing
/// recovery (Fall + grounded -> Stand).
fn update_character_state(
    physics_io: Res<PhysicsIO>,
    mut q: Query<(&mut ActionState, &AnimationState, &SpriteReq), With<Character>>,
) {
    use crate::physics::constants::Z_THRESH;

    for (mut action_state, anim_state, _sprite_req) in &mut q {
        // Auto-transition on animation complete (Attack->None, Stand->None)
        action_state.check_animation_complete(anim_state);

        // Fall detection: not grounded AND moving downward
        if !physics_io.grounded && physics_io.vel_z < -Z_THRESH {
            action_state.try_transition(ActionState::Fall);
        }
        // Jump arc (vel_z > 0, not grounded) does NOT enter Fall

        // R19-M09 FIX: Fall -> Stand transition when character lands
        if *action_state == ActionState::Fall && physics_io.grounded {
            action_state.try_transition(ActionState::Stand);
        }
    }
}

/// Advance animation frames based on current action and elapsed time.
fn advance_animation_system(
    time: Res<Time>,
    physics_io: Res<PhysicsIO>,
    mut q: Query<(&ActionState, &mut AnimationState), With<Character>>,
) {
    // CRITICAL: Uses Time::delta().as_micros() (NOT delta_secs)
    let delta_us = time.delta().as_micros() as u64;
    for (action, mut anim) in &mut q {
        anim.advance(*action, physics_io.player_stp, delta_us);
    }
}

/// The ONLY function that creates character entities via Commands.
/// All character spawning (player, NPC) MUST use this.
pub fn spawn_character(commands: &mut Commands, position: Vec3, equipment: SpriteReq) -> Entity {
    commands
        .spawn((
            Character,
            equipment,
            Transform::from_xyz(position.x, position.y, position.z),
        ))
        .id()
}

/// Startup system: spawn the player character.
fn spawn_player(
    mut commands: Commands,
    terrain: Option<Res<RuntimeTerrain>>,
    mut physics_io: ResMut<PhysicsIO>,
) {
    let spawn_x = 0.0;
    let spawn_y = 0.0;
    let spawn_z = terrain
        .as_ref()
        .and_then(|t| t.interpolate_height(spawn_x as f64, spawn_y as f64))
        .map(|h| h as f32)
        .unwrap_or(0.0)
        + 50.0; // 50 units above terrain (terrain likely None at Startup)

    // MUST set PhysicsIO.pos to spawn coords (physics starts at [0,0,0] otherwise)
    physics_io.pos = [spawn_x, spawn_y, spawn_z];

    let _entity = spawn_character(
        &mut commands,
        Vec3::new(spawn_x, spawn_y, spawn_z),
        SpriteReq::default(),
    );
    // Entity ID used in Phase 7 for player-specific tagging (Replication component)
}

/// Character plugin. Registers all character systems and components.
///
/// CRITICAL: Does NOT add PhysicsPlugin or GamePlugin as sub-plugins.
pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        // Startup: spawn player
        app.add_systems(Startup, spawn_player);

        // PreUpdate (input consumed by FixedUpdate in same frame)
        app.add_systems(
            PreUpdate,
            (clear_sprite_queue_system, accumulate_player_input)
                .chain()
                .in_set(CharacterSet::PreUpdateInput),
        );

        // PostUpdate (after physics, before render)
        app.add_systems(
            PostUpdate,
            (
                update_character_state,
                advance_animation_system,
                query_character_sprites,
            )
                .chain(),
        );
        app.add_systems(
            PostUpdate,
            query_character_sprites.in_set(CharacterSet::SpritePush),
        );

        info!("CharacterPlugin registered");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_components_auto_inserted() {
        // Verify Character Required Components auto-inserts ActionState, SpriteReq, AnimationState, Transform
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Spawn with ONLY Character -- Required Components should auto-insert the rest
        let entity = app.world_mut().spawn(Character).id();

        // Required components are inserted via hooks, need an update to process
        app.update();

        let world = app.world();
        assert!(
            world.get::<ActionState>(entity).is_some(),
            "ActionState should be auto-inserted"
        );
        assert!(
            world.get::<SpriteReq>(entity).is_some(),
            "SpriteReq should be auto-inserted"
        );
        assert!(
            world.get::<AnimationState>(entity).is_some(),
            "AnimationState should be auto-inserted"
        );
        assert!(
            world.get::<Transform>(entity).is_some(),
            "Transform should be auto-inserted"
        );
    }
}
