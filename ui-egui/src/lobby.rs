use eframe::egui;
use libp2p::{Multiaddr, PeerId};
use std::collections::HashMap;
use std::time::Instant;

/// Game lobby for discovering other relays and games
pub struct GameLobby {
    /// Available games
    games: HashMap<String, GameListing>,
    /// Online relays
    online_relays: HashMap<PeerId, RelayInfo>,
    /// Direct connect input
    direct_connect_addr: String,
    /// Status message
    status: String,
    /// Last refresh time
    last_refresh: Instant,
}

#[derive(Clone, Debug)]
pub struct GameListing {
    pub game_id: String,
    pub host: PeerId,
    pub players: Vec<String>,
    pub board_size: u8,
    pub time_control: String,
    pub created_at: Instant,
}

#[derive(Clone, Debug)]
pub struct RelayInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub last_seen: Instant,
    pub connection_type: String,
    pub latency_ms: Option<u64>,
}

impl GameLobby {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            online_relays: HashMap::new(),
            direct_connect_addr: String::new(),
            status: "Searching for games...".to_string(),
            last_refresh: Instant::now(),
        }
    }
    
    /// Update relay list from network events
    pub fn update_relay(&mut self, peer_id: PeerId, info: RelayInfo) {
        self.online_relays.insert(peer_id, info);
    }
    
    /// Update game listings
    pub fn update_game(&mut self, game: GameListing) {
        self.games.insert(game.game_id.clone(), game);
    }
    
    /// Render the lobby UI
    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<LobbyAction> {
        let mut action = None;
        
        ui.heading("🎮 Game Lobby");
        ui.separator();
        
        // Status and refresh
        ui.horizontal(|ui| {
            ui.label(&self.status);
            
            if ui.button("🔄 Refresh").clicked() {
                self.last_refresh = Instant::now();
                action = Some(LobbyAction::Refresh);
            }
            
            let elapsed = self.last_refresh.elapsed().as_secs();
            ui.label(format!("Last refresh: {}s ago", elapsed));
        });
        
        ui.separator();
        
        // Direct connect section
        ui.collapsing("Direct Connect", |ui| {
            ui.horizontal(|ui| {
                ui.label("Multiaddr:");
                ui.text_edit_singleline(&mut self.direct_connect_addr);
                
                if ui.button("Connect").clicked() && !self.direct_connect_addr.is_empty() {
                    action = Some(LobbyAction::DirectConnect(self.direct_connect_addr.clone()));
                }
            });
            
            ui.label("Example: /ip4/192.168.1.100/tcp/4001/p2p/12D3KooW...");
        });
        
        ui.separator();
        
        // Online relays section
        ui.collapsing("Online Relays", |ui| {
            if self.online_relays.is_empty() {
                ui.label("No relays discovered yet. Make sure mDNS is enabled for local discovery.");
            } else {
                egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (peer_id, info) in &self.online_relays {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Connection type indicator
                                let icon = match info.connection_type.as_str() {
                                    "Direct" => "🟢",
                                    "Relayed" => "🟡",
                                    "Local" => "🔵",
                                    _ => "⚫",
                                };
                                ui.label(icon);
                                
                                // Peer ID (shortened)
                                let short_id = format!("{}...{}", 
                                    &peer_id.to_string()[..8],
                                    &peer_id.to_string()[peer_id.to_string().len()-4..]
                                );
                                ui.monospace(&short_id);
                                
                                // Latency if available
                                if let Some(latency) = info.latency_ms {
                                    ui.label(format!("{}ms", latency));
                                }
                                
                                // Connect button
                                if ui.button("Connect").clicked() {
                                    action = Some(LobbyAction::ConnectToPeer(*peer_id));
                                }
                            });
                            
                            // Show addresses
                            for addr in &info.addresses {
                                ui.small(addr.to_string());
                            }
                        });
                    }
                });
            }
        });
        
        ui.separator();
        
        // Available games section
        ui.heading("Available Games");
        
        if self.games.is_empty() {
            ui.label("No active games. Create a new game or wait for others to host.");
            
            if ui.button("➕ Create New Game").clicked() {
                action = Some(LobbyAction::CreateGame);
            }
        } else {
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for (_, game) in &self.games {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}x{}", game.board_size, game.board_size));
                            ui.separator();
                            ui.label(&game.time_control);
                            ui.separator();
                            ui.label(format!("{} players", game.players.len()));
                            
                            if game.players.len() < 2 {
                                if ui.button("Join").clicked() {
                                    action = Some(LobbyAction::JoinGame(game.game_id.clone()));
                                }
                            } else {
                                if ui.button("Spectate").clicked() {
                                    action = Some(LobbyAction::SpectateGame(game.game_id.clone()));
                                }
                            }
                        });
                    });
                }
            });
        }
        
        action
    }
}

/// Actions from lobby UI
#[derive(Debug, Clone)]
pub enum LobbyAction {
    Refresh,
    DirectConnect(String),
    ConnectToPeer(PeerId),
    CreateGame,
    JoinGame(String),
    SpectateGame(String),
}