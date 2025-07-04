# Biological Computing Model for P2P Go

## Overview

- **DNA** = Neural Network architectures (each player has slight variations)
- **RNA** = CBOR-encoded training data packets (game records, moves, patterns)
- **Proteins** = UI functions that change based on the neural net's "expression"

This creates a living system where neural nets influence how the game interface behaves.

## 1. DNA: Neural Network Genomes

Each player's neural net has a unique "genome" with slight variations:

```rust
// core/src/neural_genome.rs
pub struct NeuralGenome {
    /// Base architecture genes
    pub architecture_genes: ArchitectureGenes,
    /// Learning behavior genes  
    pub learning_genes: LearningGenes,
    /// Expression control genes
    pub regulatory_genes: RegulatoryGenes,
    /// Mutation history
    pub lineage: Vec<Mutation>,
}

pub struct ArchitectureGenes {
    /// Number of hidden layers (2-5)
    pub depth_gene: u8,
    /// Width multiplier for each layer (0.5x - 2.0x)
    pub width_genes: Vec<f32>,
    /// Activation function preferences
    pub activation_genes: Vec<ActivationType>,
    /// Skip connection probability (0.0 - 1.0)
    pub skip_connection_rate: f32,
    /// Attention mechanism strength
    pub attention_strength: f32,
}

pub struct LearningGenes {
    /// Base learning rate (0.0001 - 0.1)
    pub learning_rate_gene: f32,
    /// Momentum coefficient
    pub momentum_gene: f32,
    /// Dropout rate for regularization
    pub dropout_gene: f32,
    /// Batch size preference (16 - 128)
    pub batch_size_gene: u32,
    /// Memory retention (how much history to keep)
    pub memory_gene: f32,
}

pub struct RegulatoryGenes {
    /// Which genes are active in different game phases
    pub phase_expression: HashMap<GamePhase, Vec<GeneActivation>>,
    /// Stress response (how to adapt when losing)
    pub stress_response: StressResponse,
    /// Exploration vs exploitation balance
    pub curiosity_gene: f32,
}

#[derive(Clone)]
pub enum ActivationType {
    ReLU,
    LeakyReLU(f32),
    Tanh,
    GELU,
    Swish,
}

pub struct Mutation {
    pub generation: u32,
    pub gene_affected: String,
    pub old_value: f32,
    pub new_value: f32,
    pub trigger: MutationTrigger,
}

pub enum MutationTrigger {
    PerformancePressure,    // Losing too much
    EnvironmentalChange,    // New opponent styles
    RandomDrift,           // Natural variation
    HorizontalTransfer,    // Learning from others
}
```

### DNA Expression Example

```rust
impl NeuralGenome {
    /// Build actual neural net from genome
    pub fn express(&self) -> NeuralNetwork {
        let mut layers = Vec::new();
        
        // Input layer (361 for 19x19 board)
        let input_size = 361;
        let mut prev_size = input_size;
        
        // Hidden layers based on genes
        for i in 0..self.architecture_genes.depth_gene {
            let width = (256.0 * self.architecture_genes.width_genes[i as usize]) as usize;
            let activation = &self.architecture_genes.activation_genes[i as usize];
            
            layers.push(Layer {
                weights: Matrix::random(prev_size, width),
                bias: Vector::zeros(width),
                activation: activation.clone(),
                dropout: self.learning_genes.dropout_gene,
            });
            
            // Add skip connection based on gene
            if rand::random::<f32>() < self.architecture_genes.skip_connection_rate {
                layers.push(SkipConnection::new(prev_size, width));
            }
            
            prev_size = width;
        }
        
        // Output layer (361 for move probabilities)
        layers.push(Layer {
            weights: Matrix::random(prev_size, 361),
            bias: Vector::zeros(361),
            activation: ActivationType::Softmax,
            dropout: 0.0,
        });
        
        NeuralNetwork {
            layers,
            learning_rate: self.learning_genes.learning_rate_gene,
            momentum: self.learning_genes.momentum_gene,
        }
    }
}
```

## 2. RNA: Training Data Messages

RNA molecules carry information between cells - our CBOR packets do the same:

