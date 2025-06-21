#!/bin/bash

# SPDX-License-Identifier: MIT OR Apache-2.0
# End-to-end game test script for P2P Go

set -e

echo "=== P2P Go E2E Game Test ==="
echo

# Build the project with iroh features
echo "Building project with iroh features..."
cargo build --features iroh --bin p2pgo-ui-egui

# Create temporary directory for logs
TEST_DIR=$(mktemp -d)
echo "Test logs will be in: $TEST_DIR"

# Kill any existing instances
pkill -f "p2pgo-ui-egui" || true

# Start player 1 (host) in background
echo "Starting Player 1 (host)..."
cargo run --features iroh --bin p2pgo-ui-egui -- --headless 2>&1 | tee "$TEST_DIR/player1.log" &
PLAYER1_PID=$!

# Give player 1 time to start and generate ticket
sleep 3

# Extract ticket from player 1 log
echo "Extracting connection ticket..."
TICKET=""
for i in {1..10}; do
    if TICKET=$(grep -o "Share this ticket.*" "$TEST_DIR/player1.log" 2>/dev/null | sed 's/Share this ticket with opponent://' | tr -d ' \n' | head -1); then
        if [ -n "$TICKET" ]; then
            break
        fi
    fi
    echo "Waiting for ticket generation... (attempt $i/10)"
    sleep 1
done

if [ -z "$TICKET" ]; then
    echo "Failed to extract ticket from player 1 log"
    echo "Player 1 log:"
    cat "$TEST_DIR/player1.log"
    kill $PLAYER1_PID
    exit 1
fi

echo "Got ticket: ${TICKET:0:50}..."

# Start player 2 (guest) with the ticket
echo "Starting Player 2 (guest) with ticket..."
cargo run --features iroh --bin p2pgo-ui-egui -- --headless --ticket "$TICKET" 2>&1 | tee "$TEST_DIR/player2.log" &
PLAYER2_PID=$!

# Let the game run for 30 seconds
echo "Running game for 30 seconds..."
sleep 30

# Stop both players
echo "Stopping players..."
kill $PLAYER1_PID || true
kill $PLAYER2_PID || true

# Wait for processes to clean up
sleep 2

echo
echo "=== Test Results ==="

# Check for successful connection
if grep -q "GameJoined" "$TEST_DIR/player1.log"; then
    echo "✅ Player 1 successfully joined a game"
else
    echo "❌ Player 1 did not join a game"
fi

if grep -q "GameJoined" "$TEST_DIR/player2.log"; then
    echo "✅ Player 2 successfully joined a game"
else
    echo "❌ Player 2 did not join a game"
fi

# Check for successful connection
if grep -q "Successfully connected" "$TEST_DIR/player2.log" || grep -q "Auto-joining game" "$TEST_DIR/player2.log"; then
    echo "✅ Player 2 successfully connected to Player 1"
else
    echo "⚠️  Player 2 connection status unclear"
fi

# Check for move synchronization
if grep -q "MoveMade" "$TEST_DIR/player1.log" && grep -q "MoveMade" "$TEST_DIR/player2.log"; then
    echo "✅ Moves synchronized between players"
else
    echo "⚠️  Move synchronization not detected"
fi

# Check for gossip activity
if grep -q "gossip" "$TEST_DIR/player1.log" || grep -q "gossip" "$TEST_DIR/player2.log"; then
    echo "✅ Gossip networking active"
else
    echo "⚠️  Gossip networking not detected"
fi

# Check for iroh document activity
if grep -q "document" "$TEST_DIR/player1.log" || grep -q "document" "$TEST_DIR/player2.log"; then
    echo "✅ Iroh document synchronization active"
else
    echo "⚠️  Iroh document activity not detected"
fi

# Check for enhanced ticket format
if grep -q "Enhanced" "$TEST_DIR/player1.log" || grep -q "CBOR" "$TEST_DIR/player1.log"; then
    echo "✅ Enhanced ticket format in use"
else
    echo "⚠️  Enhanced ticket format not detected"
fi

# Look for score acceptance (unlikely in 30 seconds but check anyway)
if grep -q "ScoreAcceptedByBoth" "$TEST_DIR/player1.log" && grep -q "ScoreAcceptedByBoth" "$TEST_DIR/player2.log"; then
    echo "✅ Score accepted by both players"
elif grep -q "ScoreCalculated" "$TEST_DIR/player1.log" || grep -q "ScoreCalculated" "$TEST_DIR/player2.log"; then
    echo "⚠️  Score calculated but not yet accepted"
else
    echo "ℹ️  No scoring events (game likely still in progress)"
fi

echo
echo "=== Error Analysis ==="

# Check for errors
ERRORS1=$(grep -i "error\|failed\|panic" "$TEST_DIR/player1.log" | wc -l)
ERRORS2=$(grep -i "error\|failed\|panic" "$TEST_DIR/player2.log" | wc -l)

if [ "$ERRORS1" -eq 0 ] && [ "$ERRORS2" -eq 0 ]; then
    echo "✅ No errors detected in logs"
else
    echo "⚠️  Errors detected: Player1=$ERRORS1, Player2=$ERRORS2"
    if [ "$ERRORS1" -gt 0 ]; then
        echo "Player 1 errors:"
        grep -i "error\|failed\|panic" "$TEST_DIR/player1.log" | head -5
    fi
    if [ "$ERRORS2" -gt 0 ]; then
        echo "Player 2 errors:"
        grep -i "error\|failed\|panic" "$TEST_DIR/player2.log" | head -5
    fi
fi

echo
echo "=== Network Statistics ==="

# Count various network events
GAME_ADVERTS=$(grep -c "GameAdvertised\|game advertisement" "$TEST_DIR"/*.log 2>/dev/null || echo 0)
GOSSIP_EVENTS=$(grep -c "gossip" "$TEST_DIR"/*.log 2>/dev/null || echo 0)
DOC_EVENTS=$(grep -c "document" "$TEST_DIR"/*.log 2>/dev/null || echo 0)

echo "Game advertisements: $GAME_ADVERTS"
echo "Gossip events: $GOSSIP_EVENTS"
echo "Document events: $DOC_EVENTS"

echo
echo "=== Summary ==="

# Overall assessment
if grep -q "GameJoined" "$TEST_DIR/player1.log" && grep -q "GameJoined" "$TEST_DIR/player2.log" && [ "$ERRORS1" -lt 3 ] && [ "$ERRORS2" -lt 3 ]; then
    echo "🎉 E2E test PASSED - Basic networking functionality working"
    EXIT_CODE=0
else
    echo "❌ E2E test FAILED - Check logs for issues"
    EXIT_CODE=1
fi

echo "Full logs available at: $TEST_DIR"
echo

# Keep logs for debugging if test failed
if [ $EXIT_CODE -ne 0 ]; then
    echo "Test failed. Logs preserved at: $TEST_DIR"
    echo "To debug:"
    echo "  Player 1 log: cat $TEST_DIR/player1.log"
    echo "  Player 2 log: cat $TEST_DIR/player2.log"
else
    # Clean up logs on success
    rm -rf "$TEST_DIR"
fi

exit $EXIT_CODE
