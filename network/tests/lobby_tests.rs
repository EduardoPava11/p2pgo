// SPDX-License-Identifier: MIT OR Apache-2.0

use p2pgo_network::lobby::{Lobby, LobbyEvent};

#[tokio::test]
async fn test_lobby_create_and_list_games() {
    let lobby = Lobby::new();
    
    // Create a few games
    let game1 = lobby.create_game(Some("Game 1".to_string()), 9, false).await.unwrap();
    let game2 = lobby.create_game(Some("Game 2".to_string()), 13, false).await.unwrap();
    let game3 = lobby.create_game(Some("Game 3".to_string()), 19, true).await.unwrap();
    
    // List games
    let games = lobby.list_games().await;
    assert_eq!(games.len(), 3);
    
    // Check individual games
    let mut found_games = 0;
    for game in games {
        if game.id == game1 {
            assert_eq!(game.name, Some("Game 1".to_string()));
            assert_eq!(game.board_size, 9);
            assert!(!game.needs_password);
            found_games += 1;
        } else if game.id == game2 {
            assert_eq!(game.name, Some("Game 2".to_string()));
            assert_eq!(game.board_size, 13);
            assert!(!game.needs_password);
            found_games += 1;
        } else if game.id == game3 {
            assert_eq!(game.name, Some("Game 3".to_string()));
            assert_eq!(game.board_size, 19);
            assert!(game.needs_password);
            found_games += 1;
        }
    }
    assert_eq!(found_games, 3);
}

#[tokio::test]
async fn test_lobby_events() {
    let lobby = Lobby::new();
    
    // Subscribe to lobby events
    let mut rx = lobby.subscribe();
    
    // Create a game
    let game_id = lobby.create_game(Some("Event Test Game".to_string()), 9, false).await.unwrap();
    
    // Check for game created event
    let event = rx.recv().await.unwrap();
    match event {
        LobbyEvent::GameCreated(info) => {
            assert_eq!(info.id, game_id);
            assert_eq!(info.name, Some("Event Test Game".to_string()));
        },
        _ => panic!("Expected GameCreated event"),
    }
    
    // Start the game
    lobby.start_game(&game_id).await.unwrap();
    
    // Check for game started event
    let event = rx.recv().await.unwrap();
    match event {
        LobbyEvent::GameStarted(id) => {
            assert_eq!(id, game_id);
        },
        _ => panic!("Expected GameStarted event"),
    }
    
    // Remove the game
    lobby.remove_game(&game_id).await.unwrap();
    
    // Check for game ended event
    let event = rx.recv().await.unwrap();
    match event {
        LobbyEvent::GameEnded(id) => {
            assert_eq!(id, game_id);
        },
        _ => panic!("Expected GameEnded event"),
    }
}

#[tokio::test]
async fn test_lobby_game_interaction() {
    use p2pgo_core::{Move, Coord, Color, GameEvent};
    
    let lobby = Lobby::new();
    
    // Create a game
    let game_id = lobby.create_game(None, 9, false).await.unwrap();
    
    // Start the game
    lobby.start_game(&game_id).await.unwrap();
    
    // Get a channel for the game
    let channel = lobby.get_game_channel(&game_id).await.unwrap();
    let mut rx = channel.subscribe();
    
    // Post a move through the lobby
    let mv = Move::Place(Coord::new(4, 4));
    lobby.post_move(&game_id, mv.clone()).await.unwrap();
    
    // Check the event was received through the game channel
    let event = rx.recv().await.unwrap();
    match event {
        GameEvent::MoveMade { mv: received_mv, by } => {
            assert_eq!(received_mv, mv);
            assert_eq!(by, Color::Black); // First move is by Black
        },
        _ => panic!("Expected MoveMade event"),
    }
}
