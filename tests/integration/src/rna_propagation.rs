use crate::test_helpers::*;
use anyhow::Result;
use std::time::Duration;
use tracing::info;

use p2pgo_network::rna::{RNAMessage, RNAType};

#[tokio::test]
async fn test_rna_propagation_chain() -> Result<()> {
    // Create chain of relays: 1 -> 2 -> 3
    let mut relay1 = TestRelay::new(4301).await?;
    let mut relay2 = TestRelay::new(4302).await?;
    let mut relay3 = TestRelay::new(4303).await?;

    // Subscribe all to RNA topic
    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;
    relay3.subscribe_rna().await?;

    // Connect in chain
    let addr1 = relay1.listening_addresses()[0].clone();
    let addr2 = relay2.listening_addresses()[0].clone();

    relay2.connect_to_peer(addr1).await?;
    relay3.connect_to_peer(addr2).await?;

    // Wait for connections
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Relay 1 broadcasts RNA
    let sgf_rna = relay1.create_sgf_rna(test_data::SGF_TEST_DATA.to_string(), (0, 20));
    info!("Broadcasting RNA from relay1: {}", sgf_rna.id);
    relay1.broadcast_rna(sgf_rna.clone()).await?;

    // Should propagate to relay 2
    let received2 = relay2.wait_for_rna(Duration::from_secs(2)).await;
    assert!(received2.is_some(), "Relay 2 should receive RNA");
    assert_eq!(received2.unwrap().id, sgf_rna.id);
    info!("Relay 2 received RNA");

    // Should propagate to relay 3 via relay 2
    let received3 = relay3.wait_for_rna(Duration::from_secs(2)).await;
    assert!(received3.is_some(), "Relay 3 should receive RNA via relay 2");
    assert_eq!(received3.unwrap().id, sgf_rna.id);
    info!("Relay 3 received RNA");

    Ok(())
}

