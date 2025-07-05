// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end network tests for the UI worker

#[cfg(feature = "iroh")]
mod iroh_tests {
    use crossbeam_channel::{unbounded, Receiver, Sender};
    use p2pgo_core::{Coord, Move};
    use p2pgo_ui_egui::{
        msg::{NetToUi, UiToNet},
        worker::spawn_worker,
    };
    use std::thread;
    use tokio::time::{timeout, Duration};

    fn setup_worker(
        player_name: &str,
        board_size: u8,
    ) -> (Sender<UiToNet>, Receiver<NetToUi>, thread::JoinHandle<()>) {
        let (ui_tx, net_rx) = unbounded();
        let (net_tx, ui_rx) = unbounded();

        let handle = spawn_worker(net_rx, net_tx, board_size, player_name.to_string())
            .expect("Failed to spawn worker");

        (ui_tx, ui_rx, handle)
    }

    #[tokio::test]
    async fn test_enhanced_ticket_connection() {
        // Test the new EnhancedTicket format with multi-game support
        let (ui_tx1, ui_rx1, _handle1) = setup_worker("Player1", 9);
        let (ui_tx2, ui_rx2, _handle2) = setup_worker("Player2", 9);

        // Wait for workers to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Get ticket from player 1
        ui_tx1
            .send(UiToNet::GetTicket)
            .expect("Failed to send GetTicket");

        let ticket = loop {
            if let Ok(msg) = ui_rx1.try_recv() {
                match msg {
                    NetToUi::Ticket { ticket } => break ticket,
                    _ => continue,
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };

        // Player 2 connects using the ticket
        ui_tx2
            .send(UiToNet::ConnectByTicket { ticket })
            .expect("Failed to send ConnectByTicket");

        // Wait for connection to establish
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Player 1 creates a 9x9 game
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 9 })
            .expect("Failed to send CreateGame");

        // Both players should eventually see a GameJoined event
        let mut player1_joined = false;
        let mut player2_joined = false;

        for _ in 0..50 {
            // Wait up to 5 seconds
            if let Ok(msg) = ui_rx1.try_recv() {
                if matches!(msg, NetToUi::GameJoined { .. }) {
                    player1_joined = true;
                }
            }
            if let Ok(msg) = ui_rx2.try_recv() {
                if matches!(msg, NetToUi::GameJoined { .. }) {
                    player2_joined = true;
                }
            }
            if player1_joined && player2_joined {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert!(player1_joined, "Player 1 should have joined the game");
        assert!(player2_joined, "Player 2 should have joined the game");
    }

    #[tokio::test]
    async fn test_multi_board_size_games() {
        // Test that workers can handle multiple board sizes
        let (ui_tx1, ui_rx1, _handle1) = setup_worker("Player1", 9);

        // Wait for worker to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Create games for different board sizes
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 9 })
            .expect("Failed to create 9x9 game");
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 13 })
            .expect("Failed to create 13x13 game");
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 19 })
            .expect("Failed to create 19x19 game");

        // Should receive GameJoined for all three
        let mut games_joined = 0;

        for _ in 0..30 {
            // Wait up to 3 seconds
            if let Ok(msg) = ui_rx1.try_recv() {
                if matches!(msg, NetToUi::GameJoined { .. }) {
                    games_joined += 1;
                }
            }
            if games_joined >= 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert_eq!(
            games_joined, 3,
            "Should have joined 3 games for different board sizes"
        );

        // Try to create another 9x9 game (should fail - already have one)
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 9 })
            .expect("Failed to send second 9x9 game request");

        // Should receive an error
        let mut received_error = false;
        for _ in 0..10 {
            if let Ok(msg) = ui_rx1.try_recv() {
                if matches!(msg, NetToUi::Error { .. }) {
                    received_error = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert!(
            received_error,
            "Should receive error when trying to create duplicate board size game"
        );
    }

    #[tokio::test]
    async fn test_move_with_board_size() {
        // Test making moves with board_size parameter
        let (ui_tx1, ui_rx1, _handle1) = setup_worker("Player1", 9);

        tokio::time::sleep(Duration::from_millis(500)).await;

        // Create a 13x13 game
        ui_tx1
            .send(UiToNet::CreateGame { board_size: 13 })
            .expect("Failed to create 13x13 game");

        // Wait for game creation
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Make a move specifying the board size
        let test_move = Move::Place(Coord::new(6, 6)); // Center of 13x13 board
        ui_tx1
            .send(UiToNet::MakeMove {
                mv: test_move.clone(),
                board_size: Some(13),
            })
            .expect("Failed to send move");

        // Should receive a game event
        let mut received_move_event = false;
        for _ in 0..20 {
            if let Ok(msg) = ui_rx1.try_recv() {
                if let NetToUi::GameEvent { event } = msg {
                    if let p2pgo_core::GameEvent::MoveMade { mv, .. } = event {
                        if mv == test_move {
                            received_move_event = true;
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert!(
            received_move_event,
            "Should receive move event for the played move"
        );
    }
}

#[cfg(not(feature = "iroh"))]
mod stub_tests {
    use crossbeam_channel::{unbounded, Receiver, Sender};
    use p2pgo_core::{Coord, Move};
    use p2pgo_ui_egui::{
        msg::{NetToUi, UiToNet},
        worker::spawn_worker,
    };
    use std::thread;

    fn setup_worker(
        player_name: &str,
        board_size: u8,
    ) -> (Sender<UiToNet>, Receiver<NetToUi>, thread::JoinHandle<()>) {
        let (ui_tx, net_rx) = unbounded();
        let (net_tx, ui_rx) = unbounded();

        let handle = spawn_worker(net_rx, net_tx, board_size, player_name.to_string())
            .expect("Failed to spawn worker");

        (ui_tx, ui_rx, handle)
    }

    #[tokio::test]
    async fn test_stub_basic_functionality() {
        let (ui_tx, ui_rx, _handle) = setup_worker("TestPlayer", 9);

        // Give worker time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test basic operations
        ui_tx
            .send(UiToNet::GetNodeId)
            .expect("Failed to send GetNodeId");
        ui_tx
            .send(UiToNet::GetTicket)
            .expect("Failed to send GetTicket");
        ui_tx
            .send(UiToNet::CreateGame { board_size: 9 })
            .expect("Failed to send CreateGame");

        // Should receive responses
        let mut received_node_id = false;
        let mut received_ticket = false;
        let mut received_game_joined = false;

        for _ in 0..20 {
            if let Ok(msg) = ui_rx.try_recv() {
                match msg {
                    NetToUi::NodeId { .. } => received_node_id = true,
                    NetToUi::Ticket { .. } => received_ticket = true,
                    NetToUi::GameJoined { .. } => received_game_joined = true,
                    _ => {}
                }
            }

            if received_node_id && received_ticket && received_game_joined {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert!(received_node_id, "Should receive node ID");
        assert!(received_ticket, "Should receive ticket");
        assert!(received_game_joined, "Should receive game joined event");
    }

    #[tokio::test]
    async fn test_stub_move_handling() {
        let (ui_tx, ui_rx, _handle) = setup_worker("TestPlayer", 9);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Create a game first
        ui_tx
            .send(UiToNet::CreateGame { board_size: 9 })
            .expect("Failed to send CreateGame");

        // Wait for game creation
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Send a move
        let test_move = Move::Place(Coord::new(3, 3));
        ui_tx
            .send(UiToNet::MakeMove {
                mv: test_move.clone(),
                board_size: None, // Use default board size
            })
            .expect("Failed to send move");

        // Should receive a game event
        let mut received_move_event = false;
        for _ in 0..20 {
            if let Ok(msg) = ui_rx.try_recv() {
                if let NetToUi::GameEvent { event } = msg {
                    if let p2pgo_core::GameEvent::MoveMade { mv, .. } = event {
                        if mv == test_move {
                            received_move_event = true;
                            break;
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        assert!(
            received_move_event,
            "Should receive move event for the played move"
        );
    }
}
