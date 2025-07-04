#!/bin/bash
# Create DMG for offline P2P Go game

set -e

echo "Creating DMG for P2P Go Offline..."

# Create app bundle structure
APP_NAME="P2P Go.app"
TEMP_DIR=$(mktemp -d)
APP_DIR="$TEMP_DIR/$APP_NAME"

mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp target/release/offline_game "$APP_DIR/Contents/MacOS/P2P Go"

# Create Info.plist
cat > "$APP_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.offline</string>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
</dict>
</plist>
EOF

# Make executable
chmod +x "$APP_DIR/Contents/MacOS/P2P Go"

# Create DMG
DMG_NAME="P2PGo-Offline-$(date +%Y%m%d).dmg"
hdiutil create -volname "P2P Go" -srcfolder "$TEMP_DIR" -ov -format UDZO "$DMG_NAME"

# Cleanup
rm -rf "$TEMP_DIR"

echo "Created $DMG_NAME"
echo "Features:"
echo "- Traditional 2D Go (9×9)"
echo "- 3D Three-Plane Go (243 positions)"
echo "- Three players: Black, White, Red"