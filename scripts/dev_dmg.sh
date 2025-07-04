#!/usr/bin/env bash
set -euo pipefail

APP="P2P Go"
BIN="p2pgo-ui-egui"
DMG="${APP}.dmg"

# Read version from VERSION file
VERSION=$(cat "$(dirname "$0")/../VERSION")
echo "Building P2P Go version ${VERSION} for Apple Silicon only"

# Set IROH_RELAY_MODE environment variable for network configuration
export IROH_RELAY_MODE="${IROH_RELAY_MODE:-default}"
echo "🌐 Using relay mode: ${IROH_RELAY_MODE}"

# Prereqs -------------------------------------------------------------
echo "🔍 Checking Apple Silicon prerequisites..."

# Check for rustup
command -v rustup >/dev/null || { 
    echo "❌ Error: rustup not found. Please install rustup from https://rustup.rs/"; 
    exit 1; 
}

# Check that we're on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo "❌ Error: This script is designed for macOS only";
    exit 1;
fi

# Check for Apple Silicon Mac
if [[ "$(uname -m)" != "arm64" ]]; then
    echo "⚠️  Warning: This script is optimized for Apple Silicon Macs";
    echo "   You appear to be on $(uname -m). Build may still work.";
fi

# Check for brew
command -v brew >/dev/null || {
    echo "❌ Error: Homebrew not found. Please install brew from https://brew.sh/";
    exit 1;
}

# Check and install create-dmg if needed
command -v create-dmg >/dev/null || {
    echo "� Installing create-dmg via Homebrew..."
    brew install create-dmg || { echo "❌ Error: Failed to install create-dmg"; exit 1; }
}

command -v dylibbundler >/dev/null || {
    echo "📦 Installing dylibbundler via Homebrew..."
    brew install dylibbundler || { echo "❌ Error: Failed to install dylibbundler"; exit 1; }
}

echo "🚀 Ensuring Apple Silicon target is available..."
rustup target add aarch64-apple-darwin || { echo "❌ Error: Failed to add aarch64-apple-darwin target"; exit 1; }

# Build ---------------------------------------------------------------
echo "📝 Building P2P Go version ${VERSION} for Apple Silicon only"
echo "🌐 Using IROH_RELAY_MODE=${IROH_RELAY_MODE}"

export RUSTFLAGS="-C link-arg=-Wl,-rpath,@executable_path/../Frameworks -C link-arg=-Wl,-rpath,/usr/lib -C link-arg=-Wl,-rpath,@loader_path/../Frameworks"

# Build for Apple Silicon with relay configuration
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
 <key>CFBundleShortVersionString</key>   <string>${VERSION}</string>
 <key>LSMinimumSystemVersion</key>       <string>11.0</string>
 <key>NSHighResolutionCapable</key>      <true/>
</dict></plist>
PLIST

# Bundle the correct libunwind.dylib
echo "📦 Setting up libunwind..."

# Create frameworks directory
mkdir -p "${APP}.app/Contents/Frameworks"

FOUND_LIBUNWIND=0

# First try to extract libunwind directly from the rust stdlib
RUSTLIB_DIR=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/lib
if [ -d "$RUSTLIB_DIR" ]; then
    echo "🔍 Looking for libunwind in Rust stdlib: $RUSTLIB_DIR"
    # Find any libunwind in the rust stdlib
    for file in $(find "$RUSTLIB_DIR" -name "*libunwind*.dylib" -o -name "*libunwind*.a" 2>/dev/null); do
        echo "Found potential libunwind in Rust stdlib: $file"
        if [ -f "$file" ] && file "$file" | grep -q "Mach-O.*arm64"; then
            echo "✅ Using Rust stdlib libunwind from $file"
            cp "$file" "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            install_name_tool -id @rpath/libunwind.1.0.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            chmod 755 "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            FOUND_LIBUNWIND=1
            break
        fi
    done
fi

# If not found in rustlib, try to find libunwind from Homebrew or system locations
if [ "$FOUND_LIBUNWIND" -eq 0 ]; then
    LIBUNWIND_PATHS=(
        "/opt/homebrew/lib/libunwind.1.dylib"
        "/opt/homebrew/opt/llvm/lib/libunwind.1.dylib"
        "/usr/lib/system/libunwind.dylib"
        "/usr/lib/libunwind.1.dylib"
    )

    # Add Homebrew Cellar locations using find
    if [ -d "/opt/homebrew/Cellar" ]; then
        for file in $(find /opt/homebrew/Cellar -name "libunwind.1.dylib" 2>/dev/null); do
            if [ -f "$file" ] && file "$file" | grep -q "Mach-O.*arm64"; then
                LIBUNWIND_PATHS+=("$file")
            fi
        done
    fi

    # Try each path in order
    for path in "${LIBUNWIND_PATHS[@]}"; do
        if [ -f "$path" ] && file "$path" | grep -q "Mach-O.*arm64"; then
            echo "✅ Found ARM64 libunwind at $path"
            # Copy with the exact name the binary is looking for
            cp "$path" "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            # Set the correct ID
            install_name_tool -id @rpath/libunwind.1.0.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            # Ensure it's executable
            chmod 755 "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
            FOUND_LIBUNWIND=1
            break
        fi
    done
fi

# Try to extract libunwind from the Rust compiler itself
if [ "$FOUND_LIBUNWIND" -eq 0 ]; then
    echo "🔍 Trying to extract libunwind from the Rust compiler binaries..."
    # Create a simple Rust program that pulls in libunwind
    TMP_DIR=$(mktemp -d)
    cd "$TMP_DIR"
    
    cat > unwind.rs << 'EOF'
    fn main() { println!("Test libunwind dependency"); }
