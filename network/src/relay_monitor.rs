// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relay health monitoring and metrics collection

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "iroh")]
use iroh::Endpoint;

/// Statistics for a single relay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStats {
    pub address: String,
    pub latency_ms: Option<u64>,
    pub last_checked: Instant,
    pub is_reachable: bool,
    pub is_home_relay: bool,
    pub connection_attempts: u64,
    pub successful_connections: u64,
}

impl RelayStats {
    pub fn new(address: String) -> Self {
        Self {
            address,
            latency_ms: None,
            last_checked: Instant::now(),
            is_reachable: false,
            is_home_relay: false,
            connection_attempts: 0,
            successful_connections: 0,
        }
    }
    
    /// Calculate the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.connection_attempts == 0 {
            0.0
        } else {
            (self.successful_connections as f64 / self.connection_attempts as f64) * 100.0
        }
    }
    
    /// Get a health status indicator
    pub fn health_status(&self) -> RelayHealth {
        if !self.is_reachable {
            return RelayHealth::Offline;
        }
        
        match self.latency_ms {
            Some(latency) if latency < 50 => RelayHealth::Excellent,
            Some(latency) if latency < 100 => RelayHealth::Good,
            Some(latency) if latency < 200 => RelayHealth::Fair,
            Some(_) => RelayHealth::Poor,
            None => RelayHealth::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayHealth {
    Excellent,  // < 50ms
    Good,       // < 100ms
    Fair,       // < 200ms
    Poor,       // >= 200ms
    Unknown,    // No latency data
    Offline,    // Not reachable
}

impl RelayHealth {
    pub fn emoji(&self) -> &'static str {
        match self {
            RelayHealth::Excellent => "🟢",
            RelayHealth::Good => "🟡",
            RelayHealth::Fair => "🟠",
            RelayHealth::Poor => "🔴",
            RelayHealth::Unknown => "❓",
            RelayHealth::Offline => "❌",
        }
    }
    
    pub fn description(&self) -> &'static str {
        match self {
            RelayHealth::Excellent => "Excellent",
            RelayHealth::Good => "Good",
            RelayHealth::Fair => "Fair",
            RelayHealth::Poor => "Poor",
            RelayHealth::Unknown => "Unknown",
            RelayHealth::Offline => "Offline",
        }
    }
}

/// Relay monitoring service
pub struct RelayMonitor {
    #[cfg(feature = "iroh")]
    endpoint: Endpoint,
    stats: Arc<RwLock<HashMap<String, RelayStats>>>,
    monitoring_active: Arc<std::sync::atomic::AtomicBool>,
}

