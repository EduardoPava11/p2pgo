//! Offline 9x9 Go Game Implementation
//!
//! This module provides a complete offline Go game for testing with:
//! - 9x9 board with square window layout
//! - Click-to-toggle territory marking (black/white/null)
//! - Full UI customization via UiConfig
//! - WASM tensor parameter integration

use crate::go3d_wireframe::Go3DWireframe;
use crate::ui_config::{create_font_id, styled_button, TerritoryMarkerType, UiConfig};
use egui::{Color32, Context, CursorIcon, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use p2pgo_core::{sgf::SgfProcessor, Color, GameState, Move};
use p2pgo_network::guilds::{DistanceFeatures, Guild, GuildClassifier, StoneVector};
use serde_json;
use std::collections::{HashMap, HashSet};

/// Game mode selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameMode {
    Traditional2D,
    ThreePlanes3D,
}

/// Territory marking state for each intersection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerritoryMark {
    /// No territory marking
    None,
    /// Black territory
    Black,
    /// White territory
    White,
}

/// Consensus phase state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsensusPhase {
    /// Not in consensus phase
    None,
    /// Both players marking territory simultaneously (OGS style)
    BothMarking,
    /// Both players have marked, waiting for agreement
    WaitingAgreement,
    /// Players agreed on territory
    Agreed,
}

/// Detailed score breakdown
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub black_stones: u32,
    pub white_stones: u32,
    pub black_territory: u32,
    pub white_territory: u32,
    pub black_captures: u16,
    pub white_captures: u16,
    pub komi: f32,
}

impl ScoreBreakdown {
    /// Get total black score
    pub fn black_total(&self) -> f32 {
        self.black_stones as f32 + self.black_territory as f32 + self.black_captures as f32
    }

    /// Get total white score
    pub fn white_total(&self) -> f32 {
        self.white_stones as f32
            + self.white_territory as f32
            + self.white_captures as f32
            + self.komi
    }
}

impl TerritoryMark {
    /// Cycle to next marking state
    pub fn cycle(&self) -> Self {
        match self {
            TerritoryMark::None => TerritoryMark::Black,
            TerritoryMark::Black => TerritoryMark::White,
            TerritoryMark::White => TerritoryMark::None,
        }
    }
}

/// Offline Go game state with UI
pub struct OfflineGoGame {
    /// Core game state
    game_state: GameState,
    /// UI configuration
    ui_config: UiConfig,
    /// Territory markings (for scoring phase)
    territory_marks: HashMap<(u8, u8), TerritoryMark>,
    /// Whether we're in territory marking mode
    marking_territory: bool,
    /// Last move for highlighting
    last_move: Option<(u8, u8)>,
    /// Error message to display
    error_message: Option<String>,
    /// WASM tensor values for UI adjustment
    tensor_params: Vec<f32>,
    /// Guild classifier for analyzing play style
    guild_classifier: GuildClassifier,
    /// Move history with guild classifications
    guild_history: Vec<(StoneVector, Guild)>,
    /// Current player guild affinity
    player_guild: Option<Guild>,
    /// Current game mode
    game_mode: GameMode,
    /// 3D game instance
    go3d_game: Go3DWireframe,
    /// Toggle for showing guild statistics
    show_guild_stats: bool,
    /// Dead stone groups marked by user
    dead_stones: HashMap<(u8, u8), bool>,
    /// Consensus phase state
    consensus_phase: ConsensusPhase,
    /// WASM engine handle (would be actual WASM instance in production)
    wasm_engine: Option<()>, // Placeholder for WASM engine
    /// SGF replay mode
    sgf_replay_mode: bool,
    /// Current move index in SGF replay
    sgf_current_move: usize,
    /// Total moves in SGF game
    sgf_total_moves: usize,
    /// Original SGF game state for replay
    sgf_original_state: Option<GameState>,
}

impl OfflineGoGame {
    /// Create a new offline game
    pub fn new() -> Self {
        let mut ui_config = UiConfig::default();

        // Ensure square window for 9x9 board
        ui_config.window.initial_size = (900.0, 900.0);
        ui_config.board.size = 800.0;

        Self {
            game_state: GameState::new(9), // 9x9 board
            ui_config,
            territory_marks: HashMap::new(),
            marking_territory: false,
            last_move: None,
            error_message: None,
            tensor_params: vec![0.5, 0.5, 0.5], // Default tensor values
            guild_classifier: GuildClassifier {
                layer_weights: HashMap::new(),
                distance_features: DistanceFeatures {
                    from_last_stone: 0.0,
                    from_last_capture: None,
                    from_center: 0.0,
                    from_nearest_friend: 0.0,
                    from_nearest_enemy: 0.0,
                },
                pattern_affinity: HashMap::new(),
            },
            guild_history: Vec::new(),
            player_guild: None,
            game_mode: GameMode::Traditional2D,
            go3d_game: Go3DWireframe::new(),
            show_guild_stats: false,
            dead_stones: HashMap::new(),
            consensus_phase: ConsensusPhase::None,
            wasm_engine: None, // Would load WASM engine here
            sgf_replay_mode: false,
            sgf_current_move: 0,
            sgf_total_moves: 0,
            sgf_original_state: None,
        }
    }

    /// Load UI configuration from file
    pub fn load_config(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.ui_config = UiConfig::load_from_file(path)?;
        Ok(())
    }

    /// Set WASM tensor parameters
    pub fn set_tensor_params(&mut self, params: Vec<f32>) {
        self.tensor_params = params;
        self.ui_config.apply_tensor_params(&self.tensor_params);
    }