EOF

    # Compile with static libunwind
    echo "Compiling with bundled libunwind..."
    rustc -C target-feature=+crt-static unwind.rs -o unwind_test
    
    # Check if the binary contains libunwind references
    if otool -L unwind_test | grep -q libunwind; then
        cp unwind_test "${APP}.app/Contents/Frameworks/unwind_test"
        LIBUNWIND_PATH=$(otool -L "${APP}.app/Contents/Frameworks/unwind_test" | grep libunwind | awk '{print $1}')
        
        if [ -n "$LIBUNWIND_PATH" ]; then
            echo "Found reference to libunwind at $LIBUNWIND_PATH"
            # Try to locate the actual file
            if [[ "$LIBUNWIND_PATH" == /* ]] && [ -f "$LIBUNWIND_PATH" ]; then
                cp "$LIBUNWIND_PATH" "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
                install_name_tool -id @rpath/libunwind.1.0.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
                chmod 755 "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
                FOUND_LIBUNWIND=1
            fi
        fi
        rm -f "${APP}.app/Contents/Frameworks/unwind_test"
    fi
    
    cd - > /dev/null
    rm -rf "$TMP_DIR"
fi

# If we still haven't found it, try other potential locations and fallback to system dynamic linker
if [ "$FOUND_LIBUNWIND" -eq 0 ]; then
    echo "🔍 Trying Apple system libunwind..."
    
    # Try system libunwind.dylib and make a copy with the right name
    if [ -f "/usr/lib/system/libunwind.dylib" ]; then
        echo "✅ Using system libunwind.dylib"
        cp "/usr/lib/system/libunwind.dylib" "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
        install_name_tool -id @rpath/libunwind.1.0.dylib "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
        chmod 755 "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
        FOUND_LIBUNWIND=1
    fi
fi

# Check if we managed to set up libunwind
if [ "$FOUND_LIBUNWIND" -eq 1 ]; then
    echo "✅ libunwind.1.0.dylib set up in Frameworks directory"
    # Validate the library
    if file "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib" | grep -q "Mach-O.*arm64"; then
        echo "✓ Verified ARM64 Mach-O format"
    else
        echo "⚠️ WARNING: libunwind.1.0.dylib is not in ARM64 Mach-O format"
    fi
else
    echo "❌ ERROR: Could not find a suitable libunwind library. The application will not run properly."
    exit 1
fi

# Fix deps
echo "📦 Bundling dependencies..."
dylibbundler -b -x "${APP}.app/Contents/MacOS/${BIN}" \
  -d "${APP}.app/Contents/Frameworks" -p @rpath/ -of

# If we have the libunwind.1.0.dylib in place, make sure the binary links to it properly
if [ "$FOUND_LIBUNWIND" -eq 1 ]; then
    echo "🔗 Updating binary to reference bundled libunwind..."
    # Find all libunwind references in the binary and update them to use @rpath
    LIBUNWIND_REFS=$(otool -L "${APP}.app/Contents/MacOS/${BIN}" | grep libunwind | awk '{print $1}')
    
    if [ -n "$LIBUNWIND_REFS" ]; then
        for ref in $LIBUNWIND_REFS; do
            echo "Updating reference: $ref -> @rpath/libunwind.1.0.dylib"
            install_name_tool -change "$ref" "@rpath/libunwind.1.0.dylib" "${APP}.app/Contents/MacOS/${BIN}"
        done
    fi
    
    # Make sure the libunwind library is executable
    echo "🔧 Setting executable permissions on libunwind..."
    chmod 755 "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib"
    
    # Verify the library can be loaded
    echo "🔍 Verifying libunwind can be loaded..."
    if ! otool -L "${APP}.app/Contents/Frameworks/libunwind.1.0.dylib" &>/dev/null; then
        echo "⚠️ WARNING: libunwind.1.0.dylib might not be a valid library"
    else
        echo "✅ libunwind.1.0.dylib appears to be valid"
    fi
    
    # Add additional rpaths to the binary to ensure libraries are found
    echo "🔧 Adding additional rpaths to binary..."
    install_name_tool -add_rpath "@executable_path/../Frameworks" "${APP}.app/Contents/MacOS/${BIN}" 2>/dev/null || true
    install_name_tool -add_rpath "/usr/lib" "${APP}.app/Contents/MacOS/${BIN}" 2>/dev/null || true
fi

# Code signing
echo "🔏 Signing application bundle..."
# First sign all frameworks individually
find "${APP}.app/Contents/Frameworks" -type f -name "*.dylib" -o -name "*.so" | while read -r lib; do
    codesign --force --sign - "$lib"
done

# Then sign the app as a whole with --deep
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

# Generate SHA-256 checksum
echo "🔒 Generating SHA-256 checksum..."
shasum -a 256 "${DMG}" > "${DMG}.sha256"

echo "✅ Built ${DMG} (version ${VERSION}) with checksum ${DMG}.sha256"
echo "🌐 Relay mode: ${IROH_RELAY_MODE}"
echo "📱 Apple Silicon optimized build complete"
echo ""
echo "💡 To test relay connectivity:"
echo "   1. Open the DMG and install the app"
echo "   2. Run the app and check logs for relay multiaddr output"
echo "   3. Test multiplayer games to verify RTT < 500ms"
