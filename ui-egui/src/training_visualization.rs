use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Ui, Vec2};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::time::Instant;

/// Neural network training visualization
pub struct TrainingVisualization {
    /// Training history
    policy_loss_history: VecDeque<(f32, f32)>, // (epoch, loss)
    value_loss_history: VecDeque<(f32, f32)>,
    /// Current training metrics
    current_metrics: TrainingMetrics,
    /// Training start time
    start_time: Option<Instant>,
    /// Total games trained
    total_games: usize,
    /// Weight difference visualization
    weight_changes: WeightChangeViz,
    /// Training phase
    training_phase: TrainingPhase,
}

#[derive(Clone, Default)]
pub struct TrainingMetrics {
    pub epoch: u32,
    pub policy_loss: f32,
    pub value_loss: f32,
    pub learning_rate: f32,
    pub games_in_batch: usize,
    pub consensus_rate: f32,
    pub time_per_epoch: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TrainingPhase {
    Idle,
    CollectingData,
    Training,
    Validating,
    SharingWeights,
}

struct WeightChangeViz {
    /// Heatmap of weight changes per layer
    policy_changes: Vec<Vec<f32>>,
    value_changes: Vec<Vec<f32>>,
    /// Max change for normalization
    max_change: f32,
}

impl TrainingVisualization {
    pub fn new() -> Self {
        Self {
            policy_loss_history: VecDeque::with_capacity(100),
            value_loss_history: VecDeque::with_capacity(100),
            current_metrics: TrainingMetrics::default(),
            start_time: None,
            total_games: 0,
            weight_changes: WeightChangeViz {
                policy_changes: vec![],
                value_changes: vec![],
                max_change: 0.1,
            },
            training_phase: TrainingPhase::Idle,
        }
    }
    
    /// Update with new training metrics
    pub fn update_metrics(&mut self, metrics: TrainingMetrics) {
        self.policy_loss_history.push_back((metrics.epoch as f32, metrics.policy_loss));
        self.value_loss_history.push_back((metrics.epoch as f32, metrics.value_loss));
        
        if self.policy_loss_history.len() > 100 {
            self.policy_loss_history.pop_front();
        }
        if self.value_loss_history.len() > 100 {
            self.value_loss_history.pop_front();
        }
        
        self.current_metrics = metrics.clone();
        self.total_games += metrics.games_in_batch;
        
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
    }
    
    /// Update weight changes
    pub fn update_weight_changes(&mut self, policy: Vec<Vec<f32>>, value: Vec<Vec<f32>>) {
        self.weight_changes.policy_changes = policy.clone();
        self.weight_changes.value_changes = value.clone();
        
        // Find max change for normalization
        let max_p = policy.clone().iter()
            .flat_map(|layer| layer.iter())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.1);
        
        let max_v = value.clone().iter()
            .flat_map(|layer| layer.iter())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .copied()
            .unwrap_or(0.1);
        
        self.weight_changes.max_change = max_p.max(max_v);
    }
    
    /// Set training phase
    pub fn set_phase(&mut self, phase: TrainingPhase) {
        self.training_phase = phase;
    }
    
    /// Render the training visualization
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("🧠 Neural Network Training");
        
        // Training status
        self.render_status_bar(ui);
        ui.separator();
        
        // Main visualization area
        ui.columns(2, |columns| {
            // Left: Loss graphs
            columns[0].group(|ui| {
                self.render_loss_graphs(ui);
            });
            
            // Right: Weight changes
            columns[1].group(|ui| {
                self.render_weight_changes(ui);
            });
        });
        
        ui.separator();
        
        // Training metrics
        self.render_metrics_panel(ui);
        
