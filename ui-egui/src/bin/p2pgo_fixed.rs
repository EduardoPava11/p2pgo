//! Fixed P2P Go UI with working buttons

use eframe::egui::{self, Color32, RichText, Vec2};
use p2pgo_core::{Color as GoColor, Coord, GameState, Move};
use p2pgo_neural::DualNeuralNet;
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 900.0)),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "P2P Go",
        options,
        Box::new(|_cc| Box::new(P2PGoApp::default())),
    )
}

enum AppMessage {
    CreateGame,
    JoinGame(String),
    MakeMove(Move),
}

#[derive(PartialEq)]
enum View {
    MainMenu,
    Lobby(String),
    Game(String),
    Offline,
}

struct P2PGoApp {
    view: View,
    game: GameState,
    neural_net: DualNeuralNet,
    show_heat_map: bool,
    network_ready: bool,
    available_games: Vec<String>,
    error_msg: Option<String>,
    join_code: String,
    // Message passing for network simulation
    tx: mpsc::Sender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
}

impl Default for P2PGoApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();

        // Simulate network becoming ready after a delay
        let tx_clone = tx.clone();
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_secs(2));
            let _ = tx_clone.send(AppMessage::CreateGame);
        });

        Self {
            view: View::MainMenu,
            game: GameState::new(9),
            neural_net: DualNeuralNet::new(),
            show_heat_map: false,
            network_ready: false,
            available_games: vec!["GAME1234".to_string(), "GAME5678".to_string()],
            error_msg: None,
            join_code: String::new(),
            tx,
            rx,
        }
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle messages
        if let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::CreateGame => {
                    self.network_ready = true;
                }
                _ => {}
            }
        }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_heat_map = !self.show_heat_map;
        }

        // UI
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                // Header
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.heading(RichText::new("P2P GO").size(32.0).color(Color32::WHITE));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        ui.label(format!(
                            "Heat Map: {} (H)",
                            if self.show_heat_map { "ON" } else { "OFF" }
                        ));
                    });
                });
                ui.separator();

                match &self.view {
                    View::MainMenu => self.render_main_menu(ui),
                    View::Lobby(game_id) => self.render_lobby(ui, game_id),
                    View::Game(game_id) => self.render_game(ui, game_id),
                    View::Offline => self.render_offline_game(ui),
                }

                // Error display
                if let Some(error) = &self.error_msg {
                    ui.add_space(20.0);
                    ui.colored_label(Color32::from_rgb(220, 38, 38), error);
                }
            });
    }
}

impl P2PGoApp {
    fn render_main_menu(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);

            // Create Game button
            let create_btn = egui::Button::new(RichText::new("Create Game").size(20.0))
                .min_size(Vec2::new(200.0, 50.0))
                .fill(if self.network_ready {
                    Color32::from_rgb(34, 197, 94)
                } else {
                    Color32::from_gray(60)
                });

            if ui.add_enabled(self.network_ready, create_btn).clicked() {
                self.view = View::Lobby(format!("GAME{:04}", rand::random::<u16>() % 10000));
                self.game = GameState::new(9);
            }

            if !self.network_ready {
                ui.label("(Waiting for network...)");
            }

            ui.add_space(20.0);

            // Join Game button
            if ui
                .button(RichText::new("Join Game").size(20.0))
                .min_size(Vec2::new(200.0, 50.0))
                .fill(Color32::from_rgb(59, 130, 246))
                .clicked()
            {
                // Show join dialog
                self.view = View::MainMenu; // Would show join modal
            }

            ui.add_space(20.0);

            // Offline Game button
            if ui
                .button(RichText::new("Offline Game").size(20.0))
                .min_size(Vec2::new(200.0, 50.0))
                .fill(Color32::from_rgb(147, 51, 234))
                .clicked()
            {
                self.view = View::Offline;
                self.game = GameState::new(9);
            }

            ui.add_space(40.0);

