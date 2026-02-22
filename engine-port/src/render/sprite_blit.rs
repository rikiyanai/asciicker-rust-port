//! Deferred sprite blit system.
//!
//! Sprites are queued during the WORLD stage and blitted after RESOLVE
//! in far-to-near order. This matches the C++ engine's deferred sprite
//! rendering pattern.

use std::cmp::Ordering;

use bevy::prelude::*;

use crate::output::ascii_cell_grid::AsciiCellGrid;
use crate::render::sample_buffer::SampleBuffer;

/// A single sprite render entry queued during the WORLD stage.
#[derive(Debug, Clone)]
pub struct SpriteRenderEntry {
    /// Distance along the camera view direction (for far-to-near sort).
    pub dist: f32,
    /// Screen X position (ASCII cell coordinate).
    pub screen_x: i32,
    /// Screen Y position (ASCII cell coordinate).
    pub screen_y: i32,
    /// Sprite name (for asset lookup).
    pub sprite_name: String,
    /// World-space position.
    pub pos: [f32; 3],
    /// Yaw angle.
    pub yaw: f32,
    /// Animation index.
    pub anim: u32,
    /// Frame index.
    pub frame: u32,
}

/// Queue of deferred sprite render entries.
///
/// Sprites are pushed during the WORLD stage, sorted far-to-near after
/// RESOLVE, then blitted onto the AsciiCellGrid.
///
/// Phase 5: SpriteQueue is cleared at Stage 3 WORLD start.
/// Phase 6 migration: clear moves to PreUpdate (clear_sprite_queue_system).
///
/// ORDERING CONTRACT: SpriteQueue
///
/// WRITERS (ResMut<SpriteQueue>):
///   PreUpdate:  clear_sprite_queue_system   [CharacterPlugin, Phase 6]
///   PostUpdate: query_character_sprites     [CharacterPlugin, Phase 6]
///   PostUpdate: render_pipeline_system      [CpuRasterizerPlugin, Phase 5]
///     Stage 3 WORLD: appends world sprite instances
///
/// READERS (Res<SpriteQueue>):
///   PostUpdate: render_pipeline_system post-RESOLVE blit
///
/// PHASE 5 -> PHASE 6 MIGRATION:
///   Phase 5: sprite_queue.clear() inside Stage 3 WORLD.
///   Phase 6: REMOVE Stage 3 clear, add clear_sprite_queue_system in PreUpdate.
#[derive(Resource, Default)]
pub struct SpriteQueue {
    entries: Vec<SpriteRenderEntry>,
}

impl SpriteQueue {
    /// Push a sprite entry onto the queue.
    pub fn push(&mut self, entry: SpriteRenderEntry) {
        debug_assert!(!entry.dist.is_nan(), "SpriteRenderEntry.dist must not be NaN");
        self.entries.push(entry);
    }

    /// Sort entries by descending distance (far-to-near).
    pub fn sort_far_to_near(&mut self) {
        self.entries
            .sort_by(|a, b| b.dist.partial_cmp(&a.dist).unwrap_or(Ordering::Equal));
    }

    /// Drain all entries from the queue.
    pub fn drain(&mut self) -> std::vec::Drain<'_, SpriteRenderEntry> {
        self.entries.drain(..)
    }

    /// Number of entries in the queue.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Phase 5 placeholder sprite blit.
///
/// Marks the sprite's screen position with a visible 'S' character.
/// `_sample_buffer` parameter is for future depth testing (Phase 6+ scope).
/// Full XP frame blit is deferred to Phase 7.
pub fn blit_sprite(
    cell_grid: &mut AsciiCellGrid,
    entry: &SpriteRenderEntry,
    _sample_buffer: &SampleBuffer,
) {
    let x = entry.screen_x;
    let y = entry.screen_y;

    // Bounds check
    if x < 0 || y < 0 || x >= cell_grid.width as i32 || y >= cell_grid.height as i32 {
        return;
    }

    let ux = x as u32;
    let uy = y as u32;

    // Placeholder: mark sprite position with 'S'
    cell_grid.set_cell(
        ux,
        uy,
        b'S' as u16,
        [255, 255, 0, 255],  // yellow foreground
        [64, 0, 64, 255],    // dark purple background
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_sort_far_to_near() {
        let mut queue = SpriteQueue::default();

        // Push entries with varying distances
        for (i, dist) in [5.0_f32, 20.0, 1.0, 15.0, 10.0].iter().enumerate() {
            queue.push(SpriteRenderEntry {
                dist: *dist,
                screen_x: i as i32,
                screen_y: 0,
                sprite_name: format!("sprite_{i}"),
                pos: [0.0; 3],
                yaw: 0.0,
                anim: 0,
                frame: 0,
            });
        }

        queue.sort_far_to_near();

        // Verify descending order
        let dists: Vec<f32> = queue.entries.iter().map(|e| e.dist).collect();
        assert_eq!(dists, vec![20.0, 15.0, 10.0, 5.0, 1.0]);
    }

    #[test]
    fn test_sprite_queue_push_drain() {
        let mut queue = SpriteQueue::default();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue.push(SpriteRenderEntry {
            dist: 10.0,
            screen_x: 5,
            screen_y: 5,
            sprite_name: "test".to_string(),
            pos: [0.0; 3],
            yaw: 0.0,
            anim: 0,
            frame: 0,
        });

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let drained: Vec<_> = queue.drain().collect();
        assert_eq!(drained.len(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_sprite_queue_clear() {
        let mut queue = SpriteQueue::default();
        for i in 0..5 {
            queue.push(SpriteRenderEntry {
                dist: i as f32,
                screen_x: 0,
                screen_y: 0,
                sprite_name: String::new(),
                pos: [0.0; 3],
                yaw: 0.0,
                anim: 0,
                frame: 0,
            });
        }
        assert_eq!(queue.len(), 5);
        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_blit_sprite_placeholder() {
        let mut grid = AsciiCellGrid::new(10, 10);
        let buf = SampleBuffer::new(10, 10);

        let entry = SpriteRenderEntry {
            dist: 10.0,
            screen_x: 5,
            screen_y: 5,
            sprite_name: "test".to_string(),
            pos: [0.0; 3],
            yaw: 0.0,
            anim: 0,
            frame: 0,
        };

        blit_sprite(&mut grid, &entry, &buf);

        let (ch, fg, _bg) = grid.cell_at(5, 5);
        assert_eq!(ch, b'S' as u16, "Sprite placeholder should be 'S'");
        assert_eq!(fg, [255, 255, 0, 255], "Sprite fg should be yellow");
    }

    #[test]
    fn test_blit_sprite_out_of_bounds() {
        let mut grid = AsciiCellGrid::new(10, 10);
        let buf = SampleBuffer::new(10, 10);

        // Out of bounds -- should not panic
        let entry = SpriteRenderEntry {
            dist: 10.0,
            screen_x: -1,
            screen_y: 5,
            sprite_name: "test".to_string(),
            pos: [0.0; 3],
            yaw: 0.0,
            anim: 0,
            frame: 0,
        };
        blit_sprite(&mut grid, &entry, &buf);

        let entry2 = SpriteRenderEntry {
            dist: 10.0,
            screen_x: 100,
            screen_y: 5,
            sprite_name: "test".to_string(),
            pos: [0.0; 3],
            yaw: 0.0,
            anim: 0,
            frame: 0,
        };
        blit_sprite(&mut grid, &entry2, &buf);
        // No panic = pass
    }
}
