//! UI for neural network configuration

use eframe::egui::{self, RichText, Slider};
use p2pgo_neural::config::{NeuralConfig, ConfigWizard};

/// Neural network configuration UI
pub struct NeuralConfigUI {
    pub config: NeuralConfig,
    pub wizard: ConfigWizard,
    pub current_question: usize,
    pub is_complete: bool,
    pub show_explanations: bool,
}

impl NeuralConfigUI {
    pub fn new() -> Self {
        Self {
            config: NeuralConfig::default(),
            wizard: ConfigWizard::new(),
            current_question: 0,
            is_complete: false,
            show_explanations: true,
        }
    }

    /// Render the configuration UI
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("🧠 Neural Network Configuration");

        // Explanation of dual network system
        ui.collapsing("Understanding Your Neural Networks", |ui| {
            ui.label(RichText::new("Like AlphaGo, you have TWO neural networks:").strong());
            ui.add_space(5.0);

            ui.label("1️⃣ Policy Network (Move Predictor):");
            ui.label("   • Suggests where to play next");
            ui.label("   • Shows as heat map overlay");
            ui.label("   • Trained on move patterns from games");
            ui.add_space(5.0);

            ui.label("2️⃣ Value Network (Position Evaluator):");
            ui.label("   • Evaluates who's winning");
            ui.label("   • Shows win probability percentage");
            ui.label("   • Trained on game outcomes");
            ui.add_space(10.0);

            ui.label(RichText::new("Your answers configure BOTH networks!").color(egui::Color32::YELLOW));
        });

        ui.separator();

        if !self.is_complete {
            self.render_wizard(ui);
        } else {
            self.render_configured(ui);
        }
    }

    /// Render the configuration wizard
    fn render_wizard(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        if let Some((question, description)) = self.wizard.get_question(self.current_question) {
            ui.label(RichText::new(format!("Question {} of 10", self.current_question + 1))
                .size(16.0));

            ui.add_space(10.0);

            ui.label(RichText::new(question).size(18.0).strong());
            ui.label(description);

            ui.add_space(20.0);

            // Value slider
            let mut value = 5u8;
            ui.horizontal(|ui| {
                ui.label("Your answer:");
                ui.add(Slider::new(&mut value, 1..=10)
                    .show_value(true)
                    .clamp_to_range(true));
            });

            ui.add_space(10.0);

            // Show neural network impact
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("🧠 Neural Impact:").strong());
                let impact = match self.current_question {
                    0 => format!("Aggression {}: Policy net will suggest {} attacking moves, Value net will {} fighting positions",
                        value,
                        if value > 6 { "more" } else { "fewer" },
                        if value > 6 { "favor" } else { "avoid" }
                    ),
                    1 => format!("Territory {}: Networks will {} solid territory over influence",
                        value,
                        if value > 6 { "prioritize" } else { "devalue" }
                    ),
                    2 => format!("Fighting {}: Networks will {} complex tactical situations",
                        value,
                        if value > 6 { "seek out" } else { "simplify" }
                    ),
                    3 => format!("Patterns {}: {} on learned shapes vs deep calculation",
                        value,
                        if value > 6 { "More reliance" } else { "Less reliance" }
                    ),
                    4 => format!("Risk {}: Networks will {} uncertain positions",
                        value,
                        if value > 6 { "embrace" } else { "avoid" }
                    ),
                    5 => format!("Opening {}: Early game will be {}",
                        value,
                        if value > 6 { "aggressive and fast" } else { "solid and careful" }
                    ),
                    6 => format!("Middle game {}: Networks {} at complex middle game",
                        value,
                        if value > 6 { "excel" } else { "struggle" }
                    ),
                    7 => format!("Endgame {}: Networks {} in endgame counting",
                        value,
                        if value > 6 { "very precise" } else { "approximate" }
                    ),
                    8 => format!("Learning {}: Networks will adapt {} from training",
                        value,
                        if value > 6 { "quickly" } else { "slowly" }
                    ),
                    9 => format!("Creativity {}: Networks will suggest {} moves",
                        value,
                        if value > 6 { "unconventional" } else { "standard" }
                    ),
                    _ => String::new(),
                };
                ui.label(impact);
            });

            ui.add_space(10.0);

            // Show what this value means
            let meaning = match self.current_question {
                0 => match value { // Aggression
                    1..=3 => "Very defensive - Focus on solid territory",
                    4..=6 => "Balanced - Mix of territory and fighting",
                    7..=8 => "Aggressive - Actively seek fights",
                    9..=10 => "Very aggressive - Constant pressure",
                    _ => "",
                },
                1 => match value { // Territory focus
                    1..=3 => "Fighting focused - Influence over territory",
                    4..=6 => "Balanced approach",
                    7..=8 => "Territory focused - Secure points",
                    9..=10 => "Very territorial - Maximum efficiency",
                    _ => "",
                },
                2 => match value { // Fighting spirit
                    1..=3 => "Peaceful - Avoid complications",
                    4..=6 => "Normal - Fight when necessary",
                    7..=8 => "Combative - Enjoy tactical battles",
                    9..=10 => "Warrior - Seek every fight",
                    _ => "",
                },
                3 => match value { // Pattern recognition
                    1..=3 => "Calculate everything deeply",
                    4..=6 => "Mix patterns and calculation",
                    7..=8 => "Trust learned patterns",
                    9..=10 => "Intuitive pattern-based play",
                    _ => "",
                },
                4 => match value { // Risk tolerance
                    1..=3 => "Very safe - Minimize uncertainty",
                    4..=6 => "Moderate risk when justified",
                    7..=8 => "Accept risks for gains",
                    9..=10 => "High risk, high reward",
                    _ => "",
                },
                _ => "",
            };

            if !meaning.is_empty() {
                ui.label(RichText::new(meaning).italics());
            }

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                if self.current_question > 0 {
                    if ui.button("← Previous").clicked() {
                        self.current_question -= 1;
                    }
                }

                if ui.button("Next →").clicked() {
                    self.wizard.answer(value);

                    if self.wizard.is_complete() {
                        if let Some(config) = self.wizard.build_config() {
                            self.config = config;
                            self.is_complete = true;
                        }
                    } else {
                        self.current_question += 1;
                    }
                }
            });
        }
    }

    /// Render the configured state
    fn render_configured(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("✅ Neural Network Configured").color(egui::Color32::GREEN));

        ui.separator();

        // Show configuration summary
        egui::Grid::new("config_summary")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                ui.label("Aggression:");
                ui.label(format!("{}/10", self.config.aggression));
                ui.end_row();

                ui.label("Territory Focus:");
                ui.label(format!("{}/10", self.config.territory_focus));
                ui.end_row();

                ui.label("Fighting Spirit:");
                ui.label(format!("{}/10", self.config.fighting_spirit));
                ui.end_row();

                ui.label("Pattern Recognition:");
                ui.label(format!("{}/10", self.config.pattern_recognition));
                ui.end_row();

                ui.label("Risk Tolerance:");
                ui.label(format!("{}/10", self.config.risk_tolerance));
                ui.end_row();
            });

        ui.add_space(20.0);

        // Preset buttons
        ui.label(RichText::new("Quick Presets:").strong());
        ui.horizontal(|ui| {
            if ui.button("Balanced").clicked() {
                self.config = NeuralConfig::balanced();
            }
            if ui.button("Aggressive").clicked() {
                self.config = NeuralConfig::aggressive();
            }
            if ui.button("Territorial").clicked() {
                self.config = NeuralConfig::territorial();
            }
        });

        ui.add_space(10.0);

        if ui.button("Reconfigure").clicked() {
            self.wizard = ConfigWizard::new();
            self.current_question = 0;
            self.is_complete = false;
        }
    }
}

