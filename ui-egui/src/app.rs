// SPDX-License-Identifier: MIT OR Apache-2.0

//! Main application state and UI logic.

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use p2pgo_core::{Color, Coord, Move};
use std::thread::JoinHandle;

use crate::board_widget::BoardWidget;
use crate::clipboard_helper::ClipboardHelper;
use crate::connection_status::{ConnectionState, ConnectionStatusWidget};
use crate::dual_heat_map::DualHeatMap;
use crate::error_logger::{ErrorLogViewer, ErrorLogger};
use crate::heat_map::HeatMapOverlay;
use crate::labeled_input::{show_labeled_identifier, show_labeled_input, IdentifierType};
use crate::msg::{NetToUi, UiToNet};
use crate::network_panel::NetworkPanel;
use crate::neural_placeholder::{NeuralOverlay, NeuralTrainingUI};
use crate::offline_game::OfflineGoGame;
use crate::toast_manager::{ToastManager, ToastType};
use crate::view::View;
// use crate::update_checker::{UpdateChecker, UpdateCheckResult, Version};
// use crate::update_ui::{UpdateNotification, UpdateDialog, UpdateAction};

// Version from build.rs
const VERSION: &str = env!("P2PGO_VERSION");

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
    /// Network diagnostics panel
    network_panel: NetworkPanel,
    /// Clipboard helper for ticket copying
    clipboard_helper: ClipboardHelper,
    /// Toast notification manager
    toast_manager: ToastManager,
    /// Show advanced config editor
    show_config_editor: bool,
    /// Config JSON text for editing
    config_json: String,
    /// Update checker instance
    update_checker: Option<()>,
    /// Current update notification
    update_notification: Option<()>,
    /// Update dialog for showing progress
    update_dialog: Option<()>,
    /// Last update check time
    last_update_check: Option<std::time::Instant>,
    /// Update check interval (12 hours by default)
    update_check_interval: std::time::Duration,
    /// Offline game instance
    offline_game: OfflineGoGame,
    /// Neural training UI
    neural_training_ui: NeuralTrainingUI,
    /// Neural overlay for gameplay
    neural_overlay: NeuralOverlay,
    /// Error logger
    error_logger: ErrorLogger,
    /// Error log viewer
    error_log_viewer: ErrorLogViewer,
    /// Show neural training UI
    show_neural_training: bool,
    /// Show error log viewer
    show_error_log: bool,
    /// Connection status widget
    connection_status: ConnectionStatusWidget,
    /// Network operation in progress
    network_operation: Option<String>,
    /// Track if we've shown the ghost moves error message
    ghost_moves_error_shown: bool,
    /// Heat map overlay for neural network visualization
    heat_map: HeatMapOverlay,
    /// Dual heat map for sword and shield networks
    dual_heat_map: DualHeatMap,
}

impl App {
    pub fn new(
        ui_tx: Sender<UiToNet>,
        ui_rx: Receiver<NetToUi>,
        board_size: u8,
        player_name: String,
    ) -> Self {
        // Request node ID on startup
        let _ = ui_tx.send(UiToNet::GetNodeId);

        // Initialize update checker with current version
        let update_checker = None;

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
            network_panel: NetworkPanel::new(),
            clipboard_helper: ClipboardHelper::new(),
            toast_manager: ToastManager::new(),
            show_config_editor: false,
            config_json: String::new(),
            update_checker: update_checker,
            update_notification: None,
            update_dialog: None,
            last_update_check: None,
            update_check_interval: std::time::Duration::from_secs(12 * 60 * 60), // 12 hours
            offline_game: OfflineGoGame::new(),
            neural_training_ui: NeuralTrainingUI::new(),
            neural_overlay: NeuralOverlay::new(),
            error_logger: ErrorLogger::new(),
            error_log_viewer: ErrorLogViewer::new(),
            show_neural_training: false,
            show_error_log: false,
            connection_status: ConnectionStatusWidget::new(),
            network_operation: None,
            ghost_moves_error_shown: false,
            heat_map: HeatMapOverlay::new(),
            dual_heat_map: DualHeatMap::new(),
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
            network_panel: NetworkPanel::new(),
            clipboard_helper: ClipboardHelper::new(),
            toast_manager: ToastManager::new(),
            show_config_editor: false,
            config_json: String::new(),
            update_checker: None,
            update_notification: None,
            update_dialog: None,
            last_update_check: None,
            update_check_interval: std::time::Duration::from_secs(12 * 60 * 60),
            offline_game: OfflineGoGame::new(),
            neural_training_ui: NeuralTrainingUI::new(),
            neural_overlay: NeuralOverlay::new(),
            error_logger: ErrorLogger::new(),
            error_log_viewer: ErrorLogViewer::new(),
            show_neural_training: false,
            show_error_log: false,
            connection_status: ConnectionStatusWidget::new(),
            network_operation: None,
            ghost_moves_error_shown: false,
            heat_map: HeatMapOverlay::new(),
            dual_heat_map: DualHeatMap::new(),
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
            board_widget: BoardWidget::new(19),
            show_overlay: false,
            last_blob_hash: None,
            rx_queue_length: 0,
            node_id: None,
            show_ticket_modal: false,
            current_ticket: None,
            ticket_input: String::new(),
            nat_report: None,
            default_board_size: 19,
            network_panel: NetworkPanel::new(),
            clipboard_helper: ClipboardHelper::new(),
            toast_manager: ToastManager::new(),
            show_config_editor: false,
            config_json: String::new(),
            update_checker: None,
            update_notification: None,
            update_dialog: None,
            last_update_check: None,
            update_check_interval: std::time::Duration::from_secs(12 * 60 * 60),
            offline_game: OfflineGoGame::new(),
            neural_training_ui: NeuralTrainingUI::new(),
            neural_overlay: NeuralOverlay::new(),
            error_logger: ErrorLogger::new(),
            error_log_viewer: ErrorLogViewer::new(),
            show_neural_training: false,
            show_error_log: false,
            connection_status: ConnectionStatusWidget::new(),
            network_operation: None,
            ghost_moves_error_shown: false,
            heat_map: HeatMapOverlay::new(),
            dual_heat_map: DualHeatMap::new(),
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
    }

