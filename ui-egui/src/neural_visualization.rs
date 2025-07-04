use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Neural network visualization for both Policy and Value networks
pub struct NeuralNetworkVisualization {
    /// Animation phase
    animation_phase: f32,
    /// Show weights
    show_weights: bool,
    /// Show activations
    show_activations: bool,
    /// Current activations (for live visualization)
    policy_activations: Vec<Vec<f32>>,
    value_activations: Vec<Vec<f32>>,
}

#[derive(Clone)]
pub struct NetworkArchitecture {
    pub name: String,
    pub layers: Vec<LayerInfo>,
    pub color_scheme: NetworkColorScheme,
}

#[derive(Clone)]
pub struct LayerInfo {
    pub name: String,
    pub neurons: usize,
    pub activation_type: String,
}

#[derive(Clone, Copy)]
pub enum NetworkColorScheme {
    Policy,  // Blue-based
    Value,   // Green-based
}

impl NeuralNetworkVisualization {
    pub fn new() -> Self {
        Self {
            animation_phase: 0.0,
            show_weights: true,
            show_activations: true,
            policy_activations: vec![],
            value_activations: vec![],
        }
    }
    
    /// Update activations from neural network
    pub fn update_activations(&mut self, policy: Vec<Vec<f32>>, value: Vec<Vec<f32>>) {
        self.policy_activations = policy;
        self.value_activations = value;
    }
    
    /// Render both neural networks side by side
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("🧠 Neural Networks");
        
