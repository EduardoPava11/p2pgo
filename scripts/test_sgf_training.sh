#!/bin/bash
# Test SGF training functionality manually

echo "Testing SGF training functionality..."

# Create test SGF file
mkdir -p test_data
cat > test_data/test_game.sgf << 'EOF'
(;GM[1]FF[4]CA[UTF-8]AP[P2PGo:0.1.0]ST[2]
RU[Japanese]SZ[9]KM[6.5]
PW[White Player]WR[5k]
PB[Black Player]BR[4k]
DT[2024-07-04]
RE[B+2.5]
;B[ee];W[eg];B[ce];W[cg];B[ge];W[gg]
;B[dc];W[gc];B[fc];W[fd];B[ed];W[fe]
;B[ff];W[gf];B[he];W[hf];B[ie];W[ef]
;B[de];W[df];B[cf];W[dg];B[bg];W[bh]
;B[bf];W[ch];B[fb];W[gb];B[fa];W[ga]
;B[eb];W[hd];B[id];W[hc];B[ic];W[hb]
;B[ib];W[ha];B[ia];W[if];B[ag];W[ah]
;B[af];W[pass];B[pass])
EOF

echo "✅ Created test SGF file"

# Run the UI and check if file dialog works
echo ""
echo "🧪 Testing SGF file selection in UI..."
echo "1. Launch the UI"
echo "2. Navigate to Training"
echo "3. Click 'Select SGF Files'"
echo "4. Choose test_data/test_game.sgf"
echo "5. Click 'Start Training'"
echo ""
echo "The training should show progress and complete successfully."

# Build and run ui-v2
echo ""
echo "Building UI v2..."
cargo build -p p2pgo-ui-v2 --release 2>&1 | tail -10

echo ""
echo "✅ Test setup complete. You can now:"
echo "1. Run: cargo run -p p2pgo-ui-v2"
echo "2. Test the SGF training functionality"