// SPDX-License-Identifier: MIT OR Apache-2.0

//! Main application state and UI logic.

use crossbeam_channel::{Sender, Receiver};
use eframe::egui;
use p2pgo_core::{Move, Color};
use std::thread::JoinHandle;

use crate::msg::{UiToNet, NetToUi};
use crate::view::View;
use crate::board_widget::BoardWidget;

#[allow(dead_code)]
const DEFAULT_SIZE: u8 = 9; // was 19

/// Application configuration settings
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Whether to auto-refresh the lobby
    pub auto_refresh: bool,
    /// Number of completed games
    pub games_finished: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            games_finished: 0,
        }
    }
}

/// Main application state
pub struct App {
    /// Channel to send messages to network worker
    ui_tx: Sender<UiToNet>,
    /// Channel to receive messages from network worker
    ui_rx: Receiver<NetToUi>,
    /// Worker thread handle for proper cleanup
    worker_handle: Option<JoinHandle<()>>,
    /// Current view/screen
    #[cfg(test)]
    pub current_view: View,
    #[cfg(not(test))]
    current_view: View,
    /// Error message to display
    error_msg: Option<String>,
    /// Player name
    player_name: String,
    /// App configuration
    #[cfg(test)]
    pub config: AppConfig,
    #[cfg(not(test))]
    config: AppConfig,
    /// Board widget for rendering
    #[cfg(test)]
    pub board_widget: BoardWidget,
    #[cfg(not(test))]
    board_widget: BoardWidget,
    /// Show debug overlay
    show_overlay: bool,
    /// Last blob hash for debug display
    last_blob_hash: Option<String>,
    /// Receive queue length for debug display
    rx_queue_length: usize,
    /// Current node ID for display
    node_id: Option<String>,
    /// Show ticket modal
    show_ticket_modal: bool,
    /// Current ticket string
    current_ticket: Option<String>,
    /// Ticket input field
    ticket_input: String,
    /// NAT report result
    nat_report: Option<String>,
    /// Default board size for game creation and gossip subscription
    default_board_size: u8,
}

impl App {
    pub fn new(ui_tx: Sender<UiToNet>, ui_rx: Receiver<NetToUi>, board_size: u8, player_name: String) -> Self {
        // Request node ID on startup
        let _ = ui_tx.send(UiToNet::GetNodeId);
        
        Self {
            ui_tx,
            ui_rx,
            worker_handle: None,
            current_view: View::default(),
            error_msg: None,
            player_name,
            config: AppConfig::default(),
            board_widget: BoardWidget::new(board_size),
            show_overlay: false,
            last_blob_hash: None,
            rx_queue_length: 0,
            node_id: None,
            show_ticket_modal: false,
            current_ticket: None,
            ticket_input: String::new(),
            nat_report: None,
            default_board_size: board_size,
        }
    }

    /// Create a new headless app for testing
    #[cfg(test)]
    pub fn new_headless(ui_tx: Sender<UiToNet>, ui_rx: Receiver<NetToUi>) -> Self {
        Self {
            ui_tx,
            ui_rx,
            worker_handle: None,
            current_view: View::default(),
            error_msg: None,
            player_name: "HeadlessPlayer".to_string(),
            config: AppConfig::default(),
            board_widget: BoardWidget::new(DEFAULT_SIZE),
            show_overlay: false,
            last_blob_hash: None,
            rx_queue_length: 0,
            node_id: None,
            show_ticket_modal: false,
            current_ticket: None,
            ticket_input: String::new(),
            nat_report: None,
            default_board_size: DEFAULT_SIZE,
        }
    }
    
    /// Create a new headless app with provided channels
    #[allow(dead_code)]
    pub fn new_headless_with_channels(ui_tx: Sender<UiToNet>, ui_rx: Receiver<NetToUi>) -> Self {
        Self {
            ui_tx,
            ui_rx,
            worker_handle: None,
            current_view: View::default(),
            error_msg: None,
            player_name: "HeadlessPlayer".to_string(),
            config: AppConfig::default(),
            board_widget: BoardWidget::new(DEFAULT_SIZE),
            show_overlay: false,
            last_blob_hash: None,
            rx_queue_length: 0,
            node_id: None,
            show_ticket_modal: false,
            current_ticket: None,
            ticket_input: String::new(),
            nat_report: None,
            default_board_size: DEFAULT_SIZE,
        }
    }

