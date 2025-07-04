# P2P Go MVP Architecture

## Overview

This MVP implements a decentralized Go game with neural network assistance and federated learning, focusing on:
1. Direct peer connections using libp2p Circuit Relay v2
2. Dual neural networks (Policy + Value) like AlphaGo
3. RNA-based training data sharing
4. Game lobby for discovery
5. Auto-update mechanism

## Key Components

### 1. Network Layer (`network/`)
- **libp2p Circuit Relay v2**: NAT traversal with automatic firewall handling
- **mDNS**: Local network discovery
- **Gossipsub**: RNA (training data) propagation
- **Kademlia DHT**: Peer discovery (future: Elo ratings)

### 2. Neural Networks (`neural/`)
- **Policy Network**: Predicts move probabilities (like AlphaGo)
- **Value Network**: Evaluates board positions
- **Federated Training**: Local training with weight sharing
- **Heat Map Visualization**: Shows AI suggestions in UI

### 3. UI Components (`ui-egui/`)
- **SGF Upload Tool**: Convert game records to training data (mRNA)
- **Heat Map Overlay**: Visualize neural network predictions
- **Game Lobby**: Discover and join games
- **Bootstrap Status**: Network connection progress

### 4. RNA System
- **mRNA (Game Data)**: Full game records for training
- **SGF RNA**: Uploaded game segments as training data
- **Gossipsub Topics**: 
  - `p2pgo/games/v1`: Active games
  - `p2pgo/rna/v1`: Training data
  - `p2pgo/lobby/v1`: Game discovery

## Quick Start

### 1. Build the Relay
```bash
cd network
cargo build --release --bin p2pgo-relay
```

### 2. Start First Relay
```bash
./target/release/p2pgo-relay
# Note the peer ID and listening addresses
```

### 3. Connect Second Relay
```bash
# Use the address from first relay
./target/release/p2pgo-relay --connect /ip4/192.168.1.100/tcp/4001/p2p/12D3KooW...
```

### 4. Upload Training Data
```bash
# Upload SGF file as training data
./target/release/p2pgo-relay --sgf game.sgf --range 0-50
```

## Connection Flow

1. **Bootstrap**: 
   - Try mDNS for local peers
   - Setup Circuit Relay if behind NAT
   - Subscribe to gossipsub topics

2. **Direct Connection**:
   - Exchange multiaddrs through lobby
   - Attempt direct dial
   - Fallback to relay circuit

3. **RNA Sharing**:
   - Create mRNA from completed games
   - Broadcast via gossipsub
   - Receivers evaluate quality

## Neural Network Training

1. **Local Training**:
   - Collect consensus games
   - Train policy and value networks
   - Export model weights

2. **Federated Learning**:
   - Share model updates as RNA
   - Merge weights from multiple relays
   - No raw data leaves relay

## Future Enhancements

1. **TrueSkill Ratings**: Store in DHT for matchmaking
2. **Pattern Tool (tRNA)**: Share specific positions
3. **Style Transfer (lncRNA)**: WASM-based style mixing
4. **Gene Marketplace**: Trade trained models

## DMG Distribution

The DMG will include:
- P2P Go application with integrated relay
- Auto-update mechanism
- Neural network models
- Bootstrap node list

Users can:
1. Install DMG
2. Launch app (starts relay automatically)
3. Share multiaddr with friends
4. Play games and train neural nets together