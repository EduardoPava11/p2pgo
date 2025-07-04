// SPDX-License-Identifier: MIT OR Apache-2.0

//! Iroh networking endpoint management for P2P Go

use anyhow::Result;

#[cfg(feature = "iroh")]
use {
    iroh::{NodeAddr, Endpoint},
    iroh::{endpoint::Connection, protocol::{ProtocolHandler, Router}},
    iroh_docs::{AuthorId, NamespaceId, protocol::Docs},
    iroh_blobs::net_protocol::Blobs,
    iroh_gossip::{proto::TopicId, net::{Gossip, GossipTopic}},
    base64::engine::general_purpose::STANDARD as B64,
    base64::Engine,
    tokio::sync::mpsc as tokio_mpsc,
    flume,
    futures_lite::{future::Boxed as BoxedFuture, StreamExt},
    std::sync::Arc,
    bytes::Bytes,
    iroh::PublicKey,
};

#[cfg(not(feature = "iroh"))]
// Import used with iroh feature

#[cfg(not(feature = "iroh"))]
// Stub implementation only needs minimal stubs defined below
pub struct EndpointStub;

/// Hard-coded ALPN for p2pgo protocol
#[allow(dead_code)]
const P2PGO_ALPN: &[u8] = b"p2pgo";

/// P2P Go protocol handler for iroh v0.35
#[cfg(feature = "iroh")]
#[derive(Debug, Clone)]
pub struct P2PGoProtocol {
    // Channel to send incoming connections to the application layer
    connection_tx: Arc<tokio_mpsc::UnboundedSender<Connection>>,
}

#[cfg(feature = "iroh")]
impl P2PGoProtocol {
    pub fn new(connection_tx: tokio_mpsc::UnboundedSender<Connection>) -> Self {
        Self {
            connection_tx: Arc::new(connection_tx),
        }
    }
}

#[cfg(feature = "iroh")]
impl ProtocolHandler for P2PGoProtocol {
    fn accept(&self, connection: Connection) -> BoxedFuture<anyhow::Result<()>> {
        let connection_tx = self.connection_tx.clone();
        Box::pin(async move {
            match connection.remote_node_id() {
                Ok(remote_id) => {
                    tracing::info!("P2P Go connection accepted from: {}", remote_id);
                }
                Err(e) => {
                    tracing::warn!("Could not get remote node ID: {}", e);
                }
            }
            
            // Send the connection to the application layer for handling
            if let Err(e) = connection_tx.send(connection) {
                tracing::error!("Failed to send connection to application layer: {}", e);
                return Err(anyhow::anyhow!("Failed to route connection: {}", e));
            }
            
            Ok(())
        })
    }
}

/// CBOR-encoded, base64-text ticket (forward compatible)
#[cfg(feature = "iroh")]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EnhancedTicket {
    /// Full network address incl. NodeId & multiaddrs
    pub node: NodeAddr,
    /// Optional document to sync immediately (e.g. game doc)
    pub doc: Option<NamespaceId>,
    /// Optional capability string for private docs
    pub cap: Option<String>,
    /// Optional board size hint (9/13/19)
    pub game_size: Option<u8>,
    /// Protocol version – bump safely later
    pub version: u8,
}

/// Game advertisement structure for gossip
#[cfg(feature = "iroh")]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct GameAdvert {
    pub gid: String,
    pub size: u8,
    pub host: String,
    pub bot: bool,
}

/// Iroh networking context
#[cfg(feature = "iroh")]
#[derive(Clone)]
pub struct IrohCtx {
    endpoint: Endpoint,
    router: Router,
    // Eagerly initialized networking components
    gossip: Arc<Gossip>,
    docs: Arc<Docs>,
    blobs: Arc<Blobs>,
    #[allow(dead_code)]
    default_author: AuthorId,
    my_id: String,
    // Channel for receiving incoming connections
    connection_rx: Arc<tokio::sync::Mutex<tokio_mpsc::UnboundedReceiver<Connection>>>,
    // Relay monitoring
    relay_stats: Arc<tokio::sync::RwLock<std::collections::HashMap<String, crate::relay_monitor::RelayStats>>>,
    // Embedded relay service (if enabled)
    relay_service: Option<Arc<tokio::sync::Mutex<crate::relay_monitor::RestartableRelay>>>,
    // Mode
    is_relay_mode: bool,
}

