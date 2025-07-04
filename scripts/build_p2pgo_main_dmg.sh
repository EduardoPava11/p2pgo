#!/bin/bash
set -e

# Build p2pgo_main DMG with Lichess-inspired UI

echo "Building P2P Go with new UI..."

# Clean previous builds
rm -rf "P2P Go New.app"
rm -f "P2PGo-New.dmg"

# Build the p2pgo_main binary
echo "Building p2pgo_main binary..."
cargo build --release --bin p2pgo_main -p p2pgo-ui-egui || {
    echo "Build failed - trying with --no-default-features"
    cargo build --release --bin p2pgo_main -p p2pgo-ui-egui --no-default-features --features native
}

# Create app bundle
echo "Creating app bundle..."
mkdir -p "P2P Go New.app/Contents/MacOS"
mkdir -p "P2P Go New.app/Contents/Resources"

# Copy binary
cp target/release/p2pgo_main "P2P Go New.app/Contents/MacOS/P2P Go New"

# Create Info.plist
cat > "P2P Go New.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleDisplayName</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.desktop</string>
    <key>CFBundleVersion</key>
    <string>0.1.5</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.5</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go New</string>
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
    cp assets/appicon.icns "P2P Go New.app/Contents/Resources/"
fi

# Make executable
chmod +x "P2P Go New.app/Contents/MacOS/P2P Go New"

# Create DMG
echo "Creating DMG..."
hdiutil create -volname "P2P Go" -srcfolder "P2P Go New.app" -ov -format UDZO "P2PGo-New.dmg"

# Clean up
rm -rf "P2P Go New.app"

echo "✅ DMG created: P2PGo-New.dmg"
echo ""
echo "New features included:"
echo "- Lichess-inspired UI design"
echo "- SGF file selection (1-10 files)"
echo "- Visual neural network training"
echo "- Consistent dark theme throughout"
echo "- Integrated game controls around board"