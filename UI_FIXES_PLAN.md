# P2P Go UI/UX Fixes Plan

## Issues to Fix

### 1. Animation Performance Issues
**Problem**: Stone placement animation is glitchy/not smooth
**Root Cause**: 
- Animation frame rate limiting might be too aggressive (16.67ms target)
- Multiple animations running concurrently without proper optimization
- Possible conflicts between hover animations and placement animations

**Solution**:
- Optimize animation manager to better handle concurrent animations
- Implement animation priority system (placement > hover > pending)
- Use requestAnimationFrame properly with egui's repaint system
- Reduce animation complexity for lower-end devices

### 2. Ghost Moves Error Spam
**Problem**: "Ghost moves will be available after completing 5 games" appears on every stone placement
**Root Cause**: 
- `GetGhostMoves` is sent after every move (line 790 and 808 in app.rs)
- Worker checks games_finished < 5 and sends error message each time

**Solution**:
- Add a flag to track if ghost moves are available
- Only send GetGhostMoves if threshold is met
- Show the error message only once per session
- Consider removing ghost moves feature until enough games are played

### 3. Yellow "Initializing..." During Ticket Generation
**Problem**: Shows yellow "Initializing..." text when ticket is being generated
**Root Cause**: 
- Line 714-720 in app.rs shows "Initializing..." when ticket length <= 50
- This is a temporary state while ticket is being generated

**Solution**:
- Show a proper loading spinner instead of yellow text
- Use ConnectionStatusWidget to track network initialization state
- Only show ticket-related UI when ticket is fully ready
- Add proper loading states for network operations

### 4. Board Size Too Small
**Problem**: Board is too small compared to available screen space
**Root Cause**: 
- Fixed cell_size of 30.0 pixels (line 38 in board_widget.rs)
- Fixed window size of 1200x900 (line 176 in main.rs)
- Board doesn't scale with window size

**Solution**:
- Calculate cell_size dynamically based on available space
- Use minimum of width/height to maintain square aspect ratio
- Target 80-90% of available space for the board
- Implement responsive sizing like OGS/Lichess

### 5. Too Much Empty Space
**Problem**: Layout has too much empty space, especially during gameplay
**Root Cause**: 
- Two-column layout in main menu wastes horizontal space
- Game view doesn't maximize board space
- Side panels take up unnecessary space during gameplay

**Solution**:
- Implement board-centered layout like Lichess
- Move controls to corners or overlay them
- Use collapsible side panels for neural controls
- Minimize UI chrome during active gameplay

### 6. Old DMG Cleanup
**Problem**: Multiple old DMG files cluttering the directory
**Files to Remove**:
- P2PGo-Offline-1.0.0.dmg
- P2PGo-FL.dmg
- P2PGo-Clean.dmg
- P2PGo-Working.dmg
- P2PGo-v2-fixed.dmg
- P2PGo-v2-fixed copy.dmg
- P2PGo-v2-fixed copy 2.dmg
- P2PGo-universal.dmg

**Keep Only**:
- P2P Go.dmg (latest release)
- P2PGo-Offline.dmg (offline version)

## Implementation Priority

1. **Fix board sizing** (High priority - major UX issue)
2. **Fix ghost moves error spam** (High priority - annoying bug)
3. **Clean up DMGs** (Quick win)
4. **Fix animation performance** (Medium priority)
5. **Fix yellow initializing text** (Low priority - cosmetic)
6. **Optimize layout/spacing** (Medium priority - overall UX)

## Best Practices from OGS & Lichess

### From OGS:
- Clean, minimalist design with white background
- Board takes center stage
- Minimal UI chrome
- Clear visual hierarchy
- Responsive board sizing

### From Lichess:
- Board-centered layout
- Smooth animations with Chessground library
- No registration/ads/plugins
- CSS-only styling for flexibility
- Distraction-free environment
- Fast DOM updates with custom diff algorithm

## Technical Implementation Notes

### Board Sizing Algorithm
```rust
// Calculate optimal cell size based on available space
let available_size = ui.available_size();
let margin = 40.0; // Total margin
let board_size = available_size.min_elem() - margin;
let cell_size = board_size / (board_size_cells as f32 - 1.0);
```

### Animation Optimization
```rust
// Only update animations when visible and needed
if ui.is_rect_visible(rect) && self.animation_manager.has_animations() {
    self.animation_manager.update();
    ui.ctx().request_repaint();
}
```

### Layout Structure
```
CentralPanel (full window)
├── Board (80-90% of space, centered)
├── Controls (overlay or minimal sidebar)
└── Status (minimal top bar)
```

## Success Metrics
- Board uses 80%+ of available screen space
- Smooth 60 FPS animations
- No error message spam
- Clean, professional appearance
- Fast, responsive UI