#[cfg(not(feature = "iroh"))]
pub struct IrohCtx {
    _ep: EndpointStub,
    #[allow(dead_code)]
    my_id: String,
}

impl IrohCtx {
    /// Create a new Iroh networking context
    #[tracing::instrument(level = "debug")]
    pub async fn new() -> Result<Self> {
        #[cfg(feature = "iroh")]
        {
            tracing::debug!("Creating Iroh endpoint with relay support");
            
            // Load network configuration
            let config = load_config().unwrap_or_else(|e| {
                tracing::warn!("Failed to load network config, using defaults: {}", e);
                crate::config::NetworkConfig::default()
            });
            
            tracing::info!("Network config loaded: relay_mode={:?}, {} relay addresses", 
                config.relay_mode, config.relay_addrs.len());
            
            // If using self relay mode, defer to new_with_port
            if config.relay_mode == RelayModeConfig::SelfRelay {
                return Self::new_with_port(config, None).await;
            }
            
            // Standard client mode setup
            // Create iroh endpoint with configured relay support
            let mut endpoint_builder = Endpoint::builder();
            
            // Configure relay mode based on config
            match config.relay_mode {
                RelayModeConfig::Default => {
                    tracing::info!("Using default Iroh relays");
                    endpoint_builder = endpoint_builder.relay_mode(iroh::RelayMode::Default);
                },
                RelayModeConfig::Custom => {
                    if config.relay_addrs.is_empty() {
                        tracing::warn!("Custom relay mode specified but no relays configured, falling back to default");
                        endpoint_builder = endpoint_builder.relay_mode(iroh::RelayMode::Default);
                    } else {
                        tracing::info!("Using custom relays: {:?}", config.relay_addrs);
                        let mut relay_opts = iroh::relay::RelayOptions::new();
                        
                        // Add configured relays
                        for addr_str in &config.relay_addrs {
                            match addr_str.parse() {
                                Ok(addr) => {
                                    relay_opts = relay_opts.add_bootstrap(addr);
                                    tracing::info!("Added relay: {}", addr_str);
                            },
                            Err(e) => tracing::error!("Invalid relay address {}: {}", addr_str, e),
                        }
                    }
                    
                    endpoint_builder = endpoint_builder.relay_mode(iroh::RelayMode::Custom(relay_opts));
                }
                },
                RelayModeConfig::SelfRelay => {
                    // This should never be reached as we handle this above
                    unreachable!("Self relay mode should be handled by new_with_port");
                },
            }
            
            let endpoint = endpoint_builder
                .bind()
                .await
                .context("Failed to bind iroh endpoint")?;
            
            let node_addr = endpoint.node_addr().await?;
            tracing::info!("Iroh endpoint bound successfully, node ID: {}, addr: {:?}", 
                endpoint.node_id(), node_addr);
            
            // Log external addresses for debugging
            let public_addrs = endpoint.direct_addresses().get();
            tracing::info!("🌐 PUBLIC addrs: {:?}", public_addrs);
            
            // Ensure we have external addresses for internet connectivity
            if node_addr.direct_addresses.is_empty() {
                tracing::warn!("NodeAddr missing external addresses - relay may not be ready yet");
            }
            
            // Create gossip instance
            let gossip = iroh_gossip::net::Gossip::builder()
                .spawn(endpoint.clone())
                .await
                .context("Failed to create gossip instance")?;
            let gossip = Arc::new(gossip);
            
            // Create docs instance for document sync
            let docs = Docs::builder()
                .mem()
                .spawn(endpoint.clone())
                .await
                .context("Failed to create docs instance")?;
            let docs = Arc::new(docs);
            
            // Create blobs instance for blob storage
            let blobs = Blobs::memory()
                .spawn(endpoint.clone())
                .await
                .context("Failed to create blobs instance")?;
            let blobs = Arc::new(blobs);
            
            // Create a channel for incoming connections
            let (connection_tx, connection_rx) = tokio::sync::mpsc::unbounded_channel();
            
            // Create the P2P Go protocol handler
            let p2pgo_protocol = P2PGoProtocol::new(connection_tx);
            
            // Set up the router with all protocols
            let router = Router::builder(endpoint.clone())
                .accept(iroh_gossip::ALPN, gossip.clone())
                .accept(iroh_docs::ALPN, docs.clone())
                .accept(iroh_blobs::ALPN, blobs.clone())
                .accept(P2PGO_ALPN, p2pgo_protocol)
                .spawn();
            
            let my_id = endpoint.node_id().to_string();
            
            // Create a default author ID for signing documents
            let default_author = AuthorId::from([0; 32]);
            
            // Initialize relay monitoring
            let relay_monitor = RelayMonitor::new(endpoint.clone(), config.relay_addrs.clone());
            let relay_stats = relay_monitor.start_monitoring();
            
            tracing::info!("Iroh networking context initialized successfully");
            
            Ok(Self {
                endpoint,
                router,
                gossip,
                docs,
                blobs,
                default_author,
                my_id,
                connection_rx: Arc::new(tokio::sync::Mutex::new(connection_rx)),
                relay_stats,
                relay_service: None,
                is_relay_mode: false,
            })
        }
        
        // Fallback to TCP loopback stub
        #[cfg(not(feature = "iroh"))]
        {
            tracing::info!("Using TCP loopback stub for networking");
            
            return Ok(Self {
                _ep: EndpointStub,
                my_id: "loopback-node".to_string(),
            });
        }
    }
    
