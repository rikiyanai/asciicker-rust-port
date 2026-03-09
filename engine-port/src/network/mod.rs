//! Networking module: client-server multiplayer via bevy_replicon + renet2.
//!
//! Provides NetworkPlugin with three modes:
//! - Standalone: no network transport, singleplayer (default)
//! - Server: accepts client connections, broadcasts state
//! - Client: connects to a server, sends/receives updates
//!
//! Uses bevy_replicon 0.39 for server-authoritative entity replication
//! with bevy_replicon_renet2 0.14 as the transport bridge.

pub mod client;
pub mod protocol;
pub mod server;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RepliconRenetPlugins;

use protocol::PoseUpdate;

/// Network operating mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkMode {
    /// No network transport. Singleplayer mode. Default.
    Standalone,
    /// Listen for client connections on the given port.
    Server { port: u16 },
    /// Connect to a server at the given address.
    Client { addr: String },
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Standalone
    }
}

/// Network configuration resource.
#[derive(Resource, Debug, Clone)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Standalone,
        }
    }
}

/// Network plugin. Registers bevy_replicon + renet2 transport, PoseUpdate
/// replication, and conditionally adds server/client systems.
///
/// R19-007 FIX: RepliconPlugins registered only in Server/Client modes
/// to avoid transport overhead in Standalone mode.
pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        // Initialize config resource (default: Standalone)
        app.init_resource::<NetworkConfig>();

        let config = app.world().resource::<NetworkConfig>().clone();

        match config.mode {
            NetworkMode::Standalone => {
                // No network plugins needed in standalone mode.
                // RepliconPlugins are NOT registered to avoid transport overhead.
                info!("NetworkPlugin registered (Standalone mode - no transport)");
            }
            NetworkMode::Server { .. } | NetworkMode::Client { .. } => {
                // Register replicon + renet2 transport plugins
                app.add_plugins((RepliconPlugins, RepliconRenetPlugins));

                // Register PoseUpdate as a replicated component
                app.replicate::<PoseUpdate>();

                match &config.mode {
                    NetworkMode::Server { .. } => {
                        app.init_resource::<server::ConnectedPlayers>();
                        app.add_systems(
                            PreUpdate,
                            server::handle_player_connect
                                .after(ServerSystems::Receive),
                        );
                        info!("NetworkPlugin registered (Server mode)");
                    }
                    NetworkMode::Client { .. } => {
                        app.add_systems(
                            PostUpdate,
                            (
                                client::send_local_pose,
                                client::apply_remote_poses,
                            ),
                        );
                        info!("NetworkPlugin registered (Client mode)");
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
