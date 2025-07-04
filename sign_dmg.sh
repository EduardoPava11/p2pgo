#!/bin/bash
# Ad-hoc sign the P2P Go DMG for macOS

set -e

DMG_FILE="P2PGo-Offline-1.0.0.dmg"
APP_NAME="P2P Go Offline"
MOUNT_POINT="/tmp/p2pgo_dmg_mount"

echo "🔏 Ad-hoc signing P2P Go..."

# Check if DMG exists
if [ ! -f "$DMG_FILE" ]; then
    echo "❌ DMG file not found: $DMG_FILE"
    echo "   Please run ./create_simple_dmg.sh first"
    exit 1
fi

# Create a temporary copy to work with
echo "📋 Creating temporary copy..."
cp "$DMG_FILE" "${DMG_FILE}.unsigned"

# Mount the DMG
echo "💿 Mounting DMG..."
hdiutil attach "$DMG_FILE" -mountpoint "$MOUNT_POINT" -nobrowse -quiet

# Copy app to temporary location
echo "📦 Extracting app..."
TEMP_APP="/tmp/${APP_NAME}.app"
rm -rf "$TEMP_APP"
cp -R "$MOUNT_POINT/${APP_NAME}.app" "$TEMP_APP"

# Unmount DMG
echo "💿 Unmounting DMG..."
hdiutil detach "$MOUNT_POINT" -quiet

# Sign the app with ad-hoc signature
echo "🔏 Signing app with ad-hoc signature..."
codesign --force --deep --sign - "$TEMP_APP"

# Verify signature
echo "✅ Verifying signature..."
codesign --verify --verbose "$TEMP_APP"

# Create new DMG with signed app
echo "💿 Creating signed DMG..."
rm -f "${DMG_FILE%.dmg}-signed.dmg"

# Create temporary directory for new DMG
SIGNED_DIR="/tmp/p2pgo_signed"
rm -rf "$SIGNED_DIR"
mkdir -p "$SIGNED_DIR"

# Copy signed app
cp -R "$TEMP_APP" "$SIGNED_DIR/"

# Create Applications symlink
ln -s /Applications "$SIGNED_DIR/Applications"

# Copy README
cat > "$SIGNED_DIR/README.txt" << EOF
P2P Go - Offline Mode (Ad-hoc Signed)
Version 1.0.0

This app has been ad-hoc signed for easier installation.
You should still see a warning on first launch, but it
should be less restrictive than an unsigned app.

INSTALLATION:
1. Drag "P2P Go Offline.app" to Applications
2. Double-click to launch
3. If prompted, click "Open" in the security dialog

FEATURES:
- 9x9 Go board with territory marking
- 9-layer stone rendering with golden ratio
- Beautiful gradient effects
- Detailed score breakdown

Enjoy!
EOF

# Create the signed DMG
hdiutil create -volname "${APP_NAME}" -srcfolder "$SIGNED_DIR" -ov -format UDZO "${DMG_FILE%.dmg}-signed.dmg"

# Cleanup
rm -rf "$TEMP_APP" "$SIGNED_DIR"
mv "$DMG_FILE" "${DMG_FILE}.unsigned"
mv "${DMG_FILE%.dmg}-signed.dmg" "$DMG_FILE"

echo "✅ Successfully created ad-hoc signed DMG!"
echo ""
echo "📦 Signed DMG: $DMG_FILE"
echo "📦 Original (unsigned): ${DMG_FILE}.unsigned"
echo ""
echo "🔏 The app is now ad-hoc signed, which means:"
echo "   - It will run more easily on your Mac"
echo "   - Other Macs will still see security warnings"
echo "   - For distribution, you'd need a Developer ID certificate"
echo ""
echo "🚀 To test:"
echo "   1. Double-click $DMG_FILE"
echo "   2. Install and run the app"
echo "   3. Check Console.app for any logs"