#!/bin/bash
set -e

echo "🎯 Building P2P Go Demo DMG..."

# Configuration
APP_NAME="P2P Go Demo"
BUNDLE_ID="io.p2pgo.demo"
VERSION="0.1.0"
DMG_NAME="P2PGo-Demo.dmg"

# First, let's check if we have a working binary from before
if [ -f "target/release/p2pgo_main" ]; then
    echo "✅ Found existing p2pgo_main binary"
    BINARY_PATH="target/release/p2pgo_main"
elif [ -f "target/release/offline_game" ]; then
    echo "✅ Found existing offline_game binary"
    BINARY_PATH="target/release/offline_game"
elif [ -f "target/release/simple_ui" ]; then
    echo "✅ Found existing simple_ui binary"
    BINARY_PATH="target/release/simple_ui"
else
    echo "❌ No suitable binary found"
    echo "Attempting to build quick_demo..."
    
    # Try to build just the core without network
    rustc quick_demo.rs \
        --edition 2021 \
        -O \
        --extern p2pgo_core=target/release/deps/libp2pgo_core.rlib \
        --extern eframe=target/release/deps/libeframe.dylib \
        --extern egui=target/release/deps/libegui.dylib \
        -L target/release/deps \
        -o target/release/quick_demo || {
        echo "❌ Build failed"
        echo "Looking for any working binary..."
        
        # Find any binary that might work
        for binary in target/release/p2pgo* target/release/offline* target/release/simple*; do
            if [ -f "$binary" ] && [ -x "$binary" ]; then
                echo "✅ Found binary: $binary"
                BINARY_PATH="$binary"
                break
            fi
        done
    }
    
    if [ -f "target/release/quick_demo" ]; then
        BINARY_PATH="target/release/quick_demo"
    fi
fi

if [ -z "$BINARY_PATH" ] || [ ! -f "$BINARY_PATH" ]; then
    echo "❌ No binary available for DMG creation"
    exit 1
fi

echo "📦 Using binary: $BINARY_PATH"

# Create app bundle structure
APP_DIR="${APP_NAME}.app"
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

# Copy binary
cp "${BINARY_PATH}" "${APP_DIR}/Contents/MacOS/P2P Go Demo"
chmod +x "${APP_DIR}/Contents/MacOS/P2P Go Demo"

# Create Info.plist
cat > "${APP_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go Demo</string>
    <key>CFBundleDisplayName</key>
    <string>P2P Go Demo</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go Demo</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
</dict>
</plist>
EOF

# Create DMG
echo "💿 Creating DMG..."
rm -f "${DMG_NAME}"

# Create a temporary directory for DMG contents
DMG_TEMP="/tmp/p2pgo_dmg_$$"
rm -rf "${DMG_TEMP}"
mkdir -p "${DMG_TEMP}"

# Copy app to temporary directory
cp -R "${APP_DIR}" "${DMG_TEMP}/"

# Create Applications symlink
ln -s /Applications "${DMG_TEMP}/Applications"

# Create README
cat > "${DMG_TEMP}/README.txt" << EOF
P2P Go - Demo Version
Version ${VERSION}

INSTALLATION:
1. Drag "P2P Go Demo.app" to the Applications folder
2. Double-click to launch

FEATURES:
- Classic Go gameplay
- 9x9, 13x13, and 19x19 boards
- Territory counting
- Capture detection
- Ko rule enforcement

This is a demo version showcasing the core Go game engine.
Full P2P networking features coming soon!

Enjoy playing Go!
EOF

# Create the DMG
hdiutil create -volname "${APP_NAME}" -srcfolder "${DMG_TEMP}" -ov -format UDZO "${DMG_NAME}"

# Cleanup
rm -rf "${DMG_TEMP}"
rm -rf "${APP_DIR}"

echo "✅ DMG created successfully!"
echo ""
echo "📦 Output: ${PWD}/${DMG_NAME}"
echo "📏 Size: $(du -h "${DMG_NAME}" | cut -f1)"
echo ""
echo "🚀 To install:"
echo "   1. Double-click ${DMG_NAME}"
echo "   2. Drag 'P2P Go Demo' to Applications"
echo "   3. Launch from Applications"
echo ""
echo "💡 Note: The app is unsigned. On first launch:"
echo "   - Right-click the app and select 'Open'"
echo "   - Click 'Open' in the security dialog"