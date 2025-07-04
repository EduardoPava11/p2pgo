#!/bin/bash
set -e

# P2P Go Universal DMG Build Script
# This script creates a universal binary DMG for macOS with M-series chip support

echo "🎯 P2P Go Universal DMG Builder"
echo "=============================="

# Configuration
APP_NAME="P2P Go"
BUNDLE_ID="io.p2pgo.desktop"
VERSION="0.1.0"
DMG_NAME="P2PGo-${VERSION}-universal.dmg"
VOLUME_NAME="P2P Go ${VERSION}"

# Paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
UI_DIR="${PROJECT_ROOT}/ui-egui"
BUILD_DIR="${PROJECT_ROOT}/target"
DIST_DIR="${PROJECT_ROOT}/dist"
APP_BUNDLE="${DIST_DIR}/${APP_NAME}.app"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
print_status() {
    echo -e "${GREEN}▶${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Clean previous builds
print_status "Cleaning previous builds..."
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}"

# Check for required tools
print_status "Checking required tools..."
command -v cargo >/dev/null 2>&1 || { print_error "cargo is required but not installed."; exit 1; }
command -v lipo >/dev/null 2>&1 || { print_error "lipo is required but not installed."; exit 1; }
command -v create-dmg >/dev/null 2>&1 || print_warning "create-dmg not found. Will use hdiutil instead."

# Build for both architectures
print_status "Building for x86_64..."
cd "${UI_DIR}"
cargo build --release --target x86_64-apple-darwin

print_status "Building for aarch64 (Apple Silicon)..."
cargo build --release --target aarch64-apple-darwin

# Create universal binary
print_status "Creating universal binary..."
lipo -create \
    "${BUILD_DIR}/x86_64-apple-darwin/release/p2pgo-ui-egui" \
    "${BUILD_DIR}/aarch64-apple-darwin/release/p2pgo-ui-egui" \
    -output "${DIST_DIR}/p2pgo-universal"

# Verify universal binary
print_status "Verifying universal binary..."
lipo -info "${DIST_DIR}/p2pgo-universal"

# Create app bundle structure
print_status "Creating app bundle..."
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

# Copy binary
cp "${DIST_DIR}/p2pgo-universal" "${APP_BUNDLE}/Contents/MacOS/${APP_NAME}"
chmod +x "${APP_BUNDLE}/Contents/MacOS/${APP_NAME}"

# Create Info.plist
print_status "Creating Info.plist..."
cat > "${APP_BUNDLE}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSApplicationName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
EOF

# Create icon if it doesn't exist
if [ ! -f "${PROJECT_ROOT}/assets/appicon.icns" ]; then
    print_warning "Icon file not found. Creating placeholder..."
    mkdir -p "${PROJECT_ROOT}/assets"
    # Create a simple icon using iconutil or sips if available
    if command -v sips >/dev/null 2>&1; then
        # Create a simple colored square as placeholder
        echo '<?xml version="1.0" encoding="UTF-8"?>
<svg width="1024" height="1024" xmlns="http://www.w3.org/2000/svg">
  <rect width="1024" height="1024" fill="#1a1a1a"/>
  <circle cx="512" cy="512" r="400" fill="#ffffff"/>
  <circle cx="512" cy="512" r="350" fill="#1a1a1a"/>
  <text x="512" y="600" font-family="Arial" font-size="400" fill="#ffffff" text-anchor="middle">囲</text>
</svg>' > "${DIST_DIR}/icon.svg"
        
        # Convert SVG to PNG (placeholder - would need proper conversion)
        touch "${PROJECT_ROOT}/assets/appicon.icns"
    fi
fi

# Copy icon
if [ -f "${PROJECT_ROOT}/assets/appicon.icns" ]; then
    cp "${PROJECT_ROOT}/assets/appicon.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"
else
    print_warning "Icon file still not found. App will use default icon."
fi

# Sign the app if certificate is available
if security find-identity -v -p codesigning | grep -q "Developer ID Application"; then
    print_status "Signing application..."
    SIGNING_IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | awk '{print $2}')
    codesign --force --deep --sign "${SIGNING_IDENTITY}" "${APP_BUNDLE}"
    print_status "Application signed successfully"
else
    print_warning "No Developer ID certificate found. App will not be signed."
    print_warning "Users may need to right-click and select 'Open' to run the app."
fi

# Create DMG
print_status "Creating DMG..."
if command -v create-dmg >/dev/null 2>&1; then
    # Use create-dmg for a nicer DMG with background
    create-dmg \
        --volname "${VOLUME_NAME}" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "${APP_NAME}.app" 200 190 \
        --hide-extension "${APP_NAME}.app" \
        --app-drop-link 600 185 \
        "${DIST_DIR}/${DMG_NAME}" \
        "${APP_BUNDLE}"
else
    # Fallback to hdiutil
    print_status "Using hdiutil to create DMG..."
    
    # Create temporary DMG
    hdiutil create -volname "${VOLUME_NAME}" \
        -srcfolder "${APP_BUNDLE}" \
        -ov -format UDZO \
        "${DIST_DIR}/${DMG_NAME}"
fi

# Verify DMG
print_status "Verifying DMG..."
hdiutil verify "${DIST_DIR}/${DMG_NAME}"

# Notarize if credentials are available
if [ -n "$APPLE_ID" ] && [ -n "$APPLE_ID_PASSWORD" ]; then
    print_status "Notarizing DMG..."
    xcrun altool --notarize-app \
        --primary-bundle-id "${BUNDLE_ID}" \
        --username "${APPLE_ID}" \
        --password "${APPLE_ID_PASSWORD}" \
        --file "${DIST_DIR}/${DMG_NAME}"
    print_status "Notarization request submitted. Check status with xcrun altool --notarization-history"
else
    print_warning "Skipping notarization. Set APPLE_ID and APPLE_ID_PASSWORD to enable."
fi

# Summary
echo
print_status "Build complete!"
echo "  DMG: ${DIST_DIR}/${DMG_NAME}"
echo "  Size: $(du -h "${DIST_DIR}/${DMG_NAME}" | cut -f1)"
echo
print_status "Distribution checklist:"
echo "  ✓ Universal binary (Intel + Apple Silicon)"
echo "  ✓ macOS 11.0+ compatible"
if security find-identity -v -p codesigning | grep -q "Developer ID Application"; then
    echo "  ✓ Code signed"
else
    echo "  ✗ Not signed (users will see security warning)"
fi
if [ -n "$APPLE_ID" ]; then
    echo "  ✓ Notarization submitted"
else
    echo "  ✗ Not notarized (users will see security warning)"
fi

echo
print_status "To test the DMG:"
echo "  1. Copy to a different Mac"
echo "  2. Double-click to mount"
echo "  3. Drag ${APP_NAME} to Applications"
echo "  4. Launch from Applications folder"