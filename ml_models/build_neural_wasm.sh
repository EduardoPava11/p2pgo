#!/bin/bash
# Build script for P2PGo neural network WASM modules
# Compiles sword_net and shield_net to optimized WASM binaries

set -e

echo "🗡️ Building P2PGo Neural Network WASM Modules"

# Check for required tools
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust toolchain."
    exit 1
fi

if ! command -v wasm-pack &> /dev/null; then
    echo "📦 Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Add WASM target if not present
rustup target add wasm32-unknown-unknown

# Create output directory
mkdir -p ../mobile_demo/android/app/src/main/assets/ai_models

echo "⚔️ Building Sword Net (Offensive/Black Strategy)"
cd sword_net

# Build for WASM with size optimization
cargo build --target wasm32-unknown-unknown --release --no-default-features

# Check WASM binary size
SWORD_SIZE=$(stat -f%z ../target/wasm32-unknown-unknown/release/sword_net.wasm 2>/dev/null || stat -c%s ../target/wasm32-unknown-unknown/release/sword_net.wasm)
echo "📊 Sword Net WASM size: $SWORD_SIZE bytes"

if [ $SWORD_SIZE -gt 409600 ]; then  # 400 KiB limit
    echo "⚠️ Warning: Sword Net exceeds 400 KiB size limit"
fi

# Copy to Android assets
cp ../target/wasm32-unknown-unknown/release/sword_net.wasm ../../mobile_demo/android/app/src/main/assets/ai_models/

cd ..

echo "🛡️ Building Shield Net (Defensive/White Strategy)"
cd shield_net

# Build for WASM with size optimization  
cargo build --target wasm32-unknown-unknown --release --no-default-features

# Check WASM binary size
SHIELD_SIZE=$(stat -f%z ../target/wasm32-unknown-unknown/release/shield_net.wasm 2>/dev/null || stat -c%s ../target/wasm32-unknown-unknown/release/shield_net.wasm)
echo "📊 Shield Net WASM size: $SHIELD_SIZE bytes"

if [ $SHIELD_SIZE -gt 409600 ]; then  # 400 KiB limit
    echo "⚠️ Warning: Shield Net exceeds 400 KiB size limit"
fi

# Copy to Android assets
cp ../target/wasm32-unknown-unknown/release/shield_net.wasm ../../mobile_demo/android/app/src/main/assets/ai_models/

cd ..

echo "🔍 Verifying WASM modules..."

# Verify WASM modules are valid
if command -v wasm-validate &> /dev/null; then
    wasm-validate ../mobile_demo/android/app/src/main/assets/ai_models/sword_net.wasm
    wasm-validate ../mobile_demo/android/app/src/main/assets/ai_models/shield_net.wasm
    echo "✅ WASM modules are valid"
else
    echo "⚠️ wasm-validate not found. Install wabt tools for validation."
fi

# Generate module manifest for Android
cat > ../mobile_demo/android/app/src/main/assets/ai_models/manifest.json << EOF
{
  "version": "1.0.0",
  "models": [
    {
      "id": "sword_net",
      "name": "Sword Network",
      "type": "offensive",
      "player": "black",
      "wasm_file": "sword_net.wasm",
      "size": $SWORD_SIZE,
      "model_id": 1001,
      "version": 100,
      "description": "Aggressive territorial expansion strategy"
    },
    {
      "id": "shield_net", 
      "name": "Shield Network",
      "type": "defensive",
      "player": "white", 
      "wasm_file": "shield_net.wasm",
      "size": $SHIELD_SIZE,
      "model_id": 1002,
      "version": 100,
      "description": "Defensive territory protection strategy"
    }
  ],
  "built_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total_size": $((SWORD_SIZE + SHIELD_SIZE))
}
EOF

echo "📋 Generated manifest.json"

# Calculate BLAKE3 hashes for integrity verification
if command -v b3sum &> /dev/null; then
    echo "🔐 Calculating BLAKE3 hashes..."
    cd ../mobile_demo/android/app/src/main/assets/ai_models/
    b3sum *.wasm > checksums.b3
    echo "✅ BLAKE3 checksums saved to checksums.b3"
    cat checksums.b3
else
    echo "⚠️ b3sum not found. Install BLAKE3 tools for hash verification."
fi

echo ""
echo "🎉 Neural network WASM modules built successfully!"
echo "📁 Output directory: mobile_demo/android/app/src/main/assets/ai_models/"
echo "⚔️ Sword Net: $SWORD_SIZE bytes"
echo "🛡️ Shield Net: $SHIELD_SIZE bytes"
echo "📦 Total size: $((SWORD_SIZE + SHIELD_SIZE)) bytes"
echo ""
echo "🚀 Next steps:"
echo "  1. Test WASM modules in Android app"
echo "  2. Implement Dynamic Feature Module (G3 gate)"
echo "  3. Deploy to DeFi smart contracts"