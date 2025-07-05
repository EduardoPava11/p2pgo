//! Peer list display

use egui::{Color32, Ui};

/// Peer list display component
pub struct PeerList {
    peers: Vec<PeerInfo>,
}

#[derive(Clone)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub latency_ms: Option<u32>,
    pub connected: bool,
}

impl PeerList {
    pub fn new() -> Self {
        Self { peers: Vec::new() }
    }

    /// Update peer list
    pub fn update_peers(&mut self, peers: Vec<PeerInfo>) {
        self.peers = peers;
    }

    /// Render peer list
    pub fn render(&self, ui: &mut Ui) {
        ui.label(format!("Connected Peers ({})", self.peers.len()));

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for peer in &self.peers {
                    ui.horizontal(|ui| {
                        // Connection indicator
                        let color = if peer.connected {
                            Color32::GREEN
                        } else {
                            Color32::GRAY
                        };
                        ui.colored_label(color, "●");

                        // Peer name
                        ui.label(&peer.name);

                        // Latency
                        if let Some(latency) = peer.latency_ms {
                            ui.label(format!("{}ms", latency));
                        }
                    });
                }

                if self.peers.is_empty() {
                    ui.label("No peers connected");
                }
            });
    }
}
