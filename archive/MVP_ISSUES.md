# P2P Go MVP Issues and Fixes Needed

## Current Status

The project has a working offline game but needs fixes to reach network-enabled MVP status. Here's what's missing:

## 1. Build Configuration Issues

### Problem: Feature flag mismatch
- `ui-egui/Cargo.toml` expects `iroh` feature in `p2pgo-network`
- Network crate has the feature defined but compilation fails due to API changes

### Fix Needed:
- Update network crate to use stub mode by default for MVP
- Fix libp2p API usage (version 0.53 has different APIs than expected)
- Add missing dependencies like `humantime_serde`

## 2. Missing Network Implementations

### Core Network Stack
- **IrohCtx**: Partially implemented, needs completion for basic P2P connectivity
- **GameChannel**: Module structure exists but missing network message handling
- **Lobby**: In-memory implementation exists but needs network discovery

### Key Missing Features:
1. **Peer Discovery**: No working implementation for finding other players
2. **NAT Traversal**: Circuit relay v2 started but not integrated
3. **Move Synchronization**: Protocol defined but not connected to UI
4. **Network Transport**: No working connection establishment

## 3. Integration Gaps

### UI ↔ Network
- Worker thread (`worker.rs`) has handlers but network calls are stubs
- Message types defined but not fully wired
- Ghost move suggestions implemented but gated behind game completion threshold

### Network ↔ Core
- Move validation exists in core but not integrated with network
- Score calculation works offline but network consensus missing

## 4. Compilation Errors

### libp2p API Issues:
```rust
// Line 105-108: gossipsub initialization error
// Line 130: relay client initialization missing
// Line 88: SwarmBuilder API changed
```

### Missing Dependencies:
- `humantime_serde` not in Cargo.toml
- Feature flags need cleanup

## 5. Neural Network Integration

### Current State:
- Models defined in `ml_models/` directory
- WASM builds exist but not loaded
- UI has ghost move display but network doesn't provide data

### Needed:
- Load WASM models in worker
- Connect model predictions to ghost move UI
- Implement move suggestion limits

## MVP Path Forward

### Phase 1: Fix Compilation (1-2 hours)
1. Add missing dependencies
2. Update libp2p API usage to match v0.53
3. Default to stub networking for initial testing

### Phase 2: Basic Networking (2-4 hours)
1. Implement simple TCP connection between two players
2. Add move serialization/deserialization
3. Create basic lobby with manual IP connection

### Phase 3: Game Synchronization (2-3 hours)
1. Connect GameChannel to network messages
2. Implement move broadcasting
3. Add basic conflict resolution

### Phase 4: Neural Network (1-2 hours)
1. Load WASM models in worker
2. Connect predictions to UI
3. Test ghost move display

### Phase 5: DMG Packaging (1 hour)
1. Fix build scripts for network-enabled version
2. Create signed DMG with all features
3. Test on clean macOS system

## Minimum Viable Product Requirements

For a working DMG that allows two players to connect and play:

1. **Network**: Direct IP connection (no discovery needed for MVP)
2. **Lobby**: Simple game ID system for joining
3. **Sync**: Basic move exchange with ordering
4. **Neural**: At least display placeholder suggestions
5. **UI**: All existing UI works with network games

## Estimated Time to MVP: 8-12 hours of focused development

The foundation is solid - the game logic, UI, and neural network infrastructure are all in place. The main work is connecting these pieces through a working network layer.