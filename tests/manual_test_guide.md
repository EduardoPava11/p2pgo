# P2P Go V1 Manual Test Guide

## Local Two-Player Test Instructions

This guide will help you verify that the game works correctly and produces CBOR files.

### Setup

1. Open two terminal windows
2. Create test directories:
```bash
# Terminal 1 (Alice - Black)
mkdir -p /tmp/alice_test
cd /tmp/alice_test
export HOME=/tmp/alice_test

# Terminal 2 (Bob - White)  
mkdir -p /tmp/bob_test
cd /tmp/bob_test
export HOME=/tmp/bob_test
```

### Start the Applications

3. Start both instances:
```bash
# Terminal 1 (Alice)
/Users/daniel/p2pgo/target/release/p2pgo-ui-egui --player-name "Alice" --board-size 9

# Terminal 2 (Bob)
/Users/daniel/p2pgo/target/release/p2pgo-ui-egui --player-name "Bob" --board-size 9
```

### Create and Join Game

4. In Alice's UI:
   - Click "Create Game"
   - Copy the connection ticket

5. In Bob's UI:
   - Click "Join Game"
   - Paste the ticket
   - Click "Connect"

### Play the SGF Game

6. Follow this move sequence exactly (SGF coordinates → UI coordinates):

| Move | Player | SGF | UI Click | Description |
|------|--------|-----|----------|-------------|
| 1  | Black | dc | D7 | Top-left area |
| 2  | White | ff | F5 | Center |
| 3  | Black | dg | D3 | Bottom-left |
| 4  | White | ce | C5 | Left side |
| 5  | Black | fh | F2 | Bottom-right |
| 6  | White | fc | F7 | Top-right |
| 7  | Black | ec | E7 | Top-center |
| 8  | White | fd | F6 | Right-center |
| 9  | Black | hg | H3 | Bottom-right corner |
| 10 | White | bc | B7 | Top-left corner |

Continue with remaining 66 moves...

### Complete the Game

7. After all moves (or when you want to end):
   - Both players click "Pass"
   - Mark any dead stones if needed
   - Both click "Accept Score"

### Verify Results

8. Expected outcome:
   - Score should be approximately W+34.5
   - Black territory: ~15-20 points
   - White territory: ~50-55 points

9. Check for CBOR files:
```bash
# Terminal 1 (Alice)
ls -la "/tmp/alice_test/Library/Application Support/p2pgo/finished/"

# Terminal 2 (Bob)
ls -la "/tmp/bob_test/Library/Application Support/p2pgo/finished/"
```

### Verify CBOR Content

10. Test the CBOR file:
```bash
# Find the CBOR file
CBOR_FILE=$(find /tmp/alice_test -name "*.cbor*" | head -1)

# Check file details
file "$CBOR_FILE"
ls -lh "$CBOR_FILE"

# If it's .cbor.gz, it's compressed (game was large)
# If it's .cbor, it's uncompressed
```

### Test Neural Network Training

11. Use the CBOR file for training:
```bash
cd /Users/daniel/p2pgo
cargo run --bin train_neural -- "$CBOR_FILE"
```

## Quick Test (First 10 Moves)

If you want a quicker test, just play the first 10 moves and then both pass:

1. Black D7 (3,2)
2. White F5 (5,5)
3. Black D3 (3,6)
4. White C5 (2,4)
5. Black F2 (5,7)
6. White F7 (5,2)
7. Black E7 (4,2)
8. White F6 (5,3)
9. Black H3 (7,6)
10. White B7 (1,2)
11. Both pass
12. Accept score

This should still create a valid CBOR file.

## Troubleshooting

- **UI not responding**: Check terminal for error messages
- **Can't connect**: Ensure both instances are running and network is working
- **No CBOR file**: Make sure both players accepted the score
- **Wrong score**: Double-check the move sequence

## Success Criteria

✅ Both players can connect  
✅ Moves synchronize instantly  
✅ Game completes with score consensus  
✅ CBOR file is created in archive directory  
✅ CBOR file can be loaded for training  

When all criteria are met, V1 is ready for release!