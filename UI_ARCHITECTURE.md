# P2P Go UI Architecture & Design System

## Design Philosophy
Inspired by OGS (Online Go Server) with a focus on clarity, consistency, and always-visible neural network insights.

## Visual Design System

### Color Palette
```rust
// Primary Colors (Low saturation for readability)
const BOARD_COLOR: Color32 = Color32::from_rgb(220, 179, 92);  // Traditional Kaya wood
const BLACK_STONE: Color32 = Color32::from_gray(15);           // Near black
const WHITE_STONE: Color32 = Color32::from_gray(245);          // Off white

// UI Colors (Muted for better readability)
const BACKGROUND: Color32 = Color32::from_gray(28);            // Dark gray
const SURFACE: Color32 = Color32::from_gray(38);               // Slightly lighter
const PRIMARY: Color32 = Color32::from_rgb(67, 160, 71);       // Muted green
const SECONDARY: Color32 = Color32::from_rgb(66, 115, 179);    // Muted blue
const ACCENT: Color32 = Color32::from_rgb(179, 67, 67);        // Muted red
const TEXT_PRIMARY: Color32 = Color32::from_gray(230);         // Light gray
const TEXT_SECONDARY: Color32 = Color32::from_gray(180);       // Medium gray
```

### Typography
```rust
// Font Hierarchy
const FONT_TITLE: f32 = 24.0;      // App title
const FONT_HEADING: f32 = 18.0;    // Section headers
const FONT_BODY: f32 = 14.0;       // Normal text
const FONT_SMALL: f32 = 12.0;      // Labels, hints
const FONT_MONO: f32 = 13.0;       // Game codes, coordinates
```

### Spacing & Layout
```rust
const SPACING_UNIT: f32 = 8.0;     // Base spacing unit
const PADDING_SM: f32 = 8.0;       // 1x unit
const PADDING_MD: f32 = 16.0;      // 2x unit
const PADDING_LG: f32 = 24.0;      // 3x unit
const BORDER_RADIUS: f32 = 4.0;    // Consistent rounding
```

## UI Layer Architecture

### Layer 1: Core Components (Lowest Level)
```
core/
├── theme.rs           // Design tokens, colors, fonts
├── components/
│   ├── button.rs      // Styled button component
│   ├── card.rs        // Container with surface color
│   ├── input.rs       // Text input with consistent styling
│   └── icons.rs       // Icon system (using Unicode symbols)
```

### Layer 2: Domain Widgets
```
widgets/
├── board_widget.rs    // Go board rendering
├── stone_widget.rs    // Stone with shadow/gradient
├── coord_widget.rs    // Board coordinates (A-J, 1-9)
├── neural_viz.rs      // Always-visible neural network
└── heat_map.rs        // Move probability overlay
```

### Layer 3: Feature Modules
```
features/
├── game/
│   ├── game_view.rs   // Active game UI
│   ├── controls.rs    // Pass, Resign, Undo buttons
│   └── info_panel.rs  // Captures, time, move count
├── lobby/
│   ├── lobby_view.rs  // Game list, quick match
│   ├── game_card.rs   // Individual game listing
│   └── filters.rs     // Board size, time control
├── training/
│   ├── sgf_import.rs  // File selection & preview
│   ├── progress.rs    // Training visualization
│   └── history.rs     // Training history graph
└── consensus/
    ├── territory.rs   // Territory marking UI
    ├── agreement.rs   // Consensus status
    └── review.rs      // Post-game analysis
```

### Layer 4: Application Shell
```
app/
├── app.rs             // Main application state
├── router.rs          // View routing
├── layout.rs          // App-wide layout (header, sidebar)
└── networking.rs      // P2P connection management
```

## UI State Flow

### Main Menu → Game Flow
```
1. Launch App
   ├─ Show connection status (top bar)
   ├─ Display neural network status (sidebar)
   └─ Present game options (center)
   
2. Create/Join Game
   ├─ Generate/Enter game code
   ├─ Show "Waiting for opponent" with shareable link
   └─ Neural net continues training in background
   
3. Active Game
   ├─ Board (center)
   ├─ Neural suggestions (always visible overlay)
   ├─ Game info (right panel)
   └─ Chat/Notes (bottom panel)
   
4. Game End → Consensus
   ├─ Territory marking phase
   ├─ Show both players' markings
   ├─ Highlight disagreements
   └─ Confirm consensus → Training data
```

## Component Examples

### Styled Button
```rust
pub fn styled_button(text: &str, style: ButtonStyle) -> impl Widget {
    Button::new(text)
        .fill(match style {
            ButtonStyle::Primary => PRIMARY,
            ButtonStyle::Secondary => SECONDARY,
            ButtonStyle::Danger => ACCENT,
        })
        .min_size(Vec2::new(120.0, 40.0))
        .rounding(BORDER_RADIUS)
}
```

### Neural Network Panel (Always Visible)
```rust
pub struct NeuralPanel {
    position: PanelPosition,  // Left, Right, Float
    opacity: f32,             // 0.3-1.0 for transparency
    show_details: bool,       // Collapsed/Expanded
}

impl NeuralPanel {
    pub fn render(&self, ui: &mut Ui, game_state: &GameState) {
        Frame::none()
            .fill(SURFACE.linear_multiply(self.opacity))
            .show(ui, |ui| {
                ui.heading("Neural Analysis");
                
                // Always show top 3 moves
                for (i, move) in self.top_moves.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", i + 1));
                        ui.label(move.coord.to_string());
                        ui.label(format!("{:.1}%", move.probability * 100.0));
                    });
                }
                
                // Win probability bar
                self.render_win_probability(ui);
                
                // Expandable details
                if self.show_details {
                    self.render_network_visualization(ui);
                }
            });
    }
}
```

## Implementation Steps

### Phase 1: Design System (Week 1)
1. Create `ui-v2/` with clean architecture
2. Implement core components with design tokens
3. Build style guide demo

### Phase 2: Core Features (Week 2)
1. Game board with proper OGS-style rendering
2. Always-visible neural panel
3. P2P connection status
4. Basic game flow

### Phase 3: Advanced Features (Week 3)
1. SGF import with visualization
2. Consensus UI for territory
3. Training progress monitoring
4. Network replay system

### Phase 4: Polish (Week 4)
1. Animations and transitions
2. Sound effects
3. Keyboard shortcuts
4. Accessibility

## Key Improvements Over Current UI

1. **Consistent Design Language**: Every component uses the same colors, spacing, and typography
2. **Always-Visible Neural Network**: Floating panel shows AI insights at all times
3. **Clear Visual Hierarchy**: Important actions are prominent, secondary features are subtle
4. **OGS-Inspired Layout**: Familiar to Go players, proven UX patterns
5. **Proper P2P Status**: Always know connection state and relay health
6. **Integrated Training**: SGF import → visualization → training is one smooth flow

This architecture separates concerns properly and makes it clear where each piece of functionality belongs.