use eframe::egui;
use p2pgo_core::{GameState, SGFParser, ko_detector::{KoDetector, KoSituation}};
// Removed unused imports: training::NeuralTrainer, config::NeuralConfig
use p2pgo_neural::TrainingData;
use std::path::PathBuf;
use std::collections::HashMap;

/// Complete neural network training UI
pub struct NeuralTrainingUI {
    /// Current tab/view
    current_tab: TrainingTab,
    /// SGF upload component
    sgf_upload: SGFUploadComponent,
    /// Game manager
    game_manager: GameManager,
    /// Training manager
    training_manager: TrainingManager,
    /// Ko analysis results
    ko_analysis: KoAnalysisResults,
    /// Error/success messages
    messages: Vec<(MessageType, String)>,
}

#[derive(Debug, Clone, PartialEq)]
enum TrainingTab {
    Upload,
    Games,
    Training,
    Analysis,
}

#[derive(Debug, Clone)]
enum MessageType {
    Success,
    Warning,
    Error,
}

/// SGF upload component with batch support
struct SGFUploadComponent {
    /// Selected files
    selected_files: Vec<PathBuf>,
    /// Direct paste content
    paste_content: String,
    /// Upload progress
    upload_progress: Option<(usize, usize)>,
    /// Parser instance
    parser: SGFParser,
}

/// Manages multiple uploaded games
struct GameManager {
    /// Stored games with metadata
    games: HashMap<String, GameInfo>,
    /// Selected games for training
    selected_games: Vec<String>,
    /// Filter settings
    filter: GameFilter,
}

#[derive(Debug, Clone)]
struct GameInfo {
    game_state: GameState,
    source_file: Option<String>,
    upload_time: std::time::Instant,
    has_ko: bool,
    move_count: usize,
    result: String,
}

#[derive(Debug, Clone)]
struct GameFilter {
    min_moves: usize,
    max_moves: usize,
    only_with_ko: bool,
    only_without_ko: bool,
}

/// Training data manager
struct TrainingManager {
    /// Training datasets created
    datasets: Vec<TrainingDataset>,
    /// Current training status
    training_status: TrainingStatus,
    /// Training config
    config: TrainingConfig,
}

#[derive(Debug, Clone)]
struct TrainingDataset {
    id: String,
    game_count: usize,
    total_positions: usize,
    ko_positions: usize,
    created_time: std::time::Instant,
}

#[derive(Debug, Clone)]
enum TrainingStatus {
    Idle,
    Preparing,
    Training { progress: f32, eta: String },
    Completed { accuracy: f32 },
    Failed(String),
}

#[derive(Debug, Clone)]
struct TrainingConfig {
    include_ko_positions: bool,
    augment_data: bool,
    validation_split: f32,
    batch_size: usize,
}

/// Ko analysis results
struct KoAnalysisResults {
    total_games_analyzed: usize,
    games_with_ko: usize,
    total_ko_situations: usize,
    ko_situations: Vec<(String, Vec<KoSituation>)>, // (game_id, situations)
}

