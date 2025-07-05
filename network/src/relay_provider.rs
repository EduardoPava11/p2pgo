//! Unified relay provider interface
//!
//! This module provides a common trait for different relay implementations,
//! allowing the game to select the appropriate relay mode based on player
//! preferences and game configuration.

use anyhow::Result;
use async_trait::async_trait;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

/// Player preferences for relay usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPreferences {
    /// Maximum acceptable latency in milliseconds
    pub max_latency_ms: u64,
    /// Whether to prefer direct connections over relayed
    pub prefer_direct: bool,
    /// Maximum relay hops allowed
    pub max_relay_hops: u8,
    /// Guild affiliation affects relay behavior
    pub guild: PlayerGuild,
}

/// Player guild types that affect relay preferences
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerGuild {
    /// Aggressive players - prefer direct connections
    Activity,
    /// Defensive players - accept relay usage for stability
    Reactivity,
    /// Balanced players - flexible relay usage
    Avoidance,
}

impl Default for RelayPreferences {
    fn default() -> Self {
        Self {
            max_latency_ms: 200,
            prefer_direct: true,
            max_relay_hops: 2,
            guild: PlayerGuild::Avoidance,
        }
    }
}

/// Relay connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayState {
    /// Direct peer-to-peer connection
    Direct,
    /// Connection through relay
    Relayed { relay_id: PeerId, latency_ms: u64 },
    /// Attempting to establish connection
    Connecting,
    /// No connection established
    Disconnected,
}

/// Common interface for all relay providers
#[async_trait]
pub trait RelayProvider: Send + Sync {
    /// Get the name of this relay provider
    fn name(&self) -> &str;

    /// Check if this provider is suitable for the given player count
    fn supports_player_count(&self, count: usize) -> bool;

    /// Initialize the relay provider
    async fn initialize(&mut self) -> Result<()>;

    /// Connect to a peer, potentially through relay
    async fn connect_to_peer(
        &mut self,
        peer_id: PeerId,
        known_addrs: Vec<Multiaddr>,
        preferences: &RelayPreferences,
    ) -> Result<RelayState>;

    /// Get current connection state to a peer
    fn get_connection_state(&self, peer_id: &PeerId) -> RelayState;

    /// Send a message to a peer
    async fn send_message(&mut self, peer_id: &PeerId, message: Vec<u8>) -> Result<()>;

    /// Receive messages (non-blocking)
    async fn receive_messages(&mut self) -> Result<Vec<(PeerId, Vec<u8>)>>;

    /// Disconnect from a peer
    async fn disconnect_from_peer(&mut self, peer_id: &PeerId) -> Result<()>;

    /// Get relay statistics
    fn get_stats(&self) -> RelayStats;

    /// Shutdown the relay provider
    async fn shutdown(&mut self) -> Result<()>;
}

/// Statistics for relay performance
#[derive(Debug, Clone, Default)]
pub struct RelayStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Number of active connections
    pub active_connections: usize,
    /// Number of relayed connections
    pub relayed_connections: usize,
    /// Credits spent on relay usage
    pub credits_spent: u64,
    /// Credits earned by providing relay
    pub credits_earned: u64,
}

/// Factory for creating relay providers based on game configuration
pub struct RelayProviderFactory;

impl RelayProviderFactory {
    /// Create appropriate relay provider based on player count and preferences
    pub fn create_provider(
        player_count: usize,
        preferences: Vec<RelayPreferences>,
    ) -> Box<dyn RelayProvider> {
        match player_count {
            2 => {
                // For 2 players, use simple direct connection with relay fallback
                Box::new(SimpleRelayProvider::new(preferences))
            }
            3 => {
                // For 3 players, use triangular circuit relay
                Box::new(CircuitRelayV2Provider::new(preferences))
            }
            _ => {
                // For more players, use mesh relay network
                Box::new(MeshRelayProvider::new(preferences))
            }
        }
    }

    /// Create provider based on guild preferences
    pub fn create_for_guild(guild: PlayerGuild, player_count: usize) -> Box<dyn RelayProvider> {
        let preferences = match guild {
            PlayerGuild::Activity => {
                // Aggressive players want minimal relay usage
                RelayPreferences {
                    max_latency_ms: 50,
                    prefer_direct: true,
                    max_relay_hops: 1,
                    guild,
                }
            }
            PlayerGuild::Reactivity => {
                // Defensive players accept relay for stability
                RelayPreferences {
                    max_latency_ms: 300,
                    prefer_direct: false,
                    max_relay_hops: 3,
                    guild,
                }
            }
            PlayerGuild::Avoidance => {
                // Balanced approach
                RelayPreferences::default()
            }
        };

        let all_preferences = vec![preferences; player_count];
        Self::create_provider(player_count, all_preferences)
    }
}

// Placeholder implementations - these would be implemented by the actual relay modules

struct SimpleRelayProvider {
    #[allow(dead_code)]
    preferences: Vec<RelayPreferences>,
}

impl SimpleRelayProvider {
    fn new(preferences: Vec<RelayPreferences>) -> Self {
        Self { preferences }
    }
}

#[async_trait]
impl RelayProvider for SimpleRelayProvider {
    fn name(&self) -> &str {
        "Simple Relay"
    }

    fn supports_player_count(&self, count: usize) -> bool {
        count == 2
    }

