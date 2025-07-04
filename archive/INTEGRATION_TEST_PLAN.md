# Integration Test Plan for P2P Go MVP

## Overview
This document outlines step-by-step integration tests to validate each component as we connect them. Each test builds on the previous one, ensuring we catch issues early.

## Test Environment Setup

```bash
# Terminal 1 - First Relay (Discoverable)
export RUST_LOG=debug,libp2p=info
export P2PGO_TEST_MODE=true
cargo run --bin p2pgo-relay -- --port 4001

# Terminal 2 - Second Relay (Discoverer) 
export RUST_LOG=debug,libp2p=info
cargo run --bin p2pgo-relay -- --port 4002 --connect /ip4/127.0.0.1/tcp/4001/p2p/{PEER_ID}

# Terminal 3 - Test Monitor
cargo test --package integration-tests -- --nocapture
```

## Phase 1: Network Discovery Tests

### Test 1.1: Local mDNS Discovery
```rust
// tests/discovery/mdns_test.rs
#[tokio::test]
async fn test_local_mdns_discovery() {
    // Start relay 1
    let relay1 = TestRelay::new(4001).await;
    let peer_id1 = relay1.peer_id();
    
    // Start relay 2 
    let relay2 = TestRelay::new(4002).await;
    
    // Wait for mDNS discovery
    let discovered = relay2.wait_for_peer(peer_id1, Duration::from_secs(5)).await;
    assert!(discovered, "Relay 2 should discover Relay 1 via mDNS");
    
    // Verify connection type
    let conn_type = relay2.get_connection_type(peer_id1).await;
    assert_eq!(conn_type, ConnectionType::Local);
}
```

### Test 1.2: Direct Connection
```rust
#[tokio::test]
async fn test_direct_connection() {
    let relay1 = TestRelay::new(4001).await;
    let addr1 = relay1.listening_addresses()[0].clone();
    
    let relay2 = TestRelay::new(4002).await;
    
    // Direct dial
    relay2.connect_to_peer(addr1).await.unwrap();
    
    // Verify connected
    assert!(relay2.is_connected_to(relay1.peer_id()).await);
    
    // Measure connection latency
    let latency = relay2.measure_latency(relay1.peer_id()).await;
    assert!(latency < Duration::from_millis(10), "Local connection should be fast");
}
```

### Test 1.3: NAT Traversal
```rust
#[tokio::test] 
async fn test_circuit_relay_connection() {
    // Simulate NAT by binding to localhost only
    let relay_server = TestRelay::new_relay_server(4000).await;
    
    let relay1 = TestRelay::new_behind_nat(4001).await;
    relay1.connect_via_relay(&relay_server).await.unwrap();
    
    let relay2 = TestRelay::new_behind_nat(4002).await;
    relay2.connect_via_relay(&relay_server).await.unwrap();
    
    // Should establish circuit connection
    relay2.connect_to_peer_via_relay(relay1.peer_id(), &relay_server).await.unwrap();
    
    let conn_type = relay2.get_connection_type(relay1.peer_id()).await;
    assert_eq!(conn_type, ConnectionType::Relayed);
    
    // Test DCUtR upgrade
    tokio::time::sleep(Duration::from_secs(2)).await;
    let upgraded = relay2.get_connection_type(relay1.peer_id()).await;
    assert_eq!(upgraded, ConnectionType::Direct, "Should upgrade to direct connection");
}
```

## Phase 2: RNA (Training Data) Tests

### Test 2.1: SGF Upload and RNA Generation
```rust
// tests/rna/sgf_upload_test.rs
#[tokio::test]
async fn test_sgf_to_rna_conversion() {
    let sgf_content = std::fs::read_to_string("/Users/daniel/Downloads/76794817-078-worki-ve..sgf")
        .expect("SGF file should exist");
    
    let relay = TestRelay::new(4001).await;
    
    // Create RNA from SGF
    let rna = relay.create_sgf_rna(sgf_content.clone(), (0, 50));
    
    assert_eq!(rna.rna_type, RNAType::SGFData { .. });
    assert!(rna.data.len() > 0, "RNA should contain game data");
    
    // Broadcast RNA
    relay.broadcast_rna(rna.clone()).await.unwrap();
    
    // Verify in gossipsub
    let topics = relay.get_subscribed_topics().await;
    assert!(topics.contains(&"p2pgo/rna/v1"));
}
```

