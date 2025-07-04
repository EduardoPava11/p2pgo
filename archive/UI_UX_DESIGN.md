# P2P Go UI/UX Design Guide

## Core Principles

### 1. **Game-First Design**
- The board is the hero element
- Minimal UI chrome
- Clear visual hierarchy
- Instant feedback on actions

### 2. **9x9 Board Focus**
- Square window (900x900px default)
- Board fills available space
- Consistent margins (20px)
- Responsive to window resize

### 3. **Visual Language**
- Middle gray (127) as neutral base
- Subtle probability-based color shifts
- Golden ratio for aesthetic proportions
- 9-layer gradients matching board size

## Current Implementation

### Window Setup
```rust
NativeOptions {
    initial_window_size: Some(Vec2::new(900.0, 900.0)),
    min_window_size: Some(Vec2::new(600.0, 600.0)),
    centered: true,
    vsync: true,
}
```

### Board Rendering
- **Grid**: 9x9 with star points at proper positions
- **Stones**: Multi-layer gradient with golden ratio highlights
- **Territory**: Click-to-toggle marking system
- **Coordinates**: Optional A-J (skip I), 1-9 labels

### Stone Design
```rust
// 9 layers for each stone
const LAYERS: usize = 9;

// Golden ratio positioning
let highlight_offset = radius / GOLDEN_RATIO;
let highlight_radius = radius / (GOLDEN_RATIO * GOLDEN_RATIO);
```

## UI Elements

### 1. **Main Game View**
```
┌─────────────────────────────────┐
│            9×9 Go               │
│                                 │
│  Current: ● Captures: ● 2 ○ 0  │
│                                 │
│  ┌─────────────────────────┐   │
│  │                         │   │
│  │      [Go Board]         │   │
│  │                         │   │
│  └─────────────────────────┘   │
│                                 │
│  [Pass] [Resign] [Territory]   │
│                                 │
│  Guild: Activity (68.2%)        │
└─────────────────────────────────┘
```

### 2. **Score Display**
- Real-time score tracking
- Detailed breakdown on game end
- Clear arithmetic showing point sources

### 3. **Guild Indicator**
- Shows after 5 moves
- Percentage affinities for each guild
- Color-coded (Red/Blue/Green)

## Interaction Design

### Click Behavior
1. **On Board**: Place stone at nearest intersection
2. **Territory Mode**: Toggle marking (None→Black→White→None)
3. **Hover**: Show ghost stone at valid positions

### Keyboard Shortcuts
- `Space`: Pass
- `R`: Resign
- `T`: Toggle territory mode
- `N`: New game
- `Esc`: Exit territory mode

## Color Palette

### Base Colors
- **Board**: Wood texture (#DEB887)
- **Grid**: Dark brown (#654321)
- **Background**: Middle gray (#7F7F7F)

### Guild Colors
- **Activity**: Warm red (#DC6464)
- **Reactivity**: Cool blue (#6464DC)
- **Avoidance**: Balanced green (#64DC64)

### Feedback Colors
- **Valid move**: Green glow
- **Invalid move**: Red flash
- **Last move**: Yellow marker

## Animation & Feedback

### Stone Placement
- Subtle scale animation (0.8 → 1.0)
- Soft drop shadow appearance
- Sound effect (future)

### Captures
- Fade out animation
- Particle effect (future)
- Update capture count

### Territory Marking
- Smooth color transition
- Border highlight on hover
- Clear visual states

## Responsive Design

### Window Resizing
```rust
let available = ui.available_size();
let board_size = available.x.min(available.y) - 100.0;
```

### Minimum Sizes
- Window: 600x600px
- Board: 400x400px
- Stones: Scale with board

## Future Enhancements

### 1. **Sound Design**
- Stone placement clicks
- Capture sounds
- Ambient game music
- Win/loss themes

### 2. **Visual Effects**
- Ko fight indicators
- Atari warnings
- Connection strength lines
- Heat map overlays

### 3. **Advanced UI**
- Move tree viewer
- Game analysis panel
- Pattern library
- Statistics dashboard

### 4. **Customization**
- Theme selection
- Board textures
- Stone styles
- Color schemes

## Accessibility

### Visual
- High contrast mode
- Colorblind-safe palettes
- Adjustable stone markers
- Clear territory indicators

### Interaction
- Keyboard navigation
- Touch-friendly targets
- Undo/redo support
- Move confirmation

## Performance Targets

- 60 FPS rendering
- <16ms frame time
- <100ms move validation
- <1MB memory per game

## Testing Checklist

- [ ] Board renders as perfect square
- [ ] Stones have 9-layer gradients
- [ ] Golden ratio highlights visible
- [ ] Territory marking works smoothly
- [ ] Guild classification displays correctly
- [ ] Score calculation is accurate
- [ ] Window resizing maintains proportions
- [ ] All buttons are responsive
- [ ] No visual glitches or artifacts
- [ ] Smooth animations throughout