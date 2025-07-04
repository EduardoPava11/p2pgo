#!/bin/bash
set -e

echo "Testing DMG creation..."

# Build for current architecture only (faster)
cd /Users/daniel/p2pgo/ui-egui
cargo build --release

# Create simple DMG
APP_NAME="P2P Go"
DMG_NAME="P2PGo-test.dmg"
DIST_DIR="../dist"

# Clean and create directories
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/${APP_NAME}.app/Contents/MacOS"
mkdir -p "$DIST_DIR/${APP_NAME}.app/Contents/Resources"

# Copy binary
cp ../target/release/p2pgo-ui-egui "$DIST_DIR/${APP_NAME}.app/Contents/MacOS/${APP_NAME}"
chmod +x "$DIST_DIR/${APP_NAME}.app/Contents/MacOS/${APP_NAME}"

# Create minimal Info.plist
cat > "$DIST_DIR/${APP_NAME}.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.desktop</string>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Create DMG
hdiutil create -volname "P2P Go" -srcfolder "$DIST_DIR/${APP_NAME}.app" -ov -format UDZO "$DIST_DIR/$DMG_NAME"

echo "DMG created at: $DIST_DIR/$DMG_NAME"
echo "Size: $(du -h "$DIST_DIR/$DMG_NAME" | cut -f1)"