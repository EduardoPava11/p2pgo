#!/bin/bash
# Launch the 3D Go game

echo "Launching 3D Go (9×9×9)..."
echo "================================"
echo ""
echo "Game Features:"
echo "- Three-player variant (Black, White, Red)"
echo "- 9x9x9 cubic board with 729 positions"
echo "- Three orthogonal viewing planes (XY, XZ, YZ)"
echo "- Navigate between 9 levels in each plane"
echo "- Spherical stones with 3D shading"
echo ""
echo "Controls:"
echo "- Click on intersections to toggle stones"
echo "- Switch between XY, XZ, YZ plane views"
echo "- Select levels 1-9 within each plane"
echo "- 'Clear Board' to reset"
echo "- 'Random Stones' for testing"
echo ""

# Launch the 3D game
./target/release/go3d