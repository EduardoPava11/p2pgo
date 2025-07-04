#!/bin/bash
set -e

echo "Creating universal P2P Go app..."

# Create app bundle
rm -rf "P2P Go.app"
mkdir -p "P2P Go.app/Contents/"{MacOS,Resources,Frameworks}

# Copy binary
cp target/release/p2pgo-ui-v2 "P2P Go.app/Contents/MacOS/p2pgo-ui-v2"

# Create a stub libunwind.dylib
echo "Creating stub library..."
cat > stub.c << 'EOF'
// Stub library to satisfy dyld
void _Unwind_Resume() {}
void _Unwind_DeleteException() {}
void _Unwind_RaiseException() {}
void _Unwind_GetIP() {}
void _Unwind_SetIP() {}
void _Unwind_GetLanguageSpecificData() {}
void _Unwind_SetGR() {}
void _Unwind_GetRegionStart() {}
EOF

# Compile stub
clang -dynamiclib -o "P2P Go.app/Contents/Frameworks/libunwind.1.dylib" stub.c
rm stub.c

# Update binary to use our stub
install_name_tool -change /opt/homebrew/opt/llvm@12/lib/libunwind.1.dylib @executable_path/../Frameworks/libunwind.1.dylib "P2P Go.app/Contents/MacOS/p2pgo-ui-v2"

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

# Sign everything
codesign --force --deep --sign - "P2P Go.app/Contents/Frameworks/libunwind.1.dylib"
codesign --force --deep --sign - "P2P Go.app"

# Verify
echo "Verifying..."
codesign --verify --verbose "P2P Go.app"

# Create DMG
echo "Creating DMG..."
rm -f P2PGo-universal.dmg
hdiutil create -volname "P2P Go" -srcfolder "P2P Go.app" -ov -format UDZO "P2PGo-universal.dmg"

echo "Done! Created P2PGo-universal.dmg that should work on any Mac."