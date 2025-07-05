// Integration tests for P2P Go
// Run with: cargo test --test integration_tests

use p2pgo_core::{GameState, Move, Color, Coord};
use p2pgo_network::{NodeContext, lobby::Lobby, game_channel::GameChannel};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_two_player_game_flow() {
    // Initialize two nodes
    let node1 = NodeContext::new_test("node1").await.unwrap();
    let node2 = NodeContext::new_test("node2").await.unwrap();
    
    // Create lobbies
    let lobby1 = Lobby::new(node1.clone());
    let lobby2 = Lobby::new(node2.clone());
    
    // Node 1 creates a game
    let game_id = "test-game-123";
    let game_channel1 = GameChannel::create(
        node1.clone(),
        game_id.to_string(),
        9, // board size
    ).await.unwrap();
    
    // Node 2 joins the game
    let game_channel2 = GameChannel::join(
        node2.clone(),
        game_id.to_string(),
        node1.node_id(),
    ).await.unwrap();
    
    // Subscribe to game events
    let mut events1 = game_channel1.subscribe().await.unwrap();
    let mut events2 = game_channel2.subscribe().await.unwrap();
    
    // Make moves alternately
    let moves = vec![
        Move::Place { x: 3, y: 3, color: Color::Black },
        Move::Place { x: 5, y: 5, color: Color::White },
        Move::Place { x: 4, y: 4, color: Color::Black },
    ];
    
    for (i, mv) in moves.iter().enumerate() {
        let channel = if i % 2 == 0 { &game_channel1 } else { &game_channel2 };
        channel.send_move(mv.clone()).await.unwrap();
        
        // Wait for move to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Verify both nodes have same game state
    let state1 = game_channel1.get_state().await.unwrap();
    let state2 = game_channel2.get_state().await.unwrap();
    
    assert_eq!(state1.moves.len(), 3);
    assert_eq!(state2.moves.len(), 3);
    assert_eq!(state1.current_player, state2.current_player);
}

#[tokio::test]
async fn test_relay_mode_switching() {
    use p2pgo_network::relay_config::{RelayMode, RelayConfig};
    
    let node = NodeContext::new_test("relay-test").await.unwrap();
    
    // Test each relay mode
    let modes = vec![
        RelayMode::Disabled,
        RelayMode::Minimal,
        RelayMode::Normal { max_reservations: 25, max_circuits: 50 },
        RelayMode::Provider { max_reservations: 50, max_circuits: 100, require_credits: false },
    ];
    
    for mode in modes {
        let config = RelayConfig {
            mode: mode.clone(),
            max_bandwidth: Some(1_000_000),
            max_connections: 10,
            relay_timeout: Duration::from_secs(3600),
            enable_metrics: true,
        };
        
        // Apply configuration
        node.update_relay_config(config).await.unwrap();
        
        // Verify mode is set
        let current_mode = node.get_relay_mode().await.unwrap();
        assert_eq!(current_mode, mode);
    }
}

#[tokio::test]
async fn test_score_consensus() {
    use p2pgo_core::scoring::calculate_final_score;
    use p2pgo_core::value_labeller::ScoringMethod;
    
    // Create finished game state
    let mut game = GameState::new(9);
    
    // Simulate some moves
    game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap();
    game.apply_move(Move::Place { x: 5, y: 5, color: Color::White }).unwrap();
    game.apply_move(Move::Pass).unwrap();
    game.apply_move(Move::Pass).unwrap();
    
    assert!(game.is_game_over());
    
    // Calculate score
    let komi = 5.5;
    let dead_stones = std::collections::HashSet::new();
    let score_proof = calculate_final_score(
        &game,
        komi,
        ScoringMethod::Territory,
        &dead_stones,
    );
    
    // Verify score proof
    assert!(score_proof.final_score != 0.0);
    assert_eq!(score_proof.komi, komi);
}

#[tokio::test]
async fn test_cbor_archiving() {
    use p2pgo_core::archiver::archive_finished_game;
    use tempfile::tempdir;
    
    // Create a finished game
    let mut game = GameState::new(9);
    game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap();
    game.apply_move(Move::Pass).unwrap();
    game.apply_move(Move::Pass).unwrap();
    
    // Archive to temporary directory
    let temp_dir = tempdir().unwrap();
    std::env::set_var("HOME", temp_dir.path());
    
    let archive_path = archive_finished_game(&game, "opponent").unwrap();
    
    // Verify archive exists
    assert!(archive_path.exists());
    assert!(archive_path.to_string_lossy().contains(".cbor"));
    
    // Read back and verify
    let restored = p2pgo_core::archiver::read_game_archive(&archive_path).unwrap();
    assert_eq!(restored.moves.len(), game.moves.len());
}

#[tokio::test] 
async fn test_network_recovery() {
    let node1 = NodeContext::new_test("recovery1").await.unwrap();
    let node2 = NodeContext::new_test("recovery2").await.unwrap();
    
    // Establish connection
    let game_id = "recovery-test";
    let game_channel1 = GameChannel::create(
        node1.clone(),
        game_id.to_string(),
        9,
    ).await.unwrap();
    
    let game_channel2 = GameChannel::join(
        node2.clone(),
        game_id.to_string(),
        node1.node_id(),
    ).await.unwrap();
    
    // Make a move
    game_channel1.send_move(Move::Place { x: 3, y: 3, color: Color::Black }).await.unwrap();
    
    // Simulate network disruption
    // In real test, would disconnect network
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Make another move
    game_channel2.send_move(Move::Place { x: 5, y: 5, color: Color::White }).await.unwrap();
    
    // Verify moves are eventually consistent
    let result = timeout(Duration::from_secs(5), async {
        loop {
            let state1 = game_channel1.get_state().await.unwrap();
            let state2 = game_channel2.get_state().await.unwrap();
            
            if state1.moves.len() == 2 && state2.moves.len() == 2 {
                return Ok(());
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_training_data_consent() {
    use p2pgo_network::training::TrainingConfig;
    
    let node = NodeContext::new_test("training-test").await.unwrap();
    
    // Enable training consent
    let mut config = TrainingConfig::default();
    config.consent_given = true;
    config.share_anonymized = true;
    
    node.update_training_config(config).await.unwrap();
    
    // Verify consent is recorded
    let current_config = node.get_training_config().await.unwrap();
    assert!(current_config.consent_given);
    assert!(current_config.share_anonymized);
}

// Performance test
#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_high_throughput_moves() {
    let node1 = NodeContext::new_test("perf1").await.unwrap();
    let node2 = NodeContext::new_test("perf2").await.unwrap();
    
    let game_channel1 = GameChannel::create(
        node1.clone(),
        "perf-test".to_string(),
        19, // Large board
    ).await.unwrap();
    
    let game_channel2 = GameChannel::join(
        node2.clone(),
        "perf-test".to_string(),
        node1.node_id(),
    ).await.unwrap();
    
    // Make 100 moves rapidly
    let start = std::time::Instant::now();
    
    for i in 0..100 {
        let x = (i % 19) as u8;
        let y = (i / 19) as u8;
        let color = if i % 2 == 0 { Color::Black } else { Color::White };
        let mv = Move::Place { x, y, color };
        
        let channel = if i % 2 == 0 { &game_channel1 } else { &game_channel2 };
        channel.send_move(mv).await.unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("100 moves completed in {:?}", elapsed);
    
    // Should complete in under 10 seconds
    assert!(elapsed.as_secs() < 10);
}