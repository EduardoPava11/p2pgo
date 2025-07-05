#!/bin/bash
# Build script for P2P Go macOS DMG
# Creates a distributable DMG with the app bundle

set -e

echo "<�  P2P Go DMG Builder"
echo "====================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
APP_NAME="P2P Go"
BUNDLE_ID="com.p2pgo.app"
VERSION="1.0.0"
DMG_NAME="P2PGo-${VERSION}-macOS"
BUILD_DIR="target/release/bundle"
DMG_DIR="target/dmg"

# Clean previous builds
echo ">� Cleaning previous builds..."
rm -rf "$BUILD_DIR" "$DMG_DIR"
mkdir -p "$BUILD_DIR" "$DMG_DIR"

# Check if binary exists, build if needed
if [ ! -f "target/release/p2pgo-ui-egui" ]; then
    echo -e "\n=( Building release binary..."
    cargo build --release -p p2pgo-ui-egui --bin p2pgo-ui-egui
    
    # Check if binary exists after build
    if [ ! -f "target/release/p2pgo-ui-egui" ]; then
        echo -e "${RED}L Build failed: binary not found${NC}"
        exit 1
    fi
else
    echo -e "\n=( Using existing binary..."
fi

echo -e "${GREEN} Binary built successfully${NC}"

# Create app bundle structure
echo -e "\n=� Creating app bundle..."
APP_BUNDLE="$BUILD_DIR/${APP_NAME}.app"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy binary
cp "target/release/p2pgo-ui-egui" "$APP_BUNDLE/Contents/MacOS/P2PGo"
chmod +x "$APP_BUNDLE/Contents/MacOS/P2PGo"

# Remove quarantine attributes from the app bundle
echo "=� Removing quarantine attributes from app..."
xattr -cr "$APP_BUNDLE" 2>/dev/null || true

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>P2PGo</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright � 2024 P2P Go. MIT License.</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
EOF

# Create a simple icon (you can replace with actual icon)
echo -e "\n<� Creating app icon..."
# For now, create a placeholder icon
touch "$APP_BUNDLE/Contents/Resources/AppIcon.icns"

# Copy any additional resources
echo "=� Copying resources..."
# Copy models if they exist
if [ -d "neural/models" ]; then
    cp -r "neural/models" "$APP_BUNDLE/Contents/Resources/"
fi

# Create DMG
echo -e "\n=� Creating DMG..."
DMG_TEMP="$DMG_DIR/${DMG_NAME}-temp.dmg"
DMG_FINAL="$DMG_DIR/${DMG_NAME}.dmg"

# Create temporary DMG
hdiutil create -size 150m -fs HFS+ -volname "${APP_NAME}" "$DMG_TEMP"

# Mount it
MOUNT_DIR="/Volumes/${APP_NAME}"
hdiutil attach "$DMG_TEMP"

# Copy app bundle
cp -R "$APP_BUNDLE" "$MOUNT_DIR/"

# Create Applications symlink
ln -s /Applications "$MOUNT_DIR/Applications"

# Create background and positioning (optional)
mkdir -p "$MOUNT_DIR/.background"

# Create a comprehensive README with security instructions
cat > "$MOUNT_DIR/README.txt" << EOF
P2P Go - Decentralized Go Game
Version ${VERSION}

INSTALLATION INSTRUCTIONS:
1. Drag "P2P Go.app" to your Applications folder
2. Launch from Applications or Launchpad

IMPORTANT SECURITY NOTE:
If you see "P2P Go is damaged and can't be opened":
1. Right-click on P2P Go.app in Applications
2. Select "Open" from the context menu
3. Click "Open" when prompted with the security warning
4. The app will then run normally in future launches

Alternative method:
1. Open System Preferences > Security & Privacy
2. Click "Open Anyway" if the app appears there
3. Or run in Terminal: xattr -cr "/Applications/P2P Go.app"

This warning appears because the app is not signed with an Apple Developer certificate. The app is safe - you can verify the source code at: https://github.com/EduardoPava11/p2pgo

FEATURES:
- Peer-to-peer gameplay (no servers required)
- Neural network heat maps for move prediction
- Multiple relay modes for privacy control
- Training data export in CBOR format
- 9x9 and 19x19 board support

GETTING STARTED:
1. Enter your player name
2. Create a game and share the code with a friend
3. Or join a game using a friend's code
4. Enjoy decentralized Go!

For more info: https://eduardopava11.github.io/p2pgo/
Source code: https://github.com/EduardoPava11/p2pgo

System Requirements: macOS 10.15 (Catalina) or later
Architecture: ARM64 (Apple Silicon optimized)
EOF

# Unmount
hdiutil detach "$MOUNT_DIR"

# Convert to compressed DMG
echo "=�  Compressing DMG..."
hdiutil convert "$DMG_TEMP" -format UDZO -o "$DMG_FINAL"

# Clean up
rm "$DMG_TEMP"

# Remove quarantine attributes that cause "damaged" errors
echo "=� Removing quarantine attributes..."
xattr -cr "$DMG_FINAL" 2>/dev/null || true

# Sign the DMG (optional, requires Apple Developer ID)
if command -v codesign &> /dev/null; then
    echo -e "\n= Attempting to sign DMG..."
    codesign --sign - "$DMG_FINAL" 2>/dev/null || echo "�  Signing skipped (no certificate)"
fi

# Get final size
DMG_SIZE=$(ls -lh "$DMG_FINAL" | awk '{print $5}')

echo -e "\n${GREEN} DMG created successfully!${NC}"
echo "=� Location: $DMG_FINAL"
echo "=� Size: $DMG_SIZE"
echo ""
echo "=� Ready to upload to website!"

# Create website update info
cat > "$DMG_DIR/dmg_info.json" << EOF
{
  "version": "${VERSION}",
  "filename": "${DMG_NAME}.dmg",
  "size": "${DMG_SIZE}",
  "date": "$(date -u +%Y-%m-%d)",
  "checksum": "$(shasum -a 256 "$DMG_FINAL" | cut -d' ' -f1)"
}
EOF

echo -e "\n=� DMG info saved to: $DMG_DIR/dmg_info.json"