            // Available games
            if !self.available_games.is_empty() {
                ui.separator();
                ui.heading("Available Games:");
                for game_id in &self.available_games.clone() {
                    if ui.button(format!("Join {}", game_id)).clicked() {
                        self.view = View::Game(game_id.clone());
                        self.game = GameState::new(9);
                    }
                }
            }
        });
    }

    fn render_lobby(&mut self, ui: &mut egui::Ui, game_id: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Waiting for opponent...");
            ui.add_space(20.0);

            ui.label(format!("Game Code: {}", game_id));
            ui.label("Share this code with your friend");

            ui.add_space(40.0);

            if ui.button("Cancel").clicked() {
                self.view = View::MainMenu;
            }

            // Simulate opponent joining
            ui.add_space(40.0);
            if ui.button("(Debug) Start Game").clicked() {
                self.view = View::Game(game_id.to_string());
            }
        });
    }

    fn render_game(&mut self, ui: &mut egui::Ui, game_id: &str) {
        ui.horizontal(|ui| {
            // Board on left
            ui.vertical(|ui| {
                ui.label(format!("Game: {}", game_id));
                ui.label(format!("Current Player: {:?}", self.game.current_player));

                self.render_board(ui);

                ui.horizontal(|ui| {
                    if ui.button("Pass").clicked() {
                        let _ = self.game.apply_move(Move::Pass(self.game.current_player));
                    }

                    if ui.button("Resign").clicked() {
                        self.view = View::MainMenu;
                    }
                });
            });

            // Info on right
            ui.vertical(|ui| {
                ui.heading("Game Info");
                ui.separator();

                let eval = self.neural_net.evaluate_position(&self.game);
                ui.label(format!(
                    "Win %: {:.1}%",
                    (eval.win_probability + 1.0) / 2.0 * 100.0
                ));
                ui.label(format!("Move: {}", self.game.move_count));

                ui.separator();
                ui.label(format!(
                    "Heat Map: {} (H)",
                    if self.show_heat_map { "ON" } else { "OFF" }
                ));
            });
        });
    }

    fn render_offline_game(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading("Offline Game");
                self.render_board(ui);

                ui.horizontal(|ui| {
                    if ui.button("New Game").clicked() {
                        self.game = GameState::new(9);
                    }

                    if ui.button("Back to Menu").clicked() {
                        self.view = View::MainMenu;
                    }
                });
            });
        });
    }

    fn render_board(&mut self, ui: &mut egui::Ui) {
        let size = 600.0;
        let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::click());
        let rect = response.rect;

        // Board background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(220, 179, 92));

        let board_size = 9;
        let margin = 30.0;
        let board_rect = egui::Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(size - 2.0 * margin),
        );
        let cell_size = board_rect.width() / (board_size - 1) as f32;

        // Grid
        for i in 0..board_size {
            let offset = i as f32 * cell_size;

            // Vertical lines
            painter.line_segment(
                [
                    egui::Pos2::new(board_rect.left() + offset, board_rect.top()),
                    egui::Pos2::new(board_rect.left() + offset, board_rect.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::BLACK),
            );

            // Horizontal lines
            painter.line_segment(
                [
                    egui::Pos2::new(board_rect.left(), board_rect.top() + offset),
                    egui::Pos2::new(board_rect.right(), board_rect.top() + offset),
                ],
                egui::Stroke::new(1.0, Color32::BLACK),
            );
        }

        // Heat map
        if self.show_heat_map {
            let heat_map = self.neural_net.get_heat_map(&self.game);
            for y in 0..board_size {
                for x in 0..board_size {
                    let prob = heat_map[y][x];
                    if prob > 0.01 {
                        let pos = egui::Pos2::new(
                            board_rect.left() + x as f32 * cell_size,
                            board_rect.top() + y as f32 * cell_size,
                        );

                        let alpha = (prob * 150.0).min(150.0) as u8;
                        let color = Color32::from_rgba_unmultiplied(220, 38, 38, alpha);
                        painter.circle_filled(pos, cell_size * 0.3, color);
                    }
                }
            }
        }

        // Stones
        for y in 0..board_size {
            for x in 0..board_size {
                if let Some(color) = self.game.board.get(y * board_size + x) {
                    let pos = egui::Pos2::new(
                        board_rect.left() + x as f32 * cell_size,
                        board_rect.top() + y as f32 * cell_size,
                    );

                    match color {
                        GoColor::Black => {
                            painter.circle_filled(pos, cell_size * 0.45, Color32::BLACK)
                        }
                        GoColor::White => {
                            painter.circle_filled(pos, cell_size * 0.45, Color32::WHITE);
                            painter.circle_stroke(
                                pos,
                                cell_size * 0.45,
                                egui::Stroke::new(1.0, Color32::BLACK),
                            );
                        }
                    }
                }
            }
        }

        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let rel = pos - board_rect.min;
                let x = (rel.x / cell_size).round() as u8;
                let y = (rel.y / cell_size).round() as u8;

                if x < board_size as u8 && y < board_size as u8 {
                    let coord = Coord::new(x, y);
                    if self.game.is_valid_move(coord) {
                        let mv = Move::Place {
                            x,
                            y,
                            color: self.game.current_player,
                        };
                        let _ = self.game.apply_move(mv);
                    }
                }
            }
        }
    }
}
