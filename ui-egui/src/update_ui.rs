// SPDX-License-Identifier: MIT OR Apache-2.0

//! UI components for update notifications and dialogs

use eframe::egui;
use crate::update_checker::{UpdateCheckResult, UpdateInfo, Version};

/// Update notification state
#[derive(Debug, Clone)]
pub struct UpdateNotification {
    /// Result of the update check
    pub result: UpdateCheckResult,
    
    /// Whether the notification is shown
    pub visible: bool,
    
    /// Whether the user has dismissed this notification
    pub dismissed: bool,
    
    /// Whether to show detailed release notes
    pub show_details: bool,
}

impl UpdateNotification {
    /// Create a new update notification
    pub fn new(result: UpdateCheckResult) -> Self {
        Self {
            visible: result.update_available,
            result,
            dismissed: false,
            show_details: false,
        }
    }
    
    /// Show the update notification UI
    pub fn show(&mut self, ctx: &egui::Context) -> UpdateAction {
        if !self.visible || self.dismissed {
            return UpdateAction::None;
        }
        
        let mut action = UpdateAction::None;
        
        // Determine notification style based on whether update is required
        let (title, icon, style) = if self.result.update_required {
            ("Required Update Available", "⚠️", NotificationStyle::Critical)
        } else {
            ("Update Available", "🔄", NotificationStyle::Info)
        };
        
        // Position the notification at top-right corner
        let screen_rect = ctx.screen_rect();
        let notification_width = 350.0;
        let notification_pos = egui::pos2(
            screen_rect.max.x - notification_width - 20.0,
            screen_rect.min.y + 20.0,
        );
        
        egui::Window::new("update_notification")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(notification_pos)
            .fixed_size(egui::vec2(notification_width, 0.0))
            .show(ctx, |ui| {
                // Apply styling based on notification type
                match style {
                    NotificationStyle::Critical => {
                        ui.visuals_mut().widgets.noninteractive.bg_fill = 
                            egui::Color32::from_rgb(50, 20, 20);
                    }
                    NotificationStyle::Info => {
                        ui.visuals_mut().widgets.noninteractive.bg_fill = 
                            egui::Color32::from_rgb(20, 30, 50);
                    }
                }
                
                ui.vertical(|ui| {
                    // Header
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{} {}", icon, title))
                            .size(16.0)
                            .strong());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.result.update_required && ui.small_button("✕").clicked() {
                                self.dismissed = true;
                            }
                        });
                    });
                    
                    ui.add_space(5.0);
                    
                    // Version info
                    if let Some(ref latest) = self.result.latest_version {
                        ui.horizontal(|ui| {
                            ui.label("Current version:");
                            ui.label(egui::RichText::new(self.result.current_version.to_string())
                                .monospace());
                        });
                        ui.horizontal(|ui| {
                            ui.label("Latest version:");
                            ui.label(egui::RichText::new(latest.to_string())
                                .monospace()
                                .color(egui::Color32::from_rgb(100, 200, 100)));
                        });
                    }
                    
                    // Announcement
                    if let Some(ref announcement) = self.result.announcement {
                        ui.add_space(5.0);
                        ui.separator();
                        ui.add_space(5.0);
                        ui.label(announcement);
                    }
                    
                    // Update info
                    if let Some(ref update_info) = self.result.update_info {
                        ui.add_space(5.0);
                        
                        // Show release notes toggle
                        if ui.link("View release notes").clicked() {
                            self.show_details = !self.show_details;
                        }
                        
                        if self.show_details {
                            ui.add_space(5.0);
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    ui.label(&update_info.release_notes);
                                });
                        }
                        
                        // Update size info
                        if update_info.size > 0 {
                            ui.add_space(5.0);
                            let size_mb = update_info.size as f64 / 1_048_576.0;
                            ui.label(format!("Download size: {:.1} MB", size_mb));
                        }
                    }
                    
                    // Action buttons
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);
                    
                    ui.horizontal(|ui| {
                        if self.result.update_required {
                            if ui.button("Update Now").clicked() {
                                action = UpdateAction::UpdateNow;
                            }
                            ui.label("This update is required to continue using the app.");
                        } else {
                            if ui.button("Update Now").clicked() {
                                action = UpdateAction::UpdateNow;
                            }
                            if ui.button("Remind Me Later").clicked() {
                                action = UpdateAction::RemindLater;
                                self.dismissed = true;
                            }
                            if ui.button("Skip This Version").clicked() {
                                action = UpdateAction::SkipVersion;
                                self.dismissed = true;
                            }
                        }
                    });
                });
            });
        
        action
    }
}

