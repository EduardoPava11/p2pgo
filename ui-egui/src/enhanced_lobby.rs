use eframe::egui::{self, Color32, RichText, Stroke};
use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Enhanced game lobby with decentralized design patterns
pub struct EnhancedGameLobby {
    /// Active games (currently playing)
    active_games: HashMap<String, ActiveGame>,
    /// Open challenges waiting for opponents
    open_challenges: Vec<Challenge>,
    /// Quick match preferences
    quick_match_prefs: QuickMatchPreferences,
    /// Player profiles cache
    player_cache: HashMap<PeerId, PlayerProfile>,
    /// UI state
    ui_state: LobbyUIState,
}

#[derive(Clone)]
pub struct ActiveGame {
    pub game_id: String,
    pub opponent: PlayerProfile,
    pub my_color: GoColor,
    pub current_turn: GoColor,
    pub move_count: u32,
    pub time_remaining: Duration,
    pub last_move_time: Instant,
}

#[derive(Clone)]
pub struct Challenge {
    pub challenge_id: String,
    pub challenger: PlayerProfile,
    pub time_control: TimeControl,
    pub board_size: u8,
    pub handicap: u8,
    pub ranked: bool,
    pub created_at: Instant,
}

#[derive(Clone)]
pub struct PlayerProfile {
    pub peer_id: PeerId,
    pub nickname: String,
    pub rank: GoRank,
    pub games_played: u32,
    pub win_rate: f32,
    pub reliability: f32, // Completion rate
    pub preferred_time: TimeControl,
    pub is_online: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GoRank {
    Kyu(u8), // 30k to 1k
    Dan(u8), // 1d to 9d
}

#[derive(Clone, Copy, PartialEq)]
pub enum GoColor {
    Black,
    White,
}

#[derive(Clone, PartialEq)]
pub enum TimeControl {
    Blitz { minutes: u32, increment: u32 },
    Rapid { minutes: u32, increment: u32 },
    Correspondence { days_per_move: u32 },
}

#[derive(Default)]
pub struct QuickMatchPreferences {
    pub rank_range: u8, // ±N ranks
    pub time_control: Option<TimeControl>,
    pub board_size: u8,
    pub allow_handicap: bool,
}

#[derive(Default)]
struct LobbyUIState {
    selected_game: Option<String>,
    selected_challenge: Option<String>,
    create_challenge_open: bool,
    new_challenge: NewChallenge,
}

#[derive(Default)]
struct NewChallenge {
    time_control_type: usize,
    minutes: u32,
    increment: u32,
    days_per_move: u32,
    board_size: u8,
    handicap: u8,
    ranked: bool,
}

impl EnhancedGameLobby {
    pub fn new() -> Self {
        Self {
            active_games: HashMap::new(),
            open_challenges: Vec::new(),
            quick_match_prefs: QuickMatchPreferences {
                rank_range: 3,
                time_control: None,
                board_size: 9,
                allow_handicap: true,
            },
            player_cache: HashMap::new(),
            ui_state: LobbyUIState::default(),
        }
    }
    
    /// Render the enhanced lobby
    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<LobbyAction> {
        let mut action = None;
        
        // Header with stats
        self.render_header(ui);
        ui.separator();
        
        // Main content in three columns
        ui.columns(3, |columns| {
            // Left column: Active games
            columns[0].group(|ui| {
                if let Some(new_action) = self.render_active_games(ui) {
                    action = Some(new_action);
                }
            });
            
            // Middle column: Open challenges
            columns[1].group(|ui| {
                if let Some(new_action) = self.render_open_challenges(ui) {
                    action = Some(new_action);
                }
            });
            
            // Right column: Quick actions
            columns[2].group(|ui| {
                if let Some(new_action) = self.render_quick_actions(ui) {
                    action = Some(new_action);
                }
            });
        });
        
        action
    }
    
