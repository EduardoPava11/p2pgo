// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test to verify duplicate moves are properly deduplicated

use anyhow::Result;
use p2pgo_core::{Color, Coord, GameEvent, GameState, Move};
use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx, lobby::Lobby};
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

/// Set up two test peers with their own IrohCtx instances
async fn setup_test_peers() -> Result<(Arc<IrohCtx>, Arc<IrohCtx>)> {
    // Create two separate Iroh contexts (Alice and Bob)
    let alice_ctx = Arc::new(IrohCtx::new().await?);
    let bob_ctx = Arc::new(IrohCtx::new().await?);

    // Connect the contexts
    let ticket = alice_ctx.ticket().await?;
    bob_ctx.connect_by_ticket(&ticket).await?;

    // Give some time for connection to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((alice_ctx, bob_ctx))
}

#[cfg(feature = "iroh")]
#[tokio::test]
async fn test_duplicate_move_deduplication() -> Result<()> {
    // Create two connected peers: Alice and Bob
    let (alice_ctx, bob_ctx) = setup_test_peers().await?;

    // Create lobbies for hosting/joining games
    let alice_lobby = Lobby::new();
    let bob_lobby = Lobby::new();

    // Alice creates a game
    let board_size = 9;
    let game_id = alice_lobby
        .create_game(Some("Alice".to_string()), board_size, false)
        .await?;

    // Alice advertises the game
    alice_ctx.advertise_game(&game_id, board_size).await?;

    // Both get their channels
    let alice_channel = alice_lobby.get_game_channel(&game_id).await?;
    let bob_channel = bob_lobby.get_game_channel(&game_id).await?;

    // Subscribe to events
    let mut alice_events = alice_channel.subscribe();
    let mut bob_events = bob_channel.subscribe();

    // Setup connection to GameChannel on both peers
    alice_channel.connect_iroh(alice_ctx.clone()).await?;
    bob_channel.connect_iroh(bob_ctx.clone()).await?;

    // Wait for connections to be established
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Get initial hash to compare after duplicate moves
    let initial_state = bob_channel.get_latest_state().await?.unwrap();
    let initial_hash = initial_state.hash();
    let initial_move_count = bob_channel.get_all_moves().await.len();

    // Alice plays a move
    let test_move = Move::Place(Coord::new(4, 4));
    alice_channel.send_move(test_move.clone()).await?;

    // Wait for move to be propagated
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify Bob received the move
    let bob_moves = bob_channel.get_all_moves().await;
    assert_eq!(
        bob_moves.len(),
        initial_move_count + 1,
        "Bob should have received one move"
    );
    assert_eq!(
        bob_moves.last().unwrap(),
        &test_move,
        "Bob should have received the correct move"
    );

    // Save the board hash after first move delivery
    let state_after_first_delivery = bob_channel.get_latest_state().await?.unwrap();
    let hash_after_first_delivery = state_after_first_delivery.hash();
    assert_ne!(
        hash_after_first_delivery, initial_hash,
        "Game state should have changed after move"
    );

    // Now artificially deliver the same move again by forcing a direct message from Alice to Bob
    // This simulates a duplicate delivery over the network
    // Get the move record that was already processed
    let all_move_records = bob_channel.get_all_move_records().await;
    let duplicate_record = all_move_records.last().unwrap().clone();

    // Directly inject the duplicate move again
    // This simulates what happens when a move is delivered twice through the network
    bob_channel
        .handle_duplicate_move_test(duplicate_record)
        .await?;

    // Wait for potential processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify Bob still has the same number of moves (no duplicates)
    let bob_moves_after_duplicate = bob_channel.get_all_moves().await;
    assert_eq!(
        bob_moves_after_duplicate.len(),
        initial_move_count + 1,
        "Bob should still have the same number of moves after duplicate delivery"
    );

    // Verify the hash hasn't changed (no duplicate application)
    let state_after_duplicate = bob_channel.get_latest_state().await?.unwrap();
    let hash_after_duplicate = state_after_duplicate.hash();
    assert_eq!(
        hash_after_duplicate, hash_after_first_delivery,
        "Game state should not have changed after duplicate move"
    );

    // Also verify that the processed_sequences HashSet was used for deduplication
    // by checking the debug logs or a custom test method that exposes this information
    assert!(
        bob_channel.was_move_deduplicated().await,
        "Move should have been deduplicated via processed_sequences"
    );

    Ok(())
}
