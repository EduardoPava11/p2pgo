//! Complete neural network UI for gameplay

use eframe::egui::{self, RichText, Color32};
use crate::heat_map::HeatMapOverlay;
use p2pgo_core::GameState;
use p2pgo_neural::BoardEvaluation;

/// Neural network game UI components
pub struct NeuralGameUI {
    /// Heat map overlay
    pub heat_map: HeatMapOverlay,
    /// Show evaluation panel
    pub show_evaluation: bool,
    /// Last board evaluation
    pub last_evaluation: Option<BoardEvaluation>,
}

impl NeuralGameUI {
    pub fn new() -> Self {
        Self {
            heat_map: HeatMapOverlay::new(),
            show_evaluation: true,
            last_evaluation: None,
        }
    }
    
    /// Render neural network controls
    pub fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("🧠 Neural Assistant:");
            
            // Heat map toggle
            let heat_text = if self.heat_map.is_enabled() {
                RichText::new("Heat Map ON").color(Color32::from_rgb(255, 100, 100))
            } else {
                RichText::new("Heat Map OFF").color(Color32::GRAY)
            };
            
            if ui.button(heat_text).clicked() {
                self.heat_map.toggle();
            }
            
            ui.label("(Press H)");
            
            ui.separator();
            
            // Evaluation toggle
            ui.toggle_value(&mut self.show_evaluation, "Show Evaluation");
        });
    }
    
    /// Render position evaluation
    pub fn render_evaluation(&self, ui: &mut egui::Ui, game_state: &GameState) {
        if !self.show_evaluation {
            return;
        }
        
        ui.group(|ui| {
            ui.label(RichText::new("📊 Position Evaluation").strong());
            
            if let Some(eval) = &self.last_evaluation {
                // Win probability bar
                ui.horizontal(|ui| {
                    ui.label("Win %:");
                    
                    let win_pct = (eval.win_probability + 1.0) / 2.0 * 100.0;
                    let bar_width = 200.0;
                    let bar_height = 20.0;
                    
                    let (rect, _) = ui.allocate_space(egui::Vec2::new(bar_width, bar_height));
                    
                    // Background
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        Color32::from_gray(50),
                    );
                    
                    // Fill based on win probability
                    let fill_width = bar_width * (win_pct / 100.0);
                    let fill_color = if win_pct > 50.0 {
                        Color32::from_rgb(0, 200, 0)
                    } else {
                        Color32::from_rgb(200, 0, 0)
                    };
                    
                    let fill_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::Vec2::new(fill_width, bar_height),
                    );
                    
                    ui.painter().rect_filled(
                        fill_rect,
                        2.0,
                        fill_color,
                    );
                    
                    // Text overlay
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("{:.1}%", win_pct),
                        egui::FontId::default(),
                        Color32::WHITE,
                    );
                });
                
                // Confidence
                ui.horizontal(|ui| {
                    ui.label("Confidence:");
                    let confidence_text = match (eval.confidence * 100.0) as u32 {
                        0..=30 => "Low",
                        31..=70 => "Medium",
                        71..=100 => "High",
                        _ => "Unknown",
                    };
                    ui.label(format!("{} ({:.0}%)", confidence_text, eval.confidence * 100.0));
                });
                
                // Game phase
                let move_count = game_state.moves.len();
                let phase = match move_count {
                    0..=20 => "Opening",
                    21..=100 => "Middle Game",
                    _ => "Endgame",
                };
                ui.label(format!("Phase: {} (move {})", phase, move_count));
            } else {
                ui.label("Calculating...");
                ui.spinner();
            }
        });
    }
    
    /// Handle keyboard shortcuts
    pub fn handle_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.heat_map.toggle();
        }
    }
    
    /// Update evaluation
    pub fn update_evaluation(&mut self, game_state: &GameState) {
        // This would normally call the neural network
        // For now, simulate with the value network
        let neural_net = self.heat_map.get_neural_net();
        self.last_evaluation = Some(neural_net.evaluate_position(game_state));
    }
}

/// Training progress UI
pub struct TrainingProgressUI {
    pub games_processed: usize,
    pub total_games: usize,
    pub current_accuracy: f32,
    pub is_training: bool,
}

impl TrainingProgressUI {
    pub fn render(&self, ui: &mut egui::Ui) {
        if !self.is_training {
            return;
        }
        
        ui.group(|ui| {
            ui.label(RichText::new("🎓 Training Progress").strong());
            
            // Progress bar
            let progress = self.games_processed as f32 / self.total_games.max(1) as f32;
            ui.add(egui::ProgressBar::new(progress)
                .text(format!("{}/{} games", self.games_processed, self.total_games)));
            
            // Accuracy
            ui.label(format!("Current accuracy: {:.1}%", self.current_accuracy * 100.0));
            
            // Time estimate
            if self.games_processed > 0 {
                let remaining = self.total_games - self.games_processed;
                ui.label(format!("~{} games remaining", remaining));
            }
        });
    }
}