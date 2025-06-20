#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Script to convert PNG to ICNS for macOS app icons
#
# Usage: ./scripts/make_icns.sh assets/icon.png assets/appicon.icns

set -e

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <input.png> <output.icns>"
    echo "Example: $0 assets/icon.png assets/appicon.icns"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_FILE="$2"

if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file '$INPUT_FILE' does not exist"
    exit 1
fi

# Create temporary directory
TEMP_DIR=$(mktemp -d)
ICONSET_DIR="$TEMP_DIR/icon.iconset"
mkdir -p "$ICONSET_DIR"

echo "Creating iconset from $INPUT_FILE..."

# Generate different icon sizes
sips -z 16 16     "$INPUT_FILE" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32     "$INPUT_FILE" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32     "$INPUT_FILE" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64     "$INPUT_FILE" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128   "$INPUT_FILE" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256   "$INPUT_FILE" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256   "$INPUT_FILE" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512   "$INPUT_FILE" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512   "$INPUT_FILE" --out "$ICONSET_DIR/icon_512x512.png"
sips -z 1024 1024 "$INPUT_FILE" --out "$ICONSET_DIR/icon_512x512@2x.png"

# Convert iconset to icns
iconutil -c icns "$ICONSET_DIR" -o "$OUTPUT_FILE"

# Clean up
rm -rf "$TEMP_DIR"

echo "✅ ICNS file created at $OUTPUT_FILE"
