//! Clean UI implementation

use crate::game::GameManager;
use crate::network::NetworkManager;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use p2pgo_core::Color as GoColor;
use p2pgo_neural::DualNeuralNet;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(PartialEq, Clone)]
pub enum View {
    MainMenu,
    Lobby { game_id: String },
    Game { game_id: String },
    Settings,
}

pub struct P2PGoApp {
    view: View,
    game_manager: Arc<Mutex<GameManager>>,
    network_manager: Arc<NetworkManager>,
    neural_net: DualNeuralNet,
    show_heat_map: bool,

    // UI state
    error_message: Option<String>,
    join_game_input: String,
    player_name: String,

    // Runtime
    tokio_runtime: tokio::runtime::Runtime,
}

impl P2PGoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let game_manager = Arc::new(Mutex::new(GameManager::new()));
        let network_manager = Arc::new(NetworkManager::new());

        Self {
            view: View::MainMenu,
            game_manager,
            network_manager,
            neural_net: DualNeuralNet::new(),
            show_heat_map: false,
            error_message: None,
            join_game_input: String::new(),
            player_name: "Player".to_string(),
            tokio_runtime: runtime,
        }
    }

    fn create_game(&mut self) {
        let game_id = format!("GAME{:04}", rand::random::<u16>() % 10000);

        let game_manager = self.game_manager.clone();
        let game_id_clone = game_id.clone();

        self.tokio_runtime.spawn(async move {
            let mut gm = game_manager.lock().await;
            gm.create_game(&game_id_clone, 9);
        });

        self.view = View::Lobby { game_id };
    }

    fn join_game(&mut self, game_id: String) {
        let game_manager = self.game_manager.clone();
        let game_id_clone = game_id.clone();

        self.tokio_runtime.spawn(async move {
            let mut gm = game_manager.lock().await;
            gm.join_game(&game_id_clone);
        });

        self.view = View::Game { game_id };
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.show_heat_map = !self.show_heat_map;
        }

        // Apply dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Main panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_gray(20)))
            .show(ctx, |ui| {
                self.render_header(ui);

                match &self.view.clone() {
                    View::MainMenu => self.render_main_menu(ui),
                    View::Lobby { game_id } => self.render_lobby(ui, game_id),
                    View::Game { game_id } => self.render_game(ui, game_id),
                    View::Settings => self.render_settings(ui),
                }

                // Error display
                if let Some(error) = &self.error_message {
                    ui.add_space(20.0);
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(Color32::from_rgb(220, 38, 38), error);
                    });
                }
            });
    }
}

impl P2PGoApp {
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.heading(RichText::new("P2P GO").size(28.0));

            ui.separator();

            // Navigation
            if ui
                .selectable_label(self.view == View::MainMenu, "Menu")
                .clicked()
            {
                self.view = View::MainMenu;
            }

