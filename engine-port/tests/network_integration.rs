//! Network integration tests: server+client connectivity over renet transport.
//!
//! Uses bevy_replicon + bevy_replicon_renet2 with in-memory transport sockets
//! to verify transport-level connectivity without binding real UDP ports.
//!
//! The connectivity test is #[ignore] by default because it exercises the full
//! network stack (timing-dependent initialization). Run with:
//!   cargo test -- --ignored network_integration

use std::time::SystemTime;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::netcode::{
    in_memory_server_addr, new_memory_sockets, ClientAuthentication, MemorySocketClient,
    NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig,
};
use bevy_replicon_renet2::renet2::{ConnectionConfig, RenetClient, RenetServer};
use bevy_replicon_renet2::{RenetChannelsExt, RepliconRenetPlugins};

use asciicker_engine::network::protocol::PoseUpdate;

const PROTOCOL_ID: u64 = 7007; // Arbitrary protocol ID for tests.

/// Create in-memory server transport and client sockets.
///
/// Uses `memory_transport` feature of bevy_replicon_renet2 to avoid UDP.
fn create_server_transport(num_clients: usize) -> (NetcodeServerTransport, Vec<MemorySocketClient>) {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerSetupConfig {
        socket_addresses: vec![vec![in_memory_server_addr()]],
        current_time,
        max_clients: num_clients,
        protocol_id: PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
    };
    let client_ids: Vec<u16> = (1..=num_clients as u16).collect();
    let (server_socket, client_sockets) = new_memory_sockets(client_ids, false, false);

    (
        NetcodeServerTransport::new(server_config, server_socket).unwrap(),
        client_sockets,
    )
}

/// Create in-memory client transport from a pre-created socket.
fn create_client_transport(socket: MemorySocketClient) -> NetcodeClientTransport {
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let authentication = ClientAuthentication::Unsecure {
        client_id: socket.id(),
        protocol_id: PROTOCOL_ID,
        socket_id: 0,
        server_addr: in_memory_server_addr(),
        user_data: None,
    };

    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
}

/// Build a server App with RepliconPlugins + RepliconRenetPlugins + PoseUpdate replication.
///
/// Returns the App and client sockets for connecting clients.
fn build_server_app(num_clients: usize) -> (App, Vec<MemorySocketClient>) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        RepliconRenetPlugins,
    ))
    .replicate::<PoseUpdate>()
    .finish();

    let channels = app.world().resource::<RepliconChannels>();
    let server_channels = channels.server_configs();
    let client_channels = channels.client_configs();

    let server = RenetServer::new(ConnectionConfig::from_channels(
        server_channels,
        client_channels,
    ));
    let (transport, client_sockets) = create_server_transport(num_clients);

    app.insert_resource(server).insert_resource(transport);

    (app, client_sockets)
}

/// Build a client App with RepliconPlugins + RepliconRenetPlugins + PoseUpdate replication.
fn build_client_app(socket: MemorySocketClient) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        RepliconRenetPlugins,
    ))
    .replicate::<PoseUpdate>()
    .finish();

    let channels = app.world().resource::<RepliconChannels>();
    let server_channels = channels.server_configs();
    let client_channels = channels.client_configs();

    let client = RenetClient::new(
        ConnectionConfig::from_channels(server_channels, client_channels),
        false,
    );
    let transport = create_client_transport(socket);

    app.insert_resource(client).insert_resource(transport);

    app
}

/// Run update loops until the client connects (or give up after N iterations).
fn wait_for_connection(server_app: &mut App, client_app: &mut App, max_iters: usize) -> bool {
    for _ in 0..max_iters {
        server_app.update();
        client_app.update();
        if client_app
            .world()
            .resource::<RenetClient>()
            .is_connected()
        {
            return true;
        }
    }
    false
}

