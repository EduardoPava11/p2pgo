//! Always-visible neural network panel

use egui::{Align2, Color32, FontId, Frame, Pos2, Rect, Response, RichText, Stroke, Ui, Vec2};
use crate::core::{Colors, Spacing, Styles, Typography};
use p2pgo_neural::DualNeuralNet;
use p2pgo_core::Coord;

#[derive(Clone, Copy, PartialEq)]
pub enum PanelPosition {
    Left,
    Right,
    Float,
}

pub struct NeuralPanel {
    position: PanelPosition,
    opacity: f32,
    show_details: bool,
    top_moves: Vec<SuggestedMove>,
    win_probability: f32,
    show_network_viz: bool,
}

#[derive(Clone)]
pub struct SuggestedMove {
    pub coord: Coord,
    pub probability: f32,
    pub explanation: String,
}

impl NeuralPanel {
    pub fn new() -> Self {
        Self {
            position: PanelPosition::Right,
            opacity: 0.9,
            show_details: false,
            top_moves: Vec::new(),
            win_probability: 0.5,
            show_network_viz: false,
        }
    }
    
    pub fn position(mut self, position: PanelPosition) -> Self {
        self.position = position;
        self
    }
    
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.3, 1.0);
        self
    }
    
    pub fn update_suggestions(&mut self, net: &DualNeuralNet, game_state: &p2pgo_core::GameState) {
        // Get move predictions
        let predictions = net.predict_moves(game_state);
        
        // Convert to suggested moves and get top 5
        let mut moves: Vec<_> = predictions.into_iter()
            .filter(|pred| pred.probability > 0.01)
            .map(|pred| {
                SuggestedMove {
                    coord: pred.coord,
                    probability: pred.probability,
                    explanation: self.get_move_explanation(pred.coord.x, pred.coord.y),
                }
            })
            .collect();
        
        moves.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap());
        moves.truncate(5);
        
        self.top_moves = moves;
        
        // Update win probability
        let evaluation = net.evaluate_position(game_state);
        // Convert win probability from [-1, 1] to [0, 1] where 0.5 is even
        self.win_probability = (evaluation.win_probability + 1.0) / 2.0;
    }
    
    fn get_move_explanation(&self, x: u8, y: u8) -> String {
        // Simple explanations based on position
        match (x, y) {
            (4, 4) => "Center control".to_string(),
            (2, 2) | (2, 6) | (6, 2) | (6, 6) => "Corner approach".to_string(),
            (0..=2, 0..=2) | (6..=8, 0..=2) | (0..=2, 6..=8) | (6..=8, 6..=8) => "Corner play".to_string(),
            (3..=5, _) | (_, 3..=5) => "Side development".to_string(),
            _ => "Territory expansion".to_string(),
        }
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        let panel_width = 250.0;
        let available_rect = ui.ctx().screen_rect();
        
        let panel_rect = match self.position {
            PanelPosition::Left => Rect::from_min_size(
                Pos2::new(Spacing::MD, 60.0),
                Vec2::new(panel_width, available_rect.height() - 120.0),
            ),
            PanelPosition::Right => Rect::from_min_size(
                Pos2::new(available_rect.width() - panel_width - Spacing::MD, 60.0),
                Vec2::new(panel_width, available_rect.height() - 120.0),
            ),
            PanelPosition::Float => {
                // Floating panel that can be moved
                let default_pos = Pos2::new(available_rect.width() - panel_width - 50.0, 100.0);
                Rect::from_min_size(default_pos, Vec2::new(panel_width, 400.0))
            }
        };
        
        ui.allocate_ui_at_rect(panel_rect, |ui| {
            Frame::none()
                .fill(Colors::SURFACE.linear_multiply(self.opacity))
                .inner_margin(Spacing::MD)
                .rounding(Styles::rounding())
                .shadow(crate::core::theme::elevation_2())
                .show(ui, |ui| {
                    self.render_content(ui);
                });
        });
    }
    
    fn render_content(&mut self, ui: &mut Ui) {
        // Header with toggle
        ui.horizontal(|ui| {
            ui.heading("🧠 Neural Analysis");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(if self.show_details { "▼" } else { "▶" }).clicked() {
                    self.show_details = !self.show_details;
                }
            });
        });
        
        ui.separator();
        ui.add_space(Spacing::SM);
        
        // Top suggested moves
        ui.label(RichText::new("Suggested Moves").strong());
        ui.add_space(Spacing::XS);
        
        for (i, suggested_move) in self.top_moves.iter().enumerate() {
            ui.horizontal(|ui| {
                // Rank
                ui.label(format!("{}.", i + 1));
                
                // Coordinate
                let letter = if suggested_move.coord.x < 8 { 
                    (b'A' + suggested_move.coord.x) as char 
                } else { 
                    'J' 
                };
                let number = 9 - suggested_move.coord.y;
                let coord_text = format!("{}{}", letter, number);
                ui.label(RichText::new(coord_text).family(egui::FontFamily::Monospace));
                
                // Probability bar
                let bar_width = 60.0;
                let bar_height = 12.0;
                let (_, bar_rect) = ui.allocate_space(Vec2::new(bar_width, bar_height));
                
                ui.painter().rect_filled(
                    bar_rect,
                    2.0,
                    Color32::from_rgb(
                (Colors::SURFACE.r() as f32 * 1.5).min(255.0) as u8,
                (Colors::SURFACE.g() as f32 * 1.5).min(255.0) as u8,
                (Colors::SURFACE.b() as f32 * 1.5).min(255.0) as u8,
            ),
                );
                
                let filled_width = bar_width * suggested_move.probability;
                let filled_rect = Rect::from_min_size(
                    bar_rect.min,
                    Vec2::new(filled_width, bar_height),
                );
                
                ui.painter().rect_filled(
                    filled_rect,
                    2.0,
                    Colors::NEURAL_POLICY.linear_multiply(0.8),
                );
                
                // Percentage
                ui.label(format!("{:.1}%", suggested_move.probability * 100.0));
            });
            
            if self.show_details {
                ui.indent(ui.id().with(i), |ui| {
                    ui.label(
                        RichText::new(&suggested_move.explanation)
                            .small()
                            .color(Colors::TEXT_SECONDARY)
                    );
                });
            }
        }
        
        ui.add_space(Spacing::MD);
        
        // Win probability
        self.render_win_probability(ui);
        
        // Expandable details
        if self.show_details {
            ui.add_space(Spacing::MD);
            ui.separator();
            ui.add_space(Spacing::SM);
            
            self.render_network_details(ui);
        }
        
        // Settings
        ui.add_space(Spacing::MD);
        ui.separator();
        ui.add_space(Spacing::SM);
        
        ui.horizontal(|ui| {
            ui.label("Opacity:");
            if ui.add(egui::Slider::new(&mut self.opacity, 0.3..=1.0)).changed() {
                // Opacity updated
            }
        });
        
        ui.checkbox(&mut self.show_network_viz, "Show Network Visualization");
    }
    
    fn render_win_probability(&self, ui: &mut Ui) {
        ui.label(RichText::new("Win Probability").strong());
        ui.add_space(Spacing::XS);
        
        let bar_height = 30.0;
        let (_, bar_rect) = ui.allocate_space(Vec2::new(ui.available_width(), bar_height));
        
        // Background
        ui.painter().rect_filled(
            bar_rect,
            4.0,
            Color32::from_rgb(
                (Colors::SURFACE.r() as f32 * 1.5).min(255.0) as u8,
                (Colors::SURFACE.g() as f32 * 1.5).min(255.0) as u8,
                (Colors::SURFACE.b() as f32 * 1.5).min(255.0) as u8,
            ),
        );
        
        // Black probability (left side)
        let black_width = bar_rect.width() * (1.0 - self.win_probability);
        let black_rect = Rect::from_min_size(
            bar_rect.min,
            Vec2::new(black_width, bar_height),
        );
        
        ui.painter().rect_filled(
            black_rect,
            4.0,
            Colors::BLACK_STONE.linear_multiply(0.8),
        );
        
        // White probability (right side)
        let white_rect = Rect::from_min_size(
            Pos2::new(bar_rect.min.x + black_width, bar_rect.min.y),
            Vec2::new(bar_rect.width() - black_width, bar_height),
        );
        
        ui.painter().rect_filled(
            white_rect,
            4.0,
            Colors::WHITE_STONE.linear_multiply(0.8),
        );
        
        // Center line
        let center_x = bar_rect.min.x + bar_rect.width() * 0.5;
        ui.painter().line_segment(
            [
                Pos2::new(center_x, bar_rect.min.y),
                Pos2::new(center_x, bar_rect.max.y),
            ],
            Stroke::new(1.0, Colors::TEXT_SECONDARY),
        );
        
        // Labels
        ui.painter().text(
            Pos2::new(bar_rect.min.x + 10.0, bar_rect.center().y),
            Align2::LEFT_CENTER,
            format!("{:.1}%", (1.0 - self.win_probability) * 100.0),
            Typography::small(),
            Colors::TEXT_PRIMARY,
        );
        
        ui.painter().text(
            Pos2::new(bar_rect.max.x - 10.0, bar_rect.center().y),
            Align2::RIGHT_CENTER,
            format!("{:.1}%", self.win_probability * 100.0),
            Typography::small(),
            Colors::BLACK_STONE,
        );
    }
    
    fn render_network_details(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Network Details").strong());
        ui.add_space(Spacing::XS);
        
        ui.label(
            RichText::new("Policy Network: Suggests moves based on pattern recognition")
                .small()
                .color(Colors::TEXT_SECONDARY)
        );
        
        ui.label(
            RichText::new("Value Network: Evaluates position strength")
                .small()
                .color(Colors::TEXT_SECONDARY)
        );
        
        ui.add_space(Spacing::SM);
        
        ui.label(RichText::new("Configuration").strong());
        ui.label(
            RichText::new("Your neural network is configured based on your playing style questionnaire")
                .small()
                .color(Colors::TEXT_SECONDARY)
        );
    }
}

impl Default for NeuralPanel {
    fn default() -> Self {
        Self::new()
    }
}