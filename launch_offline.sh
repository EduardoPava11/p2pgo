#!/bin/bash
# Launch the offline Go game

echo "Launching P2P Go..."
echo "================================"
echo ""
echo "Game Modes:"
echo "1. Traditional 2D (9×9)"
echo "   - Clean OGS-style interface"
echo "   - Guild classification system"
echo "   - Territory marking for scoring"
echo ""
echo "2. 3D Three Planes (9×9×9)"
echo "   - Three orthogonal intersecting planes"
echo "   - 243 total positions"
echo "   - Three players: Black, White, Red"
echo ""
echo "Switch between modes using the menu bar at the top."
echo ""

# Launch the game
cd "$(dirname "$0")"
./target/release/offline_game