    /// Get network panel reference for testing
    #[cfg(test)]
    pub fn network_panel(&self) -> &NetworkPanel {
        &self.network_panel
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

    /// Handle incoming messages from the network worker
    fn handle_network_messages(&mut self) {
        // Try to receive all pending messages without blocking
        while let Ok(msg) = self.ui_rx.try_recv() {
            #[cfg(feature = "headless")]
            println!("App received message: {:?}", msg);

            // Update connection status widget
            self.connection_status.update_from_message(&msg);

            match msg {
                NetToUi::GamesUpdated { games } => {
                    if let View::MainMenu {
                        available_games,
                        creating_game: _,
                        board_size: _,
                    } = &mut self.current_view
                    {
                        *available_games = games;
                    }
                }
                NetToUi::GameEvent { event } => {
                    match &event {
                        p2pgo_core::GameEvent::MoveMade { mv, .. } => {
                            #[cfg(feature = "headless")]
                            println!("Move made event received: {:?}", mv);

                            // Handle move events for both Game and Lobby views
                            match &mut self.current_view {
                                View::Game { game_state, .. } => {
                                    // Animate the move
                                    match mv {
                                        p2pgo_core::Move::Place { x, y, color } => {
                                            let coord = Coord { x: *x, y: *y };
                                            self.board_widget.animate_stone_placement(coord, *color);
                                        }
                                        _ => {} // Pass and Resign don't need animation
                                    }

                                    let _ = game_state.apply_move(mv.clone());
                                    // Update last blob hash for debug overlay - use move type description
                                    self.last_blob_hash = Some(format!("{:?}", mv));
                                }
                                View::Lobby { game_id } => {
                                    // Someone made a move! Transition from lobby to game
                                    let board_size = self.board_widget.get_board_size();
                                    let mut game_state = p2pgo_core::GameState::new(board_size);
                                    let _ = game_state.apply_move(mv.clone());
                                    
                                    self.current_view = View::Game {
                                        game_id: game_id.clone(),
                                        game_state,
                                        our_color: None, // Will be determined
                                    };
                                    
                                    // Animate the move
                                    match mv {
                                        p2pgo_core::Move::Place { x, y, color } => {
                                            let coord = Coord { x: *x, y: *y };
                                            self.board_widget.animate_stone_placement(coord, *color);
                                        }
                                        _ => {}
                                    }
                                    
                                    self.toast_manager.add_toast("Opponent connected! Game started.", crate::toast_manager::ToastType::Success);
                                }
                                _ => {
                                    // Ignore move events if not in game or lobby
                                }
                            }
                        }
                        p2pgo_core::GameEvent::GameFinished {
                            black_score,
                            white_score,
                        } => {
                            // Wait for ScoreCalculated message to transition to score dialog
                            // We'll just collect the scores here for now
                            tracing::info!(
                                "Game finished - black: {}, white: {}",
                                black_score,
                                white_score
                            );
                        }
                        _ => {
                            // Request ghost moves after the move is applied
                            let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                        }
                    }
                }
                NetToUi::GameJoined { game_id } => {
                    #[cfg(feature = "headless")]
                    println!("Game joined: {}, transitioning to Game view", game_id);

                    // Get board size from the board widget
                    let board_size = self.board_widget.get_board_size();
                    let game_state = p2pgo_core::GameState::new(board_size);

                    // Update network panel with game connection
                    self.network_panel
                        .update_game_connection(game_id.clone(), 1, true);

                    // Check if this is a game we created (we're the host) or joining an existing game
                    // If we have no moves yet, we're probably the creator, so go to lobby
                    // If the game has moves, we're joining an existing game, so go to game view
                    if game_state.moves.is_empty() {
                        // This is a new game we created - go to lobby to wait for opponent
                        self.current_view = View::Lobby { game_id };
                    } else {
                        // This is an existing game we're joining - go directly to game view
                        self.current_view = View::Game {
                            game_id,
                            game_state,
                            our_color: None, // Will be determined by first move
                        };
                    }

                    // Request initial ghost moves when joining a game if threshold met
                    if self.config.games_finished >= 5 {
                        let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                    }
                }
                NetToUi::GameLeft => {
                    // Remove game from network panel
                    if let View::Game { game_id, .. } = &self.current_view {
                        self.network_panel.remove_game_connection(game_id);
                    }
                    self.current_view = View::default();
                }
                NetToUi::Error { message } => {
                    // Check if this is the ghost moves error
                    if message.contains("Ghost moves will be available") {
                        // Only show this error once per session
                        if !self.ghost_moves_error_shown {
                            self.ghost_moves_error_shown = true;
                            self.error_logger.log(
                                crate::error_logger::ErrorLevel::Info,
                                "Network",
                                &message,
                            );
                            self.toast_manager.add_toast(&message, ToastType::Info);
                        }
                        // Don't set error_msg for ghost moves message
                    } else {
                        // For other errors, show as usual
                        self.error_logger.log(
                            crate::error_logger::ErrorLevel::Error,
                            "Network",
                            &message,
                        );
                        self.error_msg = Some(message);
                    }
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
                    if let View::Game {
                        game_id,
                        game_state,
                        ..
                    } = &self.current_view
                    {
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
                    if let View::ScoreDialog {
                        game_id,
                        game_state,
                        dead_stones,
                        ..
                    } = &self.current_view
                    {
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
                    self.error_msg = Some(format!(
                        "Score acceptance timed out for {}×{} game. Game will be discarded.",
                        board_size, board_size
                    ));
                    // Return to main menu
                    self.current_view = View::default();
                }
                NetToUi::Debug(message) => {
                    tracing::debug!("Debug message: {}", message);
                    self.rx_queue_length = self.ui_rx.len();
                }
                NetToUi::GameAdvertised {
                    game_id,
                    host_id,
                    board_size,
                } => {
                    tracing::info!(
                        "Game advertisement received: {} ({}×{}) from {}",
                        game_id,
                        board_size,
                        board_size,
                        host_id
                    );
                    // Request a refresh of the game list to show the advertised game
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                }
                NetToUi::NetRestarting { reason } => {
                    self.toast_manager.add_toast(
                        format!("Network restarting: {}", reason),
                        ToastType::Warning,
                    );
                    tracing::info!("Network restarting: {}", reason);
                }
                NetToUi::NetRestartCompleted => {
                    self.toast_manager
                        .add_toast("Network restarted successfully", ToastType::Success);
                    // Request refresh and ticket
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                    let _ = self.ui_tx.send(UiToNet::GetTicket);
                    let _ = self.ui_tx.send(UiToNet::GetNodeId);
                }
                NetToUi::RelayHealth {
                    status,
                    port,
                    is_relay_node,
                    last_restart,
                } => {
                    // Update network panel with health info
                    #[cfg(feature = "iroh")]
                    self.network_panel.update_relay_health(status.clone(), port);

                    // Update relay node flag separately
                    self.network_panel.set_is_relay_node(is_relay_node);

                    // Clone for continued use
                    let status = status.clone();

                    // Add toast notification for important status changes
                    match status {
                        p2pgo_network::relay_monitor::RelayHealthStatus::Healthy => {
                            if is_relay_node {
                                let port_info = if let Some(p) = port {
                                    format!(" on port {}", p)
                                } else {
                                    String::new()
                                };

                                self.toast_manager.add_toast(
                                    format!("Relay active{}", port_info),
                                    ToastType::Success,
                                );
                            }
                        }
                        p2pgo_network::relay_monitor::RelayHealthStatus::Restarting => {
                            let reason = if let Some(time) = last_restart {
                                // Format restart time nicely
                                let now = std::time::SystemTime::now();
                                if let Ok(duration) = now.duration_since(time) {
                                    if duration.as_secs() < 5 {
                                        "just now".to_string()
                                    } else {
                                        format!("{} sec ago", duration.as_secs())
                                    }
                                } else {
                                    "now".to_string()
                                }
                            } else {
                                "now".to_string()
                            };

                            self.toast_manager.add_toast(
                                format!("Relay restarting {}", reason),
                                ToastType::Warning,
                            );
                        }
                        p2pgo_network::relay_monitor::RelayHealthStatus::Failed => {
                            self.toast_manager
                                .add_toast("Relay service failed to start", ToastType::Error);
                        }
                        _ => {}
                    }
                }
                NetToUi::RelayCapacity {
                    current_connections,
                    max_connections,
                    ..
                } => {
                    // Update network panel with relay capacity info
                    self.network_panel
                        .update_relay_capacity(current_connections, max_connections);
                }
                NetToUi::GameRestored {
                    game_id,
                    move_count,
                } => {
                    // Show toast notification that game was restored
                    self.toast_manager.add_toast(
                        format!("Restored game {} with {} moves", game_id, move_count),
                        ToastType::Info,
                    );
                    tracing::info!(
                        "Game restored from snapshot: {} with {} moves",
                        game_id,
                        move_count
                    );
                }
                NetToUi::ConnectionTestResults { results } => {
                    // Update network panel with test results
                    self.network_panel.update_test_results(results);
                }
                NetToUi::TrainingProgress { progress } => {
                    // Update training progress in UI
                    self.toast_manager.add_toast(
                        format!("Training progress: {:.1}%", progress * 100.0),
                        ToastType::Info,
                    );
                }
                NetToUi::TrainingCompleted { stats } => {
                    // Training completed successfully
                    self.toast_manager.add_toast(
                        format!("Training completed! Games trained: {}", stats.games_trained),
                        ToastType::Success,
                    );
                }
                NetToUi::TrainingError { message } => {
                    // Training failed
                    self.error_logger.log(
                        crate::error_logger::ErrorLevel::Error,
                        "Training",
                        &message,
                    );
                    self.toast_manager
                        .add_toast(format!("Training failed: {}", message), ToastType::Error);
                }
                NetToUi::RelayModeChanged { mode } => {
                    self.network_panel.update_relay_mode(mode);
                    self.toast_manager.add_toast(
                        format!("Relay mode changed to: {:?}", mode),
                        ToastType::Info,
                    );
                }
                NetToUi::TrainingConsentStatus { enabled } => {
                    self.network_panel.update_training_consent(enabled);
                }
                NetToUi::RelayStatsUpdate { stats } => {
                    // Update relay stats in network panel
                    let credits = stats.calculate_credits();
                    self.network_panel.update_relay_credits(credits);
                }
                NetToUi::TrainingCreditsEarned { credits } => {
                    self.network_panel.update_relay_credits(credits);
                    self.toast_manager.add_toast(
                        format!("Earned {} training credits!", credits),
                        ToastType::Success,
                    );
                }
            }
        }
    }

    fn render_main_menu(&mut self, ui: &mut egui::Ui) {
        // Extract data we need before the closure
        let available_games_list = if let View::MainMenu {
            available_games, ..
        } = &self.current_view
        {
            available_games.clone()
        } else {
            vec![]
        };

        if let View::MainMenu { .. } = &mut self.current_view {
            // Title centered
            ui.vertical_centered(|ui| {
                ui.heading("P2P Go");
                ui.label(format!("Version: {}", VERSION));
                ui.add_space(10.0);
            });

            // Main content in two columns
            ui.columns(2, |columns| {
                // Left column: Game Menu with simple buttons
                columns[0].vertical_centered(|ui| {
                    ui.heading("Game Menu");
                    ui.add_space(20.0);

                    // Create Game button - prominent and always visible
                    let create_enabled = self.current_ticket.is_some();
                    let create_button = ui.add_sized(
                        egui::Vec2::new(200.0, 50.0),
                        egui::Button::new("Create Game").fill(if create_enabled {
                            egui::Color32::from_rgb(0, 150, 0)
                        } else {
                            egui::Color32::from_rgb(100, 100, 100)
                        }),
                    );

                    if create_button.clicked() && create_enabled {
                        let _ = self.ui_tx.send(UiToNet::CreateGame { board_size: 9 });
                    }

                    if !create_enabled {
                        ui.label("(Waiting for network...)");
                    }

                    ui.add_space(15.0);

                    // Join Game button
                    if ui
                        .add_sized(
                            egui::Vec2::new(200.0, 50.0),
                            egui::Button::new("Join Game")
                                .fill(egui::Color32::from_rgb(0, 100, 200)),
                        )
                        .clicked()
                    {
                        self.show_ticket_modal = true;
                    }

                    ui.add_space(15.0);

                    // Offline Game button
                    if ui
                        .add_sized(
                            egui::Vec2::new(200.0, 50.0),
                            egui::Button::new("Offline Game")
                                .fill(egui::Color32::from_rgb(100, 100, 200)),
                        )
                        .clicked()
                    {
                        self.current_view = View::OfflineGame;
                    }

                    ui.add_space(30.0);

                    // Settings button (currently no-op, but visible)
                    if ui
                        .add_sized(
                            egui::Vec2::new(200.0, 40.0),
                            egui::Button::new("Settings").fill(egui::Color32::from_rgb(80, 80, 80)),
                        )
                        .clicked()
                    {
                        // TODO: Implement settings view
                        self.toast_manager
                            .add_toast("Settings not yet implemented", ToastType::Info);
                    }
                });

                // Right column: Network status and available games
                columns[1].vertical(|ui| {
                    ui.heading("Network Status");

                    if let Some(node_id) = &self.node_id {
                        ui.horizontal(|ui| {
                            ui.label("Node ID:");
                            ui.label(format!("{:.8}...", node_id));
                            if ui.button("Copy").clicked() {
                                if let Err(e) = self
                                    .clipboard_helper
                                    .copy_ticket(node_id, &mut self.toast_manager)
                                {
                                    tracing::warn!("Failed to copy node ID: {}", e);
                                }
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Generate Ticket").clicked() {
                            let _ = self.ui_tx.send(UiToNet::GetTicket);
                        }

                        if ui.button("Refresh Games").clicked() {
                            let _ = self.ui_tx.send(UiToNet::RefreshGames);
                        }
                    });

                    if let Some(ticket) = &self.current_ticket {
                        let is_stub = ticket == "loopback-ticket";
                        let relay_status = ticket.len() > 50;

                        ui.horizontal(|ui| {
                            if is_stub {
                                ui.colored_label(
                                    egui::Color32::from_rgb(100, 100, 200),
                                    "Status: Local Mode",
                                );
                            } else if relay_status {
                                ui.colored_label(egui::Color32::GREEN, "Status: Network Ready");
                            } else {
                                // Show a spinner for initializing state
                                ui.spinner();
                                ui.label("Connecting to network...");
                            }
                        });
                    }

                    ui.separator();
                    ui.heading("🎮 Available Games");
                    ui.add_space(8.0);

                    if available_games_list.is_empty() {
                        ui.group(|ui| {
                            ui.vertical_centered(|ui| {
                                ui.label("🔍 No games found");
                                ui.add_space(4.0);
                                ui.label("• Create a game to start playing");
                                ui.label("• Or ask a friend to share their game ticket");
                                ui.add_space(8.0);
                                ui.label("💡 Games appear here when other players create them");
                            });
                        });
                    } else {
                        ui.label(format!("Found {} game(s):", available_games_list.len()));
                        ui.add_space(4.0);
                        
                        for game in &available_games_list {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    // Game info
                                    ui.vertical(|ui| {
                                        ui.label(format!("🎯 Game: {}", game.id));
                                        ui.label(format!("📐 Board: {}×{}", game.board_size, game.board_size));
                                    });
                                    
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("🚀 Join Game").clicked() {
                                            let _ = self.ui_tx.send(UiToNet::JoinGame {
                                                game_id: game.id.clone(),
                                            });
                                        }
                                    });
                                });
                            });
                            ui.add_space(4.0);
                        }
                    }
                });
            });
        }
    }

    fn render_lobby(&mut self, ui: &mut egui::Ui) {
        if let View::Lobby { game_id, .. } = &self.current_view {
            // Title and status
            ui.vertical_centered(|ui| {
                ui.heading("🎯 Game Created Successfully!");
                ui.add_space(8.0);
                ui.label("Now waiting for an opponent to join...");
                ui.add_space(16.0);
            });

            // Instructions panel
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.heading("📋 How to Invite a Friend:");
                    ui.add_space(8.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("1. ");
                        ui.label("Copy the Connection Ticket below");
                    });
                    ui.horizontal(|ui| {
                        ui.label("2. ");
                        ui.label("Send it to your friend (via text, email, chat)");
                    });
                    ui.horizontal(|ui| {
                        ui.label("3. ");
                        ui.label("Your friend clicks 'Join Game' and pastes the ticket");
                    });
                    ui.horizontal(|ui| {
                        ui.label("4. ");
                        ui.label("Game starts automatically when they connect!");
                    });
                });
            });

            ui.add_space(16.0);

            // Game ID section
            ui.group(|ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("🎮 Game ID:");
                        ui.label(game_id);
                    });
                    ui.add_space(4.0);
                    ui.label("(This is your game identifier)");
                });
            });

            ui.add_space(12.0);

            // Connection ticket section
            if let Some(ticket) = &self.current_ticket {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.heading("🎫 Connection Ticket (Share This!):");
                        ui.add_space(8.0);
                        
                        // Make the ticket more prominent and copyable
                        ui.horizontal(|ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                // Truncate very long tickets for display
                                let display_ticket = if ticket.len() > 100 {
                                    format!("{}...", &ticket[..100])
                                } else {
                                    ticket.clone()
                                };
                                
                                ui.add(
                                    egui::TextEdit::multiline(&mut display_ticket.clone())
                                        .desired_width(400.0)
                                        .desired_rows(3)
                                        .font(egui::TextStyle::Monospace)
                                );
                            });
                            
                            ui.vertical(|ui| {
                                if ui.button("📋 Copy Ticket").clicked() {
                                    if let Err(e) = self.clipboard_helper.copy_ticket(ticket, &mut self.toast_manager) {
                                        tracing::warn!("Failed to copy ticket: {}", e);
                                    } else {
                                        self.toast_manager.add_toast("Connection ticket copied! Share it with your friend.", crate::toast_manager::ToastType::Success);
                                    }
                                }
                                
                                if ui.button("🔄 New Ticket").clicked() {
                                    let _ = self.ui_tx.send(UiToNet::GetTicket);
                                }
                            });
                        });
                        
                        ui.add_space(4.0);
                        ui.label("💡 Your friend needs this entire ticket to connect");
                    });
                });
            } else {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating connection ticket...");
                    });
                });
            }

            ui.add_space(20.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("🔙 Back to Menu").clicked() {
                    let _ = self.ui_tx.send(UiToNet::LeaveGame);
                    self.current_view = View::default();
                }
                
                ui.add_space(20.0);
                
                if ui.button("🔄 Refresh").clicked() {
                    let _ = self.ui_tx.send(UiToNet::RefreshGames);
                    let _ = self.ui_tx.send(UiToNet::GetTicket);
                }
            });

            ui.add_space(20.0);

            // Status information
            ui.separator();
            ui.add_space(8.0);
            ui.label("ℹ️ The game will start automatically when your opponent connects.");
            ui.label("   Both players will see the game board and can begin playing.");
        }
    }

    fn render_game(&mut self, ui: &mut egui::Ui) {
        if let View::Game {
            game_id,
            game_state,
            ..
        } = &self.current_view
        {
            // Store the game ID in UI memory for the board widget to access
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new("current_game_id"), game_id.clone());
            });

            // Center the board with minimal UI chrome
            ui.vertical_centered(|ui| {
                // Compact status bar at top
                ui.horizontal(|ui| {
                    let current_player = match game_state.current_player {
                        Color::Black => "Black",
                        Color::White => "White",
                    };
                    ui.label(format!("Current player: {}", current_player));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Connection status on the right
                        self.connection_status.show(ui);
                    });
                });

                ui.add_space(8.0);

                // Board takes center stage
                if let Some(coord) = self.board_widget.render(ui, game_state, Some(&self.ui_tx)) {
                    let color = game_state.current_player;
                    let mv = Move::Place {
                        x: coord.x,
                        y: coord.y,
                        color,
                    };

                    // Optimistic UI: Show move as pending immediately
                    self.board_widget.show_pending_move(coord, color);

                    // Send move to network
                    let _ = self.ui_tx.send(UiToNet::MakeMove {
                        mv,
                        board_size: None,
                    });
                    // Only request ghost moves if we have completed enough games
                    if self.config.games_finished >= 5 {
                        let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                    }
                }

                // Render heat map overlays on the board
                if let (Some(board_rect), Some(cell_size)) = ui.ctx().data(|data| {
                    (
                        data.get_temp::<egui::Rect>(egui::Id::new("board_rect")),
                        data.get_temp::<f32>(egui::Id::new("board_cell_size")),
                    )
                }) {
                    // Single heat map overlay
                    if self.heat_map.is_enabled() {
                        self.heat_map
                            .render_overlay(ui, board_rect, cell_size, game_state);
                    }

                    // Dual heat map overlay (sword & shield)
                    if self.dual_heat_map.is_enabled() {
                        // TODO: Get actual predictions from neural networks
                        let sword_predictions = [[0.0; 19]; 19]; // Placeholder
                        let shield_predictions = [[0.0; 19]; 19]; // Placeholder
                        self.dual_heat_map.render_overlay(
                            ui,
                            board_rect,
                            cell_size,
                            game_state,
                            &sword_predictions,
                            &shield_predictions,
                        );
                    }
                }

                // Game controls at bottom
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Pass").clicked() {
                        let _ = self.ui_tx.send(UiToNet::MakeMove {
                            mv: Move::Pass,
                            board_size: None,
                        });
                        if self.config.games_finished >= 5 {
                            let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                        }
                    }
                    if ui.button("Resign").clicked() {
                        let _ = self.ui_tx.send(UiToNet::MakeMove {
                            mv: Move::Resign,
                            board_size: None,
                        });
                        if self.config.games_finished >= 5 {
                            let _ = self.ui_tx.send(UiToNet::GetGhostMoves);
                        }
                    }
                    ui.separator();
                    if ui.button("Leave Game").clicked() {
                        let _ = self.ui_tx.send(UiToNet::LeaveGame);
                        let _ = self.ui_tx.send(UiToNet::Shutdown);
                    }
                });
            });
        }
    }

    fn render_score_dialog(&mut self, ui: &mut egui::Ui) {
        if let View::ScoreDialog {
            game_id,
            game_state: _,
            score_proof,
            dead_stones: _,
            score_pending: _,
            score_accepted,
        } = &mut self.current_view.clone()
        {
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
            let winner = if final_score > 0 {
                "Black"
            } else if final_score < 0 {
                "White"
            } else {
                "Draw"
            };
            ui.heading(format!("Winner: {} (by {})", winner, final_score.abs()));
            ui.separator();

            if !*score_accepted {
                if ui.button("Accept Result").clicked() {
                    // Send AcceptScore message to worker
                    let _ = self.ui_tx.send(UiToNet::AcceptScore {
                        score_proof: score_proof.clone(),
                    });

                    // Update UI state directly without creating a new View
                    if let View::ScoreDialog {
                        score_accepted: ref mut acc,
                        score_pending: ref mut pend,
                        ..
                    } = self.current_view
                    {
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

    fn render_offline_game(&mut self, ui: &mut egui::Ui) {
        // Add a back button at the top
        if ui.button("← Back to Menu").clicked() {
            self.current_view = View::default();
        }

        ui.separator();

        // Render the offline game
        self.offline_game.ui(ui.ctx());
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
                    View::OfflineGame => "OfflineGame",
                };
                ui.label(format!("View: {}", current_view_name));

                if let View::Lobby { game_id, .. } | View::Game { game_id, .. } = &self.current_view
                {
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

    /// Check for updates if enough time has passed since last check
    fn check_for_updates_if_needed(&mut self) {
        // Disabled for now
        return;

        // Alternative: Check from URL (commented out for now)
        /*
        let update_url = "https://example.com/p2pgo/update_manifest.json";
        let checker_clone = checker.clone();
        tokio::spawn(async move {
            match checker_clone.check_url(update_url).await {
                Ok(result) => {
                    // Send result back to UI thread
                }
                Err(e) => {
                    tracing::error!("Failed to check for updates: {}", e);
                }
            }
        });
        */
    }

    /// Start the update process
    fn start_update_process(&mut self) {
        // Disabled for now
        return;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply the dark theme for better contrast
        crate::dark_theme::apply_dark_theme(ctx);
        crate::dark_theme::apply_compact_spacing(ctx);

        self.handle_network_messages();

        // Update network panel
        self.network_panel.update_ui(ctx);

        // Update toast notifications
        self.toast_manager.update(ctx);

        // Check for updates periodically
        // self.check_for_updates_if_needed();

        // Show update notification if available
        // if let Some(ref mut notification) = self.update_notification {
        //     match notification.show(ctx) {
        //         UpdateAction::UpdateNow => {
        //             // Start update process
        //             self.start_update_process();
        //         }
        //         UpdateAction::RemindLater => {
        //             // Will check again after interval
        //             self.last_update_check = Some(std::time::Instant::now());
        //         }
        //         UpdateAction::SkipVersion => {
        //             // TODO: Store skipped version in preferences
        //             self.update_notification = None;
        //         }
        //         UpdateAction::None => {}
        //     }
        // }

        // Show update dialog if in progress
        // if let Some(ref mut dialog) = self.update_dialog {
        //     if !dialog.show(ctx) {
        //         self.update_dialog = None;
        //     }
        // }

        // Handle F1 key to toggle debug overlay
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_overlay = !self.show_overlay;
            tracing::debug!("Debug overlay toggled: {}", self.show_overlay);
        }

        // Top menu bar with network status
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("P2P Go");
                ui.separator();

                // Show connection status widget
                self.connection_status.show(ui);
                ui.separator();

                // Add menu buttons
                if ui.button("🧠 Neural Training").clicked() {
                    self.show_neural_training = !self.show_neural_training;
                }
                if ui.button("📋 Error Log").clicked() {
                    self.show_error_log = !self.show_error_log;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(node_id) = &self.node_id {
                        ui.label(format!("Node: {:.8}...", node_id));
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match &self.current_view {
                View::MainMenu { .. } => self.render_main_menu(ui),
                View::Lobby { .. } => self.render_lobby(ui),
                View::Game { .. } => self.render_game(ui),
                View::ScoreDialog { .. } => self.render_score_dialog(ui),
                View::OfflineGame => self.render_offline_game(ui),
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
                egui::Window::new("🎫 Join Friend's Game")
                    .collapsible(false)
                    .resizable(false)
                    .default_width(500.0)
                    .show(ctx, |ui| {
                        ui.add_space(8.0);
                        
                        // Instructions
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.heading("📋 How to Join:");
                                ui.add_space(4.0);
                                ui.label("1. Ask your friend to create a game");
                                ui.label("2. Get the connection ticket from your friend");
                                ui.label("3. Paste the ticket below and click Connect");
                                ui.label("4. You'll join their game automatically!");
                            });
                        });

                        ui.add_space(12.0);

                        // Ticket input with better labeling
                        ui.vertical(|ui| {
                            ui.heading("🎫 Connection Ticket:");
                            ui.add_space(4.0);
                            ui.label("Paste the ticket your friend shared with you:");
                            
                            ui.add(
                                egui::TextEdit::multiline(&mut self.ticket_input)
                                    .desired_width(ui.available_width())
                                    .desired_rows(4)
                                    .hint_text("Paste the long connection ticket string here...")
                                    .font(egui::TextStyle::Monospace)
                            );
                        });

                        ui.add_space(12.0);
                        
                        // Action buttons
                        ui.horizontal(|ui| {
                            let connect_btn = ui.add_enabled(
                                !self.ticket_input.trim().is_empty(),
                                egui::Button::new("🚀 Connect to Game")
                                    .fill(egui::Color32::from_rgb(0, 150, 0))
                            );

                            if connect_btn.clicked() {
                                let _ = self.ui_tx.send(UiToNet::ConnectByTicket {
                                    ticket: self.ticket_input.clone(),
                                });
                                self.ticket_input.clear();
                                self.show_ticket_modal = false;
                                self.toast_manager.add_toast("Connecting to game...", crate::toast_manager::ToastType::Info);
                            }
                            
                            ui.add_space(20.0);
                            
                            if ui.button("❌ Cancel").clicked() {
                                self.ticket_input.clear();
                                self.show_ticket_modal = false;
                            }
                        });
                        
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.label("💡 Tip: Connection tickets are long strings that look like random text");
                    });
            }
        });

        // Show network diagnostics panel
        if let Some(action) = self.network_panel.show(ctx) {
            let _ = self.ui_tx.send(action);
        }

        // Show neural training UI window
        if self.show_neural_training {
            egui::Window::new("Neural Network Training")
                .open(&mut self.show_neural_training)
                .default_width(800.0)
                .default_height(600.0)
                .show(ctx, |ui| {
                    if let Some(training_data) = self.neural_training_ui.render(ui) {
                        // Handle training data creation
                        self.toast_manager
                            .add_toast("Training data created successfully", ToastType::Success);
                        // Log the event
                        self.error_logger.log(
                            crate::error_logger::ErrorLevel::Info,
                            "NeuralTraining",
                            "Created training data",
                        );
                    }
                });
        }

        // Show error log viewer window
        if self.show_error_log {
            egui::Window::new("Error Log")
                .open(&mut self.show_error_log)
                .default_width(600.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    self.error_log_viewer.render(ui, &self.error_logger);
                });
        }

        // Show heat map controls window if in game
        if let View::Game { .. } = &self.current_view {
            egui::Window::new("Heat Map Controls")
                .default_pos(egui::Pos2::new(10.0, 100.0))
                .default_width(250.0)
                .show(ctx, |ui| {
                    // Single heat map controls
                    ui.heading("Single Network Heat Map");
                    self.heat_map.render_controls(ui);

                    ui.separator();

                    // Dual heat map controls
                    ui.heading("Dual Network Heat Map");
                    self.dual_heat_map.render_controls(ui);

                    ui.separator();

                    // Keyboard shortcuts
                    ui.label("Shortcuts:");
                    ui.label("H - Toggle single heat map");
                    ui.label("D - Toggle dual heat map");
                    ui.label("S - Toggle sword network");
                    ui.label("Shift+S - Toggle shield network");
                });
        }

        // Handle keyboard shortcuts for heat maps
        if ctx.input(|i| i.key_pressed(egui::Key::H)) {
            self.heat_map.toggle();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.dual_heat_map.toggle_shield();
            } else {
                self.dual_heat_map.toggle_sword();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.dual_heat_map.toggle_shield();
            } else {
                self.dual_heat_map.toggle_sword();
            }
        }

        // Update neural overlay if in game
        if let View::Game { game_state, .. } = &self.current_view {
            self.neural_overlay
                .update(game_state, ctx.input(|i| i.stable_dt));
        }

        // Render neural overlay controls in side panel during game
        if matches!(self.current_view, View::Game { .. }) {
            egui::SidePanel::right("neural_controls")
                .resizable(true)
                .default_width(200.0)
                .show(ctx, |ui| {
                    self.neural_overlay.render_controls(ui);

                    ui.separator();

                    // Show win probability
                    if let View::Game { game_state, .. } = &self.current_view {
                        self.neural_overlay.render_win_probability(ui, game_state);
                    }
                });
        }

        // Render debug overlay on top
        self.render_debug_overlay(ctx);

        // Optimize repaint strategy based on focus and network activity
        if ctx.input(|i| !i.focused) && self.ui_rx.is_empty() {
            // Window is unfocused and no network messages - reduce repaint frequency
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
        } else {
            // Either focused or has network messages - continue with normal repaint
            ctx.request_repaint();
        }
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