### Test 2.2: RNA Propagation
```rust
#[tokio::test]
async fn test_rna_propagation() {
    let relay1 = TestRelay::new(4001).await;
    let relay2 = TestRelay::new(4002).await;
    let relay3 = TestRelay::new(4003).await;
    
    // Connect in chain: 1 -> 2 -> 3
    relay2.connect_to_peer(relay1.listening_addresses()[0].clone()).await.unwrap();
    relay3.connect_to_peer(relay2.listening_addresses()[0].clone()).await.unwrap();
    
    // Subscribe to RNA topic
    relay2.subscribe_rna().await;
    relay3.subscribe_rna().await;
    
    // Relay 1 broadcasts RNA
    let sgf_rna = relay1.create_sgf_rna(SGF_TEST_DATA, (0, 20));
    relay1.broadcast_rna(sgf_rna.clone()).await.unwrap();
    
    // Should propagate to relay 2
    let received2 = relay2.wait_for_rna(Duration::from_secs(2)).await;
    assert!(received2.is_some(), "Relay 2 should receive RNA");
    
    // Should propagate to relay 3
    let received3 = relay3.wait_for_rna(Duration::from_secs(2)).await;
    assert!(received3.is_some(), "Relay 3 should receive RNA via relay 2");
    
    // Verify RNA content
    assert_eq!(received3.unwrap().source_peer, relay1.peer_id().to_string());
}
```

### Test 2.3: RNA Quality Evaluation
```rust
#[tokio::test]
async fn test_rna_quality_scoring() {
    let relay = TestRelay::new(4001).await;
    
    // Load multiple SGF files
    let sgf_files = vec![
        "/Users/daniel/Downloads/76794817-078-worki-ve..sgf",
        "/Users/daniel/Downloads/76794796-064-しろめし-dh.sgf",
        "/Users/daniel/Downloads/76794548-073-caminofrances-drunken master bot.sgf",
    ];
    
    for sgf_path in sgf_files {
        let sgf_content = std::fs::read_to_string(sgf_path).unwrap();
        let rna = relay.create_sgf_rna(sgf_content, (0, 100));
        
        // Parse and evaluate quality
        let game_state = parse_sgf_to_game_state(&rna);
        let quality_score = evaluate_game_quality(&game_state);
        
        println!("SGF: {} - Quality: {:.2}", sgf_path, quality_score);
        assert!(quality_score > 0.5, "Professional games should have high quality");
    }
}
```

## Phase 3: Neural Network Training Tests

### Test 3.1: Load Pre-trained Models
```rust
// tests/neural/model_loading_test.rs
#[test]
fn test_load_neural_models() {
    let policy_net = PolicyNetwork::load("models/policy_0090.onnx").unwrap();
    let value_net = ValueNetwork::load("models/value_0090.onnx").unwrap();
    
    // Test inference
    let test_board = create_test_position();
    let policy_pred = policy_net.predict(&test_board);
    let value_pred = value_net.evaluate(&test_board);
    
    assert!(!policy_pred.is_empty(), "Policy should predict moves");
    assert!(value_pred.win_probability >= -1.0 && value_pred.win_probability <= 1.0);
}
```

### Test 3.2: Training Data Collection
```rust
#[tokio::test]
async fn test_training_data_collection() {
    let mut trainer = FederatedTrainer::new();
    let relay = TestRelay::new(4001).await;
    
    // Collect RNA messages
    let rna_messages = relay.collect_rna_for_duration(Duration::from_secs(10)).await;
    
    // Convert to training data
    for rna in rna_messages {
        if let RNAType::SGFData { sgf_content, move_range } = rna.rna_type {
            let training_data = TrainingData::from_sgf(&sgf_content, move_range);
            trainer.add_training_data(training_data);
        }
    }
    
    assert!(trainer.training_buffer.len() > 0, "Should have collected training data");
}
```

### Test 3.3: Local Training Step
```rust
#[tokio::test]
async fn test_local_training() {
    let mut trainer = FederatedTrainer::new();
    
    // Add training data from SGF
    let training_data = load_sgf_training_data();
    for data in training_data {
        trainer.add_training_data(data);
    }
    
    // Perform training
    let metrics = trainer.train_local_batch().await.unwrap();
    
    assert!(metrics.policy_loss < 2.0, "Policy loss should be reasonable");
    assert!(metrics.value_loss < 1.0, "Value loss should be reasonable");
    assert_eq!(metrics.games_trained, trainer.training_buffer.len());
}
```

## Phase 4: UI Integration Tests

### Test 4.1: Network Visualization
```rust
// tests/ui/network_viz_test.rs
#[test]
fn test_network_visualization_update() {
    let mut viz = NetworkVisualization::new();
    
    // Add relays
    viz.add_relay("relay1".to_string(), Pos2::new(100.0, 100.0), true);
    viz.add_relay("relay2".to_string(), Pos2::new(300.0, 100.0), false);
    
    // Add connection
    viz.add_connection("relay1".to_string(), "relay2".to_string(), ConnectionType::Direct);
    
    // Send packet
    viz.send_packet("relay1", "relay2", RNAVisualizationType::GameData, 150.0);
    
    // Verify packet animation
    assert_eq!(viz.packets.len(), 1);
    assert_eq!(viz.stats.packets_sent, 1);
    assert_eq!(viz.stats.total_sent_kb, 150.0);
}
```

