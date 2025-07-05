//! Game activity logger for monitoring and debugging

use crate::msg::{NetToUi, UiToNet};
use chrono::{DateTime, Local};
use p2pgo_core::{Color, Coord, GameEvent, GameState, Move};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Game activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameActivityEntry {
    /// Timestamp of the entry
    pub timestamp: DateTime<Local>,
    /// Unix timestamp for sorting
    pub unix_timestamp: u64,
    /// Entry type
    pub entry_type: ActivityType,
    /// Game ID if applicable
    pub game_id: Option<String>,
    /// Detailed data
    pub data: serde_json::Value,
}

/// Types of activity to log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActivityType {
    /// Game started
    GameStarted {
        board_size: u8,
        our_color: Option<Color>,
        opponent_id: Option<String>,
    },
    /// Move made
    MoveMade {
        move_data: MoveData,
        game_state: GameStateSnapshot,
    },
    /// Move received
    MoveReceived {
        move_data: MoveData,
        latency_ms: Option<u32>,
    },
    /// Network operation
    NetworkOp {
        operation: NetworkOperation,
        success: bool,
        error: Option<String>,
    },
    /// UI interaction
    UiInteraction { interaction: UiInteraction },
    /// Game ended
    GameEnded {
        result: String,
        final_score: Option<(f32, f32)>,
        total_moves: u32,
    },
    /// Error occurred
    Error {
        component: String,
        error: String,
        context: Option<serde_json::Value>,
    },
    /// Performance metric
    Performance { metric: PerformanceMetric },
}

/// Move data for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub move_type: String,
    pub coord: Option<(u8, u8)>,
    pub color: String,
    pub move_number: u32,
    pub thinking_time_ms: Option<u32>,
}

/// Snapshot of game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub board_size: u8,
    pub current_player: String,
    pub captures: (u16, u16),
    pub pass_count: u8,
    pub stone_count: (usize, usize),
}

/// Network operation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkOperation {
    pub op_type: String,
    pub peer_id: Option<String>,
    pub relay_id: Option<String>,
    pub latency_ms: Option<u32>,
    pub bytes_sent: Option<u64>,
    pub bytes_received: Option<u64>,
}

/// UI interaction details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInteraction {
    pub action: String,
    pub target: String,
    pub details: Option<serde_json::Value>,
}

/// Performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub metric_type: String,
    pub value: f64,
    pub unit: String,
    pub context: Option<String>,
}

/// Game activity logger
pub struct GameActivityLogger {
    /// Log file path
    log_file: PathBuf,
    /// File handle
    file_handle: Arc<Mutex<File>>,
    /// In-memory buffer for recent entries
    recent_entries: Arc<Mutex<Vec<GameActivityEntry>>>,
    /// Maximum entries to keep in memory
    max_memory_entries: usize,
    /// Whether to also log to console
    console_output: bool,
}

impl GameActivityLogger {
    /// Create a new game activity logger
    pub fn new(console_output: bool) -> Result<Self, std::io::Error> {
        // Create log directory
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("p2pgo")
            .join("game_logs");

        std::fs::create_dir_all(&log_dir)?;

        // Create log file with timestamp
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let log_file = log_dir.join(format!("game_activity_{}.jsonl", timestamp));

        let file_handle = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        Ok(Self {
            log_file,
            file_handle: Arc::new(Mutex::new(file_handle)),
            recent_entries: Arc::new(Mutex::new(Vec::new())),
            max_memory_entries: 1000,
            console_output,
        })
    }

    /// Log a game start
    pub fn log_game_start(&self, game_id: &str, board_size: u8, our_color: Option<Color>) {
        self.log_entry(
            ActivityType::GameStarted {
                board_size,
                our_color,
                opponent_id: None,
            },
            Some(game_id.to_string()),
        );
    }

    /// Log a move made by us
    pub fn log_move_made(
        &self,
        game_id: &str,
        mv: &Move,
        game_state: &GameState,
        thinking_time_ms: u32,
    ) {
        let move_data =
            self.move_to_data(mv, game_state.moves.len() as u32, Some(thinking_time_ms));
        let state_snapshot = self.snapshot_game_state(game_state);

        self.log_entry(
            ActivityType::MoveMade {
                move_data,
                game_state: state_snapshot,
            },
            Some(game_id.to_string()),
        );
    }

