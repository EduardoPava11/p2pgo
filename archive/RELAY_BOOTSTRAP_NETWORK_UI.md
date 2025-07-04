# Relay Bootstrap & Living Network UI

## 1. Relay Bootstrap Questionnaire

Each question uses a 1-10 scale to set neural genome gradients:

```rust
// relay/src/bootstrap.rs
pub struct RelayBootstrap {
    pub questions: Vec<GradientQuestion>,
}

pub struct GradientQuestion {
    pub text: &'static str,
    pub low_label: &'static str,  // What 1 means
    pub high_label: &'static str, // What 10 means
    pub gene_mappings: Vec<GeneMapping>,
}

pub struct GeneMapping {
    pub gene_name: &'static str,
    pub scaling_function: fn(f32) -> f32, // Transform 1-10 to gene value
}

impl RelayBootstrap {
    pub fn new() -> Self {
        Self {
            questions: vec![
                GradientQuestion {
                    text: "On a scale of 1-10, how often do you secure territory early vs fight for influence?",
                    low_label: "Always secure territory",
                    high_label: "Always fight",
                    gene_mappings: vec![
                        GeneMapping {
                            gene_name: "territorial_instinct",
                            scaling_function: |x| (11.0 - x) / 10.0, // Invert: 10 = fight = low territory
                        },
                        GeneMapping {
                            gene_name: "combat_threshold",
                            scaling_function: |x| x / 10.0, // Direct: 10 = high combat
                        },
                        GeneMapping {
                            gene_name: "opening_aggression",
                            scaling_function: |x| (x - 5.0).max(0.0) / 5.0, // Only >5 is aggressive
                        },
                    ],
                },
                GradientQuestion {
                    text: "On a scale of 1-10, when losing, how often do you take risks vs play safe?",
                    low_label: "Always play safe",
                    high_label: "Always take risks",
                    gene_mappings: vec![
                        GeneMapping {
                            gene_name: "stress_risk_multiplier",
                            scaling_function: |x| x / 10.0,
                        },
                        GeneMapping {
                            gene_name: "mutation_under_pressure",
                            scaling_function: |x| (x / 10.0).powf(2.0), // Exponential for extremes
                        },
                        GeneMapping {
                            gene_name: "desperation_threshold",
                            scaling_function: |x| 1.0 - (x / 20.0), // Lower threshold = more desperate moves
                        },
                    ],
                },
                GradientQuestion {
                    text: "On a scale of 1-10, how many moves ahead do you typically calculate?",
                    low_label: "1-2 moves (intuition)",
                    high_label: "10+ moves (calculation)",
                    gene_mappings: vec![
                        GeneMapping {
                            gene_name: "search_depth",
                            scaling_function: |x| (x * 2.0).floor(), // 1-10 -> 2-20 plies
                        },
                        GeneMapping {
                            gene_name: "pruning_aggression",
                            scaling_function: |x| 1.0 - (x / 15.0), // Deep thinkers prune less
                        },
                        GeneMapping {
                            gene_name: "time_per_move",
                            scaling_function: |x| x * 3.0, // 3-30 seconds base time
                        },
                    ],
                },
                GradientQuestion {
                    text: "On a scale of 1-10, how much do you enjoy complex fighting positions?",
                    low_label: "Prefer simple positions",
                    high_label: "Love complexity",
                    gene_mappings: vec![
                        GeneMapping {
                            gene_name: "complexity_tolerance",
                            scaling_function: |x| x / 10.0,
                        },
                        GeneMapping {
                            gene_name: "branching_comfort",
                            scaling_function: |x| 5.0 + x, // 6-15 comfortable branches
                        },
                        GeneMapping {
                            gene_name: "chaos_enjoyment",
                            scaling_function: |x| (x > 7.0) as u8 as f32, // Binary: likes chaos or not
                        },
                    ],
                },
            ],
        }
    }
    
    pub fn generate_genome(&self, answers: Vec<u8>) -> NeuralGenome {
        let mut genome = NeuralGenome::default();
        
        for (question, answer) in self.questions.iter().zip(answers.iter()) {
            let normalized = *answer as f32; // 1-10
            
            for mapping in &question.gene_mappings {
                let gene_value = (mapping.scaling_function)(normalized);
                genome.set_gene(mapping.gene_name, gene_value);
            }
        }
        
        // Set activation functions based on preferences
        genome.architecture_genes.stress_activation = StressActivation {
            threshold: genome.get_gene("desperation_threshold").unwrap_or(0.5),
            response: genome.get_gene("stress_risk_multiplier").unwrap_or(0.5),
        };
        
        genome
    }
}
```

## 2. Living Network UI: RNA Transfer Visualization

The network should feel alive with RNA molecules flowing between relays:

