# Memory Usage Analysis (MEMCHECK.md)

## Overview

This document provides an analysis of memory usage patterns in the P2P Go application, identifying the largest heap types and potential optimizations.

## Largest Heap Types

| Type | Estimated Size | Notes |
|------|---------------|-------|
| `Board (Vec<Option<Color>>)` | ~3.2 KiB | 19×19 board = 361 cells × 8 bytes/Option + Vec overhead |
| `GameState` | ~3.5 KiB | Contains Board + metadata (current_player, move_count, etc.) |
| `MoveChain` | Variable | Grows with game length; each MoveBlob ~4-5 KiB |
| `HashMap<GameId, GameInfo>` | Variable | Scales with number of active games |
| `broadcast::Receiver` buffers | ~800 bytes | 100-item buffer × 8 bytes/pointer |

## Memory Usage Patterns

### Game State Storage
- **Board representation**: `Vec<Option<Color>>` for 19×19 = 361 × 8 bytes = ~2.9 KiB
- **Vec overhead**: ~24 bytes (ptr, len, capacity)
- **Total per board**: ~3.2 KiB - this is acceptable for a Go board

### Move History
- **MoveBlob**: Contains Move + GameState + metadata
- **Growth**: Linear with game length (typically 200-400 moves)
- **Peak usage**: ~2 MB for a complete game with full history

### Network Buffers
- **Broadcast channels**: 100-item buffer per channel
- **GameChannel events**: Temporary allocations, cleaned up quickly
- **Tokio runtime**: Background task overhead ~64 KiB

## Debug vs Production

### Development Mode
- **JSON serialization**: `serde_json::to_value()` in validation creates temporary allocations
- **State comparison**: Creates duplicate states for validation
- **Impact**: ~2x memory usage during move validation

### Production Mode
- Validation disabled with `cfg!(debug_assertions)` guards
- Only necessary state copies retained
- Estimated 50% memory reduction vs debug builds

## Optimization Notes

### Completed Optimizations
1. **RwLock over Mutex**: Reduced lock contention in read-heavy scenarios
2. **Debug-only validation**: Expensive clones only in debug builds
3. **Move reuse**: Eliminated duplicate Move clones in hot paths

### Future Considerations
1. **Board representation**: Could use `Vec<u8>` with packed encoding (2 bits per cell)
2. **Move compression**: Store moves as coordinates rather than full enum
3. **State snapshots**: Only store periodic snapshots + deltas for long games
4. **String interning**: GameId strings could be interned for deduplication

## Memory Safety

- All allocations are managed by Rust's ownership system
- No unsafe code in core game logic
- RAII ensures proper cleanup on game/lobby destruction
- Thread-safe shared state uses Arc<RwLock<T>> patterns

## Recommendations

1. **Current usage is acceptable** for typical Go games
2. **Monitor MoveChain growth** in very long games (>500 moves)
3. **Consider periodic state snapshots** if memory becomes constrained
4. **Profile with real workloads** to identify actual bottlenecks

The current memory footprint is well within reasonable bounds for a P2P Go application, with room for optimization if needed in the future.

# Memory Usage Analysis (MEMCHECK.md)

## Overview

This document provides an analysis of memory usage patterns in the P2P Go application, identifying the largest heap types and potential optimizations.

## Largest Heap Types

| Type | Estimated Size | Notes |
|------|---------------|-------|
| `Board (Vec<Option<Color>>)` | ~3.2 KiB | 19×19 board = 361 cells × 8 bytes/Option + Vec overhead |
| `GameState` | ~3.5 KiB | Contains Board + metadata (current_player, move_count, etc.) |
| `MoveChain` | Variable | Grows with game length; each MoveBlob ~4-5 KiB |
| `HashMap<GameId, GameInfo>` | Variable | Scales with number of active games |
| `broadcast::Receiver` buffers | ~800 bytes | 100-item buffer × 8 bytes/pointer |

## Memory Usage Patterns

### Game State Storage
- **Board representation**: `Vec<Option<Color>>` for 19×19 = 361 × 8 bytes = ~2.9 KiB
- **Vec overhead**: ~24 bytes (ptr, len, capacity)
- **Total per board**: ~3.2 KiB – acceptable for a Go board

### Move History
- **MoveBlob**: Contains Move + GameState + metadata
- **Growth**: Linear with game length (typically 200‑400 moves)
- **Peak usage**: ~2 MB for a complete game with full history

### Network Buffers
- **Broadcast channels**: 100‑item buffer per channel
- **GameChannel events**: Temporary allocations, cleaned up quickly
- **Tokio runtime**: Background task overhead ~64 KiB

## Debug vs Production

### Development Mode
- **JSON serialization**: `serde_json::to_value()` in validation creates temporary allocations
- **State comparison**: Creates duplicate states for validation
- **Impact**: ~2× memory usage during move validation

### Production Mode
- Validation disabled with `cfg!(debug_assertions)` guards
- Only necessary state copies retained
- Estimated 50 % memory reduction vs debug builds

## Optimization Notes

### Completed Optimizations
1. **RwLock over Mutex** – reduced lock contention in read‑heavy scenarios
2. **Debug‑only validation** – expensive clones only in debug builds
3. **Move reuse** – eliminated duplicate `Move` clones in hot paths

### Future Considerations
1. **Board representation** – pack into `Vec<u8>` (2 bits per cell)
2. **Move compression** – store moves as coordinates rather than enum
3. **State snapshots** – periodic snapshots + deltas for long games
4. **String interning** – deduplicate `GameId` strings

## Memory Safety

- All allocations managed by Rust’s ownership system
- **No `unsafe` code** in core game logic
- RAII ensures cleanup on game/lobby destruction
- Shared state uses `Arc<RwLock<T>>` for thread‑safety

## Recommendations

1. Current usage is acceptable for typical games
2. Monitor `MoveChain` growth in very long games (> 500 moves)
3. Consider snapshots if memory becomes constrained
4. Profile real workloads to identify bottlenecks

The current memory footprint is well within reasonable bounds for a P2P Go application, with room for future optimizations.