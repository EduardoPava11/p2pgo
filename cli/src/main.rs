// SPDX-License-Identifier: MIT OR Apache-2.0

//! P2P Go CLI - Command-line interface for testing
//!
//! This binary provides a headless interface for running and testing
//! the P2P Go game without the UI. It's primarily used for integration
//! tests and automated testing.

// Initialize logging at the start of the program
use flexi_logger::{Logger, FileSpec, Naming, Cleanup, Criterion};
use std::path::PathBuf;
use anyhow::{Result, anyhow};

// Initialize logging functionality first thing in the program
fn init_logging() -> Result<()> {
    // Get log directory
    let log_dir = match std::env::consts::OS {
        "macos" => {
            let mut path = PathBuf::from(std::env::var("HOME")?);
            path.push("Library");
            path.push("Logs");
            path.push("p2pgo-cli");
            path
        },
        _ => {
            let mut path = PathBuf::from(".");
            path.push("logs");
            path
        }
    };
    
    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir)?;
    
    // Configure and start the logger
    Logger::try_with_str("info")?
        .log_to_file(
            FileSpec::default()
                .directory(&log_dir)
                .basename("p2pgo-cli")
                .suffix("log")
        )
        .rotate(
            Criterion::Size(1024 * 1024 * 1024), // 1GB per file
            Naming::Timestamps,
            Cleanup::KeepLogFiles(5), // Keep 5 files
        )
        // Process ID is already included in the log format
        // Error context is added via tracing subscriber
        .start()?;
    
    Ok(())
}

// Initialize logging as the first action
#[allow(unused_variables)]
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

fn ensure_logging_initialized() -> Result<()> {
    let mut result = Ok(());
    LOGGER_INIT.call_once(|| {
        match init_logging() {
            Ok(_) => {},
            Err(e) => {
                result = Err(e);
            }
        }
    });
    
    result
}

mod render;

use clap::{Parser, ValueEnum};
use tokio::signal;
use tokio::io::AsyncBufReadExt;
use p2pgo_core::{GameState, Move, Coord};
use p2pgo_network::{
    Lobby,
    GameChannel,
};

/// Command-line arguments
#[derive(Parser, Debug)]
#[clap(
    name = "p2pgo-cli",
    about = "P2P Go game command-line interface",
    version
)]
struct Args {
    /// The role of this instance
    #[clap(short, long, value_enum)]
    role: Option<Role>,
    
    /// Game ID for joining an existing game
    #[clap(short, long)]
    game_id: Option<uuid::Uuid>,
    
    /// Board size (9, 13, or 19)
    #[clap(short, long, default_value = "19")]
    size: u8,
    
    /// List available games and exit
    #[clap(long)]
    list: bool,
    
    /// Path to engine executable (future feature)
    #[clap(long)]
    engine: Option<String>,
    
    /// Enable debug logging and blob echo
    #[clap(long)]
    debug: bool,
    
    /// Run as spectator-only seed node (no game participation)
    #[clap(long)]
    spectator: bool,
    
    /// Connect directly using a ticket string
    #[clap(long)]
    ticket: Option<String>,
    
    /// Maximum connections allowed to the relay (default: 200)
    #[clap(long)]
    relay_max_conns: Option<usize>,
    
    /// Maximum bandwidth in Mbps for the relay (default: 10 MB/s)
    #[clap(long)]
    relay_max_mbps: Option<f64>,
}