impl RelayMonitor {
    /// Create a new relay monitor
    #[cfg(feature = "iroh")]
    pub fn new(endpoint: Endpoint, relay_addrs: Vec<String>) -> Self {
        let mut stats = HashMap::new();
        
        for addr in relay_addrs {
            stats.insert(addr.clone(), RelayStats::new(addr));
        }
        
        Self {
            endpoint,
            stats: Arc::new(RwLock::new(stats)),
            monitoring_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Create a stub relay monitor for non-iroh builds
    #[cfg(not(feature = "iroh"))]
    pub fn new_stub(relay_addrs: Vec<String>) -> Self {
        let mut stats = HashMap::new();
        
        for addr in relay_addrs {
            stats.insert(addr.clone(), RelayStats::new(addr));
        }
        
        Self {
            stats: Arc::new(RwLock::new(stats)),
            monitoring_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Start monitoring relays in the background
    pub fn start_monitoring(self) -> Arc<RwLock<HashMap<String, RelayStats>>> {
        let stats_clone = self.stats.clone();
        
        // Mark monitoring as active
        self.monitoring_active.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Spawn background monitoring task
        tokio::spawn(async move {
            tracing::info!("Starting relay monitoring");
            
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            while self.monitoring_active.load(std::sync::atomic::Ordering::Relaxed) {
                interval.tick().await;
                
                if let Err(e) = self.check_relays().await {
                    tracing::error!("Relay health check failed: {}", e);
                }
            }
            
            tracing::info!("Relay monitoring stopped");
        });
        
        stats_clone
    }
    
    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.monitoring_active.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Check all configured relays
    async fn check_relays(&self) -> Result<()> {
        #[cfg(feature = "iroh")]
        {
            tracing::debug!("Checking relay health");
            
            // Get current metrics from Iroh endpoint
            match self.endpoint.metrics().await {
                Ok(metrics) => {
                    let mut stats = self.stats.write().await;
                    
                    // Check home relay
                    if let Some(home_relay) = metrics.relay_home() {
                        let home_addr = home_relay.to_string();
                        tracing::debug!("Home relay: {}", home_addr);
                        
                        // Update all relays to mark home relay
                        for (addr, stat) in stats.iter_mut() {
                            let was_home = stat.is_home_relay;
                            stat.is_home_relay = addr.contains(&home_addr) || home_addr.contains(addr);
                            
                            if stat.is_home_relay && !was_home {
                                tracing::info!("🏠 {} is now the home relay", addr);
                            }
                        }
                    }
                    
                    // Update latencies from metrics
                    if let Some(relays) = metrics.relay_latencies() {
                        for (addr, latency) in relays {
                            let addr_str = addr.to_string();
                            
                            // Find matching relay stat (may be partial match)
                            let mut matched = false;
                            for (config_addr, stat) in stats.iter_mut() {
                                if config_addr.contains(&addr_str) || addr_str.contains(config_addr) {
                                    let latency_ms = latency.as_millis() as u64;
                                    stat.latency_ms = Some(latency_ms);
                                    stat.last_checked = Instant::now();
                                    stat.is_reachable = true;
                                    stat.successful_connections += 1;
                                    
                                    tracing::debug!("Relay {} latency: {}ms", config_addr, latency_ms);
                                    matched = true;
                                }
                            }
                            
                            if !matched {
                                tracing::debug!("Unknown relay in metrics: {}", addr_str);
                            }
                        }
                    }
                    
                    // Mark any relays not in metrics as potentially unreachable
                    let now = Instant::now();
                    for stat in stats.values_mut() {
                        if now.duration_since(stat.last_checked) > Duration::from_secs(120) {
                            if stat.is_reachable {
                                tracing::warn!("Relay {} appears to be unreachable", stat.address);
                                stat.is_reachable = false;
                            }
                        }
                        stat.connection_attempts += 1;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get endpoint metrics: {}", e);
                }
            }
        }
        
        #[cfg(not(feature = "iroh"))]
        {
            // Stub implementation - just mark all relays as reachable with fake latency
            let mut stats = self.stats.write().await;
            for stat in stats.values_mut() {
                stat.is_reachable = true;
                stat.latency_ms = Some(42); // Fake latency
                stat.last_checked = Instant::now();
                stat.successful_connections += 1;
                stat.connection_attempts += 1;
            }
        }
        
        Ok(())
    }
    
    /// Get current relay statistics
    pub async fn get_stats(&self) -> HashMap<String, RelayStats> {
        self.stats.read().await.clone()
    }
    
    /// Get statistics for a specific relay
    pub async fn get_relay_stats(&self, address: &str) -> Option<RelayStats> {
        self.stats.read().await.get(address).cloned()
    }
    
    /// Perform an immediate health check
    pub async fn check_now(&self) -> Result<()> {
        self.check_relays().await
    }
}

impl Drop for RelayMonitor {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_relay_stats_creation() {
        let stats = RelayStats::new("test-relay".to_string());
        assert_eq!(stats.address, "test-relay");
        assert!(!stats.is_reachable);
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.health_status(), RelayHealth::Offline);
    }
    
    #[test]
    fn test_success_rate_calculation() {
        let mut stats = RelayStats::new("test".to_string());
        stats.connection_attempts = 10;
        stats.successful_connections = 7;
        
        assert_eq!(stats.success_rate(), 70.0);
    }
    
    #[test]
    fn test_health_status() {
        let mut stats = RelayStats::new("test".to_string());
        stats.is_reachable = true;
        
        stats.latency_ms = Some(25);
        assert_eq!(stats.health_status(), RelayHealth::Excellent);
        
        stats.latency_ms = Some(75);
        assert_eq!(stats.health_status(), RelayHealth::Good);
        
        stats.latency_ms = Some(150);
        assert_eq!(stats.health_status(), RelayHealth::Fair);
        
        stats.latency_ms = Some(300);
        assert_eq!(stats.health_status(), RelayHealth::Poor);
    }
}