    /// Load an SGF file for replay
    pub fn load_sgf(&mut self, sgf_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut processor = SgfProcessor::new(GameState::new(9));
        let loaded_state = processor.parse(sgf_content)?;

        // Store the original game state
        self.sgf_original_state = Some(loaded_state.clone());

        // Count total moves
        self.sgf_total_moves = loaded_state.moves.len();
        self.sgf_current_move = 0;

        // Reset game to start of SGF
        self.game_state = GameState::new(9);
        self.sgf_replay_mode = true;

        // Clear all UI state
        self.territory_marks.clear();
        self.marking_territory = false;
        self.last_move = None;
        self.error_message = None;
        self.dead_stones.clear();
        self.consensus_phase = ConsensusPhase::None;
        self.guild_history.clear();
        self.player_guild = None;

        Ok(())
    }

    /// Step forward in SGF replay
    pub fn sgf_step_forward(&mut self) {
        if let Some(original) = &self.sgf_original_state {
            if self.sgf_current_move < self.sgf_total_moves {
                // Apply the next move
                if let Some(mv) = original.moves.get(self.sgf_current_move) {
                    if let Ok(events) = self.game_state.apply_move(mv.clone()) {
                        self.sgf_current_move += 1;

                        // Update last move for highlighting
                        if let Move::Place { x, y, .. } = mv {
                            self.last_move = Some((*x, *y));

                            // Track guild history during replay
                            if self.sgf_current_move > 1 {
                                if let Some(prev_move) =
                                    original.moves.get(self.sgf_current_move - 2)
                                {
                                    if let Move::Place {
                                        x: prev_x,
                                        y: prev_y,
                                        ..
                                    } = prev_move
                                    {
                                        let vector = StoneVector {
                                            from: (*prev_x, *prev_y),
                                            to: (*x, *y),
                                            from_capture: false,
                                        };
                                        let features = self.calculate_distance_features(*x, *y);
                                        let guild =
                                            self.guild_classifier.classify_move(&vector, &features);
                                        self.guild_history.push((vector, guild));
                                    }
                                }
                            }
                        }

                        // Process game events
                        for event in events {
                            match event {
                                p2pgo_core::GameEvent::StonesCaptured {
                                    count,
                                    positions: _,
                                    player,
                                } => {
                                    eprintln!(
                                        "Captured {} {} stones",
                                        count,
                                        if player == Color::Black {
                                            "black"
                                        } else {
                                            "white"
                                        }
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    /// Step backward in SGF replay
    pub fn sgf_step_backward(&mut self) {
        if self.sgf_current_move > 0 {
            // Reset to beginning and replay up to previous move
            self.game_state = GameState::new(9);
            self.sgf_current_move = 0;
            self.last_move = None;
            self.guild_history.clear();

            if let Some(original) = &self.sgf_original_state {
                let target_move = self.sgf_current_move - 1;

                // Replay moves up to target
                for i in 0..target_move {
                    if let Some(mv) = original.moves.get(i) {
                        if self.game_state.apply_move(mv.clone()).is_ok() {
                            self.sgf_current_move = i + 1;

                            if let Move::Place { x, y, .. } = mv {
                                self.last_move = Some((*x, *y));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Jump to end of SGF game
    pub fn sgf_jump_to_end(&mut self) {
        while self.sgf_current_move < self.sgf_total_moves {
            self.sgf_step_forward();
        }

        // Calculate final guild affinity
        if self.guild_history.len() >= 3 {
            self.calculate_final_guild_affinity();
        }
    }

    /// Exit SGF replay mode
    pub fn exit_sgf_replay(&mut self) {
        self.sgf_replay_mode = false;
        self.sgf_original_state = None;
        self.sgf_current_move = 0;
        self.sgf_total_moves = 0;

        // Start a new game
        self.game_state = GameState::new(9);
        self.territory_marks.clear();
        self.marking_territory = false;
        self.last_move = None;
        self.error_message = None;
        self.dead_stones.clear();
        self.consensus_phase = ConsensusPhase::None;
        self.guild_history.clear();
        self.player_guild = None;
    }

    /// Toggle territory marking mode
    pub fn toggle_territory_mode(&mut self) {
        if !self.marking_territory && self.game_state.is_game_over() {
            // Enter consensus phase
            self.marking_territory = true;
            self.consensus_phase = ConsensusPhase::BothMarking;
        } else if self.marking_territory {
            // Progress through consensus phases
            match self.consensus_phase {
                ConsensusPhase::BothMarking => {
                    // Both players have finished marking - check for agreement
                    self.consensus_phase = ConsensusPhase::WaitingAgreement;
                    // In a real implementation, this would compare both players' marks
                    // For now, we'll accept the current marks
                }
                ConsensusPhase::WaitingAgreement => {
                    // Accept current marks
                    self.consensus_phase = ConsensusPhase::Agreed;
                    self.marking_territory = false;
                    self.generate_training_data();
                }
                ConsensusPhase::Agreed => {
                    // Reset everything
                    self.marking_territory = false;
                    self.consensus_phase = ConsensusPhase::None;
                }
                _ => {}
            }
        }
    }

    /// Get the current score including territory with detailed breakdown
    pub fn calculate_score(&self) -> (f32, f32) {
        let mut black_score = self.game_state.captures.1 as f32; // White stones captured
        let mut white_score = self.game_state.captures.0 as f32 + 6.5; // Black stones captured + komi

        // Count stones on board (excluding dead stones)
        for y in 0..9 {
            for x in 0..9 {
                let idx = y * 9 + x;
                let pos = (x as u8, y as u8);

                if let Some(color) = self.game_state.board[idx] {
                    if !self.dead_stones.contains_key(&pos) {
                        // Living stone
                        match color {
                            Color::Black => black_score += 1.0,
                            Color::White => white_score += 1.0,
                        }
                    } else {
                        // Dead stone counts as capture for opponent
                        match color {
                            Color::Black => white_score += 1.0,
                            Color::White => black_score += 1.0,
                        }
                    }
                }
            }
        }

        // Count territory marks
        for mark in self.territory_marks.values() {
            match mark {
                TerritoryMark::Black => black_score += 1.0,
                TerritoryMark::White => white_score += 1.0,
                TerritoryMark::None => {}
            }
        }

        (black_score, white_score)
    }

    /// Get detailed score breakdown
    pub fn get_score_breakdown(&self) -> ScoreBreakdown {
        let mut black_stones = 0;
        let mut white_stones = 0;
        let mut black_territory = 0;
        let mut white_territory = 0;
        let mut dead_black_stones = 0;
        let mut dead_white_stones = 0;

        // Count stones on board
        for y in 0..9 {
            for x in 0..9 {
                let idx = y * 9 + x;
                let pos = (x as u8, y as u8);

                if let Some(color) = self.game_state.board[idx] {
                    if !self.dead_stones.contains_key(&pos) {
                        // Living stone
                        match color {
                            Color::Black => black_stones += 1,
                            Color::White => white_stones += 1,
                        }
                    } else {
                        // Dead stone
                        match color {
                            Color::Black => dead_black_stones += 1,
                            Color::White => dead_white_stones += 1,
                        }
                    }
                }
            }
        }

        // Count territory marks
        for mark in self.territory_marks.values() {
            match mark {
                TerritoryMark::Black => black_territory += 1,
                TerritoryMark::White => white_territory += 1,
                TerritoryMark::None => {}
            }
        }

        ScoreBreakdown {
            black_stones,
            white_stones,
            black_territory,
            white_territory,
            black_captures: self.game_state.captures.1 + dead_white_stones as u16, // White stones captured by black
            white_captures: self.game_state.captures.0 + dead_black_stones as u16, // Black stones captured by white
            komi: 6.5,
        }
    }

    /// Handle board click
    fn handle_board_click(&mut self, x: u8, y: u8) {
        // Don't allow clicks during SGF replay
        if self.sgf_replay_mode {
            return;
        }

        if self.marking_territory {
            let key = (x, y);
            let idx = (y as usize) * 9 + (x as usize);

            // Check if clicking on a stone to mark as dead
            if let Some(color) = self.game_state.board[idx] {
                // Mark entire group as dead
                self.mark_group_as_dead(x, y, color);
            } else {
                // Territory marking on empty intersection
                let current = self
                    .territory_marks
                    .get(&key)
                    .copied()
                    .unwrap_or(TerritoryMark::None);

                // Flood fill territory marking
                if current == TerritoryMark::None {
                    // Determine which color to fill with based on surrounding stones
                    let fill_color = self.determine_territory_color(x, y);
                    self.flood_fill_territory(x, y, fill_color);
                } else {
                    // Clear the territory region
                    self.clear_territory_region(x, y);
                }
            }
        } else if !self.game_state.is_game_over() {
            // Normal move placement
            let mv = Move::Place {
                x,
                y,
                color: self.game_state.current_player,
            };

            match self.game_state.apply_move(mv) {
                Ok(events) => {
                    // Process game events (captures, etc)
                    for event in events {
                        match event {
                            p2pgo_core::GameEvent::StonesCaptured {
                                count,
                                positions: _,
                                player,
                            } => {
                                // Visual feedback for captures could be added here
                                eprintln!(
                                    "Captured {} {} stones",
                                    count,
                                    if player == p2pgo_core::Color::Black {
                                        "black"
                                    } else {
                                        "white"
                                    }
                                );
                            }
                            _ => {}
                        }
                    }
                    // Store move data for end-game guild calculation
                    if let Some((prev_x, prev_y)) = self.last_move {
                        let vector = StoneVector {
                            from: (prev_x, prev_y),
                            to: (x, y),
                            from_capture: false, // TODO: Track captures
                        };

                        // Calculate distance features
                        let features = self.calculate_distance_features(x, y);
                        let guild = self.guild_classifier.classify_move(&vector, &features);

                        self.guild_history.push((vector, guild));
                    }

                    self.last_move = Some((x, y));
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Invalid move: {}", e));
                }
            }
        }
    }

    /// Render detailed score breakdown
    fn render_score_breakdown(&self, ui: &mut Ui) {
        let breakdown = self.get_score_breakdown();

        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.heading("Score Breakdown");

                // Black score components
                ui.horizontal(|ui| {
                    ui.label("Black:");
                    ui.add_space(10.0);
                    ui.monospace(format!("{} stones", breakdown.black_stones));
                    if breakdown.black_captures > 0 {
                        ui.label("+");
                        ui.monospace(format!("{} captures", breakdown.black_captures));
                    }
                    if breakdown.black_territory > 0 {
                        ui.label("+");
                        ui.monospace(format!("{} territory", breakdown.black_territory));
                    }
                    ui.label("=");
                    ui.strong(format!("{:.1}", breakdown.black_total()));
                });

                // White score components
                ui.horizontal(|ui| {
                    ui.label("White:");
                    ui.add_space(10.0);
                    ui.monospace(format!("{} stones", breakdown.white_stones));
                    if breakdown.white_captures > 0 {
                        ui.label("+");
                        ui.monospace(format!("{} captures", breakdown.white_captures));
                    }
                    if breakdown.white_territory > 0 {
                        ui.label("+");
                        ui.monospace(format!("{} territory", breakdown.white_territory));
                    }
                    ui.label("+");
                    ui.monospace(format!("{} komi", breakdown.komi));
                    ui.label("=");
                    ui.strong(format!("{:.1}", breakdown.white_total()));
                });

                // Final result
                ui.separator();
                let black_total = breakdown.black_total();
                let white_total = breakdown.white_total();
                let diff = (black_total - white_total).abs();
                let winner = if black_total > white_total {
                    "Black"
                } else {
                    "White"
                };

                ui.horizontal(|ui| {
                    ui.strong("Result:");
                    ui.colored_label(
                        Color32::from(self.ui_config.colors.success),
                        format!("{} wins by {:.1} points", winner, diff),
                    );
                });
            });
        });
    }

    /// Render the game UI
    pub fn ui(&mut self, ctx: &Context) {
        // Menu bar for mode selection
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Game Mode:");
                if ui
                    .selectable_label(self.game_mode == GameMode::Traditional2D, "2D (9×9)")
                    .clicked()
                {
                    self.game_mode = GameMode::Traditional2D;
                }
                ui.separator();
                if ui
                    .selectable_label(
                        self.game_mode == GameMode::ThreePlanes3D,
                        "3D (Three Planes)",
                    )
                    .clicked()
                {
                    self.game_mode = GameMode::ThreePlanes3D;
                }
            });
        });

        // Render the appropriate game
        match self.game_mode {
            GameMode::Traditional2D => self.render_2d_game(ctx),
            GameMode::ThreePlanes3D => self.go3d_game.ui(ctx),
        }
    }

    /// Render the traditional 2D game
    fn render_2d_game(&mut self, ctx: &Context) {
        // Clean light background like OGS
        let ui_color = Color32::from_rgb(245, 245, 245);

        egui::CentralPanel::default()
            .frame(egui::Frame::default()
                .fill(ui_color)
                .inner_margin(egui::Margin::same(20.0)))
            .show(ctx, |ui| {
                // Use fixed layout to prevent board resizing
                ui.vertical_centered(|ui| {
                    // Minimal title
                    ui.add_space(10.0);
                    ui.heading("9×9 Go");
                    ui.add_space(10.0);

                    // Game status and info
                    ui.horizontal(|ui| {
                        if self.game_state.is_game_over() {
                            ui.strong("Game Over");
                            ui.separator();
                            // Determine winner by score
                            let (black_score, white_score) = self.calculate_score();
                            if black_score > white_score {
                                ui.label("● Black wins");
                            } else if white_score > black_score {
                                ui.label("○ White wins");
                            } else {
                                ui.label("Draw");
                            }
                        } else {
                            ui.label("Current:");
                            let (response, painter) = ui.allocate_painter(Vec2::splat(20.0), Sense::click());
                            let stone_color = if self.game_state.current_player == Color::Black {
                                Color32::from_gray(20)
                            } else {
                                Color32::from_gray(235)
                            };
                            painter.circle_filled(response.rect.center(), 8.0, stone_color);
                        }

                        ui.add_space(20.0);

                        // Captures
                        let (black_captures, white_captures) = self.game_state.captures;
                        ui.label(format!("Captures: ● {} ○ {}", white_captures, black_captures));
                    });

                    // Detailed score breakdown when game is over or marking territory
                    if self.game_state.is_game_over() || self.marking_territory {
                        ui.separator();
                        self.render_score_breakdown(ui);
                    }

                    // Guild affinity display - only show at game end
                    if self.game_state.is_game_over() {
                        // Calculate final guild affinity if not already done
                        if self.player_guild.is_none() && self.guild_history.len() >= 3 {
                            self.calculate_final_guild_affinity();
                        }

                        if let Some(guild) = self.player_guild {
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Play Style Guild:");
                                let guild_color = match guild {
                                    Guild::Activity => Color32::from_rgb(220, 100, 100),
                                    Guild::Reactivity => Color32::from_rgb(100, 100, 220),
                                    Guild::Avoidance => Color32::from_rgb(100, 220, 100),
                                };
                                ui.colored_label(guild_color, format!("{:?}", guild));

                                // Toggle button for detailed stats
                                if ui.small_button(if self.show_guild_stats { "Hide Stats" } else { "Show Stats" }).clicked() {
                                    self.show_guild_stats = !self.show_guild_stats;
                                }
                            });

                            // Show guild statistics as bar graph if toggled
                            if self.show_guild_stats {
                                self.render_guild_bar_graph(ui);
                            }
                        }
                    }

                    // Error message
                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::from(self.ui_config.colors.error), error);
                    }

                    // Board with proper spacing
                    ui.add_space(15.0);
                    ui.group(|ui| {
                        self.draw_board(ui);
                    });
                    ui.add_space(15.0);

                    // Control buttons with better spacing
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            if self.sgf_replay_mode {
                                // SGF replay controls
                                if styled_button(ui, &self.ui_config.button, "◀◀").clicked() {
                                    self.sgf_current_move = 0;
                                    self.game_state = GameState::new(9);
                                    self.last_move = None;
                                    self.guild_history.clear();
                                }

                                ui.add_space(5.0);

                                if styled_button(ui, &self.ui_config.button, "◀").clicked() {
                                    self.sgf_step_backward();
                                }

                                ui.add_space(5.0);

                                ui.label(format!("{}/{}", self.sgf_current_move, self.sgf_total_moves));

                                ui.add_space(5.0);

                                if styled_button(ui, &self.ui_config.button, "▶").clicked() {
                                    self.sgf_step_forward();
                                }

                                ui.add_space(5.0);

                                if styled_button(ui, &self.ui_config.button, "▶▶").clicked() {
                                    self.sgf_jump_to_end();
                                }

                                ui.add_space(20.0);
                                ui.separator();
                                ui.add_space(20.0);

                                if styled_button(ui, &self.ui_config.button, "Exit Replay").clicked() {
                                    self.exit_sgf_replay();
                                }
                            } else {
                                // Game control buttons
                                if !self.game_state.is_game_over() {
                                    if styled_button(ui, &self.ui_config.button, "Pass").clicked() {
                                        self.game_state.apply_move(Move::Pass).ok();
                                    }

                                    ui.add_space(10.0);

                                    if styled_button(ui, &self.ui_config.button, "Resign").clicked() {
                                        self.game_state.apply_move(Move::Resign).ok();
                                    }
                                }
                            }

                            ui.add_space(20.0);
                            ui.separator();
                            ui.add_space(20.0);

                            // Territory marking button - only show after game ends
                            if self.game_state.is_game_over() && !self.sgf_replay_mode {
                                let territory_text = match self.consensus_phase {
                                    ConsensusPhase::None => "Mark Territory",
                                    ConsensusPhase::BothMarking => "Done Marking ✓",
                                    ConsensusPhase::WaitingAgreement => "Accept Territory",
                                    ConsensusPhase::Agreed => "Territory Agreed ✓",
                                };

                                if styled_button(ui, &self.ui_config.button, territory_text).clicked() {
                                    self.toggle_territory_mode();
                                }
                            }

                            ui.add_space(10.0);

                            if styled_button(ui, &self.ui_config.button, "New Game").clicked() {
                                self.game_state = GameState::new(9);
                                self.territory_marks.clear();
                                self.marking_territory = false;
                                self.last_move = None;
                                self.error_message = None;
                                self.guild_history.clear();
                                self.player_guild = None;
                                self.show_guild_stats = false;
                                self.dead_stones.clear();
                                self.consensus_phase = ConsensusPhase::None;
                                self.sgf_replay_mode = false;
                                self.sgf_original_state = None;
                                self.sgf_current_move = 0;
                                self.sgf_total_moves = 0;
                            }

                            ui.add_space(10.0);

                            // Load SGF button (temporary - normally would use file dialog)
                            if styled_button(ui, &self.ui_config.button, "Load SGF").clicked() {
                                // For testing, load one of the user's SGF files
                                if let Ok(content) = std::fs::read_to_string("/Users/daniel/Downloads/76794817-078-worki-ve..sgf") {
                                    if let Err(e) = self.load_sgf(&content) {
                                        self.error_message = Some(format!("Failed to load SGF: {}", e));
                                    }
                                }
                            }
                        });
                    });

                    // Game status
                    if self.game_state.is_game_over() {
                        ui.add_space(10.0);
                        let (black_score, white_score) = self.calculate_score();
                        let winner = if black_score > white_score { "Black" } else { "White" };
                        ui.colored_label(
                            Color32::from(self.ui_config.colors.success),
                            format!("Game Over! {} wins by {:.1} points", winner, (black_score - white_score).abs())
                        );
                    }

                    // Mode indicator
                    if self.marking_territory {
                        let instruction = match self.consensus_phase {
                            ConsensusPhase::BothMarking => "Click empty intersections to mark territory. Click stones to mark as dead.",
                            ConsensusPhase::WaitingAgreement => "Review territory markings. Click 'Accept Territory' if correct.",
                            _ => "Click empty intersections to mark territory",
                        };
                        ui.colored_label(
                            Color32::from(self.ui_config.colors.info),
                            instruction
                        );
                    }
                });
            });
    }

