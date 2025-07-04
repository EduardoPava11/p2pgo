# P2P Go Guild System

## Overview

The guild system classifies players into three orthogonal play styles based on their move patterns:

### The Three Guilds

1. **Activity Guild** (Red)
   - Forward-moving, aggressive play
   - Measures vectors from previous stone to new stone
   - Prefers positions closer to enemy stones
   - 10% relay fuel discount for aggressive exploration

2. **Reactivity Guild** (Blue)  
   - Response-based, defensive play
   - Strong reaction to captures
   - Measures backward vectors and responds to threats
   - Standard relay fuel pricing

3. **Avoidance Guild** (Green)
   - Balance-seeking, territorial play
   - Seeks midpoint positions
   - Prefers distance from center and balanced territory
   - 10% premium on relay fuel for careful navigation

## Orthogonal Input Expression

Each move is analyzed through three orthogonal perspectives:

```rust
// Activity: forward vector (from -> to)
let activity_score = (dx.abs() + dy.abs()) / 2.0;

// Reactivity: backward vector (to -> from)  
let reactivity_score = if from_capture { 
    activity_score * 1.5  // Strong reaction to captures
} else { 
    activity_score * 0.8 
};

// Avoidance: distance from midpoint
let mid_x = (from.x + to.x) / 2.0;
let mid_y = (from.y + to.y) / 2.0;
let distance_from_center = ((mid_x - 4.0).abs() + (mid_y - 4.0).abs()) / 2.0;
let avoidance_score = 1.0 / (1.0 + distance_from_center);
```

## Relay Fuel System

Credits work as binary fuel for network mobility:
- 1 credit = 1 relay hop
- 1 credit = 1 new relay friend
- No fractional credits

### Guild-Based Pricing
- Activity Guild: 0.9x cost (rewards exploration)
- Reactivity Guild: 1.0x cost (standard)
- Avoidance Guild: 1.1x cost (careful resource use)

## DJED Stablecoin Integration

A simple stablecoin for marketplace payments:
- 1 DJED = 1 relay credit (target price)
- 150% minimum collateralization
- ELO-based rewards and locking

### ELO Bracket Requirements
- 0-1200: 10 DJED locked
- 1201-1500: 25 DJED locked  
- 1501-1800: 50 DJED locked
- 1801-2000: 100 DJED locked
- 2000+: 200 DJED locked

### Entropy Rewards
Higher ELO players earn more DJED from the rewards pool:
- 0-1200: 1 DJED per win
- 1201-1500: 2 DJED per win
- 1501-1800: 3 DJED per win
- 1801-2000: 5 DJED per win
- 2000+: 8 DJED per win

## Three-Player Higher Order Nodes

Future enhancement where three players (one from each guild) can form a higher-order relay node:
- Must have one player from each guild
- Combined ELO rating
- 1.5x entropy reward multiplier for diversity

## Best Play Activation

Players have limited "best play" hints per game:
- Specialization by game phase (Opening/Middle/Endgame)
- Limited uses force strategic thinking
- Tracks when players activate hints
- Specialized bots work best in their phase

## Marketplace Integration

Models in the marketplace now include:
- Guild affinity (which play style they favor)
- Best play configuration (how many hints, which phase)
- Pricing in fuel credits or DJED
- Specialization tracking

## Implementation Status

✅ Implemented:
- Guild classification system
- Orthogonal move analysis  
- Relay fuel credit system
- DJED stablecoin basics
- Marketplace integration
- Guild display in offline game

🚧 Future Work:
- ink! smart contract integration
- Network relay fuel consumption
- Actual DJED minting/burning
- Three-player node formation
- Best play hint limits in UI