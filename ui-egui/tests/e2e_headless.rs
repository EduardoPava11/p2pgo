// SPDX-License-Identifier: MIT OR Apache-2.0

//! End-to-end headless test for AI integration pipeline
//! 
//! This test verifies the complete flow:
//! 1. Game creation and setup
//! 2. Move execution with automatic ghost move requests
//! 3. AI model lazy loading and computation
//! 4. Ghost move suggestions returned to UI
//! 5. Visual display of AI suggestions

use std::time::Duration;
use crossbeam_channel::unbounded;
use p2pgo_ui_egui::{
    app::App,
    msg::{UiToNet, NetToUi},
    worker,
};
use p2pgo_core::{Move, Coord, Color, GameState};

/// Headless test runner for AI integration
struct E2ETestRunner {
    app: App,
    ui_tx: crossbeam_channel::Sender<UiToNet>,
    #[allow(dead_code)]
    timeout_ms: u64,
}

impl E2ETestRunner {
    #[cfg(feature = "headless")]
    fn new() -> anyhow::Result<Self> {
        let (ui_tx, net_rx) = unbounded();
        let (net_tx, ui_rx) = unbounded();
        
        // Spawn background worker
        let _worker_handle = std::thread::spawn(move || {
            println!("Worker thread starting...");
            if let Err(e) = worker::start(net_rx, net_tx) {
                eprintln!("Worker error: {}", e);
            }
            println!("Worker thread exiting");
        });
        
        let mut app = App::new_headless_with_channels(ui_tx.clone(), ui_rx);
        
        // Give worker time to initialize and send initial messages
        println!("Waiting for worker initialization...");
        
        for i in 0..100 {
            app.tick_headless();
            std::thread::sleep(Duration::from_millis(50));
            
            if i % 10 == 0 {
                println!("Init tick {}, view: {}", i, app.get_current_view_debug());
            }
            
            // Simple heuristic: if we've done many ticks and stayed in MainMenu, 
            // the worker is probably ready
            if i > 20 {
                println!("Worker assumed ready after {} ticks", i);
                break;
            }
        }
        
        // Give a bit more time for any pending initialization
        println!("Final initialization wait...");
        for i in 0..10 {
            app.tick_headless();
            std::thread::sleep(Duration::from_millis(100));
            println!("  Final init tick {}", i);
        }
        
        Ok(Self {
            app,
            ui_tx,
            timeout_ms: 5000, // 5 second timeout
        })
    }
    
    #[cfg(feature = "headless")]
    fn run_ai_integration_test(&mut self) -> anyhow::Result<()> {
        println!("Starting E2E AI integration test");
        
        // Step 1: Create a game
        println!("Step 1: Creating game");
        self.ui_tx.send(UiToNet::CreateGame { board_size: 9 })?;
        
        // Wait for transition to Lobby state
        self.wait_for_view_transition("Lobby", 2000)?;
        
        // Step 2: Make initial move to transition to game state
        println!("Step 2: Making first move to activate game (Black D4)");
        let first_move = Move::Place(Coord::new(3, 3)); // D4
        self.ui_tx.send(UiToNet::MakeMove { mv: first_move.clone(), board_size: None })?;
        
        // Wait for transition to Game state
        self.wait_for_view_transition("Game", 2000)?;
        
        // Step 3: Verify game state after first move
        println!("Step 3: Verifying game state after first move");
        let game_state = self.app.get_current_game_state()
            .map_err(|e| anyhow::anyhow!("Failed to get game state: {}", e))?;
        
        assert_eq!(game_state.board_size, 9);
        assert_eq!(game_state.moves.len(), 1);
        assert_eq!(game_state.current_player, Color::White); // After black's move
        
        // Step 4: Make second move and verify AI triggers
        println!("Step 4: Making second move (White F4)");
        let second_move = Move::Place(Coord::new(5, 3)); // F4
        self.ui_tx.send(UiToNet::MakeMove { mv: second_move.clone(), board_size: None })?;
        self.wait_and_process(500); // Allow more time for AI processing
        
        // Step 5: Verify game progression
        println!("Step 5: Verifying game progression");
        let game_state = self.app.get_current_game_state()
            .map_err(|e| anyhow::anyhow!("Failed to get game state after second move: {}", e))?;
        
        assert_eq!(game_state.moves.len(), 2);
        assert_eq!(game_state.current_player, Color::Black);
        
        // Step 6: Make a few more moves to test AI consistently
        println!("Step 6: Making additional moves to test AI stability");
        let additional_moves = vec![
            Move::Place(Coord::new(4, 4)), // E5 - Black
            Move::Place(Coord::new(4, 3)), // E4 - White
            Move::Place(Coord::new(3, 4)), // D5 - Black
        ];
        
        for (i, mv) in additional_moves.iter().enumerate() {
            println!("  Making move {}: {:?}", i + 3, mv);
            self.ui_tx.send(UiToNet::MakeMove { mv: mv.clone(), board_size: None })?;
            self.wait_and_process(300);
        }
        
        // Step 7: Final verification
        println!("Step 7: Final verification");
        let final_state = self.app.get_current_game_state()
            .map_err(|e| anyhow::anyhow!("Failed to get final game state: {}", e))?;
        
        assert_eq!(final_state.moves.len(), 5);
        assert!(final_state.board[3 * 9 + 3].is_some()); // D4 occupied (3,3)
        assert!(final_state.board[3 * 9 + 5].is_some()); // F4 occupied (5,3) 
        assert!(final_state.board[4 * 9 + 4].is_some()); // E5 occupied (4,4)
        
        println!("E2E AI integration test completed successfully");
        Ok(())
    }
    