    /// Draw stone with simple, clean rendering like OGS
    fn draw_stone_with_gradient(&self, painter: &Painter, pos: Pos2, radius: f32, color: Color) {
        // Simple 3-layer gradient for subtle depth without excessive effects
        const LAYERS: usize = 3;

        match color {
            Color::Black => {
                // Black stone: simple dark gradient
                for i in 0..LAYERS {
                    let t = i as f32 / (LAYERS - 1) as f32;
                    let layer_radius = radius * (1.0 - t * 0.08); // Very subtle layering
                    let gray_value = (10.0 + t * 15.0) as u8; // Dark gradient

                    painter.circle_filled(pos, layer_radius, Color32::from_gray(gray_value));
                }

                // Subtle outline
                painter.circle(
                    pos,
                    radius,
                    Color32::TRANSPARENT,
                    Stroke::new(
                        self.ui_config.board.stone_outline_width * 0.8,
                        Color32::from_gray(5),
                    ),
                );
            }
            Color::White => {
                // White stone: simple light gradient
                for i in 0..LAYERS {
                    let t = i as f32 / (LAYERS - 1) as f32;
                    let layer_radius = radius * (1.0 - t * 0.08);
                    let gray_value = (250.0 - t * 15.0) as u8; // Light gradient

                    painter.circle_filled(pos, layer_radius, Color32::from_gray(gray_value));
                }

                // Single subtle highlight (much smaller than before)
                let highlight_offset = radius * 0.3;
                let highlight_pos = pos + Vec2::new(-highlight_offset, -highlight_offset);
                painter.circle_filled(
                    highlight_pos,
                    radius * 0.15, // Small highlight
                    Color32::from_rgba_unmultiplied(255, 255, 255, 100),
                );

                // Subtle gray outline
                painter.circle(
                    pos,
                    radius,
                    Color32::TRANSPARENT,
                    Stroke::new(
                        self.ui_config.board.stone_outline_width,
                        Color32::from_gray(180),
                    ),
                );
            }
        }
    }

