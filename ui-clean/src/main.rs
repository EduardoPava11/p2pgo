//! Clean P2P Go implementation with working UI

mod ui;
mod network;
mod game;

use eframe::egui;
fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // UI options
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 900.0)),
        centered: true,
        ..Default::default()
    };
    
    // Run the app
    eframe::run_native(
        "P2P Go",
        options,
        Box::new(|cc| Box::new(ui::P2PGoApp::new(cc))),
    )
}