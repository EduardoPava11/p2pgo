//! Game status display

use egui::Ui;
use p2pgo_core::{Color, GameState};

/// Game status display component
pub struct GameStatus;

impl GameStatus {
    /// Render game status
    pub fn render(ui: &mut Ui, game_state: &GameState) {
        let current_player = match game_state.current_player {
            Color::Black => "Black",
            Color::White => "White",
        };

        ui.horizontal(|ui| {
            ui.label(format!("Current player: {}", current_player));

            ui.separator();

            // Show captures
            ui.label(format!(
                "Captures - Black: {} White: {}",
                game_state.captures.0, game_state.captures.1
            ));

            // Show move count
            ui.label(format!("Move: {}", game_state.moves.len()));
        });
    }
}
