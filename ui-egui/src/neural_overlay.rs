use eframe::egui;
use p2pgo_core::{GameState, Coord, Color};
use p2pgo_neural::DualNeuralNet;

/// Neural network visualization overlay for gameplay
pub struct NeuralOverlay {
    /// Neural network instance
    neural_net: DualNeuralNet,
    /// Whether overlay is enabled
    pub enabled: bool,
    /// Visualization mode
    mode: VisualizationMode,
    /// Heat map cache
    heat_map_cache: Option<HeatMapData>,
    /// Last game state hash
    last_state_hash: u64,
    /// Transparency level
    transparency: f32,
    /// Show move predictions
    show_predictions: bool,
    /// Show win probability
    show_win_prob: bool,
    /// Animation timer
    animation_timer: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum VisualizationMode {
    HeatMap,        // Show move probability heat map
    TopMoves,       // Show top N move predictions
    Influence,      // Show territory influence
    Combined,       // Show all visualizations
}

#[derive(Debug, Clone)]
struct HeatMapData {
    values: [[f32; 19]; 19],
    max_value: f32,
    top_moves: Vec<(Coord, f32)>,
}

impl NeuralOverlay {
    pub fn new() -> Self {
        Self {
            neural_net: DualNeuralNet::new(),
            enabled: false,
            mode: VisualizationMode::HeatMap,
            heat_map_cache: None,
            last_state_hash: 0,
            transparency: 0.7,
            show_predictions: true,
            show_win_prob: true,
            animation_timer: 0.0,
        }
    }

    /// Toggle overlay on/off
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Update visualization with new game state
    pub fn update(&mut self, game_state: &GameState, dt: f32) {
        self.animation_timer += dt;

        // Calculate hash of current game state
        let state_hash = self.calculate_state_hash(game_state);

        // Only recalculate if state changed
        if state_hash != self.last_state_hash {
            self.last_state_hash = state_hash;
            self.update_heat_map(game_state);
        }
    }

    /// Render overlay on board
    pub fn render_overlay(
        &self,
        ui: &mut egui::Ui,
        board_rect: egui::Rect,
        cell_size: f32,
        game_state: &GameState,
    ) {
        if !self.enabled {
            return;
        }

        let painter = ui.painter();

        match self.mode {
            VisualizationMode::HeatMap => {
                self.render_heat_map(painter, board_rect, cell_size, game_state);
            }
            VisualizationMode::TopMoves => {
                self.render_top_moves(painter, board_rect, cell_size, game_state);
            }
            VisualizationMode::Influence => {
                self.render_influence_map(painter, board_rect, cell_size, game_state);
            }
            VisualizationMode::Combined => {
                self.render_heat_map(painter, board_rect, cell_size, game_state);
                self.render_top_moves(painter, board_rect, cell_size, game_state);
            }
        }
    }

    /// Render control panel
    pub fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("🧠 Neural Network Visualization", |ui| {
            ui.checkbox(&mut self.enabled, "Enable overlay");

            if self.enabled {
                ui.separator();

                // Mode selector
                ui.label("Visualization mode:");
                ui.radio_value(&mut self.mode, VisualizationMode::HeatMap, "Heat Map");
                ui.radio_value(&mut self.mode, VisualizationMode::TopMoves, "Top Moves");
                ui.radio_value(&mut self.mode, VisualizationMode::Influence, "Influence");
                ui.radio_value(&mut self.mode, VisualizationMode::Combined, "Combined");

                ui.separator();

                // Settings
                ui.add(egui::Slider::new(&mut self.transparency, 0.1..=1.0)
                    .text("Transparency"));
                ui.checkbox(&mut self.show_predictions, "Show move predictions");
                ui.checkbox(&mut self.show_win_prob, "Show win probability");

                // Neural net info
                if let Some(heat_map) = &self.heat_map_cache {
                    ui.separator();
                    ui.label("Neural Network Analysis:");

                    // Top moves
                    if !heat_map.top_moves.is_empty() {
                        ui.label("Top predicted moves:");
                        for (i, (coord, prob)) in heat_map.top_moves.iter().take(3).enumerate() {
                            ui.label(format!("  {}. ({}, {}) - {:.1}%",
                                i + 1, coord.x, coord.y, prob * 100.0));
                        }
                    }
                }
            }
        });
    }

