//! SGF training interface with visualization

use crate::core::{primary_button, secondary_button, Card, Colors, Spacing};
use egui::{Color32, ProgressBar, RichText, Ui, Widget};
use p2pgo_neural::{config::NeuralConfig, training::TrainingStats};
use std::path::PathBuf;

pub struct TrainingView {
    pub selected_files: Vec<PathBuf>,
    pub training_progress: f32,
    pub training_active: bool,
    pub last_stats: Option<TrainingStats>,
    pub error_message: Option<String>,
    pub show_visualization: bool,
}

impl TrainingView {
    pub fn new() -> Self {
        Self {
            selected_files: Vec::new(),
            training_progress: 0.0,
            training_active: false,
            last_stats: None,
            error_message: None,
            show_visualization: false,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> TrainingAction {
        let mut action = TrainingAction::None;

        ui.vertical(|ui| {
            ui.heading("Neural Network Training");
            ui.label(
                RichText::new("Train your AI with SGF game files").color(Colors::TEXT_SECONDARY),
            );

            ui.add_space(Spacing::MD);

            // File selection card
            Card::new().show(ui, |ui| {
                ui.heading("📁 SGF File Selection");
                ui.separator();

                ui.horizontal(|ui| {
                    if primary_button("Select SGF Files (1-10)")
                        .enabled(!self.training_active)
                        .ui(ui)
                        .clicked()
                    {
                        action = TrainingAction::SelectFiles;
                    }

                    if !self.selected_files.is_empty() {
                        ui.label(format!("{} files selected", self.selected_files.len()));
                    }
                });

                if !self.selected_files.is_empty() {
                    ui.add_space(Spacing::SM);
                    ui.label("Selected files:");

                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            for file in &self.selected_files {
                                ui.horizontal(|ui| {
                                    ui.label("•");
                                    ui.label(
                                        file.file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "Unknown".to_string()),
                                    );
                                });
                            }
                        });
                }
            });

            ui.add_space(Spacing::MD);

            // Training controls
            Card::new().show(ui, |ui| {
                ui.heading("🚀 Training");
                ui.separator();

                if self.training_active {
                    ui.label("Training in progress...");
                    ui.add(ProgressBar::new(self.training_progress).show_percentage());

                    // Animated visualization
                    if self.show_visualization {
                        ui.add_space(Spacing::SM);
                        self.render_training_visualization(ui);
                    }

                    ui.add_space(Spacing::SM);
                    if secondary_button("Cancel Training").ui(ui).clicked() {
                        action = TrainingAction::CancelTraining;
                    }
                } else {
                    ui.horizontal(|ui| {
                        let can_train = !self.selected_files.is_empty();

                        if primary_button("Start Training")
                            .enabled(can_train)
                            .ui(ui)
                            .clicked()
                        {
                            action = TrainingAction::StartTraining;
                        }

                        ui.checkbox(&mut self.show_visualization, "Show visualization");
                    });

                    if self.selected_files.is_empty() {
                        ui.label(
                            RichText::new("Select SGF files to begin training")
                                .color(Colors::TEXT_SECONDARY)
                                .italics(),
                        );
                    }
                }

                // Error display
                if let Some(error) = &self.error_message {
                    ui.add_space(Spacing::SM);
                    ui.label(RichText::new(error).color(Colors::ERROR));
                }
            });

            // Statistics
            if let Some(stats) = &self.last_stats {
                ui.add_space(Spacing::MD);

                Card::new().show(ui, |ui| {
                    ui.heading("📊 Training Statistics");
                    ui.separator();

                    egui::Grid::new("training_stats")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Games trained:");
                            ui.label(RichText::new(format!("{}", stats.games_trained)).strong());
                            ui.end_row();

                            ui.label("Total positions:");
                            ui.label(RichText::new(format!("{}", stats.total_positions)).strong());
                            ui.end_row();

                            ui.label("Avg positions/game:");
                            let avg = if stats.games_trained > 0 {
                                stats.total_positions / stats.games_trained
                            } else {
                                0
                            };
                            ui.label(RichText::new(format!("{}", avg)).strong());
                            ui.end_row();

                            ui.label("Configuration:");
                            ui.label(RichText::new("Custom").strong());
                            ui.end_row();
                        });
                });
            }

            ui.add_space(Spacing::MD);

            // Tips
            Card::new().show(ui, |ui| {
                ui.collapsing("💡 Training Tips", |ui| {
                    ui.label("• Use games from strong players for better pattern recognition");
                    ui.label("• Mix different playing styles for a balanced AI");
                    ui.label("• More games = better understanding of positions");
                    ui.label("• Training updates patterns based on your style configuration");
                    ui.label("• Save your network after training to preserve progress");
                });
            });
        });

        action
    }

    fn render_training_visualization(&self, ui: &mut Ui) {
        let response = ui.allocate_response(
            egui::Vec2::new(ui.available_width(), 100.0),
            egui::Sense::hover(),
        );
        let rect = response.rect;

        let painter = ui.painter();

        // Background
        painter.rect_filled(rect, 4.0, Colors::SURFACE.linear_multiply(0.8));

        // Animated neural network connections
        let time = ui.ctx().frame_nr() as f32 * 0.02;

        // Draw simplified network layers
        let layer_count = 5;
        let layer_spacing = rect.width() / (layer_count as f32 + 1.0);

        for i in 0..layer_count {
            let x = rect.left() + layer_spacing * (i as f32 + 1.0);
            let nodes = match i {
                0 => 8, // Input
                4 => 2, // Output
                _ => 4, // Hidden
            };

            for j in 0..nodes {
                let y = rect.top() + rect.height() * (j as f32 + 1.0) / (nodes as f32 + 1.0);
                let pos = egui::Pos2::new(x, y);

                // Node glow based on training progress
                let activation = (time + i as f32 + j as f32).sin() * 0.5 + 0.5;
                let node_color =
                    Colors::NEURAL_POLICY.linear_multiply(activation * self.training_progress);

                painter.circle_filled(pos, 4.0, node_color);

                // Connections to next layer
                if i < layer_count - 1 {
                    let next_x = rect.left() + layer_spacing * (i as f32 + 2.0);
                    let next_nodes = if i == layer_count - 2 { 2 } else { 4 };

                    for k in 0..next_nodes {
                        let next_y = rect.top()
                            + rect.height() * (k as f32 + 1.0) / (next_nodes as f32 + 1.0);
                        let next_pos = egui::Pos2::new(next_x, next_y);

                        let weight = ((time * 2.0 + i as f32 + j as f32 + k as f32).sin() * 0.5
                            + 0.5)
                            * self.training_progress;
                        let edge_color = Colors::NEURAL_POLICY.linear_multiply(weight * 0.3);

                        painter.line_segment([pos, next_pos], egui::Stroke::new(1.0, edge_color));
                    }
                }
            }
        }

        // Progress text
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("Training: {:.0}%", self.training_progress * 100.0),
            egui::FontId::proportional(14.0),
            Colors::TEXT_PRIMARY,
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrainingAction {
    None,
    SelectFiles,
    StartTraining,
    CancelTraining,
}
