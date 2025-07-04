# P2P Go Cleanup Tasks

## Immediate Fixes Needed

### 1. UI Library Compilation (40+ errors)
The ui-egui library has numerous compilation errors preventing new features from being built:
- Type mismatches in worker.rs
- Missing trait implementations (Debug, Clone)
- API mismatches with neural/network modules
- Moved value errors in visualization code

**Action**: Need to systematically fix each error or create a new clean UI module

### 2. Lichess-Inspired UI Not Integrated
The new UI design (p2pgo_main.rs, lichess_ui.rs) exists but can't compile due to library issues:
- Clean black/white/red color scheme ✓
- Buttons around board ✓
- SGF file selection (1-10 files) ✓
- Visual training feedback ✓
- Consistent design ✓

**Action**: Either fix library or create standalone UI binary

### 3. Code Organization Issues
- Too many markdown files (cleaned up - moved to archive/)
- Temporary debug code in /tmp (removed)
- Out-of-scope features mixed with core functionality
- Complex features implemented but not needed for launch

## Current Working State

### What Works:
- Core game logic (9x9 Go)
- Neural network (basic implementation)
- P2P networking foundation (libp2p 0.53)
- Existing UI (though not Lichess-style)

### What's Ready for Testing:
- Two-player connection via relay
- Basic game play
- Neural heat maps (toggle with H)
- SGF file loading

## Priority Order:

1. **Fix UI Compilation** - Get a clean, working UI with Lichess design
2. **Test P2P Connection** - Verify two computers can connect and play
3. **Polish UI** - Ensure consistent look and feel
4. **Create Website** - Simple GitHub Pages site with download link

## Files to Focus On:
- `/ui-egui/src/bin/p2pgo_clean.rs` - New clean UI (needs library fixes)
- `/ui-egui/src/worker.rs` - Fix compilation errors
- `/ui-egui/src/lib.rs` - Remove strict warnings
- `/neural/src/lib.rs` - Ensure clean API

## Out of Scope (Already Archived):
- 3D visualization
- Complex relay economics
- Blockchain integration
- Guild system
- Biological computing models
- All MVP planning docs