    fn render_header(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🎮 P2P Go Lobby");
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Network status
                let online_count = self.player_cache.values().filter(|p| p.is_online).count();
                ui.label(format!("🟢 {} players online", online_count));
                
                ui.separator();
                
                // Active games count
                ui.label(format!("📋 {} active games", self.active_games.len()));
            });
        });
    }
    
    fn render_active_games(&mut self, ui: &mut egui::Ui) -> Option<LobbyAction> {
        let mut action = None;
        
        ui.label(RichText::new("Active Games").heading());
        ui.separator();
        
        if self.active_games.is_empty() {
            ui.label("No active games");
        } else {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (game_id, game) in &self.active_games {
                    let is_my_turn = game.current_turn == game.my_color;
                    let selected = self.ui_state.selected_game.as_ref() == Some(game_id);
                    
                    let response = ui.selectable_label(selected, "");
                    let response_rect = response.rect;
                    
                    // Custom rendering
                    ui.painter().rect(
                        response_rect,
                        5.0,
                        if is_my_turn {
                            Color32::from_rgba_unmultiplied(255, 200, 100, 30)
                        } else {
                            Color32::from_rgba_unmultiplied(200, 200, 200, 20)
                        },
                        Stroke::new(1.0, if selected { Color32::WHITE } else { Color32::GRAY }),
                    );
                    
                    ui.allocate_ui_at_rect(response_rect.shrink(5.0), |ui| {
                        ui.vertical(|ui| {
                            // Opponent info
                            ui.horizontal(|ui| {
                                ui.label(self.color_symbol(game.my_color));
                                ui.label("vs");
                                ui.label(&game.opponent.nickname);
                                ui.label(format!("({})", self.format_rank(&game.opponent.rank)));
                            });
                            
                            // Game state
                            ui.horizontal(|ui| {
                                if is_my_turn {
                                    ui.colored_label(Color32::YELLOW, "Your turn");
                                } else {
                                    ui.label("Waiting");
                                }
                                ui.separator();
                                ui.label(format!("Move {}", game.move_count));
                                ui.separator();
                                ui.label(self.format_time_remaining(game.time_remaining));
                            });
                        });
                    });
                    
                    if response.clicked() {
                        self.ui_state.selected_game = Some(game_id.clone());
                        action = Some(LobbyAction::ResumeGame(game_id.clone()));
                    }
                }
            });
        }
        
        action
    }
    
    fn render_open_challenges(&mut self, ui: &mut egui::Ui) -> Option<LobbyAction> {
        let mut action = None;
        
        ui.label(RichText::new("Open Challenges").heading());
        ui.separator();
        
        if self.open_challenges.is_empty() {
            ui.label("No open challenges");
            ui.label("Create one or wait for others");
        } else {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                // Sort by rank proximity
                let my_rank = GoRank::Kyu(5); // TODO: Get actual rank
                let mut sorted_challenges = self.open_challenges.clone();
                sorted_challenges.sort_by_key(|c| self.rank_distance(&c.challenger.rank, &my_rank));
                
                for challenge in &sorted_challenges {
                    let _selected = self.ui_state.selected_challenge.as_ref() == Some(&challenge.challenge_id);
                    
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            // Challenger info
                            ui.horizontal(|ui| {
                                ui.label(&challenge.challenger.nickname);
                                ui.label(format!("({})", self.format_rank(&challenge.challenger.rank)));
                                
                                // Reliability indicator
                                let reliability_color = if challenge.challenger.reliability > 0.9 {
                                    Color32::GREEN
                                } else if challenge.challenger.reliability > 0.7 {
                                    Color32::YELLOW
                                } else {
                                    Color32::RED
                                };
                                ui.colored_label(
                                    reliability_color,
                                    format!("⚡ {:.0}%", challenge.challenger.reliability * 100.0)
                                );
                            });
                            
                            // Challenge details
                            ui.horizontal(|ui| {
                                ui.label(format!("{}×{}", challenge.board_size, challenge.board_size));
                                ui.separator();
                                ui.label(self.format_time_control(&challenge.time_control));
                                if challenge.handicap > 0 {
                                    ui.separator();
                                    ui.label(format!("H{}", challenge.handicap));
                                }
                                if challenge.ranked {
                                    ui.separator();
                                    ui.colored_label(Color32::GOLD, "Ranked");
                                }
                            });
                            
                            // Action button
                            ui.horizontal(|ui| {
                                if ui.button("Accept").clicked() {
                                    action = Some(LobbyAction::AcceptChallenge(challenge.challenge_id.clone()));
                                }
                                
                                let age = challenge.created_at.elapsed();
                                ui.label(format!("{} ago", self.format_duration(age)));
                            });
                        });
                    });
                }
            });
        }
        
        action
    }
    
    fn render_quick_actions(&mut self, ui: &mut egui::Ui) -> Option<LobbyAction> {
        let mut action = None;
        
        ui.label(RichText::new("Quick Actions").heading());
        ui.separator();
        
        // Quick match
        ui.group(|ui| {
            ui.label(RichText::new("Quick Match").strong());
            
            ui.horizontal(|ui| {
                ui.label("Rank range:");
                ui.add(egui::Slider::new(&mut self.quick_match_prefs.rank_range, 1..=9)
                    .suffix(" ranks"));
            });
            
            ui.checkbox(&mut self.quick_match_prefs.allow_handicap, "Allow handicap");
            
            if ui.button("🎲 Find Match").clicked() {
                action = Some(LobbyAction::QuickMatch(self.quick_match_prefs.clone()));
            }
        });
        
        ui.separator();
        
        // Create challenge
        ui.group(|ui| {
            if ui.button("➕ Create Challenge").clicked() {
                self.ui_state.create_challenge_open = !self.ui_state.create_challenge_open;
            }
            
            if self.ui_state.create_challenge_open {
                self.render_create_challenge(ui);
                
                if ui.button("Create").clicked() {
                    action = Some(LobbyAction::CreateChallenge {
                        time_control: self.create_time_control(),
                        board_size: self.ui_state.new_challenge.board_size,
                        handicap: self.ui_state.new_challenge.handicap,
                        ranked: self.ui_state.new_challenge.ranked,
                    });
                    self.ui_state.create_challenge_open = false;
                }
            }
        });
        
        ui.separator();
        
        // Direct challenge
        ui.group(|ui| {
            ui.label(RichText::new("Direct Challenge").strong());
            ui.label("Share this link:");
            
            let challenge_link = format!("p2pgo://challenge/{}", "your-peer-id");
            ui.horizontal(|ui| {
                ui.monospace(&challenge_link);
                if ui.button("📋").on_hover_text("Copy").clicked() {
                    ui.output_mut(|o| o.copied_text = challenge_link);
                }
            });
        });
        
        action
    }
    
    fn render_create_challenge(&mut self, ui: &mut egui::Ui) {
        let nc = &mut self.ui_state.new_challenge;
        
        // Time control
        ui.horizontal(|ui| {
            ui.label("Time:");
            ui.selectable_value(&mut nc.time_control_type, 0, "Blitz");
            ui.selectable_value(&mut nc.time_control_type, 1, "Rapid");
            ui.selectable_value(&mut nc.time_control_type, 2, "Correspondence");
        });
        
        match nc.time_control_type {
            0 | 1 => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut nc.minutes).suffix(" min"));
                    ui.label("+");
                    ui.add(egui::DragValue::new(&mut nc.increment).suffix(" sec"));
                });
            }
            2 => {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut nc.days_per_move).suffix(" days/move"));
                });
            }
            _ => {}
        }
        
        // Board size (fixed at 9 for MVP)
        nc.board_size = 9;
        ui.label("Board: 9×9");
        
        // Handicap
        ui.horizontal(|ui| {
            ui.label("Handicap:");
            ui.add(egui::Slider::new(&mut nc.handicap, 0..=9));
        });
        
        // Ranked
        ui.checkbox(&mut nc.ranked, "Ranked game");
    }
    
    // Helper methods
    fn color_symbol(&self, color: GoColor) -> &'static str {
        match color {
            GoColor::Black => "●",
            GoColor::White => "○",
        }
    }
    
    fn format_rank(&self, rank: &GoRank) -> String {
        match rank {
            GoRank::Kyu(k) => format!("{}k", k),
            GoRank::Dan(d) => format!("{}d", d),
        }
    }
    
    fn rank_distance(&self, a: &GoRank, b: &GoRank) -> u8 {
        let a_val = match a {
            GoRank::Kyu(k) => 30 - k,
            GoRank::Dan(d) => 30 + d,
        };
        let b_val = match b {
            GoRank::Kyu(k) => 30 - k,
            GoRank::Dan(d) => 30 + d,
        };
        (a_val as i16 - b_val as i16).abs() as u8
    }
    
    fn format_time_control(&self, tc: &TimeControl) -> String {
        match tc {
            TimeControl::Blitz { minutes, increment } => format!("{}+{}", minutes, increment),
            TimeControl::Rapid { minutes, increment } => format!("{}+{}", minutes, increment),
            TimeControl::Correspondence { days_per_move } => format!("{} days", days_per_move),
        }
    }
    
    fn format_time_remaining(&self, duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{}:{:02}", minutes, seconds)
        }
    }
    
    fn format_duration(&self, duration: Duration) -> String {
        let seconds = duration.as_secs();
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m", seconds / 60)
        } else {
            format!("{}h", seconds / 3600)
        }
    }
    
    fn create_time_control(&self) -> TimeControl {
        let nc = &self.ui_state.new_challenge;
        match nc.time_control_type {
            0 => TimeControl::Blitz { minutes: nc.minutes, increment: nc.increment },
            1 => TimeControl::Rapid { minutes: nc.minutes, increment: nc.increment },
            2 => TimeControl::Correspondence { days_per_move: nc.days_per_move },
            _ => TimeControl::Rapid { minutes: 10, increment: 5 },
        }
    }
}

/// Lobby actions
#[derive(Debug, Clone)]
pub enum LobbyAction {
    ResumeGame(String),
    AcceptChallenge(String),
    QuickMatch(QuickMatchPreferences),
    CreateChallenge {
        time_control: TimeControl,
        board_size: u8,
        handicap: u8,
        ranked: bool,
    },
}