use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use p2pgo_core::{
    board::Board,
    ko_detector::{KoDetector, KoSituation, KoSequenceAnalyzer, KoTrainingSequence},
    Color, Coord, GameState, Move,
};
use p2pgo_core::sgf::SgfProcessor;
use p2pgo_neural::training::TrainingData;

/// Ko Analyzer - Extracts Ko situations from SGF files and generates mRNA CBOR files
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// SGF file to analyze
    #[arg(short, long)]
    sgf: PathBuf,

    /// Output directory for CBOR files
    #[arg(short, long, default_value = "./ko_mrna")]
    output: PathBuf,

    /// Context moves before Ko
    #[arg(long, default_value = "10")]
    context_before: usize,

    /// Context moves after Ko
    #[arg(long, default_value = "10")]
    context_after: usize,

    /// Generate full training data CBOR
    #[arg(long)]
    full_training: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

/// mRNA structure for Ko sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoMRNA {
    /// Unique ID for this mRNA
    pub id: String,
    /// Source SGF file
    pub source_sgf: String,
    /// Ko situation details
    pub ko_situation: KoSituation,
    /// Training sequence with context
    pub training_sequence: KoTrainingSequence,
    /// Feature planes for neural network
    pub feature_planes: Vec<Vec<f32>>,
    /// Policy targets (move probabilities)
    pub policy_targets: Vec<Vec<f32>>,
    /// Value targets (position evaluations)
    pub value_targets: Vec<f32>,
    /// Metadata
    pub metadata: KoMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoMetadata {
    pub black_player: String,
    pub white_player: String,
    pub black_rank: String,
    pub white_rank: String,
    pub game_result: String,
    pub ko_resolution: KoResolution,
    pub sequence_quality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KoResolution {
    Captured,
    Recaptured,
    Avoided,
    GameEnded,
}

/// Full training data CBOR structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTrainingCBOR {
    /// All positions from the game
    pub positions: Vec<TrainingPosition>,
    /// Game metadata
    pub metadata: GameMetadata,
    /// Ko sequences found
    pub ko_sequences: Vec<KoMRNA>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPosition {
    pub move_number: usize,
    pub features: Vec<f32>,
    pub policy_target: Vec<f32>,
    pub value_target: f32,
    pub is_ko_related: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub source_file: String,
    pub total_moves: usize,
    pub board_size: u8,
    pub komi: f32,
    pub result: String,
    pub ko_count: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Create output directory
    fs::create_dir_all(&args.output)?;

    // Load and parse SGF
    log::info!("Loading SGF file: {:?}", args.sgf);
    let sgf_content = fs::read_to_string(&args.sgf)?;

    // Parse SGF to game state
    let initial_state = GameState::new(9); // Will be updated from SGF
    let mut sgf_processor = SgfProcessor::new(initial_state);
    let game_state_from_sgf = sgf_processor.parse(&sgf_content)?;

    // Extract metadata from SGF
    let metadata = extract_metadata_from_sgf(&sgf_content)?;

    // Play through the game detecting Ko situations
    let mut board = Board::new(metadata.board_size);
    let mut game_state = GameState::new(metadata.board_size);
    let mut ko_detector = KoDetector::new();
    let mut all_moves = Vec::new();
    let mut feature_extractor = FeatureExtractor::new();

    log::info!("Analyzing game with {} moves...", game.main_variation().count());

    for (move_num, node) in game.main_variation().enumerate() {
        if let Some((color, coord)) = node.get_move() {
            let m = Move::Place { x: coord.x, y: coord.y, color };
            all_moves.push(m.clone());

            // Apply move and detect captures
            let events = game_state.apply_move(m.clone())?;
            let captured_coords = extract_captures(&events);

            // Check for Ko
            if let Some(ko_situation) = ko_detector.process_move(&board, &m, &captured_coords) {
                log::info!("Ko detected at move {} at {:?}", move_num, ko_situation.ko_point);
            }

            // Update board
            board.place(coord, color);
            for cap in &captured_coords {
                board.remove(*cap);
            }
        }
    }

    // Extract Ko sequences with context
    let analyzer = KoSequenceAnalyzer::new(args.context_before, args.context_after);
    let ko_sequences = analyzer.extract_ko_sequences(ko_detector.get_ko_situations(), &all_moves);

    log::info!("Found {} Ko situations", ko_sequences.len());

    // Generate mRNA CBOR files for each Ko sequence
    let mut all_mrna = Vec::new();

    for (idx, ko_seq) in ko_sequences.iter().enumerate() {
        log::info!("Processing Ko sequence {} (moves {}-{})",
            idx, ko_seq.start_move_num, ko_seq.end_move_num);

        // Generate training data for this sequence
        let mrna = generate_ko_mrna(
            &args.sgf,
            ko_seq,
            &metadata,
            &game_state,
            &feature_extractor,
            idx,
        )?;

        // Write individual mRNA CBOR file
        let mrna_path = args.output.join(format!("ko_mrna_{}.cbor", mrna.id));
        let mrna_data = serde_cbor::to_vec(&mrna)?;
        fs::write(&mrna_path, mrna_data)?;
        log::info!("Wrote mRNA to {:?} ({} bytes)", mrna_path, mrna_data.len());

        all_mrna.push(mrna);
    }

    // Generate full training data if requested
    if args.full_training {
        log::info!("Generating full training data CBOR...");

        let full_training = generate_full_training_cbor(
            &args.sgf,
            &game,
            &metadata,
            &all_mrna,
            &feature_extractor,
        )?;

        let full_path = args.output.join("full_training.cbor");
        let full_data = serde_cbor::to_vec(&full_training)?;
        fs::write(&full_path, full_data)?;
        log::info!("Wrote full training data to {:?} ({} bytes)", full_path, full_data.len());
    }

    // Summary
    log::info!("\nAnalysis complete:");
    log::info!("  Total moves: {}", all_moves.len());
    log::info!("  Ko situations: {}", ko_sequences.len());
    log::info!("  mRNA files generated: {}", all_mrna.len());

    if ko_sequences.is_empty() {
        log::warn!("No Ko situations found in this game");
    } else {
        for (idx, ko) in ko_sequences.iter().enumerate() {
            log::info!("  Ko {}: moves {}-{}, point {:?}",
                idx,
                ko.ko_situation.start_move,
                ko.ko_situation.recapture_move.unwrap_or(ko.ko_situation.capture_move),
                ko.ko_situation.ko_point
            );
        }
    }

    Ok(())
}

fn extract_metadata(root: &p2pgo_sgf::Node) -> GameMetadata {
    GameMetadata {
        source_file: String::new(), // Will be set later
        total_moves: 0, // Will be counted
        board_size: root.get_property("SZ")
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(19),
        komi: root.get_property("KM")
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
            .unwrap_or(6.5),
        result: root.get_property("RE")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_default(),
        ko_count: 0, // Will be set later
    }
}

fn extract_captures(events: &[p2pgo_core::GameEvent]) -> Vec<Coord> {
    events.iter().filter_map(|event| {
        match event {
            p2pgo_core::GameEvent::StonesCaptured { positions, .. } => Some(positions.clone()),
            _ => None,
        }
    }).flatten().collect()
}

fn generate_ko_mrna(
    sgf_path: &Path,
    ko_seq: &KoTrainingSequence,
    game_metadata: &GameMetadata,
    game_state: &GameState,
    feature_extractor: &FeatureExtractor,
    idx: usize,
) -> Result<KoMRNA> {
    // Generate features for each position in the sequence
    let mut feature_planes = Vec::new();
    let mut policy_targets = Vec::new();
    let mut value_targets = Vec::new();

    // Replay the sequence to generate features
    let mut temp_board = Board::new(game_metadata.board_size);

    for (i, context_move) in ko_seq.sequence_moves.iter().enumerate() {
        // Extract features for current position
        let features = feature_extractor.extract_features(&temp_board, context_move.color);
        feature_planes.push(features);

        // Generate policy target (next move)
        let mut policy = vec![0.0; (game_metadata.board_size * game_metadata.board_size) as usize];
        if let Some(coord) = context_move.coord {
            let idx = coord.y as usize * game_metadata.board_size as usize + coord.x as usize;
            policy[idx] = 1.0;
        }
        policy_targets.push(policy);

        // Generate value target (simplified - based on game result)
        let value = calculate_value_target(&game_metadata.result, context_move.color);
        value_targets.push(value);

        // Apply move to temp board
        if let Some(coord) = context_move.coord {
            temp_board.place(coord, context_move.color);
        }
    }

    // Determine Ko resolution
    let resolution = if ko_seq.ko_situation.recapture_move.is_some() {
        KoResolution::Recaptured
    } else if ko_seq.end_move_num >= ko_seq.ko_situation.capture_move + 5 {
        KoResolution::Avoided
    } else {
        KoResolution::Captured
    };

    // Calculate sequence quality
    let quality = calculate_sequence_quality(ko_seq, game_metadata);

    Ok(KoMRNA {
        id: format!("{}_ko_{}", sgf_path.file_stem().unwrap().to_string_lossy(), idx),
        source_sgf: sgf_path.to_string_lossy().to_string(),
        ko_situation: ko_seq.ko_situation.clone(),
        training_sequence: ko_seq.clone(),
        feature_planes,
        policy_targets,
        value_targets,
        metadata: KoMetadata {
            black_player: String::new(), // Would extract from SGF
            white_player: String::new(),
            black_rank: String::new(),
            white_rank: String::new(),
            game_result: game_metadata.result.clone(),
            ko_resolution: resolution,
            sequence_quality: quality,
        },
    })
}

fn calculate_value_target(result: &str, color: Color) -> f32 {
    if result.starts_with("B+") {
        if color == Color::Black { 1.0 } else { -1.0 }
    } else if result.starts_with("W+") {
        if color == Color::White { 1.0 } else { -1.0 }
    } else {
        0.0
    }
}

fn calculate_sequence_quality(ko_seq: &KoTrainingSequence, metadata: &GameMetadata) -> f32 {
    let mut quality = 0.5;

    // Longer sequences are more valuable
    let seq_length = ko_seq.end_move_num - ko_seq.start_move_num;
    quality += (seq_length as f32 / 20.0).min(0.3);

    // Recaptured Ko are more interesting
    if ko_seq.ko_situation.recapture_move.is_some() {
        quality += 0.2;
    }

    quality.min(1.0)
}

fn generate_full_training_cbor(
    sgf_path: &Path,
    game: &p2pgo_sgf::Game,
    metadata: &GameMetadata,
    ko_mrnas: &[KoMRNA],
    feature_extractor: &FeatureExtractor,
) -> Result<FullTrainingCBOR> {
    let mut positions = Vec::new();
    let mut board = Board::new(metadata.board_size);

    // Mark Ko-related positions
    let ko_moves: std::collections::HashSet<usize> = ko_mrnas.iter()
        .flat_map(|mrna| mrna.training_sequence.start_move_num..mrna.training_sequence.end_move_num)
        .collect();

    for (move_num, node) in game.main_variation().enumerate() {
        if let Some((color, coord)) = node.get_move() {
            // Extract features before move
            let features = feature_extractor.extract_features(&board, color);

            // Policy target (this move)
            let mut policy = vec![0.0; (metadata.board_size * metadata.board_size) as usize];
            let idx = coord.y as usize * metadata.board_size as usize + coord.x as usize;
            policy[idx] = 1.0;

            // Value target
            let value = calculate_value_target(&metadata.result, color);

            positions.push(TrainingPosition {
                move_number: move_num,
                features,
                policy_target: policy,
                value_target: value,
                is_ko_related: ko_moves.contains(&move_num),
            });

            // Apply move
            board.place(coord, color);
        }
    }

    let mut full_metadata = metadata.clone();
    full_metadata.source_file = sgf_path.to_string_lossy().to_string();
    full_metadata.total_moves = positions.len();
    full_metadata.ko_count = ko_mrnas.len();

    Ok(FullTrainingCBOR {
        positions,
        metadata: full_metadata,
        ko_sequences: ko_mrnas.to_vec(),
    })
}

// Feature extraction implementation
impl FeatureExtractor {
    fn new() -> Self {
        Self {}
    }

    fn extract_features(&self, board: &Board, next_player: Color) -> Vec<f32> {
        let size = board.size() as usize;
        let mut features = vec![0.0; 8 * size * size];

        // Feature planes:
        // 0: Black stones
        // 1: White stones
        // 2: Empty points
        // 3: Black liberties (1-4)
        // 4: White liberties (1-4)
        // 5: Black to play
        // 6: White to play
        // 7: Ko points (if any)

        for y in 0..board.size() {
            for x in 0..board.size() {
                let coord = Coord::new(x, y);
                let idx = y as usize * size + x as usize;

                match board.get(coord) {
                    Some(Color::Black) => {
                        features[idx] = 1.0;
                        // Add liberty count
                        let liberties = count_liberties(board, coord);
                        features[3 * size * size + idx] = (liberties as f32 / 4.0).min(1.0);
                    }
                    Some(Color::White) => {
                        features[size * size + idx] = 1.0;
                        // Add liberty count
                        let liberties = count_liberties(board, coord);
                        features[4 * size * size + idx] = (liberties as f32 / 4.0).min(1.0);
                    }
                    None => {
                        features[2 * size * size + idx] = 1.0;
                    }
                }
            }
        }

        // Next player planes
        let player_plane = if next_player == Color::Black { 5 } else { 6 };
        for i in 0..size * size {
            features[player_plane * size * size + i] = 1.0;
        }

        features
    }
}

fn count_liberties(board: &Board, coord: Coord) -> usize {
    coord.adjacent_coords().iter()
        .filter(|&&adj| adj.is_valid(board.size()) && board.get(adj).is_none())
        .count()
}