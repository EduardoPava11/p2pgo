# 3D Go Design Document - Three Intersecting Planes

## Overview

A three-player variant of Go played on three orthogonal 9×9 planes that intersect at their middle rows/columns. This creates 243 unique positions (81 per plane, minus overlaps). Players use black, white, and red stones (spheres) to capture territory.

## Board Structure

### Dimensions
- **Total Positions**: 243 (not 729)
- **Three 9×9 Planes**:
  - XY plane at Z=4 (horizontal)
  - XZ plane at Y=4 (vertical front-back)
  - YZ plane at X=4 (vertical left-right)
- **Intersections**: 
  - Planes share 9 positions along each intersection line
  - Center point (4,4,4) is shared by all three planes

### Valid Positions
A position (x,y,z) is valid if:
- z=4 and x,y ∈ [0,8] (XY plane), OR
- y=4 and x,z ∈ [0,8] (XZ plane), OR  
- x=4 and y,z ∈ [0,8] (YZ plane)

## Game Mechanics

### Players
- **Black**: Traditional first player
- **White**: Second player
- **Red**: Third player
- Turn order: Black → White → Red → Black...

### Stone Placement
- Stones are spheres placed at lattice intersections
- Each position can hold only one stone
- Stones "float" in 3D space at their coordinates

### Adjacency
- Each position has up to 6 adjacent positions:
  - ±X (left/right)
  - ±Y (front/back)  
  - ±Z (up/down)
- Corner positions: 3 adjacent
- Edge positions: 4 adjacent
- Face positions: 5 adjacent
- Interior positions: 6 adjacent

### Capture Rules (Future)
- Groups need liberties in 3D (empty adjacent positions)
- Capture when a group has no liberties
- Three-player dynamics create temporary alliances

## UI Implementation

### View System
The 3D board is visualized using:

1. **Main View**: 2D projection of selected plane
   - Shows one 9×9 slice at a time
   - Grid lines in black on white background
   - Intersection lines from other planes shown faintly

2. **3D Overview**: Small isometric view
   - Shows all 9 planes stacked
   - Current plane highlighted
   - All stones visible as dots

### Navigation
- **Plane Selection**: XY, XZ, or YZ view
- **Level Selection**: Choose which of 9 levels to view
- **Example**: XY plane at Z=4 shows the middle horizontal slice

### Visual Design
- **Board**: Clean white background with black grid
- **Stones**: Simple 3-layer gradient spheres
  - Black: Dark gradient (10-30 gray)
  - White: Light gradient (250-230 gray)
  - Red: Red gradient (200-140 red channel)
- **Highlights**: Green outline for last move

## Future Enhancements

### Game Rules
1. **3D Ko Rule**: Prevent infinite capture loops in 3D
2. **Territory Calculation**: 3D regions owned by players
3. **Seki Detection**: 3D mutual life situations
4. **Handicap System**: Fair starting positions for 3 players

### Visualization
1. **Transparency Mode**: See through layers
2. **Rotation Controls**: Rotate the 3D view
3. **Slice Animation**: Animate between layers
4. **Heat Map**: Show influence in 3D

### Network Play
1. **Three-Way Relay**: Each player acts as relay for others
2. **Consensus Protocol**: Agree on game state
3. **Spectator Nodes**: Watch 3D games

### AI Integration
1. **3D Pattern Recognition**: Neural nets for 3D Go
2. **Multi-Agent Training**: Three AIs compete
3. **Strategy Discovery**: New 3D tactics

## Technical Considerations

### Performance
- Rendering 729 positions efficiently
- Occlusion culling for hidden stones
- Level-of-detail for distant stones

### Data Structure
```rust
struct Board3D {
    stones: HashMap<Coord3D, Color3D>,
    current_player: Color3D,
    move_history: Vec<(Coord3D, Color3D)>,
}
```

### Coordinate System
- Origin (0,0,0) at bottom-left-back
- X increases rightward
- Y increases forward
- Z increases upward

## Economic Integration

The 3D board can represent:
- **Multi-Layer Networks**: Each Z-level is a network layer
- **Cross-Chain Bridges**: Plane intersections as bridges
- **3D Routing**: Optimal paths through relay network
- **Resource Allocation**: Territory in 3D space

## Testing Strategy

1. **Unit Tests**: Coordinate projections, adjacency
2. **Visual Tests**: Ensure correct rendering
3. **Gameplay Tests**: Valid moves, captures
4. **Performance Tests**: Handle full board

The 3D Go implementation serves as a testbed for complex multi-agent interactions in a spatial economy.