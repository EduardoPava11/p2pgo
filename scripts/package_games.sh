#!/bin/bash
# Package both offline and 3D games into DMG

set -e

echo "Packaging P2P Go Games..."

# Create temporary directory for DMG contents
TEMP_DIR=$(mktemp -d)
APP_DIR="$TEMP_DIR/P2P Go Games"
mkdir -p "$APP_DIR"

# Copy binaries
echo "Copying game binaries..."
cp target/release/offline_game "$APP_DIR/P2P Go.app"
cp target/release/go3d "$APP_DIR/P2P Go 3D.app"

# Create simple launcher script
cat > "$APP_DIR/README.txt" << EOF
P2P Go Games
============

This package contains:

1. P2P Go.app - Traditional 9x9 Go game
   - Clean OGS-style interface
   - Guild classification system
   - Territory marking

2. P2P Go 3D.app - Experimental 3D 9x9x9 Go
   - Three-player variant (Black/White/Red)
   - 729 positions in 3D space
   - Navigate between viewing planes

Double-click either app to launch.

For more information: https://github.com/yourusername/p2pgo
EOF

# Create DMG
echo "Creating DMG..."
DMG_NAME="P2PGo-Games-$(date +%Y%m%d).dmg"
hdiutil create -volname "P2P Go Games" -srcfolder "$TEMP_DIR" -ov -format UDZO "$DMG_NAME"

# Cleanup
rm -rf "$TEMP_DIR"

echo "Created $DMG_NAME"