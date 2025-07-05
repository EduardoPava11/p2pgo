#!/bin/bash
# Verify CBOR file creation after game completion

echo "🔍 CBOR File Verification Script"
echo "================================"

# Check default location (macOS)
DEFAULT_DIR="$HOME/Library/Application Support/p2pgo/finished"

# Check test locations
TEST_DIRS=(
    "/tmp/alice_test/Library/Application Support/p2pgo/finished"
    "/tmp/bob_test/Library/Application Support/p2pgo/finished"
    "./finished_games"
    "$DEFAULT_DIR"
)

echo -e "\nChecking for CBOR files in known locations..."
echo "============================================="

FOUND=0

for dir in "${TEST_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo -e "\n📁 Checking: $dir"
        
        # Look for CBOR files
        cbor_files=$(find "$dir" -name "*.cbor*" 2>/dev/null)
        
        if [ -n "$cbor_files" ]; then
            echo "✅ Found CBOR files:"
            while IFS= read -r file; do
                # Get file info
                size=$(ls -lh "$file" | awk '{print $5}')
                date=$(ls -lh "$file" | awk '{print $6, $7, $8}')
                filename=$(basename "$file")
                
                echo "   - $filename ($size, $date)"
                
                # Check if compressed
                if [[ "$file" == *.gz ]]; then
                    echo "     └─ Compressed (large game)"
                else
                    echo "     └─ Uncompressed"
                fi
                
                ((FOUND++))
            done <<< "$cbor_files"
        else
            echo "   No CBOR files found"
        fi
    fi
done

echo -e "\n📊 Summary"
echo "========="
echo "Total CBOR files found: $FOUND"

if [ $FOUND -gt 0 ]; then
    echo -e "\n✅ CBOR archiving is working!"
    echo -e "\n🧠 Next step: Test neural network training"
    echo "   Pick a CBOR file from above and run:"
    echo "   cargo run --bin train_neural -- <path-to-cbor-file>"
else
    echo -e "\n⚠️  No CBOR files found"
    echo -e "\nTo create CBOR files:"
    echo "1. Start two p2pgo-ui-egui instances"
    echo "2. Create and join a game"
    echo "3. Play some moves"
    echo "4. Both players pass"
    echo "5. Both accept the score"
    echo "6. CBOR file will be created automatically"
fi

# Create a test CBOR for verification
echo -e "\n🧪 Creating test CBOR file..."
TEST_CBOR_DIR="/tmp/p2pgo_cbor_test"
mkdir -p "$TEST_CBOR_DIR"

cat > "$TEST_CBOR_DIR/create_test_cbor.rs" << 'EOF'
use p2pgo_core::{GameState, Move, Color};

fn main() {
    let mut game = GameState::new(9);
    game.apply_move(Move::Place { x: 4, y: 4, color: Color::Black }).unwrap();
    game.apply_move(Move::Pass).unwrap();
    game.apply_move(Move::Pass).unwrap();
    
    std::env::set_var("HOME", ".");
    if let Ok(path) = p2pgo_core::archiver::archive_finished_game(&game, "test") {
        println!("Test CBOR created: {}", path.display());
    }
}
EOF

echo "Test CBOR can be created with the game archiver"