    async fn initialize(&mut self) -> Result<()> {
        // TODO: Initialize simple relay
        Ok(())
    }

    async fn connect_to_peer(
        &mut self,
        _peer_id: PeerId,
        _known_addrs: Vec<Multiaddr>,
        _preferences: &RelayPreferences,
    ) -> Result<RelayState> {
        // TODO: Implement connection logic
        Ok(RelayState::Connecting)
    }

    fn get_connection_state(&self, _peer_id: &PeerId) -> RelayState {
        RelayState::Disconnected
    }

    async fn send_message(&mut self, _peer_id: &PeerId, _message: Vec<u8>) -> Result<()> {
        Ok(())
    }

    async fn receive_messages(&mut self) -> Result<Vec<(PeerId, Vec<u8>)>> {
        Ok(vec![])
    }

    async fn disconnect_from_peer(&mut self, _peer_id: &PeerId) -> Result<()> {
        Ok(())
    }

    fn get_stats(&self) -> RelayStats {
        RelayStats::default()
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

struct CircuitRelayV2Provider {
    #[allow(dead_code)]
    preferences: Vec<RelayPreferences>,
}

impl CircuitRelayV2Provider {
    fn new(preferences: Vec<RelayPreferences>) -> Self {
        Self { preferences }
    }
}

#[async_trait]
impl RelayProvider for CircuitRelayV2Provider {
    fn name(&self) -> &str {
        "Circuit Relay V2"
    }

    fn supports_player_count(&self, count: usize) -> bool {
        count == 3
    }

    async fn initialize(&mut self) -> Result<()> {
        // TODO: Initialize circuit relay
        Ok(())
    }

    async fn connect_to_peer(
        &mut self,
        _peer_id: PeerId,
        _known_addrs: Vec<Multiaddr>,
        _preferences: &RelayPreferences,
    ) -> Result<RelayState> {
        // TODO: Implement triangular relay logic
        Ok(RelayState::Connecting)
    }

    fn get_connection_state(&self, _peer_id: &PeerId) -> RelayState {
        RelayState::Disconnected
    }

    async fn send_message(&mut self, _peer_id: &PeerId, _message: Vec<u8>) -> Result<()> {
        Ok(())
    }

    async fn receive_messages(&mut self) -> Result<Vec<(PeerId, Vec<u8>)>> {
        Ok(vec![])
    }

    async fn disconnect_from_peer(&mut self, _peer_id: &PeerId) -> Result<()> {
        Ok(())
    }

    fn get_stats(&self) -> RelayStats {
        RelayStats::default()
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

struct MeshRelayProvider {
    #[allow(dead_code)]
    preferences: Vec<RelayPreferences>,
}

impl MeshRelayProvider {
    fn new(preferences: Vec<RelayPreferences>) -> Self {
        Self { preferences }
    }
}

#[async_trait]
impl RelayProvider for MeshRelayProvider {
    fn name(&self) -> &str {
        "Mesh Relay"
    }

    fn supports_player_count(&self, count: usize) -> bool {
        count > 3
    }

    async fn initialize(&mut self) -> Result<()> {
        // TODO: Initialize mesh relay
        Ok(())
    }

    async fn connect_to_peer(
        &mut self,
        _peer_id: PeerId,
        _known_addrs: Vec<Multiaddr>,
        _preferences: &RelayPreferences,
    ) -> Result<RelayState> {
        // TODO: Implement mesh relay logic
        Ok(RelayState::Connecting)
    }

    fn get_connection_state(&self, _peer_id: &PeerId) -> RelayState {
        RelayState::Disconnected
    }

    async fn send_message(&mut self, _peer_id: &PeerId, _message: Vec<u8>) -> Result<()> {
        Ok(())
    }

    async fn receive_messages(&mut self) -> Result<Vec<(PeerId, Vec<u8>)>> {
        Ok(vec![])
    }

    async fn disconnect_from_peer(&mut self, _peer_id: &PeerId) -> Result<()> {
        Ok(())
    }

    fn get_stats(&self) -> RelayStats {
        RelayStats::default()
    }

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_provider_selection() {
        // Test 2-player game
        let provider = RelayProviderFactory::create_provider(2, vec![]);
        assert_eq!(provider.name(), "Simple Relay");
        assert!(provider.supports_player_count(2));

        // Test 3-player game
        let provider = RelayProviderFactory::create_provider(3, vec![]);
        assert_eq!(provider.name(), "Circuit Relay V2");
        assert!(provider.supports_player_count(3));

        // Test 4+ player game
        let provider = RelayProviderFactory::create_provider(4, vec![]);
        assert_eq!(provider.name(), "Mesh Relay");
        assert!(provider.supports_player_count(4));
    }

    #[test]
    fn test_guild_preferences() {
        // Activity guild prefers direct connections
        let provider = RelayProviderFactory::create_for_guild(PlayerGuild::Activity, 2);
        assert_eq!(provider.name(), "Simple Relay");

        // Reactivity guild accepts relay usage
        let provider = RelayProviderFactory::create_for_guild(PlayerGuild::Reactivity, 2);
        assert_eq!(provider.name(), "Simple Relay");

        // Avoidance guild uses defaults
        let provider = RelayProviderFactory::create_for_guild(PlayerGuild::Avoidance, 2);
        assert_eq!(provider.name(), "Simple Relay");
    }
}
