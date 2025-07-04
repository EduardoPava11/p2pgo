# P2P Go - Project Focus

## Current Priority: PRODUCT LAUNCH

### What We're Building
A peer-to-peer Go game that works between two computers using relay nodes, with neural network assistance and a clean, Lichess-inspired UI.

### Core Features for Launch

1. **P2P Networking**
   - Two players connect via relay nodes
   - Simple lobby system
   - Game state synchronization

2. **UI Requirements** 
   - Colors: Black, White, Red only
   - Bold typography
   - Large window (1200x900) but not fullscreen
   - Consistent design between menu and game board
   - Buttons around the board (New Game, Join Game at top)

3. **Neural Networks**
   - Heat maps OFF by default (toggle with H key)
   - SGF file training (select 1-10 files)
   - Visual feedback during training
   - Dual network system (Policy + Value like AlphaGo)

4. **Game Features**
   - 9x9 board initially
   - Standard Go rules
   - Pass/Resign buttons
   - Move history display

### What's NOT in Scope for Launch
- CLI interface (sunset)
- Guilds system
- 3D visualization
- Complex relay economics
- Blockchain integrations
- Tournament systems

### Next Phase (After Launch)
- Federated learning with 9x9 micro-nets
- Relay optimization using neural networks
- Model marketplace
- 9x9x9 three-player variant
- 13x13 boards

### Technical Stack
- Rust with egui for UI
- libp2p for networking
- Neural networks in Rust
- JSON serialization for models
- SGF parser for training data