```rust
// core/src/training_rna.rs
use serde::{Serialize, Deserialize};

/// RNA molecule - carries training information
#[derive(Serialize, Deserialize)]
pub struct TrainingRNA {
    /// Unique RNA identifier
    pub rna_id: [u8; 32],
    /// Type of RNA (messenger, transfer, etc.)
    pub rna_type: RNAType,
    /// The actual training payload
    pub codon_sequence: Vec<Codon>,
    /// Origin cell (which neural net produced this)
    pub source_dna: NeuralGenomeHash,
    /// Timestamp
    pub transcribed_at: u64,
}

#[derive(Serialize, Deserialize)]
pub enum RNAType {
    /// Full game record (mRNA)
    MessengerRNA {
        game_record: GameRecord,
        consensus_achieved: bool,
        quality_score: f32,
    },
    /// Pattern snippet (tRNA)
    TransferRNA {
        pattern: PatternRecord,
        context: BoardRegion,
        success_rate: f32,
    },
    /// Regulatory signal (miRNA)
    MicroRNA {
        signal_type: RegulatorySignal,
        strength: f32,
        target_genes: Vec<String>,
    },
    /// Style transfer (lncRNA)
    LongNonCodingRNA {
        style_vector: Vec<f32>,
        source_player: PlayerId,
        game_phase: GamePhase,
    },
}

/// Codon - 3 base pairs encoding game information
#[derive(Serialize, Deserialize)]
pub struct Codon {
    pub base1: MoveFeature,
    pub base2: BoardFeature, 
    pub base3: TemporalFeature,
}

#[derive(Serialize, Deserialize)]
pub enum MoveFeature {
    Placement(Coord),
    Pass,
    Resign,
    Territory(Vec<Coord>),
}

#[derive(Serialize, Deserialize)]
pub enum BoardFeature {
    LocalPattern([u8; 9]),  // 3x3 pattern around move
    Liberties(u8),
    CaptureCount(u8),
    EyeSpace(bool),
}

#[derive(Serialize, Deserialize)]
pub enum TemporalFeature {
    TimeSpent(u32),        // Milliseconds
    MoveNumber(u32),
    PhaseTransition(bool),
    ClockPressure(f32),
}

/// Pattern extracted from games
#[derive(Serialize, Deserialize)]
pub struct PatternRecord {
    pub pattern_hash: [u8; 32],
    pub frequency: u32,
    pub win_rate: f32,
    pub moves: Vec<Coord>,
    pub response_map: HashMap<Coord, f32>, // Best responses
}
```

### RNA Transcription Process

```rust
impl GameRecord {
    /// Transcribe game into RNA molecules
    pub fn transcribe(&self, genome: &NeuralGenome) -> Vec<TrainingRNA> {
        let mut rna_molecules = Vec::new();
        
        // 1. Create mRNA for full game
        let mrna = TrainingRNA {
            rna_id: blake3::hash(&self.to_bytes()).as_bytes().try_into().unwrap(),
            rna_type: RNAType::MessengerRNA {
                game_record: self.clone(),
                consensus_achieved: self.consensus_achieved,
                quality_score: self.calculate_quality(),
            },
            codon_sequence: self.encode_as_codons(),
            source_dna: genome.hash(),
            transcribed_at: current_timestamp(),
        };
        rna_molecules.push(mrna);
        
        // 2. Extract tRNA for important patterns
        for pattern in self.extract_patterns() {
            if pattern.frequency > 3 && pattern.win_rate > 0.6 {
                let trna = TrainingRNA {
                    rna_id: blake3::hash(&pattern.pattern_hash).as_bytes().try_into().unwrap(),
                    rna_type: RNAType::TransferRNA {
                        pattern: pattern.clone(),
                        context: pattern.get_board_region(),
                        success_rate: pattern.win_rate,
                    },
                    codon_sequence: pattern.encode_as_codons(),
                    source_dna: genome.hash(),
                    transcribed_at: current_timestamp(),
                };
                rna_molecules.push(trna);
            }
        }
        
        // 3. Generate regulatory miRNA based on game outcome
        if self.is_crushing_defeat() {
            let mirna = TrainingRNA {
                rna_id: random_rna_id(),
                rna_type: RNAType::MicroRNA {
                    signal_type: RegulatorySignal::ReduceAggression,
                    strength: 0.8,
                    target_genes: vec!["exploration_rate".into(), "risk_tolerance".into()],
                },
                codon_sequence: vec![],
                source_dna: genome.hash(),
                transcribed_at: current_timestamp(),
            };
            rna_molecules.push(mirna);
        }
        
        rna_molecules
    }
}
```