    /// Create a new IrohCtx with a specific port for relay service
    #[cfg(feature = "iroh")]
    pub async fn new_with_port(config: crate::config::NetworkConfig, port: Option<u16>) -> Result<Self> {
        // Create port manager
        let port_manager = crate::port::PortManager::new()?;
        
        // Create health event channel for relay monitoring
        let (health_tx, mut health_rx) = tokio_mpsc::unbounded_channel();
        
        // Start the embedded relay if needed
        match config.relay_mode {
            RelayModeConfig::SelfRelay => {
                tracing::info!("Creating endpoint with embedded relay");
                
                // Create RestartableRelay service with health sender
                let mut relay_service = crate::relay_monitor::RestartableRelay::new(port_manager.clone())
                    .with_health_sender(health_tx.clone());
                
                // Start the embedded relay, which will pick TCP/UDP ports
                let (tcp_port, udp_port) = match relay_service.start_embedded_relay().await {
                    Ok(ports) => {
                        tracing::info!("Embedded relay started on TCP:{} UDP:{}", ports.0, ports.1);
                        ports
                    },
                    Err(e) => {
                        tracing::error!("Failed to start embedded relay: {}", e);
                        return Err(e);
                    }
                };
                
                // Use the relay port for our endpoint
                let listen_addr = format!("/ip4/0.0.0.0/tcp/{}/quic-v1", tcp_port);
                tracing::info!("Starting endpoint with quic listen address: {}", listen_addr);
                
                // Start endpoint with relay mode
                let endpoint_builder = Endpoint::builder()
                    .relay_mode(iroh::RelayMode::Relay)
                    .listen_on(Some(listen_addr.parse()?));
                
                // Create connection channel
                let (connection_tx, connection_rx) = tokio_mpsc::unbounded_channel();
                
                // Create P2P Go protocol handler
                let p2pgo_protocol = P2PGoProtocol::new(connection_tx);
                
                // Create router with protocol handlers
                let router = Router::new();
                
                let (endpoint, router) = endpoint_builder
                    .spawn_with_router(router)
                    .await?;
                
                // Initialize Gossip service
                let gossip_config = iroh_gossip::net::Config::new(endpoint.node_id());
                let gossip = Arc::new(Gossip::new(endpoint.clone(), gossip_config).await?);
                
                // Initialize Docs service for document synchronization
                let docs = Arc::new(Docs::new(endpoint.clone()));
                
                // Initialize Blobs service for large binary data
                let blobs = Arc::new(Blobs::new(endpoint.clone()));
                
                // Register protocol handlers
                let router = router
                    .accept(iroh_gossip::net::ALPN, gossip.clone())
                    .accept(iroh_docs::protocol::ALPN, docs.clone())
                    .accept(iroh_blobs::ALPN, blobs.clone())
                    .accept(P2PGO_ALPN, p2pgo_protocol)
                    .spawn();
                
                let my_id = endpoint.node_id().to_string();
                
                // Create a default author ID for signing documents
                let default_author = AuthorId::from([0; 32]);
                
                // Initialize relay monitoring for external relays
                let relay_monitor = RelayMonitor::new(endpoint.clone(), config.relay_addrs.clone());
                let relay_stats = relay_monitor.start_monitoring();
                
                // Spawn a task to forward health events to the UI layer if a sender is configured
                if let Some(ui_sender) = config.ui_sender {
                    let ui_sender = ui_sender.clone();
                    
                    let _health_forward = spawn_cancelable!(
                        name: "relay_health_forwarder",
                        max_restarts: 3,
                        restart_delay_ms: 1000,
                        window_secs: 60,
                        |shutdown| async move {
                            use p2pgo_ui_egui::msg::NetToUi;
                            use std::time::SystemTime;
                            
                            while !shutdown.is_cancelled() {
                                match tokio::time::timeout(Duration::from_secs(1), health_rx.recv()).await {
                                    Ok(Some(event)) => {
                                        // Convert to SystemTime for UI compatibility
                                        let last_restart = event.last_restart.map(|instant| {
                                            let now = Instant::now();
                                            let elapsed = if now >= instant {
                                                now - instant
                                            } else {
                                                Duration::from_secs(0)
                                            };
                                            SystemTime::now().checked_sub(elapsed).unwrap_or_else(SystemTime::now)
                                        });
                                        
                                        // Forward to UI
                                        let ui_msg = NetToUi::RelayHealth {
                                            status: event.status,
                                            port: event.port,
                                            is_relay_node: event.is_self_relay,
                                            last_restart,
                                        };
                                        
                                        if let Err(e) = ui_sender.send(ui_msg) {
                                            tracing::warn!("Failed to forward relay health event: {}", e);
                                        }
                                    },
                                    Ok(None) => {
                                        // Channel closed
                                        break;
                                    },
                                    Err(_) => {
                                        // Timeout - continue
                                    }
                                }
                            }
                            
                            Ok(())
                        }
                    );
                }
                
                tracing::info!("Iroh networking context initialized successfully with embedded relay");
                
                Ok(Self {
                    endpoint,
                    router,
                    gossip,
                    docs,
                    blobs,
                    default_author,
                    my_id,
                    connection_rx: Arc::new(tokio::sync::Mutex::new(connection_rx)),
                    relay_stats,
                    relay_service: Some(Arc::new(tokio::sync::Mutex::new(relay_service))),
                    is_relay_mode: true,
                })
            },
            _ => {
                // Use the existing non-relay endpoint creation code
                Self::create_client_endpoint(config, port_manager).await
            }
        }
    }
    
