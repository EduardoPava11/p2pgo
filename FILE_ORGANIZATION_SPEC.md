# P2P Go File Organization Specification

## Overview

P2P Go is a decentralized Go game with neural networks, organized as a Rust workspace with multiple specialized crates. This document specifies the file organization to ensure consistent, maintainable development.

## Project Structure

```
p2pgo/
├── Cargo.toml                 # Workspace manifest
├── README.md                  # Project overview
├── UI_ARCHITECTURE.md         # UI design specification
├── FILE_ORGANIZATION_SPEC.md  # This document
├── LICENSE                    # Project license
├── .gitignore                # Git ignore patterns
├── fix_binary.sh             # macOS binary fixing script
├── build_universal.sh        # Universal DMG build script
│
├── core/                     # Core game logic
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Core exports
│   │   ├── board.rs         # Board representation
│   │   ├── game.rs          # Game state and rules
│   │   ├── sgf.rs           # SGF parser/writer
│   │   ├── scoring.rs       # Territory scoring
│   │   └── moves.rs         # Move validation
│   │
│   └── tests/
│       └── game_tests.rs    # Core game tests
│
├── network/                  # P2P networking
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Network exports
│   │   ├── lobby.rs         # Game lobby system
│   │   ├── iroh_transport.rs # Iroh P2P transport
│   │   ├── gossip.rs        # Gossip protocol
│   │   ├── sync.rs          # Game state sync
│   │   └── relay.rs         # NAT traversal relay
│   │
│   └── tests/
│       └── network_tests.rs # Network integration tests
│
├── neural/                   # Neural network AI
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Neural exports
│   │   ├── dual_net.rs      # Dual neural network
│   │   ├── policy_net.rs    # Policy network
│   │   ├── value_net.rs     # Value network
│   │   ├── relay_net.rs     # Relay decision network
│   │   ├── training/
│   │   │   ├── mod.rs       # Training module
│   │   │   ├── sgf_to_cbor.rs # SGF conversion
│   │   │   └── trainer.rs   # Training pipeline
│   │   └── config.rs        # Neural net config
│   │
│   └── models/              # Saved model weights
│       └── README.md        # Model documentation
│
├── ui-egui/                 # Main UI (egui)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs          # Entry point
│   │   ├── lib.rs           # UI library
│   │   ├── app.rs           # Main application
│   │   ├── worker.rs        # Network worker thread
│   │   ├── msg.rs           # UI<->Network messages
│   │   ├── game_ui.rs       # Game board UI
│   │   ├── lobby_ui.rs      # Lobby UI
│   │   ├── ui_config.rs     # UI configuration
│   │   └── theme.rs         # Visual theming
│   │
│   ├── tests/
│   │   └── ui_tests.rs      # UI integration tests
│   │
│   └── assets/              # UI assets
│       └── icon.png         # Application icon
│
├── ui-v2/                   # Refactored UI (WIP)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # UI v2 library
│   │   ├── main.rs          # Entry point
│   │   ├── core/            # Core components
│   │   │   ├── mod.rs
│   │   │   ├── theme.rs     # Design system
│   │   │   ├── button.rs    # Button component
│   │   │   ├── card.rs      # Card component
│   │   │   └── input.rs     # Input component
│   │   ├── widgets/         # Domain widgets
│   │   │   ├── mod.rs
│   │   │   ├── board_widget.rs
│   │   │   ├── stone_widget.rs
│   │   │   └── neural_panel.rs
│   │   ├── features/        # Feature modules
│   │   │   ├── mod.rs
│   │   │   ├── game_view.rs
│   │   │   ├── lobby_view.rs
│   │   │   └── training_view.rs
│   │   └── app/             # Application shell
│   │       ├── mod.rs
│   │       ├── app.rs       # Main app state
│   │       └── router.rs    # View routing
│   │
│   └── README.md            # UI v2 documentation
│
├── ml_models/               # ML model experiments
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   └── candle_model.rs  # Candle ML framework
│   │
│   └── notebooks/           # Jupyter notebooks
│       └── analysis.ipynb   # Model analysis
│
├── target/                  # Build artifacts (gitignored)
│   ├── debug/
│   └── release/
│
├── data/                    # Game data (gitignored)
│   ├── games/              # Saved games
│   ├── sgf/                # SGF files
│   └── training/           # Training data
│
└── docs/                    # Documentation
    ├── architecture.md      # System architecture
    ├── protocol.md         # P2P protocol spec
    └── api.md              # API documentation
```