            if ui
                .selectable_label(self.view == View::Settings, "Settings")
                .clicked()
            {
                self.view = View::Settings;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                ui.label(format!(
                    "Heat Map: {} (H)",
                    if self.show_heat_map { "🔥" } else { "❄️" }
                ));
                ui.separator();

                // Network status
                let status_color = if self.network_manager.is_connected() {
                    Color32::GREEN
                } else {
                    Color32::YELLOW
                };
                ui.colored_label(status_color, "●");
                ui.label("Network");
            });
        });

        ui.separator();
    }

    fn render_main_menu(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);

            // Create Game button - always enabled now
            let button = egui::Button::new(RichText::new("🎮 Create Game").size(20.0))
                .fill(Color32::from_rgb(34, 197, 94));
            if ui.add_sized(Vec2::new(250.0, 60.0), button).clicked() {
                self.create_game();
            }

            ui.add_space(20.0);

            // Join Game section
            ui.group(|ui| {
                ui.heading("Join Game");
                ui.horizontal(|ui| {
                    ui.label("Game Code:");
                    ui.text_edit_singleline(&mut self.join_game_input);
                    if ui.button("Join").clicked() && !self.join_game_input.is_empty() {
                        let game_id = self.join_game_input.clone();
                        self.join_game_input.clear();
                        self.join_game(game_id);
                    }
                });
            });

            ui.add_space(20.0);

            // Quick Play
            let button = egui::Button::new(RichText::new("⚡ Quick Play").size(20.0))
                .fill(Color32::from_rgb(59, 130, 246));
            if ui.add_sized(Vec2::new(250.0, 60.0), button).clicked() {
                // Join any available game or create new one
                self.create_game();
            }

            ui.add_space(20.0);

            // Offline Game
            let button = egui::Button::new(RichText::new("🖥️ Offline Game").size(20.0))
                .fill(Color32::from_rgb(147, 51, 234));
            if ui.add_sized(Vec2::new(250.0, 60.0), button).clicked() {
                let game_id = "OFFLINE".to_string();
                self.view = View::Game { game_id };
            }
        });
    }

    fn render_lobby(&mut self, ui: &mut egui::Ui, game_id: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Waiting for opponent...");

            ui.add_space(30.0);

            // Game code display
            ui.group(|ui| {
                ui.heading("Game Code");
                ui.label(RichText::new(game_id).size(24.0).monospace());
                if ui.button("📋 Copy").clicked() {
                    ui.output_mut(|o| o.copied_text = game_id.to_string());
                }
            });

            ui.add_space(20.0);
            ui.label("Share this code with your friend to play!");

            ui.add_space(40.0);

            // Actions
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    self.view = View::MainMenu;
                }

                // Debug: start game immediately
                if ui.button("Start Game (Debug)").clicked() {
                    self.view = View::Game {
                        game_id: game_id.to_string(),
                    };
                }
            });
        });
    }

    fn render_game(&mut self, ui: &mut egui::Ui, game_id: &str) {
        ui.horizontal(|ui| {
            // Left side - Game board
            ui.vertical(|ui| {
                ui.label(format!("Game: {}", game_id));

                // Get game state
                let game_state = self.tokio_runtime.block_on(async {
                    let gm = self.game_manager.lock().await;
                    gm.get_game_state(game_id).cloned()
                });

                if let Some(game_state) = game_state {
                    ui.label(format!("Current: {:?}", game_state.current_player));

                    // Render board
                    self.render_board(ui, &game_state, game_id);

                    // Game controls
                    ui.horizontal(|ui| {
                        if ui.button("Pass").clicked() {
                            self.make_move(game_id, p2pgo_core::Move::Pass);
                        }

                        if ui.button("Resign").clicked() {
                            self.make_move(game_id, p2pgo_core::Move::Resign);
                            self.view = View::MainMenu;
                        }

                        if ui.button("Leave").clicked() {
                            self.view = View::MainMenu;
                        }
                    });
                } else {
                    ui.label("Game not found!");
                    if ui.button("Back to Menu").clicked() {
                        self.view = View::MainMenu;
                    }
                }
            });

            // Right side - Info panel
            ui.vertical(|ui| {
                ui.heading("Game Info");
                ui.separator();

                if let Some(game_state) = self.tokio_runtime.block_on(async {
                    let gm = self.game_manager.lock().await;
                    gm.get_game_state(game_id).cloned()
                }) {
                    // Neural evaluation
                    let eval = self.neural_net.evaluate_position(&game_state);
                    ui.label(format!(
                        "Win %: {:.1}%",
                        (eval.win_probability + 1.0) / 2.0 * 100.0
                    ));
                    ui.label(format!("Confidence: {:.0}%", eval.confidence * 100.0));

                    ui.separator();

                    // Game stats
                    ui.label(format!("Move: {}", game_state.moves.len()));
                    ui.label(format!("Captured ⚫: {}", game_state.captures.0));
                    ui.label(format!("Captured ⚪: {}", game_state.captures.1));
                }
            });
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

        ui.checkbox(&mut self.show_heat_map, "Show heat map by default");

        ui.separator();

        if ui.button("SGF Training").clicked() {
            // Open file dialog
            if let Some(paths) = rfd::FileDialog::new()
                .add_filter("SGF files", &["sgf"])
                .pick_files()
            {
                ui.label(format!("Selected {} files", paths.len()));
            }
        }
    }

    fn render_board(
        &mut self,
        ui: &mut egui::Ui,
        game_state: &p2pgo_core::GameState,
        game_id: &str,
    ) {
        let size = 600.0;
        let (response, painter) = ui.allocate_painter(Vec2::new(size, size), egui::Sense::click());
        let rect = response.rect;

        // Board background
        painter.rect_filled(rect, 0.0, Color32::from_rgb(220, 179, 92));

        let board_size = game_state.board_size as usize;
        let margin = 30.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(size - 2.0 * margin),
        );
        let cell_size = board_rect.width() / (board_size - 1) as f32;

        // Grid lines
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

        // Heat map
        if self.show_heat_map {
            let heat_map = self.neural_net.get_heat_map(game_state);
            for y in 0..board_size.min(19) {
                for x in 0..board_size.min(19) {
                    let prob = heat_map[y][x];
                    if prob > 0.01 {
                        let pos = Pos2::new(
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
                let idx = y * board_size + x;
                if idx < game_state.board.len() {
                    if let Some(color) = &game_state.board[idx] {
                        let pos = Pos2::new(
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
                                    Stroke::new(1.0, Color32::BLACK),
                                );
                            }
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
                    // Check if position is empty
                    let idx = (y as usize) * board_size + (x as usize);
                    if idx < game_state.board.len() && game_state.board[idx].is_none() {
                        let mv = p2pgo_core::Move::Place {
                            x,
                            y,
                            color: game_state.current_player,
                        };
                        self.make_move(game_id, mv);
                    }
                }
            }
        }
    }

    fn make_move(&mut self, game_id: &str, mv: p2pgo_core::Move) {
        let game_manager = self.game_manager.clone();
        let game_id = game_id.to_string();

        self.tokio_runtime.spawn(async move {
            let mut gm = game_manager.lock().await;
            let _ = gm.make_move(&game_id, mv);
        });
    }
}
