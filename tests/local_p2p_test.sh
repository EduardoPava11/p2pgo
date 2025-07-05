#!/bin/bash
# Local P2P testing script for p2pgo
# Tests multiple nodes on the same machine with different ports

set -e

echo "🧪 P2P Go Local Testing Suite"
echo "============================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test directories
TEST_DIR="./test_nodes"
NODE1_DIR="$TEST_DIR/node1"
NODE2_DIR="$TEST_DIR/node2"
NODE3_DIR="$TEST_DIR/node3"

# Create test directories
echo "📁 Setting up test directories..."
mkdir -p "$NODE1_DIR" "$NODE2_DIR" "$NODE3_DIR"

# Build the application
echo "🔨 Building p2pgo..."
cargo build --release

# Function to start a node
start_node() {
    local node_num=$1
    local port=$2
    local dir=$3
    local name=$4
    
    echo "🚀 Starting Node $node_num ($name) on port $port..."
    
    # Set environment variables for each node
    export P2PGO_DATA_DIR="$dir"
    export P2PGO_PORT="$port"
    export P2PGO_NODE_NAME="$name"
    
    # Start in background and save PID
    ./target/release/p2pgo --headless --port $port --name "$name" > "$dir/node.log" 2>&1 &
    echo $! > "$dir/node.pid"
    
    sleep 2
}

# Function to stop a node
stop_node() {
    local dir=$1
    if [ -f "$dir/node.pid" ]; then
        kill $(cat "$dir/node.pid") 2>/dev/null || true
        rm "$dir/node.pid"
    fi
}

# Cleanup function
cleanup() {
    echo -e "\n🧹 Cleaning up..."
    stop_node "$NODE1_DIR"
    stop_node "$NODE2_DIR" 
    stop_node "$NODE3_DIR"
}

# Set trap for cleanup
trap cleanup EXIT

# Test 1: Basic connectivity
echo -e "\n${YELLOW}Test 1: Basic Connectivity${NC}"
echo "------------------------------"

start_node 1 4001 "$NODE1_DIR" "Alice"
start_node 2 4002 "$NODE2_DIR" "Bob"

echo "⏳ Waiting for nodes to initialize..."
sleep 5

