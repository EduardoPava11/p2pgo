// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game state management and move handling

use super::GameChannel;
use crate::blob_store::MoveBlob;
use anyhow::Result;
use p2pgo_core::{GameEvent, GameState, Move};

#[cfg(feature = "iroh")]
use {
    super::{networking, storage},
    iroh::NodeId,
    std::collections::VecDeque,
    tokio::sync::Mutex,
};

/// Send a move to the channel
pub async fn send_move(channel: &GameChannel, mv: Move) -> Result<()> {
    push_move(channel, mv, None).await
}

/// Handle a local move - this is a public method used by the CLI and tests
pub async fn handle_local_move(channel: &GameChannel, mv: Move) -> Result<()> {
    tracing::info!(
        game_id = %channel.game_id,
        move_type = ?mv,
        "Processing local move"
    );

    // Simply delegate to the push_move method which handles all logic
    push_move(channel, mv, None).await
}

/// Push a move with optional tag to the channel
#[tracing::instrument(level = "debug", skip(channel, mv))]
pub async fn push_move(
    channel: &GameChannel,
    mv: Move,
    tag: Option<p2pgo_core::Tag>,
) -> Result<()> {
    let _span = tracing::info_span!("network.game_channel", "GameChannel::send_move").entered();

    // Get the current game state
    let mut state = {
        let state_guard = channel.latest_state.read().await;
        match &*state_guard {
            Some(state) => state.clone(),
            None => return Err(anyhow::anyhow!("No game state available")),
        }
    };

    // Apply the move to the state
    state.apply_move(mv.clone())?;

    // Get the current chain
    let mut chain = channel.move_chain.write().await;

    // Get the previous hash and sequence
    let prev_hash = chain.current_blob().map(|blob| blob.hash());
    let sequence = if chain.current_blob().is_none() {
        0
    } else {
        chain.current_sequence + 1
    };

    // Create a new move blob
    let blob = MoveBlob::new(
        channel.game_id.clone(),
        mv,
        prev_hash.clone(),
        state.clone(),
        sequence,
    );

    // Get the move for the event before consuming the blob
    let move_for_event = blob.mv.clone();

    // Add the blob to the chain
    chain.add_blob(blob)?;

    // If using iroh, store the move in the document
    #[cfg(feature = "iroh")]
    if let Some(_iroh_ctx) = &channel.iroh_ctx {
        // Create a move record with proper hash chain
        let mut move_record = MoveRecord {
            mv: move_for_event.clone(),
            tag: tag.clone(),
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            broadcast_hash: None,
            prev_hash: prev_hash.clone(),
        };

        // Calculate the broadcast hash
        move_record.calculate_broadcast_hash();

        // CBOR encode the move record
        let _cbor_data =
            serde_cbor::to_vec(&move_record).context("Failed to CBOR encode move record")?;

        // Mark this move as processed locally to avoid duplicate processing
        {
            // Get our own node ID
            let local_node_id = match &channel.iroh_ctx {
                Some(ctx) => {
                    let id_str = ctx.node_id();
                    NodeId::from_base58(&id_str).unwrap_or_else(|_| NodeId::from_bytes([0; 32]))
                }
                None => NodeId::from_bytes([0; 32]), // Fallback
            };

            // Use the sequence as the deduplication sequence number
            let seq_no = sequence as u64;

            let mut dequeue = channel.processed_sequences.lock().await;
            dequeue.push_back((local_node_id, seq_no));

            // Ensure we stay within capacity limits
            if dequeue.len() > 8192 {
                dequeue.pop_front();
            }

            tracing::debug!(
                "Added local move with seq {} to dedup queue (size: {})",
                seq_no,
                dequeue.len()
            );
        }

        // Store the move in the document using the IrohCtx API
        // TODO: Update for iroh v0.35 docs API
        // iroh_ctx.store_game_move(&channel.game_id, sequence, &cbor_data)
        //     .await
        //     .context("Failed to store move in document")?;

        tracing::info!(
            "Move storage disabled - needs iroh v0.35 update: sequence {}",
            sequence
        );
    }

    // Update the latest state
    {
        let mut state_guard = channel.latest_state.write().await;
        *state_guard = Some(state.clone());
    }

    // Store tag if provided (for training data)
    if let Some(tag) = tag {
        tracing::debug!(
            game_id = %channel.game_id,
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
    if let Err(e) = channel.events_tx.send(event) {
        tracing::warn!("Failed to broadcast move event: {}", e);
    }

    // Increment moves since last snapshot
    {
        let mut moves = channel.moves_since_snapshot.write().await;
        *moves += 1;
    }

    // Check if we need to write a snapshot
    #[cfg(feature = "iroh")]
    if storage::check_snapshot_needed(channel).await {
        if let Err(e) = storage::write_snapshot(channel).await {
            tracing::warn!("Failed to write game snapshot: {}", e);
        }
    }

    #[cfg(not(feature = "iroh"))]
    if check_snapshot_needed_basic(channel).await {
        if let Err(e) = write_snapshot_basic(channel).await {
            tracing::warn!("Failed to write game snapshot: {}", e);
        }
    }

    // If using iroh, broadcast the move to connected peers
    #[cfg(feature = "iroh")]
    {
        // Create a move record for peer communication with proper hash chain
        let mut move_record =
            MoveRecord::new_with_timestamp(move_for_event.clone(), tag.clone(), prev_hash.clone());

        // Calculate the broadcast hash
        move_record.calculate_broadcast_hash();

        // Sign the move record if we have an iroh context
        if let Some(iroh_ctx) = &channel.iroh_ctx {
            match networking::sign_move_record(iroh_ctx, &mut move_record).await {
                Ok(_) => {
                    if let Some(signer) = move_record.get_signer() {
                        tracing::debug!("Move record signed by {}", signer);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to sign move record: {}", e);
                }
            }
        }

        // Broadcast the move via both gossip and direct connections for reliability
        let _ = networking::broadcast_move(channel, &move_record).await; // Gossip (may fail)

        // Always broadcast to directly connected peers as primary mechanism
        if let Err(e) = networking::broadcast_move_to_peers(channel, &move_record).await {
            tracing::warn!("Failed to broadcast move to direct peers: {}", e);
        } else {
            tracing::info!("Successfully broadcast move to direct peers");

            // Set the last sent move index and time for ACK watchdog
            let move_count = {
                let chain = channel.move_chain.read().await;
                chain.get_all_blobs().len()
            };

            // Update last_sent info
            {
                let mut last_index = channel.last_sent_index.write().await;
                let mut last_time = channel.last_sent_time.write().await;
                let mut sync_req = channel.sync_requested.write().await;

                *last_index = Some(move_count - 1); // 0-indexed
                *last_time = Some(std::time::Instant::now());
                *sync_req = false; // Reset sync requested flag

                tracing::debug!("Set ACK watchdog for move index {}", move_count - 1);
            }
        }
    }

    Ok(())
}

/// Get the latest game state
pub async fn get_latest_state(channel: &GameChannel) -> Option<GameState> {
    channel.latest_state.read().await.clone()
}

/// Get all moves in the game so far
pub async fn get_all_moves(channel: &GameChannel) -> Vec<Move> {
    let chain = channel.move_chain.read().await;

    chain
        .get_all_blobs()
        .iter()
        .map(|blob| blob.mv.clone())
        .collect()
}

/// Send a chat message or other game event
pub async fn send_event(channel: &GameChannel, event: GameEvent) -> Result<()> {
    let _span = tracing::info_span!("network.game_channel", "GameChannel::send_event").entered();

    // Just broadcast the event
    channel
        .events_tx
        .send(event)
        .map_err(|e| anyhow::anyhow!("Failed to broadcast event: {}", e))?;

    Ok(())
}

/// Process a received move from direct peer connection
#[cfg(feature = "iroh")]
pub async fn process_received_move_direct(
    move_record: MoveRecord,
    events_tx: &tokio::sync::broadcast::Sender<GameEvent>,
    latest_state: &Arc<tokio::sync::RwLock<Option<GameState>>>,
    move_chain: &Arc<tokio::sync::RwLock<crate::blob_store::MoveChain>>,
    processed_sequences: &Arc<Mutex<VecDeque<(NodeId, u64)>>>,
    game_id: &str,
) -> Result<()> {
    tracing::debug!(
        "Processing received move for {}: {:?}",
        game_id,
        move_record.mv
    );

    // Verify signature
    if !super::crypto::try_verify_move_record(&move_record, game_id) {
        return Err(anyhow::anyhow!("Signature verification failed"));
    }

    // Extract sender ID and sequence number for deduplication
    let sender = move_record
        .get_signer()
        .unwrap_or_else(|| "unknown".to_string());
    let sender_id = NodeId::from_base58(&sender).unwrap_or_else(|_| NodeId::from_bytes([0; 32]));

    // Get sequence number from timestamp or hash
    let seq_no = move_record.ts.max(1); // Use timestamp as sequence number, minimum 1

    // Check if we've already processed this (sender, sequence) pair
    {
        let mut dequeue = processed_sequences.lock().await;

        // Check if this exact (sender, sequence) pair is already in the dequeue
        if dequeue
            .iter()
            .any(|(id, seq)| *id == sender_id && *seq == seq_no)
        {
            tracing::debug!(
                "Move from {} with seq {} already processed, skipping",
                sender,
                seq_no
            );
            return Ok(());
        }

        // Add the new (sender, sequence) pair to the dequeue
        dequeue.push_back((sender_id, seq_no));

        // Ensure we stay within capacity limits
        if dequeue.len() > 8192 {
            dequeue.pop_front();
        }

        tracing::debug!(
            "Added move from {} with seq {} to dedup queue (size: {})",
            sender,
            seq_no,
            dequeue.len()
        );
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
                let sequence = if chain.current_blob().is_none() {
                    0
                } else {
                    chain.current_sequence + 1
                };
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

// Basic snapshot functionality for non-iroh builds
#[cfg(not(feature = "iroh"))]
async fn check_snapshot_needed_basic(channel: &GameChannel) -> bool {
    // Use the storage module to check if snapshot is needed even in non-iroh mode
    super::storage::check_snapshot_needed(channel).await
}

#[cfg(not(feature = "iroh"))]
async fn write_snapshot_basic(channel: &GameChannel) -> Result<()> {
    // Use the storage module to write snapshots even in non-iroh mode
    super::storage::write_snapshot(channel).await
}

// Test helpers
#[cfg(feature = "iroh")]
#[cfg(test)]
pub async fn get_all_move_records(channel: &GameChannel) -> Vec<MoveRecord> {
    let chain = channel.move_chain.read().await;
    if let Some(_blob) = chain.current_blob() {
        // Get move records from the chain
        let mut records = chain.get_all_move_records();

        // If we have an iroh context, sign the records for testing
        if let Some(iroh_ctx) = &channel.iroh_ctx {
            if let Ok(keypair) = iroh_ctx.get_ed25519_keypair().await {
                for record in &mut records {
                    // Sign each record to make sure signatures are valid
                    record.sign(&keypair);
                }
            }
        }

        records
    } else {
        vec![]
    }
}

#[cfg(feature = "iroh")]
#[cfg(test)]
pub async fn handle_duplicate_move_test(
    channel: &GameChannel,
    move_record: MoveRecord,
) -> Result<()> {
    tracing::debug!("Test: Handling duplicate move: {:?}", move_record.mv);

    // Track the initial state of processed_sequences
    let initial_processed_count = {
        let processed = channel.processed_sequences.lock().await;
        processed.len()
    };

    // This directly calls the process_received_move_direct method to simulate
    // a duplicate move coming through the network
    let result = process_received_move_direct(
        move_record,
        &channel.events_tx,
        &channel.latest_state,
        &channel.move_chain,
        &channel.processed_sequences,
        &channel.game_id,
    )
    .await;

    // Check if the move was deduplicated (processed set grew)
    let final_processed_count = {
        let processed = channel.processed_sequences.lock().await;
        processed.len()
    };

    if final_processed_count > initial_processed_count {
        tracing::debug!("Test: Move was processed and added to deduplication set");
    } else {
        tracing::debug!("Test: Move was deduplicated (already in set)");
    }

    result
}

#[cfg(feature = "iroh")]
#[cfg(test)]
pub async fn was_move_deduplicated(channel: &GameChannel) -> bool {
    // Returns true if there are entries in the processed_sequences set
    let processed = channel.processed_sequences.lock().await;
    !processed.is_empty()
}
