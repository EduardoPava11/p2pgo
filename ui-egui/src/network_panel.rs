// SPDX-License-Identifier: MIT OR Apache-2.0

//! Network diagnostics panel for P2P Go

use std::collections::HashMap;
use std::time::Instant;
use eframe::egui;
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Directed;
use std::sync::Arc;

#[cfg(feature = "iroh")]
use p2pgo_network::relay_monitor::{RelayStats, RelayHealth, RelayHealthStatus, RelayHealthEvent};

/// Network connection state machine
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkState {
    /// No network connection
    Offline,
    /// Starting or restarting relay
    StartingRelay,
    /// Synchronizing with network
    Syncing,
    /// Fully online with healthy connection
    Online,
    /// Online but with degraded performance
    Degraded,
}

impl Default for NetworkState {
    fn default() -> Self {
        NetworkState::Offline
    }
}

/// Connection quality metrics
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss: f32,
    /// Average latency in milliseconds
    pub avg_latency: f32,
    /// Minimum latency in milliseconds
    pub min_latency: f32,
    /// Maximum latency in milliseconds
    pub max_latency: f32,
    /// Jitter (latency variation) in milliseconds
    pub jitter: f32,
    /// Connection stability score (0.0 - 1.0)
    pub stability: f32,
    /// Last update time
    pub last_update: Instant,
}

impl Default for ConnectionQuality {
    fn default() -> Self {
        Self {
            packet_loss: 0.0,
            avg_latency: 0.0,
            min_latency: f32::MAX,
            max_latency: 0.0,
            jitter: 0.0,
            stability: 1.0,
            last_update: Instant::now(),
        }
    }
}

impl ConnectionQuality {
    /// Update metrics with new latency measurement
    pub fn update_latency(&mut self, latency: f32) {
        self.min_latency = self.min_latency.min(latency);
        self.max_latency = self.max_latency.max(latency);
        
        // Simple moving average
        if self.avg_latency == 0.0 {
            self.avg_latency = latency;
        } else {
            self.avg_latency = self.avg_latency * 0.9 + latency * 0.1;
        }
        
        // Calculate jitter as absolute difference from average
        self.jitter = (latency - self.avg_latency).abs();
        
        self.last_update = Instant::now();
    }
    
    /// Get overall quality score (0.0 - 1.0)
    pub fn quality_score(&self) -> f32 {
        // Score based on latency, packet loss, and jitter
        let latency_score = (1.0 - (self.avg_latency / 500.0).min(1.0)).max(0.0);
        let loss_score = 1.0 - self.packet_loss;
        let jitter_score = (1.0 - (self.jitter / 100.0).min(1.0)).max(0.0);
        
        // Weighted average
        (latency_score * 0.4 + loss_score * 0.4 + jitter_score * 0.2) * self.stability
    }
}

/// Game connection info
#[derive(Debug, Clone)]
pub struct GameConnectionInfo {
    pub game_id: String,
    pub peer_count: usize,
    pub move_latency: Option<f32>,
    pub last_sync: Instant,
    pub is_synced: bool,
}

/// Network diagnostics panel
pub struct NetworkPanel {
    #[cfg(feature = "iroh")]
    relay_stats: Option<Arc<tokio::sync::RwLock<HashMap<String, RelayStats>>>>,
    
    #[cfg(feature = "iroh")]
    relay_health: Option<RelayHealth>,
    
    // Port the relay is listening on
    relay_port: Option<u16>,
    
    // Network state
    network_state: NetworkState,
    state_start_time: Instant,
    
    // UI state
    visible: bool,
    show_relay_details: bool,
    show_advanced_panel: bool,
    bootstrap_relays: String,
    latency_history: HashMap<String, Vec<(f64, f64)>>, // (time, latency) pairs
    last_update: Instant,
    
    // Connection graph
    connection_graph: ConnectionGraph,
    
    // Relay status display
    relay_health_status: Option<p2pgo_network::relay_monitor::RelayHealthStatus>,
    is_relay_node: bool,
    
    // Advanced panel
    restart_network_requested: bool,
    
    // Relay capacity info
    relay_active_connections: Option<usize>,
    relay_connection_limit: Option<usize>,
    
    // Connection quality tracking
    connection_quality: ConnectionQuality,
    
    // Game connection tracking
    game_connections: HashMap<String, GameConnectionInfo>,
    
