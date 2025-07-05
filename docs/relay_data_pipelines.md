# Relay Service Data Pipelines & Memory Safety Analysis

## Overview
This document maps the data flow through the P2P relay service to identify potential memory issues and ensure safe operation.

## 1. Core Data Structures & Memory Ownership

### Game Channel Pipeline
```
GameChannel (Arc<RwLock>)
├── move_chain: Arc<RwLock<MoveChain>>
│   └── moves: Vec<MoveRecord> [MEMORY RISK: Unbounded growth]
├── events_tx: broadcast::Sender<GameEvent>
│   └── buffer_size: 256 [BOUNDED]
├── latest_state: Arc<RwLock<Option<GameState>>>
│   └── board: Vec<Option<Color>> [FIXED SIZE: board_size²]
└── peer_connections: Arc<RwLock<HashMap<PeerId, bool>>>
    └── [MEMORY RISK: Unbounded peer connections]
```

### P2P Node Pipeline
```
P2PNode
├── swarm: Swarm<P2PBehaviour>
│   └── [LIBP2P MANAGES MEMORY]
├── connected_peers: Arc<RwLock<HashSet<PeerId>>>
│   └── [MEMORY RISK: Unbounded peer set]
├── known_relays: Arc<RwLock<HashMap<PeerId, RelayInfo>>>
│   └── [MEMORY RISK: Unbounded relay discovery]
└── relay_reservations: Arc<RwLock<HashMap<PeerId, RelayReservation>>>
    └── [BOUNDED BY: max_reservations config]
```

## 2. Message Flow & Buffering

### Inbound Message Pipeline
```
Network Layer → Message Buffer → Game Channel → Game Logic
     ↓              ↓                ↓              ↓
libp2p recv    MPSC channel    broadcast::tx   GameState
              (unbounded!)      (bounded:256)   update
```

**MEMORY RISKS:**
- Unbounded MPSC channels between network and game logic
- No backpressure mechanism for slow consumers
- Message deserialization happens in memory

### Outbound Message Pipeline
```
Game Logic → Serialization → Network Buffer → Network Layer
     ↓            ↓               ↓               ↓
Move/Event   CBOR/JSON      MPSC channel    libp2p send
                           (unbounded!)
```

**MEMORY RISKS:**
- Serialization creates temporary copies
- Unbounded outbound queues
- No rate limiting on sends

## 3. Relay-Specific Memory Concerns

### Circuit Relay V2 Buffers
```
Relay Node
├── Active Circuits: HashMap<CircuitId, Circuit>
│   ├── inbound_buffer: Vec<u8> [RISK: No size limit]
│   ├── outbound_buffer: Vec<u8> [RISK: No size limit]
│   └── bandwidth_used: u64
└── Reservations: HashMap<PeerId, Reservation>
    └── expires: Instant
```

### Relay Credit System
```
CreditTracker
├── peer_credits: HashMap<PeerId, Credits>
│   └── [MEMORY RISK: Grows with peer count]
└── training_data_earned: HashMap<GameId, DataInfo>
    └── [MEMORY RISK: Grows with game count]
```

## 4. Memory Safety Measures Needed

### A. Bounded Collections
```rust
// Add bounds to all growing collections
const MAX_PEERS_PER_GAME: usize = 10;
const MAX_MOVES_PER_GAME: usize = 500;
const MAX_KNOWN_RELAYS: usize = 100;
const MAX_RELAY_BUFFER: usize = 1_000_000; // 1MB per circuit
```

### B. Message Rate Limiting
```rust
// Add rate limiting to prevent DoS
struct RateLimiter {
    messages_per_second: u32,
    bytes_per_second: u64,
    last_reset: Instant,
}
```

### C. Garbage Collection Tasks
```rust
// Periodic cleanup of stale data
async fn cleanup_task() {
    // Remove expired relay reservations
    // Clear old game channels
    // Prune peer connection history
    // Archive completed games
}
```

### D. Backpressure Implementation
```rust
// Use bounded channels everywhere
let (tx, rx) = mpsc::channel::<Message>(1000);

// Implement slow consumer detection
if tx.is_full() {
    warn!("Channel full, applying backpressure");
    // Drop non-critical messages or slow down
}
```

## 5. Resource Limits Configuration

### Per-Game Limits
- Max players: 4
- Max moves: 500
- Max game duration: 4 hours
- Max message size: 64KB

### Per-Peer Limits
- Max concurrent games: 10
- Max relay circuits: 5
- Max bandwidth: 1MB/s
- Connection timeout: 5 minutes

### Global Limits
- Max total games: 1000
- Max total peers: 10000
- Max memory usage: 500MB
- Max relay bandwidth: 10MB/s

## 6. Monitoring & Metrics

### Memory Metrics to Track
```rust
struct MemoryMetrics {
    total_allocated: usize,
    game_channels: usize,
    peer_connections: usize,
    relay_buffers: usize,
    message_queues: usize,
}
```

### Alert Thresholds
- Warn at 80% memory usage
- Start dropping connections at 90%
- Emergency shutdown at 95%

## 7. Implementation Priority

1. **Immediate** - Fix unbounded channels
2. **High** - Add collection size limits
3. **High** - Implement cleanup tasks
4. **Medium** - Add rate limiting
5. **Medium** - Implement backpressure
6. **Low** - Add comprehensive metrics

## 8. Testing Strategy

### Memory Leak Tests
- Long-running relay under load
- Many games starting/stopping
- Peer churn simulation
- Network partition scenarios

### Load Tests
- 100 concurrent games
- 1000 peer connections
- 10MB/s relay traffic
- Message burst scenarios