```rust
// ui/src/network_visualization.rs
pub struct NetworkVisualization {
    /// Active RNA transfers
    rna_flows: Vec<RNAFlow>,
    /// Relay nodes
    relays: HashMap<RelayId, RelayNode>,
    /// Network activity pulse
    pulse_phase: f32,
}

pub struct RNAFlow {
    pub rna_type: RNAType,
    pub source: RelayId,
    pub destination: RelayId,
    pub progress: f32, // 0.0 to 1.0
    pub path: Vec<Point>,
    pub color: Color32,
    pub size: f32,
}

impl NetworkVisualization {
    pub fn render(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        
        // Draw relay nodes as cells
        for (relay_id, relay) in &self.relays {
            self.draw_relay_cell(painter, relay, rect);
        }
        
        // Draw RNA flows
        for flow in &mut self.rna_flows {
            flow.progress += 0.01; // Animate
            
            if flow.progress <= 1.0 {
                let pos = self.interpolate_position(flow);
                
                // RNA molecule visualization
                match flow.rna_type {
                    RNAType::MessengerRNA { .. } => {
                        // Large, slow moving training data
                        painter.circle_filled(pos, 8.0, flow.color);
                        painter.circle_stroke(pos, 12.0, Stroke::new(2.0, Color32::WHITE));
                    }
                    RNAType::MicroRNA { .. } => {
                        // Small, fast regulatory signals
                        painter.circle_filled(pos, 3.0, flow.color);
                        // Pulsing effect
                        let pulse = (self.pulse_phase * 10.0).sin() * 2.0;
                        painter.circle_stroke(pos, 3.0 + pulse, Stroke::new(1.0, flow.color));
                    }
                    RNAType::ProliferationRNA { .. } => {
                        // Star burst for game proliferation
                        self.draw_starburst(painter, pos, 6.0, flow.color);
                    }
                }
            }
        }
        
        // Remove completed flows
        self.rna_flows.retain(|f| f.progress <= 1.0);
        
        // Update pulse
        self.pulse_phase += 0.02;
    }
    
    fn draw_relay_cell(&self, painter: &Painter, relay: &RelayNode, rect: Rect) {
        let pos = relay.position;
        
        // Cell membrane
        painter.circle_stroke(pos, 30.0, Stroke::new(2.0, Color32::from_gray(100)));
        
        // Nucleus (neural net)
        let nucleus_color = self.genome_to_color(&relay.genome);
        painter.circle_filled(pos, 10.0, nucleus_color);
        
        // Activity indicators
        if relay.is_training {
            // Mitosis-like animation
            painter.circle_stroke(pos, 15.0, Stroke::new(1.0, Color32::GREEN));
        }
        
        // Health/reputation as cell brightness
        let health = relay.reputation as f32 / 100.0;
        let glow = Color32::from_rgba_premultiplied(
            255, 255, 255, (health * 50.0) as u8
        );
        painter.circle_filled(pos, 35.0, glow);
    }
}
```

## 3. Network Management Tools

```rust
// ui/src/network_tools.rs
pub struct NetworkManagementUI {
    /// RNA inbox
    rna_inbox: RNAInbox,
    /// Relay health monitor
    health_monitor: HealthMonitor,
    /// Gene pool explorer
    gene_pool: GenePoolExplorer,
}

pub struct RNAInbox {
    /// Incoming RNA molecules
    pending_rna: Vec<IncomingRNA>,
    /// Filters
    quality_threshold: f32,
    trusted_sources: HashSet<RelayId>,
}

impl RNAInbox {
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("RNA Inbox");
        
        // Quality filter slider
        ui.add(Slider::new(&mut self.quality_threshold, 0.0..=1.0)
            .text("Quality Filter"));
        
        ScrollArea::vertical().show(ui, |ui| {
            for rna in &self.pending_rna {
                ui.horizontal(|ui| {
                    // RNA type icon
                    match &rna.rna_type {
                        RNAType::MessengerRNA { quality_score, .. } => {
                            ui.colored_label(Color32::BLUE, "📜");
                            ui.label(format!("Training Data (Q: {:.2})", quality_score));
                        }
                        RNAType::MicroRNA { signal_type, .. } => {
                            ui.colored_label(Color32::YELLOW, "⚡");
                            ui.label(format!("Behavior Signal: {:?}", signal_type));
                        }
                        RNAType::ProliferationRNA { game_id, .. } => {
                            ui.colored_label(Color32::GREEN, "🌟");
                            ui.label(format!("Game Discovery: {}", game_id));
                        }
                    }
                    
                    // Accept/Reject buttons
                    if ui.button("✓").clicked() {
                        self.accept_rna(rna);
                    }
                    if ui.button("✗").clicked() {
                        self.reject_rna(rna);
                    }
                });
            }
        });
    }
}

pub struct HealthMonitor {
    /// Time series data
    metrics: TimeSeriesMetrics,
}

impl HealthMonitor {
    pub fn render(&mut self, ui: &mut Ui) {
        ui.heading("Relay Health");
        
        // Real-time metrics
        Grid::new("health_grid").show(ui, |ui| {
            ui.label("Games/Hour:");
            ui.label(format!("{}", self.metrics.games_per_hour));
            ui.end_row();
            
            ui.label("Win Rate:");
            ui.label(format!("{:.1}%", self.metrics.win_rate * 100.0));
            ui.end_row();
            
            ui.label("RNA Received:");
            ui.label(format!("{}/hr", self.metrics.rna_per_hour));
            ui.end_row();
            
            ui.label("Mutation Rate:");
            let color = if self.metrics.mutation_rate > 0.1 {
                Color32::RED
            } else {
                Color32::GREEN
            };
            ui.colored_label(color, format!("{:.3}", self.metrics.mutation_rate));
            ui.end_row();
        });
        
        // Stress indicators
        if self.metrics.stress_level > 0.7 {
            ui.colored_label(Color32::RED, "⚠️ HIGH STRESS - Neural net may mutate!");
        }
    }
}
```

