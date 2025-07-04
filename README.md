# P2P Go 🎯

A decentralized Go game with neural network AI - no servers required! Play Go directly with friends over peer-to-peer connections, enhanced by a powerful neural network that provides move suggestions and game analysis.

![P2P Go Screenshot](docs/images/gameplay.png)

## ✨ Features

### 🌐 True Peer-to-Peer
- **No servers needed** - Connect directly to other players
- **NAT traversal** - Works behind firewalls and routers
- **Automatic discovery** - Find games on your local network
- **Ticket-based invites** - Share a code to connect from anywhere

### 🧠 Neural Network AI
- **Dual network architecture** - Separate policy and value networks like AlphaGo
- **Real-time suggestions** - See top move recommendations as you play
- **Win probability** - Track game advantage in real-time
- **Local inference** - AI runs entirely on your machine

### 🎮 Modern Interface
- **Clean design** - Inspired by OGS and Lichess
- **Always-visible AI panel** - Neural network insights at a glance
- **Keyboard shortcuts** - Quick access to common actions
- **Dark theme** - Easy on the eyes for long sessions

### 📚 Training & Analysis
- **SGF support** - Import and export standard game records
- **Self-training** - Train the neural network on your own games
- **Game replay** - Review and analyze past games
- **Position evaluation** - Understand critical moments

## 🚀 Quick Start

### Download & Install

**macOS**: [Download P2PGo.dmg](https://github.com/EduardoPava11/p2pgo/releases/latest/download/P2PGo-universal.dmg)

Simply download, open the DMG, and drag P2P Go to your Applications folder.

### First Game

1. **Create a game**: Click "Create Game" in the lobby
2. **Share the code**: Send the game code to your friend
3. **Start playing**: Once they join, you can start playing immediately!

## 🛠️ Building from Source

### Prerequisites

- Rust 1.75 or later
- macOS 11.0+ / Linux / Windows 10+

### Build Steps

```bash
# Clone the repository
git clone https://github.com/EduardoPava11/p2pgo.git
cd p2pgo

# Build in release mode
cargo build --release

# Run the application
cargo run --release
```

### Creating a macOS DMG

```bash
# Run the universal build script
./build_universal.sh

# The DMG will be created as P2PGo-universal.dmg
```

## 🏗️ Architecture

P2P Go is built as a Rust workspace with specialized crates:

- **`core/`** - Game logic, rules, and SGF handling
- **`network/`** - P2P networking with libp2p and Iroh
- **`neural/`** - Neural network implementation and training
- **`ui-egui/`** - User interface built with egui

See [FILE_ORGANIZATION_SPEC.md](FILE_ORGANIZATION_SPEC.md) for detailed structure.

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-expand

# Run with hot reload
cargo watch -x run

# Run tests
cargo test

# Check code quality
cargo clippy -- -D warnings
```

## 🧪 Testing P2P Connections

### Local Testing
```bash
# Terminal 1: Start first instance
cargo run -- --player-name "Player 1"

# Terminal 2: Start second instance
cargo run -- --player-name "Player 2"
```

### Network Testing
1. Ensure both computers are on the same network
2. Create a game on one computer
3. Use the ticket/code to connect from the other

## 📖 Documentation

- [Architecture Overview](docs/architecture.md)
- [P2P Protocol Specification](docs/protocol.md)
- [Neural Network Design](docs/neural_network.md)
- [UI Architecture](UI_ARCHITECTURE.md)

## 🗺️ Roadmap

### Version 1.0 (Current)
- ✅ Basic P2P gameplay
- ✅ Neural network move suggestions
- ✅ SGF import/export
- ✅ macOS support

### Version 2.0 (Planned)
- [ ] Windows and Linux releases
- [ ] Tournament/ladder system
- [ ] Spectator mode
- [ ] Advanced time controls
- [ ] Opening book integration

### Version 3.0 (Future)
- [ ] Mobile support
- [ ] Web version (WASM)
- [ ] Cloud-based training
- [ ] Play style analysis

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **egui** - Immediate mode GUI framework
- **libp2p** - P2P networking stack
- **Iroh** - Direct connections and NAT traversal
- **Burn** - Neural network framework
- **OGS/Lichess** - UI/UX inspiration

## 💬 Community

- **Issues**: [GitHub Issues](https://github.com/EduardoPava11/p2pgo/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EduardoPava11/p2pgo/discussions)

---

Made with ❤️ by the P2P Go community. No servers, just Go!