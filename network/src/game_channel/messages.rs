// SPDX-License-Identifier: MIT OR Apache-2.0

//! Message types for game channel communication

use serde::{Serialize, Deserialize};
use p2pgo_core::{Move, GameState};

/// Acknowledgment message for received moves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveAck {
    /// Game ID this ACK is for
    pub game_id: String,
    /// Index of the move being acknowledged
    pub move_index: usize,
    /// Timestamp when the ACK was created
    pub timestamp: u64,
}

/// Request for game state synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Game ID to sync
    pub game_id: String,
    /// Timestamp when the sync request was created
    pub timestamp: u64,
}

/// Response containing game state for synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Game ID this response is for
    pub game_id: String,
    /// All moves in the game
    pub moves: Vec<Move>,
    /// Current game state
    pub state: GameState,
    /// Timestamp when the sync response was created
    pub timestamp: u64,
}

/// Heartbeat message to check connection liveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Game ID this heartbeat is for
    pub game_id: String,
    /// Timestamp when the heartbeat was created
    pub timestamp: u64,
    /// Optional sequence number for tracking
    pub sequence: Option<u64>,
}

/// Response to a heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    /// Game ID this response is for
    pub game_id: String,
    /// Timestamp when the response was created
    pub timestamp: u64,
    /// Sequence number from the original heartbeat
    pub sequence: Option<u64>,
    /// Round-trip time in milliseconds (optional)
    pub rtt_ms: Option<u64>,
}

/// Request to join a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// Game ID to join
    pub game_id: String,
    /// Player's preferred color (if any)
    pub preferred_color: Option<p2pgo_core::Color>,
    /// Player's display name
    pub player_name: String,
    /// Timestamp when the request was created
    pub timestamp: u64,
}

/// Response to a join request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    /// Game ID
    pub game_id: String,
    /// Whether the join was successful
    pub success: bool,
    /// Assigned color (if successful)
    pub assigned_color: Option<p2pgo_core::Color>,
    /// Error message (if unsuccessful)
    pub error_message: Option<String>,
    /// Current game state (if successful)
    pub game_state: Option<GameState>,
    /// Timestamp when the response was created
    pub timestamp: u64,
}

/// Chat message between players
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Game ID this message is for
    pub game_id: String,
    /// Player who sent the message
    pub player_name: String,
    /// Message content
    pub message: String,
    /// Timestamp when the message was created
    pub timestamp: u64,
}

/// Game event notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEventMessage {
    /// Game ID this event is for
    pub game_id: String,
    /// The game event
    pub event: p2pgo_core::GameEvent,
    /// Timestamp when the event occurred
    pub timestamp: u64,
}

/// Status update for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStatusUpdate {
    /// Game ID
    pub game_id: String,
    /// Player name
    pub player_name: String,
    /// New status
    pub status: super::PlayerStatus,
    /// Timestamp when the status changed
    pub timestamp: u64,
}

/// Wrapper for all message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameChannelMessage {
    MoveAck(MoveAck),
    SyncRequest(SyncRequest),
    SyncResponse(SyncResponse),
    Heartbeat(Heartbeat),
    HeartbeatResponse(HeartbeatResponse),
    JoinRequest(JoinRequest),
    JoinResponse(JoinResponse),
    ChatMessage(ChatMessage),
    GameEvent(GameEventMessage),
    PlayerStatus(PlayerStatusUpdate),
}

impl GameChannelMessage {
    /// Get the game ID for any message type
    pub fn game_id(&self) -> &str {
        match self {
            GameChannelMessage::MoveAck(msg) => &msg.game_id,
            GameChannelMessage::SyncRequest(msg) => &msg.game_id,
            GameChannelMessage::SyncResponse(msg) => &msg.game_id,
            GameChannelMessage::Heartbeat(msg) => &msg.game_id,
            GameChannelMessage::HeartbeatResponse(msg) => &msg.game_id,
            GameChannelMessage::JoinRequest(msg) => &msg.game_id,
            GameChannelMessage::JoinResponse(msg) => &msg.game_id,
            GameChannelMessage::ChatMessage(msg) => &msg.game_id,
            GameChannelMessage::GameEvent(msg) => &msg.game_id,
            GameChannelMessage::PlayerStatus(msg) => &msg.game_id,
        }
    }
    
