// SPDX-License-Identifier: MIT OR Apache-2.0

//! Network diagnostics panel for P2P Go

use std::collections::HashMap;
use std::time::Instant;
use eframe::egui;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Directed;

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
                
                // Update latency history
                for (addr, stat) in stats.iter() {
                    if let Some(latency) = stat.latency_ms {
                        let entry = self.latency_history.entry(addr.clone()).or_insert_with(Vec::new);
                        entry.push((elapsed, latency as f64));
                        
                        // Keep only recent history (last 5 minutes)
                        while entry.len() > 1 && entry[0].0 < elapsed - 300.0 {
                            entry.remove(0);
                        }
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
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }
        
        let mut visible = self.visible;
        egui::Window::new("Network Diagnostics")
            .open(&mut visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });
        self.visible = visible;
    }
    
    fn draw_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.show_relay_details, false, "Overview");
            ui.selectable_value(&mut self.show_relay_details, true, "Relay Details");
        });
        
        ui.separator();
        
        if self.show_relay_details {
            self.draw_relay_details(ui);
        } else {
            self.draw_overview(ui);
        }
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
