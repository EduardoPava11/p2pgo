// SPDX-License-Identifier: MIT OR Apache-2.0

//! Synchronization and ACK timeout handling

use anyhow::Result;
use super::GameChannel;

#[cfg(feature = "iroh")]
use {
    iroh::endpoint::Connection,
    super::networking,
};

/// Check if we need to request a sync due to missing ACKs
#[cfg(feature = "iroh")]
pub async fn check_sync_timeouts(channel: &GameChannel) -> Result<()> {
    // Get the current state of the watchdog
    let (should_request_sync, last_index) = {
        let last_index_guard = channel.last_sent_index.read().await;
        let last_time_guard = channel.last_sent_time.read().await;
        let sync_requested_guard = channel.sync_requested.read().await;
        
        // If no move has been sent or sync already requested, do nothing
        if last_index_guard.is_none() || last_time_guard.is_none() || *sync_requested_guard {
            return Ok(());
        }
        
        let last_index = last_index_guard.unwrap();
        let last_time = last_time_guard.unwrap();
        
        // Check if the timeout has elapsed (3 seconds)
        let elapsed = last_time.elapsed();
        let timeout = std::time::Duration::from_secs(3);
        
        if elapsed > timeout {
            tracing::warn!("ACK timeout for move index {}. Elapsed: {:?}, Threshold: {:?}", 
                last_index, elapsed, timeout);
            
            // Check if the move chain length has advanced (indicating the move was actually processed)
            let current_move_count = {
                let chain = channel.move_chain.read().await;
                chain.get_all_blobs().len()
            };
            
            // If the move count has advanced beyond our last sent index, we don't need to request sync
            if current_move_count > last_index + 1 {
                tracing::debug!("Move chain already advanced beyond last_sent_index, no sync needed");
                return Ok(());
            }
            
            (true, last_index)
        } else {
            (false, 0) // No need to request sync
        }
    };
    
    // Request sync if needed
    if should_request_sync {
        // Mark that we've requested a sync for this move
        {
            let mut sync_requested = channel.sync_requested.write().await;
            *sync_requested = true;
        }
        
        tracing::info!("Requesting sync due to missing ACK for move index {}", last_index);
        request_sync(channel).await?;
    }
    
    Ok(())
}