#[tokio::test]
async fn test_rna_mesh_propagation() -> Result<()> {
    // Create mesh network
    //    1---2
    //    |\ /|
    //    | X |
    //    |/ \|
    //    3---4

    let mut relays = vec![
        TestRelay::new(4304).await?,
        TestRelay::new(4305).await?,
        TestRelay::new(4306).await?,
        TestRelay::new(4307).await?,
    ];

    // Subscribe all to RNA
    for relay in &mut relays {
        relay.subscribe_rna().await?;
    }

    // Create mesh connections
    let addrs: Vec<_> = relays.iter()
        .map(|r| r.listening_addresses()[0].clone())
        .collect();

    // Connect each to all others
    for i in 0..4 {
        for j in 0..4 {
            if i != j {
                relays[i].connect_to_peer(addrs[j].clone()).await.ok();
            }
        }
    }

    // Wait for mesh to stabilize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test RNA from each relay reaches all others
    for source_idx in 0..4 {
        let rna = RNAMessage {
            id: format!("mesh-test-{}", source_idx),
            source_peer: relays[source_idx].peer_id().to_string(),
            rna_type: RNAType::ModelWeights {
                model_type: "policy".to_string(),
                layer_updates: vec![vec![0.1, 0.2, 0.3]],
                consensus_count: 3,
            },
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            quality_score: 0.9,
            data: vec![],
        };

        info!("Broadcasting from relay {}", source_idx);
        relays[source_idx].broadcast_rna(rna.clone()).await?;

        // All others should receive it
        for (target_idx, relay) in relays.iter_mut().enumerate() {
            if target_idx != source_idx {
                let received = relay.wait_for_rna(Duration::from_secs(2)).await;
                assert!(received.is_some(),
                    "Relay {} should receive RNA from relay {}", target_idx, source_idx);
                assert_eq!(received.unwrap().id, rna.id);
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_rna_types_propagation() -> Result<()> {
    let mut relay1 = TestRelay::new(4308).await?;
    let mut relay2 = TestRelay::new(4309).await?;

    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;

    // Connect
    let addr1 = relay1.listening_addresses()[0].clone();
    relay2.connect_to_peer(addr1).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test different RNA types
    let rna_types = vec![
        // SGF Data (mRNA)
        RNAMessage {
            id: "sgf-rna-1".to_string(),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::SGFData {
                sgf_content: test_data::SGF_TEST_DATA.to_string(),
                move_range: (0, 30),
                player_ranks: ("1d".to_string(), "2d".to_string()),
            },
            timestamp: 0,
            quality_score: 0.95,
            data: vec![],
        },
        // Pattern Data (tRNA)
        RNAMessage {
            id: "pattern-rna-1".to_string(),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::PatternData {
                pattern_type: "joseki".to_string(),
                board_region: (0, 0, 9, 9),
                frequency: 0.8,
            },
            timestamp: 0,
            quality_score: 0.85,
            data: vec![],
        },
        // Model Weights
        RNAMessage {
            id: "weight-rna-1".to_string(),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::ModelWeights {
                model_type: "value".to_string(),
                layer_updates: vec![
                    vec![0.01, -0.02, 0.03],
                    vec![-0.01, 0.02, -0.03],
                ],
                consensus_count: 5,
            },
            timestamp: 0,
            quality_score: 0.9,
            data: vec![],
        },
        // Regulatory Signal (miRNA)
        RNAMessage {
            id: "regulatory-rna-1".to_string(),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::RegulatorySignal {
                signal_type: "training_rate".to_string(),
                value: 0.0001,
                confidence: 0.95,
            },
            timestamp: 0,
            quality_score: 0.8,
            data: vec![],
        },
    ];

    // Broadcast each type
    for rna in &rna_types {
        info!("Broadcasting RNA type: {:?}", match &rna.rna_type {
            RNAType::SGFData { .. } => "SGFData",
            RNAType::PatternData { .. } => "PatternData",
            RNAType::ModelWeights { .. } => "ModelWeights",
            RNAType::RegulatorySignal { .. } => "RegulatorySignal",
        });

        relay1.broadcast_rna(rna.clone()).await?;

        let received = relay2.wait_for_rna(Duration::from_secs(1)).await;
        assert!(received.is_some(), "Should receive RNA type");

        let received_rna = received.unwrap();
        assert_eq!(received_rna.id, rna.id);
        assert_eq!(received_rna.quality_score, rna.quality_score);
    }

    Ok(())
}

#[tokio::test]
async fn test_rna_quality_filtering() -> Result<()> {
    let mut relay1 = TestRelay::new(4310).await?;
    let mut relay2 = TestRelay::new(4311).await?;

    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;

    // Connect
    let addr1 = relay1.listening_addresses()[0].clone();
    relay2.connect_to_peer(addr1).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send RNA with different quality scores
    let quality_scores = vec![0.3, 0.5, 0.7, 0.9, 0.95];

    for (i, quality) in quality_scores.iter().enumerate() {
        let rna = RNAMessage {
            id: format!("quality-test-{}", i),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::SGFData {
                sgf_content: test_data::SGF_TEST_DATA.to_string(),
                move_range: (0, 10),
                player_ranks: ("5k".to_string(), "5k".to_string()),
            },
            timestamp: 0,
            quality_score: *quality,
            data: vec![],
        };

        relay1.broadcast_rna(rna).await?;
    }

    // Collect all received RNA
    let mut received_rna = Vec::new();
    for _ in 0..quality_scores.len() {
        if let Some(rna) = relay2.wait_for_rna(Duration::from_millis(500)).await {
            received_rna.push(rna);
        }
    }

    // All should be received (filtering would be done at application level)
    assert_eq!(received_rna.len(), quality_scores.len());

    // Verify quality scores preserved
    for rna in &received_rna {
        info!("Received RNA {} with quality {}", rna.id, rna.quality_score);
        assert!(quality_scores.contains(&rna.quality_score));
    }

    Ok(())
}

#[tokio::test]
async fn test_rna_bandwidth_tracking() -> Result<()> {
    let mut relay1 = TestRelay::new(4312).await?;
    let mut relay2 = TestRelay::new(4313).await?;

    relay1.subscribe_rna().await?;
    relay2.subscribe_rna().await?;

    // Connect
    let addr1 = relay1.listening_addresses()[0].clone();
    relay2.connect_to_peer(addr1).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Track bandwidth
    let mut total_bytes_sent = 0usize;
    let start_time = tokio::time::Instant::now();

    // Send multiple RNA messages
    for i in 0..10 {
        let rna = RNAMessage {
            id: format!("bandwidth-test-{}", i),
            source_peer: relay1.peer_id().to_string(),
            rna_type: RNAType::ModelWeights {
                model_type: "policy".to_string(),
                layer_updates: vec![vec![0.1; 100]; 5], // Larger payload
                consensus_count: 3,
            },
            timestamp: 0,
            quality_score: 0.9,
            data: vec![0u8; 1000], // Additional data
        };

        let serialized = serde_cbor::to_vec(&rna)?;
        total_bytes_sent += serialized.len();

        relay1.broadcast_rna(rna).await?;

        // Verify received
        let received = relay2.wait_for_rna(Duration::from_secs(1)).await;
        assert!(received.is_some());
    }

    let elapsed = start_time.elapsed();
    let bandwidth_kbps = (total_bytes_sent as f64 * 8.0) / (elapsed.as_secs_f64() * 1000.0);

    info!("Sent {} bytes in {:?}", total_bytes_sent, elapsed);
    info!("Effective bandwidth: {:.2} kbps", bandwidth_kbps);

    assert!(total_bytes_sent > 10000, "Should send substantial data");
    assert!(bandwidth_kbps > 100.0, "Should achieve reasonable bandwidth");

    Ok(())
}