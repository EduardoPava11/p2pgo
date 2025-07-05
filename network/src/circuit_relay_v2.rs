//! Circuit Relay v2 - Custom relay protocol for 3-player P2P Go
//!
//! This module implements a triangular relay system where each of the three
//! players can act as a relay for the other two, with credit-based incentives.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Player identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub [u8; 32]);

impl PlayerId {
    /// Create a new random player ID
    pub fn new() -> Self {
        let mut id = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut id);
        Self(id)
    }
    
    /// Create from public key bytes
    pub fn from_pubkey(key: &[u8; 32]) -> Self {
        Self(*key)
    }
}

/// Connection state between players
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Direct P2P connection established
    Direct { addr: SocketAddr },
    /// Connection through relay
    Relayed { via: PlayerId, hop_count: u8 },
    /// Searching for connection path
    Searching { since: Instant },
    /// No path available
    Disconnected,
}

/// Relay message types for 3-player game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayMessage {
    /// Request to establish connection
    Connect {
        from: PlayerId,
        to: PlayerId,
        via: Option<PlayerId>,
    },
    
    /// Game move (signed)
    GameMove {
        from: PlayerId,
        move_data: Vec<u8>, // Serialized Move3D
        signature: Vec<u8>,
        sequence: u64,
    },
    
    /// Request relay service
    RelayRequest {
        from: PlayerId,
        target: PlayerId,
        credits_offered: u64,
        ttl: u8, // Time to live (hop count)
    },
    
    /// Accept relay request
    RelayAccept {
        request_id: [u8; 16],
        relay: PlayerId,
    },
    
    /// State synchronization
    StateSync {
        board_hash: [u8; 32],
        move_count: u64,
        players: [PlayerId; 3],
        timestamp: u64,
    },
    
    /// Credit transfer
    CreditTransfer {
        from: PlayerId,
        to: PlayerId,
        amount: u64,
        reason: CreditReason,
    },
    
    /// Heartbeat/keepalive
    Ping {
        from: PlayerId,
        nonce: u64,
    },
    
    /// Heartbeat response
    Pong {
        from: PlayerId,
        nonce: u64,
    },
}

/// Reason for credit transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditReason {
    RelayService,
    GameCompletion,
    InitialAllocation,
}

/// Relay incentive structure
#[derive(Debug, Clone)]
pub struct RelayIncentive {
    /// Base credits per relayed message
    pub base_rate: u64,
    /// Bonus for maintaining stable connection
    pub stability_bonus: u64,
    /// Penalty for dropping messages
    pub drop_penalty: u64,
    /// Minimum credits to offer relay service
    pub min_balance: u64,
}

impl Default for RelayIncentive {
    fn default() -> Self {
        Self {
            base_rate: 1,           // 1 credit per message
            stability_bonus: 10,    // 10 credits per stable hour
            drop_penalty: 5,        // 5 credit penalty per drop
            min_balance: 100,       // Need 100 credits to relay
        }
    }
}

/// Circuit Relay v2 node for 3-player game
pub struct CircuitRelayNode {
    /// Our player ID
    pub id: PlayerId,
    /// Connection states to other players
    pub connections: HashMap<PlayerId, ConnectionState>,
    /// Credit balances
    pub credits: HashMap<PlayerId, u64>,
    /// Active relay sessions
    pub relay_sessions: HashMap<[u8; 16], RelaySession>,
    /// Incentive configuration
    pub incentives: RelayIncentive,
    /// Message sequence numbers
    pub sequences: HashMap<PlayerId, u64>,
    /// Pending relay requests
    pub pending_requests: Vec<PendingRelay>,
}

/// Active relay session
#[derive(Debug, Clone)]
pub struct RelaySession {
    pub id: [u8; 16],
    pub from: PlayerId,
    pub to: PlayerId,
    pub credits_paid: u64,
    pub messages_relayed: u64,
    pub started: Instant,
    pub last_activity: Instant,
}

/// Pending relay request
#[derive(Debug, Clone)]
pub struct PendingRelay {
    pub from: PlayerId,
    pub target: PlayerId,
    pub credits_offered: u64,
    pub created: Instant,
    pub ttl: u8,
}

impl CircuitRelayNode {
    /// Create a new relay node
    pub fn new(id: PlayerId) -> Self {
        let mut credits = HashMap::new();
        credits.insert(id, 1000); // Initial self-credits
        
        Self {
            id,
            connections: HashMap::new(),
            credits,
            relay_sessions: HashMap::new(),
            incentives: RelayIncentive::default(),
            sequences: HashMap::new(),
            pending_requests: Vec::new(),
        }
    }
    
    /// Initialize 3-player game connections
    pub fn init_triangle(&mut self, player2: PlayerId, player3: PlayerId) {
        // Give initial credits to all players
        self.credits.insert(player2, 500);
        self.credits.insert(player3, 500);
        
        // Mark connections as searching
        self.connections.insert(player2, ConnectionState::Searching { 
            since: Instant::now() 
        });
        self.connections.insert(player3, ConnectionState::Searching { 
            since: Instant::now() 
        });
    }
    
