# P2P Go Test Harness

This document provides an overview of the test harness implementation for P2P-Go, focusing on relay limits, ACK watchdog, VecDeque-based deduplication, snapshot cadence, and idle repaint logic.

## 1. Test Structure

### 1.1 Helper Modules
- `tests/common/mod.rs`: Provides helpers for spawning test peers and relays
- `tests/common/test_utils.rs`: Utilities for working with games, moves, and synchronization

### 1.2 Integration Tests
- `tests/duplicate_delivery.rs`: Tests VecDeque-based deduplication
- `tests/ack_timeout.rs`: Tests ACK watchdog timeout and recovery
- `tests/relay_limits.rs`: Tests connection and bandwidth limits
- `tests/snapshot_cadence.rs`: Tests snapshot timing and files
- `tests/property_reorder.rs`: Property testing for move reordering

### 1.3 Fuzzing Tests
- `fuzz/fuzz_targets/stack_desync.rs`: Fuzzing for move deserialization

## 2. Key Features Tested

### 2.1 VecDeque-based Deduplication
- Tests that multiple identical moves are processed only once
- Validates that the deduplication queue size stays within limits (8192)
- Checks deduplication queue growth under load

### 2.2 ACK Watchdog
- Tests that missing ACKs trigger SyncRequest within timeout period
- Validates that received ACKs properly reset the watchdog
- Uses tokio time control features to simulate delays

### 2.3 Relay Connection/Bandwidth Limits
- Tests connection limits by creating multiple peers
- Tests bandwidth throttling by creating move bursts
- Verifies that excess connections are rejected

### 2.4 Snapshot Cadence
- Tests that snapshots are created after move sequences
- Tests that snapshots are created after time-based intervals
- Validates snapshot file integrity

### 2.5 Property/Fuzz Testing
- Tests that move reordering produces consistent board positions
- Fuzzes move deserialization to ensure no panics

## 3. Usage

Run individual tests:
```
cargo test --test duplicate_delivery --features iroh -- --nocapture
cargo test --test ack_timeout --features iroh -- --nocapture
cargo test --test relay_limits --features iroh -- --nocapture
cargo test --test snapshot_cadence --features iroh -- --nocapture
cargo test --test property_reorder -- --nocapture
```

Run all tests:
```
cargo test --all --all-features
```

Run fuzzing tests (requires nightly):
```
cd network
cargo +nightly fuzz run stack_desync
```

## 4. CI Integration

The `.github/workflows/ci.yml` file has been updated to run all tests as part of the continuous integration pipeline:
- Runs each integration test individually
- Runs all tests with all features
- Gates merges on tests passing
