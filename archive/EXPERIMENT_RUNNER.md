# P2P Go Experiment Runner

## Quick Start

To test the network design without assumptions, run these experiments:

```bash
# 1. Test relay efficiency
cargo test --package p2pgo-network test_relay_efficiency -- --nocapture

# 2. Test training data quality  
cargo test --package trainer test_consensus_games -- --nocapture

# 3. Test gene marketplace (when implemented)
cargo test --package p2pgo-core test_gene_evolution -- --nocapture
```

## Experiment 1: Relay Credit System

### Setup
```rust
// In network/src/tests/relay_experiments.rs
#[tokio::test]
async fn test_relay_efficiency() {
    // Spawn 3 relays with different credit allocations
    let relay1 = spawn_relay_with_credits(1000);
    let relay2 = spawn_relay_with_credits(500);
    let relay3 = spawn_relay_with_credits(100);
    
    // Run 100 games through the network
    let results = run_games_batch(100, vec![relay1, relay2, relay3]).await;
    
    // Measure key metrics
    assert!(results.completion_rate > 0.95);
    assert!(results.avg_latency_ms < 50.0);
}
```

### Metrics to Collect
- Game completion rate
- Average move latency
- Credit consumption per game
- Relay participation distribution

## Experiment 2: Training Data Pipeline

### Setup
```rust
// In trainer/src/tests/data_quality.rs
#[test]
fn test_consensus_vs_no_consensus() {
    // Load games with consensus
    let consensus_games = GoDataset::from_cbor_dir("data/consensus_games").unwrap();
    
    // Load games without consensus  
    let no_consensus_games = GoDataset::from_cbor_dir("data/regular_games").unwrap();
    
    // Train models
    let model1 = train_model(consensus_games);
    let model2 = train_model(no_consensus_games);
    
    // Compare performance
    let perf1 = evaluate_model(model1);
    let perf2 = evaluate_model(model2);
    
    assert!(perf1.accuracy > perf2.accuracy);
}
```

### Data Collection Script
```bash
#!/bin/bash
# collect_training_data.sh

# Start relay with training collection enabled
P2PGO_COLLECT_TRAINING=true cargo run --bin p2pgo-relay &

# Run automated games
for i in {1..1000}; do
    cargo run --bin p2pgo-bot -- --games 1 --consensus true
done

# Package training data
tar -czf training_data_$(date +%Y%m%d).tar.gz data/training/
```

## Experiment 3: Gene Marketplace Testing

### Smart Contract Tests (Solidity)
```solidity
// test/GeneMarketplace.test.sol
contract GeneMarketplaceTest is Test {
    function testGeneBreeding() public {
        // Create two parent genes
        uint256 gene1 = marketplace.mintGene(modelHash1, proof1);
        uint256 gene2 = marketplace.mintGene(modelHash2, proof2);
        
        // Breed them
        uint256 childGene = marketplace.crossBreed(gene1, gene2);
        
        // Verify child inherits traits
        Gene memory child = marketplace.getGene(childGene);
        assertEq(child.generation, 2);
    }
}
```

### Performance Tracking
```python
# scripts/track_gene_performance.py
import matplotlib.pyplot as plt
import numpy as np

def track_gene_evolution(generations=10):
    performance = []
    
    for gen in range(generations):
        # Load generation data
        genes = load_generation(gen)
        
        # Evaluate each gene
        scores = [evaluate_gene(g) for g in genes]
        performance.append(np.mean(scores))
    
    # Plot performance curve
    plt.plot(performance)
    plt.xlabel('Generation')
    plt.ylabel('Average Win Rate')
    plt.savefig('gene_evolution.png')
```

## Experiment 4: Network Topology

### Test Different Relay Configurations
```yaml
# experiments/topology_test.yaml
experiments:
  - name: "star_topology"
    relays:
      central:
        credits: 10000
        bandwidth: 100
      edge:
        count: 5
        credits: 1000
        bandwidth: 10
        
  - name: "mesh_topology"
    relays:
      all:
        count: 6
        credits: 2000
        bandwidth: 20
        interconnect: full
        
  - name: "hierarchical"
    relays:
      tier1:
        count: 2
        credits: 5000
        bandwidth: 50
      tier2:
        count: 4
        credits: 1000
        bandwidth: 10
```