## 3. Proteins: UI Functions

Proteins are the workhorses - they do things. Our UI functions change based on neural net expression:

```rust
// ui/src/protein_functions.rs
pub trait ProteinFunction {
    /// Which genes control this protein's expression
    fn controlling_genes(&self) -> Vec<String>;
    
    /// Express the function based on gene activation
    fn express(&self, activation_level: f32, genome: &NeuralGenome) -> Box<dyn Fn(&mut egui::Ui)>;
}

/// Stone placement preview protein
pub struct StonePlacementProtein;

impl ProteinFunction for StonePlacementProtein {
    fn controlling_genes(&self) -> Vec<String> {
        vec!["attention_strength".into(), "exploration_rate".into()]
    }
    
    fn express(&self, activation_level: f32, genome: &NeuralGenome) -> Box<dyn Fn(&mut egui::Ui)> {
        let attention = genome.architecture_genes.attention_strength;
        let exploration = genome.regulatory_genes.curiosity_gene;
        
        Box::new(move |ui: &mut egui::Ui| {
            // High attention = show heat map of likely moves
            if attention > 0.7 {
                ui.label("Neural attention map:");
                // Draw heat map based on neural net's attention
            }
            
            // High exploration = show unusual move suggestions
            if exploration > 0.8 && activation_level > 0.5 {
                ui.label("Experimental moves:");
                // Highlight unconventional options
            }
            
            // Low activation = minimal UI
            if activation_level < 0.3 {
                ui.set_visible(false);
            }
        })
    }
}

/// Territory estimation protein
pub struct TerritoryVisualizationProtein;

impl ProteinFunction for TerritoryVisualizationProtein {
    fn controlling_genes(&self) -> Vec<String> {
        vec!["pattern_recognition".into(), "spatial_reasoning".into()]
    }
    
    fn express(&self, activation_level: f32, genome: &NeuralGenome) -> Box<dyn Fn(&mut egui::Ui)> {
        Box::new(move |ui: &mut egui::Ui| {
            let opacity = activation_level;
            
            // Neural net confident = solid territory display
            if activation_level > 0.8 {
                // Show territory with high confidence
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba(255, 0, 0, (opacity * 255.0) as u8)
                );
            } else if activation_level > 0.5 {
                // Show territory with uncertainty
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(2.0, Color32::from_rgba(255, 0, 0, (opacity * 255.0) as u8))
                );
            }
            // Low activation = no territory display
        })
    }
}

/// Move timing suggestion protein
pub struct TimingProtein;

impl ProteinFunction for TimingProtein {
    fn controlling_genes(&self) -> Vec<String> {
        vec!["tempo_sensitivity".into(), "patience_factor".into()]
    }
    
    fn express(&self, activation_level: f32, genome: &NeuralGenome) -> Box<dyn Fn(&mut egui::Ui)> {
        let tempo_gene = genome.get_gene("tempo_sensitivity").unwrap_or(0.5);
        
        Box::new(move |ui: &mut egui::Ui| {
            if tempo_gene > 0.7 && activation_level > 0.6 {
                // Urgent move indicator
                ui.colored_label(Color32::RED, "⚡ Critical timing!");
            } else if tempo_gene < 0.3 && activation_level > 0.5 {
                // Patient play indicator
                ui.colored_label(Color32::GREEN, "🐢 Take your time");
            }
        })
    }
}

/// Style adaptation protein
pub struct StyleAdaptationProtein;

impl ProteinFunction for StyleAdaptationProtein {
    fn express(&self, activation_level: f32, genome: &NeuralGenome) -> Box<dyn Fn(&mut egui::Ui)> {
        let current_style = genome.compute_style_vector();
        
        Box::new(move |ui: &mut egui::Ui| {
            // Morph UI based on playing style
            match current_style.dominant_trait() {
                StyleTrait::Territorial => {
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = 
                        Stroke::new(2.0, Color32::from_rgb(0, 100, 0));
                }
                StyleTrait::Fighting => {
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = 
                        Stroke::new(3.0, Color32::from_rgb(200, 0, 0));
                }
                StyleTrait::Balanced => {
                    // Default style
                }
            }
            
            // Show style indicator
            if activation_level > 0.4 {
                ui.label(format!("Playing style: {}", current_style.describe()));
            }
        })
    }
}
```