    /// Get UI color based on win probability from neural nets
    pub fn get_probability_color(&self, black_prob: f32, white_prob: f32) -> Color32 {
        const MIDDLE_GRAY: u8 = 127;

        // Calculate color shift based on probability difference
        let prob_diff = black_prob - white_prob; // -1.0 to 1.0

        // Subtle shift: max 20% deviation from middle gray
        let shift_factor = prob_diff * 0.2;
        let gray_value = (MIDDLE_GRAY as f32 * (1.0 - shift_factor)) as u8;

        Color32::from_gray(gray_value)
    }

    /// Draw the Go board
    fn draw_board(&mut self, ui: &mut Ui) {
        // Use available space for board
        let available = ui.available_size();
        let board_size = (available.x.min(available.y) - 40.0).min(self.ui_config.board.size);
        let (response, painter) = ui.allocate_painter(Vec2::splat(board_size), Sense::click());

        let rect = response.rect;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(self.ui_config.board.margin),
            Vec2::splat(board_size - 2.0 * self.ui_config.board.margin),
        );

        // Board background - pure white like OGS
        painter.rect_filled(board_rect, 0.0, Color32::WHITE);

        // Calculate cell size
        let cell_size = board_rect.width() / 8.0; // 8 cells between 9 lines

        // Draw grid lines
        let grid_stroke = Stroke::new(
            self.ui_config.board.grid_line_width,
            Color32::from(self.ui_config.board.grid_color),
        );

