#!/bin/bash
# Simple script to build Apple Silicon DMG

set -e

APP_NAME="P2P Go"
BIN_NAME="p2pgo-ui-egui"
DMG_NAME="${APP_NAME}.dmg"

# Target
rustup target add aarch64-apple-darwin

# Build with proper flags
RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Frameworks -C link-arg=-unwindlib=system" \
  cargo build -p ${BIN_NAME} --release --target aarch64-apple-darwin

# Create app bundle structure
rm -rf "${APP_NAME}.app"
mkdir -p "${APP_NAME}.app/Contents/"{MacOS,Resources,Frameworks}

# Copy binary and icon
cp "target/aarch64-apple-darwin/release/${BIN_NAME}" "${APP_NAME}.app/Contents/MacOS/"
chmod +x "${APP_NAME}.app/Contents/MacOS/${BIN_NAME}"
cp assets/appicon.icns "${APP_NAME}.app/Contents/Resources/"

# Create Info.plist
cat > "${APP_NAME}.app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${BIN_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.desktop</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.3</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "App bundle created: ${APP_NAME}.app"
echo "You can run it directly with open '${APP_NAME}.app'"
