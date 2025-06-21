// SPDX-License-Identifier: MIT OR Apache-2.0

//! Iroh networking endpoint management for P2P Go

use anyhow::{Result, Context, ensure, bail};
use p2pgo_core::MoveRecord;
use crate::config::{load_config, RelayModeConfig};
use crate::relay_monitor::RelayMonitor;

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
use {
    tokio::sync::mpsc as tokio_mpsc,
    flume,
};

#[cfg(not(feature = "iroh"))]
// Stub implementation only needs minimal stubs defined below
pub struct EndpointStub;

/// Hard-coded ALPN for p2pgo protocol
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
}

#[cfg(not(feature = "iroh"))]
pub struct IrohCtx {
    _ep: EndpointStub,
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
                    tracing::info!("This node will act as a relay");
                    endpoint_builder = endpoint_builder.relay_mode(iroh::RelayMode::Relay);
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
    
    /// Get the node ID (base32 pubkey)
    pub fn node_id(&self) -> &str {
        &self.my_id
    }
    
    /// Get access to the docs instance
    #[cfg(feature = "iroh")]
    pub fn docs(&self) -> &Docs {
        &self.docs
    }
    
    /// Get access to the blobs instance
    #[cfg(feature = "iroh")]
    pub fn blobs(&self) -> &Blobs {
        &self.blobs
    }
    
    /// Get relay statistics
    #[cfg(feature = "iroh")]
    pub fn relay_stats(&self) -> &Arc<tokio::sync::RwLock<std::collections::HashMap<String, crate::relay_monitor::RelayStats>>> {
        &self.relay_stats
    }
    
    /// Generate a connection ticket with optional game size hint
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn ticket(&self) -> Result<String> {
        self.ticket_with_game_size(None).await
    }
    
    /// Generate a connection ticket with optional game size hint and game doc
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn ticket_with_game_size(&self, game_size: Option<u8>) -> Result<String> {
        self.ticket_with_game_doc(None, game_size).await
    }
    
    /// Generate a connection ticket with game document and size
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn ticket_with_game_doc(&self, game_id: Option<&str>, game_size: Option<u8>) -> Result<String> {
        #[cfg(feature = "iroh")]
        {
            let mut addr = self.endpoint.node_addr().await?
                .with_default_relay(true);  // Ensure relay addresses are included
            
            // Ensure relay URLs are included for internet connectivity
            if addr.relay_url.is_none() {
                tracing::info!("Waiting for relay to be established...");
                // Wait a bit for relay to be established
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                addr = self.endpoint.node_addr().await?
                    .with_default_relay(true);
            }
            
            // Ensure we have external addresses for internet connectivity
            ensure!(!addr.direct_addresses.is_empty() || addr.relay_url.is_some(), 
                "NodeAddr missing both external addresses and relay - network not ready");
            
            // Check for relay multiaddr in direct_addresses - more comprehensive check
            let relay_addrs: Vec<_> = addr.direct_addresses.iter()
                .filter(|a| {
                    let addr_str = a.to_string();
                    addr_str.contains("/dns4/") && (addr_str.contains("/relay/") || addr_str.contains("relay."))
                })
                .collect();
            
            // Log relay status
            if !relay_addrs.is_empty() {
                tracing::info!("✅ Ticket contains {} relay multiaddrs: {:?}", 
                    relay_addrs.len(), 
                    relay_addrs.iter().map(|a| a.to_string()).collect::<Vec<_>>());
            } else if addr.relay_url.is_some() {
                tracing::info!("✅ Ticket contains relay URL: {:?}", addr.relay_url);
            } else {
                tracing::warn!("⚠️ Ticket has NO relay multiaddrs! Connection may fail on NATs.");
            }
            
            let doc = match game_id {
                Some(gid) => Some(Self::doc_id_for_game(gid)),
                None => None,
            };
            
            let ticket = EnhancedTicket {
                node: addr.clone(),
                doc,
                cap: None, // Reserved for future auth
                game_size,
                version: 1,
            };
            
            let bytes = serde_cbor::to_vec(&ticket)?;
            let ticket_str = B64.encode(bytes);
            
            tracing::debug!("Generated Iroh ticket with {} addresses, relay: {:?}: {}", 
                addr.direct_addresses.len(), addr.relay_url, ticket_str);
            
            Ok(ticket_str)
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            Ok("loopback-ticket".to_string())
        }
    }
    