impl NeuralTrainingUI {
    pub fn new() -> Self {
        Self {
            current_tab: TrainingTab::Upload,
            sgf_upload: SGFUploadComponent::new(),
            game_manager: GameManager::new(),
            training_manager: TrainingManager::new(),
            ko_analysis: KoAnalysisResults::default(),
            messages: Vec::new(),
        }
    }
    
    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<TrainingData> {
        let mut training_data = None;
        
        // Header
        ui.heading("🧠 Neural Network Training Center");
        ui.separator();
        
        // Tab selector
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.current_tab, TrainingTab::Upload, "📤 Upload SGF");
            ui.selectable_value(&mut self.current_tab, TrainingTab::Games, "🎮 Game Library");
            ui.selectable_value(&mut self.current_tab, TrainingTab::Training, "🔧 Training");
            ui.selectable_value(&mut self.current_tab, TrainingTab::Analysis, "📊 Ko Analysis");
        });
        
        ui.separator();
        
        // Render current tab
        match self.current_tab {
            TrainingTab::Upload => {
                if let Some(games) = self.render_upload_tab(ui) {
                    self.process_uploaded_games(games);
                }
            }
            TrainingTab::Games => {
                self.render_games_tab(ui);
            }
            TrainingTab::Training => {
                training_data = self.render_training_tab(ui);
            }
            TrainingTab::Analysis => {
                self.render_analysis_tab(ui);
            }
        }
        
        // Show messages
        self.render_messages(ui);
        
        training_data
    }
    
    fn render_upload_tab(&mut self, ui: &mut egui::Ui) -> Option<Vec<GameState>> {
        self.sgf_upload.render(ui)
    }
    
    fn render_games_tab(&mut self, ui: &mut egui::Ui) {
        self.game_manager.render(ui)
    }
    
    fn render_training_tab(&mut self, ui: &mut egui::Ui) -> Option<TrainingData> {
        self.training_manager.render(ui, &self.game_manager)
    }
    
    fn render_analysis_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ko Situation Analysis");
        
        if ui.button("🔍 Analyze All Games").clicked() {
            self.analyze_ko_situations();
        }
        
        ui.separator();
        
        // Show analysis results
        if self.ko_analysis.total_games_analyzed > 0 {
            egui::Grid::new("ko_stats").show(ui, |ui| {
                ui.label("Total games analyzed:");
                ui.label(format!("{}", self.ko_analysis.total_games_analyzed));
                ui.end_row();
                
                ui.label("Games with Ko:");
                ui.label(format!("{} ({:.1}%)", 
                    self.ko_analysis.games_with_ko,
                    (self.ko_analysis.games_with_ko as f32 / self.ko_analysis.total_games_analyzed as f32) * 100.0
                ));
                ui.end_row();
                
                ui.label("Total Ko situations:");
                ui.label(format!("{}", self.ko_analysis.total_ko_situations));
                ui.end_row();
            });
            
            ui.separator();
            ui.heading("Ko Situations by Game");
            
            if self.ko_analysis.games_with_ko == 0 {
                ui.colored_label(egui::Color32::YELLOW, 
                    "⚠️ No Ko situations found in uploaded games");
                
                ui.separator();
                if ui.button("🎲 Generate Ko Training Patterns").clicked() {
                    self.generate_ko_patterns();
                }
                
                ui.label("This will create synthetic Ko situations for training when real game data lacks Ko examples.");
            } else {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (game_id, situations) in &self.ko_analysis.ko_situations {
                        ui.collapsing(format!("Game {} ({} Ko situations)", 
                            &game_id[..8], situations.len()), |ui| {
                            for (i, ko) in situations.iter().enumerate() {
                                ui.label(format!("Ko #{}: Move {} at ({}, {})", 
                                    i + 1, ko.capture_move, ko.ko_point.x, ko.ko_point.y));
                                if let Some(recap) = ko.recapture_move {
                                    ui.label(format!("  Recaptured at move {}", recap));
                                }
                            }
                        });
                    }
                });
            }
        } else {
            ui.label("No Ko analysis performed yet. Click 'Analyze All Games' to start.");
        }
    }
    
    fn render_messages(&mut self, ui: &mut egui::Ui) {
        if !self.messages.is_empty() {
            ui.separator();
            
            // Show latest messages
            let messages_to_show: Vec<_> = self.messages.iter().rev().take(5).collect();
            for (msg_type, msg) in messages_to_show {
                let color = match msg_type {
                    MessageType::Success => egui::Color32::GREEN,
                    MessageType::Warning => egui::Color32::YELLOW,
                    MessageType::Error => egui::Color32::RED,
                };
                ui.colored_label(color, msg);
            }
            
            // Clear old messages button
            if self.messages.len() > 5 && ui.small_button("Clear messages").clicked() {
                self.messages.clear();
            }
        }
    }
    
    fn process_uploaded_games(&mut self, games: Vec<GameState>) {
        for game in games {
            let game_id = game.id.clone();
            
            // Analyze Ko situations
            let mut detector = KoDetector::new();
            let mut board = p2pgo_core::board::Board::new(game.board_size);
            
            for mv in &game.moves {
                if let p2pgo_core::Move::Place { x, y, color } = mv {
                    board.place(p2pgo_core::Coord::new(*x, *y), *color);
                    // Note: We'd need to track captures properly here
                    detector.process_move(&board, mv, &[]);
                }
            }
            
            let has_ko = !detector.get_ko_situations().is_empty();
            
            let info = GameInfo {
                game_state: game.clone(),
                source_file: None,
                upload_time: std::time::Instant::now(),
                has_ko,
                move_count: game.moves.len(),
                result: format!("{:?}", game.result),
            };
            
            self.game_manager.games.insert(game_id.clone(), info);
            self.messages.push((MessageType::Success, 
                format!("Uploaded game {} ({} moves)", &game_id[..8], game.moves.len())));
        }
    }
    
    fn analyze_ko_situations(&mut self) {
        let mut total_ko = 0;
        let mut games_with_ko = 0;
        let mut ko_situations = Vec::new();
        
        for (game_id, info) in &self.game_manager.games {
            let mut detector = KoDetector::new();
            let mut board = p2pgo_core::board::Board::new(info.game_state.board_size);
            
            for mv in &info.game_state.moves {
                if let p2pgo_core::Move::Place { x, y, color } = mv {
                    board.place(p2pgo_core::Coord::new(*x, *y), *color);
                    detector.process_move(&board, mv, &[]);
                }
            }
            
            let ko_list = detector.get_ko_situations();
            if !ko_list.is_empty() {
                games_with_ko += 1;
                total_ko += ko_list.len();
                ko_situations.push((game_id.clone(), ko_list.to_vec()));
            }
        }
        
        self.ko_analysis = KoAnalysisResults {
            total_games_analyzed: self.game_manager.games.len(),
            games_with_ko,
            total_ko_situations: total_ko,
            ko_situations,
        };
        
        self.messages.push((MessageType::Success, 
            format!("Analyzed {} games, found {} Ko situations", 
                self.game_manager.games.len(), total_ko)));
    }
    
    fn generate_ko_patterns(&mut self) {
        use p2pgo_core::ko_generator::KoTrainingGenerator;
        
        let generator = KoTrainingGenerator::new();
        let ko_games = generator.generate_training_situations(5);
        
        for (i, game) in ko_games.into_iter().enumerate() {
            let game_id = format!("synthetic_ko_{}", i);
            
            let info = GameInfo {
                game_state: game,
                source_file: Some("Generated Ko Pattern".to_string()),
                upload_time: std::time::Instant::now(),
                has_ko: true,
                move_count: 10, // Approximate
                result: "Ko Training Pattern".to_string(),
            };
            
            self.game_manager.games.insert(game_id, info);
        }
        
        self.messages.push((MessageType::Success, 
            "Generated 5 Ko training patterns".to_string()));
    }
}

