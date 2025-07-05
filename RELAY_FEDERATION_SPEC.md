# P2P Go Relay Federation & Federated Learning Reward System

## Core Concept: Teaching Games as Training Currency

The relay service becomes a **federated learning marketplace** where:
- High-quality games (teaching moments) are identified and valued
- Players contributing valuable training data earn MCP tokens
- API consumers pay tokens to access crowd-trained models
- Relay maintainers earn from facilitating this exchange

## 1. Game Value Classification System

### High-Value Teaching Games

#### Type A: High ELO Differential, Low Variance
```rust
pub struct TeachingGame {
    /// ELO difference between players
    elo_differential: i32, // > 200 points
    /// Game variance (mistake rate)
    variance: f32, // < 0.2
    /// Teaching value score
    teaching_score: f32,
    /// Specific lessons demonstrated
    lessons: Vec<Lesson>,
}

pub enum Lesson {
    OpeningTheory { joseki: String },
    MiddleGameStrategy { concept: String },
    EndgameAccuracy { points_saved: i32 },
    TacticalSequence { complexity: u8 },
}
```

These games show **what TO do** - expert play demonstrating correct patterns.

#### Type B: Low ELO Differential, High Variance  
```rust
pub struct DogfightGame {
    /// ELO difference between players
    elo_differential: i32, // < 50 points
    /// Game variance (dramatic swings)
    variance: f32, // > 0.8
    /// Combat intensity score
    combat_score: f32,
    /// Critical turning points
    turning_points: Vec<TurningPoint>,
}

pub struct TurningPoint {
    move_number: u32,
    evaluation_swing: f32,
    mistake_type: MistakeType,
    recovery_sequence: Option<Vec<Move>>,
}
```

These games show **what NOT to do** and **how to recover** - valuable for resilience training.

## 2. Federated Learning Chain Protocol

### Snapshot Backpropagation Chain
```rust
pub struct FederatedChain {
    /// Participating nodes in training chain
    participants: Vec<ChainNode>,
    /// Games selected for this round
    training_games: Vec<(GameId, GameValue)>,
    /// Gradient checkpoints
    snapshots: Vec<GradientSnapshot>,
    /// Consensus mechanism
    consensus: ChainConsensus,
}

pub struct ChainNode {
    peer_id: PeerId,
    neural_net: NeuralNetHash,
    contribution_score: f32,
    mcp_balance: u64,
}

pub async fn execute_training_chain(
    chain: &FederatedChain,
    games: Vec<ClassifiedGame>,
) -> Result<ChainResult> {
    // Step 1: Each node trains on assigned games
    let local_gradients = chain.participants
        .par_iter()
        .map(|node| node.train_on_games(&games))
        .collect();
    
    // Step 2: Secure gradient aggregation
    let aggregated = secure_aggregate(local_gradients)?;
    
    // Step 3: Snapshot and propagate
    let snapshot = GradientSnapshot::create(aggregated);
    
    // Step 4: Backpropagate through chain
    for node in chain.participants.iter().rev() {
        node.apply_gradient_snapshot(&snapshot)?;
    }
    
    Ok(ChainResult {
        model_improvement: calculate_improvement(&snapshot),
        participant_rewards: calculate_rewards(&chain),
    })
}
```

## 3. MCP Token Economics

### Token Distribution Model
```rust
pub struct TokenEconomics {
    /// Base reward per teaching game
    base_teaching_reward: u64, // 10 MCP
    /// Base reward per dogfight game
    base_dogfight_reward: u64, // 5 MCP
    /// Multipliers for quality
    quality_multipliers: QualityMultipliers,
    /// API pricing tiers
    api_pricing: ApiPricing,
}

pub struct QualityMultipliers {
    /// For games that demonstrate rare patterns
    rarity_bonus: f32, // 1.5x - 3x
    /// For games with clear teaching moments
    clarity_bonus: f32, // 1.2x - 2x
    /// For games that improve model accuracy
    accuracy_bonus: f32, // 1.1x - 5x
}

pub fn calculate_game_reward(
    game: &ClassifiedGame,
    model_impact: f32,
) -> u64 {
    let base = match game.game_type {
        GameType::Teaching => 10,
        GameType::Dogfight => 5,
    };
    
    let multiplier = 
        game.rarity_score * 1.5 +
        game.clarity_score * 1.2 +
        model_impact * 3.0;
    
    (base as f32 * multiplier) as u64
}
```

