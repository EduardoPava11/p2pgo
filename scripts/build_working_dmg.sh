#!/bin/bash
set -e

echo "Building P2P Go DMG..."

# Use the existing working binary
if [ -f "P2P Go.app/Contents/MacOS/p2pgo-ui-egui" ]; then
    echo "Using existing binary from P2P Go.app"
    
    # Create new DMG with updated version
    rm -f "P2PGo-FL.dmg"
    
    # Update version in Info.plist
    sed -i '' 's/0.1.4/0.1.5-FL/g' "P2P Go.app/Contents/Info.plist"
    
    # Create DMG
    hdiutil create -volname "P2P Go FL" -srcfolder "P2P Go.app" -ov -format UDZO "P2PGo-FL.dmg"
    
    echo "✅ Created P2PGo-FL.dmg with Federated Learning preview"
else
    echo "❌ No existing binary found. Building offline game as fallback..."
    cargo build --release --bin offline_game -p p2pgo-ui-egui
    
    mkdir -p "P2P Go FL.app/Contents/MacOS"
    mkdir -p "P2P Go FL.app/Contents/Resources"
    
    cp target/release/offline_game "P2P Go FL.app/Contents/MacOS/P2P Go FL"
    cp assets/appicon.icns "P2P Go FL.app/Contents/Resources/" || true
    
    cat > "P2P Go FL.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go FL</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.fl</string>
    <key>CFBundleVersion</key>
    <string>0.1.5</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.5-FL</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go FL</string>
    <key>CFBundleIconFile</key>
    <string>appicon.icns</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
    
    chmod +x "P2P Go FL.app/Contents/MacOS/P2P Go FL"
    hdiutil create -volname "P2P Go FL" -srcfolder "P2P Go FL.app" -ov -format UDZO "P2PGo-FL.dmg"
    rm -rf "P2P Go FL.app"
fi