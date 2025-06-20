#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Script to build a universal macOS DMG package for development/testing
#
# This script uses cargo-dist to build a macOS DMG with universal binary (x86_64 + arm64)
# It's meant for local development and testing of the DMG packaging

set -e

# Check if cargo-dist is installed
if ! command -v cargo-dist &> /dev/null; then
    echo "Installing cargo-dist..."
    cargo install cargo-dist
fi

# Clean any previous dist builds
echo "Cleaning previous builds..."
rm -rf target/dist

# Build universal macOS binary and DMG
echo "Building universal macOS binary and DMG..."
VERSION=$(cat VERSION 2>/dev/null || echo "0.1.0")
cargo dist build --tag="v$VERSION" --artifacts=host

# Copy the DMG to the project root for convenience
echo "Copying DMG to project root..."
find target/distrib -name "*.dmg" -exec cp {} . \; 2>/dev/null || find target/dist -name "*.dmg" -exec cp {} . \; 2>/dev/null || echo "No DMG found in expected locations"

# Check if the DMG was created
DMG_FILE=$(find . -maxdepth 1 -name "*.dmg")
if [ -n "$DMG_FILE" ]; then
    echo "✅ DMG created successfully: $DMG_FILE"
    echo "You can now install it by double-clicking the DMG file."
else
    echo "❌ Failed to create DMG"
    exit 1
fi
