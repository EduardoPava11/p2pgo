// SPDX-License-Identifier: MIT OR Apache-2.0

//! Core GameChannel initialization and basic functionality

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use p2pgo_core::GameState;
use crate::GameId;
use crate::blob_store::MoveChain;
use super::GameChannel;

#[cfg(feature = "iroh")]
use {
    crate::iroh_endpoint::IrohCtx,
    iroh::NodeId,
    std::collections::VecDeque,
    tokio::sync::Mutex,
    super::{networking, sync},
};

/// Create a new game channel
pub fn new(game_id: GameId, initial_state: GameState) -> GameChannel {
    let _span = tracing::info_span!("network.game_channel", "GameChannel::new").entered();
    
    // Create a broadcast channel for events with buffer size 100
    let (events_tx, _) = broadcast::channel(100);
    
    // Create move chain
    let move_chain = MoveChain::new(game_id.clone());
    
    #[cfg(not(feature = "iroh"))]
    return GameChannel {
        game_id,
        move_chain: Arc::new(RwLock::new(move_chain)),
        events_tx,
        latest_state: Arc::new(RwLock::new(Some(initial_state))),
        last_snapshot_time: Arc::new(RwLock::new(std::time::Instant::now())),
        moves_since_snapshot: Arc::new(RwLock::new(0)),
        snapshot_dir: Arc::new(RwLock::new(None)),
    };
    
    #[cfg(feature = "iroh")]
    return GameChannel {
        game_id,
        move_chain: Arc::new(RwLock::new(move_chain)),
        events_tx,
        latest_state: Arc::new(RwLock::new(Some(initial_state))),
        last_snapshot_time: Arc::new(RwLock::new(std::time::Instant::now())),
        moves_since_snapshot: Arc::new(RwLock::new(0)),
        snapshot_dir: Arc::new(RwLock::new(None)),
        iroh_ctx: None,
        peer_connections: Arc::new(RwLock::new(Vec::new())),
        _connection_task: None,
        processed_sequences: Arc::new(Mutex::new(VecDeque::with_capacity(8192))),
        last_sent_index: Arc::new(RwLock::new(None)),
        last_sent_time: Arc::new(RwLock::new(None)),
        sync_requested: Arc::new(RwLock::new(false)),
    };
}

/// Create a new game channel with an Iroh context for network synchronization
#[cfg(feature = "iroh")]
#[tracing::instrument(level = "debug", skip(iroh_ctx))]
pub async fn with_iroh(game_id: GameId, initial_state: GameState, iroh_ctx: Arc<IrohCtx>) -> Result<GameChannel> {
    tracing::info!("Creating GameChannel with Iroh for game {}", game_id);
    
    // Create the basic channel
    let mut channel = new(game_id.clone(), initial_state.clone());
    
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
                if let Err(e) = networking::handle_peer_connection(
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
        
        tracing::warn!("Connection handler for game {} exited", game_id_for_task);
        Ok(())
    });
    
    channel._connection_task = Some(connection_task);
    
    // Subscribe to gossip topic for this game (best effort)
    if let Err(e) = networking::subscribe_to_game_topic(&mut channel).await {
        tracing::warn!("Failed to subscribe to gossip topic (will rely on direct connections): {}", e);
    }
    
    tracing::info!("Successfully created game channel with Iroh for game: {}", game_id);
    
    // Convert the channel to Arc and register it
    let channel_arc = std::sync::Arc::new(channel);
    super::registry::register(&game_id, &channel_arc);
    
    // Schedule regular checks for ACK timeouts
    let channel_weak = std::sync::Arc::downgrade(&channel_arc);
    let game_id_clone = game_id.clone();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        
        loop {
            interval.tick().await;
            
            if let Some(channel) = channel_weak.upgrade() {
                if let Err(e) = sync::check_sync_timeouts(&channel).await {
                    tracing::error!("Error checking sync timeouts for {}: {}", game_id_clone, e);
                }
            } else {
                // Channel has been dropped, exit the loop
                tracing::debug!("ACK watchdog stopped for {}", game_id_clone);
                break;
            }
        }
    });
    
    Ok((*channel_arc).clone())
}
