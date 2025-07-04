# SGF to CBOR Converter for Neural Network Training

This module provides functionality to convert SGF (Smart Game Format) Go game files into CBOR (Concise Binary Object Representation) format optimized for neural network training.

## Features

- **Game Replay**: Replays SGF games move by move to extract board positions
- **Feature Extraction**: Generates 8 feature planes for each position:
  - Black stones
  - White stones
  - Empty points
  - Black liberties
  - White liberties
  - Black to play
  - White to play
  - Ko points
- **Training Labels**: Extracts both policy (move played) and value (game outcome) labels
- **Batch Processing**: Convert multiple SGF files into training batches
- **Filtering Options**: Control which positions to include (opening, endgame, minimum move number)
- **Quality Scoring**: Automatically scores games based on player ranks

## Usage

### Basic Conversion

```rust
use p2pgo_neural::training::SgfToCborConverter;
use std::path::Path;

// Create converter for 9x9 board
let converter = SgfToCborConverter::new(9);

// Convert single SGF file
converter.convert_file(
    Path::new("game.sgf"),
    Path::new("game.cbor")
)?;
```

### Batch Conversion

```rust
// Convert multiple SGF files to a single batch
let sgf_files = vec![
    Path::new("game1.sgf"),
    Path::new("game2.sgf"),
    Path::new("game3.sgf"),
];

converter.convert_batch(
    &sgf_files.iter().map(|p| p.as_ref()).collect::<Vec<_>>(),
    Path::new("batch.cbor")
)?;
```

### Advanced Filtering

```rust
// Create converter with custom filters
let converter = SgfToCborConverter::new(9)
    .with_opening(false)    // Skip opening positions
    .with_endgame(true)     // Include endgame positions
    .with_min_move(10);     // Start from move 10
```

### Directory Processing

```rust
use p2pgo_neural::training::batch_process_directory;

// Process entire directory of SGF files
batch_process_directory(
    Path::new("sgf_games/"),
    Path::new("cbor_batches/"),
    100  // Games per batch
).await?;
```

## CBOR Format

The converter produces CBOR files with the following structure:

```rust
CBORTrainingBatch {
    batch_id: String,              // Unique identifier
    source: TrainingSource {       // Game metadata
        game_id: String,
        black_player: String,
        white_player: String,
        black_rank: String,
        white_rank: String,
        result: String,
    },
    examples: Vec<TrainingExample>, // Training positions
    metadata: BatchMetadata {       // Batch information
        created_at: u64,
        example_count: usize,
        quality_score: f32,
        compressed: bool,
        data_hash: String,
    },
}
```

Each `TrainingExample` contains:
- `move_number`: Position in the game
- `features`: 8 feature planes representing the board state
- `policy_target`: The move that was played (sparse representation)
- `value_target`: Game outcome from current player's perspective (-1 to 1)
- `context`: Additional information (ko, opening, endgame)

## Integration with Neural Network Training

The CBOR format is designed to be efficiently loaded during training:

```rust
use p2pgo_neural::cbor_format::CBORDataLoader;

let loader = CBORDataLoader::new();
let batch = loader.load_batch(Path::new("batch.cbor"))?;

// Process training examples
for example in &batch.examples {
    // Use example.features for network input
    // Use example.policy_target for move prediction training
    // Use example.value_target for position evaluation training
}
```

## Performance Considerations

- Feature planes are stored as flattened vectors for efficient memory layout
- Policy targets use sparse representation to save space
- Batch processing reduces file I/O overhead
- Quality scoring helps prioritize high-quality games for training

## Future Enhancements

- Compression support for smaller file sizes
- Parallel processing for faster conversion
- Additional feature planes (capture information, territory estimates)
- Support for different board sizes (19x19, 13x13)