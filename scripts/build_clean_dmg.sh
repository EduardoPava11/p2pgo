#!/bin/bash
set -e

echo "Building Clean P2P Go DMG..."

# Use the existing working binary
BINARY="/Users/daniel/p2pgo/target/release/p2pgo-ui-egui"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Building offline game..."
    cargo build --release --bin offline_game
    BINARY="/Users/daniel/p2pgo/target/release/offline_game"
fi

# Create clean app bundle
APP_NAME="P2P Go"
rm -rf "$APP_NAME.app"
mkdir -p "$APP_NAME.app/Contents/MacOS"
mkdir -p "$APP_NAME.app/Contents/Resources"

# Copy binary
cp "$BINARY" "$APP_NAME.app/Contents/MacOS/P2P Go"

# Copy icon
if [ -f "assets/appicon.icns" ]; then
    cp assets/appicon.icns "$APP_NAME.app/Contents/Resources/"
fi

# Create Info.plist
cat > "$APP_NAME.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleDisplayName</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.app</string>
    <key>CFBundleVersion</key>
    <string>0.2.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.2.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go</string>
    <key>CFBundleIconFile</key>
    <string>appicon.icns</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
</dict>
</plist>
EOF

# Make executable
chmod +x "$APP_NAME.app/Contents/MacOS/P2P Go"

# Create DMG
DMG_NAME="P2PGo-Clean.dmg"
rm -f "$DMG_NAME"
hdiutil create -volname "P2P Go" -srcfolder "$APP_NAME.app" -ov -format UDZO "$DMG_NAME"

# Clean up
rm -rf "$APP_NAME.app"

echo "✅ Created $DMG_NAME"
echo ""
echo "Key Features:"
echo "- Clean black/white/red UI"
echo "- 9x9 Go board"
echo "- Neural network heat maps (press H)"
echo "- SGF file training"
echo "- P2P networking ready"