# P2P Go Network Design Specification

## Overview
This document maps out the integrated design of AI training, relay infrastructure, and marketplace for P2P Go.

## 1. AI Training Data Pipeline

### 1.1 Data Collection Architecture
```
Game Play → Consensus Agreement → CBOR Encoding → Training Pool
    ↓             ↓                    ↓              ↓
  Moves      Territory          Validated      Neural Network
  History    Agreement          Game Data      Training
```

### 1.2 Training Data Requirements
- **Move Sequences**: Complete game history with timestamps
- **Territory Consensus**: Both players agree on final territory
- **Player Profiles**: Guild classifications for style transfer
- **Game Metadata**: Board size, komi, rule variations

### 1.3 Data Quality Metrics
```haskell
-- Haskell type for validated training data
data TrainingGame = TrainingGame
  { moves :: [ValidatedMove]
  , consensus :: TerritoryConsensus
  , playerStyles :: (GuildProfile, GuildProfile)
  , gameQuality :: QualityScore
  }

data QualityScore = QualityScore
  { moveValidity :: Float  -- 0.0 to 1.0
  , consensusStrength :: Float
  , gameCompleteness :: Bool
  }
```

## 2. Relay Network as Training Infrastructure

### 2.1 Relay Roles
1. **Game Relay**: Forward game moves between players
2. **Training Relay**: Collect and validate training data
3. **Model Relay**: Distribute trained models
4. **Consensus Relay**: Facilitate territory agreement

### 2.2 Credit-Based Incentives
```
Player A ←→ Relay ←→ Player B
    ↓         ↓         ↓
 Credits   Training   Credits
           Data Pool
```

### 2.3 Relay Topology for Training
```
       Training Coordinator
       /        |        \
   Relay 1   Relay 2   Relay 3
   /    \    /    \    /    \
Game1  Game2  Game3  Game4  Game5
```

## 3. Smart Contract Gene Marketplace

### 3.1 Gene Definition
A "gene" is a trained model component that exhibits specific playing characteristics:
- Opening patterns
- Fighting style
- Endgame precision
- Territory management

### 3.2 Marketplace Architecture
```
Model Creator → Gene NFT → Marketplace → Model Consumer
      ↓            ↓           ↓              ↓
   Training    Ownership    Trading       Integration
    Proof       Token       Market         & Testing
```

### 3.3 Smart Contract Interface
```solidity
interface IGeneMarketplace {
    struct Gene {
        bytes32 modelHash;
        uint256 generation;
        uint256 winRate;
        GuildProfile style;
        address creator;
    }
    
    function mintGene(bytes32 modelHash, TrainingProof proof) returns (uint256);
    function crossBreed(uint256 gene1, uint256 gene2) returns (uint256);
    function evaluateGene(uint256 geneId) returns (Performance);
}
```

## 4. Testable Experiments

### 4.1 Experiment 1: Relay Efficiency
**Hypothesis**: Credit-based relays improve network reliability
**Test**: 
- Deploy 3 relay nodes
- Run 100 games with varying credit allocations
- Measure: latency, completion rate, data quality

### 4.2 Experiment 2: Training Data Quality
**Hypothesis**: Consensus-based games produce better training data
**Test**:
- Collect 1000 games with consensus
- Collect 1000 games without consensus
- Train models on each dataset
- Compare model performance

### 4.3 Experiment 3: Gene Evolution
**Hypothesis**: Crossbreeding genes improves performance
**Test**:
- Create 10 base genes from different play styles
- Breed top performers
- Track performance over generations

## 5. Implementation Phases

### Phase 1: Core Infrastructure (Weeks 1-4)
- [ ] Implement training data collection in relays
- [ ] Create CBOR schema for training games
- [ ] Build validation pipeline in Haskell

### Phase 2: Training Pipeline (Weeks 5-8)
- [ ] Integrate PyTorch/TensorFlow training
- [ ] Create model evaluation framework
- [ ] Implement distributed training coordinator

### Phase 3: Marketplace MVP (Weeks 9-12)
- [ ] Deploy smart contracts on testnet
- [ ] Create gene minting interface
- [ ] Implement basic trading mechanics

### Phase 4: Integration Testing (Weeks 13-16)
- [ ] End-to-end game → training → marketplace flow
- [ ] Performance benchmarking
- [ ] Security audit

## 6. Formal Verification with Haskell

### 6.1 Verify Game Rules
```haskell
-- Prove that captures are correctly calculated
captureCorrectness :: GameState -> Move -> Property
captureCorrectness state move = 
  let (newState, captures) = applyMove state move
  in length captures == countCapturedStones state move
```

### 6.2 Verify Consensus Protocol
```haskell
-- Prove consensus convergence
consensusConvergence :: Player -> Player -> Eventually Territory
consensusConvergence p1 p2 = 
  eventually $ agreementReached p1 p2
```

### 6.3 Verify Training Data Integrity
```haskell
-- Prove training data maintains game invariants
trainingDataValid :: TrainingGame -> Bool
trainingDataValid game = 
  all moveIsLegal (moves game) &&
  consensusMatchesFinalBoard (consensus game) (finalBoard game)
```

## 7. Network Testing Framework

### 7.1 Local Test Environment
```bash
# Spawn test network
./scripts/spawn_test_network.sh --relays 3 --players 6

# Run experiments
./scripts/run_experiment.sh relay_efficiency
./scripts/run_experiment.sh training_quality
./scripts/run_experiment.sh gene_evolution
```

### 7.2 Metrics Collection
- Move latency distribution
- Consensus agreement time
- Training data throughput
- Model performance curves
- Gene trading volume

### 7.3 A/B Testing Framework
```yaml
experiment:
  name: "credit_allocation"
  variants:
    - name: "baseline"
      credits_per_game: 100
    - name: "high_incentive"
      credits_per_game: 500
  metrics:
    - relay_participation_rate
    - game_completion_rate
    - training_data_quality
```

## 8. Economic Model

### 8.1 Token Flow
```
Players → Play Games → Earn Credits → Buy Genes
   ↓                      ↓              ↓
Provide            Relay Games      Improve
Training Data      For Others       Performance
```

### 8.2 Value Creation
- **Players**: Better opponents, training data rewards
- **Relays**: Transaction fees, training coordination rewards
- **Developers**: Gene sales, model improvements
- **Network**: Stronger AI, more engaging games

## Next Steps

1. **Implement Phase 1** focusing on training data collection
2. **Deploy test relays** with measurement infrastructure
3. **Create Haskell verification** suite for core protocols
4. **Design gene encoding** format for model components
5. **Build experiment runner** for automated testing

This specification provides a foundation for building and testing each component systematically without making assumptions about what will work.