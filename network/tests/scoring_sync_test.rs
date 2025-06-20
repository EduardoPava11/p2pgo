// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test the score and game finishing functionality with two connected nodes.

use std::sync::Arc;
use std::collections::HashSet;
use anyhow::Result;
use p2pgo_core::{Coord, Move, GameState, GameEvent, Color};
use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx};
use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

#[cfg(feature = "iroh")]
#[tokio::test]
async fn test_scoring_agreement_between_nodes() -> Result<()> {
    // Create two separate Iroh contexts
    let iroh_ctx1 = Arc::new(IrohCtx::new().await?);
    let iroh_ctx2 = Arc::new(IrohCtx::new().await?);
    
    // Create a unique game ID for this test
    let game_id = format!("score-test-{}", Uuid::new_v4());
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
    
    // Play a series of moves to create a simple game position
    let moves = [
        (Move::Place(Coord::new(2, 2)), &channel1),  // Black at 2,2
        (Move::Place(Coord::new(3, 3)), &channel2),  // White at 3,3
        (Move::Place(Coord::new(2, 3)), &channel1),  // Black at 2,3
        (Move::Place(Coord::new(3, 2)), &channel2),  // White at 3,2
        (Move::Place(Coord::new(2, 4)), &channel1),  // Black at 2,4
        (Move::Pass, &channel2),                    // White passes
        (Move::Pass, &channel1),                    // Black passes
    ];
    
    // Execute all moves with small delay between them
    for (mv, channel) in &moves {
        channel.send_move(mv.clone()).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Get game states from both players
    let state1 = channel1.get_latest_state().await.expect("Channel 1 should have a game state");
    let state2 = channel2.get_latest_state().await.expect("Channel 2 should have a game state");
    
    // Check both states match
    assert_eq!(state1.moves.len(), state2.moves.len(), "Both players should have the same number of moves");
    
    // Verify game is over
    assert!(state1.is_game_over(), "Game should be over after two passes");
    assert!(state2.is_game_over(), "Game should be over after two passes");
    
    // Calculate score without any dead stones
    let komi = 5.5; // Standard for 9x9
    let empty_dead_stones = HashSet::new();
    
    let score1 = p2pgo_core::scoring::calculate_final_score(
        &state1,
        komi,
        p2pgo_core::value_labeller::ScoringMethod::Territory,
        &empty_dead_stones
    );
    
    let score2 = p2pgo_core::scoring::calculate_final_score(
        &state2,
        komi,
        p2pgo_core::value_labeller::ScoringMethod::Territory,
        &empty_dead_stones
    );
    
    // Verify score proofs match
    assert_eq!(score1.final_score, score2.final_score, "Final scores should match");
    assert_eq!(score1.territory_black, score2.territory_black, "Black territory should match");
    assert_eq!(score1.territory_white, score2.territory_white, "White territory should match");
    
    // Simulate accepting the score and sharing with opponent via iroh
    channel1.send_event(GameEvent::GameFinished {
        black_score: score1.territory_black as f32,
        white_score: score1.territory_white as f32 + komi as f32,
    }).await?;
    
    // Wait for event propagation
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Check if player 2 received the game finished event
    let mut received_game_finished = false;
    while let Ok(event) = events2.try_recv() {
        if let GameEvent::GameFinished { .. } = event {
            received_game_finished = true;
            break;
        }
    }
    
    assert!(received_game_finished, "Player 2 should have received game finished event");
    
    Ok(())
}
