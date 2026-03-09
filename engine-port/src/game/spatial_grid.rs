use bevy::prelude::*;
use std::collections::HashMap;

/// Size of each spatial grid cell in world units.
/// Matching VISUAL_CELLS from C++ (8.0).
pub const CELL_SIZE: f32 = 8.0;

/// SpatialGrid Resource for dynamic entity indexing.
///
/// Provides O(1) entity insertion and removal.
/// Replaces C++ dynamic BSP rebuilding for proximity and raycast queries.
#[derive(Resource, Default)]
pub struct SpatialGrid {
    /// Cell index to list of entities.
    pub cells: HashMap<(i32, i32, i32), Vec<Entity>>,
    /// Entity to cell index reverse mapping for efficient removal.
    pub entity_cells: HashMap<Entity, (i32, i32, i32)>,
}

impl SpatialGrid {
    /// Convert world position to grid coordinates.
    pub fn world_to_grid(pos: Vec3) -> (i32, i32, i32) {
        (
            (pos.x / CELL_SIZE).floor() as i32,
            (pos.y / CELL_SIZE).floor() as i32,
            (pos.z / CELL_SIZE).floor() as i32,
        )
    }

    /// Add an entity to a specific cell.
    pub fn add(&mut self, cell: (i32, i32, i32), entity: Entity) {
        self.cells.entry(cell).or_default().push(entity);
        self.entity_cells.insert(entity, cell);
    }

    /// Remove an entity from its current cell.
    pub fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(entities) = self.cells.get_mut(&cell) {
                if let Some(pos) = entities.iter().position(|&e| e == entity) {
                    entities.swap_remove(pos);
                }
                if entities.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    /// Query entities in a 3x3x3 neighborhood around a world position.
    pub fn nearby_entities(&self, pos: Vec3) -> impl Iterator<Item = Entity> + '_ {
        let (gx, gy, gz) = Self::world_to_grid(pos);
        let mut result = Vec::new();

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(entities) = self.cells.get(&(gx + dx, gy + dy, gz + dz)) {
                        result.extend(entities.iter().cloned());
                    }
                }
            }
        }

        result.into_iter()
    }
}

/// Component tracking which spatial grid cell an entity resides in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialGridCell(pub i32, pub i32, pub i32);

/// System to sync SpatialGrid resource with entity movement.
///
/// Runs in PostUpdate so it captures final transform changes.
/// TRAP-G01: Always use floor() for grid coordinates to handle negative coords correctly.
pub fn sync_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    mut query: Query<(Entity, &GlobalTransform, Option<&mut SpatialGridCell>)>,
    mut commands: Commands,
) {
    for (entity, transform, mut cell_opt) in query.iter_mut() {
        let pos = transform.translation();
        let (gx, gy, gz) = SpatialGrid::world_to_grid(pos);

        if let Some(ref mut cell) = cell_opt {
            if cell.0 != gx || cell.1 != gy || cell.2 != gz {
                // Entity moved cells
                grid.remove(entity);
                grid.add((gx, gy, gz), entity);
                cell.0 = gx;
                cell.1 = gy;
                cell.2 = gz;
            }
        } else {
            // New entity: add to grid and insert component
            grid.add((gx, gy, gz), entity);
            commands.entity(entity).insert(SpatialGridCell(gx, gy, gz));
        }
    }
}

/// System to clean up SpatialGrid resource when SpatialGridCell component is removed.
///
/// This handles entity despawn and explicit component removal.
pub fn cleanup_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    mut removed: RemovedComponents<SpatialGridCell>,
) {
    for entity in removed.read() {
        grid.remove(entity);
    }
}