    /// Render win probability display
    pub fn render_win_probability(&self, ui: &mut egui::Ui, game_state: &GameState) {
        if !self.enabled || !self.show_win_prob {
            return;
        }

        let evaluation = self.neural_net.evaluate_position(game_state);

        // Convert to percentage for current player
        let win_prob = (evaluation.win_probability + 1.0) / 2.0; // Convert from [-1, 1] to [0, 1]
        let color = match game_state.current_player {
            Color::Black => egui::Color32::from_gray(20),
            Color::White => egui::Color32::from_gray(235),
        };

        ui.horizontal(|ui| {
            ui.label("Win probability:");

            // Progress bar visualization
            let bar_rect = ui.available_rect_before_wrap();
            let bar_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(200.0, 20.0));

            ui.painter().rect_filled(
                bar_rect,
                egui::Rounding::same(4.0),
                egui::Color32::from_gray(50),
            );

            let fill_rect = egui::Rect::from_min_size(
                bar_rect.min,
                egui::vec2(bar_rect.width() * win_prob, bar_rect.height()),
            );

            ui.painter().rect_filled(
                fill_rect,
                egui::Rounding::same(4.0),
                color,
            );

            // Text overlay
            ui.painter().text(
                bar_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:.1}%", win_prob * 100.0),
                egui::FontId::default(),
                egui::Color32::WHITE,
            );

            ui.allocate_rect(bar_rect, egui::Sense::hover());
        });

        // Confidence indicator
        ui.label(format!("Confidence: {:.0}%", evaluation.confidence * 100.0));
    }

    fn update_heat_map(&mut self, game_state: &GameState) {
        let predictions = self.neural_net.predict_moves(game_state);
        let heat_map = self.neural_net.get_heat_map(game_state);

        // Find max value for normalization
        let max_value = heat_map.iter()
            .flat_map(|row| row.iter())
            .fold(0.0f32, |max, &val| max.max(val));

        // Get top moves
        let mut top_moves: Vec<(Coord, f32)> = predictions.into_iter()
            .map(|pred| (pred.coord, pred.probability))
            .collect();
        top_moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        self.heat_map_cache = Some(HeatMapData {
            values: heat_map,
            max_value,
            top_moves,
        });
    }

    fn render_heat_map(
        &self,
        painter: &egui::Painter,
        board_rect: egui::Rect,
        cell_size: f32,
        game_state: &GameState,
    ) {
        if let Some(heat_map) = &self.heat_map_cache {
            for y in 0..game_state.board_size {
                for x in 0..game_state.board_size {
                    let value = heat_map.values[y as usize][x as usize];
                    if value > 0.01 {
                        let normalized = value / heat_map.max_value;
                        let opacity = (normalized * self.transparency * 255.0) as u8;

                        // Use color gradient from blue (low) to red (high)
                        let color = if normalized < 0.5 {
                            let t = normalized * 2.0;
                            egui::Color32::from_rgba_unmultiplied(
                                (t * 255.0) as u8,
                                0,
                                ((1.0 - t) * 255.0) as u8,
                                opacity,
                            )
                        } else {
                            let t = (normalized - 0.5) * 2.0;
                            egui::Color32::from_rgba_unmultiplied(
                                255,
                                ((1.0 - t) * 255.0) as u8,
                                0,
                                opacity,
                            )
                        };

                        let pos = egui::pos2(
                            board_rect.min.x + x as f32 * cell_size + cell_size / 2.0,
                            board_rect.min.y + y as f32 * cell_size + cell_size / 2.0,
                        );

                        // Animated pulse effect
                        let pulse = (self.animation_timer * 2.0).sin() * 0.1 + 0.9;
                        let radius = cell_size * 0.4 * pulse;

                        painter.circle_filled(pos, radius, color);
                    }
                }
            }
        }
    }

    fn render_top_moves(
        &self,
        painter: &egui::Painter,
        board_rect: egui::Rect,
        cell_size: f32,
        _game_state: &GameState,
    ) {
        if let Some(heat_map) = &self.heat_map_cache {
            for (i, (coord, prob)) in heat_map.top_moves.iter().take(5).enumerate() {
                let pos = egui::pos2(
                    board_rect.min.x + coord.x as f32 * cell_size + cell_size / 2.0,
                    board_rect.min.y + coord.y as f32 * cell_size + cell_size / 2.0,
                );

                // Draw ranking number
                let color = match i {
                    0 => egui::Color32::from_rgb(255, 215, 0), // Gold
                    1 => egui::Color32::from_rgb(192, 192, 192), // Silver
                    2 => egui::Color32::from_rgb(205, 127, 50), // Bronze
                    _ => egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                };

                // Background circle
                let bg_alpha = (self.transparency * 180.0) as u8;
                painter.circle_filled(
                    pos,
                    cell_size * 0.35,
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, bg_alpha),
                );

                // Rank text
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    egui::FontId::proportional(cell_size * 0.5),
                    color,
                );

                // Probability text below
                if self.show_predictions {
                    painter.text(
                        egui::pos2(pos.x, pos.y + cell_size * 0.4),
                        egui::Align2::CENTER_TOP,
                        format!("{:.0}%", prob * 100.0),
                        egui::FontId::proportional(cell_size * 0.25),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                    );
                }
            }
        }
    }

    fn render_influence_map(
        &self,
        painter: &egui::Painter,
        board_rect: egui::Rect,
        cell_size: f32,
        game_state: &GameState,
    ) {
        // Simple influence visualization based on board evaluation
        let evaluation = self.neural_net.evaluate_position(game_state);

        // Create gradient overlay based on win probability
        let influence_color = if evaluation.win_probability > 0.0 {
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, (self.transparency * 50.0) as u8)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, (self.transparency * 50.0) as u8)
        };

        // Draw influence regions (simplified - would be more complex in reality)
        for y in 0..game_state.board_size {
            for x in 0..game_state.board_size {
                let idx = (y as usize) * (game_state.board_size as usize) + (x as usize);
                if game_state.board[idx].is_none() {
                    let pos = egui::pos2(
                        board_rect.min.x + x as f32 * cell_size,
                        board_rect.min.y + y as f32 * cell_size,
                    );

                    let rect = egui::Rect::from_min_size(
                        pos,
                        egui::vec2(cell_size, cell_size),
                    );

                    painter.rect_filled(rect, egui::Rounding::ZERO, influence_color);
                }
            }
        }
    }

    fn calculate_state_hash(&self, game_state: &GameState) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        game_state.board.hash(&mut hasher);
        game_state.current_player.hash(&mut hasher);
        hasher.finish()
    }
}