    /// Log a move received from opponent
    pub fn log_move_received(
        &self,
        game_id: &str,
        mv: &Move,
        move_number: u32,
        latency_ms: Option<u32>,
    ) {
        let move_data = self.move_to_data(mv, move_number, None);

        self.log_entry(
            ActivityType::MoveReceived {
                move_data,
                latency_ms,
            },
            Some(game_id.to_string()),
        );
    }

    /// Log a network operation
    pub fn log_network_op(&self, op: NetworkOperation, success: bool, error: Option<String>) {
        self.log_entry(
            ActivityType::NetworkOp {
                operation: op,
                success,
                error,
            },
            None,
        );
    }

    /// Log a UI message sent
    pub fn log_ui_message_sent(&self, msg: &UiToNet) {
        let op = NetworkOperation {
            op_type: format!("UiToNet::{}", self.message_type_name(msg)),
            peer_id: None,
            relay_id: None,
            latency_ms: None,
            bytes_sent: Some(self.estimate_message_size(msg)),
            bytes_received: None,
        };

        self.log_network_op(op, true, None);
    }

    /// Log a network message received
    pub fn log_net_message_received(&self, msg: &NetToUi) {
        let op = NetworkOperation {
            op_type: format!("NetToUi::{}", self.net_message_type_name(msg)),
            peer_id: None,
            relay_id: None,
            latency_ms: None,
            bytes_sent: None,
            bytes_received: Some(self.estimate_net_message_size(msg)),
        };

        self.log_network_op(op, true, None);
    }

    /// Log a UI interaction
    pub fn log_ui_interaction(
        &self,
        action: &str,
        target: &str,
        details: Option<serde_json::Value>,
    ) {
        self.log_entry(
            ActivityType::UiInteraction {
                interaction: UiInteraction {
                    action: action.to_string(),
                    target: target.to_string(),
                    details,
                },
            },
            None,
        );
    }

    /// Log game end
    pub fn log_game_end(
        &self,
        game_id: &str,
        result: &str,
        final_score: Option<(f32, f32)>,
        total_moves: u32,
    ) {
        self.log_entry(
            ActivityType::GameEnded {
                result: result.to_string(),
                final_score,
                total_moves,
            },
            Some(game_id.to_string()),
        );
    }

    /// Log an error
    pub fn log_error(&self, component: &str, error: &str, context: Option<serde_json::Value>) {
        self.log_entry(
            ActivityType::Error {
                component: component.to_string(),
                error: error.to_string(),
                context,
            },
            None,
        );
    }

    /// Log a performance metric
    pub fn log_performance(
        &self,
        metric_type: &str,
        value: f64,
        unit: &str,
        context: Option<&str>,
    ) {
        self.log_entry(
            ActivityType::Performance {
                metric: PerformanceMetric {
                    metric_type: metric_type.to_string(),
                    value,
                    unit: unit.to_string(),
                    context: context.map(|s| s.to_string()),
                },
            },
            None,
        );
    }

    /// Get recent log entries
    pub fn get_recent_entries(&self, count: usize) -> Vec<GameActivityEntry> {
        let entries = self.recent_entries.lock().unwrap();
        let start = entries.len().saturating_sub(count);
        entries[start..].to_vec()
    }

    /// Get log file path
    pub fn get_log_file_path(&self) -> &PathBuf {
        &self.log_file
    }

    /// Internal: Log an entry
    fn log_entry(&self, entry_type: ActivityType, game_id: Option<String>) {
        let entry = GameActivityEntry {
            timestamp: Local::now(),
            unix_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            entry_type: entry_type.clone(),
            game_id,
            data: serde_json::to_value(&entry_type).unwrap_or(serde_json::Value::Null),
        };

        // Write to file
        if let Ok(mut file) = self.file_handle.lock() {
            if let Ok(json) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{}", json);
                let _ = file.flush();
            }
        }

        // Add to memory buffer
        if let Ok(mut entries) = self.recent_entries.lock() {
            entries.push(entry.clone());
            if entries.len() > self.max_memory_entries {
                entries.remove(0);
            }
        }

