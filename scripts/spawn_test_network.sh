#!/bin/bash
# Spawn a test network for P2P Go experiments

set -e

# Default values
RELAYS=${1:-3}
PLAYERS=${2:-6}
BASE_PORT=${3:-8000}
LOG_DIR="logs/$(date +%Y%m%d_%H%M%S)"

echo "=== P2P Go Test Network ==="
echo "Relays: $RELAYS"
echo "Players: $PLAYERS"
echo "Base Port: $BASE_PORT"
echo "Log Dir: $LOG_DIR"
echo "=========================="

# Create log directory
mkdir -p $LOG_DIR

# Function to start a relay
start_relay() {
    local id=$1
    local port=$((BASE_PORT + id))
    local config="$LOG_DIR/relay_$id.toml"
    
    # Create relay config
    cat > $config << EOF
relay_mode = "SelfRelay"
relay_port = $port

[training]
collect_data = true
consensus_required = true
min_game_length = 20

[credits]
initial_balance = 1000
per_game_reward = 10
relay_fee = 1
EOF
    
    echo "Starting relay $id on port $port..."
    P2PGO_CONFIG=$config \
    RUST_LOG=info \
    cargo run --release --bin p2pgo-relay -- \
        --id relay_$id \
        --port $port \
        > $LOG_DIR/relay_$id.log 2>&1 &
    
    echo $! > $LOG_DIR/relay_$id.pid
}

# Function to start a player bot
start_player() {
    local id=$1
    local relay_id=$((id % RELAYS + 1))
    local relay_port=$((BASE_PORT + relay_id))
    
    echo "Starting player $id (connecting to relay $relay_id)..."
    RELAY_URL="http://localhost:$relay_port" \
    RUST_LOG=info \
    cargo run --release --bin p2pgo-bot -- \
        --name player_$id \
        --games 100 \
        --consensus true \
        > $LOG_DIR/player_$id.log 2>&1 &
    
    echo $! > $LOG_DIR/player_$id.pid
}

# Start relays
echo "Starting relays..."
for i in $(seq 1 $RELAYS); do
    start_relay $i
    sleep 1
done

# Wait for relays to initialize
echo "Waiting for relays to initialize..."
sleep 5

# Connect relays in mesh topology
echo "Connecting relays in mesh..."
for i in $(seq 1 $RELAYS); do
    for j in $(seq 1 $RELAYS); do
        if [ $i -ne $j ]; then
            port_i=$((BASE_PORT + i))
            port_j=$((BASE_PORT + j))
            curl -X POST http://localhost:$port_i/connect \
                -d "{\"peer\": \"http://localhost:$port_j\"}" \
                -H "Content-Type: application/json" || true
        fi
    done
done

# Start player bots
echo "Starting player bots..."
for i in $(seq 1 $PLAYERS); do
    start_player $i
    sleep 0.5
done

# Create monitoring script
cat > $LOG_DIR/monitor.sh << 'EOF'
#!/bin/bash
echo "=== Network Status ==="
for pid_file in *.pid; do
    if [ -f "$pid_file" ]; then
        pid=$(cat $pid_file)
        name=${pid_file%.pid}
        if ps -p $pid > /dev/null; then
            echo "✓ $name (PID: $pid)"
        else
            echo "✗ $name (PID: $pid) - NOT RUNNING"
        fi
    fi
done

echo -e "\n=== Relay Statistics ==="
for i in $(seq 1 $RELAYS); do
    port=$((BASE_PORT + i))
    echo -n "Relay $i: "
    curl -s http://localhost:$port/stats 2>/dev/null || echo "Not responding"
done

echo -e "\n=== Game Activity ==="
tail -n 5 player_*.log | grep -E "(Game started|Game finished|Consensus reached)" || echo "No recent activity"
EOF

chmod +x $LOG_DIR/monitor.sh

# Create stop script
cat > $LOG_DIR/stop.sh << 'EOF'
#!/bin/bash
echo "Stopping test network..."
for pid_file in *.pid; do
    if [ -f "$pid_file" ]; then
        pid=$(cat $pid_file)
        echo "Stopping $pid_file (PID: $pid)..."
        kill $pid 2>/dev/null || true
        rm $pid_file
    fi
done
echo "Network stopped."
EOF

chmod +x $LOG_DIR/stop.sh

echo "=========================="
echo "Test network started!"
echo "Logs: $LOG_DIR"
echo "Monitor: $LOG_DIR/monitor.sh"
echo "Stop: $LOG_DIR/stop.sh"
echo "=========================="

# Keep script running and show status
while true; do
    sleep 10
    echo -e "\n$(date): Network Status"
    cd $LOG_DIR && ./monitor.sh
    cd - > /dev/null
done