    // Diagnostic state
    show_diagnostics_tab: DiagnosticsTab,
    connection_test_in_progress: bool,
    last_connection_test: Option<Instant>,
    test_results: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum DiagnosticsTab {
    Overview,
    ConnectionQuality,
    GameNetwork,
    Troubleshooting,
}

impl Default for NetworkPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkPanel {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "iroh")]
            relay_stats: None,
            
            #[cfg(feature = "iroh")]
            relay_health: None,
            
            relay_port: None,
            
            network_state: NetworkState::Offline,
            state_start_time: Instant::now(),
            
            visible: false,
            show_relay_details: false,
            show_advanced_panel: false,
            bootstrap_relays: String::new(),
            latency_history: HashMap::new(),
            last_update: Instant::now(),
            connection_graph: ConnectionGraph::new(),
            relay_health_status: None,
            is_relay_node: false,
            restart_network_requested: false,
            relay_active_connections: None,
            relay_connection_limit: None,
            connection_quality: ConnectionQuality::default(),
            game_connections: HashMap::new(),
            show_diagnostics_tab: DiagnosticsTab::Overview,
            connection_test_in_progress: false,
            last_connection_test: None,
            test_results: Vec::new(),
        }
    }
    
    /// Set the relay stats source
    #[cfg(feature = "iroh")]
    pub fn set_relay_stats(&mut self, relay_stats: Arc<tokio::sync::RwLock<HashMap<String, RelayStats>>>) {
        self.relay_stats = Some(relay_stats);
    }
    
    /// Toggle panel visibility
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
    
    /// Check if panel is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// Update panel with new data
    pub fn update_stats(&mut self, _ctx: &egui::Context) {
        #[cfg(feature = "iroh")]
        if let Some(relay_stats) = &self.relay_stats {
            // Try to read stats without blocking
            if let Ok(stats) = relay_stats.try_read() {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_update).as_secs_f64();
                
                // Update latency history and connection quality
                for (addr, stat) in stats.iter() {
                    if let Some(latency) = stat.latency_ms {
                        let entry = self.latency_history.entry(addr.clone()).or_insert_with(Vec::new);
                        entry.push((elapsed, latency as f64));
                        
                        // Keep only recent history (last 5 minutes)
                        while entry.len() > 1 && entry[0].0 < elapsed - 300.0 {
                            entry.remove(0);
                        }
                        
                        // Update connection quality metrics
                        self.connection_quality.update_latency(latency as f32);
                    }
                    
                    // Update packet loss estimate based on success rate
                    if stat.total_probes > 0 {
                        let success_rate = stat.success_rate() / 100.0;
                        self.connection_quality.packet_loss = 1.0 - success_rate as f32;
                    }
                }
                
                self.last_update = now;
                
                // Request repaint if we have new data
                _ctx.request_repaint();
            }
        }
    }
    
    /// Run this every frame to update UI elements
    pub fn update_ui(&mut self, ctx: &egui::Context) {
        // Update stats from the relay_stats source if available
        if self.is_visible() {
            self.update_stats(ctx);
        }
        
        // If panel is visible, request repaint
        if self.is_visible() {
            ctx.request_repaint();
        }
    }
    
    /// Draw the network status badge in the top bar
    pub fn draw_status_badge(&mut self, ui: &mut egui::Ui) {
        let (status_text, status_color) = self.get_overall_status();
        
        // Create badge label with relay info if needed
        let badge_text = if self.is_relay_node {
            #[cfg(feature = "iroh")]
            let text = if let Some(port) = self.relay_port {
                format!("Relay:{} {}", port, status_text)
            } else {
                format!("Relay: {}", status_text)
            };
            #[cfg(not(feature = "iroh"))]
            let text = format!("Relay: {}", status_text);
            text
        } else {
            format!("Relay: {}", status_text)
        };
        
        let button = egui::Button::new(badge_text)
            .fill(status_color)
            .rounding(egui::Rounding::same(4.0));
        
        let response = ui.add(button);
        if response.clicked() {
            self.toggle_visibility();
        }
        
        // Tooltip with quick stats
        if response.hovered() {
            // First check direct relay status
            if let Some(status) = &self.relay_health_status {
                use p2pgo_network::relay_monitor::RelayHealthStatus;
                
                let status_desc = match status {
                    RelayHealthStatus::Healthy => "Healthy",
                    RelayHealthStatus::Degraded => "Degraded",
                    RelayHealthStatus::Unreachable => "Unreachable",
                    RelayHealthStatus::Restarting => "Restarting",
                    RelayHealthStatus::Failed => "Failed",
                };
                
                let mut tooltip_text = format!("Relay status: {}\n", status_desc);
                
                // Add port info if available
                #[cfg(feature = "iroh")]
                if let Some(port) = self.relay_port {
                    tooltip_text.push_str(&format!("Port: {}\n", port));
                }
                
                // Add extra info for relay nodes
                if self.is_relay_node {
                    tooltip_text.push_str("Mode: Embedded relay\n");
                    tooltip_text.push_str("Click for detailed status");
                } else {
                    tooltip_text.push_str("Mode: Client\n");
                    tooltip_text.push_str("Click for relay diagnostics");
                }
                
                ui.label(&tooltip_text);
                return;
            }
            
            // Fall back to relay stats collection
            #[cfg(feature = "iroh")]
            if let Some(relay_stats) = &self.relay_stats {
                if let Ok(stats) = relay_stats.try_read() {
                    let mut tooltip_text = String::new();
                    let mut relay_count = 0;
                    let mut online_count = 0;
                    
                    for (addr, stat) in stats.iter() {
                        relay_count += 1;
                        if stat.is_reachable {
                            online_count += 1;
                        }
                        
                        let short_addr = if addr.len() > 30 {
                            format!("{}...", &addr[..27])
                        } else {
                            addr.clone()
                        };
                        
                        tooltip_text.push_str(&format!(
                            "{} {} {}\n",
                            stat.health_status().emoji(),
                            short_addr,
                            if let Some(latency) = stat.latency_ms {
                                format!("{}ms", latency)
                            } else {
                                "unknown".to_string()
                            }
                        ));
                    }
                    
                    ui.show_tooltip_text(format!(
                        "Relays: {}/{} online\n\n{}",
                        online_count, relay_count, tooltip_text
                    ));
                }
            }
        }
    }
    
    /// Show the full network diagnostics window
    pub fn show(&mut self, ctx: &egui::Context) -> Option<crate::msg::UiToNet> {
        if !self.visible {
            return None;
        }
        
        let mut action = None;
        let mut visible = self.visible;
        egui::Window::new("Network Diagnostics")
            .open(&mut visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                action = self.draw_content(ui);
            });
        self.visible = visible;
        action
    }
    
    fn draw_content(&mut self, ui: &mut egui::Ui) -> Option<crate::msg::UiToNet> {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.show_diagnostics_tab, DiagnosticsTab::Overview, "Overview");
            ui.selectable_value(&mut self.show_diagnostics_tab, DiagnosticsTab::ConnectionQuality, "Connection Quality");
            ui.selectable_value(&mut self.show_diagnostics_tab, DiagnosticsTab::GameNetwork, "Game Network");
            ui.selectable_value(&mut self.show_diagnostics_tab, DiagnosticsTab::Troubleshooting, "Troubleshooting");
        });
        
        ui.separator();
        
        match self.show_diagnostics_tab {
            DiagnosticsTab::Overview => self.draw_overview(ui),
            DiagnosticsTab::ConnectionQuality => self.draw_connection_quality(ui),
            DiagnosticsTab::GameNetwork => self.draw_game_network(ui),
            DiagnosticsTab::Troubleshooting => return self.draw_troubleshooting(ui),
        }
        None
    }
    
    fn draw_overview(&mut self, ui: &mut egui::Ui) {
        ui.heading("Network Overview");
        
        // Connection topology
        ui.collapsing("Connection Topology", |ui| {
            self.connection_graph.show(ui);
        });
        
        // Quick stats
        #[cfg(feature = "iroh")]
        if let Some(relay_stats) = &self.relay_stats {
            if let Ok(stats) = relay_stats.try_read() {
                ui.collapsing("Relay Summary", |ui| {
                    egui::Grid::new("relay_summary")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Status");
                            ui.strong("Count");
                            ui.end_row();
                            
                            let mut counts = HashMap::new();
                            for stat in stats.values() {
                                *counts.entry(stat.health_status()).or_insert(0) += 1;
                            }
                            
                            for (health, count) in counts {
                                ui.label(format!("{} {}", health.emoji(), health.description()));
                                ui.label(count.to_string());
                                ui.end_row();
                            }
                        });
                });
            }
        }
    }
    
    fn draw_relay_details(&mut self, ui: &mut egui::Ui) {
        ui.heading("Relay Details");
        
        #[cfg(feature = "iroh")]
        if let Some(relay_stats) = &self.relay_stats {
            if let Ok(stats) = relay_stats.try_read() {
                // Current relay status table
                ui.collapsing("Current Status", |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("relay_status")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Status");
                                ui.strong("Relay");
                                ui.strong("Latency");
                                ui.strong("Success Rate");
                                ui.strong("Role");
                                ui.end_row();
                                
                                for (addr, stat) in stats.iter() {
                                    ui.label(stat.health_status().emoji());
                                    
                                    let short_addr = if addr.len() > 50 {
                                        format!("{}...", &addr[..47])
                                    } else {
                                        addr.clone()
                                    };
                                    ui.label(short_addr);
                                    
                                    if let Some(latency) = stat.latency_ms {
                                        ui.label(format!("{} ms", latency));
                                    } else {
                                        ui.label("Unknown");
                                    }
                                    
                                    ui.label(format!("{:.1}%", stat.success_rate()));
                                    
                                    if stat.is_home_relay {
                                        ui.label("🏠 Home");
                                    } else {
                                        ui.label("Backup");
                                    }
                                    
                                    ui.end_row();
                                }
                            });
                    });
                });
                
                // Latency plots
                ui.collapsing("Latency History", |ui| {
                    Plot::new("relay_latency_plot")
                        .height(300.0)
                        .y_axis_label("Latency (ms)")
                        .x_axis_label("Time")
                        .show(ui, |plot_ui| {
                            for (addr, history) in &self.latency_history {
                                if !history.is_empty() {
                                    let points: PlotPoints = history.iter()
                                        .map(|(time, latency)| [*time, *latency])
                                        .collect();
                                    
                                    // Get relay name for label
                                    let label = if addr.contains("relay.iroh.network") {
                                        "Iroh Public"
                                    } else if addr.len() > 20 {
                                        &addr[..20]
                                    } else {
                                        addr
                                    };
                                    
                                    plot_ui.line(Line::new(points).name(label));
                                }
                            }
                        });
                });
                
                // Success rate chart
                ui.collapsing("Success Rates", |ui| {
                    let bars: Vec<Bar> = stats.iter()
                        .enumerate()
                        .map(|(i, (addr, stat))| {
                            let label = if addr.len() > 15 {
                                format!("{}...", &addr[..12])
                            } else {
                                addr.clone()
                            };
                            
                            Bar::new(i as f64, stat.success_rate())
                                .name(label)
                                .width(0.8)
                        })
                        .collect();
                    
                    Plot::new("success_rate_chart")
                        .height(200.0)
                        .y_axis_label("Success Rate (%)")
                        .show(ui, |plot_ui| {
                            plot_ui.bar_chart(BarChart::new(bars));
                        });
                });
            }
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            ui.label("Network diagnostics not available in stub mode.");
        }
    }
    
    fn draw_connection_quality(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connection Quality Metrics");
        
        // Quality score indicator
        let quality_score = self.connection_quality.quality_score();
        let quality_color = match quality_score {
            s if s >= 0.8 => egui::Color32::from_rgb(0, 158, 115), // Green
            s if s >= 0.5 => egui::Color32::from_rgb(230, 159, 0), // Orange
            _ => egui::Color32::from_rgb(213, 94, 0), // Red
        };
        
        ui.horizontal(|ui| {
            ui.label("Overall Quality:");
            ui.add(egui::ProgressBar::new(quality_score)
                .fill(quality_color)
                .desired_width(200.0)
                .text(format!("{:.0}%", quality_score * 100.0)));
        });
        
        ui.add_space(10.0);
        
        // Detailed metrics
        egui::Grid::new("quality_metrics")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Metric");
                ui.strong("Value");
                ui.strong("Status");
                ui.end_row();
                
                // Latency
                ui.label("Average Latency");
                ui.label(format!("{:.1} ms", self.connection_quality.avg_latency));
                ui.label(if self.connection_quality.avg_latency < 100.0 { "🟢" } else if self.connection_quality.avg_latency < 200.0 { "🟠" } else { "🔴" });
                ui.end_row();
                
                ui.label("Min/Max Latency");
                ui.label(format!("{:.1} / {:.1} ms", 
                    if self.connection_quality.min_latency == f32::MAX { 0.0 } else { self.connection_quality.min_latency },
                    self.connection_quality.max_latency));
                ui.label("");
                ui.end_row();
                
                // Jitter
                ui.label("Jitter");
                ui.label(format!("{:.1} ms", self.connection_quality.jitter));
                ui.label(if self.connection_quality.jitter < 20.0 { "🟢" } else if self.connection_quality.jitter < 50.0 { "🟠" } else { "🔴" });
                ui.end_row();
                
                // Packet loss
                ui.label("Packet Loss");
                ui.label(format!("{:.1}%", self.connection_quality.packet_loss * 100.0));
                ui.label(if self.connection_quality.packet_loss < 0.01 { "🟢" } else if self.connection_quality.packet_loss < 0.05 { "🟠" } else { "🔴" });
                ui.end_row();
                
                // Stability
                ui.label("Connection Stability");
                ui.label(format!("{:.0}%", self.connection_quality.stability * 100.0));
                ui.label(if self.connection_quality.stability > 0.9 { "🟢" } else if self.connection_quality.stability > 0.7 { "🟠" } else { "🔴" });
                ui.end_row();
            });
        
        ui.add_space(10.0);
        
        // Recommendations
        ui.collapsing("Recommendations", |ui| {
            if self.connection_quality.avg_latency > 200.0 {
                ui.label("⚠️ High latency detected. Consider:");
                ui.label("  • Connecting to a closer relay");
                ui.label("  • Checking your internet connection");
            }
            
            if self.connection_quality.packet_loss > 0.05 {
                ui.label("⚠️ Significant packet loss detected. Consider:");
                ui.label("  • Switching from WiFi to wired connection");
                ui.label("  • Checking for network congestion");
            }
            
            if self.connection_quality.jitter > 50.0 {
                ui.label("⚠️ High jitter detected. This may cause:");
                ui.label("  • Inconsistent move timing");
                ui.label("  • Unpredictable game experience");
            }
            
            if quality_score >= 0.8 {
                ui.label("✅ Your connection quality is excellent!");
            }
        });
    }
    
    fn draw_game_network(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game Network Status");
        
        if self.game_connections.is_empty() {
            ui.label("No active game connections");
            return;
        }
        
        // Active games table
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("game_connections")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Game ID");
                    ui.strong("Peers");
                    ui.strong("Move Latency");
                    ui.strong("Sync Status");
                    ui.strong("Last Sync");
                    ui.end_row();
                    
                    for (_, game_info) in &self.game_connections {
                        // Shortened game ID
                        let short_id = if game_info.game_id.len() > 12 {
                            format!("{}...", &game_info.game_id[..12])
                        } else {
                            game_info.game_id.clone()
                        };
                        
                        ui.label(short_id);
                        ui.label(format!("{}", game_info.peer_count));
                        
                        if let Some(latency) = game_info.move_latency {
                            ui.label(format!("{:.0} ms", latency));
                        } else {
                            ui.label("--");
                        }
                        
                        if game_info.is_synced {
                            ui.label("✅ Synced");
                        } else {
                            ui.label("🔄 Syncing");
                        }
                        
                        let elapsed = game_info.last_sync.elapsed().as_secs();
                        ui.label(format!("{} sec ago", elapsed));
                        
                        ui.end_row();
                    }
                });
        });
        
        ui.add_space(10.0);
        
        // Game network stats summary
        ui.collapsing("Network Statistics", |ui| {
            let total_games = self.game_connections.len();
            let synced_games = self.game_connections.values().filter(|g| g.is_synced).count();
            let avg_move_latency = self.game_connections.values()
                .filter_map(|g| g.move_latency)
                .fold(0.0, |acc, l| acc + l) / total_games.max(1) as f32;
            
            ui.label(format!("Total active games: {}", total_games));
            ui.label(format!("Fully synced: {}/{}", synced_games, total_games));
            ui.label(format!("Average move latency: {:.0} ms", avg_move_latency));
        });
    }
    
    fn draw_troubleshooting(&mut self, ui: &mut egui::Ui) -> Option<crate::msg::UiToNet> {
        let mut action = None;
        
        ui.heading("Network Troubleshooting");
        
        // Connection test
        ui.group(|ui| {
            ui.heading("Connection Test");
            
            ui.horizontal(|ui| {
                let test_enabled = !self.connection_test_in_progress && 
                    (self.last_connection_test.is_none() || 
                     self.last_connection_test.unwrap().elapsed().as_secs() > 10);
                
                if ui.add_enabled(test_enabled, egui::Button::new("Run Connection Test")).clicked() {
                    self.connection_test_in_progress = true;
                    self.test_results.clear();
                    self.test_results.push("Starting connection test...".to_string());
                    action = Some(crate::msg::UiToNet::RunConnectionTest);
                }
                
                if self.connection_test_in_progress {
                    ui.spinner();
                }
            });
            
            if !self.test_results.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for result in &self.test_results {
                            ui.label(result);
                        }
                    });
            }
        });
        
        ui.add_space(10.0);
        
        // Diagnostic export
        ui.group(|ui| {
            ui.heading("Diagnostic Export");
            ui.label("Export network diagnostics for troubleshooting");
            
            if ui.button("Export Diagnostics").clicked() {
                // TODO: Implement diagnostic export
                ui.ctx().output_mut(|o| o.copied_text = self.generate_diagnostic_report());
            }
            
            ui.label("📋 Diagnostics copied to clipboard");
        });
        
        ui.add_space(10.0);
        
        // Common issues
        ui.collapsing("Common Issues & Solutions", |ui| {
            ui.strong("Can't connect to relay:");
            ui.label("• Check your firewall settings");
            ui.label("• Ensure port 4001 is not blocked");
            ui.label("• Try restarting the network");
            ui.add_space(5.0);
            
            ui.strong("High latency:");
            ui.label("• Connect to a geographically closer relay");
            ui.label("• Check for bandwidth-intensive applications");
            ui.label("• Consider using a wired connection");
            ui.add_space(5.0);
            
            ui.strong("Game sync issues:");
            ui.label("• Ensure all players have stable connections");
            ui.label("• Check if game persistence is enabled");
            ui.label("• Try rejoining the game");
        });
        
        action
    }
    
    fn generate_diagnostic_report(&self) -> String {
        let mut report = String::new();
        report.push_str("P2P Go Network Diagnostics Report\n");
        report.push_str("================================\n\n");
        
        // Network state
        report.push_str(&format!("Network State: {:?}\n", self.network_state));
        report.push_str(&format!("Is Relay Node: {}\n", self.is_relay_node));
        if let Some(port) = self.relay_port {
            report.push_str(&format!("Relay Port: {}\n", port));
        }
        
        // Connection quality
        report.push_str("\nConnection Quality:\n");
        report.push_str(&format!("  Average Latency: {:.1} ms\n", self.connection_quality.avg_latency));
        report.push_str(&format!("  Packet Loss: {:.1}%\n", self.connection_quality.packet_loss * 100.0));
        report.push_str(&format!("  Jitter: {:.1} ms\n", self.connection_quality.jitter));
        report.push_str(&format!("  Quality Score: {:.0}%\n", self.connection_quality.quality_score() * 100.0));
        
        // Game connections
        report.push_str(&format!("\nActive Games: {}\n", self.game_connections.len()));
        
        // Test results if any
        if !self.test_results.is_empty() {
            report.push_str("\nLast Connection Test:\n");
            for result in &self.test_results {
                report.push_str(&format!("  {}\n", result));
            }
        }
        
        report
    }
    
    /// Draw the advanced networking UI panel
    pub fn draw_advanced_panel(&mut self, ui: &mut egui::Ui) -> Option<crate::msg::UiToNet> {
        let mut action = None;
        
        egui::CollapsingHeader::new("Advanced Networking")
            .id_source("advanced_networking")
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                
                // Network status
                ui.heading("Network Status");
                ui.horizontal(|ui| {
                    self.draw_network_badge(ui);
                });
                ui.add_space(8.0);
                
                // Relay config
                ui.heading("Relay Configuration");
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    if self.is_relay_node {
                        ui.label("Self-Hosted Relay");
                    } else {
                        ui.label("Client (Using External Relay)");
                    }
                });
                
                if let Some(port) = self.relay_port {
                    ui.horizontal(|ui| {
                        ui.label("Relay Port:");
                        ui.monospace(port.to_string());
                    });
                }
                
                // Bootstrap relays
                ui.separator();
                ui.label("Bootstrap Relays (one per line):");
                let text_edit = ui.add_enabled(
                    !self.is_relay_node, 
                    egui::TextEdit::multiline(&mut self.bootstrap_relays)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Enter bootstrap relay addresses, one per line")
                );
                
                if text_edit.changed() {
                    // Validate addresses when changed
                }
                
                ui.horizontal(|ui| {
                    if ui.button("Apply Bootstrap Relays").clicked() {
                        // Generate config JSON
                        let config_json = format!(
                            r#"{{
                                "relay_mode": "CustomRelays",
                                "relay_addrs": [{}]
                            }}"#,
                            self.bootstrap_relays
                                .lines()
                                .filter(|l| !l.trim().is_empty())
                                .map(|a| format!("\"{}\"", a.trim()))
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        
                        action = Some(crate::msg::UiToNet::SaveConfigAndRestart {
                            config_json
                        });
                    }
                    
                    if ui.button("Restart Network").clicked() {
                        action = Some(crate::msg::UiToNet::RestartNetwork);
                    }
                });
            });
            
        action
    }
    
    fn get_overall_status(&self) -> (&'static str, egui::Color32) {
        // First check direct relay health status
        if let Some(status) = &self.relay_health_status {
            use p2pgo_network::relay_monitor::RelayHealthStatus;
            
            match status {
                RelayHealthStatus::Healthy => {
                    return ("🟢", egui::Color32::from_rgb(0, 158, 115)); // Okabe-Ito Green
                },
                RelayHealthStatus::Degraded => {
                    return ("🟠", egui::Color32::from_rgb(230, 159, 0)); // Okabe-Ito Orange
                },
                RelayHealthStatus::Unreachable => {
                    return ("🔴", egui::Color32::from_rgb(213, 94, 0)); // Okabe-Ito Vermillion
                },
                RelayHealthStatus::Restarting => {
                    return ("🔄", egui::Color32::from_rgb(0, 114, 178)); // Okabe-Ito Blue
                },
                RelayHealthStatus::Failed => {
                    return ("❌", egui::Color32::from_rgb(213, 94, 0)); // Okabe-Ito Vermillion
                },
            }
        }
        
        // Fall back to relay stats collection if direct status not available
        #[cfg(feature = "iroh")]
        if let Some(relay_stats) = &self.relay_stats {
            if let Ok(stats) = relay_stats.try_read() {
                let total = stats.len();
                let online = stats.values().filter(|s| s.is_reachable).count();
                
                if total == 0 {
                    // Bootstrap mode - no relays configured yet
                    return ("🟡", egui::Color32::from_rgb(240, 228, 66)); // Okabe-Ito Yellow
                } else if online == 0 {
                    // All relays unreachable
                    return ("🔴", egui::Color32::from_rgb(213, 94, 0)); // Okabe-Ito Vermillion
                } else if online == total {
                    // All relays healthy
                    return ("🟢", egui::Color32::from_rgb(0, 158, 115)); // Okabe-Ito Green
                } else {
                    // Some relays degraded
                    return ("🟠", egui::Color32::from_rgb(230, 159, 0)); // Okabe-Ito Orange
                }
            }
        }
        
        // Unknown/default state
        ("❓", egui::Color32::GRAY)
    }
    
    /// Update the relay health status
    #[cfg(feature = "iroh")]
    pub fn update_relay_health(&mut self, status: RelayHealthStatus, port: Option<u16>) {
        self.relay_health_status = Some(status.clone());
        self.relay_port = port;
        
        // Update network state based on relay health
        match status {
            RelayHealthStatus::Healthy => {
                if self.network_state == NetworkState::Syncing {
                    // Stay in syncing state until explicitly changed
                } else {
                    self.network_state = NetworkState::Online;
                }
            },
            RelayHealthStatus::Degraded => {
                self.network_state = NetworkState::Degraded;
            },
            RelayHealthStatus::Restarting => {
                self.network_state = NetworkState::StartingRelay;
                self.state_start_time = Instant::now();
            },
            RelayHealthStatus::Unreachable | RelayHealthStatus::Failed => {
                self.network_state = NetworkState::Offline;
            },
        }
    }
    
    /// Set relay node flag
    pub fn set_is_relay_node(&mut self, is_relay: bool) {
        self.is_relay_node = is_relay;
    }
    
    /// Update relay capacity information
    pub fn update_relay_capacity(&mut self, active_connections: usize, connection_limit: usize) {
        self.relay_active_connections = Some(active_connections);
        self.relay_connection_limit = Some(connection_limit);
    }
    
    /// Get the current network state
    pub fn network_state(&self) -> &NetworkState {
        &self.network_state
    }
    
    /// Set network state to syncing
    pub fn set_syncing(&mut self) {
        self.network_state = NetworkState::Syncing;
        self.state_start_time = Instant::now();
    }
    
    /// Check if this node is acting as a relay
    pub fn is_relay_node(&self) -> bool {
        self.is_relay_node
    }
    
    /// Set network state to online
    pub fn set_online(&mut self) {
        self.network_state = NetworkState::Online;
    }
    
    /// Get a color for the current network state
    pub fn network_state_color(&self) -> egui::Color32 {
        use p2pgo_core::color_constants::{network_status, f32_to_u8_rgb};
        
        let rgb = match self.network_state {
            NetworkState::Offline => network_status::OFFLINE,
            NetworkState::StartingRelay => network_status::CONNECTING,
            NetworkState::Syncing => network_status::SYNCING,
            NetworkState::Online => network_status::CONNECTED,
            NetworkState::Degraded => network_status::WARNING,
        };
        
        let rgb_u8 = f32_to_u8_rgb(rgb);
        egui::Color32::from_rgb(rgb_u8[0], rgb_u8[1], rgb_u8[2])
    }
    
    /// Get a descriptive message for the current network state
    pub fn network_state_message(&self) -> &'static str {
        match self.network_state {
            NetworkState::Offline => "Network offline",
            NetworkState::StartingRelay => "Starting relay...",
            NetworkState::Syncing => "Syncing with network...",
            NetworkState::Online => "Connected",
            NetworkState::Degraded => "Connected (degraded)",
        }
    }
    
    /// Draw a network status badge
    pub fn draw_network_badge(&self, ui: &mut egui::Ui) {
        let color = self.network_state_color();
        let message = self.network_state_message();
        
        // Draw a colored circle indicator
        let radius = 6.0;
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(radius * 2.0, radius * 2.0), egui::Sense::hover());
        ui.painter().circle_filled(
            rect.center(),
            radius,
            color,
        );
        
        // Add a small gap
        ui.add_space(4.0);
        
        // Show the port if available
        if let Some(port) = self.relay_port {
            let port_text = format!("{}:{}", message, port);
            ui.label(port_text);
        } else {
            ui.label(message);
        }
        
        // Show a spinner when starting or syncing
        match self.network_state {
            NetworkState::StartingRelay | NetworkState::Syncing => {
                ui.add_space(4.0);
                ui.spinner();
            },
            _ => {}
        }
    }
    
    /// Draw network status with tooltip
    pub fn draw_network_status_with_tooltip(&self, ui: &mut egui::Ui) {
        let response = ui.horizontal(|ui| {
            self.draw_network_badge(ui);
        });
        
        response.response.on_hover_ui(|ui| {
            ui.label(self.get_network_tooltip());
        });
    }
    
    /// Update game connection info
    pub fn update_game_connection(&mut self, game_id: String, peer_count: usize, is_synced: bool) {
        let entry = self.game_connections.entry(game_id.clone()).or_insert(GameConnectionInfo {
            game_id,
            peer_count,
            move_latency: None,
            last_sync: Instant::now(),
            is_synced,
        });
        
        entry.peer_count = peer_count;
        entry.is_synced = is_synced;
        entry.last_sync = Instant::now();
    }
    
    /// Update move latency for a game
    pub fn update_move_latency(&mut self, game_id: &str, latency: f32) {
        if let Some(game_info) = self.game_connections.get_mut(game_id) {
            game_info.move_latency = Some(latency);
            game_info.last_sync = Instant::now();
        }
    }
    
    /// Remove a game connection
    pub fn remove_game_connection(&mut self, game_id: &str) {
        self.game_connections.remove(game_id);
    }
    
    /// Update connection test results
    pub fn update_test_results(&mut self, results: Vec<String>) {
        self.test_results = results;
        self.connection_test_in_progress = false;
        self.last_connection_test = Some(Instant::now());
    }
    
    /// Get detailed tooltip for network status
    fn get_network_tooltip(&self) -> String {
        #[cfg(feature = "iroh")]
        if let Some(status) = &self.relay_health_status {
            let status_text = match status {
                RelayHealthStatus::Healthy => "healthy",
                RelayHealthStatus::Degraded => "degraded",
                RelayHealthStatus::Unreachable => "unreachable",
                RelayHealthStatus::Restarting => "restarting",
                RelayHealthStatus::Failed => "failed",
            };
            
            let mut tooltip = format!("Network Status: {}\n", status_text);
            
            if self.is_relay_node {
                tooltip.push_str("Mode: Self-Hosted Relay\n");
            } else {
                tooltip.push_str("Mode: Client\n");
            }
            
            if let Some(port) = self.relay_port {
                tooltip.push_str(&format!("Port: {}\n", port));
            }
            
            if self.network_state == NetworkState::StartingRelay || self.network_state == NetworkState::Syncing {
                let elapsed = self.state_start_time.elapsed();
                tooltip.push_str(&format!("Time in state: {:.1}s", elapsed.as_secs_f32()));
            }
            
            return tooltip;
        }
        
        "Network Status: Unknown".to_string()
    }
}

