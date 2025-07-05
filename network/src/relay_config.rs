//! Relay configuration for P2P Go
//!
//! Provides configuration options for relay behavior with privacy in mind.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Relay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    /// Relay mode
    pub mode: RelayMode,
    /// Maximum bandwidth for relay service (bytes/sec)
    pub max_bandwidth: Option<u64>,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Relay timeout
    pub relay_timeout: Duration,
    /// Enable relay metrics collection
    pub enable_metrics: bool,
}

/// Relay operating mode
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RelayMode {
    /// Disabled - no relay functionality
    Disabled,
    /// Minimal - only use relay when necessary, don't provide service
    Minimal,
    /// Normal - use and provide relay service with reasonable limits
    Normal {
        max_reservations: usize,
        max_circuits: usize,
    },
    /// Provider - actively provide relay service for training data credits
    Provider {
        max_reservations: usize,
        max_circuits: usize,
        require_credits: bool,
    },
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            mode: RelayMode::Minimal,
            max_bandwidth: Some(1_000_000), // 1 MB/s
            max_connections: 10,
            relay_timeout: Duration::from_secs(3600), // 1 hour
            enable_metrics: false,
        }
    }
}

impl RelayConfig {
    /// Create a minimal configuration for privacy
    pub fn minimal() -> Self {
        Self {
            mode: RelayMode::Minimal,
            max_bandwidth: Some(500_000), // 500 KB/s
            max_connections: 2, // Only active games
            relay_timeout: Duration::from_secs(1800), // 30 minutes
            enable_metrics: false,
        }
    }
    
    /// Create a normal configuration for regular play
    pub fn normal() -> Self {
        Self {
            mode: RelayMode::Normal {
                max_reservations: 20,
                max_circuits: 10,
            },
            max_bandwidth: Some(2_000_000), // 2 MB/s
            max_connections: 20,
            relay_timeout: Duration::from_secs(3600), // 1 hour
            enable_metrics: true,
        }
    }
    
    /// Create a provider configuration for earning credits
    pub fn provider() -> Self {
        Self {
            mode: RelayMode::Provider {
                max_reservations: 100,
                max_circuits: 50,
                require_credits: true,
            },
            max_bandwidth: Some(10_000_000), // 10 MB/s
            max_connections: 100,
            relay_timeout: Duration::from_secs(7200), // 2 hours
            enable_metrics: true,
        }
    }
    
    /// Check if relay service is enabled
    pub fn is_relay_enabled(&self) -> bool {
        !matches!(self.mode, RelayMode::Disabled)
    }
    
    /// Check if we provide relay service
    pub fn is_relay_provider(&self) -> bool {
        matches!(self.mode, RelayMode::Normal { .. } | RelayMode::Provider { .. })
    }
    
    /// Get maximum reservations
    pub fn max_reservations(&self) -> usize {
        match &self.mode {
            RelayMode::Disabled | RelayMode::Minimal => 0,
            RelayMode::Normal { max_reservations, .. } => *max_reservations,
            RelayMode::Provider { max_reservations, .. } => *max_reservations,
        }
    }
    
    /// Get maximum circuits
    pub fn max_circuits(&self) -> usize {
        match &self.mode {
            RelayMode::Disabled | RelayMode::Minimal => 0,
            RelayMode::Normal { max_circuits, .. } => *max_circuits,
            RelayMode::Provider { max_circuits, .. } => *max_circuits,
        }
    }
}

/// Relay usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelayStats {
    /// Total bytes relayed
    pub bytes_relayed: u64,
    /// Total connections relayed
    pub connections_relayed: u64,
    /// Current active circuits
    pub active_circuits: usize,
    /// Current reservations
    pub active_reservations: usize,
    /// Games relayed (for credit tracking)
    pub games_relayed: u64,
    /// Training data earned (in MB)
    pub training_data_earned: u64,
}

impl RelayStats {
    /// Calculate credits earned based on relay service
    pub fn calculate_credits(&self) -> u64 {
        // Simple credit calculation:
        // 1 credit per game relayed
        // 1 credit per 10 MB relayed
        // 1 credit per hour of service
        self.games_relayed + (self.bytes_relayed / 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_relay_modes() {
        let minimal = RelayConfig::minimal();
        assert_eq!(minimal.mode, RelayMode::Minimal);
        assert!(!minimal.is_relay_provider());
        
        let normal = RelayConfig::normal();
        assert!(normal.is_relay_provider());
        assert_eq!(normal.max_reservations(), 20);
        
        let provider = RelayConfig::provider();
        assert!(provider.is_relay_provider());
        assert_eq!(provider.max_circuits(), 50);
    }
    
    #[test]
    fn test_credit_calculation() {
        let mut stats = RelayStats::default();
        stats.games_relayed = 5;
        stats.bytes_relayed = 50_000_000; // 50 MB
        
        assert_eq!(stats.calculate_credits(), 10); // 5 games + 5 (50MB/10MB)
    }
}