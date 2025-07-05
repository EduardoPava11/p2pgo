// SPDX-License-Identifier: MIT OR Apache-2.0

//! Common utilities for P2P Go integration tests

use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;

use p2pgo_network::{relay_monitor::RestartableRelay, GameChannel, GameId, IrohCtx, Lobby};

/// Re-export utilities
pub mod test_utils;

// Initialize logging for tests
static INIT_LOGGING: Lazy<()> = Lazy::new(|| {
    // Only show warnings and errors unless RUST_LOG is explicitly set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    let _ = env_logger::try_init();
});

/// Configuration for test peers
#[derive(Debug, Clone)]
pub struct PeerConfig {
    /// Display name for the peer
    pub name: String,
    /// Board size to use (9, 13, or 19)
    pub board_size: u8,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            name: "TestPeer".to_string(),
            board_size: 9,
        }
    }
}

/// A test peer with networking context
pub struct TestPeer {
    /// The peer's name
    pub name: String,
    /// The IrohCtx for networking
    pub iroh_ctx: Arc<IrohCtx>,
    /// The game lobby
    pub lobby: Lobby,
    /// Active game channel (if any)
    pub game_channel: Option<Arc<GameChannel>>,
    /// Game ID (if any)
    pub game_id: Option<String>,
    /// Board size for the game
    pub board_size: u8,
}

impl TestPeer {
    /// Create a new test peer
    pub async fn new(config: PeerConfig) -> Result<Self> {
        // Ensure logging is initialized
        Lazy::force(&INIT_LOGGING);

        // Create IrohCtx
        let iroh_ctx = Arc::new(p2pgo_network::IrohCtx::new().await?);

        // Create lobby
        let lobby = Lobby::new();

        Ok(Self {
            name: config.name,
            iroh_ctx,
            lobby,
            game_channel: None,
            game_id: None,
            board_size: config.board_size,
        })
    }

    /// Create a new game
    pub async fn create_game(&mut self) -> Result<String> {
        let name = Some(format!("test-game-{}", uuid::Uuid::new_v4()));
        let needs_password = false;
        let game_id = self
            .lobby
            .create_game(name, self.board_size, needs_password)
            .await?;

        // Get the channel
        let channel = self.lobby.get_game_channel(&game_id).await?;
        self.game_channel = Some(channel);
        self.game_id = Some(game_id.clone());

        Ok(game_id)
    }

    /// Join a game by ID
    pub async fn join_game(&mut self, game_id: &str) -> Result<()> {
        // Get the channel for the game
        let channel = self.lobby.get_game_channel(&game_id).await?;
        self.game_channel = Some(channel);
        self.game_id = Some(game_id.to_string());

        Ok(())
    }

    /// Get connection ticket
    pub async fn get_ticket(&self) -> Result<String> {
        self.iroh_ctx.ticket().await
    }

    /// Connect to another peer by ticket
    pub async fn connect_by_ticket(&mut self, ticket: &str) -> Result<()> {
        self.iroh_ctx.connect_by_ticket(ticket).await
    }
}

/// Spawn two connected peers for testing
pub async fn spawn_two_peers() -> Result<(TestPeer, TestPeer)> {
    // Create Alice and Bob
    let mut alice = TestPeer::new(PeerConfig {
        name: "Alice".to_string(),
        board_size: 9,
    })
    .await?;

    let mut bob = TestPeer::new(PeerConfig {
        name: "Bob".to_string(),
        board_size: 9,
    })
    .await?;

    // Connect the peers
    let alice_ticket = alice.get_ticket().await?;
    bob.connect_by_ticket(&alice_ticket).await?;

    // Create a game with Alice as host
    let game_id = alice.create_game().await?;

    // Have Bob join
    bob.join_game(&game_id).await?;

    // Wait for connection to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((alice, bob))
}

/// Configuration for relay spawn
pub struct RelayConfig {
    /// Maximum concurrent connections allowed
    pub max_connections: usize,
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: f64,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            max_connections: 200,
            max_bandwidth_mbps: 10.0,
        }
    }
}

/// A test relay for integration tests
pub struct TestRelay {
    /// Port manager for the relay
    pub port_manager: p2pgo_network::port::PortManager,
    /// Restartable relay service
    pub relay: RestartableRelay,
    /// TCP port the relay is listening on
    pub tcp_port: u16,
    /// UDP port the relay is listening on
    pub udp_port: u16,
}

impl TestRelay {
    /// Get the relay address string
    pub fn get_relay_addr(&self) -> String {
        format!("/ip4/127.0.0.1/tcp/{}/quic-v1", self.tcp_port)
    }
}

/// Spawn a test relay with configurable limits
pub async fn spawn_relay(config: RelayConfig) -> Result<TestRelay> {
    // Ensure logging is initialized
    Lazy::force(&INIT_LOGGING);

    // Create port manager
    let port_manager = p2pgo_network::port::PortManager::new()?;

    // Create relay with limits    // Create relay with builder pattern
    let mut relay = RestartableRelay::new(port_manager.clone())
        .connection_limit(config.max_connections)
        .bandwidth_limit(config.max_bandwidth_mbps);

    // Start the relay
    let (tcp_port, udp_port) = relay.start_embedded_relay().await?;

    Ok(TestRelay {
        port_manager,
        relay,
        tcp_port,
        udp_port,
    })
}
