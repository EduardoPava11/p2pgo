# P2P Go Offline Game - Complete

## Installation
Install `P2PGo-Offline.dmg` by dragging the app to Applications.

## Game Features

### Core Gameplay ✅
- **9×9 Go board** with proper rules
- **Capture mechanics** - surrounded groups are removed
- **Ko rule** - prevents immediate recapture
- **Pass/Resign** buttons always visible
- **Fixed board size** - no UI shifting during play
- **Clean OGS-style** interface

### Consensus Phase for Territory ✅
After the game ends (pass-pass or resign):

1. **Black marks territory first**
   - Click empty areas to flood fill territory
   - Click stone groups to mark as dead
   - Click "Black: Done Marking ✓"

2. **White marks territory second**  
   - Same marking process
   - Click "White: Done Marking ✓"

3. **Consensus check**
   - If both agree: Territory accepted, CBOR data generated
   - If disagreement: Shows "Territory disagreement" message
   - Players can adjust and click "Accept Territory"

### CBOR Training Data ✅
When consensus is reached:
- Game data is prepared for CBOR encoding
- Move history with proper hash chain structure
- Territory and dead stone information included
- Ready for WASM engine validation
- Can be used to train neural networks

### Guild Classification ✅
- **Activity Guild** - Aggressive, fighting moves
- **Reactivity Guild** - Responding to opponent
- **Avoidance Guild** - Territory-focused play
- Calculated ONLY at game end
- No distracting percentages during play
- Optional bar graph visualization

### UI Improvements ✅
- Board never changes size
- Red outlines on territory marks
- Flood fill for quick territory marking
- Click stone groups to mark dead
- Bar graph instead of percentages
- Toggle button for guild stats
- Professional, focused interface

## How to Play

1. **During Game**
   - Click to place stones
   - Use Pass button when needed
   - No distractions or shifting UI

2. **After Game (Pass-Pass)**
   - Game enters consensus phase
   - Both players mark territory
   - Agreement generates training data

3. **Territory Marking**
   - Click empty area: Flood fills region
   - Click again: Clears that region
   - Click stone group: Marks all as dead
   - Red outlines for visibility

## Technical Integration

### WASM Engine (Future)
- Validates game rules in Haskell-compiled WASM
- Generates proper CBOR encoding
- Signs moves with player keys
- Ensures game integrity

### Neural Network Training
- Agreed games produce training data
- CBOR format preserves full game state
- Territory consensus ensures quality data
- Guild classification provides labels

### Relay System Ready
- Game logic separated from networking
- CBOR format ready for P2P relay
- Consensus mechanism for fair play
- Training data incentivizes proper marking

## Next Steps

1. **WASM Integration**
   - Load Haskell-compiled game engine
   - Validate moves in WASM
   - Generate signed CBOR

2. **Circuit Relay v2**
   - 3-player relay topology
   - Credit-based incentives
   - Use trained models for optimization

3. **Neural Network Pipeline**
   - Train on consensual game data
   - Improve guild classification
   - Optimize relay paths

The offline game is now a complete, professional Go implementation with consensus-based territory marking that generates training data for neural networks!