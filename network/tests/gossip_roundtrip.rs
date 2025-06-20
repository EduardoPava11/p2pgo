// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test gossip roundtrip functionality for Go moves

use std::sync::Arc;
use anyhow::Result;
use p2pgo_core::{Coord, Move, GameState, GameEvent};
use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx};
use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

#[cfg(feature = "iroh")]
#[tokio::test]
async fn test_gossip_roundtrip() -> Result<()> {
    // Create two separate Iroh contexts
    let iroh_ctx1 = Arc::new(IrohCtx::new().await?);
    let iroh_ctx2 = Arc::new(IrohCtx::new().await?);
    
    // Create a unique game ID for this test
    let game_id = format!("gossip-test-{}", Uuid::new_v4());
    let board_size = 9;
    let initial_state = GameState::new(board_size);
    
    // Create game channels for both contexts with the same game ID
    let channel1 = GameChannel::with_iroh(
        game_id.clone(),
        initial_state.clone(),
        iroh_ctx1.clone()
    ).await?;
    
    // Connect the second context to the first one
    let ticket = iroh_ctx1.ticket().await?;
    iroh_ctx2.connect_by_ticket(&ticket).await?;
    
    // Now create the second channel with the same game ID
    let channel2 = GameChannel::with_iroh(
        game_id.clone(),
        initial_state.clone(),
        iroh_ctx2.clone()
    ).await?;
    
    // Subscribe to events from both channels
    let mut events1 = channel1.subscribe();
    let mut events2 = channel2.subscribe();
    
    // Clear any initial events
    while let Ok(_) = events1.try_recv() {}
    while let Ok(_) = events2.try_recv() {}
    
    // Wait a bit for subscription setup
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Player 1 makes a move
    let mv = Move::Place(Coord::new(4, 4));
    channel1.send_move(mv.clone()).await?;
    
    // Wait a bit for move propagation
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Check if player 2 received the move event
    let mut received_move = false;
    while let Ok(event) = events2.try_recv() {
        if let GameEvent::MoveMade { mv: received_mv, by: _ } = event {
            if received_mv == mv {
                received_move = true;
                break;
            }
        }
    }
    
    assert!(received_move, "Player 2 should have received the move via gossip");
    
    // Player 2 makes a move
    let mv2 = Move::Place(Coord::new(5, 5));
    channel2.send_move(mv2.clone()).await?;
    
    // Wait a bit for move propagation
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Check if player 1 received the move event
    let mut received_move2 = false;
    while let Ok(event) = events1.try_recv() {
        if let GameEvent::MoveMade { mv: received_mv, by: _ } = event {
            if received_mv == mv2 {
                received_move2 = true;
                break;
            }
        }
    }
    
    assert!(received_move2, "Player 1 should have received the move via gossip");
    
    Ok(())
}
