//! Server-side networking systems.
//!
//! Handles client connections, player spawning, and state broadcasting.
//! Uses bevy_replicon's server-authoritative model.
//!
//! In bevy_replicon 0.39, connected clients are represented as entities with
//! the ConnectedClient component. The Entity itself serves as the client ID.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::character::{SpriteReq, spawn_character};
use crate::network::protocol::PoseUpdate;

/// Marker component for network-identified players.
///
/// Stores the client entity (from bevy_replicon's ConnectedClient) for
/// correlation between network events and game character entities.
#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkPlayer(pub Entity);

/// Tracks connected players by their client entity.
#[derive(Resource, Debug, Default)]
pub struct ConnectedPlayers {
    /// Map from client entity to player name.
    pub players: HashMap<Entity, String>,
}

impl ConnectedPlayers {
    /// Add a player.
    pub fn add(&mut self, client_entity: Entity, name: String) {
        self.players.insert(client_entity, name);
    }

    /// Remove a player. Returns their name if they existed.
    pub fn remove(&mut self, client_entity: &Entity) -> Option<String> {
        self.players.remove(client_entity)
    }

    /// Check if a player is connected.
    pub fn contains(&self, client_entity: &Entity) -> bool {
        self.players.contains_key(client_entity)
    }

    /// Number of connected players.
    pub fn count(&self) -> usize {
        self.players.len()
    }
}

/// Handle new player connections.
///
/// R14-F147 FIX: Calls spawn_character() directly (no Command struct).
/// R19-002 FIX: Passes Vec3 position and SpriteReq::default() matching 06-02 signature.
///
/// Listens for ConnectedClient additions (bevy_replicon auto-spawns these).
/// Spawns a character entity and attaches network components.
pub fn handle_player_connect(
    mut commands: Commands,
    mut connected_players: ResMut<ConnectedPlayers>,
    query: Query<Entity, Added<ConnectedClient>>,
) {
    for client_entity in &query {
        let player_name = format!("Player_{}", client_entity);

        info!(
            "Player connected: {} (entity: {:?})",
            player_name, client_entity
        );

        // Spawn character entity using the canonical spawn_character() from Phase 6 (06-02).
        // R19-002: Correct signature: (commands, position: Vec3, equipment: SpriteReq)
        let spawn_pos = Vec3::new(0.0, 0.0, 50.0);
        let entity = spawn_character(&mut commands, spawn_pos, SpriteReq::default());

        // Add network-specific components to the spawned character entity.
        commands.entity(entity).insert((
            PoseUpdate::default(),
            Replicated,
            NetworkPlayer(client_entity),
        ));

        connected_players.add(client_entity, player_name);
    }
}

/// Handle player disconnections.
///
/// Cleans up ConnectedPlayers and despawns character entities for disconnected clients.
pub fn handle_player_disconnect(
    mut connected_players: ResMut<ConnectedPlayers>,
    mut commands: Commands,
    mut removed: RemovedComponents<ConnectedClient>,
    network_players: Query<(Entity, &NetworkPlayer)>,
) {
    for disconnected_entity in removed.read() {
        // Find the game entity associated with this disconnected client
        for (game_entity, network_player) in &network_players {
            if network_player.0 == disconnected_entity {
                info!(
                    "Despawning character for disconnected client {:?}",
                    disconnected_entity
                );
                commands.entity(game_entity).despawn();
            }
        }

        // Remove from connected players tracking
        if let Some(name) = connected_players.remove(&disconnected_entity) {
            info!("Player disconnected: {}", name);
        }
    }
}
