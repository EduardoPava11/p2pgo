#!/bin/bash
# Simple DMG installer for P2P Go Offline Game on macOS

set -e

echo "🎯 Creating P2P Go DMG Installer (Simple Version)..."

# Configuration
APP_NAME="P2P Go Offline"
BUNDLE_ID="io.p2pgo.offline"
VERSION="1.0.0"
DMG_NAME="P2PGo-Offline-${VERSION}.dmg"
BUILD_DIR="target/release/bundle/osx"
BINARY_PATH="target/release/offline_game"

# Check if binary exists
if [ ! -f "${BINARY_PATH}" ]; then
    echo "❌ Binary not found at ${BINARY_PATH}"
    echo "   Please run: cargo build --bin offline_game -p p2pgo-ui-egui --release"
    exit 1
fi

# Create app bundle structure
echo "🏗️ Creating app bundle..."
APP_DIR="${BUILD_DIR}/${APP_NAME}.app"
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

# Copy binary
echo "📋 Copying binary..."
cp "${BINARY_PATH}" "${APP_DIR}/Contents/MacOS/P2P Go Offline"
chmod +x "${APP_DIR}/Contents/MacOS/P2P Go Offline"

# Create Info.plist
echo "📝 Creating Info.plist..."
cat > "${APP_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go Offline</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

# Create a simple DMG
echo "💿 Creating DMG..."
DMG_TEMP="/tmp/p2pgo_dmg_$$"
rm -rf "${DMG_TEMP}"
mkdir -p "${DMG_TEMP}"

# Copy app to temporary directory
cp -R "${APP_DIR}" "${DMG_TEMP}/"

# Create Applications symlink
ln -s /Applications "${DMG_TEMP}/Applications"

# Create README
cat > "${DMG_TEMP}/README.txt" << EOF
P2P Go - Offline Mode
Version ${VERSION}

INSTALLATION:
1. Drag "P2P Go Offline.app" to the Applications folder
2. Double-click to launch

FIRST LAUNCH:
On first launch, macOS may show a security warning.
To open:
1. Right-click on the app
2. Select "Open" from the menu
3. Click "Open" in the dialog

FEATURES:
- 9x9 Go board with territory marking
- Click to place stones
- Click empty spaces to mark territory
- Beautiful gradient stone rendering
- Detailed score breakdown

Enjoy playing Go!
EOF

# Create the DMG
rm -f "${DMG_NAME}"
hdiutil create -volname "${APP_NAME}" -srcfolder "${DMG_TEMP}" -ov -format UDZO "${DMG_NAME}"

# Cleanup
rm -rf "${DMG_TEMP}"

echo "✅ DMG created successfully!"
echo ""
echo "📦 Output: ${PWD}/${DMG_NAME}"
echo "📏 Size: $(du -h "${DMG_NAME}" | cut -f1)"
echo ""
echo "🚀 To install:"
echo "   1. Double-click ${DMG_NAME}"
echo "   2. Drag 'P2P Go Offline' to Applications"
echo "   3. Launch from Applications"
echo ""
echo "💡 Tip: The app is unsigned. On first launch:"
echo "   - Right-click the app and select 'Open'"
echo "   - Click 'Open' in the security dialog"