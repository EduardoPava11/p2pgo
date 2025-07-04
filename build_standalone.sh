#!/bin/bash
set -e

echo "Building standalone P2P Go app..."

# Clean previous builds
rm -rf target/release/p2pgo-ui-v2
rm -rf "P2P Go.app"
rm -f P2PGo-*.dmg

# Build with specific flags to avoid dynamic dependencies
export RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Frameworks"
export MACOSX_DEPLOYMENT_TARGET=11.0

cargo build --release -p p2pgo-ui-v2

# Check dependencies
echo "Checking dependencies..."
otool -L target/release/p2pgo-ui-v2

# Create app bundle
echo "Creating app bundle..."
mkdir -p "P2P Go.app/Contents/"{MacOS,Resources,Frameworks}

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

# Remove problematic library dependencies
echo "Fixing library dependencies..."
# Remove libunwind dependency
install_name_tool -change /usr/lib/libunwind.dylib @rpath/libunwind.dylib "P2P Go.app/Contents/MacOS/p2pgo-ui-v2" 2>/dev/null || true

# Sign the app
echo "Signing app..."
codesign --force --deep --sign - "P2P Go.app"

# Create DMG
echo "Creating DMG..."
hdiutil create -volname "P2P Go" -srcfolder "P2P Go.app" -ov -format UDZO "P2PGo-standalone.dmg"

echo "Done! Created P2PGo-standalone.dmg"