//! Active game view with board and controls

use egui::{Align, Color32, Frame, Layout, RichText, Ui, Vec2, Widget};
use p2pgo_core::{Coord, GameState, Move, Color};
use crate::core::{Colors, Spacing, Styles, primary_button, secondary_button, danger_button, Card};
use crate::widgets::{BoardWidget, NeuralPanel};

pub struct GameView {
    pub game: GameState,
    pub show_heat_map: bool,
    pub neural_panel: NeuralPanel,
    pub game_code: Option<String>,
    pub is_our_turn: bool,
    pub opponent_name: String,
}

impl GameView {
    pub fn new(game: GameState) -> Self {
        Self {
            game,
            show_heat_map: false,
            neural_panel: NeuralPanel::new(),
            game_code: None,
            is_our_turn: true,
            opponent_name: "Opponent".to_string(),
        }
    }
    
    pub fn show(&mut self, ui: &mut Ui, neural_net: &p2pgo_neural::DualNeuralNet) -> GameAction {
        let mut action = GameAction::None;
        
        // Main game layout
        ui.horizontal(|ui| {
            // Left side - Game info and controls
            ui.vertical(|ui| {
                ui.set_width(200.0);
                
                // Game info card
                Card::new().show(ui, |ui| {
                    ui.heading("Game Info");
                    ui.separator();
                    
                    if let Some(code) = &self.game_code {
                        ui.horizontal(|ui| {
                            ui.label("Code:");
                            ui.label(RichText::new(code).family(egui::FontFamily::Monospace));
                        });
                    }
                    
                    ui.horizontal(|ui| {
                        ui.label("Opponent:");
                        ui.label(&self.opponent_name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Turn:");
                        let turn_text = if self.is_our_turn { "Your turn" } else { "Opponent's turn" };
                        let turn_color = if self.is_our_turn { Colors::SUCCESS } else { Colors::TEXT_SECONDARY };
                        ui.label(RichText::new(turn_text).color(turn_color));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Move:");
                        ui.label(format!("{}", self.game.moves.len()));
                    });
                });
                
                ui.add_space(Spacing::MD);
                
                // Captures card
                Card::new().show(ui, |ui| {
                    ui.heading("Captures");
                    ui.separator();
                    
                    let (black_captures, white_captures) = self.game.captures;
                    
                    ui.horizontal(|ui| {
                        ui.label("●");
                        ui.label(format!("Black: {}", black_captures));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("○");
                        ui.label(format!("White: {}", white_captures));
                    });
                });
                
                ui.add_space(Spacing::MD);
                
                // Game controls
                Card::new().show(ui, |ui| {
                    ui.heading("Controls");
                    ui.separator();
                    
                    ui.vertical_centered(|ui| {
                        if primary_button("Pass")
                            .enabled(self.is_our_turn)
                            .min_width(150.0)
                            .ui(ui)
                            .clicked()
                        {
                            action = GameAction::Pass;
                        }
                        
                        ui.add_space(Spacing::SM);
                        
                        if secondary_button("Undo")
                            .enabled(self.game.moves.len() > 0)
                            .min_width(150.0)
                            .ui(ui)
                            .clicked()
                        {
                            action = GameAction::Undo;
                        }
                        
                        ui.add_space(Spacing::SM);
                        
                        if danger_button("Resign")
                            .enabled(self.is_our_turn)
                            .min_width(150.0)
                            .ui(ui)
                            .clicked()
                        {
                            action = GameAction::Resign;
                        }
                    });
                });
                
                ui.add_space(Spacing::MD);
                
                // Neural controls
                Card::new().show(ui, |ui| {
                    ui.heading("AI Assistant");
                    ui.separator();
                    
                    ui.checkbox(&mut self.show_heat_map, "Show Heat Map (H)");
                    
                    ui.label(
                        RichText::new("Heat map shows AI move suggestions")
                            .small()
                            .color(Colors::TEXT_SECONDARY)
                    );
                });
            });
            
            // Center - Game board
            ui.vertical(|ui| {
                ui.set_min_width(500.0);
                
                // Get heat map if enabled
                let heat_map = if self.show_heat_map {
                    let predictions = neural_net.predict_moves(&self.game);
                    
                    // Convert predictions to 9x9 array
                    let mut heat_map = [[0.0f32; 9]; 9];
                    for pred in predictions {
                        if pred.coord.x < 9 && pred.coord.y < 9 {
                            heat_map[pred.coord.y as usize][pred.coord.x as usize] = pred.probability;
                        }
                    }
                    Some(heat_map)
                } else {
                    None
                };
                
                let mut board_widget = BoardWidget::new(&self.game);
                
                if let Some(ref heat_map) = heat_map {
                    board_widget = board_widget.heat_map(heat_map);
                }
                
                let board_response = board_widget
                    .interactive(self.is_our_turn)
                    .show(ui);
                
                if let Some(pos) = board_response.clicked_position {
                    if self.is_our_turn {
                        // Check if position is empty
                        let idx = (pos.y as usize) * (self.game.board_size as usize) + (pos.x as usize);
                        if self.game.board[idx].is_none() {
                            action = GameAction::PlaceStone(pos);
                        }
                    }
                }
                
                // Update neural panel
                if ui.ctx().frame_nr() % 10 == 0 {
                    self.neural_panel.update_suggestions(neural_net, &self.game);
                }
            });
        });
        
        // Always-visible neural panel
        self.neural_panel.show(ui);
        
        action
    }
    
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::H) {
                self.show_heat_map = !self.show_heat_map;
            }
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameAction {
    None,
    PlaceStone(Coord),
    Pass,
    Undo,
    Resign,
}