# MVP Checklist for P2P Go

## Current Status
- ✅ **Offline game** - Fully working with scoring, territory marking
- ✅ **Game logic** - Complete with captures, ko rule, scoring
- ✅ **UI components** - Board, 3D view, SGF support
- ✅ **Neural models** - Policy and value networks implemented
- ❌ **Network layer** - Compilation errors, incomplete
- ❌ **Integration** - Components not connected

## Critical Path to MVP

### 1. Fix Network Compilation (Priority: HIGH)
```rust
// network/src/behaviour.rs
- [ ] Fix libp2p v0.53 API changes:
  - [ ] Update SwarmBuilder syntax
  - [ ] Fix relay::client::Behaviour constructor
  - [ ] Update gossipsub configuration
  - [ ] Fix NetworkBehaviour derive macro

// network/Cargo.toml
- [ ] Add missing dependencies:
  - [ ] humantime-serde = "1.1"
  - [ ] async-std (for SwarmBuilder executor)
```

### 2. Simplify Network for MVP (Priority: HIGH)
```rust
// Create network/src/simple_tcp.rs
- [ ] Direct TCP connection (skip complex P2P)
- [ ] Simple handshake protocol
- [ ] Move serialization with CBOR
- [ ] Basic game state sync

// Message types needed:
- [ ] Connect { game_id, player_info }
- [ ] Move { coord, color, move_number }
- [ ] GameState { full_state } // For reconnection
- [ ] Chat { message }
```

### 3. Wire Up Game Synchronization (Priority: HIGH)
```rust
// network/src/game_channel.rs
- [ ] Implement actual message sending
- [ ] Add move validation on receive
- [ ] Handle out-of-order moves
- [ ] Basic conflict resolution (move numbers)

// ui-egui/src/worker.rs
- [ ] Connect network messages to game state
- [ ] Update board on remote moves
- [ ] Send local moves to network
```

### 4. Neural Network Integration (Priority: MEDIUM)
```rust
// neural/src/wasm_loader.rs
- [ ] Load policy_0090.onnx
- [ ] Load value_0090.onnx
- [ ] Create inference wrapper

// ui-egui/src/board_widget.rs
- [ ] Connect heat map to neural predictions
- [ ] Update ghost stone display
- [ ] Add toggle for AI assistance
```

### 5. Create Game Lobby (Priority: MEDIUM)
```rust
// Simple lobby for MVP:
- [ ] List active games
- [ ] Create game with ID
- [ ] Join by game ID
- [ ] Direct IP connection option
```

### 6. Fix Build & Packaging (Priority: HIGH)
```rust
// Fix feature flags:
- [ ] Align network features between crates
- [ ] Remove iroh dependency for MVP
- [ ] Update build scripts

// Create release build:
- [ ] cargo build --release
- [ ] Bundle with neural models
- [ ] Create DMG installer
```

## Simplified Architecture for MVP

```
Player A                    Player B
   |                           |
   ├── UI (egui)              ├── UI (egui)
   ├── Game Logic             ├── Game Logic
   ├── Neural Nets            ├── Neural Nets
   └── TCP Client             └── TCP Client
        |                           |
        └───── Direct TCP ─────────┘
               Connection
```

## MVP Features (Minimum)
1. **Two players can connect** via IP/port or game ID
2. **Play a complete game** with proper rules
3. **See AI suggestions** via heat map
4. **Game ends properly** with scoring
5. **Can save/load games** via SGF

## Nice-to-Have (Post-MVP)
- Circuit Relay v2 for NAT traversal
- DHT-based discovery
- RNA training data sharing
- Auto-update mechanism
- Ranked matches

## Testing Checklist
- [ ] Start game on computer A
- [ ] Connect from computer B
- [ ] Play 10+ moves
- [ ] Verify moves sync correctly
- [ ] Test disconnection/reconnection
- [ ] Complete game with scoring
- [ ] Verify AI suggestions work

## Time Estimate
- Fix compilation: 2 hours
- Simple TCP network: 4 hours
- Wire up sync: 3 hours
- Neural integration: 2 hours
- Testing & polish: 3 hours
- **Total: ~14 hours**

## Next Steps
1. Fix network compilation errors
2. Implement SimpleTcpNetwork
3. Wire up to existing UI
4. Test local multiplayer
5. Package and distribute