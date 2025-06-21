// SPDX-License-Identifier: MIT OR Apache-2.0

//! Network diagnostics panel for P2P Go

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use eframe::egui;
use egui_plot::{Plot, Line, PlotPoints, Bar, BarChart};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Directed;

#[cfg(feature = "iroh")]
use p2pgo_network::relay_monitor::{RelayStats, RelayHealth};

/// Network diagnostics panel
pub struct NetworkPanel {
    #[cfg(feature = "iroh")]
    relay_stats: Option<Arc<tokio::sync::RwLock<HashMap<String, RelayStats>>>>,
    
    // UI state
    visible: bool,
    show_relay_details: bool,
    latency_history: HashMap<String, Vec<(f64, f64)>>, // (time, latency) pairs
    last_update: Instant,
    
    // Connection graph
    connection_graph: ConnectionGraph,
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
            
            visible: false,
            show_relay_details: false,
            latency_history: HashMap::new(),
            last_update: Instant::now(),
            connection_graph: ConnectionGraph::new(),
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
    pub fn update_stats(&mut self, ctx: &egui::Context) {
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
                ctx.request_repaint();
            }
        }
    }
    
    /// Draw the network status badge in the top bar
    pub fn draw_status_badge(&mut self, ui: &mut egui::Ui) {
        let (status_text, status_color) = self.get_overall_status();
        
        let button = egui::Button::new(format!("Relay: {}", status_text))
            .fill(status_color)
            .rounding(egui::Rounding::same(4.0));
        
        if ui.add(button).clicked() {
            self.toggle_visibility();
        }
        
        // Tooltip with quick stats
        if ui.response().hovered() {
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
        
        egui::Window::new("Network Diagnostics")
            .open(&mut self.visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });
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
    
    fn get_overall_status(&self) -> (&'static str, egui::Color32) {
        #[cfg(feature = "iroh")]
        if let Some(relay_stats) = &self.relay_stats {
            if let Ok(stats) = relay_stats.try_read() {
                let total = stats.len();
                let online = stats.values().filter(|s| s.is_reachable).count();
                
                if total == 0 {
                    return ("No relays", egui::Color32::GRAY);
                } else if online == 0 {
                    return ("❌", egui::Color32::RED);
                } else if online == total {
                    return ("✅", egui::Color32::GREEN);
                } else {
                    return ("⚠️", egui::Color32::YELLOW);
                }
            }
        }
        
        ("Unknown", egui::Color32::GRAY)
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
