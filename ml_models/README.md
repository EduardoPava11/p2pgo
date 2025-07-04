# ML Models for P2PGo

This workspace contains CPU-only machine learning models for Go policy hints.

## Models

### Sword Net (`sword_net/`)
- **Purpose**: Aggressive play policy
- **Strategy**: Prioritizes corners, edges, and attacking moves
- **Model ID**: 1001
- **Characteristics**: 
  - High probability for corner moves (territorial expansion)
  - Bonus for moves adjacent to opponent stones (attacking)
  - Natural board orientation

### Shield Net (`shield_net/`)
- **Purpose**: Defensive play policy  
- **Strategy**: Prioritizes center control and defensive positioning
- **Model ID**: 1002
- **Characteristics**:
  - High probability for center moves (defensive control)
  - Bonus for moves adjacent to opponent stones (blocking)
  - Y-flipped board orientation (AlphaGo-Zero symmetry trick)
  - Protection bonus for threatened friendly stones

## Architecture

Both models use lightweight CPU-only implementations with:
- 9×9 board support (81 positions)
- 3-channel input (black stones, white stones, empty)
- WASM-compatible C interface
- Deterministic inference with seeded RNG
- Probability normalization

## Building

### Native Tests
```bash
cargo test
```

### WebAssembly
```bash
./build_wasm.sh
```

This builds both models for `wasm32-wasi` target and copies them to the Android assets directory.

## WASM Interface

Each model exports the following functions:

- `{model}_infer(board_ptr, board_len, out_ptr) -> i32`
  - Input: 9×9×3 float array (243 floats)
  - Output: 81-float probability array
  - Returns: 0 for success, negative for errors

- `{model}_get_model_id() -> u32`
  - Returns unique model identifier

- `{model}_get_version() -> u32` 
  - Returns model version (100 = v1.0.0)

- `{model}_train_step(...)` 
  - Placeholder for federated learning

## Integration

The models integrate with the main P2P adapter through:
1. `burn_host.rs` - Wasmtime host for loading WASM modules
2. Lazy loading on first hint toggle
3. Thread-safe inference calls
4. Heat-map generation for UI overlay

## File Sizes

Target WASM module sizes: < 400 KiB each (optimized with `opt-level = "s"` and LTO)