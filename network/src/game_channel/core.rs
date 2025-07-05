// SPDX-License-Identifier: MIT OR Apache-2.0

//! Core GameChannel initialization and basic functionality

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use p2pgo_core::GameState;
use crate::{GameId, NodeContext};
use crate::blob_store::MoveChain;
use super::GameChannel;
use libp2p::PeerId;
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

/// Create a new game channel
pub fn new(game_id: GameId, initial_state: GameState) -> GameChannel {
    let _span = tracing::info_span!("network.game_channel", "GameChannel::new").entered();
    
    // Create a broadcast channel for events with buffer size 100
    let (events_tx, _) = broadcast::channel(100);
    
    // Create move chain
    let move_chain = MoveChain::new(game_id.clone());
    
    GameChannel {
        game_id,
        move_chain: Arc::new(RwLock::new(move_chain)),
        events_tx,
        latest_state: Arc::new(RwLock::new(Some(initial_state))),
        last_snapshot_time: Arc::new(RwLock::new(std::time::Instant::now())),
        moves_since_snapshot: Arc::new(RwLock::new(0)),
        snapshot_dir: Arc::new(RwLock::new(None)),
        node_ctx: None,
        peer_connections: Arc::new(RwLock::new(HashMap::new())),
        _message_handler_task: None,
        processed_sequences: Arc::new(Mutex::new(VecDeque::new())),
        last_sent_index: Arc::new(RwLock::new(None)),
        last_sent_time: Arc::new(RwLock::new(None)),
        sync_requested: Arc::new(RwLock::new(false)),
    }
}

/// Create a new game channel with P2P context for network synchronization
use anyhow::Result;

#[tracing::instrument(level = "debug", skip(node_ctx))]
pub async fn with_p2p(game_id: GameId, initial_state: GameState, node_ctx: Arc<NodeContext>) -> Result<GameChannel> {
    tracing::info!("Creating GameChannel with P2P for game {}", game_id);
    
    // Create the basic channel
    let mut channel = new(game_id.clone(), initial_state.clone());
    
    // Set the P2P context
    channel.node_ctx = Some(node_ctx.clone());
    
    // Start connection handler that will handle both incoming and outgoing connections
    let peer_connections = channel.peer_connections.clone();
    let events_tx = channel.events_tx.clone();
    let processed_sequences = channel.processed_sequences.clone();
    let move_chain = channel.move_chain.clone();
    let latest_state = channel.latest_state.clone();
    let game_id_for_task = game_id.clone();
    
    let connection_task = tokio::spawn(async move {
        tracing::info!("Starting connection handler for game: {}", game_id_for_task);
        
        // TODO: Implement libp2p message handling
        // This will receive messages from the libp2p swarm and process them
        tracing::info!("P2P message handler started for game: {}", game_id_for_task);
        
        // For now, just keep the task alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });
    
    channel._message_handler_task = Some(connection_task);
    
    // TODO: Subscribe to libp2p gossipsub topic for this game
    // if let Err(e) = networking::subscribe_to_game_topic(&mut channel).await {
    //     tracing::warn!("Failed to subscribe to gossip topic: {}", e);
    // }
    
    tracing::info!("Successfully created game channel with P2P for game: {}", game_id);
    
    Ok(channel)
}
