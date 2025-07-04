// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test for move deduplication
//! Tests that moves are properly deduplicated when received multiple times

use anyhow::Result;
use tokio::time::Duration;
use std::sync::Arc;
use p2pgo_core::Move;

mod common;
use common::{spawn_two_peers, TestPeer};
use common::test_utils::{random_move, wait_for_events, was_move_deduplicated, get_processed_sequences_len};

#[tokio::test]
async fn test_move_deduplication() -> Result<()> {
    // Create two connected peers with a game
    let (mut alice, bob) = spawn_two_peers().await?;
    let alice_channel = alice.game_channel.clone().unwrap();
    let bob_channel = bob.game_channel.clone().unwrap();

    // Make a move from Alice
    let game_state = alice_channel.get_latest_state().await.unwrap();
    let mv = random_move(&game_state);
    
    // Submit the move once
    alice_channel.submit_move(mv.clone()).await?;
    
    // Wait for Bob to receive the move
    let _events = wait_for_events(&bob_channel, "move", 1, 5000).await?;
    
    // Get the initial deduplication queue size on Bob
    let initial_deque_size = get_processed_sequences_len(&bob_channel).await?;
    
    // Now send the same move multiple times directly
    for i in 0..5 {
        // Submit duplicate move
        let _ = alice_channel.replicate_last_move().await;
        tracing::info!("Sent duplicate move {}", i + 1);
    }
    
    // Wait a moment for the duplicate moves to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify the move was properly deduplicated
    let was_deduplicated = was_move_deduplicated(&bob_channel, &mv, 1000).await?;
    assert!(was_deduplicated, "Move should have been deduplicated");
    
    // Verify the deduplication queue size increased
    let final_deque_size = get_processed_sequences_len(&bob_channel).await?;
    assert!(final_deque_size > initial_deque_size, 
           "Deduplication queue should have grown");
    
    // Verify the deduplication queue stays under limit (8192)
    assert!(final_deque_size <= 8192, "Deduplication queue should stay under limit");

    Ok(())
}

#[tokio::test]
async fn test_dedup_queue_limit() -> Result<()> {
    // Create two connected peers with a game
    let (mut alice, bob) = spawn_two_peers().await?;
    let alice_channel = alice.game_channel.clone().unwrap();
    let bob_channel = bob.game_channel.clone().unwrap();
    
    // Get the initial game state
    let mut game_state = alice_channel.get_latest_state().await.unwrap();
    
    // Make a large number of moves to fill the deduplication queue
    // Let's do 50 moves as it's enough to test but not too many
    for i in 0..50 {
        let mv = random_move(&game_state);
        alice_channel.submit_move(mv.clone()).await?;
        
        // Wait for Bob to receive the move
        let _events = wait_for_events(&bob_channel, "move", i+1, 2000).await?;
        
        // Update game state
        game_state = alice_channel.get_latest_state().await.unwrap();
        
        // Send duplicate moves to fill the queue
        for _ in 0..10 {
            let _ = alice_channel.replicate_last_move().await;
        }
        
        // Short pause to allow processing
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Check that the deduplication queue size stays reasonable
    let queue_size = get_processed_sequences_len(&bob_channel).await?;
    assert!(queue_size <= 8192, "Deduplication queue should stay under limit (got {})", queue_size);
    
    Ok(())
}