/// Actions that can be taken from the update notification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateAction {
    /// No action taken
    None,
    /// User wants to update now
    UpdateNow,
    /// User wants to be reminded later
    RemindLater,
    /// User wants to skip this version
    SkipVersion,
}

/// Notification visual style
#[derive(Debug, Clone, Copy)]
enum NotificationStyle {
    Info,
    Critical,
}

/// Update dialog for showing update progress
pub struct UpdateDialog {
    /// Current state of the update process
    pub state: UpdateState,
    
    /// Progress value (0.0 to 1.0)
    pub progress: f32,
    
    /// Status message
    pub status_message: String,
    
    /// Error message if any
    pub error: Option<String>,
}

/// States of the update process
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateState {
    /// Checking for updates
    Checking,
    /// Downloading update
    Downloading,
    /// Verifying download
    Verifying,
    /// Ready to install
    ReadyToInstall,
    /// Installing update
    Installing,
    /// Update complete
    Complete,
    /// Update failed
    Failed,
}

impl UpdateDialog {
    /// Create a new update dialog
    pub fn new() -> Self {
        Self {
            state: UpdateState::Checking,
            progress: 0.0,
            status_message: "Checking for updates...".to_string(),
            error: None,
        }
    }
    
    /// Show the update dialog
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut keep_open = true;
        
        egui::Window::new("Update Progress")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                
                ui.vertical(|ui| {
                    // Show current status
                    ui.heading(&self.status_message);
                    
                    // Show progress bar for applicable states
                    match self.state {
                        UpdateState::Downloading | UpdateState::Verifying | UpdateState::Installing => {
                            ui.add_space(10.0);
                            ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                        }
                        _ => {}
                    }
                    
                    // Show error if any
                    if let Some(ref error) = self.error {
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::from_rgb(200, 50, 50), error);
                    }
                    
                    // Action buttons based on state
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    match self.state {
                        UpdateState::Failed => {
                            ui.horizontal(|ui| {
                                if ui.button("Retry").clicked() {
                                    self.state = UpdateState::Checking;
                                    self.error = None;
                                    self.progress = 0.0;
                                }
                                if ui.button("Close").clicked() {
                                    keep_open = false;
                                }
                            });
                        }
                        UpdateState::Complete => {
                            ui.label("Update complete! Please restart the application.");
                            if ui.button("Restart Now").clicked() {
                                // Would trigger app restart
                                keep_open = false;
                            }
                        }
                        UpdateState::ReadyToInstall => {
                            ui.label("Update downloaded and verified. Ready to install.");
                            ui.horizontal(|ui| {
                                if ui.button("Install Now").clicked() {
                                    self.state = UpdateState::Installing;
                                    self.status_message = "Installing update...".to_string();
                                }
                                if ui.button("Install Later").clicked() {
                                    keep_open = false;
                                }
                            });
                        }
                        _ => {
                            if ui.button("Cancel").clicked() {
                                keep_open = false;
                            }
                        }
                    }
                });
            });
        
        keep_open
    }
    
    /// Update the dialog state
    pub fn set_state(&mut self, state: UpdateState, message: String) {
        self.state = state;
        self.status_message = message;
        
        // Reset progress for new states
        match state {
            UpdateState::Checking | UpdateState::Downloading | UpdateState::Verifying => {
                self.progress = 0.0;
            }
            _ => {}
        }
    }
    
    /// Set progress value
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }
    
    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.state = UpdateState::Failed;
        self.status_message = "Update failed".to_string();
    }
}

/// Helper function to show a simple update available banner
pub fn show_update_banner(ui: &mut egui::Ui, version: &Version) -> bool {
    let mut clicked = false;
    
    ui.horizontal(|ui| {
        ui.visuals_mut().widgets.noninteractive.bg_fill = 
            egui::Color32::from_rgb(30, 40, 60);
        
        egui::Frame::none()
            .inner_margin(egui::vec2(10.0, 5.0))
            .fill(egui::Color32::from_rgb(30, 40, 60))
            .show(ui, |ui| {
                ui.label("🔄");
                ui.label(format!("Version {} is available", version.to_string()));
                if ui.link("Update now").clicked() {
                    clicked = true;
                }
            });
    });
    
    clicked
}