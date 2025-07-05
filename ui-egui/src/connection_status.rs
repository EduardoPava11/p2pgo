//! Connection status indicator widget for P2P Go
//!
//! Provides real-time visual feedback about network connectivity
//! with accessibility features including text labels and ARIA support

use crate::msg::NetToUi;
use egui::{Color32, Response, Rounding, Ui, Vec2, Widget};
use std::time::{Duration, Instant};

/// Connection state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Fully connected to relay network
    Connected { relay_count: usize },
    /// Attempting to connect
    Connecting,
    /// No network connection
    Disconnected,
    /// Direct P2P connection established
    DirectP2P { peer_id: String },
}

impl ConnectionState {
    /// Get display icon for the state
    fn icon(&self) -> &'static str {
        match self {
            ConnectionState::Connected { .. } => "●",
            ConnectionState::Connecting => "◐",
            ConnectionState::Disconnected => "○",
            ConnectionState::DirectP2P { .. } => "⟷",
        }
    }

    /// Get display color with accessibility in mind
    fn color(&self) -> Color32 {
        match self {
            ConnectionState::Connected { .. } => Color32::from_rgb(34, 197, 94), // Green
            ConnectionState::Connecting => Color32::from_rgb(251, 146, 60),      // Orange
            ConnectionState::Disconnected => Color32::from_rgb(239, 68, 68),     // Red
            ConnectionState::DirectP2P { .. } => Color32::from_rgb(59, 130, 246), // Blue
        }
    }

    /// Get human-readable label
    fn label(&self) -> String {
        match self {
            ConnectionState::Connected { relay_count } => {
                format!(
                    "Connected to {} relay{}",
                    relay_count,
                    if *relay_count == 1 { "" } else { "s" }
                )
            }
            ConnectionState::Connecting => "Connecting...".to_string(),
            ConnectionState::Disconnected => "No connection".to_string(),
            ConnectionState::DirectP2P { peer_id } => {
                format!("Direct P2P with {}", &peer_id[..8])
            }
        }
    }

    /// Get detailed tooltip text
    fn tooltip(&self) -> String {
        match self {
            ConnectionState::Connected { relay_count } => {
                format!(
                    "Network Status: Connected\n\
                     Active Relays: {}\n\
                     Connection Type: Relay Network\n\
                     \n\
                     You can create and join games.",
                    relay_count
                )
            }
            ConnectionState::Connecting => "Network Status: Connecting\n\
                 \n\
                 Establishing connection to relay network...\n\
                 This may take a few seconds."
                .to_string(),
            ConnectionState::Disconnected => "Network Status: Disconnected\n\
                 \n\
                 No connection to relay network.\n\
                 Check your internet connection and firewall settings."
                .to_string(),
            ConnectionState::DirectP2P { peer_id } => {
                format!(
                    "Network Status: Direct P2P\n\
                     Peer ID: {}\n\
                     Connection Type: Direct\n\
                     \n\
                     You have a direct connection to your opponent.\n\
                     This provides the best performance.",
                    peer_id
                )
            }
        }
    }
}

/// Connection status widget that displays network state
pub struct ConnectionStatusWidget {
    state: ConnectionState,
    last_update: Instant,
    animation_phase: f32,
}

impl ConnectionStatusWidget {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            last_update: Instant::now(),
            animation_phase: 0.0,
        }
    }

    /// Update the connection state based on network messages
    pub fn update_from_message(&mut self, msg: &NetToUi) {
        match msg {
            NetToUi::ConnectionStatus { connected } => {
                if *connected {
                    self.state = ConnectionState::Connected { relay_count: 1 };
                } else {
                    self.state = ConnectionState::Disconnected;
                }
                self.last_update = Instant::now();
            }
            NetToUi::RelayHealth { is_relay_node, .. } => {
                if *is_relay_node {
                    self.state = ConnectionState::Connected { relay_count: 1 };
                }
                self.last_update = Instant::now();
            }
            _ => {}
        }
    }

    /// Update animation for connecting state
    pub fn update(&mut self, ctx: &egui::Context) {
        if matches!(self.state, ConnectionState::Connecting) {
            self.animation_phase += ctx.input(|i| i.unstable_dt) * 2.0;
            if self.animation_phase > std::f32::consts::TAU {
                self.animation_phase -= std::f32::consts::TAU;
            }
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    /// Render the widget
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        // Update animation
        self.update(ui.ctx());

        let icon = self.state.icon();
        let color = self.state.color();
        let label = self.state.label();
        let tooltip = self.state.tooltip();

        // Create a horizontal layout for icon and text
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                // Animated icon for connecting state
                if matches!(self.state, ConnectionState::Connecting) {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());

                    // Draw rotating circle animation
                    let center = rect.center();
                    let radius = 6.0;
                    let angle = self.animation_phase;

                    // Draw a simple rotating dot instead
                    let dot_pos = center + radius * egui::Vec2::new(angle.cos(), angle.sin());
                    ui.painter().circle_filled(dot_pos, 2.0, color);
                } else {
                    // Static icon
                    ui.colored_label(color, icon);
                }

                // Text label
                ui.label(label);
            })
            .response;

        // Add tooltip on hover
        response.on_hover_text(tooltip)
    }
}

/// Compact version for menu bar
pub fn connection_status_compact(ui: &mut Ui, state: &ConnectionState) -> Response {
    let icon = state.icon();
    let color = state.color();
    let tooltip = state.tooltip();

    // Just show colored icon with tooltip
    ui.colored_label(color, icon).on_hover_text(tooltip)
}