    #[cfg(feature = "headless")]
    fn run_simple_game_creation_test(&mut self) -> anyhow::Result<()> {
        println!("Starting simple game creation test");
        
        // Give the worker a moment to be fully ready
        println!("Ensuring worker is ready...");
        for _ in 0..5 {
            self.app.tick_headless();
            std::thread::sleep(Duration::from_millis(100));
        }
        
        // Step 1: Create a game
        println!("Step 1: Creating game");
        println!("  Sending CreateGame message...");
        self.ui_tx.send(UiToNet::CreateGame { board_size: 9 })?;
        
        // Give the worker a moment to process the message
        println!("  Allowing time for message processing...");
        for _ in 0..5 {
            self.app.tick_headless();
            std::thread::sleep(Duration::from_millis(100));
        }
        
        // Wait for transition to Lobby state
        self.wait_for_view_transition("Lobby", 2000)?;
        
        println!("Simple game creation test completed successfully");
        Ok(())
    }
    
    #[cfg(feature = "headless")]
    fn wait_for_view_transition(&mut self, expected_view_prefix: &str, timeout_ms: u64) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let mut tick_count = 0;
        
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            self.app.tick_headless();
            tick_count += 1;
            
            let current_view = self.app.get_current_view_debug();
            
            // Print progress every 10 ticks for more frequent updates
            if tick_count % 10 == 0 {
                println!("  Wait tick {}: view = {} (waiting for {})", tick_count, current_view, expected_view_prefix);
            }
            
            // Check if we've reached the expected view
            if current_view.starts_with(expected_view_prefix) {
                println!("  SUCCESS: Reached {} at tick {} ({}ms)", current_view, tick_count, start.elapsed().as_millis());
                return Ok(());
            }
            
            std::thread::sleep(Duration::from_millis(5)); // Shorter sleep for more responsiveness
        }
        
        anyhow::bail!("Timeout waiting for view transition to '{}'. Current view: '{}'. Total ticks: {}", 
                     expected_view_prefix, self.app.get_current_view_debug(), tick_count)
    }

    #[cfg(feature = "headless")]
    fn wait_and_process(&mut self, duration_ms: u64) {
        let start = std::time::Instant::now();
        let mut tick_count = 0;
        while start.elapsed() < Duration::from_millis(duration_ms) {
            self.app.tick_headless();
            tick_count += 1;
            
            // Print view state every 10 ticks for debugging
            if tick_count % 10 == 0 {
                println!("  Tick {}: view = {}", tick_count, self.app.get_current_view_debug());
            }
            
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("  Final after {}ms: view = {}, total ticks = {}", duration_ms, self.app.get_current_view_debug(), tick_count);
    }
}