### API Monetization
```rust
pub struct ApiService {
    /// Crowd-trained models available
    models: HashMap<ModelId, CrowdModel>,
    /// Pricing per inference
    pricing: PricingTier,
    /// Revenue distribution
    revenue_split: RevenueSplit,
}

pub struct RevenueSplit {
    /// To data contributors
    contributors: f32, // 40%
    /// To relay maintainers
    maintainers: f32, // 30%
    /// To model trainers
    trainers: f32, // 20%
    /// To protocol treasury
    treasury: f32, // 10%
}

pub async fn handle_api_request(
    request: InferenceRequest,
    payment: MpcPayment,
) -> Result<InferenceResponse> {
    // Verify payment
    verify_payment(&payment)?;
    
    // Execute inference
    let result = crowd_model.infer(&request.board_state)?;
    
    // Distribute payment
    distribute_revenue(payment, &revenue_split).await?;
    
    Ok(InferenceResponse {
        suggested_move: result.best_move,
        confidence: result.confidence,
        alternative_moves: result.alternatives,
        crowd_consensus: result.consensus_strength,
    })
}
```

## 4. Relay Service Architecture

### High Availability Design
```rust
pub struct RelayCluster {
    /// Primary relay nodes
    primary_nodes: Vec<RelayNode>,
    /// Standby nodes
    standby_nodes: Vec<RelayNode>,
    /// Load balancer
    load_balancer: GeographicLoadBalancer,
    /// Health monitor
    health_monitor: HealthMonitor,
    /// Failover coordinator
    failover: FailoverCoordinator,
}

pub struct RelayNode {
    /// Node identifier
    node_id: NodeId,
    /// Geographic location
    location: GeoLocation,
    /// Current load
    load_metrics: LoadMetrics,
    /// Federated learning capability
    fl_enabled: bool,
    /// MCP token staking
    staked_tokens: u64,
}

impl RelayCluster {
    pub async fn route_game_connection(
        &self,
        player: &Player,
    ) -> Result<RelayNode> {
        // Find closest healthy relay
        let candidates = self.primary_nodes.iter()
            .filter(|node| node.is_healthy())
            .filter(|node| node.has_capacity());
        
        // Geographic routing for low latency
        let best_node = self.load_balancer
            .select_by_geography(player.location, candidates)?;
        
        Ok(best_node)
    }
    
    pub async fn coordinate_fl_round(
        &self,
        games: Vec<ClassifiedGame>,
    ) -> Result<FlRoundResult> {
        // Select FL-capable nodes
        let fl_nodes = self.primary_nodes.iter()
            .filter(|node| node.fl_enabled)
            .filter(|node| node.staked_tokens >= MIN_STAKE);
        
        // Create training chain
        let chain = FederatedChain::new(fl_nodes, games)?;
        
        // Execute training
        let result = chain.execute().await?;
        
        // Distribute rewards
        self.distribute_rewards(result).await?;
        
        Ok(result)
    }
}
```

### Load Balancing Strategy
```rust
pub struct GeographicLoadBalancer {
    /// Region definitions
    regions: HashMap<RegionId, Region>,
    /// Latency matrix between regions
    latency_matrix: LatencyMatrix,
    /// Current load distribution
    load_distribution: LoadMap,
}

impl GeographicLoadBalancer {
    pub fn select_optimal_relay(
        &self,
        player_location: GeoLocation,
        game_type: GameType,
    ) -> Result<RelayNode> {
        // For teaching games, prefer stable, high-bandwidth relays
        if game_type == GameType::Teaching {
            return self.select_teaching_relay(player_location);
        }
        
        // For dogfight games, prefer low-latency relays
        if game_type == GameType::Dogfight {
            return self.select_combat_relay(player_location);
        }
        
        // Default selection
        self.select_nearest_relay(player_location)
    }
}
```

