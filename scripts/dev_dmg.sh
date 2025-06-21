#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Script to build a universal macOS DMG package with proper app bundling
# for local development without using GitHub Actions
#
# This script:
# 1. Builds both aarch64 and x86_64 binaries 
# 2. Creates a universal2 binary with lipo
# 3. Creates a proper macOS .app bundle
# 4. Ad-hoc signs the bundle
# 5. Creates a DMG with background image and Applications shortcut
# 6. Opens the DMG when done

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
      *)
        echo "Please install $cmd to continue"
        ;;
    esac
    exit 1
  fi
done

# Ensure Rust targets are installed
echo -e "${YELLOW}Ensuring Rust targets are installed...${NC}"
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Get version from Cargo.toml or VERSION file
VERSION=$(grep -A 1 "^\[workspace.package\]" Cargo.toml | grep "version" | cut -d'"' -f2 2>/dev/null || cat VERSION 2>/dev/null || echo "0.1.0")
echo -e "${BLUE}Building P2P Go v${VERSION}${NC}"

# Clean up previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
rm -f "P2P Go.dmg"
rm -rf "P2P Go.app"

# Build both architecture binaries
echo -e "${YELLOW}Building aarch64-apple-darwin target...${NC}"
cargo build --release -p p2pgo-ui-egui --target aarch64-apple-darwin

echo -e "${YELLOW}Building x86_64-apple-darwin target...${NC}"
cargo build --release -p p2pgo-ui-egui --target x86_64-apple-darwin

# Create universal binary with lipo
echo -e "${YELLOW}Creating universal binary with lipo...${NC}"
lipo -create \
  target/aarch64-apple-darwin/release/p2pgo-ui-egui \
  target/x86_64-apple-darwin/release/p2pgo-ui-egui \
  -output p2pgo-ui-egui-universal2

# Bundle the .app
echo -e "${YELLOW}Creating .app bundle...${NC}"
mkdir -p "P2P Go.app/Contents/MacOS"
mkdir -p "P2P Go.app/Contents/Resources"
cp p2pgo-ui-egui-universal2 "P2P Go.app/Contents/MacOS/p2pgo-ui-egui"
chmod +x "P2P Go.app/Contents/MacOS/p2pgo-ui-egui"
cp assets/appicon.icns "P2P Go.app/Contents/Resources/"

# Create Info.plist
cat > "P2P Go.app/Contents/Info.plist" << EOF
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

# Sign the app bundle
echo -e "${YELLOW}Ad-hoc signing the .app bundle...${NC}"
codesign --deep --force --sign - "P2P Go.app"

# Create DMG
echo -e "${YELLOW}Creating DMG...${NC}"
create-dmg \
  --volname "P2P Go" \
  --window-pos 200 120 \
  --window-size 660 400 \
  --icon-size 80 \
  --icon "P2P Go.app" 180 170 \
  --app-drop-link 480 170 \
  --background "assets/dmg_bg.png" \
  "P2P Go.dmg" \
  .

# Cleanup
rm -f p2pgo-ui-egui-universal2

# Open DMG
if [ -f "P2P Go.dmg" ]; then
  echo -e "${GREEN}✅ DMG created successfully: P2P Go.dmg${NC}"
  echo -e "${YELLOW}Opening DMG...${NC}"
  open "P2P Go.dmg"
else
  echo -e "${RED}❌ DMG creation failed${NC}"
  exit 1
fi
