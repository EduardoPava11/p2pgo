#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Script to build a universal macOS DMG package with proper app bundling
# for development/testing without using GitHub Actions
#
# This script:
# 1. Builds both aarch64 and x86_64 binaries 
# 2. Creates a universal2 binary
# 3. Creates a proper macOS .app bundle
# 4. Packages everything into a DMG with icon
# 5. Ad-hoc signs the bundle for local testing

set -euo pipefail

# Colors for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}====== P2P Go Universal DMG Builder ======${NC}"

# Check for required tools
for cmd in cargo lipo create-dmg codesign; do
  if ! command -v $cmd &> /dev/null; then
    echo -e "${RED}ERROR: Required tool '$cmd' is not installed.${NC}"
    case $cmd in
      "create-dmg")
        echo "Install with: brew install create-dmg"
        ;;
      "lipo")
        echo "This should be included with macOS developer tools"
        echo "Run: xcode-select --install"
        ;;
      "cargo")
        echo "Install Rust from https://rustup.rs"
        ;;
      *)
        echo "Please install $cmd to continue"
        ;;
    esac
    exit 1
  fi
done

# Read version from VERSION file or use default
VERSION=$(cat VERSION 2>/dev/null || echo "0.1.0")
echo -e "${BLUE}Building P2P Go v${VERSION}${NC}"

# Clean up previous builds
echo "Cleaning previous builds..."
rm -rf target/universal-apple-darwin
rm -rf target/dmg
rm -f "P2P-Go-v${VERSION}.dmg"
mkdir -p target/universal-apple-darwin
mkdir -p target/dmg

# Build for both architectures
echo -e "${YELLOW}Building x86_64 binary...${NC}"
cargo build --release --bin p2pgo-ui-egui --target x86_64-apple-darwin

echo -e "${YELLOW}Building aarch64 binary...${NC}"
cargo build --release --bin p2pgo-ui-egui --target aarch64-apple-darwin

# Create universal binary
echo -e "${YELLOW}Creating universal binary...${NC}"
lipo -create \
  target/x86_64-apple-darwin/release/p2pgo-ui-egui \
  target/aarch64-apple-darwin/release/p2pgo-ui-egui \
  -output target/universal-apple-darwin/p2pgo-ui-egui

# Create app bundle structure
echo -e "${YELLOW}Creating app bundle...${NC}"
APP_NAME="P2P Go.app"
APP_DIR="target/dmg/$APP_NAME"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"
mkdir -p "$APP_DIR/Contents/Frameworks"

# Copy binary
cp "target/universal-apple-darwin/p2pgo-ui-egui" "$APP_DIR/Contents/MacOS/"
chmod +x "$APP_DIR/Contents/MacOS/p2pgo-ui-egui"

# Create Info.plist
cat > "$APP_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>p2pgo-ui-egui</string>
    <key>CFBundleIdentifier</key>
    <string>io.p2pgo.desktop</string>
    <key>CFBundleName</key>
    <string>P2P Go</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>appicon</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Copy icon
cp assets/appicon.icns "$APP_DIR/Contents/Resources/"

# Add Applications symlink for DMG
echo -e "${YELLOW}Preparing DMG contents...${NC}"
ln -s /Applications "target/dmg/Applications"

# Ad-hoc sign the app bundle
echo -e "${YELLOW}Ad-hoc signing app bundle...${NC}"
codesign --force --deep --sign - "$APP_DIR"

# Create DMG
echo -e "${YELLOW}Creating DMG with background image...${NC}"
DMG_FILE="P2P-Go-v${VERSION}.dmg"

create-dmg \
  --volname "P2P Go" \
  --icon "P2P Go.app" 180 170 \
  --app-drop-link 480 170 \
  --window-size 660 400 \
  --background "assets/dmg_bg.png" \
  --icon-size 80 \
  "$DMG_FILE" \
  "target/dmg/" \
  || echo -e "${RED}Warning: DMG creation had issues, but may have succeeded${NC}"

# Check if DMG was created
if [ -f "$DMG_FILE" ]; then
  DMG_SIZE=$(du -h "$DMG_FILE" | cut -f1)
  echo -e "${GREEN}✅ DMG created successfully: ${DMG_FILE} (${DMG_SIZE})${NC}"
  echo -e "${BLUE}You can now test the DMG by double-clicking it.${NC}"
else
  echo -e "${RED}❌ DMG creation failed.${NC}"
  exit 1
fi
