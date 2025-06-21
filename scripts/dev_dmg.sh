#!/usr/bin/env bash
set -euo pipefail

APP="P2P Go"
BIN="p2pgo-ui-egui"
DMG="${APP}.dmg"

# Prereqs -------------------------------------------------------------
command -v rustup       >/dev/null || { echo "Install rustup"; exit 1; }
command -v create-dmg   >/dev/null || brew install create-dmg
command -v dylibbundler >/dev/null || brew install dylibbundler

rustup target add aarch64-apple-darwin

# Build ---------------------------------------------------------------
RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Frameworks -C link-arg=-unwindlib=system" \
  cargo build --release --target aarch64-apple-darwin -p ${BIN}

BIN_OUT="target/aarch64-apple-darwin/release/${BIN}"

# Bundle --------------------------------------------------------------
rm -rf "${APP}.app"
mkdir -p "${APP}.app/Contents/"{MacOS,Resources,Frameworks}
cp "${BIN_OUT}" "${APP}.app/Contents/MacOS/${BIN}"
cp assets/appicon.icns "${APP}.app/Contents/Resources/"

cat > "${APP}.app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
 <key>CFBundleExecutable</key>           <string>${BIN}</string>
 <key>CFBundleIconFile</key>             <string>appicon</string>
 <key>CFBundleIdentifier</key>           <string>io.p2pgo.desktop</string>
 <key>CFBundleName</key>                 <string>${APP}</string>
 <key>CFBundlePackageType</key>          <string>APPL</string>
 <key>CFBundleShortVersionString</key>   <string>0.1.3</string>
 <key>LSMinimumSystemVersion</key>       <string>11.0</string>
 <key>NSHighResolutionCapable</key>      <true/>
</dict></plist>
PLIST

# Bundle system libunwind
cp /usr/lib/libunwind.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
install_name_tool -id @rpath/libunwind.1.0.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"

# Fix deps
dylibbundler -b -x "${APP}.app/Contents/MacOS/${BIN}" \
  -d "${APP}.app/Contents/Frameworks" -p @rpath/ -of

codesign --force --deep --sign - "${APP}.app"

# DMG ---------------------------------------------------------------
rm -f "${DMG}"
create-dmg --volname "${APP}" \
  --window-size 660 400 \
  --icon-size 80 \
  --background assets/dmg_bg.png \
  --icon "${APP}.app" 180 170 \
  --app-drop-link 480 170 \
  "${DMG}" .

echo "✅ Built ${DMG}"
