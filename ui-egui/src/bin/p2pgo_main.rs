//! Main P2P Go UI - Lichess-inspired design

use eframe::egui::{self, Color32, RichText, Stroke, Vec2, Pos2, Rect, FontId, Align2};
use p2pgo_core::{GameState, Move, Color, Coord};
use p2pgo_neural::{DualNeuralNet, config::NeuralConfig, training::NeuralTrainer};
use std::path::PathBuf;
use std::collections::VecDeque;
use rand::Rng;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1400.0, 900.0)),
        min_window_size: Some(egui::vec2(1200.0, 800.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "P2P Go",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Box::new(P2PGoApp::default())
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    // Use system fonts with larger default size
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Small, FontId::new(12.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, FontId::new(14.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Button, FontId::new(14.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Heading, FontId::new(18.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(14.0, egui::FontFamily::Monospace)),
    ].iter().cloned().collect();
    ctx.set_style(style);
}

#[derive(PartialEq)]
enum View {
    Game,
    Training,
    Config,
}

struct P2PGoApp {
    // Game state
    game_state: GameState,
    move_history: VecDeque<Move>,
    
    // Neural network
    neural_net: DualNeuralNet,
    neural_trainer: Option<NeuralTrainer>,
    neural_config: NeuralConfig,
    show_heat_map: bool,
    
    // Training
    selected_sgf_files: Vec<PathBuf>,
    training_progress: f32,
    is_training: bool,
    training_log: VecDeque<String>,
    
    // UI state
    current_view: View,
    game_id: Option<String>,
    
    // Network
    player_name: String,
    opponent_name: Option<String>,
}

impl Default for P2PGoApp {
    fn default() -> Self {
        Self {
            game_state: GameState::new(9),
            move_history: VecDeque::new(),
            neural_net: DualNeuralNet::new(),
            neural_trainer: None,
            neural_config: NeuralConfig::default(),
            show_heat_map: false,
            selected_sgf_files: Vec::new(),
            training_progress: 0.0,
            is_training: false,
            training_log: VecDeque::with_capacity(100),
            current_view: View::Game,
            game_id: None,
            player_name: "Player".to_string(),
            opponent_name: None,
        }
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_heat_map = !self.show_heat_map;
        }
        
        // Main UI with Lichess-style layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(35, 35, 35)))
            .show(ctx, |ui| {
                // Header bar
                self.render_header(ui);
                
                // Main content area
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    
                    // Left panel - Board and game controls
                    ui.vertical(|ui| {
                        self.render_game_controls(ui);
                        ui.add_space(10.0);
                        self.render_board(ui);
                    });
                    
                    ui.add_space(20.0);
                    
                    // Right panel - Context-sensitive
                    ui.vertical(|ui| {
                        match self.current_view {
                            View::Game => self.render_game_panel(ui),
                            View::Training => self.render_training_panel(ui),
                            View::Config => self.render_config_panel(ui),
                        }
                    });
                });
            });
    }
}

impl P2PGoApp {
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            // Logo
            ui.label(RichText::new("P2P Go").size(24.0).strong().color(Color32::WHITE));
            
            ui.separator();
            
            // Navigation tabs
            if ui.selectable_label(self.current_view == View::Game, "Play").clicked() {
                self.current_view = View::Game;
            }
            
            if ui.selectable_label(self.current_view == View::Training, "Training").clicked() {
                self.current_view = View::Training;
            }
            