/// Request a sync of game state from peers
#[cfg(feature = "iroh")]
pub async fn request_sync(channel: &GameChannel) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        tracing::info!("Requesting game sync for {}", channel.game_id);
        
        // Create a sync request message
        let sync_req = SyncRequest {
            game_id: channel.game_id.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        
        // Serialize the sync request
        let message = serde_json::to_string(&sync_req)?;
        
        // Broadcast to all connected peers
        let connections = channel.peer_connections.read().await;
        if connections.is_empty() {
            tracing::warn!("No peers to request sync from");
            return Ok(());
        }
        
        for (i, connection) in connections.iter().enumerate() {
            match connection.open_uni().await {
                Ok(mut send_stream) => {
                    // Send the sync request message
                    if let Err(e) = send_stream.write_all(message.as_bytes()).await {
                        tracing::error!("Failed to send sync request to peer {}: {}", i, e);
                    } else if let Err(e) = send_stream.write_all(b"\n").await {
                        tracing::error!("Failed to send delimiter to peer {}: {}", i, e);
                    } else {
                        // Close the stream
                        if let Err(e) = send_stream.finish() {
                            tracing::error!("Failed to finish stream to peer {}: {}", i, e);
                        } else {
                            tracing::debug!("Successfully sent sync request to peer {}", i);
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to open stream to peer {}: {}", i, e);
                }
            }
        }
        
        // Try via gossip as well
        if let Err(e) = broadcast_sync_request(channel, &sync_req).await {
            tracing::warn!("Failed to broadcast sync request via gossip: {}", e);
        }
        
        tracing::info!("Sync request sent to {} peers", connections.len());
    }
    
    Ok(())
}

/// Broadcast a sync request via gossip
#[cfg(feature = "iroh")]
async fn broadcast_sync_request(channel: &GameChannel, sync_req: &SyncRequest) -> Result<()> {
    if let Some(iroh_ctx) = &channel.iroh_ctx {
        // Serialize the sync request to JSON for broadcast
        let json_data = serde_json::to_vec(sync_req)?;
        
        tracing::debug!("Broadcasting sync request via gossip for game {}", channel.game_id);
        
        // Try gossip broadcast using the same topic as game moves
        if let Err(e) = iroh_ctx.broadcast_to_game_topic(&channel.game_id, &json_data).await {
            tracing::warn!("Failed to broadcast sync request via gossip: {}", e);
            return Err(e);
        }
        
        tracing::info!("Successfully broadcast sync request via gossip");
    }
    
    Ok(())
}

/// Handle an incoming sync request
#[cfg(feature = "iroh")]
pub async fn handle_sync_request(channel: &GameChannel, sync_req: SyncRequest, from_connection: Option<&Connection>) -> Result<()> {
    tracing::info!("Received sync request for game {}", sync_req.game_id);
    
    // Verify this is for our game
    if sync_req.game_id != channel.game_id {
        tracing::warn!("Ignoring sync request for different game: {} (ours: {})", 
            sync_req.game_id, channel.game_id);
        return Ok(());
    }
    
    // Get our current game state
    let state = {
        let state_guard = channel.latest_state.read().await;
        match &*state_guard {
            Some(s) => s.clone(),
            None => {
                tracing::warn!("No game state available to respond to sync request");
                return Ok(());
            }
        }
    };
    
    // Get all moves
    let moves = super::state::get_all_moves(channel).await;
    
    // Create a sync response
    let response = SyncResponse {
        game_id: channel.game_id.clone(),
        moves,
        state,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    
    // Send the response
    if let Some(conn) = from_connection {
        // Reply directly to the requesting peer
        match conn.open_uni().await {
            Ok(mut send_stream) => {
                let message = serde_json::to_string(&response)?;
                
                if let Err(e) = send_stream.write_all(message.as_bytes()).await {
                    tracing::error!("Failed to send sync response: {}", e);
                } else if let Err(e) = send_stream.write_all(b"\n").await {
                    tracing::error!("Failed to send delimiter: {}", e);
                } else {
                    // Close the stream
                    if let Err(e) = send_stream.finish() {
                        tracing::error!("Failed to finish stream: {}", e);
                    } else {
                        tracing::debug!("Successfully sent sync response");
                    }
                }
            },
            Err(e) => {
                tracing::error!("Failed to open stream for sync response: {}", e);
            }
        }
    } else {
        // Broadcast to all peers (for gossip-based requests)
        let connections = channel.peer_connections.read().await;
        let message = serde_json::to_string(&response)?;
        
        for (i, connection) in connections.iter().enumerate() {
            match connection.open_uni().await {
                Ok(mut send_stream) => {
                    if let Err(e) = send_stream.write_all(message.as_bytes()).await {
                        tracing::error!("Failed to send sync response to peer {}: {}", i, e);
                    } else if let Err(e) = send_stream.write_all(b"\n").await {
                        tracing::error!("Failed to send delimiter to peer {}: {}", i, e);
                    } else {
                        // Close the stream
                        if let Err(e) = send_stream.finish() {
                            tracing::error!("Failed to finish stream to peer {}: {}", i, e);
                        } else {
                            tracing::debug!("Successfully sent sync response to peer {}", i);
                        }
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to open stream to peer {}: {}", i, e);
                }
            }
        }
    }
    
    tracing::info!("Sync response sent for game {}", channel.game_id);
    Ok(())
}

/// Handle incoming sync response
#[cfg(feature = "iroh")]
pub async fn handle_sync_response(channel: &GameChannel, sync_resp: SyncResponse) -> Result<()> {
    tracing::info!("Processing sync response for game {}", sync_resp.game_id);
    
    // Verify this is for our game
    if sync_resp.game_id != channel.game_id {
        tracing::warn!("Ignoring sync response for different game: {} (ours: {})", 
            sync_resp.game_id, channel.game_id);
        return Ok(());
    }
    
    // Get current move count
    let current_move_count = {
        let chain = channel.move_chain.read().await;
        chain.get_all_blobs().len()
    };
    
    // If we're behind, apply the moves from the sync response
    if sync_resp.moves.len() > current_move_count {
        tracing::info!("Syncing {} new moves from response", 
            sync_resp.moves.len() - current_move_count);
        
        // Apply the new moves
        for (i, mv) in sync_resp.moves.iter().enumerate() {
            if i >= current_move_count {
                if let Err(e) = super::state::send_move(channel, mv.clone()).await {
                    tracing::error!("Failed to apply move from sync: {}", e);
                }
            }
        }
        
        tracing::info!("Sync completed, applied {} new moves", 
            sync_resp.moves.len() - current_move_count);
    } else {
        tracing::info!("No new moves in sync response");
    }
    
    // Reset the sync watchdog
    {
        let mut last_index = channel.last_sent_index.write().await;
        let mut last_time = channel.last_sent_time.write().await;
        let mut sync_req = channel.sync_requested.write().await;
        
        *last_index = None;
        *last_time = None;
        *sync_req = false;
        
        tracing::debug!("ACK watchdog reset after sync");
    }
    
    Ok(())
}

// Stubs for non-iroh builds
#[cfg(not(feature = "iroh"))]
pub async fn check_sync_timeouts(_channel: &GameChannel) -> Result<()> {
    Ok(())
}

#[cfg(not(feature = "iroh"))]
pub async fn request_sync(_channel: &GameChannel) -> Result<()> {
    Ok(())
}