## 4. Activation Functions

```rust
// core/src/activation_functions.rs
pub struct ActivationEngine {
    /// Historical data for calculations
    game_history: VecDeque<GameResult>,
    last_game_time: Instant,
    current_position: Option<BoardState>,
}

impl ActivationEngine {
    /// Stress level from losing streaks
    pub fn calculate_stress_activation(&self) -> f32 {
        let recent_games = self.game_history.iter().rev().take(10);
        let losses = recent_games.filter(|g| g.is_loss()).count();
        
        // Sigmoid activation: more losses = higher stress
        let loss_ratio = losses as f32 / 10.0;
        1.0 / (1.0 + (-10.0 * (loss_ratio - 0.5)).exp())
    }
    
    /// Time decay activation
    pub fn calculate_time_activation(&self) -> f32 {
        let elapsed = self.last_game_time.elapsed();
        
        // Peaks at 5-30 minutes, decays after
        if elapsed < Duration::from_secs(300) {
            // Building up (0-5 min)
            elapsed.as_secs_f32() / 300.0
        } else if elapsed < Duration::from_secs(1800) {
            // Peak performance (5-30 min)
            1.0
        } else {
            // Decay after 30 min
            (0.5_f32).powf(elapsed.as_secs_f32() / 3600.0 - 0.5)
        }
    }
    
    /// Neural confidence in position
    pub fn calculate_confidence_activation(&self, neural_net: &NeuralNet) -> f32 {
        if let Some(position) = &self.current_position {
            // Get top N move predictions
            let predictions = neural_net.predict_moves(position);
            
            // High confidence = clear best move
            // Low confidence = many similar moves
            let top_move_prob = predictions[0].probability;
            let second_move_prob = predictions[1].probability;
            
            // Ratio indicates confidence
            (top_move_prob / (second_move_prob + 0.001)).min(5.0) / 5.0
        } else {
            0.5 // Neutral when not in game
        }
    }
    
    /// Combined activation for UI proteins
    pub fn get_protein_activation(&self, protein_name: &str) -> f32 {
        let stress = self.calculate_stress_activation();
        let time = self.calculate_time_activation();
        let confidence = self.calculate_confidence_activation();
        
        match protein_name {
            "heat_map_protein" => {
                // Active when warmed up and confident
                time * confidence * (1.0 - stress * 0.5)
            }
            "suggestion_protein" => {
                // Most active when struggling but engaged
                time * stress * (1.0 - confidence)
            }
            "analysis_protein" => {
                // Active when warmed up, regardless of performance
                time * 0.8 + 0.2
            }
            _ => 0.5,
        }
    }
}
```

## 5. RNA Network Protocol

```rust
// network/src/rna_protocol.rs
pub struct RNAProtocol {
    /// Gossip-based RNA propagation
    propagation_rules: PropagationRules,
}

pub struct PropagationRules {
    /// TTL for different RNA types
    pub mrna_ttl: u8,      // Training data - travels far
    pub mirna_ttl: u8,     // Regulatory - local only
    pub trna_ttl: u8,      // Patterns - medium range
    pub proliferation_ttl: u8, // Game discovery - wide spread
}

impl RNAProtocol {
    /// Route RNA through network
    pub async fn propagate_rna(&self, rna: TrainingRNA, network: &RelayNetwork) {
        let ttl = match &rna.rna_type {
            RNAType::MessengerRNA { .. } => self.propagation_rules.mrna_ttl,
            RNAType::MicroRNA { .. } => self.propagation_rules.mirna_ttl,
            RNAType::TransferRNA { .. } => self.propagation_rules.trna_ttl,
            RNAType::ProliferationRNA { ttl, .. } => *ttl,
        };
        
        // Find compatible receivers
        let receivers = network.find_compatible_relays(&rna);
        
        // Propagate with decay
        for (relay, distance) in receivers {
            if distance < ttl {
                let mut forwarded_rna = rna.clone();
                forwarded_rna.decay(distance); // Reduce quality/strength
                
                relay.send_rna(forwarded_rna).await;
            }
        }
    }
}
```

## Making the Game Feel Alive

1. **Visual RNA Flows**: See training data flowing between relays
2. **Pulsing Network**: Activity creates visible pulses
3. **Cell-like Relays**: Relays look like living cells that grow/shrink
4. **Real-time Mutations**: Watch your neural net evolve under stress
5. **Social Indicators**: See which relays are "friends" (exchange RNA frequently)
6. **Health Metrics**: Monitor your relay's vital signs
7. **RNA Inbox**: Accept/reject incoming knowledge

This creates a living, breathing network where players feel connected to a larger organism!