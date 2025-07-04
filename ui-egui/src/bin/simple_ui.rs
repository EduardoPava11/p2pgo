//! Simplified UI for building DMG

use eframe::egui;
use p2pgo_core::{GameState, Move, Color, Coord};
use p2pgo_neural::{DualNeuralNet, config::NeuralConfig};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 900.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "P2P Go",
        options,
        Box::new(|_cc| Box::new(P2PGoApp::default())),
    )
}

struct P2PGoApp {
    game_state: GameState,
    neural_net: DualNeuralNet,
    show_heat_map: bool,
    neural_config: NeuralConfig,
    config_step: usize,
    config_values: Vec<u8>,
}

impl Default for P2PGoApp {
    fn default() -> Self {
        Self {
            game_state: GameState::new(9),
            neural_net: DualNeuralNet::new(),
            show_heat_map: false,
            neural_config: NeuralConfig::default(),
            config_step: 0,
            config_values: vec![5; 10],
        }
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_heat_map = !self.show_heat_map;
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading("🎮 P2P Go - Neural Network Edition");
                ui.separator();
                if ui.button("🧠 Configure AI").clicked() {
                    self.config_step = 0;
                }
                ui.separator();
                let heat_text = if self.show_heat_map {
                    "🔥 Heat Map ON"
                } else {
                    "❄️ Heat Map OFF"
                };
                if ui.button(heat_text).clicked() {
                    self.show_heat_map = !self.show_heat_map;
                }
                ui.label("(Press H)");
            });
            
            ui.separator();
            
            // Main content
            ui.horizontal(|ui| {
                // Left panel - Board
                ui.group(|ui| {
                    ui.label("Go Board");
                    self.render_board(ui);
                });
                
                // Right panel - Neural config or info
                ui.group(|ui| {
                    if self.config_step < 10 {
                        self.render_config_wizard(ui);
                    } else {
                        self.render_game_info(ui);
                    }
                });
            });
        });
    }
}

impl P2PGoApp {
    fn render_board(&self, ui: &mut egui::Ui) {
        let size = 400.0;
        let (response, painter) = ui.allocate_painter(
            egui::vec2(size, size),
            egui::Sense::click(),
        );
        
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(220, 179, 92));
        
        // Grid
        let grid_size = 9;
        let cell_size = size / grid_size as f32;
        
        for i in 0..grid_size {
            let offset = (i as f32 + 0.5) * cell_size;
            // Vertical lines
            painter.line_segment(
                [
                    egui::pos2(rect.left() + offset, rect.top()),
                    egui::pos2(rect.left() + offset, rect.bottom()),
                ],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            // Horizontal lines
            painter.line_segment(
                [
                    egui::pos2(rect.left(), rect.top() + offset),
                    egui::pos2(rect.right(), rect.top() + offset),
                ],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
        }
        
        // Heat map overlay if enabled
        if self.show_heat_map {
            let heat_map = self.neural_net.get_heat_map(&self.game_state);
            for y in 0..grid_size {
                for x in 0..grid_size {
                    let prob = heat_map[y as usize][x as usize];
                    if prob > 0.01 {
                        let center = egui::pos2(
                            rect.left() + (x as f32 + 0.5) * cell_size,
                            rect.top() + (y as f32 + 0.5) * cell_size,
                        );
                        let alpha = (prob * 200.0) as u8;
                        let color = egui::Color32::from_rgba_unmultiplied(255, 0, 0, alpha);
                        painter.circle_filled(center, cell_size * 0.3, color);
                    }
                }
            }
        }
        
        // Stones
        for y in 0..grid_size {
            for x in 0..grid_size {
                let idx = y * grid_size + x;
                if let Some(color) = self.game_state.board.get(idx) {
                    let center = egui::pos2(
                        rect.left() + (x as f32 + 0.5) * cell_size,
                        rect.top() + (y as f32 + 0.5) * cell_size,
                    );
                    let stone_color = match color {
                        Color::Black => egui::Color32::BLACK,
                        Color::White => egui::Color32::WHITE,
                    };
                    painter.circle_filled(center, cell_size * 0.4, stone_color);
                    if color == Color::White {
                        painter.circle_stroke(center, cell_size * 0.4, 
                            egui::Stroke::new(1.0, egui::Color32::BLACK));
                    }
                }
            }
        }
    }
    
    fn render_config_wizard(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧠 Neural Network Configuration");
        ui.separator();
        
        let questions = [
            ("Aggression", "1=Defensive, 10=Aggressive"),
            ("Territory Focus", "1=Fighting, 10=Territory"),
            ("Fighting Spirit", "1=Peaceful, 10=Warrior"),
            ("Pattern Recognition", "1=Calculate, 10=Patterns"),
            ("Risk Tolerance", "1=Safe, 10=Risky"),
            ("Opening Style", "1=Slow, 10=Fast"),
            ("Middle Game", "1=Weak, 10=Strong"),
            ("Endgame", "1=Rough, 10=Precise"),
            ("Learning Rate", "1=Slow, 10=Fast"),
            ("Creativity", "1=Standard, 10=Creative"),
        ];
        
        if self.config_step < questions.len() {
            let (name, desc) = questions[self.config_step];
            ui.label(format!("Question {} of 10", self.config_step + 1));
            ui.separator();
            
            ui.label(egui::RichText::new(name).size(20.0).strong());
            ui.label(desc);
            ui.add_space(20.0);
            
            ui.add(egui::Slider::new(&mut self.config_values[self.config_step], 1..=10));
            
            ui.add_space(20.0);
            if ui.button("Next →").clicked() {
                self.config_step += 1;
                if self.config_step == 10 {
                    // Apply configuration
                    self.neural_config = NeuralConfig {
                        aggression: self.config_values[0],
                        territory_focus: self.config_values[1],
                        fighting_spirit: self.config_values[2],
                        pattern_recognition: self.config_values[3],
                        risk_tolerance: self.config_values[4],
                        opening_style: self.config_values[5],
                        middle_game_focus: self.config_values[6],
                        endgame_precision: self.config_values[7],
                        learning_rate: self.config_values[8],
                        creativity: self.config_values[9],
                    };
                }
            }
        }
    }
    
    fn render_game_info(&self, ui: &mut egui::Ui) {
        ui.heading("Game Information");
        ui.separator();
        
        ui.label("Neural Network Status:");
        ui.label(format!("✅ Configured (Aggression: {})", self.neural_config.aggression));
        ui.label(format!("Heat Map: {}", if self.show_heat_map { "ON" } else { "OFF" }));
        
        ui.separator();
        ui.label("Instructions:");
        ui.label("• Press H to toggle heat map");
        ui.label("• Red areas show suggested moves");
        ui.label("• Upload SGF files to train");
        ui.label("• Connect with friend via relay");
        
        ui.separator();
        ui.label("Ready for testing!");
    }
}