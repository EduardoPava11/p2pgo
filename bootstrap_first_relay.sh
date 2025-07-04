#!/bin/bash

# Bootstrap the first P2P Go relay node
# This script starts the bootstrap relay that other nodes can discover

set -e

echo "=== P2P Go Bootstrap Relay ==="
echo

# Default port
PORT=${1:-4001}

# Check if port is available
if lsof -i :$PORT > /dev/null 2>&1; then
    echo "Error: Port $PORT is already in use"
    echo "Please specify a different port: $0 <port>"
    exit 1
fi

echo "Starting bootstrap relay on port $PORT..."
echo

# Set environment variables
export RUST_LOG=info,p2pgo_network=debug,libp2p=info
export P2PGO_BOOTSTRAP=true

# Build the relay binary
echo "Building relay binary..."
cargo build --release --bin bootstrap-relay

# Create directories for relay data
RELAY_DIR="$HOME/.p2pgo/relay"
mkdir -p "$RELAY_DIR/data"
mkdir -p "$RELAY_DIR/logs"

# Start the bootstrap relay
echo
echo "Starting bootstrap relay..."
echo "This relay will:"
echo "  - Be discoverable via mDNS (local network)"
echo "  - Act as a circuit relay for NAT traversal"
echo "  - Maintain the DHT for peer discovery"
echo "  - Collect and distribute training data (RNA)"
echo

# Log file
LOG_FILE="$RELAY_DIR/logs/bootstrap_$(date +%Y%m%d_%H%M%S).log"

# Run the relay
cargo run --release --bin bootstrap-relay -- \
    --port $PORT \
    --bootstrap \
    --data-dir "$RELAY_DIR/data" \
    2>&1 | tee "$LOG_FILE" &

RELAY_PID=$!

echo "Bootstrap relay started with PID: $RELAY_PID"
echo "Log file: $LOG_FILE"
echo

# Wait for relay to start
sleep 3

# Show connection info
echo "Bootstrap relay information:"
echo "============================"

# Extract peer ID and addresses from log
if grep -q "peer_id" "$LOG_FILE"; then
    PEER_ID=$(grep "peer_id" "$LOG_FILE" | head -1 | grep -o 'peer_id: [^ ]*' | cut -d' ' -f2)
    echo "Peer ID: $PEER_ID"
fi

if grep -q "Listening on" "$LOG_FILE"; then
    echo
    echo "Listening addresses:"
    grep "Listening on" "$LOG_FILE" | sed 's/.*Listening on /  /'
fi

echo
echo "Other nodes can connect using:"
echo "  cargo run --bin bootstrap-relay -- --connect /ip4/127.0.0.1/tcp/$PORT/p2p/$PEER_ID"
echo
echo "To stop the relay: kill $RELAY_PID"
echo

# Create a helper script to connect to this relay
cat > connect_to_bootstrap.sh << EOF
#!/bin/bash
# Connect to the bootstrap relay
cargo run --release --bin bootstrap-relay -- \\
    --port \${1:-4002} \\
    --connect /ip4/127.0.0.1/tcp/$PORT/p2p/$PEER_ID
EOF

chmod +x connect_to_bootstrap.sh
echo "Created connect_to_bootstrap.sh for easy connection"

# Monitor the relay
echo
echo "Monitoring relay status..."
echo "Press Ctrl+C to stop"
echo

# Function to show relay stats
show_stats() {
    echo -e "\n[$(date '+%H:%M:%S')] Relay Statistics:"
    
    # Check connections
    CONNECTIONS=$(lsof -i :$PORT 2>/dev/null | grep ESTABLISHED | wc -l)
    echo "  Active connections: $CONNECTIONS"
    
    # Check log for RNA messages
    if [ -f "$LOG_FILE" ]; then
        RNA_COUNT=$(grep -c "RNA.*received" "$LOG_FILE" 2>/dev/null || echo "0")
        echo "  RNA messages: $RNA_COUNT"
        
        DISCOVERY_SCORE=$(grep "Discovery score" "$LOG_FILE" | tail -1 | grep -o '[0-9]\+\.[0-9]\+' || echo "1.0")
        echo "  Discovery score: $DISCOVERY_SCORE"
    fi
}

# Monitor loop
while kill -0 $RELAY_PID 2>/dev/null; do
    sleep 30
    show_stats
done

echo
echo "Relay stopped"