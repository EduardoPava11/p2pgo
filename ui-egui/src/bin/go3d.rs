//! 3D 9x9x9 Go Game Runner
//!
//! Run with: cargo run --bin go3d --features offline

use eframe::{Frame, NativeOptions, App as EframeApp};
use egui::Context;
use p2pgo_ui_egui::go3d::Go3DGame;

struct Go3DApp {
    game: Go3DGame,
}

impl Go3DApp {
    fn new() -> Self {
        Self {
            game: Go3DGame::new(),
        }
    }
}

impl EframeApp for Go3DApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.game.ui(ctx);
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    
    let options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1200.0, 800.0)),
        min_window_size: Some(egui::Vec2::new(1000.0, 700.0)),
        resizable: true,
        centered: true,
        vsync: true,
        ..Default::default()
    };
    
    eframe::run_native(
        "3D Go - 9×9×9",
        options,
        Box::new(|_cc| Box::new(Go3DApp::new())),
    )
}