        for i in 0..9 {
            let offset = i as f32 * cell_size;

            // Vertical lines
            painter.line_segment(
                [
                    Pos2::new(board_rect.min.x + offset, board_rect.min.y),
                    Pos2::new(board_rect.min.x + offset, board_rect.max.y),
                ],
                grid_stroke,
            );

            // Horizontal lines
            painter.line_segment(
                [
                    Pos2::new(board_rect.min.x, board_rect.min.y + offset),
                    Pos2::new(board_rect.max.x, board_rect.min.y + offset),
                ],
                grid_stroke,
            );
        }

        // Draw star points (for 9x9: at 2,2 2,6 4,4 6,2 6,6)
        let star_points = [(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)];
        for (x, y) in star_points {
            let pos = Pos2::new(
                board_rect.min.x + x as f32 * cell_size,
                board_rect.min.y + y as f32 * cell_size,
            );
            painter.circle_filled(
                pos,
                3.0, // Smaller star points
                Color32::BLACK,
            );
        }

        // Draw coordinates if enabled
        if self.ui_config.board.show_coordinates {
            let font_id =
                create_font_id(&self.ui_config, self.ui_config.board.coordinate_font_size);
            let text_color: Color32 = Color32::from(self.ui_config.colors.text_dark);

            for i in 0..9 {
                // Letters (A-J, skipping I)
                let letter = if i < 8 { (b'A' + i) as char } else { 'J' };
                painter.text(
                    Pos2::new(
                        board_rect.min.x + i as f32 * cell_size,
                        board_rect.min.y - 15.0,
                    ),
                    egui::Align2::CENTER_BOTTOM,
                    letter,
                    font_id.clone(),
                    text_color,
                );

                // Numbers (1-9)
                painter.text(
                    Pos2::new(
                        board_rect.min.x - 15.0,
                        board_rect.max.y - i as f32 * cell_size,
                    ),
                    egui::Align2::RIGHT_CENTER,
                    (i + 1).to_string(),
                    font_id.clone(),
                    text_color,
                );
            }
        }

