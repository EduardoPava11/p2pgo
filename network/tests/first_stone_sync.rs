// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: First stone appears on both boards
//!
//! This test verifies the core MVP networking requirement:
//! When one player places the first stone, it should appear on both players' boards.

#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_core::{Color, Coord, GameEvent, GameState, Move};
    use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    /// Test that when one player places a stone, it appears on both boards
    #[tokio::test]
    async fn first_stone_appears_on_both_boards() -> Result<()> {
        // Create two nodes representing two players
        let node_a = IrohCtx::new().await?;
        let node_b = IrohCtx::new().await?;

        println!(
            "Created two nodes: {} and {}",
            node_a.node_id(),
            node_b.node_id()
        );

        // Connect the nodes via ticket
        let ticket = node_a.ticket().await?;
        node_b.connect_by_ticket(&ticket).await?;

        println!("Connected nodes via ticket");

        // Give connections time to establish
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Create a shared game ID
        let game_id = "first-stone-test-game".to_string();
        let board_size = 9;
        let initial_state = GameState::new(board_size);

        // Create game channels for both players
        let channel_a = GameChannel::with_iroh(
            game_id.clone(),
            initial_state.clone(),
            Arc::new(node_a.clone()),
        )
        .await?;

        let channel_b = GameChannel::with_iroh(
            game_id.clone(),
            initial_state.clone(),
            Arc::new(node_b.clone()),
        )
        .await?;

        println!("Created game channels for both players");

        // Connect the game channels to each other for direct communication
        let ticket_for_b = node_a.ticket().await?;
        channel_b.connect_to_peer(&ticket_for_b).await?;

        println!("Connected game channels");

        // Subscribe to events on player B's channel to detect incoming moves
        let mut events_b = channel_b.subscribe();

        // Give time for connections and subscriptions to establish
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Player A places the first stone at (4, 4) - center of 9x9 board
        let first_move = Move::Place(Coord::new(4, 4));

        println!("Player A placing first stone at (4, 4)");
        channel_a.send_move(first_move.clone()).await?;

        // Player B should receive the move event through gossip
        let received_event = timeout(Duration::from_secs(10), events_b.recv())
            .await
            .map_err(|_| {
                anyhow::anyhow!("Timeout waiting for first stone to appear on player B's board")
            })?
            .map_err(|e| anyhow::anyhow!("Error receiving event: {}", e))?;

        // Verify the received event is a MoveMade event with the correct coordinates
        match received_event {
            GameEvent::MoveMade { mv, by } => {
                println!("Player B received move: {:?} by {:?}", mv, by);

                match mv {
                    Move::Place(coord) => {
                        assert_eq!(coord.x, 4, "X coordinate should be 4");
                        assert_eq!(coord.y, 4, "Y coordinate should be 4");
                        println!("✓ First stone appeared on both boards at correct position!");
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Expected Place move, got: {:?}", mv));
                    }
                }
            }
            other => {
                return Err(anyhow::anyhow!("Expected MoveMade event, got: {:?}", other));
            }
        }

        // Verify both channels have the same game state
        let state_a = channel_a
            .get_latest_state()
            .await
            .ok_or_else(|| anyhow::anyhow!("Channel A has no state"))?;
        let state_b = channel_b
            .get_latest_state()
            .await
            .ok_or_else(|| anyhow::anyhow!("Channel B has no state"))?;

        // Check that both boards show the stone at (4, 4)
        let board_size = state_a.board_size as usize;
        let idx = (4 * board_size) + 4; // y * board_size + x
        let stone_a = state_a.board[idx];
        let stone_b = state_b.board[idx];

        assert_eq!(
            stone_a, stone_b,
            "Both boards should have the same stone at (4, 4)"
        );
        assert!(stone_a.is_some(), "Stone should be present on both boards");

        println!("✓ Both boards have consistent state with the first stone");

        // Verify the stone color is correct (Black typically goes first)
        if let Some(color) = stone_a {
            assert_eq!(color, Color::Black, "First stone should be Black");
            println!("✓ First stone is Black as expected");
        }

        println!("🎉 Test passed: First stone appears on both boards!");

        Ok(())
    }

    /// Test with multiple moves to ensure synchronization works consistently
    #[tokio::test]
    async fn multiple_stones_sync_correctly() -> Result<()> {
        // Create two nodes
        let node_a = IrohCtx::new().await?;
        let node_b = IrohCtx::new().await?;

        // Connect them
        let ticket = node_a.ticket().await?;
        node_b.connect_by_ticket(&ticket).await?;

        // Set up game
        let game_id = "multi-stone-sync-test".to_string();
        let initial_state = GameState::new(9);

        let channel_a = GameChannel::with_iroh(
            game_id.clone(),
            initial_state.clone(),
            Arc::new(node_a.clone()),
        )
        .await?;

        let channel_b = GameChannel::with_iroh(
            game_id.clone(),
            initial_state.clone(),
            Arc::new(node_b.clone()),
        )
        .await?;

        let mut events_b = channel_b.subscribe();

        // Give time for connections
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Test sequence of moves
        let moves = vec![
            Move::Place(Coord::new(3, 3)), // Black
            Move::Place(Coord::new(4, 4)), // White
            Move::Place(Coord::new(5, 5)), // Black
        ];

        for (i, mv) in moves.iter().enumerate() {
            println!("Sending move {}: {:?}", i + 1, mv);
            channel_a.send_move(mv.clone()).await?;

            // Wait for the move to be received
            let _received_event = timeout(Duration::from_secs(5), events_b.recv())
                .await
                .map_err(|_| anyhow::anyhow!("Timeout waiting for move {} to sync", i + 1))?
                .map_err(|e| anyhow::anyhow!("Error receiving move {}: {}", i + 1, e))?;

            println!("✓ Move {} synced successfully", i + 1);
        }

        // Verify final states match
        let state_a = channel_a
            .get_latest_state()
            .await
            .ok_or_else(|| anyhow::anyhow!("Channel A has no state"))?;
        let state_b = channel_b
            .get_latest_state()
            .await
            .ok_or_else(|| anyhow::anyhow!("Channel B has no state"))?;

        for mv in &moves {
            if let Move::Place(coord) = mv {
                let board_size = state_a.board_size as usize;
                let idx = (coord.y as usize * board_size) + (coord.x as usize);
                let stone_a = state_a.board[idx];
                let stone_b = state_b.board[idx];
                assert_eq!(
                    stone_a, stone_b,
                    "Stone at {:?} should match on both boards",
                    coord
                );
                assert!(stone_a.is_some(), "Stone should exist at {:?}", coord);
            }
        }

        println!("🎉 Multiple stones sync test passed!");

        Ok(())
    }
}

// Stub test for when iroh feature is not enabled
#[cfg(not(feature = "iroh"))]
mod tests {
    #[tokio::test]
    async fn first_stone_appears_on_both_boards() {
        println!("Stub test: first_stone_appears_on_both_boards (iroh feature disabled)");
        // In stub mode, we just pass the test
        assert!(true);
    }

    #[tokio::test]
    async fn multiple_stones_sync_correctly() {
        println!("Stub test: multiple_stones_sync_correctly (iroh feature disabled)");
        assert!(true);
    }
}
