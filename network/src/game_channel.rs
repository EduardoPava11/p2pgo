// filepath: /Users/daniel/p2pgo/network/src/game_channel.rs
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game channel for communication between players

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use anyhow::{Result, Context};
use p2pgo_core::{Move, GameState, GameEvent, MoveRecord};
use crate::GameId;
use crate::blob_store::{MoveBlob, MoveChain};

// Import iroh-docs only when feature is enabled
#[cfg(feature = "iroh")]
use {
    crate::iroh_endpoint::IrohCtx,
    blake3::Hasher,
    iroh_docs::NamespaceId,
    iroh_gossip::proto::TopicId,
    iroh::{endpoint::Connection},
    std::collections::HashSet,
    tokio::task::JoinHandle,
    serde_json,
    blake3,
};

/// Status of a player in the game
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerStatus {
    /// Player has connected but not yet joined
    Connected,
    /// Player has joined the game as a specific color
    Playing(p2pgo_core::Color),
    /// Player is watching the game (not playing)
    Watching,
}

/// A channel for game-related communication
pub struct GameChannel {
    /// Game ID
    game_id: GameId,
    /// Move chain for storing game history
    move_chain: Arc<RwLock<MoveChain>>,
    /// Event broadcast channel
    events_tx: broadcast::Sender<GameEvent>,
    /// Latest game state
    latest_state: Arc<RwLock<Option<GameState>>>,
    
    /// Iroh networking context when feature is enabled
    #[cfg(feature = "iroh")]
    iroh_ctx: Option<Arc<IrohCtx>>,
    
    /// Active connections to peers for this game
    #[cfg(feature = "iroh")]
    peer_connections: Arc<RwLock<Vec<Connection>>>,
    
    /// Background task for handling incoming connections
    #[cfg(feature = "iroh")]
    _connection_task: Option<JoinHandle<()>>,
    
    /// Set of already processed sequence numbers to avoid duplicates
    #[cfg(feature = "iroh")]
    processed_sequences: Arc<RwLock<HashSet<u32>>>,
}

impl GameChannel {
    /// Create a new game channel
    pub fn new(game_id: GameId, initial_state: GameState) -> Self {
        let _span = tracing::info_span!("network.game_channel", "GameChannel::new").entered();
        
        // Create a broadcast channel for events with buffer size 100
        let (events_tx, _) = broadcast::channel(100);
        
        // Create move chain
        let move_chain = MoveChain::new(game_id.clone());
        
        #[cfg(not(feature = "iroh"))]
        return Self {
            game_id,
            move_chain: Arc::new(RwLock::new(move_chain)),
            events_tx,
            latest_state: Arc::new(RwLock::new(Some(initial_state))),
        };
        
        #[cfg(feature = "iroh")]
        return Self {
            game_id,
            move_chain: Arc::new(RwLock::new(move_chain)),
            events_tx,
            latest_state: Arc::new(RwLock::new(Some(initial_state))),
            iroh_ctx: None,
            peer_connections: Arc::new(RwLock::new(Vec::new())),
            _connection_task: None,
            processed_sequences: Arc::new(RwLock::new(HashSet::new())),
        };
    }
    
    /// Create a document ID from a game ID
    #[cfg(feature = "iroh")]
    #[allow(dead_code)]
    fn doc_id_for_game(game_id: &str) -> NamespaceId {
        let mut hasher = Hasher::new();
        hasher.update(game_id.as_bytes());
        let hash = hasher.finalize();
        
        // Convert hash to NamespaceId
        NamespaceId::from(hash.as_bytes())
    }
    
    /// Create a gossip topic ID for a game
    #[cfg(feature = "iroh")]
    fn game_topic_id(game_id: &str) -> TopicId {
        // Use the same topic generation as IrohCtx to ensure consistency
        let topic_name = format!("p2pgo.game.{}", game_id);
        TopicId::from_bytes(*blake3::hash(topic_name.as_bytes()).as_bytes())
    }
    