/// Simple connection graph visualization
struct ConnectionGraph {
    graph: Graph<String, ConnectionType, Directed>,
    node_indices: HashMap<String, NodeIndex>,
    layout: HashMap<NodeIndex, egui::Pos2>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionType {
    Direct,
    Relayed,
    Connecting,
}

impl ConnectionGraph {
    fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_indices: HashMap::new(),
            layout: HashMap::new(),
        }
    }
    
    fn add_node(&mut self, id: &str) {
        if !self.node_indices.contains_key(id) {
            let idx = self.graph.add_node(id.to_string());
            self.node_indices.insert(id.to_string(), idx);
            
            // Simple circular layout
            let angle = self.node_indices.len() as f32 * 2.0 * std::f32::consts::PI / 8.0;
            let radius = 80.0;
            self.layout.insert(
                idx, 
                egui::pos2(
                    150.0 + radius * angle.cos(), 
                    150.0 + radius * angle.sin()
                )
            );
        }
    }
    
    #[allow(dead_code)]
    fn add_connection(&mut self, from: &str, to: &str, conn_type: ConnectionType) {
        self.add_node(from);
        self.add_node(to);
        
        let from_idx = self.node_indices[from];
        let to_idx = self.node_indices[to];
        
        // Remove any existing edge
        if let Some(edge_idx) = self.graph.find_edge(from_idx, to_idx) {
            self.graph.remove_edge(edge_idx);
        }
        
        self.graph.add_edge(from_idx, to_idx, conn_type);
    }
    
    fn show(&self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(300.0, 300.0), 
            egui::Sense::hover()
        );
        
        let rect = response.rect;
        
        // Draw a simple placeholder for now
        painter.rect_filled(
            rect,
            egui::Rounding::same(4.0),
            egui::Color32::from_gray(40)
        );
        
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Connection Graph\n(Coming Soon)",
            egui::FontId::default(),
            egui::Color32::WHITE
        );
        
        // TODO: Implement actual graph visualization
        // For now, just show a placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_panel_creation() {
        let panel = NetworkPanel::new();
        assert!(!panel.is_visible());
    }
    
    #[test]
    fn test_visibility_toggle() {
        let mut panel = NetworkPanel::new();
        assert!(!panel.is_visible());
        
        panel.toggle_visibility();
        assert!(panel.is_visible());
        
        panel.toggle_visibility();
        assert!(!panel.is_visible());
    }
}
