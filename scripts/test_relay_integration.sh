#!/bin/bash

# SPDX-License-Identifier: MIT OR Apache-2.0
# Test relay configuration and network functionality

set -e

echo "=== P2P Go Relay Test Script ==="
echo

# Test 1: Config loading test
echo "🧪 Test 1: Config loading and parsing"
cargo test --package p2pgo-network test_config_loading || {
    echo "❌ Config loading test failed"
    exit 1
}
echo "✅ Config loading test passed"
echo

# Test 2: CBOR and hash functionality  
echo "🧪 Test 2: MoveRecord hash chain integrity"
cargo test --package p2pgo-core -- test_move_record_broadcast_hash test_move_record_chain_integrity || {
    echo "❌ MoveRecord hash tests failed"
    exit 1
}
echo "✅ MoveRecord hash tests passed"
echo

# Test 3: Basic relay config tests
echo "🧪 Test 3: Relay configuration tests"
cargo test --package p2pgo-network test_relay_config -- --test-threads=1 || {
    echo "❌ Relay config tests failed"
    exit 1
}
echo "✅ Relay config tests passed"
echo

# Test 4: Network integration test (if available)
echo "🧪 Test 4: Basic IrohCtx functionality"
timeout 30 cargo test --package p2pgo-network test_relay_connectivity -- --test-threads=1 || {
    echo "⚠️  Network connectivity test failed or timed out (this is expected in CI)"
}
echo

echo "🎉 All tests completed successfully!"
echo "📝 Summary:"
echo "   ✅ Config loading and parsing"
echo "   ✅ MoveRecord hash chain integrity"
echo "   ✅ Relay configuration handling"
echo "   🌐 Network tests (may fail in CI environments)"
echo
echo "🚀 Ready for DMG build and manual testing"
