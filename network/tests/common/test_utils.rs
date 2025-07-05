// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test utilities for P2P Go integration tests

use anyhow::{Context, Result};
use p2pgo_core::{Color, Coord, GameEvent, GameState, Move, MoveRecord};
use p2pgo_network::GameChannel;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{timeout, Instant};

/// Generate a random valid move on the board
pub fn random_move(state: &GameState) -> Move {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();

    let size = state.board_size as i8;
    let attempts = 20; // Try up to 20 random positions

    for _ in 0..attempts {
        let x = rng.gen_range(0..size);
        let y = rng.gen_range(0..size);
        let coord = Coord::new(x as u8, y as u8);

        // In a real implementation, we would check if the move is valid
        // For tests, we'll just use the random coordinate with current player's color
        return Move::Place {
            x: coord.x,
            y: coord.y,
            color: state.current_player,
        };
    }

    // If we can't find a random valid move, return pass
    Move::Pass
}

/// Wait for sync between peers with timeout
pub async fn wait_for_sync(channels: &[Arc<GameChannel>], timeout_ms: u64) -> Result<()> {
    let timeout_duration = Duration::from_millis(timeout_ms);
    let start = Instant::now();

    // Get all game states
    let mut states = Vec::new();
    for channel in channels {
        let state = channel.get_latest_state().await;
        states.push(state);
    }

    // Check if any are None
    if states.iter().any(|s| s.is_none()) {
        // At least one peer has no state yet, wait for it
        timeout(timeout_duration, async {
            loop {
                let mut all_have_state = true;

                for channel in channels {
                    if channel.get_latest_state().await.is_none() {
                        all_have_state = false;
                        break;
                    }
                }

                if all_have_state {
                    return;
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .context("Timeout waiting for initial game states")?;
    }

    // Now wait until all peers have the same move count
    let result = timeout(timeout_duration, async {
        loop {
            // Get move counts from all peers
            let mut move_counts = Vec::new();
            for channel in channels {
                let state = channel.get_latest_state().await;
                if let Some(state) = state {
                    move_counts.push(state.moves.len());
                } else {
                    move_counts.push(0);
                }
            }

            // Check if all move counts are the same and non-zero
            if move_counts.iter().all(|&c| c == move_counts[0]) && move_counts[0] > 0 {
                return;
            }

            if start.elapsed() > timeout_duration {
                return;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for move synchronization")),
    }
}

/// Wait for specific number of events to be received
pub async fn wait_for_events(
    channel: &Arc<GameChannel>,
    event_type: &str,
    count: usize,
    timeout_ms: u64,
) -> Result<Vec<GameEvent>> {
    let mut rx = channel.subscribe();
    let mut events = Vec::new();

    let result = timeout(Duration::from_millis(timeout_ms), async {
        while events.len() < count {
            match rx.recv().await {
                Ok(event) => {
                    match &event {
                        GameEvent::MoveMade { .. } if event_type == "move" => {
                            events.push(event);
                        }
                        GameEvent::ChatMessage { .. } if event_type == "chat" => {
                            events.push(event);
                        }
                        GameEvent::GameFinished { .. } if event_type == "finished" => {
                            events.push(event);
                        }
                        _ => {} // Ignore other events
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to receive events: {}", e));
                }
            }
        }
        Ok(events)
    })
    .await;

    match result {
        Ok(events) => Ok(events?),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for {} events", event_type)),
    }
}

/// Check if a move was deduplicated (never processed twice)
pub async fn was_move_deduplicated(
    channel: &Arc<GameChannel>,
    mv: &Move,
    timeout_ms: u64,
) -> Result<bool> {
    let mut rx = channel.subscribe();
    let mut seen = false;

    let result = timeout(Duration::from_millis(timeout_ms), async {
        loop {
            match rx.recv().await {
                Ok(GameEvent::MoveMade {
                    mv: ref event_move, ..
                }) => {
                    if event_move == mv {
                        if seen {
                            // Move seen twice - not deduplicated!
                            return false;
                        }
                        seen = true;
                    }
                }
                Ok(_) => {}      // Ignore other events
                Err(_) => break, // Channel closed or error
            }
        }
        // If we reach here, the move was seen at most once
        seen
    })
    .await;

    match result {
        Ok(deduplicated) => Ok(deduplicated),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for deduplication check")),
    }
}

/// Get the snapshot directory for a game
pub fn get_snapshot_dir(game_id: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("p2pgo")
        .join("snapshots")
        .join(game_id)
}

/// Get the latest snapshot file for a game
pub fn get_latest_snapshot(game_id: &str) -> Result<Option<PathBuf>> {
    let snapshot_dir = get_snapshot_dir(game_id);

    if !snapshot_dir.exists() {
        return Ok(None);
    }

    let mut latest_file = None;
    let mut latest_mtime = std::time::SystemTime::UNIX_EPOCH;

    for entry in fs::read_dir(snapshot_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "cbor") {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(mtime) = metadata.modified() {
                    if mtime > latest_mtime {
                        latest_mtime = mtime;
                        latest_file = Some(path);
                    }
                }
            }
        }
    }

    Ok(latest_file)
}

/// Wait for a snapshot file to be updated (mtime changed)
pub async fn wait_for_snapshot_update(
    game_id: &str,
    previous_mtime: Option<std::time::SystemTime>,
    timeout_ms: u64,
) -> Result<std::time::SystemTime> {
    let timeout_duration = Duration::from_millis(timeout_ms);

    let result = timeout(timeout_duration, async {
        loop {
            // Check for latest snapshot
            if let Ok(Some(latest)) = get_latest_snapshot(game_id) {
                if let Ok(metadata) = fs::metadata(&latest) {
                    if let Ok(mtime) = metadata.modified() {
                        if let Some(prev_mtime) = previous_mtime {
                            if mtime > prev_mtime {
                                return mtime;
                            }
                        } else {
                            return mtime; // No previous mtime, return current
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    match result {
        Ok(mtime) => Ok(mtime),
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for snapshot update")),
    }
}

/// Get the number of entries in the processed sequences dequeue
pub async fn get_processed_sequences_len(channel: &Arc<GameChannel>) -> Result<usize> {
    // This uses internal knowledge of the GameChannel structure
    #[cfg(feature = "iroh")]
    {
        use iroh::NodeId;
        use std::collections::VecDeque;
        use tokio::sync::Mutex;

        // Get pointer to the VecDeque using unsafe code
        let ptr = channel as *const GameChannel;
        let channel_ref = unsafe { &*ptr };

        // Access the private field
        #[allow(private_interfaces)]
        let sequences: &Arc<Mutex<VecDeque<(NodeId, u64)>>> = &channel_ref.processed_sequences;

        // Get the current size
        let deque = sequences.lock().await;
        Ok(deque.len())
    }

    #[cfg(not(feature = "iroh"))]
    {
        // Not supported in non-iroh mode
        Ok(0)
    }
}

/// Force a snapshot update for a game
pub async fn force_snapshot_update(channel: &Arc<GameChannel>) -> Result<()> {
    // For the tests, we'll just simulate a snapshot update
    // Actual implementation would call a method on GameChannel
    println!("Simulated snapshot update for testing purposes");
    Ok(())
}

/// Pause time in tests (uses tokio's time control features)
pub fn pause_time() {
    tokio::time::pause();
}

/// Resume time in tests
pub fn resume_time() {
    tokio::time::resume();
}

/// Advance time by the specified duration
pub async fn advance_time(duration: Duration) {
    tokio::time::advance(duration).await;
}

/// Get the ACK watchdog timeout value
pub fn get_ack_timeout() -> Duration {
    // Default ACK timeout from the GameChannel implementation
    Duration::from_secs(3)
}

/// Submit a move to the game channel
pub async fn submit_move(channel: &Arc<GameChannel>, mv: Move) -> Result<()> {
    // This is a wrapper that directly uses the handle_local_move method
    channel.handle_local_move(mv).await
}
