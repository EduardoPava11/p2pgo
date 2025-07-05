// SPDX-License-Identifier: MIT OR Apache-2.0

//! P2P networking functionality for game channels

#[cfg(feature = "iroh")]
use {
    super::{messages::*, state},
    crate::iroh_endpoint::IrohCtx,
    blake3,
    blake3::Hasher,
    iroh::{endpoint::Connection, NodeId},
    iroh_docs::NamespaceId,
    iroh_gossip::proto::TopicId,
    serde_json,
    std::collections::VecDeque,
    tokio::sync::{broadcast, Mutex, RwLock},
};

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

/// Subscribe to the game's gossip topic
#[cfg(feature = "iroh")]
pub async fn subscribe_to_game_topic(channel: &mut GameChannel) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        let _topic_id = game_topic_id(&channel.game_id);
        tracing::info!("Subscribing to gossip topic for game: {}", channel.game_id);

        // Subscribe to the game topic via gossip
        let mut gossip_events = iroh_ctx.subscribe_game_topic(&channel.game_id, 100).await?;

        // Start a background task to handle incoming gossip messages
        let events_tx = channel.events_tx.clone();
        let _processed_sequences = channel.processed_sequences.clone();
        let _move_chain = channel.move_chain.clone();
        let latest_state = channel.latest_state.clone();
        let game_id_for_gossip = channel.game_id.clone();

        tokio::spawn(async move {
            tracing::info!("Starting gossip handler for game: {}", game_id_for_gossip);

            while let Some(event) = gossip_events.recv().await {
                tracing::debug!(
                    "Received gossip event for game {}: {:?}",
                    game_id_for_gossip,
                    event
                );
                if let Err(e) = handle_gossip_event(event, &events_tx, &latest_state).await {
                    tracing::error!("Error handling gossip event: {}", e);
                }
            }
            tracing::warn!("Gossip event stream ended for game: {}", game_id_for_gossip);
        });

        tracing::info!(
            "Successfully subscribed to gossip for game: {}",
            channel.game_id
        );
    }
    Ok(())
}

/// Broadcast a move via gossip
#[cfg(feature = "iroh")]
pub async fn broadcast_move(channel: &GameChannel, record: &MoveRecord) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        // Serialize the move record to CBOR for broadcast
        let _cbor_data =
            serde_cbor::to_vec(record).context("Failed to serialize move record for gossip")?;

        tracing::debug!("Broadcasting move via gossip: {:?}", record.mv);

        // Try gossip broadcast
        match iroh_ctx
            .broadcast_move(&channel.game_id, &mut record.clone())
            .await
        {
            Ok(()) => {
                tracing::info!("Successfully broadcast move via gossip");
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to broadcast via gossip, will use direct connections: {}",
                    e
                );
            }
        }
    }
    Ok(())
}

/// Broadcast move to peers over direct connections
#[cfg(feature = "iroh")]
pub async fn broadcast_move_to_peers(
    channel: &GameChannel,
    move_record: &MoveRecord,
) -> Result<()> {
    let connections = channel.peer_connections.read().await;

    tracing::debug!(
        "Broadcasting move to {} peer connection(s)",
        connections.len()
    );

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

    tracing::debug!(
        "Completed broadcasting move to {} peer(s)",
        connections.len()
    );
    Ok(())
}

/// Connect to a peer's game channel for the same game
#[cfg(feature = "iroh")]
pub async fn connect_to_peer(channel: &GameChannel, peer_ticket: &str) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        tracing::debug!("Connecting to peer for game: {}", channel.game_id);

        // Connect to the peer
        let connection = iroh_ctx.connect_to_peer(peer_ticket).await?;

        // Add the connection to our peer list
        {
            let mut connections = channel.peer_connections.write().await;
            connections.push(connection.clone());
            tracing::debug!(
                "Added peer connection for game {}, total: {}",
                channel.game_id,
                connections.len()
            );
        }

        // Spawn a task to handle this connection
        let events_tx = channel.events_tx.clone();
        let processed_sequences = channel.processed_sequences.clone();
        let move_chain = channel.move_chain.clone();
        let latest_state = channel.latest_state.clone();
        let game_id = channel.game_id.clone();

        tokio::spawn(async move {
            tracing::debug!("Starting connection handler for game: {}", game_id);
            if let Err(e) = handle_peer_connection(
                connection,
                game_id.clone(),
                events_tx,
                processed_sequences,
                move_chain,
                latest_state,
            )
            .await
            {
                tracing::error!("Error handling peer connection for {}: {}", game_id, e);
            }
        });

        tracing::debug!(
            "Successfully connected to peer for game: {}",
            channel.game_id
        );
    }
    Ok(())
}

