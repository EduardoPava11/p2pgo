// SPDX-License-Identifier: MIT OR Apache-2.0

//! Main entry point for the egui UI

use clap::Parser;
use crossbeam_channel::unbounded;
use flexi_logger::{Logger, FileSpec, Naming, Cleanup, Criterion};
use std::path::PathBuf;
use anyhow::Result;

mod app;
mod view;
mod msg;
mod board_widget;
mod worker;

use app::App;
use msg::{UiToNet, NetToUi};

#[derive(Parser)]
#[command(name = "p2pgo-ui-egui")]
#[command(about = "Peer-to-peer Go game with egui UI")]
struct Args {
    #[arg(long, default_value = "9")]
    board_size: u8,
    
    #[arg(long, default_value = "Player")]
    player_name: String,
    
    #[arg(long)]
    debug: bool,
    
    #[arg(long, help = "Connect directly using a ticket string")]
    ticket: Option<String>,
}

/// Initialize the application logging system with rotation
fn init_logging(debug: bool) -> Result<()> {
    // Get log directory
    let log_dir = match std::env::consts::OS {
        "macos" => {
            let mut path = PathBuf::from(std::env::var("HOME")?);
            path.push("Library");
            path.push("Logs");
            path.push("p2pgo");
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
    let level = if debug { "debug" } else { "info" };
    
    Logger::try_with_str(level)?
        .log_to_file(
            FileSpec::default()
                .directory(&log_dir)
                .basename("p2pgo")
                .suffix("log")
        )
        .rotate(
            Criterion::Size(1024 * 1024 * 1024), // 1GB per file
            Naming::Timestamps,
            Cleanup::KeepLogFiles(5), // Keep 5 files
        )
        .start()?;
    
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize logging with rotation
    if let Err(e) = init_logging(args.debug) {
        eprintln!("Warning: Failed to initialize logging: {}", e);
    }
    
    // Initialize crash logger asynchronously
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = p2pgo_network::init_crash_logger().await {
            eprintln!("Warning: Failed to initialize crash logger: {}", e);
        }
    });
    
    // Setup global panic handler
    std::panic::set_hook(Box::new(|panic_info| {
        let error = format!("{}", panic_info);
        let context = format!("UI panic in thread: {:?}", std::thread::current().name());
        
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
    
    // Setup channels for communication between UI and worker
    let (ui_tx, net_rx) = unbounded::<UiToNet>();
    let (net_tx, ui_rx) = unbounded::<NetToUi>();
    
    // Capture values before moving into closure
    let board_size = args.board_size;
    let player_name = args.player_name.clone();
    let ticket = args.ticket.clone();
    
    // Spawn background worker
    let worker_handle = worker::spawn_worker(net_rx, net_tx.clone(), board_size, player_name.clone())?;
    
    // If ticket is provided, connect on startup
    if let Some(ticket_str) = ticket {
        // Short delay to allow worker to initialize
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = net_tx.send(msg::NetToUi::Debug(format!("Connecting via ticket: {}", &ticket_str)));
        let _ = ui_tx.send(msg::UiToNet::ConnectByTicket { ticket: ticket_str });
    }
    
    // Launch egui app
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "P2P Go",
        options,
        Box::new(move |_cc| {
            Box::new(App::new(ui_tx, ui_rx, board_size, player_name))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run eframe: {}", e))
}

#[cfg(feature = "headless")]
pub fn headless() -> anyhow::Result<()> {
    use crate::msg::UiToNet;
    use p2pgo_core::{Move, Coord};
    
    let (ui_tx, net_rx) = unbounded();
    let (net_tx, ui_rx) = unbounded();
    
    // Start worker in a thread with controlled shutdown
    let worker_handle = std::thread::spawn(|| {
        let _ = worker::start(net_rx, net_tx);
    });
    
    let mut app = App::new_headless_with_channels(ui_tx.clone(), ui_rx);
    app.set_worker_handle(worker_handle);
    
    // Create a game
    let _ = ui_tx.send(UiToNet::CreateGame { board_size: 9 });
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Simulate the game moves: B D4, W F4, B E5, W pass, B pass
    let moves = vec![
        Move::Place(Coord::new(3, 3)), // D4 - Black
        Move::Place(Coord::new(5, 3)), // F4 - White  
        Move::Place(Coord::new(4, 4)), // E5 - Black
        Move::Pass,                    // White pass
        Move::Pass,                    // Black pass
    ];
    
    for (i, mv) in moves.iter().enumerate() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = ui_tx.send(UiToNet::MakeMove { mv: mv.clone(), board_size: None });
        app.tick_headless();
        
        // Store the final state for testing
        if i == moves.len() - 1 {
            // Final move - store the state
            if let Ok(state) = app.get_current_game_state() {
                p2pgo_network::debug::store_latest_reconstructed(state);
            }
        }
    }
    
    // Continue ticking for a bit to ensure everything is processed
    for _ in 0..10 {
        app.tick_headless();
    }
    
    // Clean shutdown
    let _ = ui_tx.send(UiToNet::Shutdown);
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    Ok(())
}