/// Explanation panel for neural network functionality
pub struct NeuralExplanationPanel;

impl NeuralExplanationPanel {
    pub fn render(ui: &mut egui::Ui) {
        ui.collapsing("🤔 How Neural Networks Work", |ui| {
            ui.label("The neural network analyzes the board and suggests moves based on patterns learned from professional games.");

            ui.add_space(5.0);

            ui.label(RichText::new("Heat Maps:").strong());
            ui.label("• Red areas = High probability moves");
            ui.label("• Yellow areas = Medium probability moves");
            ui.label("• Blue areas = Low probability moves");
            ui.label("• Transparent = Very unlikely moves");

            ui.add_space(5.0);

            ui.label(RichText::new("Configuration Impact:").strong());
            ui.label("• Aggression: Prefers attacking moves vs defensive");
            ui.label("• Territory: Focus on securing points vs influence");
            ui.label("• Fighting: Willingness to engage in complex battles");
            ui.label("• Patterns: Rely on known shapes vs deep calculation");

            ui.add_space(5.0);

            ui.label(RichText::new("Training:").strong());
            ui.label("Upload SGF files from your games or OGS to train the network on your playing style!");
        });

        ui.collapsing("📚 Button Guide", |ui| {
            ui.label(RichText::new("Configure Neural Net:").strong());
            ui.label("Answer 10 questions to set up your AI assistant's personality");

            ui.add_space(5.0);

            ui.label(RichText::new("Upload SGF Files:").strong());
            ui.label("Train the network using game records from OGS or other sources");

            ui.add_space(5.0);

            ui.label(RichText::new("Toggle Heat Map (H key):").strong());
            ui.label("Show/hide move probability visualization during play");

            ui.add_space(5.0);

            ui.label(RichText::new("Save/Load Network:").strong());
            ui.label("Save your trained network or load a pre-trained one");
        });
    }
}