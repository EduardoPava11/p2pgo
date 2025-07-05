//! Game lobby for creating and joining games

use crate::core::{
    primary_button, secondary_button, Card, Colors, LabeledInput, Spacing, StyledInput, Styles,
    Typography,
};
use egui::{Align, Color32, Frame, Layout, RichText, Ui, Vec2, Widget};

pub struct LobbyView {
    pub create_game_code: String,
    pub join_game_code: String,
    pub available_games: Vec<GameListing>,
    pub show_create_dialog: bool,
    pub show_join_dialog: bool,
}

#[derive(Clone)]
pub struct GameListing {
    pub code: String,
    pub host: String,
    pub board_size: u8,
    pub status: String,
}

impl LobbyView {
    pub fn new() -> Self {
        Self {
            create_game_code: String::new(),
            join_game_code: String::new(),
            available_games: Vec::new(),
            show_create_dialog: false,
            show_join_dialog: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> LobbyAction {
        let mut action = LobbyAction::None;

        // Center everything
        ui.vertical_centered(|ui| {
            ui.add_space(Spacing::XL);

            // Title
            ui.heading(RichText::new("P2P Go").size(32.0));
            ui.label(
                RichText::new("Decentralized Go with Neural Networks")
                    .size(16.0)
                    .color(Colors::TEXT_SECONDARY),
            );

            ui.add_space(Spacing::XL);

            // Main buttons
            ui.horizontal(|ui| {
                if primary_button("Create Game")
                    .size(crate::core::button::ButtonSize::Large)
                    .min_width(200.0)
                    .ui(ui)
                    .clicked()
                {
                    action = LobbyAction::CreateGame(String::new());
                }

                ui.add_space(Spacing::MD);

                if secondary_button("Join Game")
                    .size(crate::core::button::ButtonSize::Large)
                    .min_width(200.0)
                    .ui(ui)
                    .clicked()
                {
                    self.show_join_dialog = true;
                }
            });

            ui.add_space(Spacing::XL);

            // Active games list
            if !self.available_games.is_empty() {
                Card::new()
                    .elevation(crate::core::card::CardElevation::Low)
                    .show(ui, |ui| {
                        ui.heading("Active Games");
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for game in &self.available_games {
                                    self.render_game_listing(ui, game, &mut action);
                                }
                            });
                    });
            } else {
                ui.label(
                    RichText::new("No active games. Create one to start playing!")
                        .color(Colors::TEXT_SECONDARY),
                );
            }
        });

        // Create game dialog
        if self.show_create_dialog {
            egui::Window::new("Create Game")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.heading("New Game");
                        ui.separator();

                        ui.label("Share this code with your opponent:");

                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.create_game_code)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(200.0)
                                    .interactive(false),
                            );

                            if ui.button("📋 Copy").clicked() {
                                ui.output_mut(|o| o.copied_text = self.create_game_code.clone());
                            }
                        });

                        ui.add_space(Spacing::MD);

                        ui.label(
                            RichText::new("Waiting for opponent to join...")
                                .color(Colors::TEXT_SECONDARY),
                        );
                        ui.spinner();

                        ui.add_space(Spacing::MD);

                        ui.horizontal(|ui| {
                            if primary_button("Start Game")
                                .enabled(false) // Enable when opponent joins
                                .ui(ui)
                                .clicked()
                            {
                                action = LobbyAction::CreateGame(self.create_game_code.clone());
                                self.show_create_dialog = false;
                            }

                            if secondary_button("Cancel").ui(ui).clicked() {
                                self.show_create_dialog = false;
                            }
                        });
                    });
                });
        }

        // Join game dialog
        if self.show_join_dialog {
            egui::Window::new("Join Game")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Join Game");
                        ui.separator();

                        LabeledInput::new("Game Code", &mut self.join_game_code)
                            .hint_text("Enter game code")
                            .show(ui);

                        ui.add_space(Spacing::MD);

                        ui.horizontal(|ui| {
                            if primary_button("Join")
                                .enabled(!self.join_game_code.is_empty())
                                .ui(ui)
                                .clicked()
                            {
                                action = LobbyAction::JoinGame(self.join_game_code.clone());
                                self.show_join_dialog = false;
                            }

                            if secondary_button("Cancel").ui(ui).clicked() {
                                self.show_join_dialog = false;
                                self.join_game_code.clear();
                            }
                        });
                    });
                });
        }

        action
    }

    fn render_game_listing(&self, ui: &mut Ui, game: &GameListing, action: &mut LobbyAction) {
        Frame::none()
            .fill(Color32::from_rgb(
                (Colors::SURFACE.r() as f32 * 1.2).min(255.0) as u8,
                (Colors::SURFACE.g() as f32 * 1.2).min(255.0) as u8,
                (Colors::SURFACE.b() as f32 * 1.2).min(255.0) as u8,
            ))
            .inner_margin(Spacing::SM)
            .rounding(Styles::rounding())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&game.code).family(egui::FontFamily::Monospace));
                        ui.label(
                            RichText::new(format!("Host: {}", game.host))
                                .small()
                                .color(Colors::TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if secondary_button("Join").ui(ui).clicked() {
                            *action = LobbyAction::JoinGame(game.code.clone());
                        }

                        ui.label(&game.status);
                        ui.label(format!("{}x{}", game.board_size, game.board_size));
                    });
                });
            });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LobbyAction {
    None,
    CreateGame(String),
    JoinGame(String),
}

fn generate_game_code() -> String {
    // Simple 6-character game code
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..6)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}
