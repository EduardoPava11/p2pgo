#!/bin/bash
set -e

echo "Building universal P2P Go app..."

# Clean
rm -rf "P2P Go.app"
rm -f P2PGo-*.dmg

# Build with specific settings
export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release -p p2pgo-ui-v2

# Create app bundle
echo "Creating app bundle..."
mkdir -p "P2P Go.app/Contents/"{MacOS,Resources}

# Copy binary
cp target/release/p2pgo-ui-v2 "P2P Go.app/Contents/MacOS/"

# Create Info.plist
cat > "P2P Go.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>p2pgo-ui-v2</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.app</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>2.0.0</string>
    <key>CFBundleVersion</key>
    <string>2.0.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

# Fix dynamic library issues
echo "Fixing library dependencies..."

# List all dependencies
echo "Current dependencies:"
otool -L "P2P Go.app/Contents/MacOS/p2pgo-ui-v2"

# Remove dependency on libunwind if it exists
if otool -L "P2P Go.app/Contents/MacOS/p2pgo-ui-v2" | grep -q "libunwind.dylib"; then
    echo "Removing libunwind dependency..."
    # Try to remove the dependency
    install_name_tool -change /usr/lib/libunwind.dylib /usr/lib/libc++abi.dylib "P2P Go.app/Contents/MacOS/p2pgo-ui-v2" || true
fi

# Sign the app
echo "Signing app..."
codesign --force --deep --sign - "P2P Go.app"

# Verify the app
echo "Verifying app..."
codesign --verify --verbose "P2P Go.app"

# Create DMG
echo "Creating DMG..."
hdiutil create -volname "P2P Go" -srcfolder "P2P Go.app" -ov -format UDZO "P2PGo-universal.dmg"

echo "Done! Created P2PGo-universal.dmg"
echo ""
echo "Final dependencies:"
otool -L "P2P Go.app/Contents/MacOS/p2pgo-ui-v2" | grep -E "^\s" | sort