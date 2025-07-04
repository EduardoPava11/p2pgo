// SPDX-License-Identifier: MIT OR Apache-2.0

//! Message types for UI-Network communication.

use p2pgo_core::{Move, GameEvent, Coord, Tag};
use p2pgo_network::lobby::GameInfo;

/// Messages sent from UI to Network worker
#[derive(Debug, Clone)]
pub enum UiToNet {
    /// Create a new game
    CreateGame { board_size: u8 },
    /// Join an existing game by ID
    JoinGame { game_id: String },
    /// Make a move in the current game
    MakeMove { mv: Move, board_size: Option<u8> },
    /// Request refresh of available games
    RefreshGames,
    /// Leave the current game
    LeaveGame,
    /// Shutdown the network worker
    Shutdown,
    /// Debug: Move placed at coordinate (for testing)
    DebugMovePlaced(Coord),
    /// Connect to peer by ticket
    ConnectByTicket { ticket: String },
    /// Request node ID
    GetNodeId,
    /// Request connection ticket
    GetTicket,
    /// Run NAT report
    RunNetReport,
    /// Update default board size for gossip subscription
    UpdateBoardSize { board_size: u8 },
    /// Set tag for a move
    #[allow(dead_code)]
    SetTag { gid: String, seq: u32, tag: Tag },
    /// Request AI ghost moves for current board state
    GetGhostMoves,
    /// Calculate score at end of game
    CalculateScore { 
        dead_stones: std::collections::HashSet<p2pgo_core::Coord> 
    },
    /// Accept score at end of game
    AcceptScore { 
        score_proof: p2pgo_core::value_labeller::ScoreProof 
    },
    /// Save new network configuration and restart networking
    SaveConfigAndRestart { 
        config_json: String 
    },
    /// Force restart the network layer
    RestartNetwork,
}

/// Messages sent from Network worker to UI
#[derive(Debug, Clone)]
pub enum NetToUi {
    /// Debug message for development
    Debug(String),
    /// Game list updated
    GamesUpdated { games: Vec<GameInfo> },
    /// Game event occurred
    GameEvent { event: GameEvent },
    /// Successfully joined/created a game
    GameJoined { game_id: String },
    /// Left the current game
    GameLeft,
    /// Network error occurred
    Error { message: String },
    /// Connection status changed
    #[allow(dead_code)]
    ConnectionStatus { connected: bool },
    /// Acknowledgment that shutdown was processed
    ShutdownAck,
    /// Node ID response
    NodeId { node_id: String },
    /// Connection ticket response
    Ticket { ticket: String },
    /// NAT report result
    NetReport { report: String },
    /// Tag acknowledgment
    TagAck,
    /// Ghost moves for AI suggestions
    #[allow(dead_code)]
    GhostMoves(Vec<Coord>),
    /// Score calculation result
    ScoreCalculated {
        score_proof: p2pgo_core::value_labeller::ScoreProof,
    },
    /// Score accepted by both players (finalized)
    ScoreAcceptedByBoth {
        score_proof: p2pgo_core::value_labeller::ScoreProof,
    },
    /// Score timeout (3 minutes)
    ScoreTimeout {
        board_size: u8,
    },
    /// Game advertisement received via gossip
    GameAdvertised {
        game_id: String,
        host_id: String,
        board_size: u8,
    },
    /// Network layer is restarting (relay restart, config change, etc.)
    NetRestarting {
        reason: String,
    },
    /// Network layer restart completed
    NetRestartCompleted,
    /// Relay health status update
    RelayHealth {
        /// Overall health status (Healthy, Degraded, Restarting, etc.)
        status: p2pgo_network::relay_monitor::RelayHealthStatus,
        /// Port the relay is listening on (if any)
        port: Option<u16>,
        /// Is this node acting as a relay
        is_relay_node: bool,
        /// Last restart time, if applicable
        last_restart: Option<std::time::SystemTime>,
    },
    /// Relay capacity status update (current connections / bandwidth)
    RelayCapacity {
        /// Current number of connections
        current_connections: usize,
        /// Maximum allowed connections
        max_connections: usize,
        /// Current bandwidth usage in Mbps
        current_bandwidth_mbps: f64,
        /// Maximum allowed bandwidth in Mbps
        max_bandwidth_mbps: f64,
    },
}

/// Extension trait for NetToUi messages
#[cfg(test)]
pub trait NetToUiExt {
    /// Try to convert the message to a string (for ticket handling)
    fn as_string(&self) -> Result<String, ()>;
}

#[cfg(test)]
impl NetToUiExt for NetToUi {
    fn as_string(&self) -> Result<String, ()> {
        match self {
            NetToUi::Ticket { ticket } => Ok(ticket.clone()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
impl ToString for NetToUi {
    fn to_string(&self) -> String {
        match self {
            NetToUi::Ticket { ticket } => ticket.clone(),
            _ => String::from("DEFAULT-TICKET-FOR-TESTING"),
        }
    }
}
