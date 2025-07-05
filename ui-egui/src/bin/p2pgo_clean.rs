//! Clean P2P Go UI - Lichess-inspired design

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Stroke, Vec2};
use p2pgo_core::{Color as GoColor, Coord, GameState, Move};
use p2pgo_neural::DualNeuralNet;
use std::collections::VecDeque;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 900.0)),
        min_window_size: Some(egui::vec2(1000.0, 700.0)),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "P2P Go",
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Box::new(P2PGoApp::default())
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Use larger, bolder fonts
    fonts.font_data.insert(
        "bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../assets/Inter-Bold.ttf")),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "bold".to_owned());

    ctx.set_fonts(fonts);

    // Dark theme with our color scheme
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals {
        dark_mode: true,
        override_text_color: Some(Color32::WHITE),
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: Color32::from_gray(25),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                fg_stroke: Stroke::new(1.0, Color32::WHITE),
                rounding: egui::Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: Color32::from_gray(35),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(80)),
                fg_stroke: Stroke::new(1.0, Color32::WHITE),
                rounding: egui::Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: Color32::from_gray(45),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(1.0, Color32::WHITE),
                rounding: egui::Rounding::same(2.0),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: Color32::from_rgb(220, 38, 38), // Red
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(220, 38, 38)),
                fg_stroke: Stroke::new(1.0, Color32::WHITE),
                rounding: egui::Rounding::same(2.0),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: Color32::from_gray(45),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(1.0, Color32::WHITE),
                rounding: egui::Rounding::same(2.0),
                expansion: 0.0,
            },
        },
        ..egui::Visuals::dark()
    };

    ctx.set_style(style);
}

#[derive(PartialEq)]
enum View {
    Game,
    Training,
    Settings,
}

struct P2PGoApp {
    // Game state
    game: GameState,
    move_history: VecDeque<Move>,
    current_view: View,

    // Neural network
    neural_net: DualNeuralNet,
    show_heat_map: bool,

    // UI state
    selected_files: Vec<String>,
    is_training: bool,
    training_progress: f32,

    // Network state
    player_name: String,
    game_code: Option<String>,
}

impl Default for P2PGoApp {
    fn default() -> Self {
        Self {
            game: GameState::new(9),
            move_history: VecDeque::with_capacity(100),
            current_view: View::Game,
            neural_net: DualNeuralNet::new(),
            show_heat_map: false,
            selected_files: Vec::new(),
            is_training: false,
            training_progress: 0.0,
            player_name: "Player".to_string(),
            game_code: None,
        }
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcut for heat map
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_heat_map = !self.show_heat_map;
        }

        // Main layout
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::BLACK))
            .show(ctx, |ui| {
                // Header
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.heading(RichText::new("P2P GO").size(28.0).color(Color32::WHITE));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);

                        // View selector
                        ui.selectable_value(&mut self.current_view, View::Settings, "⚙");
                        ui.selectable_value(&mut self.current_view, View::Training, "🧠");
                        ui.selectable_value(&mut self.current_view, View::Game, "🎮");
                    });
                });

                ui.separator();

                // Main content area
                ui.horizontal(|ui| {
                    ui.add_space(20.0);

                    // Game board column
                    ui.vertical(|ui| {
                        // Game controls at top
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("New Game").size(16.0)).clicked() {
                                self.new_game();
                            }

                            if ui.button(RichText::new("Join Game").size(16.0)).clicked() {
                                // TODO: Show join dialog
                            }

                            ui.add_space(20.0);

                            if self.game_code.is_some() {
                                ui.separator();
                                if ui.button("Pass").clicked() {
                                    self.make_move(Move::Pass(self.game.current_player));
                                }
                                if ui.button("Resign").clicked() {
                                    self.make_move(Move::Resign(self.game.current_player));
                                }
                            }
                        });

                        ui.add_space(10.0);

                        // Board
                        self.render_board(ui);
                    });

                    ui.add_space(40.0);

                    // Side panel
                    ui.vertical(|ui| {
                        ui.set_min_width(300.0);

                        match self.current_view {
                            View::Game => self.render_game_info(ui),
                            View::Training => self.render_training_panel(ui),
                            View::Settings => self.render_settings(ui),
                        }
                    });
                });
            });
    }
}

impl P2PGoApp {
    fn render_board(&mut self, ui: &mut egui::Ui) {
        let board_size = 600.0;
        let (response, painter) =
            ui.allocate_painter(Vec2::new(board_size, board_size), egui::Sense::click());

        let rect = response.rect;

        // Board background (traditional color)
        painter.rect_filled(rect, 0.0, Color32::from_rgb(220, 179, 92));

        let grid_size = 9;
        let margin = 30.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(board_size - 2.0 * margin),
        );
        let cell_size = board_rect.width() / (grid_size - 1) as f32;

