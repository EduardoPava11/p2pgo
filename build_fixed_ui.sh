#!/bin/bash
set -e

echo "Building fixed P2P Go UI..."

# Build core and neural libraries first
cargo build --release -p p2pgo-core
cargo build --release -p p2pgo-neural

# Compile the fixed UI directly without the broken ui-egui library
rustc --edition 2021 \
    ui-egui/src/bin/p2pgo_fixed.rs \
    -o target/release/p2pgo_fixed \
    --extern p2pgo_core=target/release/libp2pgo_core.rlib \
    --extern p2pgo_neural=target/release/libp2pgo_neural.rlib \
    --extern eframe=$(find target/release/deps -name "libeframe-*.rlib" | head -1) \
    --extern egui=$(find target/release/deps -name "libegui-*.rlib" | head -1) \
    --extern rand=$(find target/release/deps -name "librand-*.rlib" | head -1) \
    -L target/release/deps \
    -C opt-level=3

echo "✅ Built fixed UI successfully!"

# Create DMG
APP_NAME="P2P Go Fixed"
DMG_NAME="P2PGo-Fixed.dmg"

rm -rf "$APP_NAME.app"
mkdir -p "$APP_NAME.app/Contents/MacOS"
mkdir -p "$APP_NAME.app/Contents/Resources"

cp target/release/p2pgo_fixed "$APP_NAME.app/Contents/MacOS/P2P Go"
cp assets/appicon.icns "$APP_NAME.app/Contents/Resources/" || true

cat > "$APP_NAME.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.fixed</string>
    <key>CFBundleVersion</key>
    <string>0.3.0</string>
    <key>CFBundleExecutable</key>
    <string>P2P Go</string>
    <key>CFBundleIconFile</key>
    <string>appicon.icns</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

chmod +x "$APP_NAME.app/Contents/MacOS/P2P Go"

rm -f "$DMG_NAME"
hdiutil create -volname "P2P Go Fixed" -srcfolder "$APP_NAME.app" -ov -format UDZO "$DMG_NAME"
rm -rf "$APP_NAME.app"

echo "✅ Created $DMG_NAME with working Create Game button!"