    /// Get the timestamp for any message type
    pub fn timestamp(&self) -> u64 {
        match self {
            GameChannelMessage::MoveAck(msg) => msg.timestamp,
            GameChannelMessage::SyncRequest(msg) => msg.timestamp,
            GameChannelMessage::SyncResponse(msg) => msg.timestamp,
            GameChannelMessage::Heartbeat(msg) => msg.timestamp,
            GameChannelMessage::HeartbeatResponse(msg) => msg.timestamp,
            GameChannelMessage::JoinRequest(msg) => msg.timestamp,
            GameChannelMessage::JoinResponse(msg) => msg.timestamp,
            GameChannelMessage::ChatMessage(msg) => msg.timestamp,
            GameChannelMessage::GameEvent(msg) => msg.timestamp,
            GameChannelMessage::PlayerStatus(msg) => msg.timestamp,
        }
    }
    
    /// Serialize the message to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    /// Deserialize a message from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    /// Serialize the message to CBOR
    pub fn to_cbor(&self) -> Result<Vec<u8>, serde_cbor::Error> {
        serde_cbor::to_vec(self)
    }
    
    /// Deserialize a message from CBOR
    pub fn from_cbor(cbor: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(cbor)
    }
}

/// Helper functions for creating common messages
impl GameChannelMessage {
    /// Create a move acknowledgment message
    pub fn move_ack(game_id: String, move_index: usize) -> Self {
        GameChannelMessage::MoveAck(MoveAck {
            game_id,
            move_index,
            timestamp: current_timestamp(),
        })
    }
    
    /// Create a sync request message
    pub fn sync_request(game_id: String) -> Self {
        GameChannelMessage::SyncRequest(SyncRequest {
            game_id,
            timestamp: current_timestamp(),
        })
    }
    
    /// Create a sync response message
    pub fn sync_response(game_id: String, moves: Vec<Move>, state: GameState) -> Self {
        GameChannelMessage::SyncResponse(SyncResponse {
            game_id,
            moves,
            state,
            timestamp: current_timestamp(),
        })
    }
    
    /// Create a heartbeat message
    pub fn heartbeat(game_id: String, sequence: Option<u64>) -> Self {
        GameChannelMessage::Heartbeat(Heartbeat {
            game_id,
            timestamp: current_timestamp(),
            sequence,
        })
    }
    
    /// Create a heartbeat response message
    pub fn heartbeat_response(game_id: String, sequence: Option<u64>, rtt_ms: Option<u64>) -> Self {
        GameChannelMessage::HeartbeatResponse(HeartbeatResponse {
            game_id,
            timestamp: current_timestamp(),
            sequence,
            rtt_ms,
        })
    }
    
    /// Create a chat message
    pub fn chat(game_id: String, player_name: String, message: String) -> Self {
        GameChannelMessage::ChatMessage(ChatMessage {
            game_id,
            player_name,
            message,
            timestamp: current_timestamp(),
        })
    }
}

/// Get the current timestamp in seconds since Unix epoch
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{GameState, Move, Coord};
    
    #[test]
    fn test_message_serialization() {
        let game_id = "test-game".to_string();
        
        // Test MoveAck
        let ack = GameChannelMessage::move_ack(game_id.clone(), 5);
        let json = ack.to_json().unwrap();
        let deserialized = GameChannelMessage::from_json(&json).unwrap();
        assert_eq!(ack.game_id(), deserialized.game_id());
        
        // Test SyncRequest
        let sync_req = GameChannelMessage::sync_request(game_id.clone());
        let cbor = sync_req.to_cbor().unwrap();
        let deserialized_cbor = GameChannelMessage::from_cbor(&cbor).unwrap();
        assert_eq!(sync_req.game_id(), deserialized_cbor.game_id());
        
        // Test SyncResponse
        let state = GameState::new(9);
        let moves = vec![Move::Place { x: 4, y: 4, color: p2pgo_core::Color::Black }];
        let sync_resp = GameChannelMessage::sync_response(game_id.clone(), moves, state);
        let json = sync_resp.to_json().unwrap();
        let deserialized = GameChannelMessage::from_json(&json).unwrap();
        assert_eq!(sync_resp.game_id(), deserialized.game_id());
    }
    
    #[test]
    fn test_message_properties() {
        let game_id = "test-game".to_string();
        let msg = GameChannelMessage::heartbeat(game_id.clone(), Some(42));
        
        assert_eq!(msg.game_id(), &game_id);
        assert!(msg.timestamp() > 0);
        
        match msg {
            GameChannelMessage::Heartbeat(heartbeat) => {
                assert_eq!(heartbeat.sequence, Some(42));
            }
            _ => panic!("Expected heartbeat message"),
        }
    }
}