    /// Restart the embedded relay service
    #[cfg(feature = "iroh")]
    pub async fn restart_relay(&self) -> Result<()> {
        if let Some(relay_service) = &self.relay_service {
            tracing::info!("Relay restart requested");
            
            let mut service = relay_service.lock().await;
            let endpoint_clone = self.endpoint.clone();
            
            // Restart the relay service with a new closure to initialize the relay
            service.restart(move |port, shutdown_rx| {
                let ep = endpoint_clone.clone();
                
                async move {
                    tracing::info!("Starting embedded relay on port {}", port);
                    
                    // We already have an endpoint, so we don't need to do anything special here
                    // Just wait for the shutdown signal
                    match shutdown_rx.await {
                        Ok(_) => {
                            tracing::info!("Relay shutdown signal received");
                            Ok(())
                        },
                        Err(e) => {
                            tracing::error!("Relay shutdown channel error: {}", e);
                            Err(anyhow::anyhow!("Relay shutdown channel error: {}", e))
                        }
                    }
                }
            }).await?;
            
            // Broadcast the restart event to connected peers
            self.broadcast_restart_event().await?;
            
            Ok(())
        } else {
            bail!("No embedded relay service to restart")
        }
    }
    
    /// Check if running in relay mode
    pub fn is_relay_mode(&self) -> bool {
        #[cfg(feature = "iroh")]
        return self.is_relay_mode;
        
        #[cfg(not(feature = "iroh"))]
        false
    }
    
    /// Get the current relay status and port
    #[cfg(feature = "iroh")]
    pub async fn get_relay_status(&self) -> Result<Option<(crate::relay_monitor::RelayHealthStatus, Option<u16>)>> {
        if let Some(relay_service) = &self.relay_service {
            let service = relay_service.lock().await;
            let state = service.state();
            let state = state.read().await;
            
            Ok(Some((state.status.clone(), state.listening_port)))
        } else {
            Ok(None)
        }
    }
    
