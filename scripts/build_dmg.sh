#!/bin/bash
# Complete script to build Apple Silicon DMG

set -e

APP_NAME="P2P Go"
BIN_NAME="p2pgo-ui-egui"
DMG_NAME="${APP_NAME}.dmg"
TMP_DIR="tmp_dmg_build"

echo "=== Building P2P Go Apple Silicon DMG ==="

# Check required tools
echo "Checking required tools..."
command -v rustup > /dev/null || { echo "Error: rustup not found"; exit 1; }
command -v create-dmg > /dev/null || { echo "Installing create-dmg..."; brew install create-dmg; }
command -v dylibbundler > /dev/null || { echo "Installing dylibbundler..."; brew install dylibbundler; }

# Make sure we have the target
echo "Ensuring Apple Silicon target is available..."
rustup target add aarch64-apple-darwin

# Build with proper flags
echo "Building Apple Silicon binary..."
RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Frameworks -C link-arg=-unwindlib=system" \
  cargo build -p ${BIN_NAME} --release --target aarch64-apple-darwin

# Clean up previous builds
echo "Cleaning up previous builds..."
rm -rf "${APP_NAME}.app"
rm -f "${DMG_NAME}"
rm -rf "${TMP_DIR}"
mkdir -p "${APP_NAME}.app/Contents/"{MacOS,Resources,Frameworks}

# Copy binary and icon
echo "Creating app bundle..."
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

# Bundle dylibs
echo "Bundling dylibs..."
dylibbundler -b -x "${APP_NAME}.app/Contents/MacOS/${BIN_NAME}" \
  -d "${APP_NAME}.app/Contents/Frameworks" \
  -p @rpath/ -of

# Sign the app
echo "Signing the app bundle..."
codesign --force --deep --sign - "${APP_NAME}.app"

# Create a temporary directory for DMG creation
echo "Creating clean DMG..."
mkdir -p "${TMP_DIR}"
cp -R "${APP_NAME}.app" "${TMP_DIR}/"
# Also copy the Applications shortcut
ln -s /Applications "${TMP_DIR}/Applications"

# Create DMG with just the app and Applications shortcut
create-dmg \
  --volname "${APP_NAME}" \
  --window-pos 200 120 \
  --window-size 660 400 \
  --icon-size 80 \
  --icon "${APP_NAME}.app" 180 170 \
  --app-drop-link 480 170 \
  --background "assets/dmg_bg.png" \
  --no-internet-enable \
  --format UDZO \
  "${DMG_NAME}" \
  "${TMP_DIR}"

# Clean up
rm -rf "${TMP_DIR}"

echo "✅ ${DMG_NAME} ready"
echo "Try running: open '${DMG_NAME}'"
