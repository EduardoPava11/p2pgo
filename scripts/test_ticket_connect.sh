#!/bin/bash
# Test script for ticket-based connection functionality

set -e

# Build the project
echo "Building project..."
cargo build --features iroh

# Start first instance in host mode
echo "Starting host instance (CLI)..."
cargo run --bin p2pgo-cli -- --role host --size 9 > /tmp/p2pgo_host.log &
HOST_PID=$!

# Wait for ticket to be generated
sleep 2

# Extract the ticket from the log
TICKET=$(grep -A 1 "Share this ticket" /tmp/p2pgo_host.log | tail -n 1)

if [ -z "$TICKET" ]; then
    echo "Failed to extract ticket from host log"
    cat /tmp/p2pgo_host.log
    kill $HOST_PID
    exit 1
fi

echo "Host ticket: $TICKET"

# Start second instance with the ticket
echo "Starting client instance (CLI) with ticket..."
cargo run --bin p2pgo-cli -- --ticket "$TICKET" --size 9 > /tmp/p2pgo_client.log &
CLIENT_PID=$!

# Wait for connection to establish
sleep 5

echo "Host log:"
echo "======================================="
cat /tmp/p2pgo_host.log
echo "======================================="
echo "Client log:"
echo "======================================="
cat /tmp/p2pgo_client.log
echo "======================================="

echo "To test UI, run: cargo run --bin p2pgo-ui-egui -- --ticket \"$TICKET\""
echo ""
echo "Press Ctrl+C to terminate test processes"

# Wait for user to terminate
wait
