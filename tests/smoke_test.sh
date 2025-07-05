#!/bin/bash
# Quick smoke test - runs in 2 minutes
# Use before commits and releases

echo "🚬 P2P Go Smoke Test"
echo "==================="

# Quick build check
echo "1. Build check..."
cargo check --all-features || exit 1

# Unit tests (fast ones only)
echo "2. Unit tests..."
cargo test --lib -- --test-threads=4 || exit 1

# Start single node
echo "3. Single node startup..."
timeout 10s cargo run -- --headless &
PID=$!
sleep 5
kill $PID 2>/dev/null

# Check binary size
echo "4. Binary size check..."
cargo build --release
SIZE=$(ls -lh target/release/p2pgo | awk '{print $5}')
echo "Binary size: $SIZE"

# Lint check
echo "5. Clippy check..."
cargo clippy -- -D warnings || exit 1

echo "✅ Smoke test passed!"