    /// Broadcast a restart event to connected peers
    #[allow(dead_code)]
    async fn broadcast_restart_event(&self) -> Result<()> {
        // TODO: Implement real broadcast mechanism
        // For now, just log it
        tracing::info!("Broadcasting relay restart event");
        Ok(())
    }
    
    /// Test connectivity to the relay
    pub async fn test_relay_connectivity(&self) -> Result<RelayConnectivityResult> {
        // TODO: Implement real relay connectivity test
        // For now, just return a dummy result
        Ok(RelayConnectivityResult {
            is_online: true,
            latency_ms: Some(50),
            relay_addr: "test-relay".into(),
        })
    }
    
    /// Connect to a peer using a ticket
    pub async fn connect_by_ticket(&self, ticket: &str) -> Result<()> {
        // TODO: Implement real ticket connection
        tracing::info!("Connecting via ticket: {}", ticket);
        Ok(())
    }
    
    /// Get our connection ticket
    pub async fn ticket(&self) -> Result<String> {
        // TODO: Implement real ticket generation
        Ok("dummy-ticket-123".to_string())
    }
    
    /// Advertise a game for others to find
    pub async fn advertise_game(&self, game_id: &str, board_size: u8) -> Result<()> {
        // TODO: Implement real game advertisement
        tracing::info!("Advertising game {} with board size {}", game_id, board_size);
        Ok(())
    }
    
    /// Get the node ID
    pub fn node_id(&self) -> String {
        // TODO: Implement real node ID retrieval
        "dummy-node-id".to_string()
    }
    
    /// Store a game move
    pub async fn store_game_move(&self, game_id: &str, sequence: u64, move_record: &p2pgo_core::MoveRecord) -> Result<()> {
        // TODO: Implement real move storage
        tracing::info!("Storing move for game {} seq {}: {:?}", game_id, sequence, move_record);
        Ok(())
    }
    
    /// Store a move tag
    pub async fn store_move_tag(&self, game_id: &str, sequence: u64, tag: &str) -> Result<()> {
        // TODO: Implement real move tag storage
        tracing::info!("Storing tag for game {} seq {}: {}", game_id, sequence, tag);
        Ok(())
    }
}

#[cfg(feature = "iroh")]
impl IrohCtx {
    /// Get a reference to the endpoints node_id for signing/verification
    pub fn get_node_id_bytes(&self) -> Result<[u8; 32]> {
        let node_id = self.endpoint.node_id();
        let mut bytes = [0u8; 32];
        
        // Convert the PublicKey from node_id to raw bytes
        let node_id_bytes = node_id.as_bytes();
        
        // Ensure we have the expected format
        if node_id_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Unexpected node_id size: {}", node_id_bytes.len()));
        }
        
