use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use p2pgo_core::{
    board::Board,
    ko_detector::{KoDetector, KoSituation, KoSequenceAnalyzer, KoTrainingSequence, ContextMove},
    sgf::SgfProcessor,
    Color, Coord, GameState, Move, GameEvent,
};

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
    /// Feature planes for neural network (8 planes x board_size x board_size)
    pub feature_planes: Vec<Vec<Vec<f32>>>,
    /// Metadata
    pub metadata: KoMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoMetadata {
    pub board_size: u8,
    pub game_result: String,
    pub ko_resolution: KoResolution,
    pub sequence_quality: f32,
    pub total_moves: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KoResolution {
    Captured,
    Recaptured,
    Avoided,
    GameEnded,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(if args.verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info })
        .init();

    // Create output directory
    fs::create_dir_all(&args.output)?;

    // Load SGF
    log::info!("Loading SGF file: {:?}", args.sgf);
    let sgf_content = fs::read_to_string(&args.sgf)?;

    // Parse SGF to get initial game state
    let initial_state = GameState::new(19); // Default size, will be updated
    let mut sgf_processor = SgfProcessor::new(initial_state);
    let parsed_state = sgf_processor.parse(&sgf_content)?;

    // Extract board size from parsed state
    let board_size = parsed_state.board_size;

    // Replay the game to detect Ko situations
    let mut board = Board::new(board_size);
    let mut game_state = GameState::new(board_size);
    let mut ko_detector = KoDetector::new();
    let mut all_moves = Vec::new();

    log::info!("Analyzing game with board size {} and {} moves", board_size, parsed_state.moves.len());

    // Process each move
    for (move_num, mv) in parsed_state.moves.iter().enumerate() {
        all_moves.push(mv.clone());

        // Apply move to game state
        let events = game_state.apply_move(mv.clone())?;

        // Extract captured stones from events
        let captured_coords = extract_captures(&events);

        // Check for Ko
        if let Some(ko_situation) = ko_detector.process_move(&board, mv, &captured_coords) {
            log::info!("Ko detected at move {} at {:?}", move_num, ko_situation.ko_point);
        }

        // Update board state
        match mv {
            Move::Place { x, y, color } => {
                let coord = Coord::new(*x, *y);
                board.place(coord, *color);

                // Remove captured stones
                for cap_coord in &captured_coords {
                    board.remove(*cap_coord);
                }
            }
            _ => {} // Pass or Resign
        }
    }

    // Extract Ko sequences with context
    let analyzer = KoSequenceAnalyzer::new(args.context_before, args.context_after);
    let ko_sequences = analyzer.extract_ko_sequences(ko_detector.get_ko_situations(), &all_moves);

    log::info!("Found {} Ko situations", ko_sequences.len());

    if ko_sequences.is_empty() {
        log::warn!("No Ko situations found in this game");
        log::info!("Consider analyzing more games or adjusting detection parameters");
        return Ok(());
    }

    // Generate mRNA CBOR files for each Ko sequence
    for (idx, ko_seq) in ko_sequences.iter().enumerate() {
        log::info!("Processing Ko sequence {} (moves {}-{})",
            idx, ko_seq.start_move_num, ko_seq.end_move_num);

        // Generate feature planes for the sequence
        let feature_planes = generate_feature_planes(&ko_seq, board_size);

        // Determine Ko resolution
        let resolution = if ko_seq.ko_situation.recapture_move.is_some() {
            KoResolution::Recaptured
        } else if ko_seq.end_move_num >= ko_seq.ko_situation.capture_move + 5 {
            KoResolution::Avoided
        } else {
            KoResolution::Captured
        };

        // Calculate sequence quality
        let quality = calculate_sequence_quality(&ko_seq);

        // Create mRNA
        let mrna = KoMRNA {
            id: format!("{}_ko_{}",
                args.sgf.file_stem().unwrap().to_string_lossy(),
                idx
            ),
            source_sgf: args.sgf.to_string_lossy().to_string(),
            ko_situation: ko_seq.ko_situation.clone(),
            training_sequence: ko_seq.clone(),
            feature_planes,
            metadata: KoMetadata {
                board_size,
                game_result: extract_result(&parsed_state),
                ko_resolution: resolution,
                sequence_quality: quality,
                total_moves: all_moves.len(),
            },
        };

        // Write CBOR file
        let mrna_path = args.output.join(format!("{}.cbor", mrna.id));
        let mrna_data = serde_cbor::to_vec(&mrna)?;
        fs::write(&mrna_path, &mrna_data)?;

        log::info!("Wrote mRNA to {:?} ({} bytes)", mrna_path, mrna_data.len());
    }

    // Summary
    log::info!("\nAnalysis complete:");
    log::info!("  Total moves: {}", all_moves.len());
    log::info!("  Ko situations: {}", ko_sequences.len());
    log::info!("  Output directory: {:?}", args.output);

    Ok(())
}

fn extract_captures(events: &[GameEvent]) -> Vec<Coord> {
    events.iter().filter_map(|event| {
        match event {
            GameEvent::StonesCaptured { positions, .. } => Some(positions.clone()),
            _ => None,
        }
    }).flatten().collect()
}

fn generate_feature_planes(ko_seq: &KoTrainingSequence, board_size: u8) -> Vec<Vec<Vec<f32>>> {
    let mut feature_planes = Vec::new();
    let size = board_size as usize;

    // For each move in the sequence, generate 8 feature planes
    for context_move in &ko_seq.sequence_moves {
        let mut planes = vec![vec![vec![0.0; size]; size]; 8];

        // Simplified feature extraction
        // Plane 0: Black stones
        // Plane 1: White stones
        // Plane 2: Empty points
        // Plane 3: Ko point
        // Plane 4: Last move
        // Plane 5: Black to play
        // Plane 6: White to play
        // Plane 7: Move number / 100

        // Mark Ko point
        let ko_x = ko_seq.ko_situation.ko_point.x as usize;
        let ko_y = ko_seq.ko_situation.ko_point.y as usize;
        planes[3][ko_y][ko_x] = 1.0;

        // Mark last move if available
        if let Some(coord) = context_move.coord {
            planes[4][coord.y as usize][coord.x as usize] = 1.0;
        }

        // Turn to play
        if context_move.color == Color::Black {
            for y in 0..size {
                for x in 0..size {
                    planes[5][y][x] = 1.0;
                }
            }
        } else {
            for y in 0..size {
                for x in 0..size {
                    planes[6][y][x] = 1.0;
                }
            }
        }

        feature_planes.push(planes);
    }

    feature_planes
}

fn calculate_sequence_quality(ko_seq: &KoTrainingSequence) -> f32 {
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

fn extract_result(game_state: &GameState) -> String {
    let (black_score, white_score) = game_state.calculate_score();
    if black_score > white_score {
        format!("B+{:.1}", black_score - white_score)
    } else {
        format!("W+{:.1}", white_score - black_score)
    }
}