impl SpatialGrid {
    /// Raycast against entities in the spatial grid.
    ///
    /// Uses 3D DDA grid walk algorithm.
    /// Returns Option<(Entity, distance)> for the first entity hit.
    ///
    /// TRAP: This currently only checks which cells the ray passes through.
    /// Callers may need additional intersection logic (e.g. sphere-vs-ray)
    /// within each cell. This plan assumes dynamic entities have a
    /// standard hit volume.
    pub fn raycast_entities(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_dist: f32,
        query: &Query<&GlobalTransform>,
    ) -> Option<(Entity, f32)> {
        if direction.length_squared() < 1e-6 {
            return None;
        }

        let dir = direction.normalize();
        let (mut gx, mut gy, mut gz) = Self::world_to_grid(origin);

        let step_x = if dir.x > 0.0 { 1 } else { -1 };
        let step_y = if dir.y > 0.0 { 1 } else { -1 };
        let step_z = if dir.z > 0.0 { 1 } else { -1 };

        // Guard against axis-aligned rays: if a component is near zero,
        // set delta to f32::MAX so that axis is never the min and we
        // never step along it (avoids NaN / infinite loop).
        let delta_x = if dir.x.abs() > 1e-10 { CELL_SIZE / dir.x.abs() } else { f32::MAX };
        let delta_y = if dir.y.abs() > 1e-10 { CELL_SIZE / dir.y.abs() } else { f32::MAX };
        let delta_z = if dir.z.abs() > 1e-10 { CELL_SIZE / dir.z.abs() } else { f32::MAX };

        let mut max_x = if dir.x.abs() > 1e-10 {
            if dir.x > 0.0 {
                (((gx + 1) as f32 * CELL_SIZE) - origin.x) / dir.x
            } else {
                ((gx as f32 * CELL_SIZE) - origin.x) / dir.x
            }
        } else {
            f32::MAX
        };

        let mut max_y = if dir.y.abs() > 1e-10 {
            if dir.y > 0.0 {
                (((gy + 1) as f32 * CELL_SIZE) - origin.y) / dir.y
            } else {
                ((gy as f32 * CELL_SIZE) - origin.y) / dir.y
            }
        } else {
            f32::MAX
        };

        let mut max_z = if dir.z.abs() > 1e-10 {
            if dir.z > 0.0 {
                (((gz + 1) as f32 * CELL_SIZE) - origin.z) / dir.z
            } else {
                ((gz as f32 * CELL_SIZE) - origin.z) / dir.z
            }
        } else {
            f32::MAX
        };

        let mut current_dist = 0.0;

        while current_dist <= max_dist {
            // Check current cell
            if let Some(entities) = self.cells.get(&(gx, gy, gz)) {
                let mut best_hit: Option<(Entity, f32)> = None;

                for &entity in entities {
                    if let Ok(transform) = query.get(entity) {
                        let entity_pos = transform.translation();
                        // SIMPLE: Sphere collision with radius 1.0 for all entities for now.
                        // Phase 8 Plan 01 Objective focuses on infrastructure.
                        let to_entity = entity_pos - origin;
                        let t = to_entity.dot(dir);
                        if t > 0.0 && t <= max_dist {
                            let nearest_point = origin + dir * t;
                            let dist_sq = (nearest_point - entity_pos).length_squared();
                            if dist_sq < 1.0 {
                                // Hit!
                                if best_hit.is_none() || t < best_hit.unwrap().1 {
                                    best_hit = Some((entity, t));
                                }
                            }
                        }
                    }
                }

                if let Some(hit) = best_hit {
                    return Some(hit);
                }
            }

            // Advance to next cell
            if max_x < max_y {
                if max_x < max_z {
                    current_dist = max_x;
                    max_x += delta_x;
                    gx += step_x;
                } else {
                    current_dist = max_z;
                    max_z += delta_z;
                    gz += step_z;
                }
            } else {
                if max_y < max_z {
                    current_dist = max_y;
                    max_y += delta_y;
                    gy += step_y;
                } else {
                    current_dist = max_z;
                    max_z += delta_z;
                    gz += step_z;
                }
            }
        }

        None
    }
}