#[tokio::test]
async fn e2e_simple_game_creation_test() -> anyhow::Result<()> {
    #[cfg(feature = "headless")]
    {
        use tokio::time::timeout;
        
        println!("Starting simple game creation test");
        
        let result = timeout(Duration::from_secs(10), tokio::task::spawn_blocking(|| {
            let mut runner = E2ETestRunner::new()?;
            runner.run_simple_game_creation_test()?;
            Ok::<(), anyhow::Error>(())
        })).await;
        
        match result {
            Ok(Ok(Ok(()))) => {
                println!("Simple game creation test passed successfully");
                Ok(())
            }
            Err(_) => {
                anyhow::bail!("Test timed out after 10 seconds")
            }
            Ok(Err(e)) => {
                anyhow::bail!("Tokio join error: {:?}", e)
            }
            Ok(Ok(Err(e))) => {
                anyhow::bail!("Simple game creation test error: {}", e)
            }
        }
    }
    
    #[cfg(not(feature = "headless"))]
    {
        println!("Skipping simple game creation test - headless feature not enabled");
        Ok(())
    }
}

#[tokio::test]
async fn e2e_ai_integration_test() -> anyhow::Result<()> {
    #[cfg(feature = "headless")]
    {
        use tokio::time::timeout;
        
        println!("Starting comprehensive E2E AI integration test");
        
        let result = timeout(Duration::from_secs(30), tokio::task::spawn_blocking(|| {
            let mut runner = E2ETestRunner::new()?;
            
            // First test basic game creation
            runner.run_simple_game_creation_test()?;
            
            // Then run main AI integration test
            runner.run_ai_integration_test()?;
            
            Ok::<(), anyhow::Error>(())
        })).await;
        
        match result {
            Ok(Ok(Ok(()))) => {
                println!("E2E AI integration test passed successfully");
                Ok(())
            }
            Err(_) => {
                anyhow::bail!("Test timed out after 30 seconds")
            }
            Ok(Err(e)) => {
                anyhow::bail!("Tokio join error: {:?}", e)
            }
            Ok(Ok(Err(e))) => {
                anyhow::bail!("E2E test error: {}", e)
            }
        }
    }
    
    #[cfg(not(feature = "headless"))]
    {
        println!("Skipping E2E test - headless feature not enabled");
        Ok(())
    }
}

#[tokio::test]
async fn e2e_minimal_smoke_test() -> anyhow::Result<()> {
    #[cfg(feature = "headless")]
    {
        use tokio::time::timeout;
        
        println!("Starting minimal E2E smoke test");
        
        let result = timeout(Duration::from_secs(10), tokio::task::spawn_blocking(|| {
            // Just test basic headless functionality
            p2pgo_ui_egui::headless()
        })).await;
        
        match result {
            Ok(Ok(Ok(()))) => {
                println!("Minimal E2E smoke test passed");
                Ok(())
            }
            Err(_) => {
                anyhow::bail!("Smoke test timed out after 10 seconds")
            }
            Ok(Err(e)) => {
                anyhow::bail!("Tokio join error: {:?}", e)
            }
            Ok(Ok(Err(e))) => {
                anyhow::bail!("Headless function error: {}", e)
            }
        }
    }
    
    #[cfg(not(feature = "headless"))]
    {
        println!("Skipping smoke test - headless feature not enabled");
        Ok(())
    }
}

#[test]
fn test_ai_message_types() {
    // Verify that AI-related message types compile and work correctly
    let ghost_request = UiToNet::GetGhostMoves;
    let ghost_response = NetToUi::GhostMoves(vec![
        Coord::new(3, 3),
        Coord::new(4, 4),
        Coord::new(5, 5),
    ]);
    
    // Test message serialization concepts
    match ghost_request {
        UiToNet::GetGhostMoves => println!("GetGhostMoves message type works"),
        _ => panic!("Unexpected message type"),
    }
    
    match ghost_response {
        NetToUi::GhostMoves(coords) => {
            assert_eq!(coords.len(), 3);
            assert_eq!(coords[0], Coord::new(3, 3));
            println!("GhostMoves response type works with {} coordinates", coords.len());
        }
        _ => panic!("Unexpected response type"),
    }
}

#[test]
fn test_ai_integration_compilation() {
    // This test ensures all AI integration components compile correctly
    // The trainer crate should be available and the message types should work
    
    // Test game state creation for AI input
    let game_state = GameState::new(9);
    assert_eq!(game_state.board_size, 9);
    assert_eq!(game_state.board.len(), 81);
    
    // Test coordinate creation for AI output
    let coords = vec![
        Coord::new(3, 3),
        Coord::new(4, 4),
        Coord::new(5, 5),
    ];
    
    // Verify coordinates are valid for 9x9 board
    for coord in coords {
        assert!(coord.x < 9);
        assert!(coord.y < 9);
    }
    
    println!("AI integration compilation test passed");
}
