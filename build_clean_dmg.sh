#!/bin/bash
set -e

echo "Building P2P Go Clean DMG..."

APP_NAME="P2P Go"
DMG_NAME="P2PGo-Working.dmg"
BINARY="/Users/daniel/p2pgo/target/release/p2pgo"

# Create app bundle
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
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
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
rm -f "$DMG_NAME"
hdiutil create -volname "P2P Go" -srcfolder "$APP_NAME.app" -ov -format UDZO "$DMG_NAME"

# Clean up
rm -rf "$APP_NAME.app"

echo "✅ Created $DMG_NAME"
echo ""
echo "This version includes:"
echo "- Working Create Game button"
echo "- Join Game functionality"
echo "- Clean UI design"
echo "- Neural network heat maps (press H)"
echo "- Offline play mode"