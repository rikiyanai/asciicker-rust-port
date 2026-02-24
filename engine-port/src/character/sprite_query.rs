//! Character sprite query system.
//!
//! Reads SpriteReq + AnimationState from character entities and creates
//! SpriteRenderEntry items for the deferred sprite blit queue.
//!
//! This system bridges character state to the rendering pipeline (Gap 5).
//! Without it, character entities exist in ECS but are never drawn.
//!
//! R19-M03 NOTE: Character sprites will be INVISIBLE between 06-02 and 06-03.
//! PostUpdate push + Update pipeline clear = sprites pushed after pipeline already cleared.
//! Plan 06-03 fixes by: (1) removing Stage 3 sprite_queue.clear(), and
//! (2) moving render_pipeline_system to PostUpdate.

use bevy::prelude::*;

use crate::physics::PhysicsIO;
use crate::render::camera::GameCamera;
use crate::render::pipeline::project_world_to_screen;
use crate::render::sprite_blit::{SpriteQueue, SpriteRenderEntry};

use super::animation::AnimationState;
use super::equipment::SpriteReq;
use super::state_machine::Character;

/// Query character entities and push SpriteRenderEntry items to SpriteQueue.
///
/// Runs in PostUpdate (after physics sync). Each character produces one sprite entry.
///
/// R19-M10 NOTE: 8-direction index for sprite sheet column is not computed here.
/// The placeholder 'S' blit doesn't use direction; full sprite blit (Phase 7+)
/// will compute direction = ((entity_yaw - camera_yaw + 360 + 22.5) / 45.0) as u32 % 8.
pub fn query_character_sprites(
    characters: Query<(&SpriteReq, &AnimationState, &Transform), With<Character>>,
    camera: Res<GameCamera>,
    physics_io: Res<PhysicsIO>,
    mut sprite_queue: ResMut<SpriteQueue>,
) {
    for (sprite_req, anim_state, transform) in &characters {
        let pos = transform.translation;

        // 1. Sprite name from kind + action
        let sprite_name = format!(
            "{:?}_{:?}",
            sprite_req.kind, sprite_req.action
        ).to_lowercase();

        // 2. Animation frame (R19-M02: already u32, no cast needed)
        let frame = anim_state.frame_index;

        // 3. Screen position: project through camera
        // R19-M01 FIX: Use project_world_to_screen() (not view_tm * pos, which won't compile)
        let pos_arr = [pos.x, pos.y, pos.z];
        let (screen_x, screen_y) = match project_world_to_screen(&pos_arr, &camera) {
            Some(coords) => coords, // (i32, i32)
            None => continue,       // behind camera, skip
        };

        // 4. Distance for far-to-near sort (2D XY distance)
        let dx = camera.pos[0] - pos.x;
        let dy = camera.pos[1] - pos.y;
        let dist = dx.hypot(dy);

        // 5. Yaw from PhysicsIO (authoritative value, NOT transform rotation)
        let yaw = physics_io.yaw;

        let entry = SpriteRenderEntry {
            dist,
            screen_x,  // R19-M02: i32 from project_world_to_screen
            screen_y,   // R19-M02: i32 from project_world_to_screen
            sprite_name,
            pos: [pos.x, pos.y, pos.z],
            yaw,
            anim: sprite_req.action as u32, // R19-M02: u32 field
            frame,                           // R19-M02: u32, no cast
        };
        sprite_queue.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::equipment::SpriteKind;
    use crate::character::state_machine::ActionState;

    #[test]
    fn test_character_marker_filters_non_characters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<GameCamera>();
        app.insert_resource(PhysicsIO::default());
        app.init_resource::<SpriteQueue>();
        app.add_systems(Update, query_character_sprites);

        // Spawn character (has Character marker)
        app.world_mut().spawn((
            Character,
            SpriteReq::default(),
            AnimationState::default(),
            Transform::from_xyz(5.0, 5.0, 0.0),
            ActionState::None,
        ));

        // Spawn decoy WITHOUT Character marker (has Transform + SpriteReq but NO Character)
        app.world_mut().spawn((
            SpriteReq::default(),
            AnimationState::default(),
            Transform::from_xyz(10.0, 10.0, 0.0),
        ));

        // Initialize camera at a position where characters are in front
        {
            let mut cam = app.world_mut().get_resource_mut::<GameCamera>().unwrap();
            cam.pos = [0.0, 0.0, 0.0];
            cam.yaw = 0.0;
            cam.update(484.0, 274.0);
        }

        app.update();

        let queue = app.world().get_resource::<SpriteQueue>().unwrap();
        // Only the Character entity should produce an entry (decoy excluded)
        assert_eq!(queue.len(), 1, "Only character entity should produce sprite entry, got {}", queue.len());
    }

    #[test]
    fn test_sprite_name_format() {
        // Verify sprite_name is lowercase kind_action
        let req = SpriteReq { kind: SpriteKind::Human, action: ActionState::Attack, ..Default::default() };
        let name = format!("{:?}_{:?}", req.kind, req.action).to_lowercase();
        assert_eq!(name, "human_attack");
    }
}
