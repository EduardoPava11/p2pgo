//! Offline 9x9 Go Game Runner
//!
//! Run with: cargo run --bin offline_game --features offline

use eframe::{Frame, NativeOptions, App as EframeApp};
use egui::Context;
use p2pgo_ui_egui::offline_game::OfflineGoGame;

struct OfflineApp {
    game: OfflineGoGame,
}

impl OfflineApp {
    fn new() -> Self {
        Self {
            game: OfflineGoGame::new(),
        }
    }
}

impl EframeApp for OfflineApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.game.ui(ctx);
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    
    let options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(900.0, 900.0)),
        min_window_size: Some(egui::Vec2::new(600.0, 600.0)),
        resizable: true,
        centered: true,
        vsync: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "P2P Go Offline",
        options,
        Box::new(|_cc| Box::new(OfflineApp::new())),
    )
}