        bytes.copy_from_slice(node_id_bytes);
        Ok(bytes)
    }
    
    /// Sign data with this node's identity key
    pub async fn sign_data(&self, data: &[u8]) -> Result<([u8; 64], String)> {
        #[cfg(feature = "iroh")]
        {
            use ed25519_dalek::{SigningKey, VerifyingKey};
            
            // In real iroh, we'd get access to the keypair from the endpoint
            // For now we'll generate a deterministic keypair from the node_id
            let node_id_bytes = self.get_node_id_bytes()?;
            
            // Use the node_id as seed for a deterministic keypair until we get proper access
            // Note: In a production system, we would want to get actual access to the keypair
            let mut hasher = blake3::Hasher::new();
            hasher.update(&node_id_bytes);
            let seed = hasher.finalize();
            
            // Create a keypair using the seed
            let signing_key = SigningKey::from_bytes(&seed.into());
            
            // Get the verifying key (public key) as a hex string
            let verifying_key = signing_key.verifying_key();
            let public_key_hex = hex::encode(verifying_key.as_bytes());
            
            // Sign the data
            let signature = signing_key.sign(data);
            
            // Return the signature bytes and public key
            Ok((signature.to_bytes(), public_key_hex))
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            // Return a dummy signature for stub mode
            let signature = [0u8; 64];
            Ok((signature, "stub_node_id".to_string()))
        }
    }
    
    /// Verify a signature against a node_id
    pub fn verify_signature(&self, data: &[u8], signature: &[u8; 64], signer_hex: &str) -> Result<bool> {
        use ed25519_dalek::{Signature, VerifyingKey};
        
        // Decode the signer from hex
        let signer_bytes = hex::decode(signer_hex)?;
        if signer_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid signer key length"));
        }
        
        // Create a verifying key
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&signer_bytes);
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)?;
        
        // Create a signature
        let signature = Signature::from_bytes(signature)?;
        
        // Verify the signature
        match verifying_key.verify(data, &signature) {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("Signature verification failed: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Get an ed25519 keypair for signing
    pub async fn get_ed25519_keypair(&self) -> Result<ed25519_dalek::SigningKey> {
        use ed25519_dalek::SigningKey;
        
        // Get the node ID bytes
        let node_id_bytes = self.get_node_id_bytes()?;
        
        // Use the node_id as seed for a deterministic keypair until we get proper access
        // Note: In a production system, we would want to get actual access to the keypair
        let mut hasher = blake3::Hasher::new();
        hasher.update(&node_id_bytes);
        let seed = hasher.finalize();
        
        // Create a keypair using the seed
        let signing_key = SigningKey::from_bytes(&seed.into());
        Ok(signing_key)
    }
}

/// Result of relay connectivity test
#[derive(Debug)]
pub struct RelayConnectivityResult {
    /// Is the relay online
    pub is_online: bool,
    /// Latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Relay address that was tested
    pub relay_addr: String,
}

/// Create a client endpoint (non-relay mode)
#[cfg(feature = "iroh")]
async fn create_client_endpoint(
    config: crate::config::NetworkConfig,
    port_manager: crate::port::PortManager,
) -> Result<IrohCtx> {
    tracing::info!("Creating client endpoint (relay mode: {:?})", config.relay_mode);
    
    // Client mode - use relay_addrs from config
    let endpoint_builder = Endpoint::builder();
    
    // Configure relay mode from config
    let endpoint_builder = match config.relay_mode {
        RelayModeConfig::AutoRelay => endpoint_builder.relay_mode(iroh::RelayMode::AutoRelay),
        RelayModeConfig::CustomRelays => endpoint_builder.relay_mode(iroh::RelayMode::CustomRelays),
        RelayModeConfig::NoRelay => endpoint_builder.relay_mode(iroh::RelayMode::None),
        _ => endpoint_builder.relay_mode(iroh::RelayMode::AutoRelay),
    };
    
    // Add relay addresses if provided
    let endpoint_builder = if !config.relay_addrs.is_empty() {
        let mut builder = endpoint_builder;
        for addr in &config.relay_addrs {
            if let Ok(addr) = addr.parse() {
                builder = builder.add_relay_addr(addr);
            } else {
                tracing::warn!("Invalid relay address: {}", addr);
            }
        }
        builder
    } else {
        endpoint_builder
    };
    
    // Create connection channel
    let (connection_tx, connection_rx) = tokio_mpsc::unbounded_channel();
    
    // Create P2P Go protocol handler
    let p2pgo_protocol = P2PGoProtocol::new(connection_tx);
    
    // Create router with protocol handlers
    let router = Router::new();
    
    let (endpoint, router) = endpoint_builder
        .spawn_with_router(router)
        .await?;
    
    // Initialize Gossip service
    let gossip_config = iroh_gossip::net::Config::new(endpoint.node_id());
    let gossip = Arc::new(Gossip::new(endpoint.clone(), gossip_config).await?);
    
    // Initialize Docs service for document synchronization
    let docs = Arc::new(Docs::new(endpoint.clone()));
    
    // Initialize Blobs service for large binary data
    let blobs = Arc::new(Blobs::new(endpoint.clone()));
    
    // Register protocol handlers
    let router = router
        .accept(iroh_gossip::net::ALPN, gossip.clone())
        .accept(iroh_docs::protocol::ALPN, docs.clone())
        .accept(iroh_blobs::ALPN, blobs.clone())
        .accept(P2PGO_ALPN, p2pgo_protocol)
        .spawn();
    
    let my_id = endpoint.node_id().to_string();
    
    // Create a default author ID for signing documents
    let default_author = AuthorId::from([0; 32]);
    
    // Initialize relay monitoring
    let relay_monitor = RelayMonitor::new(endpoint.clone(), config.relay_addrs.clone());
    let relay_stats = relay_monitor.start_monitoring();
    
    tracing::info!("Iroh networking context initialized successfully in client mode");
    
    Ok(IrohCtx {
        endpoint,
        router,
        gossip,
        docs,
        blobs,
        default_author,
        my_id,
        connection_rx: Arc::new(tokio::sync::Mutex::new(connection_rx)),
        relay_stats,
        relay_service: None, // No embedded relay in client mode
        is_relay_mode: false,
    })
}

/// Generate a topic ID from a game ID
#[cfg(feature = "iroh")]
pub fn game_topic(&self, game_id: &str) -> Result<TopicId> {
    // Hash the game ID to get a consistent 32-byte value
    let hash = blake3::hash(game_id.as_bytes());
    let bytes = hash.as_bytes();
    
    // Create a TopicId from the hash
    let topic = TopicId::from_bytes(*bytes)?;
    Ok(topic)
}

/// Generate a topic ID from a game ID (stub implementation)
#[derive(Debug)]
pub struct IrohEndpoint;

impl IrohEndpoint {
    pub fn new() -> Self {
        Self
    }
    
    #[cfg(not(feature = "iroh"))]
    pub fn game_topic(&self, _game_id: &str) -> Result<()> {
        Ok(())
    }
}

/// Broadcast a move via gossip
#[cfg(feature = "iroh")]
pub async fn broadcast_move(&self, game_id: &str, move_record: &mut p2pgo_core::MoveRecord) -> Result<()> {
    tracing::debug!("Broadcasting move via gossip: {:?}", move_record.mv);
    
    // Sign the move if it doesn't already have a signature
    if move_record.signature.is_none() || move_record.signer.is_none() {
        // Get bytes to sign
        let data = move_record.to_bytes();
        
        // Sign the data
        match self.sign_data(&data).await {
            Ok((signature, signer)) => {
                move_record.signature = Some(signature);
                move_record.signer = Some(signer);
                tracing::debug!("Move signed successfully by {}", signer);
            },
            Err(e) => {
                tracing::warn!("Failed to sign move: {}", e);
                // Continue without signature
            }
        }
    }
    
    // CBOR encode the signed move record
    let cbor_data = serde_cbor::to_vec(move_record)
        .context("Failed to CBOR encode move record")?;
        
    // Generate topic ID from game_id
    let topic = self.game_topic(game_id)?;
    
    #[cfg(feature = "iroh")]
    {
        // Broadcast via gossip
        if let Some(gossip) = &self.gossip {
            tracing::debug!("Sending gossip message for game {}", game_id);
            gossip.publish(topic, Bytes::from(cbor_data)).await?;
            tracing::debug!("Gossip message sent successfully");
            return Ok(());
        }
    }
    
    Err(anyhow::anyhow!("Gossip not enabled"))
}

/// Broadcast a move via gossip (stub implementation)
impl IrohEndpoint {
    #[cfg(not(feature = "iroh"))]
    pub async fn broadcast_move(&self, game_id: &str, move_record: &mut p2pgo_core::MoveRecord) -> Result<()> {
        tracing::debug!("Mock broadcast move for game {}: {:?}", game_id, move_record.mv);
        Ok(())
    }
}

/// Broadcast arbitrary data to a game topic
#[cfg(feature = "iroh")]
pub async fn broadcast_to_game_topic(&self, game_id: &str, data: &[u8]) -> Result<()> {
    tracing::debug!("Broadcasting data to game topic: {}", game_id);
    
    // Generate topic ID from game_id
    let topic = self.game_topic(game_id)?;
    
    #[cfg(feature = "iroh")]
    {
        // Broadcast via gossip
        if let Some(gossip) = &self.gossip {
            tracing::debug!("Sending gossip data for game {}", game_id);
            gossip.publish(topic, Bytes::from(data.to_vec())).await?;
            tracing::debug!("Gossip data sent successfully");
            return Ok(());
        }
    }
    
    Err(anyhow::anyhow!("Gossip not enabled"))
}

/// Broadcast arbitrary data to a game topic (stub implementation)
impl IrohEndpoint {
    #[cfg(not(feature = "iroh"))]
    pub async fn broadcast_to_game_topic(&self, game_id: &str, _data: &[u8]) -> Result<()> {
        tracing::debug!("Mock broadcast data to game topic: {}", game_id);
        Ok(())
    }
}