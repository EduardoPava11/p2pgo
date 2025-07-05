// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz test for game move deserialization
//! Tests that feeding random bytes to move deserialization doesn't cause panics

#![no_main]

use libfuzzer_sys::fuzz_target;
use p2pgo_core::{GameState, Move};
use p2pgo_network::GameChannel;
use std::sync::Arc;
use anyhow::Result;

// Import to allow creation of the game channel
use p2pgo_network::GameId;

// Fuzz target for testing MoveMsg handling
fuzz_target!(|data: &[u8]| {
    // Skip empty data
    if data.is_empty() || data.len() < 4 {
        return;
    }

    // Try to treat the data as a MoveMsg
    let result = deserialize_and_process(data);

    // We just want to make sure it doesn't panic, so we ignore the result
    let _ = result;
});

// Helper function to set up a game channel and try to process the data
#[tokio::main]
async fn deserialize_and_process(data: &[u8]) -> Result<()> {
    // Create a game state
    let mut game_state = GameState::new(9);

    // Create a game channel for testing
    let game_id = "fuzz-test-game";

    // Try to deserialize the data as a move from CBOR
    let deserialized_result = deserialize_move_from_bytes(data);

    match deserialized_result {
        Ok(mv) => {
            // Try to apply the move
            let apply_result = game_state.apply_move(mv);
            // The result doesn't matter, we just want to make sure it doesn't panic
        },
        Err(_) => {
            // Deserialization failed, that's fine
        }
    }

    // Also try to deserialize as a game event
    let event_result = deserialize_game_event(data);
    let _ = event_result;

    Ok(())
}

// Helper function to deserialize the data as a Move
fn deserialize_move_from_bytes(data: &[u8]) -> Result<Move> {
    // Try CBOR deserialization
    let result = serde_cbor::from_slice::<Move>(data);
    result.map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))
}

// Helper function to deserialize the data as a GameEvent
fn deserialize_game_event(data: &[u8]) -> Result<p2pgo_core::GameEvent> {
    // Try CBOR deserialization
    let result = serde_cbor::from_slice::<p2pgo_core::GameEvent>(data);
    result.map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))
}
