# P2P Go Bootstrap & Network Guide

## What's New in P2PGo-BoardMenu.dmg

The main interface is now a **9×9 Go board** where you click positions to access features:
- **Center (4,2)**: Offline Game - Test the complete Go implementation
- **Left (2,4)**: Create Network Game - Start a P2P game
- **Right (6,4)**: Join Game - Connect to existing games
- **Bottom (4,6)**: Settings (future feature)

## Bootstrapping the P2P Network

### Current Bootstrap Methods

1. **Default Relay (Easy)**
   - Uses Iroh's public relay server
   - Works immediately without configuration
   - Good for testing but relies on external infrastructure

2. **Self-Hosted Relay (Independent)**
   - Run your own relay server
   - Configure in `~/Library/Application Support/p2pgo/config.toml`:
   ```toml
   relay_mode = "SelfRelay"
   relay_port = 8443
   ```
   - Other players connect using your IP:port

3. **Custom Relay Network**
   - Use multiple relay addresses
   - Configure custom relays in config:
   ```toml
   relay_mode = "Custom"
   relay_addrs = [
     "/ip4/192.168.1.100/tcp/8443",
     "/dns4/myrelay.example.com/tcp/443"
   ]
   ```

### Testing Two-Player Connection

Since you don't have two computers readily available, here are options:

1. **Local Testing (Same Machine)**
   - Run two instances with different ports:
   ```bash
   # Terminal 1
   P2PGO_PORT=9001 ./P2P\ Go\ Offline.app/Contents/MacOS/P2P\ Go\ Offline
   
   # Terminal 2  
   P2PGO_PORT=9002 ./P2P\ Go\ Offline.app/Contents/MacOS/P2P\ Go\ Offline
   ```

2. **Virtual Machine Testing**
   - Run second instance in VM
   - Use bridged networking
   - Connect via local network

3. **Remote Testing**
   - Deploy to cloud VM (AWS, DigitalOcean)
   - Use self-hosted relay mode
   - Connect from local machine

### How Games Connect

1. **Creating a Game**
   - Generate ticket (contains your node ID + relay info)
   - Game is advertised via gossip protocol
   - Wait for opponent to join

2. **Joining a Game**
   - Enter opponent's ticket OR
   - See available games in lobby (via gossip)
   - Direct P2P connection established

3. **Connection Flow**
   ```
   Player A → Relay ← Player B
        ↓                ↓
   Generate Ticket    Enter Ticket
        ↓                ↓
   Advertise Game    See Game List
        ↓                ↓
   Wait in Lobby      Join Game
        ↓                ↓
   ← Direct P2P Connection →
   ```

## Improved Guild Measurement System

The current guild system shows all players as ~48% Activity because it only measures move-to-move distances. Here's a redesigned system:

### New Guild Attributes

1. **Tempo Guild** (replaces Activity)
   - Fast vs slow play style
   - Measures: moves per minute, response time, game pace
   - Indicators: quick decisive moves vs thoughtful contemplation

2. **Territory Guild** (replaces Avoidance)  
   - Territorial vs influence focus
   - Measures: corner plays, enclosures, moyo building
   - Indicators: secure territory vs center influence

3. **Combat Guild** (replaces Reactivity)
   - Fighting vs peaceful style  
   - Measures: captures, invasions, contact plays
   - Indicators: aggressive attacks vs solid connections

### Implementation Ideas

```rust
pub struct ImprovedGuildClassifier {
    // Tempo measurements
    move_times: Vec<Duration>,
    opening_speed: f32,
    
    // Territory measurements
    corner_plays: u32,
    influence_moves: u32,
    enclosure_count: u32,
    
    // Combat measurements
    contact_plays: u32,
    captures_made: u32,
    groups_attacked: u32,
}
```

### Better Metrics

1. **Opening Patterns** (moves 1-20)
   - Corner approach style
   - Joseki knowledge
   - Framework building

2. **Middle Game** (moves 21-60)
   - Fighting intensity
   - Territory consolidation
   - Group management

3. **Endgame** (moves 61+)
   - Precision
   - Territory optimization
   - Timing of passes

This would create more meaningful player profiles and better distinguish play styles!