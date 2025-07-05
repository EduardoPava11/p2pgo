# P2P Go V1 Manual Testing Checklist

## Pre-Release Testing Protocol

### 🚀 Setup
- [ ] Build release binary: `cargo build --release`
- [ ] Create 2-3 test machines/VMs (or use different user accounts)
- [ ] Ensure machines are on same network but can also test across internet

### 🔌 Basic Connectivity Tests

#### Test 1: Direct Connection
1. [ ] Start p2pgo on Machine A
2. [ ] Start p2pgo on Machine B  
3. [ ] Generate ticket on Machine A
4. [ ] Connect from Machine B using ticket
5. [ ] Verify connection established
6. [ ] Check network panel shows "Healthy" status

#### Test 2: Relay Modes
Test each mode thoroughly:

**Disabled Mode:**
- [ ] Set relay mode to Disabled on both machines
- [ ] Verify direct connections still work on LAN
- [ ] Verify failure when behind NAT (use mobile hotspot)

**Minimal Mode:**
- [ ] Set to Minimal mode
- [ ] Verify relay only activates when direct connection fails
- [ ] Check network panel shows correct status

**Normal Mode:**
- [ ] Set to Normal mode
- [ ] Verify can relay for others
- [ ] Check relay statistics update

**Provider Mode:**
- [ ] Set to Provider mode
- [ ] Enable training consent
- [ ] Verify credits accumulate
- [ ] Check bandwidth usage is reasonable

### 🎮 Game Flow Tests

#### Test 3: Complete Game Lifecycle
1. [ ] Create game on Machine A (9x9 board)
2. [ ] Join from Machine B
3. [ ] Make 10-15 moves alternately
4. [ ] Verify moves sync instantly
5. [ ] Pass twice to end game
6. [ ] Both players mark dead stones
7. [ ] Accept score on both sides
8. [ ] Verify CBOR file created in:
   - macOS: `~/Library/Application Support/p2pgo/finished/`
   - Linux: `./finished_games/`

#### Test 4: Game Persistence
1. [ ] Start a game
2. [ ] Make 5 moves
3. [ ] Kill p2pgo on Machine B (Ctrl+C)
4. [ ] Restart p2pgo on Machine B
5. [ ] Rejoin game
6. [ ] Verify game state restored correctly

### 🧠 Neural Network Tests

#### Test 5: Heat Maps
1. [ ] During game, press 'H' to toggle heat map
2. [ ] Verify heat map shows move suggestions
3. [ ] Press 'D' for dual heat map
4. [ ] Verify orthogonal colors (red-blue)
5. [ ] Check interference patterns (purple)

#### Test 6: Training Data
1. [ ] Complete 5+ games
2. [ ] Enable ghost moves (should appear after 5 games)
3. [ ] Verify training consent UI in Provider mode
4. [ ] Check CBOR archives are created
5. [ ] Verify files are compressed if >1MB

### 🌐 Network Resilience Tests

#### Test 7: Connection Recovery
1. [ ] Start game between machines
2. [ ] Disconnect network on Machine B (airplane mode)
3. [ ] Wait 30 seconds
4. [ ] Reconnect network
5. [ ] Verify game resumes
6. [ ] Check no moves were lost

#### Test 8: Relay Failover
1. [ ] Connect through relay
2. [ ] Monitor relay health in network panel
3. [ ] Simulate relay failure (firewall block)
4. [ ] Verify fallback to another relay
5. [ ] Check game continues uninterrupted

### 🔒 Security & Privacy Tests

#### Test 9: Privacy Modes
1. [ ] Test Minimal mode doesn't leak data
2. [ ] Verify Provider mode only shares when consent given
3. [ ] Check no unexpected network connections
4. [ ] Verify ticket expiration works

### 📊 Performance Tests

#### Test 10: Stress Testing
1. [ ] Create multiple games simultaneously
2. [ ] Fast gameplay (1 move per second)
3. [ ] Large board (19x19) with 200+ moves
4. [ ] Monitor CPU and memory usage
5. [ ] Check no memory leaks over time

### 🐛 Edge Cases

#### Test 11: Error Handling
- [ ] Invalid ticket connection attempt
- [ ] Network timeout during game
- [ ] Score disagreement scenario
- [ ] CBOR export with full disk
- [ ] Relay service with no bandwidth

### 📱 Platform-Specific Tests

#### macOS:
- [ ] Verify .app bundle works
- [ ] Check code signing (if applicable)
- [ ] Test with Gatekeeper enabled
- [ ] Verify data in correct directories

#### Linux:
- [ ] Test on Ubuntu 22.04
- [ ] Test on Fedora latest
- [ ] Verify AppImage works (if created)

#### Windows:
- [ ] Test on Windows 10/11
- [ ] Check firewall prompts
- [ ] Verify data directories

## 🎯 Acceptance Criteria

### Must Pass:
- Basic connectivity between 2 nodes
- Complete game from start to CBOR export  
- Relay modes switch correctly
- Move synchronization is reliable
- Score consensus works

### Should Pass:
- Heat maps display correctly
- Network recovery works
- Performance is acceptable
- No critical errors in logs

### Nice to Have:
- All relay modes tested under NAT
- Stress tests pass without issues
- Training credits accumulate correctly

## 📝 Test Report Template

```
Date: _________
Version: _______
Tester: ________

Environment:
- OS: _________
- Network: _____

Results:
- [ ] All Must Pass criteria: PASS/FAIL
- [ ] Should Pass criteria: ___/___
- [ ] Nice to Have: ___/___

Issues Found:
1. _____________
2. _____________

Recommendation: RELEASE / FIX ISSUES
```