        // Console output if enabled
        if self.console_output {
            self.print_entry(&entry);
        }
    }

    /// Convert move to data
    fn move_to_data(&self, mv: &Move, move_number: u32, thinking_time_ms: Option<u32>) -> MoveData {
        match mv {
            Move::Place { x, y, color } => MoveData {
                move_type: "Place".to_string(),
                coord: Some((*x, *y)),
                color: format!("{:?}", color),
                move_number,
                thinking_time_ms,
            },
            Move::Pass => MoveData {
                move_type: "Pass".to_string(),
                coord: None,
                color: "N/A".to_string(),
                move_number,
                thinking_time_ms,
            },
            Move::Resign => MoveData {
                move_type: "Resign".to_string(),
                coord: None,
                color: "N/A".to_string(),
                move_number,
                thinking_time_ms,
            },
        }
    }

    /// Create game state snapshot
    fn snapshot_game_state(&self, state: &GameState) -> GameStateSnapshot {
        let black_stones = state
            .board
            .iter()
            .filter(|&&c| c == Some(Color::Black))
            .count();
        let white_stones = state
            .board
            .iter()
            .filter(|&&c| c == Some(Color::White))
            .count();

        GameStateSnapshot {
            board_size: state.board_size,
            current_player: format!("{:?}", state.current_player),
            captures: state.captures,
            pass_count: state.pass_count,
            stone_count: (black_stones, white_stones),
        }
    }

    /// Get message type name
    fn message_type_name(&self, msg: &UiToNet) -> &'static str {
        match msg {
            UiToNet::CreateGame { .. } => "CreateGame",
            UiToNet::JoinGame { .. } => "JoinGame",
            UiToNet::MakeMove { .. } => "MakeMove",
            UiToNet::LeaveGame => "LeaveGame",
            UiToNet::RefreshGames => "GetGames",
            UiToNet::GetTicket => "GetTicket",
            UiToNet::ConnectByTicket { .. } => "ConnectToTicket",
            UiToNet::Shutdown => "Shutdown",
            _ => "Other",
        }
    }

    /// Get network message type name
    fn net_message_type_name(&self, msg: &NetToUi) -> &'static str {
        match msg {
            NetToUi::GameJoined { .. } => "GameCreated",
            NetToUi::GameJoined { .. } => "GameJoined",
            NetToUi::GamesUpdated { .. } => "GameUpdate",
            NetToUi::GameLeft => "GameEnded",
            NetToUi::Error { .. } => "Error",
            NetToUi::GamesUpdated { .. } => "GamesAvailable",
            NetToUi::Ticket { .. } => "TicketGenerated",
            _ => "Other",
        }
    }

    /// Estimate message size
    fn estimate_message_size(&self, _msg: &UiToNet) -> u64 {
        256 // Rough estimate
    }

    /// Estimate network message size
    fn estimate_net_message_size(&self, _msg: &NetToUi) -> u64 {
        512 // Rough estimate
    }

    /// Print entry to console
    fn print_entry(&self, entry: &GameActivityEntry) {
        let timestamp = entry.timestamp.format("%H:%M:%S%.3f");
        match &entry.entry_type {
            ActivityType::MoveMade { move_data, .. } => {
                println!(
                    "[{}] MOVE: {} at {:?}",
                    timestamp, move_data.move_type, move_data.coord
                );
            }
            ActivityType::NetworkOp {
                operation,
                success,
                error,
            } => {
                if *success {
                    println!("[{}] NET: {} OK", timestamp, operation.op_type);
                } else {
                    println!(
                        "[{}] NET: {} FAILED - {:?}",
                        timestamp, operation.op_type, error
                    );
                }
            }
            ActivityType::Error {
                component, error, ..
            } => {
                println!("[{}] ERROR in {}: {}", timestamp, component, error);
            }
            _ => {
                println!("[{}] {:?}", timestamp, entry.entry_type);
            }
        }
    }
}

/// Global logger instance
static LOGGER: once_cell::sync::Lazy<Arc<Mutex<Option<GameActivityLogger>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global logger
pub fn init_logger(console_output: bool) -> Result<(), std::io::Error> {
    let logger = GameActivityLogger::new(console_output)?;
    *LOGGER.lock().unwrap() = Some(logger);
    Ok(())
}

/// Get the global logger
pub fn get_logger() -> Option<Arc<Mutex<Option<GameActivityLogger>>>> {
    Some(LOGGER.clone())
}

/// Log a move made
pub fn log_move(game_id: &str, mv: &Move, state: &GameState, thinking_time_ms: u32) {
    if let Some(logger_ref) = &*LOGGER.lock().unwrap() {
        logger_ref.log_move_made(game_id, mv, state, thinking_time_ms);
    }
}

/// Log a network operation
pub fn log_network(op: NetworkOperation, success: bool, error: Option<String>) {
    if let Some(logger_ref) = &*LOGGER.lock().unwrap() {
        logger_ref.log_network_op(op, success, error);
    }
}
