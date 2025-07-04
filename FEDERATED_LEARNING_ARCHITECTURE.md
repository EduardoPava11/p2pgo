# P2P Go Federated Learning Architecture

## Vision: From 9x9 Go to Decentralized Intelligence

### Phase 1: Bootstrap with 9x9 Go (Launch Ready)
- **Current State**: Two-player P2P Go with neural networks
- **Immediate Goal**: Launch beta with friend testing
- **Key Features**:
  - 9x9 board for fast training cycles (hours not days)
  - JSON-serialized micro-nets (~50KB compressed)
  - Local self-play training
  - Heat map visualization (toggle with H)
  - SGF file import for pre-training

### Phase 2: Federated Learning Network (Beta)

#### 2.1 Micro-Net Architecture
```json
{
  "architecture": {
    "conv_stem": 1,
    "residual_blocks": 6,
    "filters": 64,
    "parameters": "~65k",
    "size_compressed": "<50KB"
  },
  "components": {
    "policy_head": "softmax(81) for 9x9 moves",
    "value_head": "win probability estimation"
  }
}
```

#### 2.2 Federated Training Loop
1. **Local Training** (On each device):
   - Self-play generates training data
   - Clip gradients, quantize to 8-bit
   - Generate `delta.json` with weight updates
   
2. **Gossip Protocol**:
   - Publish deltas to `/fl/relay` topic
   - Circuit-v2 mesh distributes updates
   - 10-minute aggregation rounds
   
3. **SuperNode Aggregation**:
   - FedAvg merges ~50 deltas
   - Knowledge distillation creates student model
   - Publish `student.onnx` back to network

### Phase 3: Relay Network Optimization

#### 3.1 Dual-Purpose Neural Networks
Each device maintains THREE micro-nets:

1. **Go Policy-Value Net**:
   - Input: 9x9 board state (17 planes)
   - Output: Move probabilities + win estimation
   
2. **Relay Policy-Value Net**:
   - Input: Network graph state tensor
   - Policy: Choose {Direct, Friend, SuperNode} path
   - Value: Predict RTT/QoS score
   
3. **Distilled Joint Model**:
   - Combined knowledge from network ensemble
   - Updated hourly via federated learning

#### 3.2 Network Health Metrics
- Fragmentation detection via graph analysis
- Node churn prediction
- Optimal relay placement
- Bandwidth allocation

### Phase 4: Higher-Order Applications

#### 4.1 Crypto Market Integration

**Option A: Polkadot Parachain**
```rust
// Substrate pallet for neural net marketplace
pub trait NeuralNetMarket {
    fn list_model(cid: Cid, price: Balance) -> Result<()>;
    fn purchase_model(cid: Cid) -> Result<Model>;
    fn stake_for_training(amount: Balance) -> Result<()>;
}
```

**Option B: Lightning Network**
- Micropayments for model access
- Pay-per-inference API
- Training bounties in sats
- Model improvement rewards

#### 4.2 Advanced Go Variants
1. **9x9x9 Three-Player Go**:
   - Forces coalition dynamics
   - Tests multi-agent learning
   - New strategic depth
   
2. **13x13 Tournaments**:
   - Gradual complexity increase
   - Transfer learning from 9x9
   - Competitive ranking system

#### 4.3 Decentralized AI Services
- **Model Marketplace**: Buy/sell trained nets
- **Inference API**: Pay-per-use predictions
- **Training Pools**: Collaborative learning
- **Oracle Services**: Blockchain data feeds

### Phase 5: Implementation Roadmap

#### Q1 2025: Foundation
- [x] Basic P2P Go with neural nets
- [ ] 9x9 federated learning prototype
- [ ] JSON weight serialization
- [ ] Basic gossip protocol

#### Q2 2025: Network Intelligence
- [ ] Relay optimization neural net
- [ ] SuperNode aggregation
- [ ] Knowledge distillation pipeline
- [ ] Network health monitoring

#### Q3 2025: Crypto Integration
- [ ] Lightning Network payments
- [ ] Model marketplace MVP
- [ ] Training incentives
- [ ] Polkadot research

#### Q4 2025: Advanced Features
- [ ] 9x9x9 three-player mode
- [ ] 13x13 tournaments
- [ ] Cross-domain learning
- [ ] Mobile optimization

### Technical Specifications

#### Storage Requirements
- Per device: 3 models × 50KB = 150KB
- Delta updates: ~30KB per round
- Bandwidth: <1MB per hour active use

#### Training Performance
- 9x9 self-play: 200k games = 24h to pro level
- Inference: <50ms on mobile
- FedAvg round: <30s on WiFi
- KD compilation: 2-4 min on SuperNode

#### Security Considerations
- Commit-reveal for move validation
- Gradient clipping for privacy
- Stake-based Sybil resistance
- Encrypted model transfer

### Monetization Strategy

1. **Freemium Model**:
   - Free: Basic 9x9 play
   - Paid: Advanced training features
   - Premium: Market access

2. **Token Economics**:
   - Train-to-earn rewards
   - Relay node incentives
   - Model staking yields
   - Tournament prizes

3. **B2B Services**:
   - Private relay networks
   - Custom AI training
   - White-label deployments
   - Enterprise oracles

### Success Metrics

- **Launch**: 100 active players
- **Beta**: 1,000 federated trainers
- **Growth**: 10,000 relay nodes
- **Scale**: 100,000 micro-nets
- **Revenue**: $1M annual from services

---

## Next Steps

1. **Immediate** (This Week):
   - Test 2-player relay connection
   - Implement delta.json export
   - Create training visualizer
   
2. **Short Term** (Next Month):
   - FedAvg aggregator prototype
   - SuperNode role detection
   - Basic model marketplace UI
   
3. **Medium Term** (Q2 2025):
   - Lightning integration
   - 9x9x9 game rules
   - Mobile optimization

This architecture leverages small-board Go as a perfect bootstrapping mechanism for a larger vision: a decentralized intelligence network where every device contributes to and benefits from collective learning.