/// Verify that a server and client can connect and exchange data over in-memory
/// renet transport with bevy_replicon replication of PoseUpdate.
///
/// This is marked `#[ignore]` because it exercises the full replicon+renet2 transport
/// stack which involves timing-dependent initialization. Run with:
///   cargo test -- --ignored network_integration
#[test]
#[ignore]
fn test_server_client_join_exchange() {
    // Build server and client apps with in-memory transport.
    let (mut server_app, mut client_sockets) = build_server_app(1);
    let client_socket = client_sockets.pop().expect("should have one client socket");
    let mut client_app = build_client_app(client_socket);

    // Connect client to server via memory transport.
    let connected = wait_for_connection(&mut server_app, &mut client_app, 50);
    assert!(connected, "Client should connect to server via memory transport");

    // Verify server sees exactly one connected client.
    let renet_server = server_app.world().resource::<RenetServer>();
    assert_eq!(
        renet_server.connected_clients(),
        1,
        "Server should have exactly one connected client"
    );

    // Verify client is connected.
    let renet_client = client_app.world().resource::<RenetClient>();
    assert!(
        renet_client.is_connected(),
        "Client should report connected state"
    );

    // Verify server-side replicon sees the ConnectedClient entity.
    let mut connected_query = server_app.world_mut().query::<&ConnectedClient>();
    let connected_count = connected_query.iter(server_app.world()).count();
    assert_eq!(
        connected_count, 1,
        "Server should have one ConnectedClient entity"
    );

    // Spawn a replicated entity with PoseUpdate on the server.
    let test_pose = PoseUpdate {
        anim: 1,
        frame: 5,
        action_mount: 0,
        pos: [10.0, 20.0, 30.0],
        dir: 1.57,
        sprite: 42,
    };
    server_app
        .world_mut()
        .spawn((test_pose.clone(), Replicated));

    // Run a few update cycles to propagate replication.
    for _ in 0..5 {
        server_app.update();
        client_app.update();
    }

    // Verify the client received the replicated PoseUpdate entity.
    let mut pose_query = client_app.world_mut().query::<&PoseUpdate>();
    let poses: Vec<&PoseUpdate> = pose_query.iter(client_app.world()).collect();
    assert!(
        !poses.is_empty(),
        "Client should have received at least one replicated PoseUpdate entity"
    );

    // Verify the replicated data matches.
    let received_pose = poses[0];
    assert_eq!(received_pose.anim, 1);
    assert_eq!(received_pose.frame, 5);
    assert_eq!(received_pose.pos, [10.0, 20.0, 30.0]);
    assert_eq!(received_pose.sprite, 42);
}

/// Verify PoseUpdate implements the required traits for bevy_replicon replication:
/// - Component (required for ECS attachment and replication)
/// - Serialize + Deserialize (required for network transport)
/// - Default (required for initial state)
///
/// This test is deterministic (no network) and runs without #[ignore].
#[test]
fn test_protocol_types_are_replicon_compatible() {
    // PoseUpdate::default() should produce zero-valued fields.
    let pose = PoseUpdate::default();
    assert_eq!(pose.anim, 0, "default anim should be 0");
    assert_eq!(pose.frame, 0, "default frame should be 0");
    assert_eq!(pose.action_mount, 0, "default action_mount should be 0");
    assert_eq!(pose.pos, [0.0, 0.0, 0.0], "default pos should be origin");
    assert_eq!(pose.dir, 0.0, "default dir should be 0");
    assert_eq!(pose.sprite, 0, "default sprite should be 0");

    // Serialize + Deserialize round-trip via bincode.
    let test_pose = PoseUpdate {
        anim: 3,
        frame: 7,
        action_mount: 1,
        pos: [100.5, -200.25, 50.0],
        dir: std::f32::consts::PI,
        sprite: 42,
    };
    let bytes = bincode::serialize(&test_pose).expect("PoseUpdate should serialize");
    let deserialized: PoseUpdate =
        bincode::deserialize(&bytes).expect("PoseUpdate should deserialize");
    assert_eq!(test_pose, deserialized, "round-trip should be lossless");

    // Component trait: verify PoseUpdate can be added to a Bevy App's ECS.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let entity = app.world_mut().spawn(PoseUpdate::default()).id();
    let retrieved = app.world().get::<PoseUpdate>(entity);
    assert!(
        retrieved.is_some(),
        "PoseUpdate should be retrievable as a Component"
    );
}
