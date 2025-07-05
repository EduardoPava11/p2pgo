//! Game control buttons and interactions

use crate::msg::UiToNet;
use crossbeam_channel::Sender;
use egui::Ui;
use p2pgo_core::Move;

/// Game control buttons
pub struct GameControls;

impl GameControls {
    /// Render game control buttons
    pub fn render(ui: &mut Ui, ui_tx: &Sender<UiToNet>, games_finished: u32) -> Option<UiToNet> {
        let mut action = None;

        ui.horizontal(|ui| {
            if ui.button("Pass").clicked() {
                let _ = ui_tx.send(UiToNet::MakeMove {
                    mv: Move::Pass,
                    board_size: None,
                });

                // Only request ghost moves if threshold met
                if games_finished >= 5 {
                    let _ = ui_tx.send(UiToNet::GetGhostMoves);
                }
            }

            if ui.button("Resign").clicked() {
                let _ = ui_tx.send(UiToNet::MakeMove {
                    mv: Move::Resign,
                    board_size: None,
                });

                if games_finished >= 5 {
                    let _ = ui_tx.send(UiToNet::GetGhostMoves);
                }
            }

            ui.separator();

            if ui.button("Leave Game").clicked() {
                action = Some(UiToNet::LeaveGame);
                let _ = ui_tx.send(UiToNet::LeaveGame);
                let _ = ui_tx.send(UiToNet::Shutdown);
            }
        });

        action
    }
}
