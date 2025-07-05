# P2P Go Architecture Map

## Module Dependency Hierarchy

```
┌─────────────────────────────────────────────────────────┐
│                        UI Layer                         │
│  ui-egui/ (egui-based UI)                             │
│  - app.rs: Main app state & event handling            │
│  - worker.rs: Network worker thread                    │
│  - msg.rs: UI<->Network message types                 │
│  - heat_map.rs & dual_heat_map.rs: Neural overlays    │
│  - network_panel.rs: Relay mode controls              │
└────────────────────────┬───────────────────────────────┘
                         │ uses
┌────────────────────────▼───────────────────────────────┐
│                   Network Layer                         │
│  network/ (libp2p-based P2P)                          │
│  - lib.rs: NodeContext, main network interface        │
│  - lobby.rs: Game discovery & matchmaking             │
│  - game_channel/: Game state synchronization          │
│  - relay_config.rs: Relay modes & stats               │
│  - simple_p2p.rs: Direct P2P connections              │
└────────────────────────┬───────────────────────────────┘
                         │ uses
┌────────────────────────▼───────────────────────────────┐
│                    Neural Layer                         │
│  neural/ (Burn-based ML)                               │
│  - lib.rs: DualNeuralNet interface                    │
│  - relay_net.rs: Relay decision network               │
│  - training.rs: SGF->CBOR training pipeline           │
└────────────────────────┬───────────────────────────────┘
                         │ uses
┌────────────────────────▼───────────────────────────────┐
│                     Core Layer                          │
│  core/ (Game logic)                                    │
│  - game_state.rs: Board state & move validation       │
│  - scoring.rs: Territory calculation & ScoreProof     │
│  - value_labeller.rs: Training data annotation        │
│  - archiver.rs: CBOR game serialization               │
│  - sgf.rs: SGF parsing for training                   │
└─────────────────────────────────────────────────────────┘
```

## Critical Data Flows to Test

### 1. Game Flow
```
User Input → UI → Worker → Network → GameChannel → Core GameState
                                  ↓
                    Remote Node ← Network ← GameChannel
```

### 2. Relay Mode Flow  
```
UI Panel → SetRelayMode → Worker → NodeContext → libp2p Config
                                                ↓
                                          Circuit Relay V2
```

### 3. Training Data Flow
```
Game End → ScoreProof → Archiver → CBOR File → Training Pipeline
                     ↓
              Value Labeller → Neural Network Training
```

### 4. Heat Map Flow
```
GameState → Neural Net → Predictions → Heat Map Overlay → UI Render
```

## Test Coverage Requirements

### Core Module Tests
- [x] Move validation
- [x] Capture logic  
- [x] Ko rule enforcement
- [ ] Score calculation accuracy
- [ ] CBOR serialization/deserialization
- [ ] SGF parsing for all variants

### Network Module Tests  
- [ ] libp2p node initialization
- [ ] Peer discovery via Kademlia
- [ ] Circuit Relay V2 setup
- [ ] Game channel message ordering
- [ ] Network recovery after disconnect
- [ ] NAT traversal scenarios

### Neural Module Tests
- [ ] Model loading and inference
- [ ] Heat map generation
- [ ] Training data conversion (SGF→CBOR)
- [ ] Dual network predictions
- [ ] Relay decision network

### UI Module Tests
- [ ] Message passing (UI↔Worker)
- [ ] State synchronization
- [ ] Heat map rendering
- [ ] Relay mode UI updates
- [ ] Score dialog flow
- [ ] Error handling display

## Integration Points to Verify

1. **Core ↔ Network**: GameState synchronization
2. **Network ↔ UI**: Event propagation  
3. **Core ↔ Neural**: Board evaluation
4. **Neural ↔ UI**: Heat map display
5. **Network ↔ Neural**: Relay decisions
6. **Core ↔ Storage**: CBOR archiving