        // Draw stones and territory marks
        for y in 0..9 {
            for x in 0..9 {
                let idx = y * 9 + x;
                let pos = Pos2::new(
                    board_rect.min.x + x as f32 * cell_size,
                    board_rect.min.y + y as f32 * cell_size,
                );

                // Draw stone if present
                if let Some(color) = self.game_state.board[idx] {
                    let stone_pos = (x as u8, y as u8);
                    let is_dead = self.dead_stones.contains_key(&stone_pos);
                    let radius = cell_size * self.ui_config.board.stone_radius_ratio;

                    // Draw stone with gradient effect
                    self.draw_stone_with_gradient(&painter, pos, radius, color);

                    // Mark dead stones with X
                    if is_dead {
                        let cross_size = radius * 0.7;
                        let cross_color = match color {
                            Color::Black => Color32::from_rgb(200, 200, 200),
                            Color::White => Color32::from_rgb(80, 80, 80),
                        };
                        painter.line_segment(
                            [
                                pos - Vec2::new(cross_size, cross_size),
                                pos + Vec2::new(cross_size, cross_size),
                            ],
                            Stroke::new(3.0, cross_color),
                        );
                        painter.line_segment(
                            [
                                pos - Vec2::new(cross_size, -cross_size),
                                pos + Vec2::new(cross_size, -cross_size),
                            ],
                            Stroke::new(3.0, cross_color),
                        );
                    }

                    // Last move marker
                    if self.last_move == Some((x as u8, y as u8)) && !is_dead {
                        let marker_color = match color {
                            Color::Black => Color32::WHITE,
                            Color::White => Color32::BLACK,
                        };
                        painter.circle_filled(
                            pos,
                            radius * self.ui_config.board.last_move_marker_ratio,
                            marker_color,
                        );
                    }
                } else if let Some(mark) = self.territory_marks.get(&(x as u8, y as u8)) {
                    // Draw territory marking with red outline
                    let mark_color: Color32 = match mark {
                        TerritoryMark::Black => {
                            Color32::from(self.ui_config.territory.black_territory_color)
                        }
                        TerritoryMark::White => {
                            Color32::from(self.ui_config.territory.white_territory_color)
                        }
                        TerritoryMark::None => continue,
                    };

                    let mark_size = cell_size * self.ui_config.territory.marker_size_ratio;

                    match self.ui_config.territory.marker_type {
                        TerritoryMarkerType::Square => {
                            let rect = Rect::from_center_size(pos, Vec2::splat(mark_size));
                            painter.rect_filled(rect, 0.0, mark_color);
                            // Red outline for visibility
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
                            );
                        }
                        TerritoryMarkerType::Circle => {
                            painter.circle_filled(pos, mark_size / 2.0, mark_color);
                            // Red outline
                            painter.circle_stroke(
                                pos,
                                mark_size / 2.0,
                                Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
                            );
                        }
                        TerritoryMarkerType::Cross => {
                            let half = mark_size / 2.0;
                            // Red cross for better visibility
                            painter.line_segment(
                                [pos - Vec2::new(half, half), pos + Vec2::new(half, half)],
                                Stroke::new(3.0, Color32::from_rgb(255, 0, 0)),
                            );
                            painter.line_segment(
                                [pos - Vec2::new(half, -half), pos + Vec2::new(half, -half)],
                                Stroke::new(3.0, Color32::from_rgb(255, 0, 0)),
                            );
                        }
                        TerritoryMarkerType::Fill => {
                            let rect = Rect::from_center_size(pos, Vec2::splat(cell_size * 0.9));
                            painter.rect_filled(rect, 0.0, mark_color);
                            // Red outline
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
                            );
                        }
                        TerritoryMarkerType::Overlay => {
                            let rect = Rect::from_center_size(pos, Vec2::splat(cell_size));
                            painter.rect_filled(rect, 0.0, mark_color);
                            // Red outline
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
                            );
                        }
                    }
                }
            }
        }

        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let relative_pos = pos - board_rect.min;
                let x = (relative_pos.x / cell_size).round() as u8;
                let y = (relative_pos.y / cell_size).round() as u8;

                if x < 9 && y < 9 {
                    self.handle_board_click(x, y);
                }
            }
        }

        // Show hover cursor
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }

    /// Calculate distance features for guild classification
    fn calculate_distance_features(&self, x: u8, y: u8) -> DistanceFeatures {
        let from_center = ((x as f32 - 4.0).abs() + (y as f32 - 4.0).abs()) / 2.0;

        // Calculate distances to nearest stones
        let mut from_nearest_friend: f32 = 10.0;
        let mut from_nearest_enemy: f32 = 10.0;

        for i in 0..9 {
            for j in 0..9 {
                let idx = j * 9 + i;
                if let Some(color) = self.game_state.board[idx] {
                    let dist = (x as f32 - i as f32).abs() + (y as f32 - j as f32).abs();
                    if color == self.game_state.current_player {
                        from_nearest_friend = from_nearest_friend.min(dist);
                    } else {
                        from_nearest_enemy = from_nearest_enemy.min(dist);
                    }
                }
            }
        }

        DistanceFeatures {
            from_last_stone: self
                .last_move
                .map(|(lx, ly)| ((x as f32 - lx as f32).abs() + (y as f32 - ly as f32).abs()))
                .unwrap_or(0.0),
            from_last_capture: None, // TODO: Track capture points
            from_center,
            from_nearest_friend,
            from_nearest_enemy,
        }
    }

    /// Update player guild based on move history
    fn update_player_guild(&mut self) {
        let mut guild_counts = HashMap::new();

        // Count recent moves by guild
        for (_, guild) in self.guild_history.iter().rev().take(10) {
            *guild_counts.entry(*guild).or_insert(0) += 1;
        }

        // Find dominant guild
        self.player_guild = guild_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(guild, _)| guild);
    }

    /// Calculate final guild affinity at game end
    fn calculate_final_guild_affinity(&mut self) {
        if self.guild_history.is_empty() {
            return;
        }

        let mut guild_scores = HashMap::new();
        guild_scores.insert(Guild::Activity, 0.0);
        guild_scores.insert(Guild::Reactivity, 0.0);
        guild_scores.insert(Guild::Avoidance, 0.0);

        // Weight later moves more heavily
        let total_moves = self.guild_history.len();
        for (i, (vector, _)) in self.guild_history.iter().enumerate() {
            let weight = (i + 1) as f32 / total_moves as f32;
            let affinities = vector.guild_affinity();

            for (guild, score) in affinities {
                *guild_scores.get_mut(&guild).unwrap() += score * weight;
            }
        }

        // Find dominant guild
        self.player_guild = guild_scores
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(guild, _)| guild);
    }

    /// Render guild statistics as a bar graph
    fn render_guild_bar_graph(&self, ui: &mut Ui) {
        let mut guild_scores = HashMap::new();
        guild_scores.insert(Guild::Activity, 0.0);
        guild_scores.insert(Guild::Reactivity, 0.0);
        guild_scores.insert(Guild::Avoidance, 0.0);

        // Calculate weighted scores
        let total_moves = self.guild_history.len();
        for (i, (vector, _)) in self.guild_history.iter().enumerate() {
            let weight = (i + 1) as f32 / total_moves as f32;
            let affinities = vector.guild_affinity();

            for (guild, score) in affinities {
                *guild_scores.get_mut(&guild).unwrap() += score * weight;
            }
        }

        // Normalize scores
        let total: f32 = guild_scores.values().sum();
        if total > 0.0 {
            for score in guild_scores.values_mut() {
                *score /= total;
            }
        }

        // Draw bar graph
        ui.group(|ui| {
            ui.label("Guild Affinity Distribution:");
            ui.add_space(5.0);

            for (guild, score) in guild_scores {
                ui.horizontal(|ui| {
                    let guild_color = match guild {
                        Guild::Activity => Color32::from_rgb(220, 100, 100),
                        Guild::Reactivity => Color32::from_rgb(100, 100, 220),
                        Guild::Avoidance => Color32::from_rgb(100, 220, 100),
                    };

                    ui.label(format!("{:?}:", guild));
                    let bar_width = 150.0 * score;
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(bar_width, 16.0), Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, guild_color);
                    ui.label(format!("{:.1}%", score * 100.0));
                });
            }
        });
    }

    /// Flood fill territory marking
    fn flood_fill_territory(&mut self, start_x: u8, start_y: u8, color: TerritoryMark) {
        if color == TerritoryMark::None {
            return;
        }

        let mut stack = vec![(start_x, start_y)];
        let mut visited = HashSet::new();

        while let Some((x, y)) = stack.pop() {
            if visited.contains(&(x, y)) {
                continue;
            }
            visited.insert((x, y));

            let idx = (y as usize) * 9 + (x as usize);

            // Only fill empty intersections
            if self.game_state.board[idx].is_none() && !self.dead_stones.contains_key(&(x, y)) {
                self.territory_marks.insert((x, y), color);

                // Add adjacent cells
                if x > 0 {
                    stack.push((x - 1, y));
                }
                if x < 8 {
                    stack.push((x + 1, y));
                }
                if y > 0 {
                    stack.push((x, y - 1));
                }
                if y < 8 {
                    stack.push((x, y + 1));
                }
            }
        }
    }

    /// Clear territory region
    fn clear_territory_region(&mut self, start_x: u8, start_y: u8) {
        let mut stack = vec![(start_x, start_y)];
        let mut visited = HashSet::new();
        let target_mark = self.territory_marks.get(&(start_x, start_y)).copied();

        if target_mark.is_none() {
            return;
        }

        while let Some((x, y)) = stack.pop() {
            if visited.contains(&(x, y)) {
                continue;
            }
            visited.insert((x, y));

            if self.territory_marks.get(&(x, y)) == target_mark.as_ref() {
                self.territory_marks.remove(&(x, y));

                // Add adjacent cells
                if x > 0 {
                    stack.push((x - 1, y));
                }
                if x < 8 {
                    stack.push((x + 1, y));
                }
                if y > 0 {
                    stack.push((x, y - 1));
                }
                if y < 8 {
                    stack.push((x, y + 1));
                }
            }
        }
    }

    /// Determine territory color based on surrounding stones
    fn determine_territory_color(&self, x: u8, y: u8) -> TerritoryMark {
        let mut black_influence = 0;
        let mut white_influence = 0;

        // Check surrounding area (distance 2)
        for dx in -2i8..=2 {
            for dy in -2i8..=2 {
                let nx = x as i8 + dx;
                let ny = y as i8 + dy;

                if nx >= 0 && nx < 9 && ny >= 0 && ny < 9 {
                    let idx = (ny as usize) * 9 + (nx as usize);
                    if let Some(color) = self.game_state.board[idx] {
                        let distance = dx.abs() + dy.abs();
                        let influence = 3 - distance.min(3);

                        match color {
                            Color::Black => black_influence += influence,
                            Color::White => white_influence += influence,
                        }
                    }
                }
            }
        }

        if black_influence > white_influence {
            TerritoryMark::Black
        } else if white_influence > black_influence {
            TerritoryMark::White
        } else {
            TerritoryMark::Black // Default to black if equal
        }
    }

    /// Mark a group of stones as dead
    fn mark_group_as_dead(&mut self, x: u8, y: u8, color: Color) {
        let mut group = HashSet::new();
        let mut stack = vec![(x, y)];

        // Find all stones in the group
        while let Some((cx, cy)) = stack.pop() {
            if group.contains(&(cx, cy)) {
                continue;
            }

            let idx = (cy as usize) * 9 + (cx as usize);
            if self.game_state.board[idx] == Some(color) {
                group.insert((cx, cy));

                // Add adjacent stones
                if cx > 0 {
                    stack.push((cx - 1, cy));
                }
                if cx < 8 {
                    stack.push((cx + 1, cy));
                }
                if cy > 0 {
                    stack.push((cx, cy - 1));
                }
                if cy < 8 {
                    stack.push((cx, cy + 1));
                }
            }
        }

        // Toggle dead status for the group
        let is_dead = self.dead_stones.get(&(x, y)).copied().unwrap_or(false);
        for pos in group {
            if is_dead {
                self.dead_stones.remove(&pos);
            } else {
                self.dead_stones.insert(pos, true);
            }
        }
    }

    /// Generate CBOR training data from agreed game
    fn generate_training_data(&self) {
        use p2pgo_core::cbor::{MoveRecord, Tag};

        // Create training data structure
        let mut move_records = Vec::new();

        // Convert game moves to CBOR format (simplified for now)
        for (i, mv) in self.game_state.moves.iter().enumerate() {
            // For now, just track the moves without full CBOR structure
            // The WASM engine will handle proper CBOR generation
            move_records.push(MoveRecord {
                mv: mv.clone(),
                tag: Some(Tag::Activity), // Default tag
                ts: i as u64,
                broadcast_hash: None,
                prev_hash: None, // Would be calculated by WASM engine
                signature: None,
                signer: None,
                sequence: i as u32,
            });
        }

        // Create final game data with territory and dead stones
        let final_data = serde_json::json!({
            "game_id": &self.game_state.id,
            "board_size": self.game_state.board_size,
            "moves": move_records.len(),
            "territory_marks": self.territory_marks.iter()
                .map(|((x, y), mark)| {
                    format!("{},{},{:?}", x, y, mark)
                })
                .collect::<Vec<_>>(),
            "dead_stones": self.dead_stones.keys()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>(),
            "final_score": {
                "black": self.calculate_score().0,
                "white": self.calculate_score().1,
            },
            "consensus_reached": self.consensus_phase == ConsensusPhase::Agreed,
        });

        eprintln!("Game complete - CBOR training data ready:");
        eprintln!("{}", serde_json::to_string_pretty(&final_data).unwrap());

        // In production, this would:
        // 1. Call WASM engine to validate game
        // 2. Generate proper CBOR encoding
        // 3. Sign with player keys
        // 4. Store for neural network training
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_territory_mark_cycle() {
        let mark = TerritoryMark::None;
        assert_eq!(mark.cycle(), TerritoryMark::Black);
        assert_eq!(mark.cycle().cycle(), TerritoryMark::White);
        assert_eq!(mark.cycle().cycle().cycle(), TerritoryMark::None);
    }

    #[test]
    fn test_new_game() {
        let game = OfflineGoGame::new();
        assert_eq!(game.game_state.board_size, 9);
        assert!(!game.marking_territory);
        assert!(game.territory_marks.is_empty());
    }
}
