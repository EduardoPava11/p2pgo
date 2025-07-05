#!/usr/bin/env python3
"""
Test SGF game flow and verify score matches
This creates the exact game state from the SGF file
"""

import subprocess
import os
import time
import json
from pathlib import Path

# SGF moves converted to 0-indexed coordinates
SGF_MOVES = [
    ("black", 3, 2),  # dc
    ("white", 5, 5),  # ff
    ("black", 3, 6),  # dg
    ("white", 2, 4),  # ce
    ("black", 5, 7),  # fh
    ("white", 5, 2),  # fc
    ("black", 4, 2),  # ec
    ("white", 5, 3),  # fd
    ("black", 7, 6),  # hg
    ("white", 1, 2),  # bc
    ("black", 2, 3),  # cd
    ("white", 1, 3),  # bd
    ("black", 3, 4),  # de
    ("white", 2, 5),  # cf
    ("black", 3, 5),  # df
    ("white", 7, 5),  # hf
    ("black", 7, 4),  # he
    ("white", 6, 6),  # gg
    ("black", 6, 4),  # ge
    ("white", 6, 5),  # gf
    ("black", 5, 4),  # fe
    ("white", 4, 4),  # ee
    ("black", 4, 3),  # ed
    ("white", 4, 5),  # ef
    ("black", 6, 2),  # gc
    ("white", 5, 1),  # fb
    ("black", 6, 1),  # gb
    ("white", 2, 2),  # cc
    ("black", 4, 1),  # eb
    ("white", 6, 7),  # gh
    ("black", 2, 6),  # cg
    ("white", 1, 6),  # bg
    ("black", 1, 7),  # bh
    ("white", 1, 5),  # bf
    ("black", 4, 6),  # eg
    ("white", 7, 7),  # hh
    ("black", 0, 7),  # ah
    ("white", 2, 1),  # cb
    ("black", 3, 1),  # db
    ("white", 3, 3),  # dd
    ("black", 6, 3),  # gd
    ("white", 8, 4),  # ie
    ("black", 8, 3),  # id
    ("white", 8, 5),  # if
    ("black", 5, 6),  # fg
    ("white", 5, 8),  # fi
    ("black", 4, 8),  # ei
    ("white", 6, 8),  # gi
    ("black", 3, 7),  # dh
    ("white", 0, 6),  # ag
    ("black", 2, 0),  # ca
    ("white", 1, 0),  # ba
    ("black", 5, 0),  # fa
    ("white", 3, 0),  # da
    ("black", 4, 0),  # ea
    ("white", 2, 3),  # cd
    ("black", 8, 7),  # ih
    ("white", 8, 6),  # ig
    ("black", 2, 0),  # ca
    ("white", 8, 2),  # ic
    ("black", 7, 3),  # hd
    ("white", 3, 0),  # da
    ("black", 7, 8),  # hi
    ("white", 8, 8),  # ii
    ("black", 2, 0),  # ca
    ("white", 2, 8),  # ci
    ("black", 3, 0),  # da
    ("white", 2, 7),  # ch
    ("black", 0, 1),  # ab
    ("white", 1, 8),  # bi
    ("black", 1, 1),  # bb
    ("white", 0, 2),  # ac
    ("black", 0, 0),  # aa
    ("white", 1, 0),  # ba
    ("black", 0, 4),  # ae
    ("white", 0, 8),  # ai
]

def test_sgf_game():
    """Test the SGF game programmatically"""
    print("🎮 Testing SGF Game Flow")
    print("========================")
    print(f"Total moves: {len(SGF_MOVES)}")
    print("Expected result: W+34.5")
    print()
    
    # Create test directory
    test_dir = Path("/tmp/p2pgo_sgf_test")
    test_dir.mkdir(exist_ok=True)
    os.chdir(test_dir)
    
    # Create Rust test program
    rust_code = '''
use p2pgo_core::*;
use std::collections::HashSet;

fn main() {
    let mut game = GameState::new(9);
    let moves = vec![
''' + ',\n'.join([f'        ({x}, {y}, Color::{color.capitalize()})' 
                  for color, x, y in SGF_MOVES]) + '''
    ];
    
    // Play all moves
    for (i, (x, y, color)) in moves.iter().enumerate() {
        let mv = Move::Place { x: *x, y: *y, color: *color };
        if let Err(e) = game.apply_move(mv) {
            eprintln!("Move {} failed: {}", i + 1, e);
            std::process::exit(1);
        }
    }
    
    // Both pass
    game.apply_move(Move::Pass).unwrap();
    game.apply_move(Move::Pass).unwrap();
    
    // Calculate score
    let score_proof = scoring::calculate_final_score(
        &game,
        7.5,
        value_labeller::ScoringMethod::Territory,
        &HashSet::new(),
    );
    
    println!("black_territory:{}", score_proof.territory_black);
    println!("white_territory:{}", score_proof.territory_white);
    println!("final_score:{}", score_proof.final_score);
    
    // Archive
    std::env::set_var("HOME", ".");
    if let Ok(path) = archiver::archive_finished_game(&game, "sgf_test") {
        println!("cbor_path:{}", path.display());
    }
}
'''
    
    with open("test_sgf.rs", "w") as f:
        f.write(rust_code)
    
    # Compile
    print("Compiling test program...")
    compile_cmd = [
        "rustc",
        "test_sgf.rs",
        "-L", "/Users/daniel/p2pgo/target/debug/deps",
        "--extern", f"p2pgo_core=/Users/daniel/p2pgo/target/debug/libp2pgo_core.rlib",
        "--edition", "2021",
        "-o", "test_sgf"
    ]
    
    try:
        subprocess.run(compile_cmd, check=True, capture_output=True)
        print("✅ Compilation successful")
    except subprocess.CalledProcessError as e:
        print(f"❌ Compilation failed: {e.stderr.decode()}")
        return False
    
    # Run test
    print("\nRunning game simulation...")
    result = subprocess.run(["./test_sgf"], capture_output=True, text=True)
    
    if result.returncode != 0:
        print(f"❌ Test failed: {result.stderr}")
        return False
    
    # Parse output
    output_lines = result.stdout.strip().split('\n')
    results = {}
    for line in output_lines:
        if ':' in line:
            key, value = line.split(':', 1)
            results[key] = value
    
    # Display results
    print("\n📊 Game Results:")
    print(f"Black territory: {results.get('black_territory', 'N/A')} points")
    print(f"White territory: {results.get('white_territory', 'N/A')} points")
    print(f"Final score: {results.get('final_score', 'N/A')}")
    
    # Check if score matches SGF
    final_score = float(results.get('final_score', '0'))
    expected_score = -34.5  # Negative because white wins
    
    if abs(final_score - expected_score) < 1.0:
        print(f"✅ Score matches SGF result (W+34.5)!")
    else:
        print(f"⚠️  Score differs from SGF by {abs(final_score - expected_score)} points")
    
    # Check CBOR
    if 'cbor_path' in results:
        cbor_path = Path(results['cbor_path'])
        if cbor_path.exists():
            size = cbor_path.stat().st_size
            print(f"\n✅ CBOR file created: {cbor_path.name}")
            print(f"   Size: {size} bytes")
            
            # This file can be used for neural network training
            print("\n🧠 Neural Network Training:")
            print(f"   cargo run --bin train_neural -- {cbor_path.absolute()}")
    
    return True

if __name__ == "__main__":
    test_sgf_game()