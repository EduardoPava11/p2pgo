//! Lichess-inspired P2P Go UI

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use p2pgo_core::{Color, Coord, GameState, Move};
use p2pgo_neural::{config::NeuralConfig, DualNeuralNet};
use rand::Rng;
use std::collections::VecDeque;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1400.0, 900.0)),
        min_window_size: Some(egui::vec2(1200.0, 800.0)),
        ..Default::default()
    };

    eframe::run_native(
        "P2P Go - Lichess Edition",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Box::new(P2PGoApp::default())
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Small,
            FontId::new(12.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            FontId::new(14.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            FontId::new(14.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Heading,
            FontId::new(18.0, egui::FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            FontId::new(14.0, egui::FontFamily::Monospace),
        ),
    ]
    .iter()
    .cloned()
    .collect();

    // Dark theme
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(28, 28, 32);
    visuals.window_fill = Color32::from_rgb(35, 35, 40);
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 24);

    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

#[derive(PartialEq)]
enum View {
    Game,
    Training,
    Config,
}

struct P2PGoApp {
    game_state: GameState,
    move_history: VecDeque<Move>,
    neural_net: DualNeuralNet,
    neural_config: NeuralConfig,
    show_heat_map: bool,
    selected_sgf_files: Vec<PathBuf>,
    training_progress: f32,
    is_training: bool,
    training_log: VecDeque<String>,
    current_view: View,
    game_id: Option<String>,
    player_name: String,
    opponent_name: Option<String>,
}

impl Default for P2PGoApp {
    fn default() -> Self {
        Self {
            game_state: GameState::new(9),
            move_history: VecDeque::new(),
            neural_net: DualNeuralNet::new(),
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

        // Main UI
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(28, 28, 32)))
            .show(ctx, |ui| {
                self.render_header(ui);

                ui.horizontal(|ui| {
                    ui.add_space(20.0);

                    // Left: Board area
                    ui.vertical(|ui| {
                        self.render_game_controls(ui);
                        ui.add_space(10.0);
                        self.render_board(ui);
                    });

                    ui.add_space(20.0);

                    // Right: Context panel
                    ui.vertical(|ui| match self.current_view {
                        View::Game => self.render_game_info(ui),
                        View::Training => self.render_training_panel(ui),
                        View::Config => self.render_config_panel(ui),
                    });
                });
            });
    }
}

impl P2PGoApp {
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.label(
                RichText::new("P2P Go")
                    .size(24.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.separator();

            // Navigation
            if ui
                .selectable_label(self.current_view == View::Game, "Play")
                .clicked()
            {
                self.current_view = View::Game;
            }
            if ui
                .selectable_label(self.current_view == View::Training, "Training")
                .clicked()
            {
                self.current_view = View::Training;
            }
            if ui
                .selectable_label(self.current_view == View::Config, "Neural Config")
                .clicked()
            {
                self.current_view = View::Config;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                if ui
                    .button(format!(
                        "{} Heat Map (H)",
                        if self.show_heat_map { "🔥" } else { "❄️" }
                    ))
                    .clicked()
                {
                    self.show_heat_map = !self.show_heat_map;
                }
                ui.separator();
                ui.label(format!("👤 {}", self.player_name));
            });
        });
        ui.separator();
    }

    fn render_game_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(RichText::new("🆕 New Game").size(16.0)).clicked() {
                self.start_new_game();
            }

            if ui
                .button(RichText::new("🔗 Join Game").size(16.0))
                .clicked()
            {
                // Would show join dialog
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
        let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::click());
        let rect = response.rect;

        // Board background
        painter.rect_filled(rect, 4.0, Color32::from_rgb(220, 179, 92));
        painter.rect_stroke(rect, 4.0, Stroke::new(2.0, Color32::BLACK));

        let board_size = 9;
        let margin = 30.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(size - 2.0 * margin),
        );
        let cell_size = board_rect.width() / (board_size - 1) as f32;

        // Grid
        for i in 0..board_size {
            let offset = i as f32 * cell_size;

            painter.line_segment(
                [
                    Pos2::new(board_rect.left() + offset, board_rect.top()),
                    Pos2::new(board_rect.left() + offset, board_rect.bottom()),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );

            painter.line_segment(
                [
                    Pos2::new(board_rect.left(), board_rect.top() + offset),
                    Pos2::new(board_rect.right(), board_rect.top() + offset),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
        }

        // Star points
        let star_points = [(2, 2), (6, 2), (4, 4), (2, 6), (6, 6)];
        for (x, y) in star_points {
            let pos = Pos2::new(
                board_rect.left() + x as f32 * cell_size,
                board_rect.top() + y as f32 * cell_size,
            );
            painter.circle_filled(pos, 3.0, Color32::BLACK);
        }

        // Heat map
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

    fn render_heat_map(
        &self,
        painter: &egui::Painter,
        board_rect: &Rect,
        cell_size: f32,
        board_size: usize,
    ) {
        let heat_map = self.neural_net.get_heat_map(&self.game_state);

        for y in 0..board_size {
            for x in 0..board_size {
                let prob = heat_map[y][x];
                if prob > 0.01 {
                    let pos = Pos2::new(
                        board_rect.left() + x as f32 * cell_size,
                        board_rect.top() + y as f32 * cell_size,
                    );

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

    fn render_game_info(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game Info");
        ui.separator();

        if let Some(game_id) = &self.game_id {
            ui.label(format!("Game ID: {}", game_id));

            ui.separator();
            ui.label("Recent Moves:");

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, mv) in self.move_history.iter().enumerate().rev().take(10) {
                        ui.label(format!("{}. {:?}", self.move_history.len() - i, mv));
                    }
                });

            ui.separator();
            let eval = self.neural_net.evaluate_position(&self.game_state);
            ui.label(format!(
                "Win %: {:.1}%",
                (eval.win_probability + 1.0) / 2.0 * 100.0
            ));
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

        ui.label(RichText::new("Select SGF Files (1-10):").strong());
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("📁 Choose SGF Files").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .set_title("Select 1-10 SGF files")
                    .pick_files()
                {
                    self.selected_sgf_files = paths.into_iter().take(10).collect();
                }
            }

            if !self.selected_sgf_files.is_empty() {
                ui.label(format!("{} files selected", self.selected_sgf_files.len()));
            }
        });

        if !self.selected_sgf_files.is_empty() {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Selected files:");
                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for (i, path) in self.selected_sgf_files.iter().enumerate() {
                            ui.label(format!(
                                "{}. {}",
                                i + 1,
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ));
                        }
                    });
            });
        }

        ui.add_space(20.0);

        if ui
            .button(RichText::new("🚀 Start Training").size(18.0))
            .clicked()
            && !self.selected_sgf_files.is_empty()
            && !self.is_training
        {
            self.start_training();
        }

        if self.is_training {
            ui.separator();
            ui.label(RichText::new("Training Progress").strong());
            ui.add(
                egui::ProgressBar::new(self.training_progress)
                    .text(format!("{:.0}%", self.training_progress * 100.0)),
            );

            // Visual representation
            self.render_neural_visualization(ui);

            ui.label("Training Log:");
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for log in self.training_log.iter().rev() {
                        ui.label(log);
                    }
                });
        }
    }

    fn render_neural_visualization(&self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(Vec2::new(400.0, 200.0), egui::Sense::hover());
        let rect = response.rect;

        painter.rect_filled(rect, 4.0, Color32::from_gray(20));

        // Animated neural network
        let layers = [81, 64, 32, 361];
        let layer_width = rect.width() / (layers.len() as f32 + 1.0);

        for (i, &layer_size) in layers.iter().enumerate() {
            let x = rect.left() + (i as f32 + 1.0) * layer_width;
            let neurons = layer_size.min(8);
            let spacing = rect.height() / (neurons as f32 + 1.0);

            for j in 0..neurons {
                let y = rect.top() + (j as f32 + 1.0) * spacing;
                let activation = (self.training_progress * 10.0 + i as f32 + j as f32)
                    .sin()
                    .abs();
                let color =
                    Color32::from_rgb((255.0 * activation) as u8, (100.0 * activation) as u8, 50);
                painter.circle_filled(Pos2::new(x, y), 5.0, color);
            }
        }
    }

    fn render_config_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧠 Neural Network Configuration");
        ui.separator();

        ui.label("Configure your AI's personality (1-10):");
        ui.add_space(10.0);

        let configs = [
            ("Aggression", &mut self.neural_config.aggression),
            ("Territory Focus", &mut self.neural_config.territory_focus),
            ("Fighting Spirit", &mut self.neural_config.fighting_spirit),
            (
                "Pattern Recognition",
                &mut self.neural_config.pattern_recognition,
            ),
            ("Risk Tolerance", &mut self.neural_config.risk_tolerance),
        ];

        for (label, value) in configs {
            ui.horizontal(|ui| {
                ui.label(format!("{:20}", label));
                ui.add(egui::Slider::new(value, 1..=10).show_value(true));
            });
        }

        ui.add_space(20.0);

        if ui.button("Apply Configuration").clicked() {
            // Apply config
        }

        ui.separator();
        ui.collapsing("How it works", |ui| {
            ui.label("Each parameter influences your AI's playing style:");
            ui.label("• Aggression: How often it attacks vs defends");
            ui.label("• Territory Focus: Preference for territory vs influence");
            ui.label("• Fighting Spirit: Willingness to engage in complex battles");
            ui.label("• Pattern Recognition: Use of known joseki and patterns");
            ui.label("• Risk Tolerance: Conservative vs experimental moves");
        });
    }

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
        }
    }

    fn start_training(&mut self) {
        self.is_training = true;
        self.training_progress = 0.0;
        self.training_log.clear();

        self.training_log
            .push_back("🚀 Training started...".to_string());
        self.training_log.push_back(format!(
            "📁 Processing {} SGF files",
            self.selected_sgf_files.len()
        ));

        // Simulate progress
        self.training_progress = 0.25;
        self.training_log
            .push_back("🧠 Neural network initialized".to_string());
        self.training_log
            .push_back("📊 Loading game positions...".to_string());
    }
}
