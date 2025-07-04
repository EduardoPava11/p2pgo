# P2P Go Neural Network Guide

## Overview

P2P Go uses a dual neural network system inspired by AlphaGo:

1. **Policy Network** - Predicts the best moves (shown as heat map)
2. **Value Network** - Evaluates who's winning (shown as win percentage)

## Quick Start

### 1. Configure Your Neural Networks

When you first start, answer 10 questions (1-10 scale) to configure your AI personality:

- **Aggression** (1-10): How attacking vs defensive
- **Territory Focus** (1-10): Solid territory vs influence
- **Fighting Spirit** (1-10): Seek fights vs avoid them
- **Pattern Recognition** (1-10): Trust patterns vs calculate deeply
- **Risk Tolerance** (1-10): Safe vs risky play
- **Opening Style** (1-10): Slow/solid vs fast/aggressive
- **Middle Game** (1-10): Strength in complex positions
- **Endgame** (1-10): Precision in counting
- **Learning Rate** (1-10): How fast to adapt from training
- **Creativity** (1-10): Standard vs unconventional moves

### 2. Train Your Networks

Upload SGF files from OGS or other sources:

1. Click "Select SGF Files" 
2. Choose multiple games at once
3. Click "Start Training"
4. Watch the progress bar

The networks learn patterns based on your configuration!

### 3. Use During Play

- **Heat Map OFF by default** (as requested)
- Press **H key** to toggle heat map
- Red/bright areas = suggested moves
- Blue/dark areas = avoid these moves

## Heat Map Colors

- **Red**: High probability moves (good!)
- **Yellow**: Medium probability moves
- **Blue/Dark**: Low probability (avoid)
- **Transparent**: Very unlikely moves

## Position Evaluation

The value network shows:
- **Win %**: Your probability of winning (0-100%)
- **Confidence**: How sure the network is
- **Game Phase**: Opening/Middle/Endgame

## File Storage

Networks are saved as JSON files containing:
- Your configuration (1-10 values)
- Learned patterns from training
- Win/loss statistics
- Training history

### Why JSON?

- Human readable for debugging
- Easy to share between computers
- Can inspect/modify if needed
- Compress with gzip for smaller size

## Testing Workflow

1. **You**: Configure aggressive style (8/10 aggression)
2. **Friend**: Configure territorial style (8/10 territory)
3. **Both**: Train on your SGF collections
4. **Play**: Start game with 2 relay nodes
5. **During Game**: Toggle heat map with H key when needed
6. **Networks**: Show different suggestions based on training!

## Tips

- Upload games from similar players for consistent style
- More games = better pattern recognition
- Mix won and lost games for balance
- Save your network after training
- Share networks with friends to compare styles

## Technical Details

The networks use:
- Pattern matching for common shapes
- Position evaluation for territory/influence
- Move history for ko detection
- Configuration weights to adjust behavior

Each parameter (1-10) directly affects:
- Which patterns get higher weights
- How the position is evaluated
- What moves are suggested

This creates unique AI personalities based on your preferences!