## 5. Implementation Roadmap

### Phase 1: Game Classification Engine (Week 1-2)
```haskell
-- Haskell game analyzer compiled to WASM
module GameClassifier where

data GameFeatures = GameFeatures
  { eloDifferential :: Int
  , moveVariance :: Float
  , mistakeRate :: Float
  , gameLength :: Int
  , criticalMoments :: [CriticalMoment]
  }

classifyGame :: Game -> IO ClassifiedGame
classifyGame game = do
  features <- extractFeatures game
  value <- calculateTeachingValue features
  return $ ClassifiedGame game features value

-- Compile to WASM for edge computation
compileToWasm :: IO ()
compileToWasm = wasmCompile "game-classifier" classifyGame
```

### Phase 2: Relay Federation (Week 3-4)
1. Implement relay discovery protocol
2. Create health monitoring system
3. Build failover coordination
4. Add geographic routing
5. Test with 10+ relay nodes

### Phase 3: Federated Learning Chain (Week 5-6)
1. Implement secure aggregation
2. Create snapshot backpropagation
3. Build consensus mechanism
4. Add reward calculation
5. Test chain execution

### Phase 4: MCP Token Integration (Week 7-8)
1. Design token economics
2. Implement payment channels
3. Create API gateway
4. Build revenue distribution
5. Test end-to-end flow

## 6. Monitoring & Logging Requirements

### Game Activity Logging
```rust
#[derive(Debug, Serialize)]
pub struct GameActivityLog {
    /// Game identifier
    game_id: GameId,
    /// Players involved
    players: (PlayerId, PlayerId),
    /// Classification result
    classification: GameClassification,
    /// Teaching value score
    teaching_value: f32,
    /// Moves with annotations
    annotated_moves: Vec<AnnotatedMove>,
    /// Network operations
    network_ops: Vec<NetworkOperation>,
}

#[derive(Debug, Serialize)]
pub struct NetworkOperation {
    timestamp: Timestamp,
    operation: OpType,
    relay_node: NodeId,
    latency_ms: u32,
    success: bool,
    error: Option<String>,
}
```

### Real-time Dashboard
```rust
pub struct RelayDashboard {
    /// Active games by type
    active_games: HashMap<GameType, u32>,
    /// Token flow metrics
    token_metrics: TokenFlowMetrics,
    /// Model performance
    model_metrics: ModelMetrics,
    /// Network health
    network_health: NetworkHealth,
}
```

## 7. Testing Strategy

### Local Testing Setup
```bash
# Start local relay cluster
./scripts/start_relay_cluster.sh --nodes 3 --fl-enabled

# Generate test games
./scripts/generate_test_games.sh --teaching 100 --dogfight 50

# Run federated learning round
./scripts/run_fl_round.sh --games ./test_games --verify

# Test API endpoints
./scripts/test_api.sh --endpoint predict_move --auth $MCP_TOKEN
```

### DMG Distribution Testing
```bash
# Build universal DMG with relay support
./scripts/build_universal_dmg.sh --with-relay --with-fl

# Test on multiple machines
./scripts/test_dmg_distribution.sh --machines "mac1,mac2,mac3"

# Verify P2P connectivity
./scripts/verify_p2p_network.sh --check-relay --check-games
```

## 8. Success Metrics

### Network Health
- Relay uptime: > 99.9%
- Game connection success: > 95%
- FL round completion: > 90%
- Failover time: < 30s

### Economic Health  
- Daily active players: > 1000
- Games classified/day: > 10,000
- MCP tokens earned/day: > 100,000
- API calls/day: > 50,000

### Model Performance
- Move prediction accuracy: > 75%
- Teaching game identification: > 90%
- Model convergence: < 50 rounds
- Inference latency: < 100ms

This architecture creates a self-sustaining ecosystem where better games lead to better models, which attract more API users, generating revenue that rewards contributors and maintainers.