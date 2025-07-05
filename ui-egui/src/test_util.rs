#![cfg(test)]
use crate::msg::{NetToUi, UiToNet};
use p2pgo_core::GameState;

/// Create test apps for integration testing
pub fn host_guest_pair() -> (TestApp, TestApp) {
    // Create two TestApps that can respond to messages directly
    let host = TestApp::new("HostPlayer".to_string());
    let guest = TestApp::new("GuestPlayer".to_string());

    // Setup game state
    host.create_game();

    (host, guest)
}

/// Test application that simulates both UI and network functionality
pub struct TestApp {
    game_state: Option<GameState>,
    game_id: Option<String>,
    last_message: Option<NetToUi>,
}

impl TestApp {
    pub fn new(_name: String) -> Self {
        Self {
            game_state: None,
            game_id: Some("test-game-id".to_string()),
            last_message: None,
        }
    }

    pub fn create_game(&self) {
        // Just set up the test environment
    }

    pub fn ui_send(&mut self, msg: UiToNet) {
        match msg {
            UiToNet::MakeMove { mv, board_size } => {
                let board_size = board_size.unwrap_or(9);
                if let Some(gs) = &mut self.game_state {
                    let _ = gs.apply_move(mv);
                } else {
                    self.game_state = Some(GameState::new(board_size));
                    let _ = self.game_state.as_mut().unwrap().apply_move(mv);
                }
            }
            UiToNet::CalculateScore { dead_stones } => {
                // Calculate score and return a score proof
                let gs = self
                    .game_state
                    .as_ref()
                    .unwrap_or_else(|| panic!("No game state available"));

                let score_proof = p2pgo_core::scoring::calculate_final_score(
                    gs,
                    6.5,
                    p2pgo_core::value_labeller::ScoringMethod::Territory,
                    &dead_stones,
                );

                self.last_message = Some(NetToUi::ScoreCalculated { score_proof });
            }
            UiToNet::AcceptScore { score_proof } => {
                // Simulate accepting the score
                self.last_message = Some(NetToUi::ScoreAcceptedByBoth { score_proof });
            }
            _ => {}
        }
    }

    pub fn tick_headless(&mut self) {
        // Nothing to do in test mode
    }

    pub fn recv_last(&mut self) -> Option<NetToUi> {
        self.last_message.take()
    }

    pub fn get_current_game_id(&self) -> Option<String> {
        self.game_id.clone()
    }
}