## 4. Gene Expression System

The central dogma: DNA → RNA → Proteins

```rust
// core/src/gene_expression.rs
pub struct GeneExpressionEngine {
    /// Current genome
    genome: NeuralGenome,
    /// RNA molecules in the system
    rna_pool: Vec<TrainingRNA>,
    /// Active proteins
    expressed_proteins: HashMap<String, Box<dyn ProteinFunction>>,
    /// Environmental factors affecting expression
    environment: GameEnvironment,
}

pub struct GameEnvironment {
    pub current_board: BoardState,
    pub opponent_style: StyleVector,
    pub time_pressure: f32,
    pub game_phase: GamePhase,
    pub recent_performance: f32,
}

impl GeneExpressionEngine {
    /// Main expression loop
    pub fn update_expression(&mut self, dt: f32) {
        // 1. Environmental signals affect gene activation
        let gene_activation = self.compute_gene_activation(&self.environment);
        
        // 2. Process RNA molecules
        for rna in &self.rna_pool {
            match &rna.rna_type {
                RNAType::MessengerRNA { .. } => {
                    // Train neural net with game data
                    self.process_training_rna(rna);
                }
                RNAType::MicroRNA { signal_type, strength, target_genes } => {
                    // Regulatory RNA modifies gene expression
                    for gene in target_genes {
                        gene_activation.modify(gene, *strength);
                    }
                }
                _ => {}
            }
        }
        
        // 3. Update protein expression based on activation
        for (protein_name, protein) in &mut self.expressed_proteins {
            let controlling_genes = protein.controlling_genes();
            let activation = controlling_genes.iter()
                .map(|g| gene_activation.get(g))
                .sum::<f32>() / controlling_genes.len() as f32;
            
            // Protein folding (UI function generation)
            let ui_function = protein.express(activation, &self.genome);
            self.update_ui_binding(protein_name, ui_function);
        }
        
        // 4. Mutation under stress
        if self.environment.recent_performance < 0.3 {
            self.stress_induced_mutation();
        }
    }
    
    /// Horizontal gene transfer - learn from other players
    pub fn horizontal_transfer(&mut self, donor_rna: TrainingRNA) {
        if let RNAType::LongNonCodingRNA { style_vector, .. } = donor_rna.rna_type {
            // Incorporate successful patterns from other players
            let compatibility = self.genome.calculate_compatibility(&style_vector);
            
            if compatibility > 0.7 {
                // High compatibility - integrate the pattern
                self.genome.integrate_style_elements(style_vector);
            }
        }
    }
}
```

## 5. Evolution and Inheritance

```rust
pub struct Evolution {
    pub population: Vec<NeuralGenome>,
    pub generation: u32,
}

impl Evolution {
    /// Sexual reproduction - combine two genomes
    pub fn crossover(&self, parent1: &NeuralGenome, parent2: &NeuralGenome) -> NeuralGenome {
        let mut child = NeuralGenome::default();
        
        // Crossover architecture genes
        child.architecture_genes = if rand::random() {
            parent1.architecture_genes.clone()
        } else {
            parent2.architecture_genes.clone()
        };
        
        // Mix learning genes
        child.learning_genes.learning_rate_gene = 
            (parent1.learning_genes.learning_rate_gene + 
             parent2.learning_genes.learning_rate_gene) / 2.0;
        
        // Mutation
        if rand::random::<f32>() < 0.1 {
            child.mutate();
        }
        
        child
    }
}
```

## Biological Principles Applied

1. **Central Dogma**: DNA (NN architecture) → RNA (training data) → Proteins (UI functions)
2. **Gene Expression**: Environmental factors (game state) control which genes are active
3. **Epigenetics**: Recent games modify gene expression without changing DNA
4. **Horizontal Transfer**: Learn patterns from other players' RNA
5. **Evolution**: Successful neural nets reproduce and pass on traits
6. **Phenotype**: The actual UI behavior is the observable phenotype

This creates a living, evolving system where each player's neural net develops its own "personality" that manifests in how the UI behaves.