/// Role of this instance
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Role {
    /// Host a new game
    Host,
    /// Join an existing game
    Join,
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging as the first action in main
    if let Err(e) = ensure_logging_initialized() {
        eprintln!("Warning: Failed to initialize logging: {}", e);
    }
    
    // Parse command-line arguments
    let args = Args::parse();
    
    // Initialize crash logger
    if let Err(e) = p2pgo_network::init_crash_logger().await {
        eprintln!("Warning: Failed to initialize crash logger: {}", e);
    }
    
    // Setup global panic handler
    std::panic::set_hook(Box::new(|panic_info| {
        let error = format!("{}", panic_info);
        let context = format!("CLI panic in thread: {:?}", std::thread::current().name());
        
        // Clone for async usage
        let error_clone = error.clone();
        let context_clone = context.clone();
        
        // Log the crash asynchronously
        tokio::spawn(async move {
            if let Err(e) = p2pgo_network::log_crash(&error_clone, &context_clone).await {
                eprintln!("Failed to log crash: {}", e);
            }
        });
        
        eprintln!("PANIC: {}", error);
    }));
    
    // Setup debug logging if requested
    if args.debug {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
        println!("Debug mode enabled - blob hashes will be printed");
    }
    
    // Validate board size
    if ![9, 13, 19].contains(&args.size) {
        return Err(anyhow!("Invalid board size. Must be 9, 13, or 19."));
    }
    
    // Handle engine option (future feature)
    if let Some(engine_path) = args.engine {
        println!("Engine integration not yet implemented: {}", engine_path);
        todo!("Engine integration will be added in future versions");
    }
    
    // Create a lobby service
    let lobby = Lobby::new();
    
    // Initialize Iroh context
    let iroh_ctx = p2pgo_network::IrohCtx::new().await?;
    
    // Handle ticket connection if provided
    if let Some(ticket) = args.ticket.as_ref() {
        println!("Connecting via ticket: {}", ticket);
        iroh_ctx.connect_by_ticket(ticket).await?;
        println!("Connection established successfully");
        
        // Generate our own ticket for the other player to connect back
        match iroh_ctx.ticket().await {
            Ok(my_ticket) => println!("Your ticket: {}", my_ticket),
            Err(e) => println!("Warning: Failed to generate ticket: {}", e),
        }
        
        // Wait a moment for game advertisements to arrive
        println!("Waiting for game advertisement...");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        // Check for available games
        let games = lobby.list_games().await;
        if games.is_empty() {
            println!("No games available after connection. Try refreshing or creating a game.");
        } else {
            println!("Available games after connection:");
            for game in &games {
                println!("  {} - {}x{} - Started: {}", 
                    game.id, game.board_size, game.board_size, game.started);
            }
            
            // If there's a game, join it automatically
            if let Some(game) = games.first() {
                println!("Auto-joining game: {}", game.id);
                
                // Get the game channel
                let channel = lobby.get_game_channel(&game.id).await?;
                
                // Get current state 
                let game_state = channel.get_latest_state().await
                    .ok_or_else(|| anyhow!("Failed to get current game state"))?;
                
                // Run the game loop
                return run_game_loop(game_state, channel, lobby, game.id.clone(), args.debug).await;
            }
        }
    }
    
    // Handle list command
    if args.list {
        let games = lobby.list_games().await;
        if games.is_empty() {
            println!("No games available.");
        } else {
            println!("Available games:");
            for game in games {
                println!("  {} - {}x{} - Started: {}", 
                    game.id, game.board_size, game.board_size, game.started);
            }
        }
        return Ok(());
    }
    
    // Handle spectator mode
    if args.spectator {
        println!("Starting spectator-only seed node...");
        
        // Generate and display a ticket for others to connect
        match iroh_ctx.ticket().await {
            Ok(ticket) => {
                println!("Spectator seed node ticket (share with players):\n{}", ticket);
                println!("This node will relay network traffic without participating in games.");
            }
            Err(e) => println!("Warning: Failed to generate ticket: {}", e),
        }
        
        // Keep the node running as a relay seed
        println!("Running as spectator seed node. Press Ctrl+C to stop.");
        
        // Just wait for shutdown signal
        signal::ctrl_c().await.expect("Failed to listen for shutdown signal");
        println!("Shutting down spectator seed node...");
        return Ok(());
    }
    
    // Role is required if not listing and not using a ticket
    let role = if args.ticket.is_some() {
        // Default to host if ticket is provided but no role
        args.role.unwrap_or(Role::Host)
    } else {
        args.role.ok_or_else(|| anyhow!("Role is required (--role host or --role join)"))?
    };
    
    match role {
        Role::Host => {
            // Create a new game
            println!("Creating new game with board size {}", args.size);
            
            // Create the game
            let game_id = lobby.create_game(None, args.size, false).await?;
            println!("Game created with ID: {}", game_id);
            
            // Advertise the game via gossip
            match iroh_ctx.advertise_game(&game_id, args.size).await {
                Ok(_) => println!("Game advertisement broadcast successfully"),
                Err(e) => println!("Warning: Failed to advertise game: {}", e),
            }
            
            // Generate and display a ticket for direct connections
            match iroh_ctx.ticket().await {
                Ok(ticket) => println!("Share this ticket with opponent:\n{}", ticket),
                Err(e) => println!("Warning: Failed to generate ticket: {}", e),
            }
            
            // Get the game channel
            let channel = lobby.get_game_channel(&game_id).await?;
            
            // Get initial state 
            let game_state = channel.get_latest_state().await
                .ok_or_else(|| anyhow!("Failed to get initial game state"))?;
            
            // Run the game loop
            run_game_loop(game_state, channel, lobby, game_id, args.debug).await?;
        }
        Role::Join => {
            // Check if we have a game ID
            let game_id = args.game_id.ok_or_else(|| anyhow!("Game ID is required when joining a game"))?;
            let game_id_str = game_id.to_string();
            
            println!("Joining game with ID: {}", game_id_str);
            
            // Get the game channel
            let channel = lobby.get_game_channel(&game_id_str).await?;
            
            // Get current state 
            let game_state = channel.get_latest_state().await
                .ok_or_else(|| anyhow!("Failed to get current game state"))?;
            
            // Run the game loop
            run_game_loop(game_state, channel, lobby, game_id_str, args.debug).await?;
        }
    }
    
    Ok(())
}