### Automated Test Runner
```rust
// src/bin/experiment_runner.rs
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(short, long)]
    experiment: String,
    
    #[structopt(short, long, default_value = "100")]
    iterations: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::from_args();
    
    match args.experiment.as_str() {
        "relay_efficiency" => run_relay_experiment(args.iterations).await,
        "training_quality" => run_training_experiment(args.iterations).await,
        "gene_evolution" => run_gene_experiment(args.iterations).await,
        "network_topology" => run_topology_experiment(args.iterations).await,
        _ => eprintln!("Unknown experiment: {}", args.experiment),
    }
}
```

## Data Analysis Pipeline

### 1. Collect Raw Data
```bash
# Start data collection
mkdir -p data/experiments/$(date +%Y%m%d)
export EXPERIMENT_DIR=data/experiments/$(date +%Y%m%d)

# Run experiments with data logging
RUST_LOG=info cargo run --bin experiment_runner -- \
    --experiment relay_efficiency \
    --iterations 1000 \
    2>&1 | tee $EXPERIMENT_DIR/relay_efficiency.log
```

### 2. Process Results
```python
# scripts/analyze_experiments.py
import pandas as pd
import json

def analyze_relay_efficiency(log_file):
    data = []
    with open(log_file) as f:
        for line in f:
            if 'METRIC:' in line:
                metric = json.loads(line.split('METRIC:')[1])
                data.append(metric)
    
    df = pd.DataFrame(data)
    
    # Calculate statistics
    print(f"Completion Rate: {df['completed'].mean():.2%}")
    print(f"Average Latency: {df['latency_ms'].mean():.1f}ms")
    print(f"Credit Efficiency: {df['credits_used'].mean():.1f} per game")
```

### 3. Visualize Results
```python
# scripts/visualize_results.py
import seaborn as sns
import matplotlib.pyplot as plt

def plot_credit_vs_performance():
    # Load experiment data
    df = pd.read_csv('data/credit_allocation_results.csv')
    
    # Create visualization
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(12, 5))
    
    # Credits vs Completion Rate
    sns.scatterplot(data=df, x='credits', y='completion_rate', ax=ax1)
    ax1.set_title('Credits vs Game Completion Rate')
    
    # Credits vs Latency
    sns.scatterplot(data=df, x='credits', y='avg_latency', ax=ax2)
    ax2.set_title('Credits vs Average Latency')
    
    plt.tight_layout()
    plt.savefig('results/credit_performance.png')
```

## Haskell Verification Tests

### Verify Game Rules
```haskell
-- test/Verification.hs
module Verification where

import Test.QuickCheck
import P2PGo.Core

-- Property: No suicide moves allowed
prop_noSuicide :: GameState -> Move -> Property
prop_noSuicide state move = 
  isValidMove state move ==> 
    not (isSuicideMove state move)

-- Property: Ko rule prevents immediate recapture
prop_koRule :: GameState -> Move -> Move -> Property
prop_koRule state move1 move2 =
  let state1 = applyMove state move1
      state2 = applyMove state1 move2
  in isCapture state move1 ==> 
     not (boardsEqual (getBoard state) (getBoard state2))

-- Run all properties
runVerification :: IO ()
runVerification = do
  quickCheck prop_noSuicide
  quickCheck prop_koRule
```

## Running All Experiments

```bash
#!/bin/bash
# run_all_experiments.sh

echo "=== P2P Go Network Experiments ==="
date

# 1. Network tests
echo "Testing relay efficiency..."
cargo test test_relay_efficiency -- --nocapture

# 2. Training tests  
echo "Testing training data quality..."
cargo test test_consensus_games -- --nocapture

# 3. Verification
echo "Running Haskell verification..."
cd verification && stack test

# 4. Performance benchmarks
echo "Running performance benchmarks..."
cargo bench

# 5. Generate report
echo "Generating experiment report..."
python scripts/generate_report.py

echo "=== Experiments Complete ==="
```

This framework allows systematic testing of each component without making assumptions about what will work best.