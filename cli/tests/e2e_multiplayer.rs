// End-to-end test: 2 players, 9x9 board, predefined moves, verify identical states
use p2pgo_core::{Color, Coord, Move};
use p2pgo_network::Lobby;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_e2e_multiplayer_game() -> anyhow::Result<()> {
    let moves = vec![
        (Color::Black, Move::Place(Coord::new(3, 3))), // D4
        (Color::White, Move::Place(Coord::new(5, 3))), // F4
        (Color::Black, Move::Place(Coord::new(4, 4))), // E5
        (Color::White, Move::Pass),
        (Color::Black, Move::Pass),
    ];
    let lobby = Lobby::new();
    let game_id = lobby
        .create_game(Some("E2E Test".to_string()), 9, false)
        .await?;
    lobby.start_game(&game_id).await?;
    let channel_p1 = lobby.get_game_channel(&game_id).await?;
    let channel_p2 = lobby.get_game_channel(&game_id).await?;
    let mut events_p1 = channel_p1.subscribe();
    let mut events_p2 = channel_p2.subscribe();

    // Verify initial states are identical
    let state_p1 = channel_p1.get_latest_state().await.unwrap();
    let state_p2 = channel_p2.get_latest_state().await.unwrap();
    assert_eq!(state_p1.board_size, 9);
    assert_eq!(
        serde_json::to_string(&state_p1)?,
        serde_json::to_string(&state_p2)?
    );

    // Play through moves
    for (player, mv) in moves.iter() {
        let current = channel_p1.get_latest_state().await.unwrap();
        assert_eq!(current.current_player, *player);
        lobby.post_move(&game_id, mv.clone()).await?;

        // Both players receive identical events
        let event_p1 = timeout(Duration::from_secs(1), events_p1.recv()).await??;
        let event_p2 = timeout(Duration::from_secs(1), events_p2.recv()).await??;
        match (&event_p1, &event_p2) {
            (
                p2pgo_core::GameEvent::MoveMade { mv: mv1, by: by1 },
                p2pgo_core::GameEvent::MoveMade { mv: mv2, by: by2 },
            ) => {
                assert_eq!((mv1, by1), (mv2, by2));
                assert_eq!((mv1, by1), (mv, player));
            }
            _ => panic!("Expected MoveMade events"),
        }

        // States remain synchronized
        let s1 = channel_p1.get_latest_state().await.unwrap();
        let s2 = channel_p2.get_latest_state().await.unwrap();
        assert_eq!(serde_json::to_string(&s1)?, serde_json::to_string(&s2)?);
    }

    // Verify game over and final state
    let final_p1 = channel_p1.get_latest_state().await.unwrap();
    let final_p2 = channel_p2.get_latest_state().await.unwrap();
    assert!(final_p1.is_game_over() && final_p2.is_game_over());
    assert_eq!(final_p1.pass_count, 2);
    assert_eq!(
        serde_json::to_string(&final_p1)?,
        serde_json::to_string(&final_p2)?
    );

    // Verify board positions: D4=Black, F4=White, E5=Black
    assert_eq!(final_p1.board[3 * 9 + 3], Some(Color::Black)); // D4
    assert_eq!(final_p1.board[3 * 9 + 5], Some(Color::White)); // F4
    assert_eq!(final_p1.board[4 * 9 + 4], Some(Color::Black)); // E5
    assert_eq!(final_p1.moves.len(), 5);
    assert_eq!(final_p1.captures, (0, 0)); // No captures
    Ok(())
}

#[test]
fn test_coordinate_conversion() {
    assert_eq!(Coord::new(3, 3), Coord { x: 3, y: 3 }); // D4
    assert_eq!(Coord::new(5, 3), Coord { x: 5, y: 3 }); // F4
    assert_eq!(Coord::new(4, 4), Coord { x: 4, y: 4 }); // E5
}
