// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_core::{Color, Coord, GameEvent, GameState, Move};
use p2pgo_network::game_channel::GameChannel;

#[tokio::test]
async fn test_game_channel_send_and_receive_move() {
    let game_id = "test-game".to_string();
    let initial_state = GameState::new(9);
    let channel = GameChannel::new(game_id, initial_state);

    // Subscribe to events
    let mut rx = channel.subscribe();

    // Send a move
    let mv = Move::Place(Coord::new(4, 4));
    channel.send_move(mv.clone()).await.unwrap();

    // Check the event was received
    let event = rx.recv().await.unwrap();
    match event {
        GameEvent::MoveMade {
            mv: received_mv,
            by,
        } => {
            assert_eq!(received_mv, mv);
            assert_eq!(by, Color::Black); // First move is by Black
        }
        _ => panic!("Expected MoveMade event"),
    }
}

#[tokio::test]
async fn test_game_channel_send_chat() {
    let game_id = "test-game-chat".to_string();
    let initial_state = GameState::new(9);
    let channel = GameChannel::new(game_id, initial_state);

    // Subscribe to events
    let mut rx = channel.subscribe();

    // Send a chat message
    let chat_event = GameEvent::ChatMessage {
        from: Color::Black,
        message: "Hello!".to_string(),
    };
    channel.send_event(chat_event.clone()).await.unwrap();

    // Check the event was received
    let event = rx.recv().await.unwrap();
    match event {
        GameEvent::ChatMessage { from, message } => {
            assert_eq!(from, Color::Black);
            assert_eq!(message, "Hello!");
        }
        _ => panic!("Expected ChatMessage event"),
    }
}

#[tokio::test]
async fn test_game_channel_multiple_moves() {
    let game_id = "test-game-multiple".to_string();
    let initial_state = GameState::new(9);
    let channel = GameChannel::new(game_id, initial_state);

    // Send multiple moves
    let moves = [
        Move::Place(Coord::new(4, 4)),
        Move::Place(Coord::new(3, 3)),
        Move::Place(Coord::new(5, 5)),
    ];

    for mv in &moves {
        channel.send_move(mv.clone()).await.unwrap();
    }

    // Check all moves were stored
    let all_moves = channel.get_all_moves().await;
    assert_eq!(all_moves.len(), moves.len());

    for (i, mv) in moves.iter().enumerate() {
        assert_eq!(all_moves[i], *mv);
    }

    // Check state was properly updated
    let state = channel.get_latest_state().await.unwrap();
    assert_eq!(state.current_player, Color::White); // After 3 moves, it's White's turn again
}
