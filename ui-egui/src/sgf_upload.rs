use eframe::egui;
use p2pgo_core::{SGFParser, GameState};
use p2pgo_neural::TrainingData;
use std::path::PathBuf;

/// SGF upload tool for creating training data (mRNA)
pub struct SGFUploadTool {
    /// Selected file path
    file_path: Option<PathBuf>,
    /// SGF content
    sgf_content: Option<String>,
    /// Parsed game
    parsed_game: Option<GameState>,
    /// Move range selector
    move_range: (usize, usize),
    /// Error message
    error: Option<String>,
}

impl SGFUploadTool {
    pub fn new() -> Self {
        Self {
            file_path: None,
            sgf_content: None,
            parsed_game: None,
            move_range: (0, 361),
            error: None,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<TrainingData> {
        let mut training_data = None;

        ui.heading("📄 SGF Upload Tool");
        ui.separator();

        // File selector
        ui.horizontal(|ui| {
            ui.label("SGF File:");

            if ui.button("Browse...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .pick_file()
                {
                    self.file_path = Some(path);
                    self.load_sgf();
                }
            }

            if let Some(path) = &self.file_path {
                ui.label(path.file_name().unwrap().to_string_lossy());
            }
        });

        // Direct paste option
        ui.separator();
        ui.label("Or paste SGF content:");

        let mut sgf_text = self.sgf_content.clone().unwrap_or_default();
        if ui.text_edit_multiline(&mut sgf_text).changed() {
            self.sgf_content = Some(sgf_text.clone());
            if !sgf_text.is_empty() {
                self.parse_sgf_content(sgf_text);
            }
        }

        // Show parsed game info
        if let Some(game) = &self.parsed_game {
            ui.separator();
            ui.heading("Game Info");

            egui::Grid::new("game_info").show(ui, |ui| {
                ui.label("Total moves:");
                ui.label(format!("{}", game.moves.len()));
                ui.end_row();

                ui.label("Board size:");
                ui.label(format!("{}x{}", game.board_size, game.board_size));
                ui.end_row();

                if let Some(result) = &game.result {
                    ui.label("Result:");
                    ui.label(format!("{:?}", result));
                    ui.end_row();
                }
            });

            // Move range selector
            ui.separator();
            ui.heading("Select Move Range");

            ui.horizontal(|ui| {
                ui.label("Start move:");
                ui.add(egui::Slider::new(&mut self.move_range.0, 0..=game.moves.len())
                    .suffix(" move"));
            });

            ui.horizontal(|ui| {
                ui.label("End move:");
                ui.add(egui::Slider::new(&mut self.move_range.1, self.move_range.0..=game.moves.len())
                    .suffix(" move"));
            });

            ui.label(format!("Selected {} moves", self.move_range.1 - self.move_range.0));

            // Create training data button
            ui.separator();
            if ui.button("🧬 Create Training Data (mRNA)").clicked() {
                let mut data = TrainingData::new();

                // Add positions from the selected range
                let mut current_state = GameState::new(game.board_size);
                for i in 0..self.move_range.1.min(game.moves.len()) {
                    if i >= self.move_range.0 {
                        data.add_position(current_state.clone(), game.moves[i]);
                    }
                    if let Ok(_) = current_state.apply_move(game.moves[i]) {
                        // Move was valid
                    }
                }

                training_data = Some(data);

                ui.label("✅ Training data created!");
            }
        }

        // Show error if any
        if let Some(error) = &self.error {
            ui.separator();
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
        }

        training_data
    }

    fn load_sgf(&mut self) {
        if let Some(path) = &self.file_path {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.sgf_content = Some(content.clone());
                    self.parse_sgf_content(content);
                }
                Err(e) => {
                    self.error = Some(format!("Failed to read file: {}", e));
                }
            }
        }
    }

    fn parse_sgf_content(&mut self, content: String) {
        match SGFParser::parse(&content) {
            Ok(game) => {
                self.move_range = (0, game.moves.len());
                self.parsed_game = Some(game);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("Failed to parse SGF: {}", e));
                self.parsed_game = None;
            }
        }
    }
}