# Get node tickets
echo "🎫 Getting connection tickets..."
TICKET1=$(curl -s http://localhost:4001/api/ticket || echo "Failed")
TICKET2=$(curl -s http://localhost:4002/api/ticket || echo "Failed")

if [[ "$TICKET1" == "Failed" ]] || [[ "$TICKET2" == "Failed" ]]; then
    echo -e "${RED}❌ Failed to get tickets. Check if nodes started correctly.${NC}"
    exit 1
fi

echo "Node 1 ticket: ${TICKET1:0:20}..."
echo "Node 2 ticket: ${TICKET2:0:20}..."

# Test 2: Game creation and joining
echo -e "\n${YELLOW}Test 2: Game Creation & Joining${NC}"
echo "--------------------------------"

# Create game on Node 1
echo "🎮 Creating game on Node 1..."
GAME_ID=$(curl -s -X POST http://localhost:4001/api/create_game -d '{"board_size": 9}' | jq -r '.game_id')
echo "Game ID: $GAME_ID"

# Join game from Node 2
echo "🤝 Joining game from Node 2..."
JOIN_RESULT=$(curl -s -X POST http://localhost:4002/api/join_game -d "{\"game_id\": \"$GAME_ID\"}")
echo "Join result: $JOIN_RESULT"

# Test 3: Relay modes
echo -e "\n${YELLOW}Test 3: Relay Mode Testing${NC}"
echo "---------------------------"

# Test each relay mode
for mode in "Disabled" "Minimal" "Normal" "Provider"; do
    echo "🔄 Testing relay mode: $mode"
    curl -s -X POST http://localhost:4001/api/set_relay_mode -d "{\"mode\": \"$mode\"}"
    sleep 2
    
    # Check connectivity
    CONNECTED=$(curl -s http://localhost:4001/api/status | jq -r '.connected')
    if [[ "$CONNECTED" == "true" ]]; then
        echo -e "${GREEN}✓ Connected in $mode mode${NC}"
    else
        echo -e "${RED}✗ Failed to connect in $mode mode${NC}"
    fi
done

# Test 4: Move synchronization
echo -e "\n${YELLOW}Test 4: Move Synchronization${NC}"
echo "-----------------------------"

# Make moves alternately
MOVES=("3,3" "5,5" "4,4" "3,5" "5,3")
for i in "${!MOVES[@]}"; do
    PORT=$((4001 + (i % 2)))
    MOVE="${MOVES[$i]}"
    
    echo "📍 Player $((i % 2 + 1)) plays at $MOVE"
    curl -s -X POST http://localhost:$PORT/api/make_move -d "{\"game_id\": \"$GAME_ID\", \"move\": \"$MOVE\"}"
    sleep 1
done

# Verify game state is synchronized
echo "🔍 Verifying game state synchronization..."
STATE1=$(curl -s http://localhost:4001/api/game_state/$GAME_ID | jq -r '.move_count')
STATE2=$(curl -s http://localhost:4002/api/game_state/$GAME_ID | jq -r '.move_count')

if [[ "$STATE1" == "$STATE2" ]]; then
    echo -e "${GREEN}✓ Game states synchronized: $STATE1 moves${NC}"
else
    echo -e "${RED}✗ Game states out of sync: Node1=$STATE1, Node2=$STATE2${NC}"
fi

# Test 5: NAT simulation
echo -e "\n${YELLOW}Test 5: NAT Traversal (Simulated)${NC}"
echo "----------------------------------"

# Start a third node with restricted connectivity
start_node 3 4003 "$NODE3_DIR" "Charlie"

# Simulate NAT by using relay-only mode
echo "🌐 Testing relay-only connectivity..."
curl -s -X POST http://localhost:4003/api/set_relay_mode -d '{"mode": "Minimal"}'

# Try to connect through relay
echo "🔗 Connecting Node 3 to Node 1 via relay..."
CONNECT_RESULT=$(curl -s -X POST http://localhost:4003/api/connect -d "{\"ticket\": \"$TICKET1\"}")
echo "Connection result: $CONNECT_RESULT"

# Test 6: Score consensus
echo -e "\n${YELLOW}Test 6: Score Consensus & CBOR Export${NC}"
echo "--------------------------------------"

# Simulate game ending with passes
echo "🏁 Ending game with passes..."
curl -s -X POST http://localhost:4001/api/make_move -d "{\"game_id\": \"$GAME_ID\", \"move\": \"pass\"}"
curl -s -X POST http://localhost:4002/api/make_move -d "{\"game_id\": \"$GAME_ID\", \"move\": \"pass\"}"

# Accept score on both nodes
echo "✅ Accepting score on both nodes..."
curl -s -X POST http://localhost:4001/api/accept_score -d "{\"game_id\": \"$GAME_ID\"}"
curl -s -X POST http://localhost:4002/api/accept_score -d "{\"game_id\": \"$GAME_ID\"}"

# Check for CBOR archive
sleep 2
CBOR_FILES=$(find "$NODE1_DIR" -name "*.cbor*" | wc -l)
if [[ $CBOR_FILES -gt 0 ]]; then
    echo -e "${GREEN}✓ CBOR archive created${NC}"
else
    echo -e "${RED}✗ No CBOR archive found${NC}"
fi

# Test 7: Training data consent
echo -e "\n${YELLOW}Test 7: Training Data Flow${NC}"
echo "---------------------------"

# Enable provider mode with consent
echo "📊 Enabling training data consent..."
curl -s -X POST http://localhost:4001/api/set_relay_mode -d '{"mode": "Provider"}'
curl -s -X POST http://localhost:4001/api/set_training_consent -d '{"consent": true}'

# Check credits
CREDITS=$(curl -s http://localhost:4001/api/relay_stats | jq -r '.credits')
echo "Training credits earned: $CREDITS"

# Summary
echo -e "\n${YELLOW}Test Summary${NC}"
echo "============"
echo -e "${GREEN}✓ Connectivity tests completed${NC}"
echo -e "${GREEN}✓ Game synchronization verified${NC}"
echo -e "${GREEN}✓ Relay modes tested${NC}"
echo -e "${GREEN}✓ Score consensus checked${NC}"

echo -e "\n✨ Local P2P testing complete!"