### Test 4.2: Neural Network Visualization During Training
```rust
#[test]
fn test_neural_training_visualization() {
    let mut nn_viz = NeuralNetworkVisualization::new();
    let mut trainer = FederatedTrainer::new();
    
    // Simulate training loop
    for epoch in 0..10 {
        // Get current activations
        let policy_act = trainer.policy_net.get_activations();
        let value_act = trainer.value_net.get_activations();
        
        // Update visualization
        nn_viz.update_activations(policy_act, value_act);
        
        // Verify visualization data
        assert!(!nn_viz.policy_activations.is_empty());
        assert!(!nn_viz.value_activations.is_empty());
        
        // Simulate training step
        trainer.train_step();
    }
}
```

### Test 4.3: SGF Upload UI
```rust
#[test]
fn test_sgf_upload_ui() {
    let mut sgf_tool = SGFUploadTool::new();
    
    // Simulate file selection
    sgf_tool.file_path = Some(PathBuf::from("/Users/daniel/Downloads/76794817-078-worki-ve..sgf"));
    sgf_tool.load_sgf();
    
    assert!(sgf_tool.parsed_game.is_some(), "Should parse SGF");
    assert!(sgf_tool.error.is_none(), "Should not have errors");
    
    // Test move range selection
    sgf_tool.move_range = (10, 30);
    let training_data = sgf_tool.create_training_data();
    
    assert!(training_data.is_some());
    assert_eq!(training_data.unwrap().moves.len(), 20);
}
```

## Phase 5: End-to-End Game Test

### Test 5.1: Complete Game Flow
```rust
#[tokio::test]
async fn test_complete_game_flow() {
    // Start two relays
    let relay1 = TestRelay::new_with_ui(4001).await;
    let relay2 = TestRelay::new_with_ui(4002).await;
    
    // Connect
    relay2.connect_to_peer(relay1.listening_addresses()[0].clone()).await.unwrap();
    
    // Create game on relay 1
    let game_id = relay1.create_game(9).await;
    
    // Join game on relay 2
    relay2.join_game(&game_id).await.unwrap();
    
    // Play moves
    let moves = vec![
        (3, 3), (15, 15), (15, 3), (3, 15),
        (9, 9), (9, 10), (10, 9), (10, 10),
    ];
    
    for (i, (x, y)) in moves.iter().enumerate() {
        let relay = if i % 2 == 0 { &relay1 } else { &relay2 };
        relay.play_move(*x, *y).await.unwrap();
        
        // Wait for sync
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Verify both have same state
        let state1 = relay1.get_game_state().await;
        let state2 = relay2.get_game_state().await;
        assert_eq!(state1.moves.len(), state2.moves.len());
    }
    
    // Test neural network predictions
    let predictions1 = relay1.get_move_predictions().await;
    let predictions2 = relay2.get_move_predictions().await;
    
    assert!(!predictions1.is_empty());
    assert!(!predictions2.is_empty());
}
```

## Continuous Monitoring

### Monitor Script
```bash
#!/bin/bash
# monitor_test.sh

while true; do
    clear
    echo "=== P2P Go Integration Monitor ==="
    echo
    
    # Check relay processes
    echo "Active Relays:"
    ps aux | grep p2pgo-relay | grep -v grep
    
    # Check network connections
    echo -e "\nNetwork Connections:"
    lsof -i :4001-4010 | grep ESTABLISHED
    
    # Check RNA propagation
    echo -e "\nRNA Messages (last 10):"
    tail -n 10 logs/rna_propagation.log
    
    # Check training metrics
    echo -e "\nTraining Metrics:"
    tail -n 5 logs/training_metrics.log
    
    sleep 2
done
```

## Test Data Setup

```bash
# setup_test_data.sh
#!/bin/bash

# Create test directories
mkdir -p test_data/sgf
mkdir -p test_data/models
mkdir -p logs

# Copy SGF files
cp /Users/daniel/Downloads/76794*.sgf test_data/sgf/

# Extract model files
cd test_data/models
tar -xzf ../../models/neural_models.tar.gz

# Start test logging
touch logs/rna_propagation.log
touch logs/training_metrics.log
touch logs/network_discovery.log
```

## Success Criteria

1. **Discovery**: Second relay finds first within 5 seconds
2. **RNA Propagation**: Training data reaches all connected relays
3. **Neural Training**: Loss decreases over 10 epochs
4. **UI Updates**: Visualizations update in real-time
5. **Game Sync**: Moves propagate within 500ms
6. **Model Predictions**: Heat map shows reasonable moves

## Debugging Tools

```rust
// Add to each test for debugging
#[derive(Debug)]
struct TestMetrics {
    discovery_time: Duration,
    rna_propagation_time: Duration,
    training_loss: f32,
    move_sync_latency: Duration,
    connection_type: ConnectionType,
}

impl TestMetrics {
    fn log(&self) {
        println!("Test Metrics: {:?}", self);
        // Also write to file for monitoring
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("logs/test_metrics.log")
            .unwrap();
        writeln!(file, "{:?} - {:?}", Instant::now(), self).unwrap();
    }
}