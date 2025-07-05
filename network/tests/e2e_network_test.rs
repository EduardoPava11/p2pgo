// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end network test for two players with score calculation

use anyhow::Result;
use p2pgo_core::{Color, Coord, GameEvent, GameState, Move};
use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx, lobby::Lobby};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

#[cfg(feature = "iroh")]
#[tokio::test]
async fn test_two_player_game_with_score() -> Result<()> {
    // Create two separate Iroh contexts
    let iroh_ctx1 = Arc::new(IrohCtx::new().await?);
    let iroh_ctx2 = Arc::new(IrohCtx::new().await?);

    // Connect the contexts
    let ticket = iroh_ctx1.ticket().await?;
    iroh_ctx2.connect_by_ticket(&ticket).await?;

    // Create a lobby for hosting/joining games
    let lobby1 = Lobby::new();
    let lobby2 = Lobby::new();

    // Player 1 creates a game
    let board_size = 9;
    let game_id = lobby1
        .create_game(Some("Player1".to_string()), board_size, false)
        .await?;

    // Advertise the game
    iroh_ctx1.advertise_game(&game_id, board_size).await?;

    // Get the game channel for player 1
    let channel1 = lobby1.get_game_channel(&game_id).await?;

    // Player 2 joins the game
    let channel2 = lobby2.get_game_channel(&game_id).await?;

    // Subscribe to events from both channels
    let mut events1 = channel1.subscribe();
    let mut events2 = channel2.subscribe();

    // Wait a bit for setup
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Play a small game (black plays 3 moves, white plays 2)
    // Black plays at 2,2
    channel1.send_move(Move::Place(Coord::new(2, 2))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // White plays at 3,3
    channel2.send_move(Move::Place(Coord::new(3, 3))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Black plays at 2,3
    channel1.send_move(Move::Place(Coord::new(2, 3))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // White plays at 3,2
    channel2.send_move(Move::Place(Coord::new(3, 2))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Black plays at 2,4
    channel1.send_move(Move::Place(Coord::new(2, 4))).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Both players pass to end the game
    channel2.send_move(Move::Pass).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    channel1.send_move(Move::Pass).await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Get final game state from player 1
    let game_state = channel1.get_latest_state().await.unwrap();

    // Verify game is over
    assert!(
        game_state.is_game_over(),
        "Game should be over after two passes"
    );

    // Calculate score
    let komi = 5.5; // For 9x9
    let dead_stones = HashSet::new(); // No dead stones in this simple game

    let score_proof = p2pgo_core::scoring::calculate_final_score(
        &game_state,
        komi,
        p2pgo_core::value_labeller::ScoringMethod::Territory,
        &dead_stones,
    );

    // Verify scoring works
    assert!(
        score_proof.territory_black > 0,
        "Black should have territory"
    );
    assert!(
        score_proof.territory_white > 0,
        "White should have territory"
    );

    // Get total number of moves
    let moves = channel1.get_all_moves().await;
    assert_eq!(moves.len(), 7, "There should be 7 moves in total");

    // Check if player 2 has the same number of moves (verifies sync)
    let moves2 = channel2.get_all_moves().await;
    assert_eq!(
        moves.len(),
        moves2.len(),
        "Both players should have the same number of moves"
    );

    Ok(())
}