/// Run the main game loop
async fn run_game_loop(
    mut game_state: GameState, 
    channel: std::sync::Arc<GameChannel>,
    lobby: Lobby,
    game_id: String,
    debug: bool
) -> Result<()> {
    // Print the initial game state
    print_game_state(&game_state);
    
    // Set up subscription to game events
    let mut event_rx = channel.subscribe();
    
    // Set up a channel for handling Ctrl+C
    let mut stdin_lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    
    loop {
        println!("\n{:?} to move. Enter a move (e.g., 'D4'), 'pass', or 'resign':", game_state.current_player);
        
        tokio::select! {
            // Handle Ctrl+C gracefully
            _ = signal::ctrl_c() => {
                println!("\nReceived Ctrl+C, shutting down gracefully...");
                break;
            }
            
            // Handle user input for moves
            result = stdin_lines.next_line() => {
                let line = match result {
                    Ok(Some(line)) => line.trim().to_string(),
                    Ok(None) => break, // EOF
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        continue;
                    }
                };
                
                // Parse the input
                let mv = match parse_move(&line, game_state.board_size) {
                    Ok(mv) => mv,
                    Err(e) => {
                        eprintln!("Invalid move: {}", e);
                        continue;
                    }
                };
                
                // Keep a backup of the current state for rollback
                let backup_state = game_state.clone();
                
                // Apply the move locally first
                if let Err(e) = game_state.apply_move(mv.clone()) {
                    eprintln!("Invalid move: {}", e);
                    continue;
                }
                
                // Send the move to the network
                if let Err(e) = lobby.post_move(&game_id, mv).await {
                    eprintln!("Failed to send move: {}", e);
                    // Roll back to the backup state
                    game_state = backup_state;
                    continue;
                }
                
                // Print the updated game state
                print_game_state(&game_state);
                
                // Check if the game is over
                if game_state.is_game_over() {
                    println!("Game over!");
                    break;
                }
            }
            
            // Handle network events
            event = event_rx.recv() => {
                if let Ok(event) = event {
                    println!("Received event: {:?}", event);
                    
                    match event {
                        p2pgo_core::GameEvent::MoveMade { mv, by } => {
                            println!("Move made by {:?}: {:?}", by, mv);
                            
                            // Print blob hash in debug mode
                            if debug {
                                // Create a simple hash representation for debug display
                                let debug_hash = format!("{:08x}", 
                                    format!("{:?}", mv).chars()
                                        .map(|c| c as u32)
                                        .fold(0u32, |acc, x| acc.wrapping_mul(31).wrapping_add(x))
                                );
                                println!("DEBUG: Blob hash: {}", debug_hash);
                            }
                            
                            if let Err(e) = game_state.apply_move(mv) {
                                eprintln!("Failed to apply remote move: {}", e);
                            } else {
                                print_game_state(&game_state);
                            }
                        }
                        p2pgo_core::GameEvent::GameEnded { .. } => {
                            println!("Game over!");
                            break;
                        }
                        _ => {
                            // Handle other events as needed
                        }
                    }
                } else {
                    // Channel closed
                    println!("Connection closed.");
                    break;
                }
            }
        }
    }
    
    Ok(())
}

/// Parse a move from a string
fn parse_move(input: &str, board_size: u8) -> Result<Move> {
    let input = input.to_lowercase();
    
    if input == "pass" {
        return Ok(Move::Pass);
    } else if input == "resign" {
        return Ok(Move::Resign);
    }
    
    // Parse coordinate like "D4"
    if input.len() >= 2 {
        let col_char = input.chars().next().unwrap();
        let row_str = &input[1..];
        
        // Parse column (A-T, skipping I)
        let col = if ('a'..='h').contains(&col_char) {
            col_char as u8 - b'a'
        } else if ('j'..='t').contains(&col_char) {
            col_char as u8 - b'a' - 1 // Skip 'i'
        } else {
            return Err(anyhow!("Invalid column. Must be A-T (excluding I)."));
        };
        
        // Parse row (1-19)
        let row = match row_str.parse::<u8>() {
            Ok(r) if r > 0 && r <= board_size => r - 1, // Convert to 0-indexed
            _ => return Err(anyhow!("Invalid row. Must be between 1 and {}.", board_size)),
        };
        
        // Check if the coordinate is valid
        if col < board_size && row < board_size {
            return Ok(Move::Place { x: col, y: row, color: Color::Black }); // Color will be overridden by the game
        }
    }
    
    Err(anyhow!("Invalid move format. Examples: 'D4', 'pass', 'resign'."))
}

/// Print the current game state
fn print_game_state(game_state: &GameState) {
    println!("\nBoard size: {}x{}", game_state.board_size, game_state.board_size);
    println!("Current player: {:?}", game_state.current_player);
    println!("Moves: {}", game_state.moves.len());
    println!("Captures: Black {} - White {}", game_state.captures.0, game_state.captures.1);
    
    // Render the ASCII board
    println!("\n{}", render::render_board(game_state));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore = "Implementation needed"]
    fn test_parse_move() {
        // Test parsing various move formats
        assert!(matches!(parse_move("D4", 19).unwrap(), Move::Place(coord) if coord.x == 3 && coord.y == 3));
        assert!(matches!(parse_move("pass", 19).unwrap(), Move::Pass));
        assert!(matches!(parse_move("resign", 19).unwrap(), Move::Resign));
        assert!(parse_move("Z9", 19).is_err()); // Invalid column
    }
}