            if ui.selectable_label(self.current_view == View::Config, "Neural Config").clicked() {
                self.current_view = View::Config;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                
                // Heat map toggle
                let heat_icon = if self.show_heat_map { "🔥" } else { "❄️" };
                if ui.button(format!("{} Heat Map (H)", heat_icon)).clicked() {
                    self.show_heat_map = !self.show_heat_map;
                }
                
                ui.separator();
                
                // Player info
                ui.label(format!("👤 {}", self.player_name));
            });
        });
        
        ui.separator();
    }
    
    fn render_game_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.visuals_mut().override_text_color = Some(Color32::WHITE);
            
            // Primary game actions
            if ui.button(RichText::new("🆕 New Game").size(16.0)).clicked() {
                self.start_new_game();
            }
            
            if ui.button(RichText::new("🔗 Join Game").size(16.0)).clicked() {
                // Show join dialog
            }
            
            ui.separator();
            
            if self.game_id.is_some() {
                if ui.button("Pass").clicked() {
                    // Handle pass
                }
                
                if ui.button("Resign").clicked() {
                    // Handle resign
                }
            }
        });
    }
    
    fn render_board(&mut self, ui: &mut egui::Ui) {
        let size = 600.0;
        let (response, painter) = ui.allocate_painter(
            Vec2::new(size, size),
            egui::Sense::click(),
        );
        
        let rect = response.rect;
        
        // Board background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(220, 179, 92));
        
        // Board border
        painter.rect_stroke(rect, 4.0, Stroke::new(2.0, Color32::BLACK));
        
        let board_size = 9;
        let margin = 30.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(size - 2.0 * margin),
        );
        let cell_size = board_rect.width() / (board_size - 1) as f32;
        
        // Grid lines
        for i in 0..board_size {
            let offset = i as f32 * cell_size;
            
            // Vertical lines
            painter.line_segment(
                [
                    Pos2::new(board_rect.left() + offset, board_rect.top()),
                    Pos2::new(board_rect.left() + offset, board_rect.bottom()),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
            
            // Horizontal lines
            painter.line_segment(
                [
                    Pos2::new(board_rect.left(), board_rect.top() + offset),
                    Pos2::new(board_rect.right(), board_rect.top() + offset),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
            
            // Coordinates
            let coord_color = Color32::from_gray(100);
            let coord_font = FontId::proportional(12.0);
            
            // Letters (A-J, skipping I)
            let letter = if i < 8 { (b'A' + i as u8) as char } else { 'J' };
            painter.text(
                Pos2::new(board_rect.left() + offset, board_rect.bottom() + 15.0),
                Align2::CENTER_CENTER,
                letter,
                coord_font.clone(),
                coord_color,
            );
            
            // Numbers
            painter.text(
                Pos2::new(board_rect.left() - 15.0, board_rect.bottom() - offset),
                Align2::CENTER_CENTER,
                (i + 1).to_string(),
                coord_font,
                coord_color,
            );
        }
        
        // Star points for 9x9
        let star_points = [(2, 2), (6, 2), (4, 4), (2, 6), (6, 6)];
        for (x, y) in star_points {
            let pos = Pos2::new(
                board_rect.left() + x as f32 * cell_size,
                board_rect.top() + y as f32 * cell_size,
            );
            painter.circle_filled(pos, 3.0, Color32::BLACK);
        }
        
        // Heat map overlay
        if self.show_heat_map {
            self.render_heat_map(&painter, &board_rect, cell_size, board_size);
        }
        
        // Stones
        for y in 0..board_size {
            for x in 0..board_size {
                let idx = y * board_size + x;
                if let Some(color) = self.game_state.board.get(idx) {
                    let pos = Pos2::new(
                        board_rect.left() + x as f32 * cell_size,
                        board_rect.top() + y as f32 * cell_size,
                    );
                    
                    let stone_color = match color {
                        Color::Black => Color32::BLACK,
                        Color::White => Color32::WHITE,
                    };
                    
                    painter.circle_filled(pos, cell_size * 0.45, stone_color);
                    
                    if color == Color::White {
                        painter.circle_stroke(
                            pos,
                            cell_size * 0.45,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                    }
                }
            }
        }
        
        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let relative = pos - board_rect.min;
                let x = (relative.x / cell_size).round() as u8;
                let y = (relative.y / cell_size).round() as u8;
                
                if x < board_size as u8 && y < board_size as u8 {
                    self.try_make_move(x, y);
                }
            }
        }
    }
    
    fn render_heat_map(&self, painter: &egui::Painter, board_rect: &Rect, cell_size: f32, board_size: usize) {
        let heat_map = self.neural_net.get_heat_map(&self.game_state);
        
        for y in 0..board_size {
            for x in 0..board_size {
                let prob = heat_map[y][x];
                if prob > 0.01 {
                    let pos = Pos2::new(
                        board_rect.left() + x as f32 * cell_size,
                        board_rect.top() + y as f32 * cell_size,
                    );
                    
                    // Color based on probability
                    let alpha = (prob * 150.0).min(150.0) as u8;
                    let color = if prob > 0.5 {
                        Color32::from_rgba_unmultiplied(255, 0, 0, alpha)
                    } else if prob > 0.2 {
                        Color32::from_rgba_unmultiplied(255, 255, 0, alpha)
                    } else {
                        Color32::from_rgba_unmultiplied(0, 100, 255, alpha)
                    };
                    
                    painter.circle_filled(pos, cell_size * 0.25, color);
                }
            }
        }
    }
    
    fn render_game_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game Info");
        ui.separator();
        
        if let Some(game_id) = &self.game_id {
            ui.label(format!("Game ID: {}", game_id));
            
            if let Some(opponent) = &self.opponent_name {
                ui.label(format!("Opponent: {}", opponent));
            }
            
            ui.separator();
            
            // Move history
            ui.label("Recent Moves:");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, mv) in self.move_history.iter().enumerate().rev().take(10) {
                        ui.label(format!("{}. {:?}", self.move_history.len() - i, mv));
                    }
                });
                
            ui.separator();
            
            // Position evaluation
            let eval = self.neural_net.evaluate_position(&self.game_state);
            ui.label(format!("Win %: {:.1}%", (eval.win_probability + 1.0) / 2.0 * 100.0));
            ui.label(format!("Confidence: {:.0}%", eval.confidence * 100.0));
        } else {
            ui.label("No active game");
            ui.add_space(20.0);
            ui.label("Start a new game or join an existing one!");
        }
    }
    
    fn render_training_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🎓 Neural Network Training");
        ui.separator();
        
        // SGF file selection
        ui.label(RichText::new("Select SGF Files for Training:").strong());
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            if ui.button("📁 Choose SGF Files").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .set_title("Select 1-10 SGF files for training")
                    .pick_files()
                {
                    self.selected_sgf_files = paths;
                }
            }
            
            if !self.selected_sgf_files.is_empty() {
                ui.label(format!("{} files selected", self.selected_sgf_files.len()));
            }
        });
        
        // Show selected files
        if !self.selected_sgf_files.is_empty() {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Selected files:");
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (i, path) in self.selected_sgf_files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}.", i + 1));
                                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                                
                                if ui.small_button("❌").clicked() {
                                    // Remove file from selection
                                }
                            });
                        }
                    });
            });
        }
        
        ui.add_space(20.0);
        
        // Training controls
        ui.horizontal(|ui| {
            if ui.button(RichText::new("🚀 Start Training").size(18.0))
                .clicked() && !self.selected_sgf_files.is_empty() && !self.is_training
            {
                self.start_training();
            }
            
            if self.is_training {
                if ui.button("⏹ Stop").clicked() {
                    self.is_training = false;
                }
            }
        });
        
        // Training visualization
        if self.is_training || self.training_progress > 0.0 {
            ui.add_space(20.0);
            ui.separator();
            ui.label(RichText::new("Training Progress").strong());
            
            // Progress bar
            ui.add(egui::ProgressBar::new(self.training_progress)
                .text(format!("{:.0}%", self.training_progress * 100.0)));
            
            // Visual neural network animation
            self.render_neural_visualization(ui);
            
            // Training log
            ui.add_space(10.0);
            ui.label("Training Log:");
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for log in self.training_log.iter().rev() {
                        ui.label(log);
                    }
                });
        }
        
        ui.separator();
        ui.add_space(10.0);
        
        // Deep training explanation
        ui.collapsing("Deep Training (Backpropagation)", |ui| {
            ui.label("After each game you play:");
            ui.label("• The network learns from YOUR moves");
            ui.label("• Uses backpropagation to adjust weights");
            ui.label("• Becomes more like your playing style");
            ui.label("• SGF training = general knowledge");
            ui.label("• Your games = personalized learning");
        });
    }
    
    fn render_neural_visualization(&self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(400.0, 200.0),
            egui::Sense::hover(),
        );
        
        let rect = response.rect;
        painter.rect_filled(rect, 4.0, Color32::from_gray(20));
        
        // Draw neural network layers
        let layers = [9*9, 128, 64, 32, 361]; // Input -> Hidden -> Output
        let layer_width = rect.width() / (layers.len() as f32 + 1.0);
        
        for (i, &layer_size) in layers.iter().enumerate() {
            let x = rect.left() + (i as f32 + 1.0) * layer_width;
            let neurons_to_show = layer_size.min(10);
            let spacing = rect.height() / (neurons_to_show as f32 + 1.0);
            
            for j in 0..neurons_to_show {
                let y = rect.top() + (j as f32 + 1.0) * spacing;
                
                // Neuron activation based on training progress
                let activation = (self.training_progress * 10.0 + i as f32 + j as f32).sin().abs();
                let color = Color32::from_rgb(
                    (255.0 * activation) as u8,
                    (100.0 * activation) as u8,
                    50,
                );
                
                painter.circle_filled(Pos2::new(x, y), 5.0, color);
                
                // Draw connections to next layer
                if i < layers.len() - 1 {
                    let next_x = rect.left() + (i as f32 + 2.0) * layer_width;
                    let next_neurons = layers[i + 1].min(10);
                    let next_spacing = rect.height() / (next_neurons as f32 + 1.0);
                    
                    for k in 0..next_neurons {
                        let next_y = rect.top() + (k as f32 + 1.0) * next_spacing;
                        let alpha = (activation * 50.0) as u8;
                        painter.line_segment(
                            [Pos2::new(x, y), Pos2::new(next_x, next_y)],
                            Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
                        );
                    }
                }
            }
            
            // Layer labels
            let label = match i {
                0 => "Input",
                l if l == layers.len() - 1 => "Output",
                _ => "Hidden",
            };
            painter.text(
                Pos2::new(x, rect.bottom() - 10.0),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(12.0),
                Color32::GRAY,
            );
        }
    }
    
    fn render_config_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧠 Neural Network Configuration");
        ui.separator();
        
        ui.label("Configure your AI's personality:");
        ui.add_space(10.0);
        
        egui::Grid::new("config_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                self.render_config_slider(ui, "Aggression", &mut self.neural_config.aggression);
                self.render_config_slider(ui, "Territory Focus", &mut self.neural_config.territory_focus);
                self.render_config_slider(ui, "Fighting Spirit", &mut self.neural_config.fighting_spirit);
                self.render_config_slider(ui, "Pattern Recognition", &mut self.neural_config.pattern_recognition);
                self.render_config_slider(ui, "Risk Tolerance", &mut self.neural_config.risk_tolerance);
            });
            
        ui.add_space(20.0);
        
        if ui.button("Apply Configuration").clicked() {
            self.apply_neural_config();
        }
    }
    
    fn render_config_slider(&mut self, ui: &mut egui::Ui, label: &str, value: &mut u8) {
        ui.label(label);
        ui.add(egui::Slider::new(value, 1..=10).show_value(true));
        ui.end_row();
    }
    
    // Game logic
    fn start_new_game(&mut self) {
        self.game_state = GameState::new(9);
        self.move_history.clear();
        self.game_id = Some(format!("game_{}", rand::random::<u32>()));
    }
    
    fn try_make_move(&mut self, x: u8, y: u8) {
        let mv = Move::Place {
            x,
            y,
            color: self.game_state.current_player,
        };
        
        if self.game_state.is_valid_move(Coord::new(x, y)) {
            let _ = self.game_state.apply_move(mv.clone());
            self.move_history.push_back(mv);
            
            // Trigger deep learning from this move
            if let Some(trainer) = &mut self.neural_trainer {
                // This would trigger backpropagation
            }
        }
    }
    
    fn start_training(&mut self) {
        self.is_training = true;
        self.training_progress = 0.0;
        self.training_log.clear();
        
        if self.neural_trainer.is_none() {
            self.neural_trainer = Some(NeuralTrainer::new(self.neural_config.clone()));
        }
        
        self.training_log.push_back("🚀 Training started...".to_string());
        self.training_log.push_back(format!("📁 Processing {} SGF files", self.selected_sgf_files.len()));
        
        // Simulate training progress
        // In real implementation, this would be async
        self.training_progress = 0.1;
        self.training_log.push_back("🧠 Initializing neural network...".to_string());
        self.training_log.push_back("📊 Loading game positions...".to_string());
        self.training_log.push_back("🔄 Starting backpropagation...".to_string());
    }
    
    fn apply_neural_config(&mut self) {
        self.neural_trainer = Some(NeuralTrainer::new(self.neural_config.clone()));
        // Update neural network with new config
    }
}