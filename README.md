# p2pgo

p2pgo is a peer-to-peer Go (board game) application built in Rust. It enables users to play the game of Go over a peer-to-peer network without requiring a central server.

## MVP Status ✅

The **Internet-Ready MVP** is now **COMPLETE**! 

✅ **Mac-ready universal-2 artifact** - DMG + signed binary via cargo-dist  
✅ **P2P networking with Internet relays** - Full Iroh relay support  
✅ **Game persistence** - Finished games archived as CBOR files  
✅ **Crash logging** - 1GB rotation in `~/Library/Logs/p2pgo/`  
✅ **Spectator mode** - `--spectator` flag for Iroh seed nodes  
✅ **Complete network layer** - Router wiring for gossip, docs, blobs, P2P  
✅ **Real-time move synchronization** - Working with relay fallback  
✅ **Direct peer connections** - Working with ticket exchange  

The MVP delivers a complete peer-to-peer Go game ready for Internet deployment.

## Architecture

The project is structured as a Rust workspace with the following crates:

- **core**: Board representation, game rules, and data structures  
- **network**: Peer-to-peer networking using Iroh v0.35  
- **ui-egui**: Desktop UI using egui/eframe  
- **cli**: Headless command-line interface for testing  
- **trainer**: Machine learning models for Go AI

## Features

- ✅ Play Go on 9×9, 13×13, or 19×19 boards
- ✅ Internet-ready P2P networking with relay support
- ✅ Real-time move synchronization with fallback mechanisms
- ✅ Game persistence (finished games archived as CBOR)
- ✅ Crash logging with 1GB rotation (macOS ~/Library/Logs/)
- ✅ Spectator-only seed nodes for network reliability
- ✅ Universal-2 macOS binaries with signed DMG distribution
- ✅ Desktop UI with high-contrast monochrome design
- ✅ CLI interface with spectator mode support
- ✅ Gossip-based network discovery with direct connections
- 🔄 Game replay functionality (in progress)
- 🔄 AI training integration (in progress)

## Installation

### macOS

#### Download DMG (Recommended)

1. Download the latest macOS DMG from the [Releases](https://github.com/danielbank/p2pgo/releases) page
2. Open the DMG file
3. Drag P2P Go to your Applications folder
4. Launch from Applications or Spotlight

#### Build from Source

```bash
# Clone the repository
git clone https://github.com/danielbank/p2pgo.git
cd p2pgo

# Build and create a DMG package
./scripts/dev_dmg.sh

# Or build and run directly
cargo run --package p2pgo-ui-egui
```

## Development

### Requirements

- Rust 1.65 or later
- macOS 11+ (Big Sur or later) for DMG packaging

### Building macOS DMG

```bash
# Create development DMG (universal binary for Intel and Apple Silicon)
./scripts/dev_dmg.sh

# Generate ICNS from PNG icon (if you have updated the icon)
./scripts/make_icns.sh assets/icon.png assets/appicon.icns
./scripts/dev_dmg.sh
```

### Creating App Icons

```bash
# Convert PNG to ICNS format for macOS
./scripts/make_icns.sh assets/icon.png assets/appicon.icns
```

### CI/CD

The project uses GitHub Actions for continuous integration and deployment:

- Every Git tag matching `v*.*.*` triggers a release build
- Automated testing with `cargo test --workspace --all-features`
- macOS universal binary (x86_64 + arm64) with signed DMG
- Homebrew tap distribution

To trigger a new release:

```bash
# Update VERSION file and version in Cargo.toml if needed
# Commit changes
git commit -am "Bump version to x.y.z"

# Create and push a version tag
git tag vX.Y.Z
git push origin vX.Y.Z
```

### Data Storage

- Finished games: `~/Library/Application Support/p2pgo/finished/YYYY-MM-DD_vs_<opponent>.cbor`
- Log files: `~/Library/Logs/p2pgo/*.log` (1GB rotation, max 5 files)
- Crash reports: `~/Library/Logs/p2pgo/crash_*.log`

## Quick Start

### Testing the MVP

```bash
# Run the MVP integration test
make mvp-test

# Run all tests with networking
cargo test --features iroh

# Run headless tests (for CI)
cargo test --features "iroh,headless"
```

### Playing a Game

1. **Start the first player:**
   ```bash
   cargo run -p p2pgo-ui-egui
   ```

2. **Generate connection ticket** (shown in UI)

3. **Start the second player:**
   ```bash
   cargo run -p p2pgo-ui-egui  
   ```

4. **Join using the ticket** from step 2

5. **Play Go!** - Moves appear on both boards in real-time

### CLI Mode

```bash
# Host a game
cargo run -p p2pgo-cli -- --role host --size 19

# Join a game  
cargo run -p p2pgo-cli -- --role join --ticket <TICKET>

# Run as spectator-only seed node (helps with network reliability)
cargo run -p p2pgo-cli -- --spectator
```

### Building Releases

```bash
# Install cargo-dist for release builds
cargo install cargo-dist

# Build universal macOS binaries
cargo dist build --target universal2-apple-darwin

# Build for all platforms (requires GitHub Actions)  
cargo dist plan --output-format=json
```

## Requirements

- Rust 2021 edition (stable channel)
- No unsafe code  
- Dependencies: egui, eframe, tokio, iroh v0.35, serde

## Setup Instructions

To get started with the p2pgo project, follow these steps:

1. **Clone the repository:**
   ```
   git clone https://github.com/yourusername/p2pgo.git
   cd p2pgo
   ```

2. **Build the project:**
   ```
   cargo build --workspace
   ```

3. **Run the UI application (default):**
   ```
   cargo run
   ```
   
   or explicitly:
   ```
   cargo run -p p2pgo-ui-egui
   ```

# play on two machines
```
cargo run --release &     # window 1 – auto shows 9×9 ticket
cargo run --release &     # window 2 – paste ticket → Join
```

4. **Run the CLI application:**
   ```
   cargo run -p p2pgo-cli -- --role host --size 19
   ```
   ```

3. **Run the application:**
   ```
   cargo run
   ```

## Usage Examples

Once the application is running, you can initiate peer connections and start communicating. Refer to the documentation in `src/lib.rs` for detailed usage instructions and API references.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue for any enhancements or bug fixes.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.