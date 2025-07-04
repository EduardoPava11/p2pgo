// SPDX-License-Identifier: MIT OR Apache-2.0

//! View management for different screens.

use p2pgo_core::GameState;
use p2pgo_network::lobby::GameInfo;

/// Different views/screens in the application
#[derive(Debug, Clone)]
pub enum View {
    /// Main menu with create/join game options
    MainMenu {
        available_games: Vec<GameInfo>,
        creating_game: bool,
        board_size: u8,
    },
    /// Lobby waiting for opponent
    Lobby {
        game_id: String,
    },
    /// Active game in progress
    Game {
        game_id: String,
        game_state: GameState,
        #[allow(dead_code)]
        our_color: Option<p2pgo_core::Color>,
    },
    /// Score dialog at end of game
    ScoreDialog {
        game_id: String,
        game_state: GameState,
        score_proof: p2pgo_core::value_labeller::ScoreProof,
        dead_stones: std::collections::HashSet<p2pgo_core::Coord>,
        score_pending: bool,
        score_accepted: bool,
    },
    /// Offline game mode for testing
    OfflineGame,
}

impl Default for View {
    fn default() -> Self {
        View::MainMenu {
            available_games: Vec::new(),
            creating_game: false,
            board_size: 9,
        }
    }
}
