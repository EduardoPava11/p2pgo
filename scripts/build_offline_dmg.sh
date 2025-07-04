#!/bin/bash
set -e

# Build offline game DMG with all UI fixes

echo "Building offline P2P Go game..."

# Clean previous builds
rm -rf "P2P Go Offline.app"
rm -f "P2PGo-Offline.dmg"

# Build the offline game binary
echo "Building offline game binary..."
cargo build --release --bin offline_game -p p2pgo-ui-egui

# Create app bundle
echo "Creating app bundle..."
mkdir -p "P2P Go Offline.app/Contents/MacOS"
mkdir -p "P2P Go Offline.app/Contents/Resources"

# Copy binary
cp target/release/offline_game "P2P Go Offline.app/Contents/MacOS/P2PGo-Offline"

# Create Info.plist
cat > "P2P Go Offline.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go Offline</string>
    <key>CFBundleDisplayName</key>
    <string>P2P Go Offline</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.offline</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>P2PGo-Offline</string>
    <key>CFBundleIconFile</key>
    <string>appicon.icns</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
</dict>
</plist>
EOF

# Copy icon if available
if [ -f "assets/appicon.icns" ]; then
    cp assets/appicon.icns "P2P Go Offline.app/Contents/Resources/"
fi

# Make executable
chmod +x "P2P Go Offline.app/Contents/MacOS/P2PGo-Offline"

# Create DMG
echo "Creating DMG..."
hdiutil create -volname "P2P Go Offline" -srcfolder "P2P Go Offline.app" -ov -format UDZO "P2PGo-Offline.dmg"

# Clean up
rm -rf "P2P Go Offline.app"

echo "✅ DMG created: P2PGo-Offline.dmg"
echo ""
echo "All UI fixes included:"
echo "- Board size stability (no resizing after 5th move)"
echo "- Red outlines for territory marking"
echo "- Flood fill territory marking"
echo "- Dead stone marking by clicking groups"
echo "- Guild statistics as bar graph"
echo "- Toggle for guild stats"
echo "- Guild calculation at game end only"