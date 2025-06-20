#!/bin/bash
# Test script for UI and ticket-based connection functionality

set -e

# Build the project
echo "Building project..."
cargo build --features iroh

# Start first instance in UI mode
echo "Starting first UI instance..."
cargo run --bin p2pgo-ui-egui -- --board-size 9 &
UI1_PID=$!

# Start second instance in UI mode
echo "Starting second UI instance..."
cargo run --bin p2pgo-ui-egui -- --board-size 9 &
UI2_PID=$!

echo ""
echo "Two UI instances are now running."
echo "UI Testing Instructions:"
echo "1. In the first window, click 'Generate Ticket'"
echo "2. Copy the ticket shown"
echo "3. In the second window, click 'Connect by Ticket' and paste the ticket"
echo "4. In the first window, select a board size and click 'Create Game'"
echo "5. Verify that the second window auto-joins the game"
echo ""
echo "Press Ctrl+C to terminate test processes"

# Wait for user to terminate
wait