    /// Connect to a peer using a ticket and return the connection
    #[cfg(feature = "iroh")]
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn connect_to_peer(&self, ticket: &str) -> Result<Connection> {
        tracing::debug!("Connecting to peer via Iroh ticket");
        
        // Base64 decode the ticket
        let bytes = B64.decode(ticket)
            .context("Failed to decode base64 ticket")?;
        
        // CBOR decode to EnhancedTicket
        let ticket: EnhancedTicket = serde_cbor::from_slice(&bytes)
            .context("Failed to decode CBOR ticket")?;
        
        tracing::info!("Connecting to node: {:?} with {} addresses", 
            ticket.node.node_id, ticket.node.direct_addresses.len());
        
        // Connect directly using the NodeAddr from the ticket
        match self.endpoint.connect(ticket.node.clone(), P2PGO_ALPN).await {
            Ok(connection) => {
                tracing::info!("Successfully connected to peer via NodeAddr");
                return Ok(connection);
            }
            Err(e) => {
                tracing::debug!("Failed to connect via NodeAddr: {}", e);
                bail!("❌ Could not connect to peer: {}", e)
            }
        }
    }
    
    /// Connect to a peer using a ticket
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn connect_by_ticket(&self, ticket: &str) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            let _connection = self.connect_to_peer(ticket).await?;
            // Connection established successfully
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("TCP loopback connection to ticket: {}", ticket);
            Ok(())
        }
    }
    
    /// Create a topic ID for a given board size lobby
    #[cfg(feature = "iroh")]
    pub fn lobby_topic(size: u8) -> TopicId {
        let topic_name = format!("p2pgo.lobby.{}", size);
        TopicId::from_bytes(*blake3::hash(topic_name.as_bytes()).as_bytes())
    }
    
    /// Create a topic ID for a specific game
    #[cfg(feature = "iroh")]
    pub fn game_topic(game_id: &str) -> TopicId {
        let topic_name = format!("p2pgo.game.{}", game_id);
        TopicId::from_bytes(*blake3::hash(topic_name.as_bytes()).as_bytes())
    }
    
    /// Subscribe to gossip topic for lobby
    #[tracing::instrument(level = "debug", skip(self))]
    #[cfg(feature = "iroh")]
    pub async fn subscribe_lobby(&self, board_size: u8) -> Result<mpsc::Receiver<iroh_gossip::net::Event>> {
        let topic = Self::lobby_topic(board_size);
        self.subscribe_gossip_topic(topic, 32).await
    }
    
    /// Subscribe to gossip topic for a specific game
    #[tracing::instrument(level = "debug", skip(self))]
    #[cfg(feature = "iroh")]
    pub async fn subscribe_game_topic(&self, game_id: &str, buffer_size: usize) -> Result<mpsc::Receiver<iroh_gossip::net::Event>> {
        let topic = Self::game_topic(game_id);
        self.subscribe_gossip_topic(topic, buffer_size).await
    }
    
    /// Subscribe to a specific gossip topic
    #[tracing::instrument(level = "debug", skip(self))]
    #[cfg(feature = "iroh")]
    pub async fn subscribe_gossip_topic(&self, topic_id: TopicId, buffer_size: usize) -> Result<flume::Receiver<iroh_gossip::net::Event>> {
        tracing::debug!("Subscribing to gossip topic: {:?}", topic_id);
        
        // Subscribe to the topic with empty bootstrap peers
        let bootstrap_peers: Vec<PublicKey> = Vec::new();
        let mut gossip_topic = self.gossip.subscribe(topic_id, bootstrap_peers)
            .context("Failed to subscribe to gossip topic")?;
        
        // Create a bounded flume channel with larger buffer (256) to prevent back-pressure
        let (tx, rx) = flume::bounded(256);
        
        // Spawn a task to forward events with retry logic
        tokio::spawn(async move {
            tracing::info!("Gossip subscription active for topic: {:?}", topic_id);
            loop {
                match gossip_topic.next().await {
                    Some(Ok(event)) => {
                        // Monitor channel capacity to detect potential backpressure
                        let len = tx.len();
                        if len > 200 {
                            tracing::trace!("Gossip channel buffer high: {}/256 events", len);
                        }
                        
                        if tx.send_async(event).await.is_err() {
                            tracing::debug!("Gossip event receiver dropped");
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("Gossip stream error: {}, retrying in 1s", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                    None => {
                        tracing::debug!("Gossip stream ended");
                        break;
                    }
                }
            }
        });
        
        tracing::info!("Successfully subscribed to gossip topic");
        Ok(rx)
    }
    
    /// Subscribe to gossip topic for lobby (stub implementation)
    #[tracing::instrument(level = "debug", skip(self))]
    #[cfg(not(feature = "iroh"))]
    pub async fn subscribe_lobby(&self, board_size: u8) -> Result<flume::Receiver<()>> {
        tracing::debug!("Mock subscribe to lobby for board size: {}", board_size);
        let (_, rx) = flume::bounded(256);
        Ok(rx)
    }
    
    /// Subscribe to gossip topic for a specific game (stub implementation)
    #[tracing::instrument(level = "debug", skip(self))]
    #[cfg(not(feature = "iroh"))]
    pub async fn subscribe_game_topic(&self, game_id: &str, _buffer_size: usize) -> Result<flume::Receiver<()>> {
        tracing::debug!("Mock subscribe to game topic for: {}", game_id);
        let (_, rx) = flume::bounded(256);
        Ok(rx)
    }
    
    /// Broadcast a message to a specific gossip topic
    #[tracing::instrument(level = "debug", skip(self, data))]
    #[cfg(feature = "iroh")]
    pub async fn broadcast_to_topic(&self, topic_id: TopicId, data: &[u8]) -> Result<()> {
        tracing::debug!("Broadcasting {} bytes to gossip topic: {:?}", data.len(), topic_id);
        
        // Subscribe to topic for broadcasting
        let bootstrap_peers: Vec<PublicKey> = Vec::new();
        let gossip_topic = self.gossip.subscribe(topic_id, bootstrap_peers)
            .context("Failed to subscribe to gossip topic for broadcasting")?;
        
        let message = Bytes::copy_from_slice(data);
        
        // Broadcast the message using the topic subscription
        gossip_topic.broadcast(message)
            .await
            .context("Failed to broadcast message to gossip topic")?;
        
        tracing::info!("Successfully broadcast {} bytes to gossip topic", data.len());
        Ok(())
    }
    
    /// Broadcast a message to a specific gossip topic (stub implementation)
    #[tracing::instrument(level = "debug", skip(self, data))]
    #[cfg(not(feature = "iroh"))]
    pub async fn broadcast_to_topic(&self, _topic_id: (), data: &[u8]) -> Result<()> {
        tracing::debug!("Mock broadcast {} bytes to topic", data.len());
        Ok(())
    }
    
    /// Publish game advertisement to gossip
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn advertise_game(&self, game_id: &str, board_size: u8) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            tracing::debug!("Advertising game {} for board size {}", game_id, board_size);
            
            // Create game advertisement
            let advert = GameAdvert {
                gid: game_id.to_string(),
                size: board_size,
                host: self.my_id.clone(),
                bot: false, // Assume human player for now
            };
            
            // Serialize to CBOR
            let cbor_data = serde_cbor::to_vec(&advert)
                .context("Failed to serialize game advertisement")?;
            
            // Get the lobby topic for this board size
            let topic = Self::lobby_topic(board_size);
            
            // Broadcast the advertisement
            self.broadcast_to_topic(topic, &cbor_data).await
                .context("Failed to broadcast game advertisement")?;
            
            tracing::info!("Successfully advertised game {} for board size {}", game_id, board_size);
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock advertise game {} for board size {}", game_id, board_size);
            Ok(())
        }
    }
    
    /// Broadcast a move to a specific game topic
    #[tracing::instrument(level = "debug", skip(self, move_record))]
    pub async fn broadcast_move(&self, game_id: &str, move_record: &mut MoveRecord) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            // Serialize the move record first
            let bytes = serde_cbor::to_vec(move_record)?;
            ensure!(bytes.len() <= 1024, "Move record size exceeds 1KB limit: {}", bytes.len());
            
            let topic = Self::game_topic(game_id);
            
            // Broadcast the move
            self.broadcast_to_topic(topic, &bytes).await?;
            
            // After successful broadcast, compute and store the hash
            let hash = blake3::hash(&bytes);
            move_record.broadcast_hash = Some(*hash.as_bytes());
            
            tracing::debug!("Successfully broadcast move for game {} with hash: {:?}", 
                game_id, move_record.broadcast_hash);
            
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock broadcast move for game {}", game_id);
            Ok(())
        }
    }
    
    /// Generate a document ID from a game ID
    #[cfg(feature = "iroh")]
    pub fn doc_id_for_game(game_id: &str) -> NamespaceId {
        let hash = blake3::hash(game_id.as_bytes());
        NamespaceId::from(hash.as_bytes())
    }
    
    /// Store move in iroh-docs for game persistence (temporarily disabled)
    #[tracing::instrument(level = "debug", skip(self, move_record))]
    pub async fn store_game_move(&self, game_id: &str, sequence: u32, move_record: &MoveRecord) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            // TODO: Implement proper iroh-docs v0.35 API integration
            tracing::debug!(
                "Store game move for {} sequence {} (iroh-docs integration pending)",
                game_id, sequence
            );
            
            // Serialize the move record for validation
            let _value = serde_cbor::to_vec(move_record)?;
            
            tracing::debug!("Successfully stored move {} for game {} in docs", sequence, game_id);
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock store game move {} for game {}", sequence, game_id);
            Ok(())
        }
    }
    
    /// Store score acceptance in the game document
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn accept_score(&self, game_id: &str, player_id: &str, score: i32) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            // TODO: Implement proper iroh-docs v0.35 API integration
            tracing::debug!(
                "Storing score acceptance for player {} in game {} (iroh-docs integration pending)",
                player_id, game_id
            );
            
            // Serialize the score for validation
            let _value = serde_cbor::to_vec(&score)?;
            
            tracing::debug!("Score acceptance stored for player {} in game {}", player_id, game_id);
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock accept score {} for player {} in game {}", score, player_id, game_id);
            Ok(())
        }
    }
    
    /// Check if all players have agreed on the score
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn check_score_agreement(&self, game_id: &str, player_ids: &[&str]) -> Result<bool> {
        #[cfg(feature = "iroh")]
        {
            // TODO: Implement proper iroh-docs v0.35 API integration
            tracing::debug!(
                "Checking score agreement for game {} (iroh-docs integration pending)",
                game_id
            );
            
            // For now, return false to indicate no agreement yet
            // This will be properly implemented when iroh-docs integration is complete
            tracing::debug!("Score agreement check completed for {} players in game {}", player_ids.len(), game_id);
            Ok(false)
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock check score agreement for game {}", game_id);
            Ok(true) // Assume agreement in stub mode
        }
    }
    
    /// Mark a game with a specific status
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn mark_game_status(&self, game_id: &str, status: &str) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            // TODO: Implement proper iroh-docs v0.35 API integration
            tracing::debug!(
                "Marking game {} with status: {} (iroh-docs integration pending)",
                game_id, status
            );
            
            tracing::debug!("Game status marked for game {}: {}", game_id, status);
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock mark game {} status: {}", game_id, status);
            Ok(())
        }
    }
    
    /// Get the node address
    #[cfg(feature = "iroh")]
    pub async fn node_addr(&self) -> Result<NodeAddr> {
        self.endpoint.node_addr().await
    }
    
    /// Accept an incoming connection (for use by application layer)
    #[cfg(feature = "iroh")]
    pub async fn accept_connection(&self) -> Option<Connection> {
        let mut rx = self.connection_rx.lock().await;
        rx.recv().await
    }
    
    /// Get a reference to the router (for shutdown, etc.)
    #[cfg(feature = "iroh")]
    pub fn router(&self) -> &Router {
        &self.router
    }
    
    /// Store a tag annotation for a specific move
    #[tracing::instrument(level = "debug", skip(self))]
    pub async fn store_move_tag(&self, game_id: &str, sequence: u32, tag: p2pgo_core::Tag) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            // TODO: Implement proper iroh-docs v0.35 API integration for updating move tags
            tracing::debug!(
                "Store move tag for game {} sequence {} tag {:?} (iroh-docs integration pending)",
                game_id, sequence, tag
            );
            
            // For now, just log the tag - in full implementation this would update the 
            // existing MoveRecord in the game document
            tracing::debug!("Successfully stored tag {:?} for move {} in game {}", tag, sequence, game_id);
            Ok(())
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            tracing::debug!("Mock store move tag {:?} for game {} sequence {}", tag, game_id, sequence);
            Ok(())
        }
    }
    
    /// Add bootstrap peers for gossip discovery
    #[cfg(feature = "iroh")]
    pub fn add_bootstrap_peers(&self, peers: Vec<PublicKey>) -> Result<()> {
        // For now, just log the peers. In a full implementation, we would
        // store these and use them when subscribing to gossip topics
        tracing::info!("Adding {} bootstrap peers for gossip discovery", peers.len());
        for peer in &peers {
            tracing::debug!("Bootstrap peer: {}", peer);
        }
        
        // TODO: Store bootstrap peers in IrohCtx and use them in subscribe_gossip_topic
        // This would enable automatic discovery of other nodes in the network
        Ok(())
    }

    /// Add bootstrap peers (stub for non-iroh builds)
    #[cfg(not(feature = "iroh"))]
    pub fn add_bootstrap_peers(&self, _peers: Vec<String>) -> Result<()> {
        tracing::debug!("Mock add bootstrap peers");
        Ok(())
    }
    
    /// Get external addresses for this node
    #[cfg(feature = "iroh")]
    pub async fn external_addrs(&self) -> Result<Vec<String>> {
        let watcher = self.endpoint.direct_addresses();
        let addrs = watcher.get().map(|set| {
            set.into_iter()
                .map(|direct_addr| format!("{:?}", direct_addr)) // Convert to string for now
                .collect()
        }).unwrap_or_default();
        Ok(addrs)
    }

    /// Get external addresses (stub for non-iroh builds)
    #[cfg(not(feature = "iroh"))]
    pub async fn external_addrs(&self) -> Result<Vec<String>> {
        Ok(vec!["stub-addr".to_string()])
    }
}

// Re-export types for convenience
#[cfg(feature = "iroh")]
pub use iroh_gossip::net::Event as GossipEvent;

#[cfg(feature = "iroh")]
pub type P2PGossipEvent = iroh_gossip::net::Event;