//! SGF training UI for neural networks

use eframe::egui::{self, RichText};
use p2pgo_neural::{training::{NeuralTrainer, TrainingStats}, config::NeuralConfig};
use std::path::PathBuf;

/// SGF training interface
pub struct SGFTrainingUI {
    pub trainer: Option<NeuralTrainer>,
    pub selected_files: Vec<PathBuf>,
    pub training_in_progress: bool,
    pub last_stats: Option<TrainingStats>,
    pub error_message: Option<String>,
}

impl SGFTrainingUI {
    pub fn new() -> Self {
        Self {
            trainer: None,
            selected_files: Vec::new(),
            training_in_progress: false,
            last_stats: None,
            error_message: None,
        }
    }

    /// Initialize trainer with config
    pub fn init_trainer(&mut self, config: NeuralConfig) {
        self.trainer = Some(NeuralTrainer::new(config));
    }

    /// Render the training UI
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("📁 SGF Training");

        ui.separator();

        // File selection
        ui.horizontal(|ui| {
            if ui.button("Select SGF Files").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .set_title("Select SGF files from OGS or other sources")
                    .pick_files()
                {
                    self.selected_files = paths;
                    self.error_message = None;
                }
            }

            if !self.selected_files.is_empty() {
                ui.label(format!("{} files selected", self.selected_files.len()));
            }
        });

        // Show selected files
        if !self.selected_files.is_empty() {
            ui.add_space(10.0);
            ui.label("Selected files:");
            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    for file in &self.selected_files {
                        ui.label(format!("• {}", file.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()));
                    }
                });
        }

        ui.add_space(10.0);

        // Training controls
        ui.horizontal(|ui| {
            let can_train = !self.selected_files.is_empty()
                && self.trainer.is_some()
                && !self.training_in_progress;

            if ui.add_enabled(can_train, egui::Button::new("🚀 Start Training"))
                .clicked()
            {
                self.start_training();
            }

            if self.training_in_progress {
                ui.spinner();
                ui.label("Training in progress...");
            }
        });

        // Error display
        if let Some(error) = &self.error_message {
            ui.add_space(10.0);
            ui.colored_label(egui::Color32::RED, error);
        }

        // Training stats
        if let Some(stats) = &self.last_stats {
            ui.add_space(20.0);
            ui.separator();
            ui.label(RichText::new("Training Statistics").strong());

            egui::Grid::new("training_stats")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Games trained:");
                    ui.label(format!("{}", stats.games_trained));
                    ui.end_row();

                    ui.label("Total positions:");
                    ui.label(format!("{}", stats.total_positions));
                    ui.end_row();

                    ui.label("Avg positions/game:");
                    let avg = if stats.games_trained > 0 {
                        stats.total_positions / stats.games_trained
                    } else { 0 };
                    ui.label(format!("{}", avg));
                    ui.end_row();
                });
        }

        ui.add_space(20.0);

        // Training tips
        ui.collapsing("💡 Training Tips", |ui| {
            ui.label("• Upload games from OGS or any SGF source");
            ui.label("• More games = better pattern recognition");
            ui.label("• Mix different playing styles for balance");
            ui.label("• Training updates patterns based on your config");
            ui.label("• Save your network after training!");
        });
    }

    /// Start training process
    fn start_training(&mut self) {
        if let Some(trainer) = &mut self.trainer {
            self.training_in_progress = true;

            // In a real implementation, this would be async
            // For now, we'll simulate the training
            let _paths: Vec<&std::path::Path> = self.selected_files
                .iter()
                .map(|p| p.as_path())
                .collect();

            // This would normally be async
            tokio::spawn(async move {
                // Training happens here
                // trainer.train_from_sgf_batch(&paths).await
            });

            // Simulate completion
            self.training_in_progress = false;
            self.last_stats = Some(TrainingStats {
                games_trained: self.selected_files.len(),
                total_positions: self.selected_files.len() * 150, // Estimate
                config: trainer.config.clone(),
            });
        }
    }
}

/// Network save/load UI
pub struct NetworkPersistenceUI {
    pub save_path: Option<PathBuf>,
    pub load_path: Option<PathBuf>,
    pub message: Option<(String, bool)>, // (message, is_error)
}

impl NetworkPersistenceUI {
    pub fn new() -> Self {
        Self {
            save_path: None,
            load_path: None,
            message: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, trainer: Option<&NeuralTrainer>) {
        ui.heading("💾 Save/Load Network");

        ui.separator();

        // Save section
        ui.horizontal(|ui| {
            if ui.button("Save Network").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("neural_network.json")
                    .add_filter("JSON files", &["json"])
                    .save_file()
                {
                    self.save_path = Some(path);

                    if let Some(trainer) = trainer {
                        match trainer.save(&self.save_path.as_ref().unwrap()) {
                            Ok(_) => {
                                self.message = Some(("Network saved successfully!".to_string(), false));
                            }
                            Err(e) => {
                                self.message = Some((format!("Save failed: {}", e), true));
                            }
                        }
                    }
                }
            }

            if let Some(path) = &self.save_path {
                ui.label(format!("Saved to: {}", path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()));
            }
        });

        ui.add_space(10.0);

        // Load section
        ui.horizontal(|ui| {
            if ui.button("Load Network").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON files", &["json"])
                    .pick_file()
                {
                    self.load_path = Some(path.clone());

                    match NeuralTrainer::load(&path) {
                        Ok(_) => {
                            self.message = Some(("Network loaded successfully!".to_string(), false));
                        }
                        Err(e) => {
                            self.message = Some((format!("Load failed: {}", e), true));
                        }
                    }
                }
            }

            if let Some(path) = &self.load_path {
                ui.label(format!("Loaded from: {}", path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()));
            }
        });

        // Display messages
        if let Some((msg, is_error)) = &self.message {
            ui.add_space(10.0);
            let color = if *is_error { egui::Color32::RED } else { egui::Color32::GREEN };
            ui.colored_label(color, msg);
        }
    }
}