## Module Responsibilities

### core/
- **Purpose**: Core game logic, independent of UI/network
- **Dependencies**: None (leaf crate)
- **Exports**: GameState, Move, Board, SGF parser, scoring algorithms
- **Guidelines**: 
  - No async code
  - No UI dependencies
  - Pure game logic only

### network/
- **Purpose**: P2P networking and game synchronization
- **Dependencies**: core, libp2p, iroh, tokio
- **Exports**: Lobby, GameChannel, network messages
- **Guidelines**:
  - All async networking code
  - Handle NAT traversal
  - Ensure game state consistency

### neural/
- **Purpose**: Neural network AI for move suggestions
- **Dependencies**: core, burn/candle for ML
- **Exports**: DualNeuralNet, training pipeline
- **Guidelines**:
  - Modular network architecture
  - Support both inference and training
  - CBOR format for training data

### ui-egui/
- **Purpose**: Main user interface
- **Dependencies**: All other crates
- **Exports**: Executable binary
- **Guidelines**:
  - Handle all user interaction
  - Spawn worker thread for networking
  - Message-passing architecture

### ui-v2/
- **Purpose**: Refactored UI with better architecture
- **Dependencies**: All other crates
- **Status**: Work in progress
- **Guidelines**:
  - 4-layer architecture (core → widgets → features → app)
  - Better separation of concerns
  - Improved visual design

## File Naming Conventions

1. **Rust files**: snake_case (e.g., `game_state.rs`)
2. **Markdown**: UPPER_CASE for specs, Title_Case for docs
3. **Config files**: lowercase with extensions (e.g., `config.toml`)
4. **Scripts**: lowercase with underscores (e.g., `build_release.sh`)

## Import Organization

In each Rust file, organize imports as:
```rust
// 1. Standard library
use std::collections::HashMap;

// 2. External crates
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

// 3. Other workspace crates
use p2pgo_core::{GameState, Move};

// 4. Current crate modules
use crate::lobby::Lobby;
```

## Testing Strategy

1. **Unit tests**: In same file as code (`#[cfg(test)]`)
2. **Integration tests**: In `tests/` directory
3. **E2E tests**: In ui-egui/tests/
4. **Benchmarks**: In `benches/` when needed

## Build Configuration

- Debug builds: Fast compilation, debug symbols
- Release builds: 
  - `opt-level = 3`
  - `lto = true`
  - `panic = "abort"` (reduces binary size)
  - Strip symbols for distribution

## Data Storage

- **Config**: `~/.config/p2pgo/` (Linux/macOS)
- **Logs**: `~/Library/Logs/p2pgo/` (macOS)
- **Game saves**: `~/.local/share/p2pgo/games/`
- **Neural models**: `~/.local/share/p2pgo/models/`

## Development Workflow

1. **Feature branches**: `feature/description`
2. **Bug fixes**: `fix/description`
3. **Experiments**: `experiment/description`
4. **Version tags**: `v0.1.0`

## Future Considerations

1. **WASM support**: Prepare core/neural for WASM compilation
2. **Mobile**: Consider egui mobile support
3. **Plugins**: Design for extensibility
4. **Localization**: Prepare UI strings for i18n

## Maintenance

- Update this spec when adding new modules
- Keep module boundaries clean
- Document public APIs
- Regular dependency updates
- Performance profiling for critical paths