/// Handle a peer connection for game synchronization
#[cfg(feature = "iroh")]
pub async fn handle_peer_connection(
    connection: Connection,
    game_id: String,
    events_tx: broadcast::Sender<GameEvent>,
    processed_sequences: Arc<Mutex<VecDeque<(NodeId, u64)>>>,
    move_chain: Arc<RwLock<crate::blob_store::MoveChain>>,
    latest_state: Arc<RwLock<Option<p2pgo_core::GameState>>>,
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

                        // Parse the message (remove trailing newline if present)
                        let message = message.trim();

                        // Try parsing as different message types
                        if let Ok(move_record) = serde_json::from_str::<MoveRecord>(message) {
                            tracing::debug!(
                                "Successfully parsed move record: {:?}",
                                move_record.mv
                            );

                            // Send ACK for the received move
                            let move_index = {
                                let chain = move_chain.read().await;
                                chain.get_all_blobs().len()
                            };

                            // Process the received move
                            if let Err(e) = state::process_received_move_direct(
                                move_record,
                                &events_tx,
                                &latest_state,
                                &move_chain,
                                &processed_sequences,
                                &game_id,
                            )
                            .await
                            {
                                tracing::error!(
                                    "Error processing received move for {}: {}",
                                    game_id,
                                    e
                                );
                            } else {
                                // Send ACK only if move was processed successfully
                                // The ACK contains the index where the move was added
                                let channel = super::GameChannel::get_for_game_id(&game_id).await;
                                if let Some(channel) = channel {
                                    if let Err(e) =
                                        send_move_ack(&channel, move_index, Some(&connection)).await
                                    {
                                        tracing::warn!("Failed to send ACK: {}", e);
                                    }
                                }
                            }
                        } else if let Ok(ack) = serde_json::from_str::<MoveAck>(message) {
                            tracing::debug!(
                                "Received move ACK for index {} in game {}",
                                ack.move_index,
                                ack.game_id
                            );

                            // Find the game channel and reset its watchdog
                            let channel = super::GameChannel::get_for_game_id(&game_id).await;
                            if let Some(channel) = channel {
                                // Reset the ACK watchdog
                                let mut last_index = channel.last_sent_index.write().await;
                                let mut last_time = channel.last_sent_time.write().await;
                                let mut sync_req = channel.sync_requested.write().await;

                                // Only reset if this ACK is for the move we're waiting for
                                if let Some(index) = *last_index {
                                    if index == ack.move_index {
                                        tracing::debug!(
                                            "Resetting ACK watchdog for move index {}",
                                            index
                                        );
                                        *last_index = None;
                                        *last_time = None;
                                        *sync_req = false;
                                    }
                                }
                            }
                        } else if let Ok(sync_req) = serde_json::from_str::<SyncRequest>(message) {
                            tracing::debug!("Received sync request for game {}", sync_req.game_id);

                            // Find the game channel and handle the sync request
                            let channel =
                                super::GameChannel::get_for_game_id(&sync_req.game_id).await;
                            if let Some(channel) = channel {
                                if let Err(e) = super::sync::handle_sync_request(
                                    &channel,
                                    sync_req,
                                    Some(&connection),
                                )
                                .await
                                {
                                    tracing::error!("Error handling sync request: {}", e);
                                }
                            }
                        } else if let Ok(sync_resp) = serde_json::from_str::<SyncResponse>(message)
                        {
                            tracing::debug!(
                                "Received sync response for game {}",
                                sync_resp.game_id
                            );

                            // Find the game channel and handle the sync response
                            let channel =
                                super::GameChannel::get_for_game_id(&sync_resp.game_id).await;
                            if let Some(channel) = channel {
                                if let Err(e) =
                                    super::sync::handle_sync_response(&channel, sync_resp).await
                                {
                                    tracing::error!("Error handling sync response: {}", e);
                                }
                            }
                        } else {
                            tracing::warn!("Failed to parse message for {}: {}", game_id, message);
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
    latest_state: &Arc<RwLock<Option<p2pgo_core::GameState>>>,
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
                    process_received_move_from_gossip(move_record, events_tx, latest_state).await?;
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
async fn process_received_move_from_gossip(
    move_record: MoveRecord,
    events_tx: &broadcast::Sender<GameEvent>,
    latest_state: &Arc<RwLock<Option<p2pgo_core::GameState>>>,
) -> Result<()> {
    tracing::debug!("Processing received move: {:?}", move_record.mv);

    // Verify signature
    let game_id = "unknown"; // We don't have a game_id in this context
    if !super::crypto::try_verify_move_record(&move_record, game_id) {
        return Err(anyhow::anyhow!("Signature verification failed"));
    }

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

/// Send an acknowledgment for a received move
#[cfg(feature = "iroh")]
pub async fn send_move_ack(
    channel: &GameChannel,
    move_index: usize,
    to_connection: Option<&Connection>,
) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        tracing::debug!(
            "Sending ACK for move index {} in game {}",
            move_index,
            channel.game_id
        );

        // Create an ACK message
        let ack = MoveAck {
            game_id: channel.game_id.clone(),
            move_index,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        // Serialize the ACK
        let message = serde_json::to_string(&ack)?;

        if let Some(conn) = to_connection {
            // Send directly to the specified peer
            match conn.open_uni().await {
                Ok(mut send_stream) => {
                    if let Err(e) = send_stream.write_all(message.as_bytes()).await {
                        tracing::error!("Failed to send ACK: {}", e);
                    } else if let Err(e) = send_stream.write_all(b"\n").await {
                        tracing::error!("Failed to send delimiter: {}", e);
                    } else {
                        // Close the stream
                        if let Err(e) = send_stream.finish() {
                            tracing::error!("Failed to finish stream: {}", e);
                        } else {
                            tracing::debug!("Successfully sent ACK");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to open stream for ACK: {}", e);
                }
            }
        } else {
            // Broadcast to all peers
            let connections = channel.peer_connections.read().await;

            for (i, connection) in connections.iter().enumerate() {
                match connection.open_uni().await {
                    Ok(mut send_stream) => {
                        if let Err(e) = send_stream.write_all(message.as_bytes()).await {
                            tracing::error!("Failed to send ACK to peer {}: {}", i, e);
                        } else if let Err(e) = send_stream.write_all(b"\n").await {
                            tracing::error!("Failed to send delimiter to peer {}: {}", i, e);
                        } else {
                            // Close the stream
                            if let Err(e) = send_stream.finish() {
                                tracing::error!("Failed to finish stream to peer {}: {}", i, e);
                            } else {
                                tracing::debug!("Successfully sent ACK to peer {}", i);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to open stream to peer {}: {}", i, e);
                    }
                }
            }
        }

        // Also try via gossip
        if let Ok(json_data) = serde_json::to_vec(&ack) {
            if let Err(e) = iroh_ctx
                .broadcast_to_game_topic(&channel.game_id, &json_data)
                .await
            {
                tracing::warn!("Failed to broadcast ACK via gossip: {}", e);
            }
        }
    }

    Ok(())
}

/// Sign a move record with the appropriate key
#[cfg(feature = "iroh")]
pub async fn sign_move_record(iroh_ctx: &IrohCtx, move_record: &mut MoveRecord) -> Result<()> {
    match iroh_ctx.get_ed25519_keypair().await {
        Ok(keypair) => {
            // Sign with the keypair
            move_record.sign(&keypair);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to get keypair for signing: {}", e);
            // Try fallback signing method
            let record_bytes = move_record.to_bytes();
            match iroh_ctx.sign_data(&record_bytes).await {
                Ok((signature, signer)) => {
                    move_record.signature = Some(signature);
                    move_record.signer = Some(signer);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("Failed to sign move record: {}", e);
                    Err(e)
                }
            }
        }
    }
}