impl SGFUploadComponent {
    fn new() -> Self {
        Self {
            selected_files: Vec::new(),
            paste_content: String::new(),
            upload_progress: None,
            parser: SGFParser::new(),
        }
    }
    
    fn render(&mut self, ui: &mut egui::Ui) -> Option<Vec<GameState>> {
        let mut uploaded_games = None;
        
        ui.heading("Upload SGF Files");
        
        // File selection
        ui.horizontal(|ui| {
            if ui.button("📁 Select Files...").clicked() {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("SGF files", &["sgf"])
                    .pick_files()
                {
                    self.selected_files = paths;
                }
            }
            
            if ui.button("📂 Select Folder...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    // Find all SGF files in folder
                    if let Ok(entries) = std::fs::read_dir(&path) {
                        self.selected_files = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sgf"))
                            .collect();
                    }
                }
            }
        });
        
        // Show selected files
        if !self.selected_files.is_empty() {
            ui.separator();
            ui.label(format!("Selected {} files", self.selected_files.len()));
            
            egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                for file in &self.selected_files {
                    ui.label(format!("• {}", file.file_name().unwrap().to_string_lossy()));
                }
            });
            
            if ui.button("🚀 Upload All").clicked() {
                uploaded_games = Some(self.process_files());
            }
        }
        
        // Paste option
        ui.separator();
        ui.label("Or paste SGF content:");
        
        if ui.text_edit_multiline(&mut self.paste_content).changed() && !self.paste_content.is_empty() {
            // Auto-parse on paste
            if let Ok(game) = self.parser.parse(&self.paste_content) {
                uploaded_games = Some(vec![game]);
                self.paste_content.clear();
            }
        }
        
        // Show progress
        if let Some((current, total)) = self.upload_progress {
            ui.separator();
            let progress = current as f32 / total as f32;
            ui.add(egui::ProgressBar::new(progress)
                .text(format!("{}/{} files processed", current, total)));
        }
        
        uploaded_games
    }
    
    fn process_files(&mut self) -> Vec<GameState> {
        let mut games = Vec::new();
        let total = self.selected_files.len();
        
        for (i, path) in self.selected_files.iter().enumerate() {
            self.upload_progress = Some((i, total));
            
            if let Ok(game) = self.parser.parse_file(path) {
                games.push(game);
            }
        }
        
        self.upload_progress = None;
        self.selected_files.clear();
        games
    }
}

