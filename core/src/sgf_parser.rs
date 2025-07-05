use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ko_detector::{KoDetector, KoSituation};
use crate::sgf::SgfProcessor;
use crate::{Coord, GameState, Move};

/// SGF Parser for compatibility
pub struct SGFParser;

impl SGFParser {
    pub fn parse_file(path: &Path) -> Result<ParsedGame> {
        parse_sgf_with_ko_detection(path)
    }

    pub fn parse_string(content: &str) -> Result<ParsedGame> {
        parse_sgf_content_with_ko_detection(content, "<string>".to_string())
    }
}

/// High-level SGF parsing with Ko detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedGame {
    /// Unique game ID
    pub id: String,
    /// Source file path
    pub source_path: String,
    /// Game metadata
    pub metadata: GameMetadata,
    /// All moves in the game
    pub moves: Vec<Move>,
    /// Ko situations found
    pub ko_situations: Vec<KoSituation>,
    /// Game state after parsing
    pub final_state: GameState,
    /// Parsing warnings
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub black_player: String,
    pub white_player: String,
    pub black_rank: String,
    pub white_rank: String,
    pub result: String,
    pub date: String,
    pub board_size: u8,
    pub komi: f32,
    pub handicap: u8,
    pub total_moves: usize,
}

/// Parse SGF file and detect Ko situations
pub fn parse_sgf_with_ko_detection(sgf_path: &Path) -> Result<ParsedGame> {
    let sgf_content = fs::read_to_string(sgf_path)?;
    parse_sgf_content_with_ko_detection(&sgf_content, sgf_path.to_string_lossy().to_string())
}

/// Parse SGF content and detect Ko situations
pub fn parse_sgf_content_with_ko_detection(
    sgf_content: &str,
    source_path: String,
) -> Result<ParsedGame> {
    let mut warnings = Vec::new();

    // Parse SGF
    let initial_state = GameState::new(19); // Default, will be updated
    let mut sgf_processor = SgfProcessor::new(initial_state);

    let parsed_state = match sgf_processor.parse(sgf_content) {
        Ok(state) => state,
        Err(e) => {
            warnings.push(format!("SGF parsing warning: {}", e));
            // Try to recover with basic parsing
            return Err(anyhow!("Failed to parse SGF: {}", e));
        }
    };

    // Extract metadata
    let metadata = extract_metadata_from_content(sgf_content, &parsed_state);

    // Detect Ko situations
    let mut ko_detector = KoDetector::new();
    let mut board = crate::board::Board::new(parsed_state.board_size);
    let mut ko_situations = Vec::new();

    // Process each move
    for (_move_num, mv) in parsed_state.moves.iter().enumerate() {
        match mv {
            Move::Place { x, y, color } => {
                let coord = Coord::new(*x, *y);

                // Get captures before placing stone
                let captured = Vec::new();

                // Place stone and check for captures
                board.place(coord, *color);

                // Check for Ko
                if let Some(ko) = ko_detector.process_move(&board, mv, &captured) {
                    ko_situations.push(ko);
                }
            }
            _ => {} // Pass or Resign
        }
    }

    // Generate warnings
    if ko_situations.is_empty() {
        warnings.push("No Ko situations found in this game. Consider using Ko pattern generator for training.".to_string());
    }

    if metadata.total_moves < 50 {
        warnings.push(format!(
            "Short game with only {} moves. May have limited training value.",
            metadata.total_moves
        ));
    }

    Ok(ParsedGame {
        id: generate_game_id(&source_path),
        source_path,
        metadata,
        moves: parsed_state.moves.clone(),
        ko_situations,
        final_state: parsed_state,
        warnings,
    })
}

fn extract_metadata_from_content(sgf_content: &str, game_state: &GameState) -> GameMetadata {
    // Simple metadata extraction - would be improved with proper SGF parsing
    let extract_property = |prop: &str| -> String {
        if let Some(start) = sgf_content.find(&format!("{}[", prop)) {
            let start = start + prop.len() + 1;
            if let Some(end) = sgf_content[start..].find(']') {
                return sgf_content[start..start + end].to_string();
            }
        }
        String::new()
    };

    GameMetadata {
        black_player: extract_property("PB"),
        white_player: extract_property("PW"),
        black_rank: extract_property("BR"),
        white_rank: extract_property("WR"),
        result: extract_property("RE"),
        date: extract_property("DT"),
        board_size: game_state.board_size,
        komi: extract_property("KM").parse().unwrap_or(6.5),
        handicap: extract_property("HA").parse().unwrap_or(0),
        total_moves: game_state.moves.len(),
    }
}

fn generate_game_id(source_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    source_path.hash(&mut hasher);
    let hash = hasher.finish();

    format!("game_{:x}", hash)
}

/// Batch parse multiple SGF files
pub fn batch_parse_sgf_files(paths: &[PathBuf]) -> Vec<Result<ParsedGame>> {
    paths
        .iter()
        .map(|path| parse_sgf_with_ko_detection(path))
        .collect()
}

/// Statistics about parsed games
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLibraryStats {
    pub total_games: usize,
    pub games_with_ko: usize,
    pub total_ko_situations: usize,
    pub total_moves: usize,
    pub average_game_length: f32,
    pub player_ranks: HashMap<String, usize>,
}

use std::collections::HashMap;

/// Calculate statistics for a game library
pub fn calculate_library_stats(games: &[ParsedGame]) -> GameLibraryStats {
    let mut player_ranks = HashMap::new();
    let mut total_moves = 0;
    let mut total_ko = 0;
    let mut games_with_ko = 0;

    for game in games {
        total_moves += game.metadata.total_moves;
        total_ko += game.ko_situations.len();

        if !game.ko_situations.is_empty() {
            games_with_ko += 1;
        }

        // Count ranks
        for rank in [&game.metadata.black_rank, &game.metadata.white_rank] {
            if !rank.is_empty() {
                *player_ranks.entry(rank.clone()).or_insert(0) += 1;
            }
        }
    }

    GameLibraryStats {
        total_games: games.len(),
        games_with_ko,
        total_ko_situations: total_ko,
        total_moves,
        average_game_length: if games.is_empty() {
            0.0
        } else {
            total_moves as f32 / games.len() as f32
        },
        player_ranks,
    }
}
