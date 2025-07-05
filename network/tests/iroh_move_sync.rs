// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test iroh move synchronization between game channels

#[cfg(feature = "iroh")]
mod tests {
    use p2pgo_core::{Coord, GameState, Move};
    use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_move_sync_between_channels() {
        // Create two iroh contexts
        let ctx1 = IrohCtx::new()
            .await
            .expect("Failed to create first IrohCtx");
        let ctx2 = IrohCtx::new()
            .await
            .expect("Failed to create second IrohCtx");

        // Connect them
        let ticket = ctx1.ticket().await.expect("Failed to generate ticket");
        ctx2.connect_by_ticket(&ticket)
            .await
            .expect("Failed to connect by ticket");

        // Create game channels for the same game
        let game_id = "test-sync-game-123".to_string();
        let initial_state = GameState::new(9);

        let channel1 =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), Arc::new(ctx1))
                .await
                .expect("Failed to create first game channel");

        let channel2 =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), Arc::new(ctx2))
                .await
                .expect("Failed to create second game channel");

        // Subscribe to events on channel2
        let mut events_rx = channel2.subscribe();

        // Give time for subscriptions to establish
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Make a move on channel1
        let test_move = Move::Place(Coord::new(3, 3));
        channel1
            .send_move(test_move.clone())
            .await
            .expect("Failed to send move from channel1");

        // Channel2 should receive the move event
        let event = timeout(Duration::from_secs(5), events_rx.recv())
            .await
            .expect("Timeout waiting for move event")
            .expect("Failed to receive move event");

        // Verify the event
        match event {
            p2pgo_core::GameEvent::MoveMade { mv, by } => {
                assert_eq!(mv, test_move);
                assert_eq!(by, p2pgo_core::Color::Black); // First move should be by black
            }
            _ => panic!("Expected MoveMade event, got: {:?}", event),
        }

        // Verify game states are synchronized
        let state1 = channel1
            .get_latest_state()
            .await
            .expect("Failed to get state from channel1");
        let state2 = channel2
            .get_latest_state()
            .await
            .expect("Failed to get state from channel2");

        assert_eq!(state1.moves.len(), state2.moves.len());
        assert_eq!(state1.current_player, state2.current_player);

        // Verify the move was applied to both states
        assert_eq!(state1.moves.len(), 1);
        assert_eq!(state1.moves[0], test_move);
        assert_eq!(state2.moves[0], test_move);
    }

    #[tokio::test]
    async fn test_doc_replay_functionality() {
        // Test that a new channel can replay moves from an existing document
        let ctx1 = IrohCtx::new()
            .await
            .expect("Failed to create first IrohCtx");

        let game_id = "test-replay-game-456".to_string();
        let initial_state = GameState::new(9);

        // Create first channel and make some moves
        let channel1 = GameChannel::with_iroh(
            game_id.clone(),
            initial_state.clone(),
            Arc::new(ctx1.clone()),
        )
        .await
        .expect("Failed to create first game channel");

        // Make a series of moves
        let moves = vec![
            Move::Place(Coord::new(3, 3)),
            Move::Place(Coord::new(4, 4)),
            Move::Place(Coord::new(5, 5)),
        ];

        for mv in &moves {
            channel1
                .send_move(mv.clone())
                .await
                .expect("Failed to send move");
            tokio::time::sleep(Duration::from_millis(100)).await; // Allow time for doc sync
        }

        // Create a second channel for the same game (should replay from doc)
        let ctx2 = IrohCtx::new()
            .await
            .expect("Failed to create second IrohCtx");

        // Connect to the first context so it can access the document
        let ticket = ctx1.ticket().await.expect("Failed to generate ticket");
        ctx2.connect_by_ticket(&ticket)
            .await
            .expect("Failed to connect by ticket");

        let channel2 =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), Arc::new(ctx2))
                .await
                .expect("Failed to create second game channel");

        // Give time for document sync
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify that channel2 has the same moves
        let moves1 = channel1.get_all_moves().await;
        let moves2 = channel2.get_all_moves().await;

        assert_eq!(moves1.len(), moves2.len());
        for (i, (mv1, mv2)) in moves1.iter().zip(moves2.iter()).enumerate() {
            assert_eq!(mv1, mv2, "Move {} should be the same", i);
        }
    }

    #[tokio::test]
    async fn test_gossip_and_doc_consistency() {
        // Test that moves are propagated both via gossip and stored in docs
        let ctx1 = IrohCtx::new()
            .await
            .expect("Failed to create first IrohCtx");
        let ctx2 = IrohCtx::new()
            .await
            .expect("Failed to create second IrohCtx");

        // Connect contexts
        let ticket = ctx1.ticket().await.expect("Failed to generate ticket");
        ctx2.connect_by_ticket(&ticket)
            .await
            .expect("Failed to connect by ticket");

        let game_id = "test-consistency-game-789".to_string();
        let initial_state = GameState::new(9);

        // Create channels
        let channel1 =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), Arc::new(ctx1))
                .await
                .expect("Failed to create first game channel");

        let channel2 =
            GameChannel::with_iroh(game_id.clone(), initial_state.clone(), Arc::new(ctx2))
                .await
                .expect("Failed to create second game channel");

        // Subscribe to events
        let mut events_rx2 = channel2.subscribe();

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Make moves alternately
        let test_moves = vec![
            Move::Place(Coord::new(2, 2)),
            Move::Place(Coord::new(6, 6)),
            Move::Pass,
            Move::Pass,
        ];

        for (i, mv) in test_moves.iter().enumerate() {
            if i % 2 == 0 {
                channel1
                    .send_move(mv.clone())
                    .await
                    .expect("Failed to send move from channel1");
            } else {
                channel2
                    .send_move(mv.clone())
                    .await
                    .expect("Failed to send move from channel2");
            }

            // Wait for synchronization
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // Verify both channels have all moves
        let moves1 = channel1.get_all_moves().await;
        let moves2 = channel2.get_all_moves().await;

        assert_eq!(moves1.len(), test_moves.len());
        assert_eq!(moves2.len(), test_moves.len());

        // Verify move order is consistent
        for (i, expected_move) in test_moves.iter().enumerate() {
            assert_eq!(
                &moves1[i], expected_move,
                "Channel1 move {} should match",
                i
            );
            assert_eq!(
                &moves2[i], expected_move,
                "Channel2 move {} should match",
                i
            );
        }
    }
}

#[cfg(not(feature = "iroh"))]
mod stub_tests {
    use p2pgo_core::{Coord, GameState, Move};
    use p2pgo_network::{game_channel::GameChannel, iroh_endpoint::IrohCtx};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_stub_move_sync() {
        let ctx = IrohCtx::new().await.expect("Failed to create stub IrohCtx");

        let game_id = "stub-game".to_string();
        let initial_state = GameState::new(9);

        // In stub mode, GameChannel::with_iroh is not available
        // So we test the basic GameChannel functionality
        let channel = GameChannel::new(game_id, initial_state);

        let test_move = Move::Place(Coord::new(3, 3));
        let result = channel.send_move(test_move.clone()).await;
        assert!(result.is_ok(), "Stub move should succeed");

        // Verify the move was recorded
        let moves = channel.get_all_moves().await;
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0], test_move);
    }
}