        // Grid lines
        for i in 0..grid_size {
            let offset = i as f32 * cell_size;

            // Vertical
            painter.line_segment(
                [
                    Pos2::new(board_rect.left() + offset, board_rect.top()),
                    Pos2::new(board_rect.left() + offset, board_rect.bottom()),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );

            // Horizontal
            painter.line_segment(
                [
                    Pos2::new(board_rect.left(), board_rect.top() + offset),
                    Pos2::new(board_rect.right(), board_rect.top() + offset),
                ],
                Stroke::new(1.0, Color32::BLACK),
            );
        }

        // Star points (for 9x9)
        let stars = [(2, 2), (6, 2), (4, 4), (2, 6), (6, 6)];
        for (x, y) in stars {
            let pos = Pos2::new(
                board_rect.left() + x as f32 * cell_size,
                board_rect.top() + y as f32 * cell_size,
            );
            painter.circle_filled(pos, 3.0, Color32::BLACK);
        }

        // Heat map (if enabled)
        if self.show_heat_map {
            let heat_map = self.neural_net.get_heat_map(&self.game);
            for y in 0..grid_size {
                for x in 0..grid_size {
                    let prob = heat_map[y][x];
                    if prob > 0.01 {
                        let pos = Pos2::new(
                            board_rect.left() + x as f32 * cell_size,
                            board_rect.top() + y as f32 * cell_size,
                        );

                        let alpha = (prob * 200.0).min(200.0) as u8;
                        let color = Color32::from_rgba_unmultiplied(220, 38, 38, alpha);
                        painter.circle_filled(pos, cell_size * 0.3, color);
                    }
                }
            }
        }

        // Stones
        for y in 0..grid_size {
            for x in 0..grid_size {
                if let Some(color) = self.game.board.get(y * grid_size + x) {
                    let pos = Pos2::new(
                        board_rect.left() + x as f32 * cell_size,
                        board_rect.top() + y as f32 * cell_size,
                    );

                    match color {
                        GoColor::Black => {
                            painter.circle_filled(pos, cell_size * 0.45, Color32::BLACK);
                        }
                        GoColor::White => {
                            painter.circle_filled(pos, cell_size * 0.45, Color32::WHITE);
                            painter.circle_stroke(
                                pos,
                                cell_size * 0.45,
                                Stroke::new(1.0, Color32::BLACK),
                            );
                        }
                    }
                }
            }
        }

        // Handle clicks
        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let rel = pointer_pos - board_rect.min;
                let x = (rel.x / cell_size).round() as u8;
                let y = (rel.y / cell_size).round() as u8;

                if x < grid_size as u8 && y < grid_size as u8 {
                    self.try_place_stone(x, y);
                }
            }
        }
    }

    fn render_game_info(&self, ui: &mut egui::Ui) {
        ui.heading("Game Info");
        ui.separator();

        if let Some(code) = &self.game_code {
            ui.label(format!("Game Code: {}", code));
            ui.label(format!("Current Player: {:?}", self.game.current_player));
            ui.separator();
        }

        ui.label("Recent Moves:");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (i, mv) in self.move_history.iter().enumerate().rev().take(10) {
                    ui.label(format!("{}. {:?}", self.move_history.len() - i, mv));
                }
            });

        ui.separator();
        ui.label(format!(
            "Heat Map: {} (press H to toggle)",
            if self.show_heat_map { "ON" } else { "OFF" }
        ));
    }

    fn render_training_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Neural Network Training");
        ui.separator();

        ui.label(RichText::new("Select SGF files (1-10):").size(16.0));

        if ui.button("Choose Files").clicked() {
            // In real app, would open file dialog
            self.selected_files = vec!["game1.sgf".to_string(), "game2.sgf".to_string()];
        }

        if !self.selected_files.is_empty() {
            ui.label(format!("{} files selected", self.selected_files.len()));

            for file in &self.selected_files {
                ui.label(format!("  • {}", file));
            }

            ui.add_space(10.0);

            if !self.is_training {
                if ui
                    .button(
                        RichText::new("Start Training")
                            .size(18.0)
                            .color(Color32::from_rgb(220, 38, 38)),
                    )
                    .clicked()
                {
                    self.is_training = true;
                    self.training_progress = 0.0;
                }
            } else {
                ui.add(egui::ProgressBar::new(self.training_progress));
                ui.label("Training neural network...");

                // Simulate progress
                self.training_progress = (self.training_progress + 0.01).min(1.0);

                if self.training_progress >= 1.0 {
                    self.is_training = false;
                    ui.label("Training complete!");
                }
            }
        }

        ui.separator();
        ui.collapsing("How it works", |ui| {
            ui.label("• SGF files provide general Go knowledge");
            ui.label("• Your games provide personalized learning");
            ui.label("• Neural net has Policy (moves) + Value (evaluation)");
            ui.label("• Heat map shows move probabilities");
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Player Name:");
            ui.text_edit_singleline(&mut self.player_name);
        });

        ui.separator();
        ui.label("Neural Network Config (1-10):");

        // Simplified - in real app would connect to neural config
        let mut aggression = 5;
        ui.add(egui::Slider::new(&mut aggression, 1..=10).text("Aggression"));
    }

    fn new_game(&mut self) {
        self.game = GameState::new(9);
        self.move_history.clear();
        self.game_code = Some(format!("GAME{:04}", rand::random::<u16>() % 10000));
    }

    fn try_place_stone(&mut self, x: u8, y: u8) {
        let coord = Coord::new(x, y);
        if self.game.is_valid_move(coord) {
            let mv = Move::Place {
                x,
                y,
                color: self.game.current_player,
            };
            self.make_move(mv);
        }
    }

    fn make_move(&mut self, mv: Move) {
        if self.game.apply_move(mv.clone()).is_ok() {
            self.move_history.push_back(mv);
            if self.move_history.len() > 100 {
                self.move_history.pop_front();
            }
        }
    }
}

// Font loading fallback
#[cfg(not(feature = "embed-fonts"))]
fn setup_custom_fonts(ctx: &egui::Context) {
    // Use default fonts if Inter not embedded
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            egui::TextStyle::Small,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Body,
            FontId::new(16.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Button,
            FontId::new(16.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (
            egui::TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
    ]
    .iter()
    .cloned()
    .collect();
    ctx.set_style(style);
}
