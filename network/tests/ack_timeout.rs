// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test for the ACK watchdog
//! Tests that when an ACK is missing, the watchdog triggers a SyncRequest

use anyhow::Result;
use p2pgo_core::{GameEvent, Move};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

mod common;
use common::test_utils::{random_move, wait_for_events};
use common::{spawn_two_peers, TestPeer};

#[tokio::test]
async fn test_ack_timeout() -> Result<()> {
    // Create two connected peers with a game
    let (mut alice, bob) = spawn_two_peers().await?;
    let alice_channel = alice.game_channel.clone().unwrap();
    let bob_channel = bob.game_channel.clone().unwrap();

    // Subscribe to events on both sides
    let mut alice_rx = alice_channel.subscribe();
    let mut bob_rx = bob_channel.subscribe();

    // Make a move from Alice
    let game_state = alice_channel.get_latest_state().await.unwrap();
    let mv = random_move(&game_state);

    // Access the internal method to simulate a move without ACK
    // This is slightly hacky but necessary to test the watchdog
    // We need to intercept the ACK that would normally be sent
    alice_channel.submit_move_no_ack(mv.clone()).await?;

    // Wait for move to be received by Bob
    let _events = wait_for_events(&bob_channel, "move", 1, 5000).await?;

    // Normally Bob would send an ACK, but we're not going to let that happen
    // Alice should trigger her watchdog after around 3-5 seconds

    // Wait for the sync request
    let start_time = Instant::now();
    let mut sync_requested = false;

    // Wait up to 10 seconds for the watchdog to trigger
    while start_time.elapsed() < Duration::from_secs(10) && !sync_requested {
        match tokio::time::timeout(Duration::from_millis(500), alice_rx.recv()).await {
            Ok(Ok(event)) => {
                if let GameEvent::SyncRequested { .. } = event {
                    sync_requested = true;
                    break;
                }
            }
            _ => {}
        }
    }

    // Verify the watchdog triggered a sync request
    assert!(
        sync_requested,
        "Sync request should have been triggered by the watchdog"
    );

    // Additional test: verify that receiving an ACK properly resets the watchdog

    // Make another move
    let game_state = alice_channel.get_latest_state().await.unwrap();
    let mv2 = random_move(&game_state);

    // Submit it normally this time
    alice_channel.submit_move(mv2.clone()).await?;

    // Wait for move to be received by Bob
    let _events = wait_for_events(&bob_channel, "move", 1, 5000).await?;

    // This time the ACK should be sent automatically

    // Wait a bit to make sure no sync is triggered
    let start_time = Instant::now();
    sync_requested = false;

    // Check for 7 seconds (longer than the watchdog timeout)
    while start_time.elapsed() < Duration::from_secs(7) {
        match tokio::time::timeout(Duration::from_millis(500), alice_rx.recv()).await {
            Ok(Ok(event)) => {
                if let GameEvent::SyncRequested { .. } = event {
                    sync_requested = true;
                    break;
                }
            }
            _ => {}
        }
    }

    // Verify the watchdog did NOT trigger because the ACK was received
    assert!(
        !sync_requested,
        "No sync request should be triggered when ACK is received"
    );

    Ok(())
}

// Helper extension trait to access internal methods
trait GameChannelExt {
    async fn submit_move_no_ack(&self, mv: Move) -> Result<()>;
    async fn replicate_last_move(&self) -> Result<()>;
}

impl GameChannelExt for Arc<p2pgo_network::GameChannel> {
    async fn submit_move_no_ack(&self, mv: Move) -> Result<()> {
        // This is using reflection to access the internal methods
        // It's a bit hacky but necessary for testing

        use std::any::Any;

        // Get the concrete type as Any
        let any_ref = self as &dyn Any;

        // Try to downcast to the specific type
        if let Some(channel) = any_ref.downcast_ref::<p2pgo_network::GameChannel>() {
            // Now access the internal method
            channel.handle_local_move(mv).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to downcast to GameChannel"))
        }
    }

    async fn replicate_last_move(&self) -> Result<()> {
        // Access the last move and resend it
        if let Some(state) = self.get_latest_state().await {
            if let Some(last_move) = state.moves.last() {
                self.submit_move_no_ack(last_move.mv.clone()).await?;
            }
        }

        Ok(())
    }
}
