#!/bin/bash
# Demo script for CLI e2e multiplayer Go game
set -e

echo "=== P2P Go CLI E2E Demo ==="
echo "Board: 9x9, Move sequence: B D4, W F4, B E5, W pass, B pass"
cd "$(dirname "$0")/.."
echo "Building..." && cargo build --release --bin p2pgo-cli
echo "Testing..." && cargo test --package p2pgo-cli test_e2e_multiplayer_game
echo "✓ Integration test passed!"
echo "Manual demo: ./target/release/p2pgo-cli create --board-size 9"
echo "✓ Demo completed!"
