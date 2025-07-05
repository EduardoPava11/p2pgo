//! Main application state and logic

use egui::{CentralPanel, TopBottomPanel, Ui, Widget};
use p2pgo_core::{GameState, Coord};
use p2pgo_neural::DualNeuralNet;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use p2pgo_ui_egui::msg::{UiToNet, NetToUi};

use crate::core::{apply_theme, Colors, Spacing, ghost_button};
use crate::features::{
    GameView, GameAction, 
    LobbyView, LobbyAction,
    TrainingView, TrainingAction,
};
use crate::app::router::{Router, View};

pub struct P2PGoApp {
    router: Router,
    neural_net: DualNeuralNet,
    
    // Network communication
    net_tx: mpsc::UnboundedSender<UiToNet>,
    net_rx: mpsc::UnboundedReceiver<NetToUi>,
    
    // Views
    lobby_view: LobbyView,
    game_view: Option<GameView>,
    training_view: TrainingView,
    
    // App state
    player_name: String,
    show_network_panel: bool,
    network_status: String,
    active_game_id: Option<String>,
    board_size: u8,
}

impl P2PGoApp {
    pub fn new(cc: &eframe::CreationContext<'_>, net_tx: mpsc::UnboundedSender<UiToNet>, net_rx: mpsc::UnboundedReceiver<NetToUi>) -> Self {
        // Apply theme
        apply_theme(&cc.egui_ctx);
        
        let neural_net = DualNeuralNet::new();
        
        Self {
            router: Router::new(),
            neural_net,
            net_tx,
            net_rx,
            
            lobby_view: LobbyView::new(),
            game_view: None,
            training_view: TrainingView::new(),
            
            player_name: "Player".to_string(),
            show_network_panel: false,
            network_status: "Connecting...".to_string(),
            active_game_id: None,
            board_size: 9,
        }
    }
    
    fn render_header(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(Spacing::SM);
            
            ui.horizontal(|ui| {
                // Logo/Title
                ui.heading("P2P Go");
                
                ui.separator();
                
                // Navigation
                if self.router.can_go_back() {
                    if ghost_button("← Back").ui(ui).clicked() {
                        self.router.go_back();
                    }
                }
                
                ui.separator();
                
                // View buttons
                if ghost_button("Lobby").ui(ui).clicked() {
                    self.router.navigate_to(View::Lobby);
                }
                
                if ghost_button("Training").ui(ui).clicked() {
                    self.router.navigate_to(View::Training);
                }
                
                if ghost_button("Settings").ui(ui).clicked() {
                    self.router.navigate_to(View::Settings);
                }
                
                // Right side - network status
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🌐 Network").clicked() {
                        self.show_network_panel = !self.show_network_panel;
                    }
                });
            });
            
            ui.add_space(Spacing::SM);
        });
    }
    
    fn render_content(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            match self.router.current() {
                View::Lobby => {
                    let action = self.lobby_view.show(ui);
                    self.handle_lobby_action(action);
                }
                
                View::Game(code) => {
                    if let Some(game_view) = &mut self.game_view {
                        game_view.handle_keyboard(ctx);
                        let action = game_view.show(ui, &self.neural_net);
                        self.handle_game_action(action);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label("Loading game...");
                        });
                    }
                }
                
                View::Training => {
                    let action = self.training_view.show(ui);
                    self.handle_training_action(action);
                }
                
                View::Settings => {
                    self.render_settings(ui);
                }
            }
        });
    }
    
    fn render_settings(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.separator();
        
        ui.label("Coming soon...");
    }
    
    fn handle_lobby_action(&mut self, action: LobbyAction) {
        match action {
            LobbyAction::CreateGame(_) => {
                // Request network to create game
                let _ = self.net_tx.send(UiToNet::CreateGame { board_size: self.board_size });
                self.lobby_view.show_create_dialog = true;
                self.lobby_view.create_game_code = "Creating game...".to_string();
            }
            
            LobbyAction::JoinGame(code) => {
                // Join via ticket if it looks like one, otherwise join by game ID
                if code.contains("iroh") {
                    let _ = self.net_tx.send(UiToNet::ConnectToTicket { ticket: code });
                } else {
                    let _ = self.net_tx.send(UiToNet::JoinGame { game_id: code });
                }
            }
            
            LobbyAction::None => {}
        }
    }
    
    fn handle_game_action(&mut self, action: GameAction) {
        if let Some(game_view) = &mut self.game_view {
            match action {
                GameAction::PlaceStone(pos) => {
                    if game_view.is_our_turn {
                        let current_player = game_view.game.current_player;
                        if game_view.game.apply_move(p2pgo_core::Move::Place { x: pos.x, y: pos.y, color: current_player }).is_ok() {
                            game_view.is_our_turn = false;
                            // Send move over P2P
                            let _ = self.net_tx.send(UiToNet::SendMove { 
                                coord: Coord { x: pos.x, y: pos.y }
                            });
                        }
                    }
                }
                
                GameAction::Pass => {
                    if game_view.is_our_turn && game_view.game.apply_move(p2pgo_core::Move::Pass).is_ok() {
                        game_view.is_our_turn = false;
                        let _ = self.net_tx.send(UiToNet::Pass);
                    }
                }
                
                GameAction::Undo => {
                    let _ = self.net_tx.send(UiToNet::RequestUndo);
                }
                
                GameAction::Resign => {
                    let _ = self.net_tx.send(UiToNet::Resign);
                    self.router.navigate_to(View::Lobby);
                }
                
                GameAction::None => {}
            }
        }
    }
    
    fn handle_training_action(&mut self, action: TrainingAction) {
        match action {
            TrainingAction::SelectFiles => {
                // Open file dialog
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .set_title("Select SGF files (1-10)")
                    .pick_files()
                {
                    if paths.len() <= 10 {
                        self.training_view.selected_files = paths;
                        self.training_view.error_message = None;
                    } else {
                        self.training_view.error_message = Some("Please select at most 10 files".to_string());
                    }
                }
            }
            
            TrainingAction::StartTraining => {
                self.training_view.training_active = true;
                self.training_view.training_progress = 0.0;
                self.training_view.error_message = None;
                
                // Send message to start training
                let sgf_paths = self.training_view.selected_files.clone();
                if let Err(e) = self.net_tx.send(UiToNet::StartTraining { sgf_paths }) {
                    self.training_view.error_message = Some(format!("Failed to start training: {}", e));
                    self.training_view.training_active = false;
                }
            }
            
            TrainingAction::CancelTraining => {
                self.training_view.training_active = false;
                self.training_view.training_progress = 0.0;
                // Send cancel message
                let _ = self.net_tx.send(UiToNet::CancelTraining);
            }
            
            TrainingAction::None => {}
        }
    }
}