    /// Set the worker thread handle for proper cleanup
    #[allow(dead_code)]
    pub fn set_worker_handle(&mut self, handle: JoinHandle<()>) {
        self.worker_handle = Some(handle);
    }

    #[cfg(any(feature = "headless", test))]
    pub fn tick_headless(&mut self) {
        self.handle_network_messages();
        
        // For testing purposes, populate auto-refresh message if needed
        if self.config.auto_refresh {
            let _ = self.ui_tx.send(UiToNet::RefreshGames);
        }
    }
    
    #[cfg(test)]
    pub fn tick_headless_test(&mut self) {
        self.handle_network_messages();
        
        // For testing purposes, populate auto-refresh message if needed
        if self.config.auto_refresh {
            let _ = self.ui_tx.send(UiToNet::RefreshGames);
        }
    }

    #[cfg(feature = "headless")]
    pub fn get_current_view_debug(&self) -> String {
        match &self.current_view {
            View::MainMenu { .. } => "MainMenu".to_string(),
            View::Lobby { game_id } => format!("Lobby({})", game_id),
            View::Game { game_id, .. } => format!("Game({})", game_id),
            View::ScoreDialog { .. } => "ScoreDialog".to_string(),
        }
    }

    #[cfg(feature = "headless")]
    pub fn get_current_game_state(&self) -> Result<p2pgo_core::GameState, String> {
        if let View::Game { game_state, .. } = &self.current_view {
            Ok(game_state.clone())
        } else {
            Err("Not in a game".to_string())
        }
    }

    /// Send a message to the network component
    #[allow(dead_code)]
    #[cfg(not(test))]
    pub fn ui_send(&self, msg: UiToNet) {
        let _ = self.ui_tx.send(msg);
    }

    /// Send a message to the network worker (for testing)
    #[cfg(test)]
    pub fn ui_send(&mut self, msg: UiToNet) {
        let _ = self.ui_tx.send(msg);
    }
    
    /// Get the last received message from the network worker (for testing)
    #[cfg(test)]
    pub fn recv_last(&mut self) -> Option<NetToUi> {
        let mut last = None;
        while let Ok(msg) = self.ui_rx.try_recv() {
            last = Some(msg);
        }
        last
    }
    
    /// Wait for and receive a specific message (for testing)
    #[cfg(test)]
    pub fn blocking_recv(&mut self) -> Option<NetToUi> {
        self.ui_rx.recv().ok()
    }
    
