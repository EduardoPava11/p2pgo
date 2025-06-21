# P2P Go Development Guide

## Project Overview

P2P Go is a peer-to-peer Go (board game) application built in Rust, enabling gameplay over decentralized networks without requiring central servers. The project targets Apple Silicon Macs exclusively and uses Iroh v0.35 for networking.

## Project Structure

```
p2pgo/
├── core/           # Game logic: board representation, rules, scoring
├── network/        # P2P networking using Iroh v0.35
├── ui-egui/        # Desktop UI using egui/eframe  
├── cli/            # Headless command-line interface for testing
├── trainer/        # Machine learning models for Go AI
└── scripts/        # Build and deployment scripts
```

## Build Instructions

### Apple Silicon DMG Build
```bash
./scripts/dev_dmg.sh     # builds & opens P2P Go.dmg on macOS 11+
```

### Development Builds
```bash
# Build all packages
cargo build --workspace

# Run GUI application
cargo run -p p2pgo-ui-egui

# Run CLI application  
cargo run -p p2pgo-cli -- --role host --size 19
```

### Icon Generation
```bash
./scripts/make_icns.sh   # converts PNG to ICNS
```

## Testing Instructions

### Network Testing
```bash
./scripts/test_iroh_networking.sh    # Network layer tests
./scripts/test_e2e_game.sh           # End-to-end game tests
./scripts/run_two_gui.sh             # Local multiplayer testing
```

### Automated Testing
```bash
# Run all tests (without networking)
cargo test

# Run tests with networking features
cargo test --features iroh

# Run headless tests (for CI)
cargo test --features "iroh,headless"
```

## Development History

### MVP Completion Milestones

#### Phase 1: Core Networking (MVP_COMPLETION.md)
- ✅ Iroh v0.35 Integration with API compatibility updates
- ✅ Real-time move synchronization between players
- ✅ Event broadcasting system with GameEvent::MoveMade
- ✅ Direct peer connections via GameChannel::connect_to_peer()
- ✅ Integration test: "first stone appears on both boards"

#### Phase 2: Internet Deployment (MVP_INTERNET_COMPLETE.md)
- ✅ Iroh relay support for NAT traversal
- ✅ Universal macOS binaries (updated to Apple Silicon only)
- ✅ cargo-dist configuration for DMG distribution
- ✅ GitHub Actions workflow for automated releases
- ✅ Game persistence with CBOR archiving
- ✅ Crash logging with 1GB rotation

#### Phase 3: Network Reliability (RELAY_IMPLEMENTATION_COMPLETE.md)
- ✅ Enhanced relay mode with RelayMode::Default
- ✅ UI status indicators (Green/Yellow/Blue network status)
- ✅ Smart UI with network-ready validation
- ✅ API compatibility fixes for Iroh 0.35
- ✅ Integration tests for relay functionality

#### Phase 4: Apple Silicon Focus (Current)
- ✅ Removed all x86_64/universal2 references
- ✅ System libunwind integration with proper @rpath
- ✅ Streamlined DMG build process
- ✅ macOS 11+ minimum system requirements

## Architecture Notes

### Network Layer
- **Iroh v0.35**: Primary networking stack with relay support
- **Direct Connections**: GameChannel manages peer-to-peer streams
- **Relay Fallback**: Automatic NAT traversal via Iroh relays
- **Ticket System**: Enhanced tickets with game metadata
- **Message Format**: JSON serialization for MoveRecord transmission

### Game Logic
- **Core Module**: Pure game logic with no external dependencies
- **Board Representation**: Efficient coordinate system
- **Scoring**: Territory and area scoring with dead stone detection
- **Move Validation**: Complete rule implementation

### UI Architecture
- **egui/eframe**: Cross-platform immediate-mode GUI
- **Worker Thread**: Background networking with message passing
- **View Management**: Screen/state management system
- **Board Widget**: Interactive game board rendering

### Data Persistence
- **Game Archives**: Finished games saved as CBOR files
- **Crash Logs**: Rotated logging in ~/Library/Logs/p2pgo/
- **Configuration**: Minimal configuration via command-line args

## Release Process

### Version Management
1. Update VERSION file
2. Update version in Cargo.toml files
3. Commit changes: `git commit -am "Bump version to x.y.z"`
4. Create and push tag: `git tag vX.Y.Z && git push origin vX.Y.Z`
5. GitHub Actions automatically builds and releases DMG

### Manual Release
```bash
# Build local DMG
./scripts/dev_dmg.sh

# Test the DMG
open "P2P Go.dmg"
```

## Development Setup

### Prerequisites
- macOS 11+ (Apple Silicon)
- Rust stable toolchain
- Xcode Command Line Tools
- Homebrew packages: `create-dmg`, `dylibbundler`

### First Time Setup
```bash
# Clone repository
git clone https://github.com/danielbank/p2pgo
cd p2pgo

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Apple Silicon target
rustup target add aarch64-apple-darwin

# Install DMG tools
brew install create-dmg dylibbundler

# Build and test
cargo build --workspace
cargo test --features iroh
```

### Development Workflow
```bash
# Make changes to code
# ...

# Test changes
cargo check --all-targets
cargo test --features iroh

# Build DMG for testing
./scripts/dev_dmg.sh

# Test multiplayer locally
./scripts/run_two_gui.sh
```

## Debugging and Troubleshooting

### Common Issues
1. **Network connectivity**: Check relay status indicators in UI
2. **DMG creation**: Ensure create-dmg and dylibbundler are installed
3. **Missing libunwind**: System library bundled automatically in DMG script
4. **Code signing**: Uses ad-hoc signing for local development

### Log Locations
- **Application logs**: Console output or `RUST_LOG=debug`
- **Crash logs**: `~/Library/Logs/p2pgo/crash_*.log`
- **System logs**: Console.app for macOS system messages

### Testing Two-Player Games
```bash
# Terminal A (host)
cargo run -p p2pgo-ui-egui --features iroh
# click "Host Game", copy ticket

# Terminal B (remote) 
cargo run -p p2pgo-ui-egui --features iroh
# click "Join Game", paste ticket
```

## Contributing

### Code Standards
- Follow Rust 2021 edition conventions
- Use `cargo fmt` for formatting
- Run `cargo clippy` for linting
- Add SPDX license headers to new files
- No unsafe code allowed

### Testing Requirements
- Unit tests for core game logic
- Integration tests for networking
- End-to-end tests for complete workflows
- All tests must pass in CI

### Documentation
- Update this guide for architectural changes
- Document new API functions
- Update README.md for user-facing changes
- Maintain changelog for releases