    /// Process incoming relay message
    pub fn handle_message(&mut self, msg: RelayMessage) -> Option<RelayMessage> {
        match msg {
            RelayMessage::RelayRequest { from, target, credits_offered, ttl } => {
                self.handle_relay_request(from, target, credits_offered, ttl)
            }
            
            RelayMessage::GameMove { from, .. } => {
                self.handle_game_move(from, msg)
            }
            
            RelayMessage::CreditTransfer { from, to, amount, reason } => {
                self.handle_credit_transfer(from, to, amount, reason);
                None
            }
            
            RelayMessage::Ping { from: _, nonce } => {
                Some(RelayMessage::Pong { from: self.id, nonce })
            }
            
            _ => None,
        }
    }
    
    /// Handle relay request
    fn handle_relay_request(
        &mut self, 
        from: PlayerId, 
        target: PlayerId, 
        credits_offered: u64,
        ttl: u8
    ) -> Option<RelayMessage> {
        // Check if we can relay (have connection to target)
        if let Some(ConnectionState::Direct { .. }) = self.connections.get(&target) {
            // Check if we have minimum balance
            if self.credits.get(&self.id).copied().unwrap_or(0) >= self.incentives.min_balance {
                // Check if offered credits are sufficient
                if credits_offered >= self.incentives.base_rate {
                    // Create relay session
                    let session_id = self.create_session_id();
                    let session = RelaySession {
                        id: session_id,
                        from,
                        to: target,
                        credits_paid: credits_offered,
                        messages_relayed: 0,
                        started: Instant::now(),
                        last_activity: Instant::now(),
                    };
                    
                    self.relay_sessions.insert(session_id, session);
                    
                    return Some(RelayMessage::RelayAccept {
                        request_id: session_id,
                        relay: self.id,
                    });
                }
            }
        }
        
        // Can't relay, but forward request if TTL allows
        if ttl > 0 {
            self.forward_relay_request(from, target, credits_offered, ttl - 1)
        } else {
            None
        }
    }
    
    /// Handle game move through relay
    fn handle_game_move(&mut self, from: PlayerId, msg: RelayMessage) -> Option<RelayMessage> {
        // Find active relay session
        for session in self.relay_sessions.values_mut() {
            if session.from == from {
                // Update session
                session.messages_relayed += 1;
                session.last_activity = Instant::now();
                
                // Deduct relay fee
                if session.messages_relayed * self.incentives.base_rate <= session.credits_paid {
                    // Forward the message
                    return Some(msg);
                }
            }
        }
        
        None
    }
    
    /// Handle credit transfer
    fn handle_credit_transfer(
        &mut self, 
        from: PlayerId, 
        to: PlayerId, 
        amount: u64, 
        _reason: CreditReason
    ) {
        // Verify sender has sufficient credits
        if let Some(sender_balance) = self.credits.get_mut(&from) {
            if *sender_balance >= amount {
                *sender_balance -= amount;
                *self.credits.entry(to).or_insert(0) += amount;
            }
        }
    }
    
    /// Forward relay request to other connections
    fn forward_relay_request(
        &self, 
        from: PlayerId, 
        target: PlayerId, 
        credits_offered: u64,
        ttl: u8
    ) -> Option<RelayMessage> {
        // Find another player to forward to
        for (player, state) in &self.connections {
            if *player != from && matches!(state, ConnectionState::Direct { .. }) {
                return Some(RelayMessage::RelayRequest {
                    from,
                    target,
                    credits_offered,
                    ttl,
                });
            }
        }
        None
    }
    
    /// Create unique session ID
    fn create_session_id(&self) -> [u8; 16] {
        let mut id = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut id);
        id
    }
    
    /// Clean up expired sessions
    pub fn cleanup_sessions(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minute timeout
        
        self.relay_sessions.retain(|_id, session| {
            now.duration_since(session.last_activity) < timeout
        });
    }
    
    /// Get current credit balance
    pub fn get_balance(&self, player: &PlayerId) -> u64 {
        self.credits.get(player).copied().unwrap_or(0)
    }
    
    /// Calculate total relay earnings
    pub fn calculate_earnings(&self) -> u64 {
        self.relay_sessions.values()
            .map(|s| s.messages_relayed * self.incentives.base_rate)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_triangle_init() {
        let player1 = PlayerId::new();
        let player2 = PlayerId::new();
        let player3 = PlayerId::new();
        
        let mut node = CircuitRelayNode::new(player1);
        node.init_triangle(player2, player3);
        
        assert_eq!(node.connections.len(), 2);
        assert_eq!(node.get_balance(&player2), 500);
        assert_eq!(node.get_balance(&player3), 500);
    }
    
    #[test]
    fn test_relay_request() {
        let relay = PlayerId::new();
        let sender = PlayerId::new();
        let target = PlayerId::new();
        
        let mut node = CircuitRelayNode::new(relay);
        node.connections.insert(target, ConnectionState::Direct { 
            addr: "127.0.0.1:9000".parse().unwrap() 
        });
        
        let msg = RelayMessage::RelayRequest {
            from: sender,
            target,
            credits_offered: 10,
            ttl: 3,
        };
        
        let response = node.handle_message(msg);
        assert!(matches!(response, Some(RelayMessage::RelayAccept { .. })));
        assert_eq!(node.relay_sessions.len(), 1);
    }
}