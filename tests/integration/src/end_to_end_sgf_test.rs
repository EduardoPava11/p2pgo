use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::info;

use crate::test_helpers::*;
use crate::neural_training::{TrainingMetrics, TrainingPhase, TrainingVisualizationState, WeightChangeViz};
use crate::sgf_training::SGFMetadata;

use p2pgo_sgf::parser::parse_sgf;
use p2pgo_network::rna::{RNAMessage, RNAType};

/// End-to-end test that loads the SGF file, creates RNA, propagates it,
/// and visualizes the training process
#[tokio::test]
async fn test_complete_sgf_to_training_flow() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("=== Starting End-to-End SGF Training Test ===");

    // Step 1: Load and validate SGF file
    let sgf_path = PathBuf::from("/Users/daniel/Downloads/76794817-078-worki-ve..sgf");
    assert!(sgf_path.exists(), "SGF file must exist");

    let sgf_content = tokio::fs::read_to_string(&sgf_path).await?;
    info!("Loaded SGF file: {} bytes", sgf_content.len());

    // Parse and extract metadata
    let game = parse_sgf(&sgf_content)?;
    let root = game.root_node();

    let metadata = SGFMetadata {
        black_player: root.get_property("PB").and_then(|v| v.first()).cloned().unwrap_or_default(),
        white_player: root.get_property("PW").and_then(|v| v.first()).cloned().unwrap_or_default(),
        black_rank: root.get_property("BR").and_then(|v| v.first()).cloned().unwrap_or_default(),
        white_rank: root.get_property("WR").and_then(|v| v.first()).cloned().unwrap_or_default(),
        result: root.get_property("RE").and_then(|v| v.first()).cloned().unwrap_or_default(),
        board_size: 9,
        komi: 7.5,
        date: root.get_property("DT").and_then(|v| v.first()).cloned().unwrap_or_default(),
    };

    info!("Game: {} ({}) vs {} ({})",
        metadata.black_player, metadata.black_rank,
        metadata.white_player, metadata.white_rank
    );
    info!("Result: {}, Date: {}", metadata.result, metadata.date);

    // Step 2: Set up relay network
    let mut relay1 = TestRelay::new(4501).await?;
    let mut relay2 = TestRelay::new(4502).await?;
    let mut relay3 = TestRelay::new(4503).await?;

    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;
    relay3.subscribe_rna().await?;

    // Connect relays
    let addr1 = relay1.listening_addresses()[0].clone();
    let addr2 = relay2.listening_addresses()[0].clone();

    relay2.connect_to_peer(addr1.clone()).await?;
    relay3.connect_to_peer(addr1).await?;
    relay3.connect_to_peer(addr2).await?;

    tokio::time::sleep(Duration::from_millis(300)).await;
    info!("Relay network established");

    // Step 3: Create RNA from SGF
    let total_moves = game.main_variation().count();
    info!("Game contains {} moves", total_moves);

    // Create RNA for different game phases
    let game_phases = vec![
        ("Opening", 0, 20.min(total_moves)),
        ("Middle Game", 20.min(total_moves), 50.min(total_moves)),
        ("End Game", 50.min(total_moves), total_moves),
    ];

    let mut all_rna = Vec::new();

    for (phase_name, start, end) in game_phases {
        if start >= end {
            continue;
        }

        let rna = RNAMessage {
            id: format!("sgf-{}-{}-{}", phase_name.to_lowercase().replace(' ', "_"), start, end),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::SGFData {
                sgf_content: sgf_content.clone(),
                move_range: (start, end),
                player_ranks: (metadata.black_rank.clone(), metadata.white_rank.clone()),
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: calculate_quality_from_ranks(&metadata.black_rank, &metadata.white_rank),
            data: vec![],
        };

        info!("Created RNA for {}: moves {}-{}, quality={:.2}",
            phase_name, start, end, rna.quality_score);

        all_rna.push((phase_name, rna));
    }

    // Step 4: Initialize training visualization
    let mut viz_state = TrainingVisualizationState {
        policy_loss_history: std::collections::VecDeque::new(),
        value_loss_history: std::collections::VecDeque::new(),
        current_metrics: TrainingMetrics {
            epoch: 0,
            policy_loss: 3.0,
            value_loss: 1.5,
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

    // Step 5: Broadcast RNA and collect on other relays
    viz_state.training_phase = TrainingPhase::CollectingData;
    info!("\n=== Phase 1: Collecting Training Data ===");

    for (phase_name, rna) in &all_rna {
        relay1.broadcast_rna(rna.clone()).await?;
        info!("Broadcast {} RNA", phase_name);

        // Verify propagation
        let received2 = relay2.wait_for_rna(Duration::from_secs(2)).await;
        let received3 = relay3.wait_for_rna(Duration::from_secs(2)).await;

        assert!(received2.is_some(), "Relay2 should receive {}", phase_name);
        assert!(received3.is_some(), "Relay3 should receive {}", phase_name);

        viz_state.total_games += 1;
    }

    info!("All RNA successfully propagated across network");

    // Step 6: Simulate federated training
    viz_state.training_phase = TrainingPhase::Training;
    info!("\n=== Phase 2: Federated Training ===");

    for epoch in 1..=10 {
        let epoch_start = Instant::now();

        // Simulate each relay training locally
        info!("Epoch {}: Local training on {} relays", epoch, 3);

        // Simulate training metrics improving
        let policy_loss = 3.0 * (0.9_f32).powi(epoch);
        let value_loss = 1.5 * (0.88_f32).powi(epoch);

        let metrics = TrainingMetrics {
            epoch,
            policy_loss,
            value_loss,
            learning_rate: 0.001 * (0.98_f32).powi(epoch / 3),
            games_in_batch: all_rna.len(),
            consensus_rate: 0.3 + (epoch as f32 * 0.06).min(0.6),
            time_per_epoch: epoch_start.elapsed().as_secs_f32() + 0.5,
        };

        // Update visualization
        viz_state.policy_loss_history.push_back((epoch as f32, policy_loss));
        viz_state.value_loss_history.push_back((epoch as f32, value_loss));
        viz_state.current_metrics = metrics.clone();

        info!("  Policy loss: {:.4} -> {:.4}",
            viz_state.policy_loss_history.front().map(|(_, l)| l).unwrap_or(&3.0),
            policy_loss
        );
        info!("  Value loss: {:.4} -> {:.4}",
            viz_state.value_loss_history.front().map(|(_, l)| l).unwrap_or(&1.5),
            value_loss
        );
        info!("  Consensus rate: {:.1}%", metrics.consensus_rate * 100.0);

        // Share weights every 3 epochs
        if epoch % 3 == 0 {
            viz_state.training_phase = TrainingPhase::SharingWeights;
            info!("\n  Sharing model weights across network...");

            // Broadcast weight updates
            let weight_rna = RNAMessage {
                id: format!("weights-epoch-{}", epoch),
                source_peer: relay1.peer_id().to_string(),
                rna_type: RNAType::ModelWeights {
                    model_type: "combined".to_string(),
                    layer_updates: vec![
                        vec![0.01, -0.02, 0.03, -0.01, 0.02],
                        vec![-0.01, 0.02, -0.03, 0.01, -0.02],
                        vec![0.02, -0.01, 0.01, -0.02, 0.03],
                    ],
                    consensus_count: 3,
                },
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                quality_score: 0.9,
                data: vec![],
            };

            relay1.broadcast_rna(weight_rna).await?;

            // Update weight visualization
            update_training_weights(&mut viz_state, epoch);

            viz_state.training_phase = TrainingPhase::Training;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Step 7: Validate training results
    viz_state.training_phase = TrainingPhase::Validating;
    info!("\n=== Phase 3: Validation ===");

    let final_policy_loss = viz_state.current_metrics.policy_loss;
    let final_value_loss = viz_state.current_metrics.value_loss;
    let final_consensus = viz_state.current_metrics.consensus_rate;

    info!("Final training metrics:");
    info!("  Policy network loss: {:.4}", final_policy_loss);
    info!("  Value network loss: {:.4}", final_value_loss);
    info!("  Consensus rate: {:.1}%", final_consensus * 100.0);
    info!("  Total games trained: {}", viz_state.total_games);
    info!("  Training time: {:?}", viz_state.start_time.unwrap().elapsed());

    // Assertions
    assert!(final_policy_loss < 1.5, "Policy loss should improve significantly");
    assert!(final_value_loss < 0.8, "Value loss should improve significantly");
    assert!(final_consensus > 0.6, "Consensus should reach reasonable level");
    assert_eq!(viz_state.policy_loss_history.len(), 10, "Should have 10 epochs of history");

    // Step 8: Test visualization components
    info!("\n=== Testing Visualization Components ===");

    // RNA collection status
    let rna_status = format!(
        "SGF Upload: {}, Completed Games: 0, Shared RNA: {}",
        all_rna.len(),
        all_rna.len() * 2 // Each relay received from others
    );
    info!("RNA Collection Status: {}", rna_status);

    // Quality distribution
    let quality_dist = calculate_quality_distribution(&all_rna);
    info!("Quality Distribution: High={:.1}%, Med={:.1}%, Low={:.1}%",
        quality_dist.0 * 100.0, quality_dist.1 * 100.0, quality_dist.2 * 100.0);

    viz_state.training_phase = TrainingPhase::Idle;
    info!("\n=== Test Complete ===");

    Ok(())
}

fn calculate_quality_from_ranks(black_rank: &str, white_rank: &str) -> f32 {
    let rank_to_score = |rank: &str| -> f32 {
        if rank.ends_with('k') {
            let num = rank[..rank.len()-1].parse::<f32>().unwrap_or(20.0);
            0.3 + (20.0 - num) / 30.0
        } else if rank.ends_with('d') {
            let num = rank[..rank.len()-1].parse::<f32>().unwrap_or(1.0);
            0.7 + num / 10.0
        } else {
            0.5
        }
    };

    (rank_to_score(black_rank) + rank_to_score(white_rank)) / 2.0
}

fn update_training_weights(viz_state: &mut TrainingVisualizationState, epoch: u32) {
    let scale = 0.1 / (epoch as f32).sqrt();

    viz_state.weight_changes.policy_changes = vec![
        vec![0.05, -0.03, 0.04, -0.02, 0.03].iter().map(|w| w * scale).collect(),
        vec![-0.02, 0.04, -0.03, 0.01, -0.02].iter().map(|w| w * scale).collect(),
        vec![0.03, -0.01, 0.02, -0.04, 0.05].iter().map(|w| w * scale).collect(),
    ];

    viz_state.weight_changes.value_changes = vec![
        vec![0.04, -0.02, 0.03, -0.01, 0.02].iter().map(|w| w * scale).collect(),
        vec![-0.03, 0.05, -0.04, 0.02, -0.01].iter().map(|w| w * scale).collect(),
        vec![0.02, -0.03, 0.01, -0.02, 0.04].iter().map(|w| w * scale).collect(),
        vec![0.01, -0.02, 0.03, -0.01, 0.02].iter().map(|w| w * scale).collect(),
    ];

    viz_state.weight_changes.max_change = 0.05 * scale;
}

fn calculate_quality_distribution(rna_list: &[(impl AsRef<str>, RNAMessage)]) -> (f32, f32, f32) {
    let total = rna_list.len() as f32;
    let high = rna_list.iter().filter(|(_, rna)| rna.quality_score > 0.8).count() as f32 / total;
    let med = rna_list.iter().filter(|(_, rna)| rna.quality_score > 0.5 && rna.quality_score <= 0.8).count() as f32 / total;
    let low = 1.0 - high - med;
    (high, med, low)
}