impl P2PGoApp {
    fn handle_network_messages(&mut self) {
        while let Ok(msg) = self.net_rx.try_recv() {
            match msg {
                NetToUi::GameJoined { game_id } => {
                    self.active_game_id = Some(game_id.clone());
                    // Create game view
                    let game = GameState::new(self.board_size);
                    let mut game_view = GameView::new(game);
                    game_view.game_code = Some(game_id.clone());
                    self.game_view = Some(game_view);
                    
                    // Show game board
                    self.router.navigate_to(View::Game(game_id));
                    self.lobby_view.show_create_dialog = false;
                    self.lobby_view.show_join_dialog = false;
                }
                
                NetToUi::Ticket { ticket } => {
                    // Update create dialog with actual ticket
                    self.lobby_view.create_game_code = ticket;
                }
                
                NetToUi::GameState { game_state } => {
                    if let Some(game_view) = &mut self.game_view {
                        game_view.game = game_state;
                    }
                }
                
                NetToUi::Error { message } => {
                    // Show error in appropriate view
                    if self.lobby_view.show_join_dialog {
                        self.lobby_view.error_message = Some(message);
                    } else {
                        self.network_status = format!("Error: {}", message);
                    }
                }
                
                NetToUi::GameList { games } => {
                    self.lobby_view.available_games = games.into_iter().map(|g| {
                        crate::features::lobby_view::GameListing {
                            code: g.id,
                            host: g.host_name.unwrap_or("Unknown".to_string()),
                            board_size: g.board_size,
                            status: "Waiting".to_string(),
                        }
                    }).collect();
                }
                
                NetToUi::OpponentJoined { player_name } => {
                    if let Some(game_view) = &mut self.game_view {
                        game_view.opponent_name = player_name;
                        game_view.is_our_turn = true; // Creator goes first
                    }
                    self.network_status = "Opponent joined! You go first.".to_string();
                }
                
                NetToUi::MoveReceived { coord, color } => {
                    if let Some(game_view) = &mut self.game_view {
                        let _ = game_view.game.apply_move(p2pgo_core::Move::Place { 
                            x: coord.x, 
                            y: coord.y, 
                            color 
                        });
                        game_view.is_our_turn = true;
                    }
                }
                
                NetToUi::TrainingProgress { progress } => {
                    self.training_view.training_progress = progress;
                }
                
                NetToUi::TrainingCompleted { stats } => {
                    self.training_view.training_active = false;
                    self.training_view.training_progress = 1.0;
                    self.training_view.last_stats = Some(stats);
                    self.training_view.error_message = None;
                }
                
                NetToUi::TrainingError { message } => {
                    self.training_view.training_active = false;
                    self.training_view.training_progress = 0.0;
                    self.training_view.error_message = Some(message);
                }
                
                _ => {} // Handle other messages as needed
            }
        }
    }
}

impl eframe::App for P2PGoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle network messages
        self.handle_network_messages();
        
        // Request repaint for animations
        ctx.request_repaint();
        
        // Render UI
        self.render_header(ctx);
        self.render_content(ctx);
        
        // Network panel overlay
        if self.show_network_panel {
            egui::Window::new("Network Status")
                .open(&mut self.show_network_panel)
                .show(ctx, |ui| {
                    ui.label(&self.network_status);
                    if ui.button("Refresh Games").clicked() {
                        let _ = self.net_tx.send(UiToNet::RefreshGames);
                    }
                });
        }
    }
}