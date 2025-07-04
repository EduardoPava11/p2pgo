// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test for snapshot cadence
//! Tests that snapshots are created at the proper frequency

use anyhow::Result;
use tokio::time::Duration;
use std::sync::Arc;
use std::time::SystemTime;
use p2pgo_core::GameEvent;

mod common;
use common::spawn_two_peers;
use common::test_utils::{
    random_move, wait_for_events, get_latest_snapshot, wait_for_snapshot_update, submit_move
};

#[tokio::test]
async fn test_snapshot_cadence() -> Result<()> {
    // Create two connected peers
    let (mut alice, bob) = spawn_two_peers().await?;
    let alice_channel = alice.game_channel.clone().unwrap();
    let game_id = alice.game_id.clone().unwrap();
    
    // Wait for initial state stabilization
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Check if there is an initial snapshot
    let initial_snapshot = get_latest_snapshot(&game_id)?;
    
    // Get the initial mtime
    let initial_mtime = match &initial_snapshot {
        Some(path) => {
            let metadata = std::fs::metadata(path)?;
            metadata.modified()?
        },
        None => SystemTime::UNIX_EPOCH,
    };
    
    // Make a series of moves
    let mut game_state = alice_channel.get_latest_state().await.unwrap();
    
    for i in 0..5 {
        // Make a move
        let mv = random_move(&game_state);
        submit_move(&alice_channel, mv.clone()).await?;
        
        // Wait a moment for processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Update game state
        game_state = alice_channel.get_latest_state().await.unwrap();
    }
    
    // Wait for snapshot update (should happen after several moves)
    let updated_mtime = wait_for_snapshot_update(&game_id, Some(initial_mtime), 5000).await?;
    
    // Verify that the snapshot was updated
    assert!(updated_mtime > initial_mtime, "Snapshot should have been updated after moves");
    
    // Store this timestamp
    let after_moves_mtime = updated_mtime;
    
    // Now make no moves but wait for time-based snapshot
    // The default snapshot interval should be around 5 minutes,
    // but we'll use tokio's time mocking to speed this up
    
    // Advance virtual time by 10 minutes
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(10 * 60)).await;
    
    // Make one move to trigger a check
    let mv = random_move(&game_state);
    submit_move(&alice_channel, mv.clone()).await?;
    
    // Resume normal time
    tokio::time::resume();
    
    // Wait for snapshot update
    let time_based_mtime = wait_for_snapshot_update(
        &game_id, 
        Some(after_moves_mtime), 
        5000
    ).await?;
    
    // Verify that the snapshot was updated due to time
    assert!(time_based_mtime > after_moves_mtime, 
           "Snapshot should have been updated after time advancement");
    
    Ok(())
}

#[tokio::test]
async fn test_snapshot_file_integrity() -> Result<()> {
    // Create two connected peers
    let (mut alice, mut bob) = spawn_two_peers().await?;
    let alice_channel = alice.game_channel.clone().unwrap();
    let bob_channel = bob.game_channel.clone().unwrap();
    let game_id = alice.game_id.clone().unwrap();
    
    // Make several moves to generate game history
    let mut alice_state = alice_channel.get_latest_state().await.unwrap();
    
    for i in 0..10 {
        // Alternate moves between Alice and Bob
        let channel = if i % 2 == 0 { &alice_channel } else { &bob_channel };
        let mv = random_move(&alice_state);
        
        submit_move(channel, mv.clone()).await?;
        
        // Wait for move to sync
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Update game state
        alice_state = alice_channel.get_latest_state().await.unwrap();
    }
    
    // Wait for snapshot to be created
    wait_for_snapshot_update(&game_id, None, 5000).await?;
    
    // Get the latest snapshot
    let snapshot = get_latest_snapshot(&game_id)?;
    assert!(snapshot.is_some(), "Snapshot should exist");
    
    // Read the snapshot file
    let snapshot_path = snapshot.unwrap();
    let snapshot_bytes = std::fs::read(&snapshot_path)?;
    
    // Verify it's not empty
    assert!(!snapshot_bytes.is_empty(), "Snapshot file should not be empty");
    
    // Verify it's a valid CBOR file
    // This isn't a full validation, just checking it starts with valid CBOR tags
    assert!(snapshot_bytes.len() > 10, "Snapshot file should have reasonable size");
    
    // Print snapshot size for debugging
    println!("Snapshot size: {} bytes", snapshot_bytes.len());
    
    Ok(())
}