    /// Create a new game channel with an Iroh context for network synchronization
    #[cfg(feature = "iroh")]
    #[tracing::instrument(level = "debug", skip(iroh_ctx))]
    pub async fn with_iroh(game_id: GameId, initial_state: GameState, iroh_ctx: Arc<IrohCtx>) -> Result<Self> {
        tracing::info!("Creating GameChannel with Iroh for game {}", game_id);
        
        // Create the basic channel
        let mut channel = Self::new(game_id.clone(), initial_state.clone());
        
        // Set the iroh context
        channel.iroh_ctx = Some(iroh_ctx.clone());
        
        // Start connection handler that will handle both incoming and outgoing connections
        let peer_connections = channel.peer_connections.clone();
        let events_tx = channel.events_tx.clone();
        let processed_sequences = channel.processed_sequences.clone();
        let move_chain = channel.move_chain.clone();
        let latest_state = channel.latest_state.clone();
        let game_id_for_task = game_id.clone();
        
        let connection_task = tokio::spawn(async move {
            tracing::info!("Starting connection handler for game: {}", game_id_for_task);
            
            // Accept incoming connections and handle them
            while let Some(connection) = iroh_ctx.accept_connection().await {
                tracing::info!("New connection for game: {}", game_id_for_task);
                
                // Add connection to our list
                {
                    let mut connections = peer_connections.write().await;
                    connections.push(connection.clone());
                    tracing::info!("Total connections for game {}: {}", game_id_for_task, connections.len());
                }
                
                // Spawn a task to handle this specific connection
                let events_tx_conn = events_tx.clone();
                let processed_sequences_conn = processed_sequences.clone();
                let move_chain_conn = move_chain.clone();
                let latest_state_conn = latest_state.clone();
                let game_id_conn = game_id_for_task.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_peer_connection(
                        connection,
                        game_id_conn,
                        events_tx_conn,
                        processed_sequences_conn,
                        move_chain_conn,
                        latest_state_conn,
                    ).await {
                        tracing::error!("Error handling peer connection: {}", e);
                    }
                });
            }
        });
        
        channel._connection_task = Some(connection_task);
        
        // Subscribe to gossip topic for this game (best effort)
        if let Err(e) = channel.subscribe_to_game_topic().await {
            tracing::warn!("Failed to subscribe to gossip topic (will rely on direct connections): {}", e);
        }
        
        tracing::info!("Successfully created game channel with Iroh for game: {}", game_id);
        Ok(channel)
    }
    

    
    /// Subscribe to the game's gossip topic
    #[cfg(feature = "iroh")]
    async fn subscribe_to_game_topic(&mut self) -> Result<()> {
        if let Some(iroh_ctx) = &self.iroh_ctx {
            let _topic_id = Self::game_topic_id(&self.game_id);
            tracing::info!("Subscribing to gossip topic for game: {}", self.game_id);
            
            // Subscribe to the game topic via gossip
            let mut gossip_events = iroh_ctx.subscribe_game_topic(&self.game_id, 100).await?;
            
            // Start a background task to handle incoming gossip messages
            let events_tx = self.events_tx.clone();
            let _processed_sequences = self.processed_sequences.clone();
            let _move_chain = self.move_chain.clone();
            let latest_state = self.latest_state.clone();
            let game_id_for_gossip = self.game_id.clone();
            
            tokio::spawn(async move {
                tracing::info!("Starting gossip handler for game: {}", game_id_for_gossip);
                
                while let Some(event) = gossip_events.recv().await {
                    tracing::debug!("Received gossip event for game {}: {:?}", game_id_for_gossip, event);
                    if let Err(e) = Self::handle_gossip_event(
                        event,
                        &events_tx,
                        &latest_state,
                    ).await {
                        tracing::error!("Error handling gossip event: {}", e);
                    }
                }
                tracing::warn!("Gossip event stream ended for game: {}", game_id_for_gossip);
            });
            
            tracing::info!("Successfully subscribed to gossip for game: {}", self.game_id);
        }
        Ok(())
    }
    
    /// Broadcast a move via gossip
    #[cfg(feature = "iroh")]
    async fn broadcast_move(&self, record: &MoveRecord) -> Result<()> {
        if let Some(iroh_ctx) = &self.iroh_ctx {
            // Serialize the move record to CBOR for broadcast
            let _cbor_data = serde_cbor::to_vec(record)
                .context("Failed to serialize move record for gossip")?;
            
            tracing::debug!("Broadcasting move via gossip: {:?}", record.mv);
            
            // Try gossip broadcast
            match iroh_ctx.broadcast_move(&self.game_id, &mut record.clone()).await {
                Ok(()) => {
                    tracing::info!("Successfully broadcast move via gossip");
                }
                Err(e) => {
                    tracing::warn!("Failed to broadcast via gossip, will use direct connections: {}", e);
                }
            }
        }
        Ok(())
    }
    
    /// Broadcast a move via gossip (stub implementation)
    #[cfg(not(feature = "iroh"))]
    async fn broadcast_move(&self, record: &MoveRecord) -> Result<()> {
        tracing::debug!("Mock broadcast move {:?}", record.mv);
        Ok(())
    }
    
    /// Get a receiver for game events
    pub fn subscribe(&self) -> broadcast::Receiver<GameEvent> {
        self.events_tx.subscribe()
    }
    
    /// Send a move to the channel
    pub async fn send_move(&self, mv: Move) -> Result<()> {
        self.push_move(mv, None).await
    }
    
    /// Push a move with optional tag to the channel
    #[tracing::instrument(level = "debug", skip(self, mv))]
    pub async fn push_move(&self, mv: Move, tag: Option<p2pgo_core::Tag>) -> Result<()> {
        let _span = tracing::info_span!("network.game_channel", "GameChannel::send_move").entered();
        
        // Get the current game state
        let mut state = {
            let state_guard = self.latest_state.read().await;
            match &*state_guard {
                Some(state) => state.clone(),
                None => return Err(anyhow::anyhow!("No game state available")),
            }
        };
        
        // Apply the move to the state
        state.apply_move(mv.clone())?;
        
        // Get the current chain
        let mut chain = self.move_chain.write().await;
        
        // Get the previous hash and sequence
        let prev_hash = chain.current_blob().map(|blob| blob.hash());
        let sequence = if chain.current_blob().is_none() { 0 } else { chain.current_sequence + 1 };
        
        // Create a new move blob
        let blob = MoveBlob::new(
            self.game_id.clone(),
            mv,
            prev_hash,
            state.clone(),
            sequence
        );
        
        // Get the move for the event before consuming the blob
        let move_for_event = blob.mv.clone();
        
        // Add the blob to the chain
        chain.add_blob(blob)?;
        
        // If using iroh, store the move in the document
        #[cfg(feature = "iroh")]
        if let Some(_iroh_ctx) = &self.iroh_ctx {
            // Create a move record
            let move_record = MoveRecord {
                mv: move_for_event.clone(),
                tag: tag.clone(),
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                broadcast_hash: None, // Will be set after broadcast
                prev_hash: prev_hash.clone(),
            };
            
            // CBOR encode the move record
            let _cbor_data = serde_cbor::to_vec(&move_record)
                .context("Failed to CBOR encode move record")?;
                
            // Mark this move as processed locally to avoid duplicate processing
            {
                let mut processed = self.processed_sequences.write().await;
                processed.insert(sequence);
            }
            
            // Store the move in the document using the IrohCtx API
            // TODO: Update for iroh v0.35 docs API
            // iroh_ctx.store_game_move(&self.game_id, sequence, &cbor_data)
            //     .await
            //     .context("Failed to store move in document")?;
                
            tracing::info!("Move storage disabled - needs iroh v0.35 update: sequence {}", sequence);
        }
        
        // Update the latest state
        {
            let mut state_guard = self.latest_state.write().await;
            *state_guard = Some(state.clone());
        }
        
        // Store tag if provided (for training data)
        if let Some(tag) = tag {
            tracing::debug!(
                game_id = %self.game_id,
                sequence = sequence,
                tag = ?tag,
                "Storing move tag for training"
            );
            // Tag is stored in the CBOR MoveRecord for training purposes
        }
        
        // Create a game event for the move
        // The player making the move is the one before the current_player in the state
        let event = GameEvent::MoveMade {
            mv: move_for_event.clone(),
            by: match state.current_player {
                p2pgo_core::Color::Black => p2pgo_core::Color::White,
                p2pgo_core::Color::White => p2pgo_core::Color::Black,
            },
        };
        
        // Broadcast the event
        if let Err(e) = self.events_tx.send(event) {
            tracing::warn!("Failed to broadcast move event: {}", e);
        }
        
        // If using iroh, broadcast the move to connected peers
        #[cfg(feature = "iroh")]
        {
            // Create a move record for peer communication
            let move_record = MoveRecord {
                mv: move_for_event.clone(),
                tag: tag.clone(),
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                broadcast_hash: None, // Will be set after broadcast
                prev_hash: prev_hash.clone(),
            };
            
            // Broadcast the move via both gossip and direct connections for reliability
            let _ = self.broadcast_move(&move_record).await; // Gossip (may fail)
            
            // Always broadcast to directly connected peers as primary mechanism
            if let Err(e) = self.broadcast_move_to_peers(&move_record).await {
                tracing::warn!("Failed to broadcast move to direct peers: {}", e);
            } else {
                tracing::info!("Successfully broadcast move to direct peers");
            }
        }
        
        Ok(())
    }
    
    /// Get the latest game state
    pub async fn get_latest_state(&self) -> Option<GameState> {
        self.latest_state.read().await.clone()
    }
    
    /// Get all moves in the game so far
    pub async fn get_all_moves(&self) -> Vec<Move> {
        let chain = self.move_chain.read().await;
        
        chain.get_all_blobs().iter()
            .map(|blob| blob.mv.clone())
            .collect()
    }
    
    /// Send a chat message or other game event
    pub async fn send_event(&self, event: GameEvent) -> Result<()> {
        let _span = tracing::info_span!("network.game_channel", "GameChannel::send_event").entered();
        
        // Just broadcast the event
        self.events_tx.send(event)
            .map_err(|e| anyhow::anyhow!("Failed to broadcast event: {}", e))?;
            
        Ok(())
    }
    
    /// Handle a peer connection for game synchronization
    #[cfg(feature = "iroh")]
    async fn handle_peer_connection(
        connection: Connection,
        game_id: String,
        events_tx: broadcast::Sender<GameEvent>,
        processed_sequences: Arc<RwLock<HashSet<u32>>>,
        move_chain: Arc<RwLock<MoveChain>>,
        latest_state: Arc<RwLock<Option<GameState>>>,
    ) -> Result<()> {
        tracing::debug!("Handling peer connection for game: {}", game_id);
        
        // Listen for incoming unidirectional streams
        loop {
            match connection.accept_uni().await {
                Ok(mut recv_stream) => {
                    tracing::debug!("Accepted unidirectional stream for game: {}", game_id);
                    
                    // Read the incoming message
                    match recv_stream.read_to_end(1024).await {
                        Ok(buffer) => {
                            let message = String::from_utf8_lossy(&buffer);
                            tracing::debug!("Received message for game {}: {}", game_id, message);
                            
                            // Parse the move record (remove trailing newline if present)
                            let message = message.trim();
                            if let Ok(move_record) = serde_json::from_str::<MoveRecord>(message) {
                                tracing::debug!("Successfully parsed move record: {:?}", move_record.mv);
                                
                                // Process the received move
                                if let Err(e) = Self::process_received_move_direct(
                                    move_record,
                                    &events_tx,
                                    &latest_state,
                                    &move_chain,
                                    &processed_sequences,
                                    &game_id,
                                ).await {
                                    tracing::error!("Error processing received move for {}: {}", game_id, e);
                                }
                            } else {
                                tracing::warn!("Failed to parse move record for {}: {}", game_id, message);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error reading from stream for {}: {}", game_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error accepting stream for {}: {}", game_id, e);
                    break;
                }
            }
        }
        
        tracing::debug!("Connection handler finished for game: {}", game_id);
        Ok(())
    }

    /// Handle incoming gossip events
    #[cfg(feature = "iroh")]
    async fn handle_gossip_event(
        event: iroh_gossip::net::Event,
        events_tx: &broadcast::Sender<GameEvent>,
        latest_state: &Arc<RwLock<Option<GameState>>>,
    ) -> Result<()> {
        tracing::debug!("Received gossip event for game");
        
        // Extract bytes from the gossip event using the compatibility layer
        use crate::gossip_compat::{extract_bytes, is_received_message};
        
        // Only process received message events
        if is_received_message(&event) {
            if let Some(content) = extract_bytes(&event) {
                // Try to deserialize the message as a MoveRecord
                match serde_cbor::from_slice::<MoveRecord>(&content) {
                    Ok(move_record) => {
                        tracing::debug!("Parsed move record from gossip: {:?}", move_record.mv);
                        // Process the received move
                        Self::process_received_move(
                            move_record,
                            events_tx,
                            latest_state,
                        ).await?;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize gossip message: {}", e);
                    }
                }
            }
        } else {
            tracing::debug!("Ignoring non-message gossip event");
        }
        
        Ok(())
    }

    /// Process a received move from gossip
    #[cfg(feature = "iroh")]
    async fn process_received_move(
        move_record: MoveRecord,
        events_tx: &broadcast::Sender<GameEvent>,
        latest_state: &Arc<RwLock<Option<GameState>>>,
    ) -> Result<()> {
        tracing::debug!("Processing received move: {:?}", move_record.mv);
        
        // Get the current game state
        let mut current_state = {
            let state_guard = latest_state.read().await;
            match &*state_guard {
                Some(state) => state.clone(),
                None => return Err(anyhow::anyhow!("No game state available")),
            }
        };
        
        // Apply the move to the state
        current_state.apply_move(move_record.mv.clone())?;
        
        // Update the latest state
        {
            let mut state_guard = latest_state.write().await;
            *state_guard = Some(current_state.clone());
        }
        
        // Create a game event for the move
        let event = GameEvent::MoveMade {
            mv: move_record.mv.clone(),
            by: match current_state.current_player {
                p2pgo_core::Color::Black => p2pgo_core::Color::White,
                p2pgo_core::Color::White => p2pgo_core::Color::Black,
            },
        };
        
        // Broadcast the event locally
        if let Err(e) = events_tx.send(event) {
            tracing::warn!("Failed to broadcast received move event: {}", e);
        }
        
        tracing::info!("Successfully processed received move: {:?}", move_record.mv);
        Ok(())
    }

    /// Process a received move from direct peer connection
    #[cfg(feature = "iroh")]
    async fn process_received_move_direct(
        move_record: MoveRecord,
        events_tx: &broadcast::Sender<GameEvent>,
        latest_state: &Arc<RwLock<Option<GameState>>>,
        move_chain: &Arc<RwLock<MoveChain>>,
        processed_sequences: &Arc<RwLock<HashSet<u32>>>,
        game_id: &str,
    ) -> Result<()> {
        tracing::debug!("Processing received move for {}: {:?}", game_id, move_record.mv);
        
        // Check if we've already processed this move by hash
        if let Some(hash) = move_record.broadcast_hash {
            let processed = processed_sequences.read().await;
            if processed.contains(&(hash[0] as u32)) { // Use first byte as simple check
                tracing::debug!("Move already processed, skipping");
                return Ok(());
            }
        }
        
        // Add to processed (using hash if available)
        if let Some(hash) = move_record.broadcast_hash {
            let mut processed = processed_sequences.write().await;
            processed.insert(hash[0] as u32);
        }
        
        // Apply the move locally
        let new_state = {
            tracing::debug!("Acquiring move chain lock for {}", game_id);
            let mut chain = move_chain.write().await;
            
            // Get current state to apply move to
            tracing::debug!("Getting current state for {}", game_id);
            let current_state = latest_state.read().await;
            if let Some(mut state) = current_state.clone() {
                tracing::debug!("Applying move to state for {}", game_id);
                // Apply the move to get the new state
                if state.apply_move(move_record.mv.clone()).is_ok() {
                    tracing::debug!("Move applied successfully for {}", game_id);
                    let sequence = if chain.current_blob().is_none() { 0 } else { chain.current_sequence + 1 };
                    let prev_hash = chain.current_blob().map(|blob| blob.hash());
                    
                    let blob = MoveBlob::new(
                        game_id.to_string(),
                        move_record.mv.clone(),
                        prev_hash,
                        state.clone(),
                        sequence,
                    );
                    
                    tracing::debug!("Adding blob to chain for {}", game_id);
                    if chain.add_blob(blob).is_ok() {
                        tracing::debug!("Blob added successfully for {}", game_id);
                        Some(state)
                    } else {
                        tracing::error!("Failed to add blob to chain for {}", game_id);
                        None
                    }
                } else {
                    tracing::error!("Failed to apply move to state for {}", game_id);
                    None
                }
            } else {
                tracing::error!("No current state available for {}", game_id);
                None
            }
        };
        
        // Update state and broadcast event (outside of move_chain lock)
        if let Some(state) = new_state {
            tracing::debug!("Updating latest state for {}", game_id);
            {
                let mut state_guard = latest_state.write().await;
                *state_guard = Some(state.clone());
                tracing::debug!("State updated for {}", game_id);
            }
            
            // Broadcast the move event
            let event = GameEvent::MoveMade {
                mv: move_record.mv.clone(),
                by: match state.current_player {
                    p2pgo_core::Color::Black => p2pgo_core::Color::White,
                    p2pgo_core::Color::White => p2pgo_core::Color::Black,
                },
            };
            
            tracing::debug!("Broadcasting move event for game {}: {:?}", game_id, event);
            if let Err(e) = events_tx.send(event) {
                tracing::error!("Failed to broadcast move event for {}: {}", game_id, e);
            } else {
                tracing::debug!("Successfully broadcast move event for game: {}", game_id);
            }
        }
        
        Ok(())
    }

    /// Broadcast move to peers over direct connections
    #[cfg(feature = "iroh")]
    pub async fn broadcast_move_to_peers(&self, move_record: &MoveRecord) -> Result<()> {
        let connections = self.peer_connections.read().await;
        
        tracing::debug!("Broadcasting move to {} peer connection(s)", connections.len());
        
        if connections.is_empty() {
            tracing::debug!("No peer connections available for broadcasting");
            return Ok(());
        }
        
        let message = serde_json::to_string(move_record)?;
        tracing::debug!("Broadcasting move message: {}", message);
        
        for (i, connection) in connections.iter().enumerate() {
            tracing::debug!("Attempting to send to peer {}", i);
            // Try to open a unidirectional stream to send the move
            match connection.open_uni().await {
                Ok(mut send_stream) => {
                    tracing::debug!("Opened stream to peer {}", i);
                    // Use iroh's stream write methods
                    match send_stream.write_all(message.as_bytes()).await {
                        Ok(()) => {
                            match send_stream.write_all(b"\n").await {
                                Ok(()) => {
                                    // Close the stream
                                    if let Err(e) = send_stream.finish() {
                                        tracing::error!("Failed to finish stream to peer {}: {}", i, e);
                                    } else {
                                        tracing::debug!("Successfully sent move to peer {}", i);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to send delimiter to peer {}: {}", i, e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to send move to peer {}: {}", i, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to open stream to peer {}: {}", i, e);
                }
            }
        }
        
        tracing::debug!("Completed broadcasting move to {} peer(s)", connections.len());
        Ok(())
    }
    
    /// Connect to a peer's game channel for the same game
    #[cfg(feature = "iroh")]
    pub async fn connect_to_peer(&self, peer_ticket: &str) -> Result<()> {
        if let Some(iroh_ctx) = &self.iroh_ctx {
            tracing::debug!("Connecting to peer for game: {}", self.game_id);
            
            // Connect to the peer
            let connection = iroh_ctx.connect_to_peer(peer_ticket).await?;
            
            // Add the connection to our peer list
            {
                let mut connections = self.peer_connections.write().await;
                connections.push(connection.clone());
                tracing::debug!("Added peer connection for game {}, total: {}", self.game_id, connections.len());
            }
            
            // Spawn a task to handle this connection
            let events_tx = self.events_tx.clone();
            let processed_sequences = self.processed_sequences.clone();
            let move_chain = self.move_chain.clone();
            let latest_state = self.latest_state.clone();
            let game_id = self.game_id.clone();
            
            tokio::spawn(async move {
                tracing::debug!("Starting connection handler for game: {}", game_id);
                if let Err(e) = Self::handle_peer_connection(
                    connection,
                    game_id.clone(),
                    events_tx,
                    processed_sequences,
                    move_chain,
                    latest_state,
                ).await {
                    tracing::error!("Error handling peer connection for {}: {}", game_id, e);
                }
            });
            
            tracing::debug!("Successfully connected to peer for game: {}", self.game_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::Coord;
    
    #[tokio::test]
    async fn test_game_channel() {
        let game_id = "test-game".to_string();
        let initial_state = GameState::new(9);
        let channel = GameChannel::new(game_id, initial_state);
        
        // Send a move
        let mv = Move::Place(Coord::new(4, 4));
        channel.send_move(mv.clone()).await.unwrap();
        
        // Check the state was updated
        let state = channel.get_latest_state().await.unwrap();
        assert_eq!(state.current_player, p2pgo_core::Color::White); // Turn switched to White
    }
    
    #[cfg(feature = "iroh")]
    #[tokio::test]
    async fn test_game_channel_with_iroh() -> Result<()> {
        use crate::iroh_endpoint::IrohCtx;
        use std::sync::Arc;
        
        // Create iroh context
        let iroh_ctx = Arc::new(IrohCtx::new().await?);
        
        // Create a unique game ID
        let game_id = format!("test-game-{}", uuid::Uuid::new_v4());
        let initial_state = GameState::new(9);
        
        // Create game channel with iroh
        let channel = GameChannel::with_iroh(game_id.clone(), initial_state, iroh_ctx.clone()).await?;
        
        // Wait a bit for initialization
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Send a move
        let mv = Move::Place(Coord::new(4, 4));
        channel.send_move(mv.clone()).await?;
        
        // Wait a bit for processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Check the state was updated
        let state = channel.get_latest_state().await.unwrap();
        assert_eq!(state.current_player, p2pgo_core::Color::White); // Turn switched to White
        
        // Verify that the move is stored in iroh docs
        // TODO: Update for iroh v0.35 docs API
        // let moves = iroh_ctx.fetch_training_doc(&game_id).await?;
        // assert!(!moves.is_empty(), "Moves should be stored in the document");
        
        Ok(())
    }
}
