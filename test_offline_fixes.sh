#!/bin/bash
# Test script to verify offline game fixes

echo "Testing Offline Game UI Fixes..."
echo "================================"
echo ""
echo "This will launch the offline game to verify:"
echo "1. Board doesn't resize after 5th move"
echo "2. No guild percentages during play"
echo "3. Guild stats only at game end"
echo "4. Territory marking with red outlines"
echo "5. Flood fill territory selection"
echo "6. Dead stone group marking"
echo ""
echo "Press Enter to launch the game..."
read

# Launch the offline game
./target/release/offline_game

echo ""
echo "Test checklist:"
echo "- [ ] Board stayed same size throughout game?"
echo "- [ ] No guild percentages shown during play?"
echo "- [ ] Guild stats only appeared after game end?"
echo "- [ ] Territory marks have red outlines?"
echo "- [ ] Click empty area fills entire region?"
echo "- [ ] Click stone group marks all as dead?"
echo "- [ ] Bar graph instead of percentages?"