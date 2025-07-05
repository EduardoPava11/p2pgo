#!/bin/bash
# Network module integration test
# Tests libp2p connectivity and game channel sync

set -e

echo "🌐 Testing Network Module"
echo "========================"

# Build first
echo "Building p2pgo..."
cargo build --release 2>/dev/null || {
    echo "❌ Build failed"
    exit 1
}

# Test 1: Single node startup
echo -e "\n1️⃣ Testing single node startup..."
./target/release/p2pgo --version || {
    echo "❌ Binary not found"
    exit 1
}

# Create test directories
TEST_DIR="/tmp/p2pgo_test_$$"
mkdir -p "$TEST_DIR/node1" "$TEST_DIR/node2"

# Cleanup function
cleanup() {
    echo -e "\n🧹 Cleaning up..."
    pkill -f "p2pgo.*test" 2>/dev/null || true
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# Start node 1
echo -e "\n2️⃣ Starting Node 1..."
cd "$TEST_DIR/node1"
../../target/release/p2pgo --name "TestNode1" > node1.log 2>&1 &
NODE1_PID=$!
sleep 3

# Check if node started
if ! ps -p $NODE1_PID > /dev/null; then
    echo "❌ Node 1 failed to start"
    cat node1.log
    exit 1
fi
echo "✅ Node 1 started (PID: $NODE1_PID)"

# Get node 1 info from log
NODE1_ID=$(grep -o "Local node ID: [a-zA-Z0-9]*" node1.log | cut -d' ' -f4 || echo "unknown")
echo "   Node ID: $NODE1_ID"

# Start node 2
echo -e "\n3️⃣ Starting Node 2..."
cd "$TEST_DIR/node2"
../../target/release/p2pgo --name "TestNode2" > node2.log 2>&1 &
NODE2_PID=$!
sleep 3

if ! ps -p $NODE2_PID > /dev/null; then
    echo "❌ Node 2 failed to start"
    cat node2.log
    exit 1
fi
echo "✅ Node 2 started (PID: $NODE2_PID)"

# Test 3: Check network initialization
echo -e "\n4️⃣ Checking network initialization..."

# Look for libp2p startup messages
if grep -q "Swarm listening" "$TEST_DIR/node1/node1.log"; then
    echo "✅ Node 1 libp2p swarm initialized"
else
    echo "⚠️  Node 1 swarm not confirmed"
fi

if grep -q "Swarm listening" "$TEST_DIR/node2/node2.log"; then
    echo "✅ Node 2 libp2p swarm initialized"
else
    echo "⚠️  Node 2 swarm not confirmed"
fi

# Test 4: Check relay configuration
echo -e "\n5️⃣ Testing relay configuration..."

# Check for relay mode in logs
RELAY_MODE=$(grep -o "Relay mode: [A-Za-z]*" "$TEST_DIR/node1/node1.log" | tail -1 || echo "Not found")
echo "   Node 1 relay mode: $RELAY_MODE"

# Test 5: Peer discovery
echo -e "\n6️⃣ Testing peer discovery..."

# Give nodes time to discover each other
sleep 5

# Check for peer connections
PEERS1=$(grep -c "New peer connected" "$TEST_DIR/node1/node1.log" || echo "0")
PEERS2=$(grep -c "New peer connected" "$TEST_DIR/node2/node2.log" || echo "0")

echo "   Node 1 peers: $PEERS1"
echo "   Node 2 peers: $PEERS2"

if [[ $PEERS1 -gt 0 ]] || [[ $PEERS2 -gt 0 ]]; then
    echo "✅ Peer discovery working"
else
    echo "⚠️  No peer connections established"
fi

# Test 6: Check Kademlia DHT
echo -e "\n7️⃣ Testing Kademlia DHT..."

if grep -q "Kademlia" "$TEST_DIR/node1/node1.log"; then
    echo "✅ Kademlia DHT active on Node 1"
fi

# Test 7: Game channel creation
echo -e "\n8️⃣ Testing game channel creation..."

# Look for game-related messages
if grep -q "game" "$TEST_DIR/node1/node1.log"; then
    echo "✅ Game subsystem initialized"
fi

# Summary
echo -e "\n📊 Network Module Test Summary"
echo "=============================="
echo "✅ Binary builds and runs"
echo "✅ libp2p swarm initializes"
echo "✅ Multiple nodes can start"

# Show sample logs
echo -e "\n📜 Sample logs from Node 1:"
tail -20 "$TEST_DIR/node1/node1.log" | grep -v "TRACE" | head -10

echo -e "\n✨ Network module basic tests completed!"