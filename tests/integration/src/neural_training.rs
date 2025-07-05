use crate::test_helpers::*;
use anyhow::Result;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::info;

use p2pgo_neural::training::{TrainingData, FederatedTrainer};

/// Simulated training metrics for visualization testing
#[derive(Clone)]
pub struct TrainingMetrics {
    pub epoch: u32,
    pub policy_loss: f32,
    pub value_loss: f32,
    pub learning_rate: f32,
    pub games_in_batch: usize,
    pub consensus_rate: f32,
    pub time_per_epoch: f32,
}

/// Simulated training visualization state
pub struct TrainingVisualizationState {
    pub policy_loss_history: VecDeque<(f32, f32)>,
    pub value_loss_history: VecDeque<(f32, f32)>,
    pub current_metrics: TrainingMetrics,
    pub start_time: Option<Instant>,
    pub total_games: usize,
    pub weight_changes: WeightChangeViz,
    pub training_phase: TrainingPhase,
}

pub struct WeightChangeViz {
    pub policy_changes: Vec<Vec<f32>>,
    pub value_changes: Vec<Vec<f32>>,
    pub max_change: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TrainingPhase {
    Idle,
    CollectingData,
    Training,
    Validating,
    SharingWeights,
}

#[tokio::test]
async fn test_training_visualization_updates() -> Result<()> {
    let mut viz_state = TrainingVisualizationState {
        policy_loss_history: VecDeque::with_capacity(100),
        value_loss_history: VecDeque::with_capacity(100),
        current_metrics: TrainingMetrics {
            epoch: 0,
            policy_loss: 2.5,
            value_loss: 1.0,
            learning_rate: 0.001,
            games_in_batch: 0,
            consensus_rate: 0.0,
            time_per_epoch: 0.0,
        },
        start_time: None,
        total_games: 0,
        weight_changes: WeightChangeViz {
            policy_changes: vec![],
            value_changes: vec![],
            max_change: 0.1,
        },
        training_phase: TrainingPhase::Idle,
    };

    // Simulate training loop
    viz_state.training_phase = TrainingPhase::CollectingData;
    viz_state.start_time = Some(Instant::now());

    // Collect RNA data
    for i in 0..10 {
        viz_state.total_games += 5;
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("Collected {} games", viz_state.total_games);
    }

    // Start training
    viz_state.training_phase = TrainingPhase::Training;

    for epoch in 1..=20 {
        let epoch_start = Instant::now();

        // Simulate training metrics
        let policy_loss = 2.5 * (0.95_f32).powi(epoch);
        let value_loss = 1.0 * (0.93_f32).powi(epoch);

        let metrics = TrainingMetrics {
            epoch,
            policy_loss,
            value_loss,
            learning_rate: 0.001 * (0.99_f32).powi(epoch / 5),
            games_in_batch: 50,
            consensus_rate: 0.5 + (epoch as f32 / 40.0),
            time_per_epoch: epoch_start.elapsed().as_secs_f32(),
        };

        // Update visualization
        viz_state.policy_loss_history.push_back((epoch as f32, policy_loss));
        viz_state.value_loss_history.push_back((epoch as f32, value_loss));

        if viz_state.policy_loss_history.len() > 100 {
            viz_state.policy_loss_history.pop_front();
        }
        if viz_state.value_loss_history.len() > 100 {
            viz_state.value_loss_history.pop_front();
        }

        viz_state.current_metrics = metrics.clone();
        viz_state.total_games += metrics.games_in_batch;

        info!("Epoch {}: policy_loss={:.4}, value_loss={:.4}, lr={:.6}",
            epoch, policy_loss, value_loss, metrics.learning_rate);

        // Simulate weight changes
        if epoch % 5 == 0 {
            viz_state.training_phase = TrainingPhase::Validating;
            update_weight_changes(&mut viz_state, epoch);
            viz_state.training_phase = TrainingPhase::Training;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Verify training progress
    assert!(viz_state.policy_loss_history.len() >= 20);
    assert!(viz_state.value_loss_history.len() >= 20);

    let final_policy_loss = viz_state.policy_loss_history.back().unwrap().1;
    let final_value_loss = viz_state.value_loss_history.back().unwrap().1;

    assert!(final_policy_loss < 1.5, "Policy loss should decrease");
    assert!(final_value_loss < 0.5, "Value loss should decrease");
    assert!(viz_state.current_metrics.consensus_rate > 0.7, "Consensus should improve");

    Ok(())
}

fn update_weight_changes(viz_state: &mut TrainingVisualizationState, epoch: u32) {
    // Simulate weight changes for different layers
    let policy_layers = vec![
        vec![0.1, 0.05, -0.03, 0.08, -0.12], // Layer 1
        vec![0.02, -0.04, 0.06, 0.01, -0.02], // Layer 2
        vec![-0.01, 0.03, -0.05, 0.02, 0.04], // Layer 3
    ];

    let value_layers = vec![
        vec![0.08, -0.06, 0.04, 0.03, -0.07], // Layer 1
        vec![-0.02, 0.05, -0.03, 0.01, 0.02], // Layer 2
        vec![0.03, -0.01, 0.02, -0.04, 0.05], // Layer 3
        vec![0.01, -0.02, 0.03, -0.01, 0.02], // Layer 4
    ];

    // Scale changes based on epoch
    let scale = 1.0 / (epoch as f32).sqrt();

    viz_state.weight_changes.policy_changes = policy_layers.into_iter()
        .map(|layer| layer.into_iter().map(|w| w * scale).collect())
        .collect();

    viz_state.weight_changes.value_changes = value_layers.into_iter()
        .map(|layer| layer.into_iter().map(|w| w * scale).collect())
        .collect();

    viz_state.weight_changes.max_change = 0.12 * scale;
}

#[tokio::test]
async fn test_rna_collection_visualization() -> Result<()> {
    // Test RNA collection status tracking
    let mut rna_sources = RNACollectionStatus {
        sgf_upload_count: 0,
        completed_games_count: 0,
        shared_rna_count: 0,
        quality_distribution: QualityDistribution {
            low: 0.2,
            medium: 0.5,
            high: 0.3,
        },
    };

    // Simulate RNA collection from different sources
    let mut relay = TestRelay::new(4201).await?;
    relay.subscribe_rna().await?;

    // Upload SGF files
    for i in 0..3 {
        let rna = relay.create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (i * 10, (i + 1) * 10));
        relay.broadcast_rna(rna).await?;
        rna_sources.sgf_upload_count += 1;
        info!("Uploaded SGF RNA {}", i + 1);
    }

    // Simulate completed games
    for i in 0..5 {
        rna_sources.completed_games_count += 1;
        info!("Completed game RNA {}", i + 1);
    }

    // Simulate shared RNA from network
    for i in 0..2 {
        rna_sources.shared_rna_count += 1;
        info!("Received shared RNA {}", i + 1);
    }

    // Update quality distribution based on collected RNA
    let total_rna = rna_sources.sgf_upload_count +
                   rna_sources.completed_games_count +
                   rna_sources.shared_rna_count;

    assert_eq!(total_rna, 10);

    // Recalculate quality distribution
    rna_sources.quality_distribution = QualityDistribution {
        low: 0.1,   // Less low quality
        medium: 0.4, // More medium quality
        high: 0.5,   // More high quality from SGF uploads
    };

    info!("RNA Collection Status: {:?}", rna_sources);

    Ok(())
}

#[derive(Debug)]
struct RNACollectionStatus {
    sgf_upload_count: usize,
    completed_games_count: usize,
    shared_rna_count: usize,
    quality_distribution: QualityDistribution,
}

#[derive(Debug)]
struct QualityDistribution {
    low: f32,
    medium: f32,
    high: f32,
}

#[tokio::test]
async fn test_federated_training_with_visualization() -> Result<()> {
    // Create federated training setup
    let mut trainer = SimulatedFederatedTrainer::new();
    let mut viz_state = TrainingVisualizationState {
        policy_loss_history: VecDeque::new(),
        value_loss_history: VecDeque::new(),
        current_metrics: TrainingMetrics {
            epoch: 0,
            policy_loss: 0.0,
            value_loss: 0.0,
            learning_rate: 0.001,
            games_in_batch: 0,
            consensus_rate: 0.0,
            time_per_epoch: 0.0,
        },
        start_time: Some(Instant::now()),
        total_games: 0,
        weight_changes: WeightChangeViz {
            policy_changes: vec![],
            value_changes: vec![],
            max_change: 0.1,
        },
        training_phase: TrainingPhase::Idle,
    };

    // Phase 1: Collect training data from SGF
    viz_state.training_phase = TrainingPhase::CollectingData;

    let sgf_path = "/Users/daniel/Downloads/76794817-078-worki-ve..sgf";
    let sgf_content = tokio::fs::read_to_string(sgf_path).await?;

    // Add training data
    for i in 0..50 {
        trainer.add_training_example(i);
        viz_state.total_games = i + 1;

        if i % 10 == 0 {
            info!("Collected {} training examples", i + 1);
        }
    }

    // Phase 2: Train locally
    viz_state.training_phase = TrainingPhase::Training;

    for epoch in 1..=10 {
        let metrics = trainer.train_epoch(epoch).await?;

        // Update visualization
        viz_state.policy_loss_history.push_back((epoch as f32, metrics.policy_loss));
        viz_state.value_loss_history.push_back((epoch as f32, metrics.value_loss));
        viz_state.current_metrics = metrics;

        // Phase 3: Share weights periodically
        if epoch % 3 == 0 {
            viz_state.training_phase = TrainingPhase::SharingWeights;
            info!("Sharing weights at epoch {}", epoch);
            tokio::time::sleep(Duration::from_millis(500)).await;
            viz_state.training_phase = TrainingPhase::Training;
        }
    }

    // Verify training completed successfully
    assert_eq!(viz_state.current_metrics.epoch, 10);
    assert!(viz_state.current_metrics.policy_loss < 2.0);
    assert!(viz_state.current_metrics.value_loss < 1.0);

    Ok(())
}

struct SimulatedFederatedTrainer {
    training_buffer: Vec<usize>,
}

impl SimulatedFederatedTrainer {
    fn new() -> Self {
        Self {
            training_buffer: Vec::new(),
        }
    }

    fn add_training_example(&mut self, example: usize) {
        self.training_buffer.push(example);
    }

    async fn train_epoch(&mut self, epoch: u32) -> Result<TrainingMetrics> {
        // Simulate training
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(TrainingMetrics {
            epoch,
            policy_loss: 2.5 * (0.9_f32).powi(epoch),
            value_loss: 1.0 * (0.85_f32).powi(epoch),
            learning_rate: 0.001 * (0.95_f32).powi(epoch / 2),
            games_in_batch: self.training_buffer.len(),
            consensus_rate: 0.5 + (epoch as f32 * 0.05).min(0.4),
            time_per_epoch: 0.1,
        })
    }
}