        // Controls
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_weights, "Show Weights");
            ui.checkbox(&mut self.show_activations, "Show Activations");
        });
        
        ui.separator();
        
        // Create network architectures
        let policy_net = NetworkArchitecture {
            name: "Policy Network (Move Prediction)".to_string(),
            layers: vec![
                LayerInfo { name: "Input\n8×19×19".to_string(), neurons: 2888, activation_type: "".to_string() },
                LayerInfo { name: "Hidden 1\n512".to_string(), neurons: 512, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Hidden 2\n512".to_string(), neurons: 512, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Hidden 3\n256".to_string(), neurons: 256, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Output\n361".to_string(), neurons: 361, activation_type: "Softmax".to_string() },
            ],
            color_scheme: NetworkColorScheme::Policy,
        };
        
        let value_net = NetworkArchitecture {
            name: "Value Network (Position Evaluation)".to_string(),
            layers: vec![
                LayerInfo { name: "Input\n8×19×19".to_string(), neurons: 2888, activation_type: "".to_string() },
                LayerInfo { name: "Hidden 1\n512".to_string(), neurons: 512, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Hidden 2\n512".to_string(), neurons: 512, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Hidden 3\n256".to_string(), neurons: 256, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Hidden 4\n128".to_string(), neurons: 128, activation_type: "ReLU".to_string() },
                LayerInfo { name: "Output\n1".to_string(), neurons: 1, activation_type: "Tanh".to_string() },
            ],
            color_scheme: NetworkColorScheme::Value,
        };
        
        // Render networks
        ui.columns(2, |columns| {
            columns[0].group(|ui| {
                self.render_network(ui, &policy_net, &self.policy_activations);
            });
            
            columns[1].group(|ui| {
                self.render_network(ui, &value_net, &self.value_activations);
            });
        });
        
        // Update animation
        self.animation_phase += ui.input(|i| i.unstable_dt);
    }
    
    /// Render a single network
    fn render_network(&self, ui: &mut Ui, architecture: &NetworkArchitecture, activations: &[Vec<f32>]) {
        ui.label(egui::RichText::new(&architecture.name).heading());
        
        let available_size = ui.available_size();
        let available_rect = egui::Rect::from_min_size(ui.cursor().min, available_size);
        let response = ui.allocate_rect(available_rect, Sense::hover());
        let painter = ui.painter();
        
        let layer_spacing = available_rect.width() / (architecture.layers.len() as f32 + 1.0);
        let center_y = available_rect.center().y;
        
        // Calculate layer positions
        let mut layer_positions = Vec::new();
        for (i, layer) in architecture.layers.iter().enumerate() {
            let x = available_rect.left() + layer_spacing * (i as f32 + 1.0);
            
            // Limit neurons shown for large layers
            let neurons_to_show = layer.neurons.min(20);
            let neuron_spacing = if neurons_to_show > 1 {
                available_rect.height() * 0.8 / (neurons_to_show as f32 - 1.0)
            } else {
                0.0
            };
            
            let start_y = center_y - (neurons_to_show as f32 - 1.0) * neuron_spacing / 2.0;
            
            let mut positions = Vec::new();
            for j in 0..neurons_to_show {
                let y = start_y + j as f32 * neuron_spacing;
                positions.push(Pos2::new(x, y));
            }
            
            layer_positions.push(positions);
        }
        
        // Draw connections
        if self.show_weights {
            for i in 0..layer_positions.len() - 1 {
                let from_layer = &layer_positions[i];
                let to_layer = &layer_positions[i + 1];
                
                // Sample connections for large layers
                let from_sample = from_layer.len().min(10);
                let to_sample = to_layer.len().min(10);
                
                for j in 0..from_sample {
                    for k in 0..to_sample {
                        let from_idx = j * from_layer.len() / from_sample;
                        let to_idx = k * to_layer.len() / to_sample;
                        
                        let from = from_layer[from_idx];
                        let to = to_layer[to_idx];
                        
                        // Animate weight visualization
                        let phase = self.animation_phase + (j + k) as f32 * 0.1;
                        let weight = (phase.sin() + 1.0) * 0.5;
                        
                        let color = match architecture.color_scheme {
                            NetworkColorScheme::Policy => {
                                Color32::from_rgba_unmultiplied(
                                    (100.0 + weight * 100.0) as u8,
                                    (150.0 + weight * 50.0) as u8,
                                    255,
                                    (20.0 + weight * 30.0) as u8,
                                )
                            }
                            NetworkColorScheme::Value => {
                                Color32::from_rgba_unmultiplied(
                                    (100.0 + weight * 50.0) as u8,
                                    (200.0 + weight * 55.0) as u8,
                                    (100.0 + weight * 50.0) as u8,
                                    (20.0 + weight * 30.0) as u8,
                                )
                            }
                        };
                        
                        painter.line_segment([from, to], Stroke::new(0.5, color));
                    }
                }
            }
        }
        
        // Draw neurons
        for (layer_idx, (layer_info, positions)) in architecture.layers.iter().zip(&layer_positions).enumerate() {
            for (neuron_idx, &pos) in positions.iter().enumerate() {
                // Get activation if available
                let activation = if self.show_activations && layer_idx < activations.len() {
                    let layer_activations = &activations[layer_idx];
                    if neuron_idx < layer_activations.len() {
                        layer_activations[neuron_idx]
                    } else if !layer_activations.is_empty() {
                        // Average activation for compressed representation
                        layer_activations.iter().sum::<f32>() / layer_activations.len() as f32
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };
                
                // Neuron color based on activation
                let neuron_color = match architecture.color_scheme {
                    NetworkColorScheme::Policy => {
                        Color32::from_rgb(
                            (50.0 + activation * 150.0) as u8,
                            (100.0 + activation * 100.0) as u8,
                            255,
                        )
                    }
                    NetworkColorScheme::Value => {
                        Color32::from_rgb(
                            (50.0 + activation * 100.0) as u8,
                            (150.0 + activation * 105.0) as u8,
                            (50.0 + activation * 100.0) as u8,
                        )
                    }
                };
                
                // Draw neuron
                let radius = if layer_info.neurons > 100 { 3.0 } else { 5.0 };
                painter.circle_filled(pos, radius, neuron_color);
                
                // Activation glow
                if activation > 0.7 {
                    painter.circle_stroke(
                        pos,
                        radius + 2.0,
                        Stroke::new(1.0, neuron_color.gamma_multiply(0.5)),
                    );
                }
            }
            
            // Layer label
            painter.text(
                Pos2::new(positions[0].x, available_rect.bottom() - 40.0),
                egui::Align2::CENTER_TOP,
                &layer_info.name,
                FontId::proportional(10.0),
                Color32::GRAY,
            );
            
            // Activation function
            if !layer_info.activation_type.is_empty() {
                painter.text(
                    Pos2::new(positions[0].x, available_rect.bottom() - 20.0),
                    egui::Align2::CENTER_TOP,
                    &layer_info.activation_type,
                    FontId::proportional(9.0),
                    Color32::DARK_GRAY,
                );
            }
        }
        
        // Draw data flow arrows
        self.draw_data_flow(painter, &layer_positions, available_rect, architecture.color_scheme);
    }
    
    fn draw_data_flow(&self, painter: &Painter, layer_positions: &[Vec<Pos2>], rect: Rect, color_scheme: NetworkColorScheme) {
        let arrow_y = rect.top() + 20.0;
        let arrow_color = match color_scheme {
            NetworkColorScheme::Policy => Color32::from_rgb(100, 150, 255),
            NetworkColorScheme::Value => Color32::from_rgb(100, 200, 100),
        };
        
        // Input arrow
        if let Some(first_layer) = layer_positions.first() {
            if let Some(first_neuron) = first_layer.first() {
                let start = Pos2::new(rect.left() + 20.0, arrow_y);
                let end = Pos2::new(first_neuron.x - 20.0, arrow_y);
                
                painter.arrow(start, end - start, Stroke::new(2.0, arrow_color));
                painter.text(
                    start - Vec2::new(10.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    "Board\nState",
                    FontId::proportional(9.0),
                    arrow_color,
                );
            }
        }
        
        // Output arrow
        if let Some(last_layer) = layer_positions.last() {
            if let Some(last_neuron) = last_layer.first() {
                let start = Pos2::new(last_neuron.x + 20.0, arrow_y);
                let end = Pos2::new(rect.right() - 20.0, arrow_y);
                
                painter.arrow(start, end - start, Stroke::new(2.0, arrow_color));
                
                let label = match color_scheme {
                    NetworkColorScheme::Policy => "Move\nProbs",
                    NetworkColorScheme::Value => "Win\nRate",
                };
                
                painter.text(
                    end + Vec2::new(10.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(9.0),
                    arrow_color,
                );
            }
        }
    }
}