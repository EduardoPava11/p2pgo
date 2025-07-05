use eframe::egui::{self, Color32, RichText};
use std::time::Instant;

/// Bootstrap status display for network initialization
pub struct BootstrapStatus {
    /// Current bootstrap phase
    phase: BootstrapPhase,
    /// Status messages
    messages: Vec<StatusMessage>,
    /// Start time
    start_time: Instant,
    /// Network addresses
    addresses: Vec<String>,
    /// Peer ID
    peer_id: Option<String>,
    /// NAT status
    nat_status: NATStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootstrapPhase {
    NotStarted,
    Initializing,
    DetectingNAT,
    ConnectingBootstrap,
    SetupRelay,
    Ready,
    Failed,
}

#[derive(Clone)]
pub struct StatusMessage {
    pub timestamp: Instant,
    pub level: MessageLevel,
    pub text: String,
}

#[derive(Clone, Copy)]
pub enum MessageLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, PartialEq)]
pub enum NATStatus {
    Unknown,
    Open,
    Restricted,
    Symmetric,
    UPnPSuccess,
}

impl BootstrapStatus {
    pub fn new() -> Self {
        Self {
            phase: BootstrapPhase::NotStarted,
            messages: Vec::new(),
            start_time: Instant::now(),
            addresses: Vec::new(),
            peer_id: None,
            nat_status: NATStatus::Unknown,
        }
    }

    /// Update bootstrap phase
    pub fn set_phase(&mut self, phase: BootstrapPhase) {
        self.phase = phase;
        self.add_message(MessageLevel::Info, format!("Entering phase: {:?}", phase));
    }

    /// Add status message
    pub fn add_message(&mut self, level: MessageLevel, text: String) {
        self.messages.push(StatusMessage {
            timestamp: Instant::now(),
            level,
            text,
        });

        // Keep only last 20 messages
        if self.messages.len() > 20 {
            self.messages.remove(0);
        }
    }

    /// Set network info
    pub fn set_network_info(&mut self, peer_id: String, addresses: Vec<String>) {
        self.peer_id = Some(peer_id);
        self.addresses = addresses;
    }

    /// Set NAT status
    pub fn set_nat_status(&mut self, status: NATStatus) {
        self.nat_status = status;
    }

    /// Render the bootstrap status UI
    pub fn render(&self, ui: &mut egui::Ui) {
        ui.heading("🌐 Network Bootstrap Status");
        ui.separator();

        // Phase indicator
        self.render_phase_indicator(ui);

        ui.separator();

        // Network info
        if self.phase == BootstrapPhase::Ready {
            self.render_network_info(ui);
            ui.separator();
        }

        // Status messages
        self.render_status_messages(ui);

        // Instructions
        if self.phase == BootstrapPhase::Failed {
            ui.separator();
            self.render_troubleshooting(ui);
        }
    }

    fn render_phase_indicator(&self, ui: &mut egui::Ui) {
        let phases = [
            ("Initialize", BootstrapPhase::Initializing),
            ("Detect NAT", BootstrapPhase::DetectingNAT),
            ("Connect", BootstrapPhase::ConnectingBootstrap),
            ("Setup Relay", BootstrapPhase::SetupRelay),
            ("Ready", BootstrapPhase::Ready),
        ];

        ui.horizontal(|ui| {
            for (i, (name, phase)) in phases.iter().enumerate() {
                let is_current = *phase == self.phase;
                let is_complete = (*phase as u8) < (self.phase as u8);
                let is_failed = self.phase == BootstrapPhase::Failed;

                // Phase circle
                let color = if is_failed {
                    Color32::RED
                } else if is_complete {
                    Color32::from_rgb(0, 200, 0)
                } else if is_current {
                    Color32::from_rgb(255, 200, 0)
                } else {
                    Color32::from_gray(100)
                };

                ui.colored_label(color, if is_complete { "✓" } else { "○" });
                ui.label(*name);

                // Connector line
                if i < phases.len() - 1 {
                    ui.label("―");
                }
            }
        });

        // Elapsed time
        let elapsed = self.start_time.elapsed();
        ui.label(format!("Elapsed: {:.1}s", elapsed.as_secs_f32()));
    }

    fn render_network_info(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("Network Information").strong());

            egui::Grid::new("network_info").show(ui, |ui| {
                // Peer ID
                ui.label("Peer ID:");
                if let Some(peer_id) = &self.peer_id {
                    ui.horizontal(|ui| {
                        let short_id = if peer_id.len() > 16 {
                            format!("{}...{}", &peer_id[..8], &peer_id[peer_id.len() - 4..])
                        } else {
                            peer_id.clone()
                        };
                        ui.monospace(&short_id);

                        if ui.button("📋").on_hover_text("Copy full ID").clicked() {
                            ui.output_mut(|o| o.copied_text = peer_id.clone());
                        }
                    });
                }
                ui.end_row();

                // NAT Status
                ui.label("NAT Status:");
                let (nat_text, nat_color) = match self.nat_status {
                    NATStatus::Open => ("Open (Direct connections possible)", Color32::GREEN),
                    NATStatus::UPnPSuccess => ("UPnP configured", Color32::GREEN),
                    NATStatus::Restricted => ("Restricted (Using relay)", Color32::YELLOW),
                    NATStatus::Symmetric => {
                        ("Symmetric (Relay required)", Color32::from_rgb(255, 150, 0))
                    }
                    NATStatus::Unknown => ("Unknown", Color32::GRAY),
                };
                ui.colored_label(nat_color, nat_text);
                ui.end_row();

                // Listening addresses
                ui.label("Addresses:");
                ui.vertical(|ui| {
                    for addr in &self.addresses {
                        ui.horizontal(|ui| {
                            ui.monospace(addr);
                            if ui.button("📋").on_hover_text("Copy address").clicked() {
                                ui.output_mut(|o| o.copied_text = addr.clone());
                            }
                        });
                    }
                });
                ui.end_row();
            });
        });
    }

    fn render_status_messages(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Status Log").strong());

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for msg in &self.messages {
                    let color = match msg.level {
                        MessageLevel::Info => Color32::GRAY,
                        MessageLevel::Success => Color32::GREEN,
                        MessageLevel::Warning => Color32::YELLOW,
                        MessageLevel::Error => Color32::RED,
                    };

                    let elapsed = msg.timestamp.duration_since(self.start_time).as_secs_f32();
                    ui.horizontal(|ui| {
                        ui.monospace(format!("[{:>5.1}s]", elapsed));
                        ui.colored_label(color, &msg.text);
                    });
                }
            });
    }

    fn render_troubleshooting(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(RichText::new("⚠️ Troubleshooting").color(Color32::YELLOW));

            ui.label("Bootstrap failed. Try these steps:");
            ui.label("• Check your internet connection");
            ui.label("• Ensure firewall allows the application");
            ui.label("• Try using a direct connection address from another player");
            ui.label("• If behind strict NAT, ask another player to host");

            if ui.button("Retry Bootstrap").clicked() {
                // This will be handled by the parent component
            }
        });
    }
}