impl GameManager {
    fn new() -> Self {
        Self {
            games: HashMap::new(),
            selected_games: Vec::new(),
            filter: GameFilter {
                min_moves: 0,
                max_moves: 500,
                only_with_ko: false,
                only_without_ko: false,
            },
        }
    }
    
    fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("Game Library ({} games)", self.games.len()));
        
        // Filter controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.label("Min moves:");
            ui.add(egui::DragValue::new(&mut self.filter.min_moves).speed(1));
            ui.label("Max moves:");
            ui.add(egui::DragValue::new(&mut self.filter.max_moves).speed(1));
            ui.checkbox(&mut self.filter.only_with_ko, "Only with Ko");
            ui.checkbox(&mut self.filter.only_without_ko, "Only without Ko");
        });
        
        // Selection controls
        ui.horizontal(|ui| {
            if ui.button("Select All").clicked() {
                self.selected_games = self.get_filtered_games();
            }
            if ui.button("Clear Selection").clicked() {
                self.selected_games.clear();
            }
            ui.label(format!("{} selected", self.selected_games.len()));
        });
        
        ui.separator();
        
        // Game list
        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
            for (game_id, info) in &self.games {
                if self.matches_filter(info) {
                    let is_selected = self.selected_games.contains(game_id);
                    let mut selected = is_selected;
                    
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut selected, "").changed() {
                            if selected && !is_selected {
                                self.selected_games.push(game_id.clone());
                            } else if !selected && is_selected {
                                self.selected_games.retain(|id| id != game_id);
                            }
                        }
                        
                        ui.label(format!("{} - {} moves", &game_id[..8], info.move_count));
                        
                        if info.has_ko {
                            ui.colored_label(egui::Color32::YELLOW, "Ko");
                        }
                        
                        ui.label(&info.result);
                    });
                }
            }
        });
    }
    
    fn matches_filter(&self, info: &GameInfo) -> bool {
        if info.move_count < self.filter.min_moves || info.move_count > self.filter.max_moves {
            return false;
        }
        
        if self.filter.only_with_ko && !info.has_ko {
            return false;
        }
        
        if self.filter.only_without_ko && info.has_ko {
            return false;
        }
        
        true
    }
    
    fn get_filtered_games(&self) -> Vec<String> {
        self.games.iter()
            .filter(|(_, info)| self.matches_filter(info))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl TrainingManager {
    fn new() -> Self {
        Self {
            datasets: Vec::new(),
            training_status: TrainingStatus::Idle,
            config: TrainingConfig {
                include_ko_positions: true,
                augment_data: true,
                validation_split: 0.2,
                batch_size: 32,
            },
        }
    }
    
    fn render(&mut self, ui: &mut egui::Ui, game_manager: &GameManager) -> Option<TrainingData> {
        let mut training_data = None;
        
        ui.heading("Training Configuration");
        
        // Config settings
        egui::Grid::new("training_config").show(ui, |ui| {
            ui.label("Include Ko positions:");
            ui.checkbox(&mut self.config.include_ko_positions, "");
            ui.end_row();
            
            ui.label("Data augmentation:");
            ui.checkbox(&mut self.config.augment_data, "");
            ui.end_row();
            
            ui.label("Validation split:");
            ui.add(egui::Slider::new(&mut self.config.validation_split, 0.1..=0.4));
            ui.end_row();
            
            ui.label("Batch size:");
            ui.add(egui::DragValue::new(&mut self.config.batch_size).speed(1));
            ui.end_row();
        });
        
        ui.separator();
        
        // Create dataset button
        if !game_manager.selected_games.is_empty() {
            if ui.button(format!("🧬 Create Training Dataset ({} games)", 
                game_manager.selected_games.len())).clicked() {
                training_data = Some(self.create_training_data(game_manager));
            }
        } else {
            ui.label("Select games from the Game Library tab first");
        }
        
        // Show existing datasets
        if !self.datasets.is_empty() {
            ui.separator();
            ui.heading("Training Datasets");
            
            for dataset in &self.datasets {
                ui.horizontal(|ui| {
                    ui.label(format!("Dataset {}: {} games, {} positions", 
                        &dataset.id[..8], dataset.game_count, dataset.total_positions));
                    if dataset.ko_positions > 0 {
                        ui.label(format!("({} Ko positions)", dataset.ko_positions));
                    }
                });
            }
        }
        
        // Training status
        match &self.training_status {
            TrainingStatus::Idle => {}
            TrainingStatus::Preparing => {
                ui.spinner();
                ui.label("Preparing training data...");
            }
            TrainingStatus::Training { progress, eta } => {
                ui.add(egui::ProgressBar::new(*progress));
                ui.label(format!("Training... ETA: {}", eta));
            }
            TrainingStatus::Completed { accuracy } => {
                ui.colored_label(egui::Color32::GREEN, 
                    format!("✅ Training completed! Accuracy: {:.1}%", accuracy * 100.0));
            }
            TrainingStatus::Failed(error) => {
                ui.colored_label(egui::Color32::RED, format!("❌ Training failed: {}", error));
            }
        }
        
        training_data
    }
    
    fn create_training_data(&mut self, game_manager: &GameManager) -> TrainingData {
        // Placeholder - in real implementation, this would process selected games
        let dataset = TrainingDataset {
            id: uuid::Uuid::new_v4().to_string(),
            game_count: game_manager.selected_games.len(),
            total_positions: game_manager.selected_games.len() * 100, // Estimate
            ko_positions: 0,
            created_time: std::time::Instant::now(),
        };
        
        self.datasets.push(dataset);
        
        // Return placeholder training data
        TrainingData {
            states: vec![],
            moves: vec![],
            outcome: 0.0,
            consensus: false,
        }
    }
}

impl Default for GameFilter {
    fn default() -> Self {
        Self {
            min_moves: 0,
            max_moves: 500,
            only_with_ko: false,
            only_without_ko: false,
        }
    }
}

impl Default for KoAnalysisResults {
    fn default() -> Self {
        Self {
            total_games_analyzed: 0,
            games_with_ko: 0,
            total_ko_situations: 0,
            ko_situations: Vec::new(),
        }
    }
}