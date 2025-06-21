#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Simple DMG creation script that doesn't rely on cargo-dist
# This creates a basic DMG for local testing

set -e

echo "Creating a simple DMG from built binaries..."

# Build the binaries first
echo "Building the P2P Go application..."
cargo build --release --bin p2pgo-ui-egui

# Check if the binary was created
BINARY_PATH="target/release/p2pgo-ui-egui"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    exit 1
fi

# Create a temporary directory for the DMG contents
DMG_DIR="tmp_dmg"
APP_NAME="P2P Go.app"
rm -rf "$DMG_DIR"
mkdir -p "$DMG_DIR"

# Create the app bundle structure
mkdir -p "$DMG_DIR/$APP_NAME/Contents/MacOS"
mkdir -p "$DMG_DIR/$APP_NAME/Contents/Resources"

# Copy the binary
cp "$BINARY_PATH" "$DMG_DIR/$APP_NAME/Contents/MacOS/P2P Go"
chmod +x "$DMG_DIR/$APP_NAME/Contents/MacOS/P2P Go"

# Create Info.plist
cat > "$DMG_DIR/$APP_NAME/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.P2PGo</string>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.12</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Copy icon if it exists
if [ -f "assets/appicon.icns" ]; then
    cp "assets/appicon.icns" "$DMG_DIR/$APP_NAME/Contents/Resources/"
    # Add icon reference to Info.plist
    sed -i '' 's|</dict>|    <key>CFBundleIconFile</key>\
    <string>appicon</string>\
</dict>|' "$DMG_DIR/$APP_NAME/Contents/Info.plist"
fi

# Create Applications symlink
ln -s /Applications "$DMG_DIR/Applications"

# Create the DMG
DMG_NAME="P2P-Go-v0.1.0.dmg"
echo "Creating DMG: $DMG_NAME"

# Remove any existing DMG
rm -f "$DMG_NAME"

# Create DMG using hdiutil
hdiutil create -volname "P2P Go" -srcfolder "$DMG_DIR" -ov -format UDZO "$DMG_NAME"

# Clean up
rm -rf "$DMG_DIR"

echo "✅ DMG created successfully: $DMG_NAME"
echo "You can now install the app by double-clicking the DMG file."
