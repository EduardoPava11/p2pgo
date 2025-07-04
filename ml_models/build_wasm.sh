#!/bin/bash

# Build script for compiling Sword and Shield nets to WebAssembly (CPU-only)
# This script builds both models for wasm32-wasi target with size optimization

set -e

echo "Building ML models for WebAssembly (CPU-only)..."

# Ensure wasm32-wasi target is installed
echo "Checking for wasm32-wasi target..."
if ! rustup target list --installed | grep -q "wasm32-wasi"; then
    echo "Installing wasm32-wasi target..."
    rustup target add wasm32-wasi
fi

# Create output directory
OUTPUT_DIR="../mobile_demo/android/app/src/main/assets"
mkdir -p "$OUTPUT_DIR"

echo "Building Sword Net..."
cd sword_net
cargo build --release --target wasm32-wasi
cd ..

echo "Building Shield Net..."
cd shield_net
cargo build --release --target wasm32-wasi
cd ..

# Copy built WASM modules to assets directory
echo "Copying WASM modules to assets directory..."
cp target/wasm32-wasi/release/sword_net.wasm "$OUTPUT_DIR/sword_net.wasm"
cp target/wasm32-wasi/release/shield_net.wasm "$OUTPUT_DIR/shield_net.wasm"

# Check file sizes
echo "WASM module sizes:"
ls -lh "$OUTPUT_DIR"/sword_net.wasm "$OUTPUT_DIR"/shield_net.wasm

# Verify modules are valid
echo "Verifying WASM modules..."
if command -v wasm-validate &> /dev/null; then
    wasm-validate "$OUTPUT_DIR/sword_net.wasm" && echo "✓ sword_net.wasm is valid"
    wasm-validate "$OUTPUT_DIR/shield_net.wasm" && echo "✓ shield_net.wasm is valid"
else
    echo "wasm-validate not found, skipping validation"
fi

echo "✓ ML models built successfully!"
echo "  - sword_net.wasm: Aggressive play model"
echo "  - shield_net.wasm: Defensive play model"
echo "  - Location: $OUTPUT_DIR"