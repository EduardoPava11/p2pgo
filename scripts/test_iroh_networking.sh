#!/bin/bash
# Test the networked Go implementation with iroh feature

# Exit on error
set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building p2pgo with iroh feature...${NC}"
cargo build --features iroh

echo -e "${GREEN}Running networked Go tests with iroh...${NC}"
cargo test --features iroh -- --test-threads=1 network::game_channel::tests::test_game_channel_with_iroh
cargo test --features iroh -- --test-threads=1 network::tests::gossip_roundtrip::test_gossip_roundtrip
cargo test --features iroh -- --test-threads=1 network::tests::e2e_network_test::test_two_player_game_with_score
cargo test --features iroh -- --test-threads=1 network::tests::scoring_sync_test::test_scoring_agreement_between_nodes

echo -e "${GREEN}Running two UI instances with iroh for manual testing...${NC}"

# Start two instances of the UI in separate terminals 
# First instance
osascript -e 'tell app "Terminal" to do script "cd '$PWD' && cargo run --features iroh -p p2pgo-ui-egui"'

# Wait a bit for the first instance to start
sleep 2

# Second instance
osascript -e 'tell app "Terminal" to do script "cd '$PWD' && cargo run --features iroh -p p2pgo-ui-egui"'

echo -e "${GREEN}Both instances started. Instructions:${NC}"
echo "  1. In the first window, click 'Get Ticket' and copy the ticket"
echo "  2. In the second window, paste the ticket and click 'Connect'"
echo "  3. Create a game in the first window"
echo "  4. The second window should see and join the game automatically"
echo "  5. Play moves alternating between windows"
echo "  6. End the game with two passes and test the scoring"

echo -e "${GREEN}Test complete!${NC}"