    /// Get all received messages for assertion (for testing)
    #[cfg(test)]
    pub fn recv_log(&mut self) -> Vec<NetToUi> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.ui_rx.try_recv() {
            messages.push(msg);
        }
        messages
    }
    
    /// Get current game ID (for testing)
    #[cfg(test)]
    pub fn get_current_game_id(&self) -> Option<String> {
        match &self.current_view {
            View::Game { game_id, .. } => Some(game_id.clone()),
            View::ScoreDialog { game_id, .. } => Some(game_id.clone()),
            View::Lobby { game_id } => Some(game_id.clone()),
            _ => None,
        }
    }

    /// Process network messages for testing
    #[cfg(test)]
    pub fn update(&mut self) {
        self.handle_network_messages();
    }
    
    /// Handle the AcceptScore message for testing
    #[cfg(test)]
    pub fn handle_accept_score(&mut self) {
        if let View::ScoreDialog { .. } = &self.current_view {
            self.config.games_finished += 1;
            self.current_view = View::default();
        }
    }
    
    // Method removed to avoid duplication - using the existing tick_headless_test

    fn handle_network_messages(&mut self) {
        self.rx_queue_length = self.ui_rx.len();
        
        while let Ok(msg) = self.ui_rx.try_recv() {
            #[cfg(feature = "headless")]
            println!("App received message: {:?}", msg);
            
            match msg {
                NetToUi::GamesUpdated { games } => {
                    if let View::MainMenu { available_games, creating_game: _, board_size: _ } = &mut self.current_view {
                        *available_games = games;
                    }
                }
                NetToUi::GameEvent { event } => {
                    match &event {
                        p2pgo_core::GameEvent::MoveMade { mv, .. } => {
                            #[cfg(feature = "headless")]
                            println!("Move made event received: {:?}", mv);
                            
                            // Transition from Lobby to Game on first move
                            if let View::Lobby { game_id } = &self.current_view {
                                #[cfg(feature = "headless")]
                                println!("Transitioning from Lobby to Game on first move");
                                
                                let board_size = self.board_widget.get_board_size();
                                let game_state = p2pgo_core::GameState::new(board_size);
                                self.current_view = View::Game {
                                    game_id: game_id.clone(),
                                    game_state,
                                    our_color: None, // We'll set this based on move order
                                };
                                // Request initial ghost moves when transitioning to game view
                                let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                            }
                            
                            if let View::Game { game_state, .. } = &mut self.current_view {
                                let _ = game_state.apply_move(mv.clone());
                                // Update last blob hash for debug overlay - use move type description
                                self.last_blob_hash = Some(format!("{:?}", mv));
                            }
                        },
                        p2pgo_core::GameEvent::GameFinished { black_score, white_score } => {
                            // Wait for ScoreCalculated message to transition to score dialog
                            // We'll just collect the scores here for now
                            tracing::info!("Game finished - black: {}, white: {}", black_score, white_score);
                        },
                        _ => {
                            // Request ghost moves after the move is applied
                            let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                        }
                    }
                }
                NetToUi::GameJoined { game_id } => {
                    #[cfg(feature = "headless")]
                    println!("Game joined: {}, transitioning to Lobby", game_id);
                    self.current_view = View::Lobby { game_id };
                    // Request initial ghost moves when joining a game
                    let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                }
                NetToUi::GameLeft => {
                    self.current_view = View::default();
                }
                NetToUi::Error { message } => {
                    self.error_msg = Some(message);
                }
                NetToUi::ConnectionStatus { .. } => {}
                NetToUi::ShutdownAck => {
                    tracing::debug!("Received shutdown acknowledgment from network worker");
                }
                NetToUi::NodeId { node_id } => {
                    self.node_id = Some(node_id);
                }
                NetToUi::Ticket { ticket } => {
                    self.current_ticket = Some(ticket);
                }
                NetToUi::NetReport { report } => {
                    self.nat_report = Some(report);
                }
                NetToUi::TagAck => {
                    tracing::debug!("Move tag stored successfully");
                }
                NetToUi::GhostMoves(coords) => {
                    tracing::debug!("Received {} ghost move suggestions", coords.len());
                    self.board_widget.set_ghost_stones(coords);
                }
                NetToUi::ScoreCalculated { score_proof } => {
                    // Transition to score dialog with the calculated score
                    if let View::Game { game_id, game_state, .. } = &self.current_view {
                        self.current_view = View::ScoreDialog {
                            game_id: game_id.clone(),
                            game_state: game_state.clone(),
                            score_proof,
                            dead_stones: std::collections::HashSet::new(),
                            score_pending: true,
                            score_accepted: false,
                        };
                    }
                }
                
                NetToUi::ScoreAcceptedByBoth { score_proof } => {
                    // Update score dialog to show accepted score
                    if let View::ScoreDialog { game_id, game_state, dead_stones, .. } = &self.current_view {
                        self.current_view = View::ScoreDialog {
                            game_id: game_id.clone(),
                            game_state: game_state.clone(),
                            score_proof,
                            dead_stones: dead_stones.clone(),
                            score_pending: false,
                            score_accepted: true,
                        };
                    }
                }
                NetToUi::ScoreTimeout { board_size } => {
                    // Score acceptance timed out after 3 minutes
                    self.error_msg = Some(format!("Score acceptance timed out for {}×{} game. Game will be discarded.", board_size, board_size));
                    // Return to main menu
                    self.current_view = View::default();
                }
                NetToUi::Debug(message) => {
                    tracing::debug!("Debug message: {}", message);
                }
                NetToUi::GameAdvertised { game_id, host_id, board_size } => {
                    tracing::info!("Game advertisement received: {} ({}×{}) from {}", game_id, board_size, board_size, host_id);
                    // Request a refresh of the game list to show the advertised game
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
            }
        }
    }

    fn render_main_menu(&mut self, ui: &mut egui::Ui) {
        if let View::MainMenu { available_games, creating_game, board_size } = &mut self.current_view {
            ui.heading("P2P Go");
            
            // Node ID section
            ui.separator();
            ui.label("Network Information:");
            if let Some(node_id) = &self.node_id {
                ui.horizontal(|ui| {
                    ui.label(format!("Node ID: {}", node_id));
                    if ui.button("Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = node_id.clone());
                    }
                });
            } else {
                ui.label("Node ID: Loading...");
            }
            
            ui.horizontal(|ui| {
                if ui.button("Generate Ticket").clicked() {
                    let _ = self.ui_tx.send(UiToNet::GetTicket);
                }
                if ui.button("Connect by Ticket").clicked() {
                    self.show_ticket_modal = true;
                }
            });
            
            // Paste ticket input field with auto-connect on Enter
            ui.horizontal(|ui| {
                ui.label("Paste ticket ↵");
                let text_edit = ui.text_edit_singleline(&mut self.ticket_input);
                
                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.ticket_input.trim().is_empty() {
                    let _ = self.ui_tx.send(UiToNet::ConnectByTicket { 
                        ticket: self.ticket_input.clone() 
                    });
                    self.ticket_input.clear();
                }
            });
            
            if let Some(ticket) = &self.current_ticket {
                ui.horizontal(|ui| {
                    // Check if ticket is a stub or has relay status
                    let is_stub = ticket == "loopback-ticket";
                    let relay_status = ticket.len() > 50; // Real tickets are much longer than stub
                    
                    let label_text = if is_stub {
                        egui::RichText::new("Generated Ticket (Local Mode):").color(egui::Color32::BLUE)
                    } else if relay_status {
                        egui::RichText::new("Generated Ticket (Network Ready):").color(egui::Color32::GREEN)
                    } else {
                        egui::RichText::new("Generated Ticket (Local Only):").color(egui::Color32::YELLOW)
                    };
                    ui.label(label_text);
                    
                    if ui.button("Copy Ticket").clicked() {
                        ui.output_mut(|o| o.copied_text = ticket.clone());
                    }
                });
                ui.text_edit_singleline(&mut ticket.clone());
            }
            
            ui.separator();
            
            // Board size selection with radio buttons
            ui.label("Board size:");
            ui.horizontal(|ui| {
                let old_size = *board_size;
                
                if ui.radio_value(board_size, 9, "9×9").clicked() && old_size != 9 {
                    self.default_board_size = 9;
                    // Update the worker with the new board size
                    let _ = self.ui_tx.send(UiToNet::UpdateBoardSize { board_size: 9 });
                    // Re-subscribe to gossip with new board size
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
                if ui.radio_value(board_size, 13, "13×13").clicked() && old_size != 13 {
                    self.default_board_size = 13;
                    // Update the worker with the new board size
                    let _ = self.ui_tx.send(UiToNet::UpdateBoardSize { board_size: 13 });
                    // Re-subscribe to gossip with new board size
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
                if ui.radio_value(board_size, 19, "19×19").clicked() && old_size != 19 {
                    self.default_board_size = 19;
                    // Update the worker with the new board size
                    let _ = self.ui_tx.send(UiToNet::UpdateBoardSize { board_size: 19 });
                    // Re-subscribe to gossip with new board size
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
            });
            
            // Create Game button - disabled if no ticket available or relay not ready
            let is_stub = self.current_ticket.as_ref().map(|t| t == "loopback-ticket").unwrap_or(false);
            let network_ready = self.current_ticket.as_ref()
                .map(|ticket| is_stub || ticket.len() > 50) // Stub is always ready, real tickets should be long
                .unwrap_or(false);
            
            let create_btn = ui.add_enabled(
                self.current_ticket.is_some() && network_ready, 
                egui::Button::new("Create Game")
            );
            
            if create_btn.clicked() {
                let _ = self.ui_tx.send(UiToNet::CreateGame { board_size: *board_size });
                *creating_game = true;
            }
            
            // Show hint if button is disabled
            if let Some(_ticket) = &self.current_ticket {
                if !network_ready {
                    ui.label(egui::RichText::new("Waiting for network initialization...").italics().color(egui::Color32::GRAY));
                }
            } else {
                ui.label(egui::RichText::new("Generate a ticket first").italics().color(egui::Color32::GRAY));
            }
            
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("Refresh Games").clicked() {
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
                
                ui.checkbox(&mut self.config.auto_refresh, "Auto-refresh (2s)");
            });
            
            ui.label("Available Games:");
            for game in available_games {
                if ui.button(format!("Game {} ({}×{})", game.id, game.board_size, game.board_size)).clicked() {
                    let _ = self.ui_tx.send(UiToNet::JoinGame { game_id: game.id.clone() });
                }
            }
        }
    }

    fn render_lobby(&mut self, ui: &mut egui::Ui) {
        if let View::Lobby { game_id, .. } = &self.current_view {
            ui.heading("Waiting for opponent...");
            ui.label(format!("Game ID: {}", game_id));
            
            // Show ticket when available
            if let Some(ticket) = &self.current_ticket {
                ui.horizontal(|ui| {
                    ui.label("Your ticket:");
                    ui.label(ticket);
                    if ui.button("Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = ticket.clone());
                    }
                });
            }
            
            if ui.button("Leave Game").clicked() {
                let _ = self.ui_tx.send(UiToNet::LeaveGame);
                let _ = self.ui_tx.send(UiToNet::Shutdown);
            }
        }
    }

    fn render_game(&mut self, ui: &mut egui::Ui) {
        if let View::Game { game_id, game_state, .. } = &self.current_view {
            // Store the game ID in UI memory for the board widget to access
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new("current_game_id"), game_id.clone());
            });
            
            let current_player = match game_state.current_player {
                Color::Black => "Black",
                Color::White => "White",
            };
            ui.label(format!("Current player: {}", current_player));
            
            if let Some(coord) = self.board_widget.render(ui, game_state, Some(&self.ui_tx)) {
                let mv = Move::Place(coord);
                let _ = self.ui_tx.send(UiToNet::MakeMove { mv, board_size: None });
                // Request ghost moves after making a move
                let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
            }
            
            ui.horizontal(|ui| {
                if ui.button("Pass").clicked() {
                    let _ = self.ui_tx.send(UiToNet::MakeMove { mv: Move::Pass, board_size: None });
                    let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                }
                if ui.button("Resign").clicked() {
                    let _ = self.ui_tx.send(UiToNet::MakeMove { mv: Move::Resign, board_size: None });
                    let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                }
                if ui.button("Leave Game").clicked() {
                    let _ = self.ui_tx.send(UiToNet::LeaveGame);
                    let _ = self.ui_tx.send(UiToNet::Shutdown);
                }
            });
        }
    }
    
    fn render_score_dialog(&mut self, ui: &mut egui::Ui) {
        if let View::ScoreDialog { game_id, game_state: _, score_proof, dead_stones: _, score_pending: _, score_accepted } = &mut self.current_view.clone() {
            ui.heading("Game Finished");
            ui.label(format!("Game ID: {}", game_id));
            ui.separator();
            
            // Display score details
            ui.label(format!("Black territory: {}", score_proof.territory_black));
            ui.label(format!("White territory: {}", score_proof.territory_white));
            ui.label(format!("Black captures: {}", score_proof.captures_black));
            ui.label(format!("White captures: {}", score_proof.captures_white));
            ui.label(format!("Komi: {}", score_proof.komi));
            
            let final_score = score_proof.final_score;
            let winner = if final_score > 0 { "Black" } else if final_score < 0 { "White" } else { "Draw" };
            ui.heading(format!("Winner: {} (by {})", winner, final_score.abs()));
            ui.separator();
            
            if !*score_accepted {
                if ui.button("Accept Result").clicked() {
                    // Send AcceptScore message to worker
                    let _ = self.ui_tx.send(UiToNet::AcceptScore { 
                        score_proof: score_proof.clone() 
                    });
                    
                    // Update UI state directly without creating a new View
                    if let View::ScoreDialog { score_accepted: ref mut acc, score_pending: ref mut pend, .. } = self.current_view {
                        *acc = true;
                        *pend = false;
                    }
                }
            } else {
                // Show return button once score is accepted
                if ui.button("Return to Main Menu").clicked() {
                    // Increment completed games counter
                    self.config.games_finished += 1;
                    
                    // Return to main menu
                    self.current_view = View::default();
                    let _ = self.ui_tx.send(UiToNet::LeaveGame);
                }
            }
        }
    }
    
    fn render_debug_overlay(&mut self, ctx: &egui::Context) {
        if !self.show_overlay {
            return;
        }
        
        egui::Window::new("Debug Overlay")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 10.0))
            .show(ctx, |ui| {
                ui.label(format!("Player: {}", self.player_name));
                
                let current_view_name = match &self.current_view {
                    View::MainMenu { .. } => "MainMenu",
                    View::Lobby { .. } => "Lobby", 
                    View::Game { .. } => "Game",
                    View::ScoreDialog { .. } => "ScoreDialog",
                };
                ui.label(format!("View: {}", current_view_name));
                
                if let View::Lobby { game_id, .. } | View::Game { game_id, .. } = &self.current_view {
                    ui.label(format!("Game ID: {}", game_id));
                }
                
                if let Some(hash) = &self.last_blob_hash {
                    ui.label(format!("Last Blob: {}", hash));
                } else {
                    ui.label("Last Blob: None");
                }
                
                ui.label(format!("RX Queue: {}", self.rx_queue_length));
                
                if let View::Game { game_state, .. } = &self.current_view {
                    ui.label(format!("Turn: {:?}", game_state.current_player));
                    ui.label(format!("Moves: {}", game_state.moves.len()));
                }
                
                ui.separator();
                if ui.button("Run NAT Report").clicked() {
                    let _ = self.ui_tx.send(UiToNet::RunNetReport);
                }
                
                if let Some(report) = &self.nat_report {
                    ui.label("NAT Report:");
                    ui.text_edit_multiline(&mut report.clone());
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_network_messages();
        
        // Handle F1 key to toggle debug overlay
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_overlay = !self.show_overlay;
            tracing::debug!("Debug overlay toggled: {}", self.show_overlay);
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.current_view {
                View::MainMenu { .. } => self.render_main_menu(ui),
                View::Lobby { .. } => self.render_lobby(ui),
                View::Game { .. } => self.render_game(ui),
                View::ScoreDialog { .. } => self.render_score_dialog(ui),
            }
            
            if let Some(error) = self.error_msg.clone() {
                egui::Window::new("Error")
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(ctx, |ui| {
                        ui.heading("Error");
                        ui.separator();
                        ui.label(&error);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                self.error_msg = None;
                            }
                        });
                    });
            }
            
            if self.show_ticket_modal {
                egui::Window::new("Connect by Ticket")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Enter ticket to connect:");
                        ui.text_edit_singleline(&mut self.ticket_input);
                        ui.horizontal(|ui| {
                            let connect_btn = ui.add_enabled(
                                !self.ticket_input.trim().is_empty(),
                                egui::Button::new("Connect")
                            );
                            
                            if connect_btn.clicked() {
                                let _ = self.ui_tx.send(UiToNet::ConnectByTicket { 
                                    ticket: self.ticket_input.clone() 
                                });
                                self.ticket_input.clear();
                                self.show_ticket_modal = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.ticket_input.clear();
                                self.show_ticket_modal = false;
                            }
                        });
                    });
            }
        });
        
        // Render debug overlay on top
        self.render_debug_overlay(ctx);
        
        ctx.request_repaint();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(handle) = self.worker_handle.take() {
            // Tell worker to quit
            let _ = self.ui_tx.send(UiToNet::Shutdown);
            // Wait for graceful shutdown
            handle.join().ok();
        }
    }
}
