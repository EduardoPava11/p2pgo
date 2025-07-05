//! P2P Go UI v2 - Clean architecture with proper abstraction layers

pub mod app;
pub mod core;
pub mod features;
pub mod training;
pub mod widgets;

// Re-export main app
pub use app::P2PGoApp;

use p2pgo_ui_egui::msg::{NetToUi, UiToNet};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

// Run function for the wrapper
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use eframe::NativeOptions;

    // Configure logging
    tracing_subscriber::fmt::init();

    // Setup channels for communication between UI and worker
    let (ui_tx, net_rx) = unbounded_channel::<UiToNet>();
    let (net_tx, ui_rx) = unbounded_channel::<NetToUi>();

    // Default values
    let board_size = 9u8;
    let player_name = "Player".to_string();

    // Spawn background worker
    let _worker_handle = p2pgo_ui_egui::worker::spawn_worker(
        net_rx,
        net_tx.clone(),
        board_size,
        player_name.clone(),
    )?;

    // Window options
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("P2P Go")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(get_icon()),
        ..Default::default()
    };

    // Run app
    eframe::run_native(
        "P2P Go",
        options,
        Box::new(move |cc| Ok(Box::new(P2PGoApp::new(cc, ui_tx, ui_rx)))),
    )?;

    Ok(())
}

fn get_icon() -> egui::IconData {
    // Simple black and white icon
    let size = 32usize;
    let mut pixels = vec![0u8; size * size * 4];

    // Draw a simple Go stone pattern
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let center = size as f32 / 2.0;
            let radius = size as f32 / 2.0 - 2.0;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < radius {
                // Black stone
                pixels[idx] = 20;
                pixels[idx + 1] = 20;
                pixels[idx + 2] = 20;
                pixels[idx + 3] = 255;
            } else {
                // Transparent
                pixels[idx + 3] = 0;
            }
        }
    }

    egui::IconData {
        rgba: pixels,
        width: size as u32,
        height: size as u32,
    }
}