        // RNA collection status
        self.render_rna_status(ui);
    }
    
    fn render_status_bar(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Phase indicator
            let (phase_text, phase_color) = match self.training_phase {
                TrainingPhase::Idle => ("Idle", Color32::GRAY),
                TrainingPhase::CollectingData => ("Collecting RNA", Color32::YELLOW),
                TrainingPhase::Training => ("Training", Color32::GREEN),
                TrainingPhase::Validating => ("Validating", Color32::BLUE),
                TrainingPhase::SharingWeights => ("Sharing Weights", Color32::from_rgb(255, 100, 255)),
            };
            
            ui.colored_label(phase_color, format!("● {}", phase_text));
            
            ui.separator();
            
            // Epoch counter
            ui.label(format!("Epoch: {}", self.current_metrics.epoch));
            
            ui.separator();
            
            // Games trained
            ui.label(format!("Games: {}", self.total_games));
            
            ui.separator();
            
            // Training time
            if let Some(start) = self.start_time {
                let elapsed = start.elapsed();
                ui.label(format!("Time: {}:{:02}", 
                    elapsed.as_secs() / 60,
                    elapsed.as_secs() % 60
                ));
            }
            
            // Progress bar for current epoch
            if self.training_phase == TrainingPhase::Training {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::ProgressBar::new(0.5).text("Epoch Progress"));
                });
            }
        });
    }
    
    fn render_loss_graphs(&self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Training Loss").strong());
        
        let plot_height = 200.0;
        
        // Policy loss graph
        ui.label("Policy Network Loss");
        Plot::new("policy_loss")
            .height(plot_height)
            .width(ui.available_width())
            .show_axes([true, true])
            .show(ui, |plot_ui| {
                if !self.policy_loss_history.is_empty() {
                    let points: PlotPoints = self.policy_loss_history
                        .iter()
                        .map(|(epoch, loss)| [*epoch as f64, *loss as f64])
                        .collect();
                    
                    plot_ui.line(Line::new(points)
                        .color(Color32::from_rgb(100, 150, 255))
                        .width(2.0)
                        .name("Policy Loss"));
                }
            });
        
        ui.add_space(10.0);
        
        // Value loss graph
        ui.label("Value Network Loss");
        Plot::new("value_loss")
            .height(plot_height)
            .width(ui.available_width())
            .show_axes([true, true])
            .show(ui, |plot_ui| {
                if !self.value_loss_history.is_empty() {
                    let points: PlotPoints = self.value_loss_history
                        .iter()
                        .map(|(epoch, loss)| [*epoch as f64, *loss as f64])
                        .collect();
                    
                    plot_ui.line(Line::new(points)
                        .color(Color32::from_rgb(100, 200, 100))
                        .width(2.0)
                        .name("Value Loss"));
                }
            });
    }
    
    fn render_weight_changes(&self, ui: &mut Ui) {
        ui.label(egui::RichText::new("Weight Updates").strong());
        
        let available_rect = ui.available_rect();
        let rect_height = available_rect.height() / 2.0 - 20.0;
        
        // Policy network weight changes
        ui.label("Policy Network");
        let policy_rect = Rect::from_min_size(
            available_rect.min,
            Vec2::new(available_rect.width(), rect_height)
        );
        self.render_weight_heatmap(ui, policy_rect, &self.weight_changes.policy_changes, Color32::BLUE);
        
        ui.add_space(20.0);
        
        // Value network weight changes
        ui.label("Value Network");
        let value_rect = Rect::from_min_size(
            available_rect.min + Vec2::new(0.0, rect_height + 40.0),
            Vec2::new(available_rect.width(), rect_height)
        );
        self.render_weight_heatmap(ui, value_rect, &self.weight_changes.value_changes, Color32::GREEN);
    }
    
    fn render_weight_heatmap(&self, ui: &mut Ui, rect: Rect, changes: &[Vec<f32>], _base_color: Color32) {
        let painter = ui.painter();
        
        if changes.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No weight data yet",
                FontId::proportional(12.0),
                Color32::GRAY,
            );
            return;
        }
        
        // Draw heatmap
        let layer_width = rect.width() / changes.len() as f32;
        
        for (layer_idx, layer_changes) in changes.iter().enumerate() {
            let x = rect.left() + layer_idx as f32 * layer_width;
            let max_neurons = 50; // Limit visualization
            let neurons_to_show = layer_changes.len().min(max_neurons);
            let neuron_height = rect.height() / neurons_to_show as f32;
            
            for (neuron_idx, &change) in layer_changes.iter().take(neurons_to_show).enumerate() {
                let y = rect.top() + neuron_idx as f32 * neuron_height;
                
                // Normalize change to 0-1
                let normalized = (change.abs() / self.weight_changes.max_change).min(1.0);
                
                // Color based on change magnitude
                let color = if change > 0.0 {
                    Color32::from_rgb(
                        (255.0 * normalized) as u8,
                        (100.0 * (1.0 - normalized)) as u8,
                        0,
                    )
                } else {
                    Color32::from_rgb(
                        0,
                        (100.0 * (1.0 - normalized)) as u8,
                        (255.0 * normalized) as u8,
                    )
                };
                
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(x, y),
                        Vec2::new(layer_width - 1.0, neuron_height - 1.0)
                    ),
                    0.0,
                    color,
                );
            }
            
            // Layer label
            painter.text(
                Pos2::new(x + layer_width / 2.0, rect.bottom() + 5.0),
                egui::Align2::CENTER_TOP,
                format!("L{}", layer_idx + 1),
                FontId::proportional(9.0),
                Color32::GRAY,
            );
        }
        
        // Legend
        let legend_rect = Rect::from_min_size(
            rect.right_top() + Vec2::new(10.0, 0.0),
            Vec2::new(20.0, rect.height())
        );
        
        for i in 0..20 {
            let y = legend_rect.top() + (i as f32 / 20.0) * legend_rect.height();
            let normalized = 1.0 - (i as f32 / 20.0);
            
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(legend_rect.left(), y),
                    Vec2::new(10.0, legend_rect.height() / 20.0)
                ),
                0.0,
                Color32::from_rgb(
                    (255.0 * normalized) as u8,
                    0,
                    (255.0 * (1.0 - normalized)) as u8,
                ),
            );
        }
    }
    
    fn render_metrics_panel(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Current Metrics").strong());
            
            egui::Grid::new("metrics_grid").show(ui, |ui| {
                ui.label("Learning Rate:");
                ui.label(format!("{:.6}", self.current_metrics.learning_rate));
                ui.end_row();
                
                ui.label("Batch Size:");
                ui.label(format!("{} games", self.current_metrics.games_in_batch));
                ui.end_row();
                
                ui.label("Consensus Rate:");
                ui.add(egui::ProgressBar::new(self.current_metrics.consensus_rate)
                    .text(format!("{:.1}%", self.current_metrics.consensus_rate * 100.0)));
                ui.end_row();
                
                ui.label("Time/Epoch:");
                ui.label(format!("{:.1}s", self.current_metrics.time_per_epoch));
                ui.end_row();
            });
        });
    }
    
    fn render_rna_status(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("RNA Collection").strong());
            
            ui.horizontal(|ui| {
                ui.label("Sources:");
                
                // Show RNA sources
                let sources = vec![
                    ("SGF Upload", Color32::BLUE, 3),
                    ("Completed Games", Color32::GREEN, 5),
                    ("Shared RNA", Color32::YELLOW, 2),
                ];
                
                for (name, color, count) in sources {
                    ui.colored_label(color, format!("{}: {}", name, count));
                    ui.separator();
                }
            });
            
            // Quality distribution
            ui.label("RNA Quality Distribution:");
            self.render_quality_bar(ui);
        });
    }
    
    fn render_quality_bar(&self, ui: &mut Ui) {
        let available_width = ui.available_width();
        let bar_height = 20.0;
        
        let (_id, rect) = ui.allocate_space(Vec2::new(available_width, bar_height));
        let painter = ui.painter();
        
        // Quality segments
        let segments = vec![
            (0.2, Color32::RED, "Low"),
            (0.5, Color32::YELLOW, "Med"),
            (0.3, Color32::GREEN, "High"),
        ];
        
        let mut x = rect.left();
        for (portion, color, label) in segments {
            let width = rect.width() * portion;
            
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(x, rect.top()),
                    Vec2::new(width, bar_height)
                ),
                0.0,
                color,
            );
            
            painter.text(
                Pos2::new(x + width / 2.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional(10.0),
                Color32::WHITE,
            );
            
            x += width;
        }
        
        // Border
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GRAY));
    }
}