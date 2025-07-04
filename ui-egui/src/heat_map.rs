use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Ui, Vec2};
use p2pgo_core::{Color as GoColor, GameState};
use p2pgo_neural::DualNeuralNet;

/// Heat map visualization for neural network predictions
pub struct HeatMapOverlay {
    /// Neural network for predictions
    neural_net: DualNeuralNet,
    /// Whether to show heat map
    enabled: bool,
    /// Opacity of overlay (0.0 - 1.0)
    opacity: f32,
    /// Color scheme
    color_scheme: ColorScheme,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColorScheme {
    RedGreen,
    BlueRed,
    Viridis,
}

impl HeatMapOverlay {
    pub fn new() -> Self {
        Self {
            neural_net: DualNeuralNet::new(),
            enabled: false,  // Default to OFF
            opacity: 0.5,     // Slightly more transparent
            color_scheme: ColorScheme::RedGreen,
        }
    }
    
    /// Render heat map overlay on board
    pub fn render_overlay(
        &self,
        ui: &mut Ui,
        board_rect: Rect,
        cell_size: f32,
        game_state: &GameState,
    ) {
        if !self.enabled {
            return;
        }
        
        // Get heat map from neural network
        let heat_map = self.neural_net.get_heat_map(game_state);
        let painter = ui.painter();
        
        // Draw heat map
        for y in 0..19 {
            for x in 0..19 {
                let probability = heat_map[y][x];
                
                // Skip very low probability moves
                if probability < 0.01 {
                    continue;
                }
                
                // Calculate position
                let pos = Pos2::new(
                    board_rect.min.x + (x as f32 + 0.5) * cell_size,
                    board_rect.min.y + (y as f32 + 0.5) * cell_size,
                );
                
                // Get color based on probability
                let color = self.probability_to_color(probability);
                let radius = cell_size * 0.4 * probability.sqrt();
                
                // Draw heat indicator
                painter.circle_filled(pos, radius, color);
            }
        }
        
        // Draw top 3 move indicators
        let predictions = self.neural_net.predict_moves(game_state);
        for (i, pred) in predictions.iter().take(3).enumerate() {
            let pos = Pos2::new(
                board_rect.min.x + (pred.coord.x as f32 + 0.5) * cell_size,
                board_rect.min.y + (pred.coord.y as f32 + 0.5) * cell_size,
            );
            
            // Draw ranking number
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                format!("{}", i + 1),
                egui::FontId::proportional(cell_size * 0.5),
                Color32::WHITE,
            );
        }
    }
    
    /// Render control panel
    pub fn render_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.enabled, "Show Heat Map");
            
            if self.enabled {
                ui.separator();
                
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut self.opacity, 0.0..=1.0));
                
                ui.separator();
                
                ui.label("Color scheme:");
                ui.selectable_value(&mut self.color_scheme, ColorScheme::RedGreen, "Red-Green");
                ui.selectable_value(&mut self.color_scheme, ColorScheme::BlueRed, "Blue-Red");
                ui.selectable_value(&mut self.color_scheme, ColorScheme::Viridis, "Viridis");
            }
        });
    }
    
    /// Convert probability to color
    fn probability_to_color(&self, probability: f32) -> Color32 {
        let alpha = (self.opacity * 255.0 * probability) as u8;
        
        match self.color_scheme {
            ColorScheme::RedGreen => {
                // High probability = green, low = red
                let red = ((1.0 - probability) * 255.0) as u8;
                let green = (probability * 255.0) as u8;
                Color32::from_rgba_unmultiplied(red, green, 0, alpha)
            }
            ColorScheme::BlueRed => {
                // High probability = red, low = blue
                let red = (probability * 255.0) as u8;
                let blue = ((1.0 - probability) * 255.0) as u8;
                Color32::from_rgba_unmultiplied(red, 0, blue, alpha)
            }
            ColorScheme::Viridis => {
                // Viridis color map approximation
                let r = (probability * 0.267004 + (1.0 - probability) * 0.282623) * 255.0;
                let g = (probability * 0.004874 + (1.0 - probability) * 0.140926) * 255.0;
                let b = (probability * 0.329415 + (1.0 - probability) * 0.331543) * 255.0;
                Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, alpha)
            }
        }
    }
    
    /// Toggle heat map visibility
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
    
    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Get current opacity
    pub fn get_opacity(&self) -> f32 {
        self.opacity
    }
    
    /// Set opacity
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.1, 1.0);
    }
    
    /// Get reference to neural network
    pub fn get_neural_net(&self) -> &DualNeuralNet {
        &self.neural_net
    }
}

/// Board evaluation display
pub struct EvaluationDisplay {
    /// Show evaluation bar
    show_bar: bool,
    /// Show confidence indicator
    show_confidence: bool,
}

impl EvaluationDisplay {
    pub fn new() -> Self {
        Self {
            show_bar: true,
            show_confidence: true,
        }
    }
    
    pub fn render(
        &self,
        ui: &mut Ui,
        evaluation: p2pgo_neural::BoardEvaluation,
        current_player: GoColor,
    ) {
        if !self.show_bar {
            return;
        }
        
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.heading("Position Evaluation");
                
                // Win probability bar
                let win_prob = if current_player == GoColor::Black {
                    (evaluation.win_probability + 1.0) / 2.0
                } else {
                    (1.0 - evaluation.win_probability) / 2.0
                };
                
                ui.horizontal(|ui| {
                    ui.label("Black");
                    
                    let bar_rect = ui.available_rect_before_wrap();
                    let bar_height = 20.0;
                    let bar_width = 200.0;
                    
                    // Background
                    ui.painter().rect_filled(
                        Rect::from_min_size(bar_rect.min, Vec2::new(bar_width, bar_height)),
                        0.0,
                        Color32::from_gray(50),
                    );
                    
                    // Win probability fill
                    let fill_width = bar_width * win_prob;
                    ui.painter().rect_filled(
                        Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, bar_height)),
                        0.0,
                        if win_prob > 0.5 { Color32::BLACK } else { Color32::WHITE },
                    );
                    
                    // Center line
                    ui.painter().line_segment(
                        [
                            bar_rect.min + Vec2::new(bar_width / 2.0, 0.0),
                            bar_rect.min + Vec2::new(bar_width / 2.0, bar_height),
                        ],
                        Stroke::new(2.0, Color32::GRAY),
                    );
                    
                    ui.add_space(bar_width + 10.0);
                    ui.label("White");
                });
                
                // Win percentage
                ui.label(format!("Win rate: {:.1}%", win_prob * 100.0));
                
                // Confidence indicator
                if self.show_confidence {
                    ui.horizontal(|ui| {
                        ui.label("Confidence:");
                        ui.add(egui::ProgressBar::new(evaluation.confidence)
                            .text(format!("{:.0}%", evaluation.confidence * 100.0)));
                    });
                }
            });
        });
    }
}