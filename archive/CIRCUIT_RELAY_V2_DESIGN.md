# Circuit Relay v2 Design for 3-Player P2P Go

## Overview

Circuit Relay v2 is a custom relay protocol designed for the 3-player P2P Go game. Unlike traditional 2-party protocols, this system handles three simultaneous players with Byzantine fault tolerance.

## Core Design Principles

### 1. **Triangular Topology**
- Three players form a triangle of connections
- Each player can relay for the other two
- Resilient to single player disconnection

### 2. **Binary Fuel Credits**
- 1 credit = 1 relay hop OR 1 new connection
- Credits earned by relaying for others
- Simple, predictable economics

### 3. **Guild-Based Routing**
- Activity guild: Aggressive direct connections
- Reactivity guild: Defensive relay usage
- Avoidance guild: Balanced approach

## Protocol Architecture

### Message Types
```rust
enum RelayMessage {
    // Connection establishment
    Connect { from: PlayerId, to: PlayerId, via: Option<PlayerId> },
    
    // Game moves (signed by sender)
    GameMove { 
        move: Move3D,
        signature: Signature,
        sequence: u64,
    },
    
    // Relay requests
    RelayRequest {
        target: PlayerId,
        credits_offered: u64,
    },
    
    // State synchronization
    StateSync {
        board_hash: Hash,
        move_count: u64,
        players: [PlayerId; 3],
    },
}
```

### Connection States
```rust
enum ConnectionState {
    Direct,                    // Direct P2P connection
    Relayed { via: PlayerId }, // Through relay
    Searching,                 // Looking for path
    Disconnected,             // No path available
}
```

## 3-Player Specific Features

### 1. **Consensus Protocol**
- 2-of-3 agreement for move validation
- Automatic arbiter selection
- Fork resolution through majority

### 2. **Relay Incentives**
```rust
struct RelayIncentive {
    // Credits earned per relayed message
    base_rate: u64,
    
    // Bonus for maintaining stability
    stability_bonus: u64,
    
    // Penalty for dropping messages
    drop_penalty: u64,
}
```

### 3. **Path Discovery**
- Gossip-based announcement
- DHT for player discovery
- Fallback to known bootstrap nodes

## Implementation Plan

### Phase 1: Local Testing
```rust
// Test with three instances on localhost
fn test_local_relay() {
    let player1 = RelayNode::new("127.0.0.1:9001");
    let player2 = RelayNode::new("127.0.0.1:9002");
    let player3 = RelayNode::new("127.0.0.1:9003");
    
    // Form triangle
    player1.connect(player2);
    player2.connect(player3);
    player3.connect(player1);
}
```

### Phase 2: Network Simulation
- Use mininet or network namespaces
- Simulate latency and packet loss
- Test relay failover

### Phase 3: Internet Deployment
- Public bootstrap nodes
- NAT traversal (STUN/TURN fallback)
- Geographic relay selection

## Integration with Go AI

### 1. **Move Validation**
Use neural nets to validate moves:
```rust
fn validate_move_3d(move: &Move3D, board: &Board3D) -> bool {
    // Quick rule check
    if !move.is_legal() { return false; }
    
    // Neural net sanity check
    let confidence = neural_net.evaluate_move(move, board);
    confidence > 0.1 // Reject obviously bad moves
}
```

### 2. **Relay Selection**
Use Go AI for intelligent relay choice:
```rust
fn select_best_relay(candidates: Vec<RelayNode>) -> RelayNode {
    // Convert to "board position"
    let network_state = network_to_board(&candidates);
    
    // Use Go AI to evaluate
    let scores = kata_go.evaluate_positions(&network_state);
    
    // Select highest scoring relay
    candidates[scores.argmax()]
}
```

### 3. **Credit Optimization**
Balance credits like territory:
```rust
fn optimize_credit_usage(credits: u64, needed_hops: Vec<Hop>) -> Vec<Hop> {
    // Treat as resource allocation problem
    // Similar to securing territory in Go
    kata_go.allocate_resources(credits, needed_hops)
}
```

## Security Considerations

### 1. **Byzantine Players**
- Invalid move flooding
- Relay credit manipulation  
- State fork attacks

### 2. **Mitigations**
- Rate limiting per player
- Cryptographic move signing
- Consensus validation

### 3. **Privacy**
- Onion routing for moves
- Encrypted relay channels
- Minimal metadata

## Performance Targets

- **Latency**: <100ms for relayed moves
- **Throughput**: 100 moves/second
- **Reliability**: 99.9% message delivery
- **Scalability**: 1000+ concurrent games

## Testing Strategy

### Local Machine Testing
```bash
# Terminal 1: Bootstrap
./p2pgo --mode=relay --port=9000

# Terminal 2: Player 1
./p2pgo --mode=3d --connect=localhost:9000 --port=9001

# Terminal 3: Player 2  
./p2pgo --mode=3d --connect=localhost:9000 --port=9002

# Terminal 4: Player 3
./p2pgo --mode=3d --connect=localhost:9000 --port=9003
```

### Docker Compose
```yaml
version: '3'
services:
  bootstrap:
    image: p2pgo:latest
    command: --mode=relay
    ports: ["9000:9000"]
    
  player1:
    image: p2pgo:latest
    command: --mode=3d --connect=bootstrap:9000
    depends_on: [bootstrap]
    
  player2:
    image: p2pgo:latest
    command: --mode=3d --connect=bootstrap:9000
    depends_on: [bootstrap]
    
  player3:
    image: p2pgo:latest
    command: --mode=3d --connect=bootstrap:9000
    depends_on: [bootstrap]
```

## Next Steps

1. Implement basic relay message types
2. Add credit tracking system
3. Integrate with 3D game logic
4. Test triangular topology
5. Add neural net optimization

The relay system will enable true decentralized 3-player Go while demonstrating economic incentives for network participation.