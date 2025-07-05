// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relay health monitoring and metrics collection

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
// Using core color constants directly

// Removed iroh imports

/// Health status of a relay
#[derive(Debug, Clone, PartialEq)]
pub enum RelayHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
    Restarting,
    Failed,
}

impl std::fmt::Display for RelayHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelayHealthStatus::Healthy => write!(f, "Healthy"),
            RelayHealthStatus::Degraded => write!(f, "Degraded"),
            RelayHealthStatus::Unreachable => write!(f, "Unreachable"),
            RelayHealthStatus::Restarting => write!(f, "Restarting"),
            RelayHealthStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Health event for relay service monitoring
#[derive(Debug, Clone)]
pub struct RelayHealthEvent {
    /// Current health status
    pub status: RelayHealthStatus,
    /// Latency in milliseconds (if available)
    pub latency_ms: Option<u64>,
    /// Port the relay is listening on (if known)
    pub port: Option<u16>,
    /// Whether this is a self-hosted relay
    pub is_self_relay: bool,
    /// Last restart time (if available)
    pub last_restart: Option<Instant>,
    /// Timestamp when this event was created
    pub timestamp: Instant,
}

impl Default for RelayHealthEvent {
    fn default() -> Self {
        Self {
            status: RelayHealthStatus::Unreachable,
            latency_ms: None,
            port: None,
            is_self_relay: false,
            last_restart: None,
            timestamp: Instant::now(),
        }
    }
}

/// Statistics for a single relay
#[derive(Debug, Clone)]
pub struct RelayStats {
    pub address: String,
    pub latency_ms: Option<u64>,
    pub last_checked: Instant,
    pub is_reachable: bool,
    pub is_home_relay: bool,
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub health_status: RelayHealthStatus,
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
            health_status: RelayHealthStatus::Unreachable,
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
            Some(latency) if latency < 80 => RelayHealth::Good,
            Some(latency) if latency < 200 => RelayHealth::Fair,
            Some(_) => RelayHealth::Poor,
            None => RelayHealth::Unknown,
        }
    }

    /// Get status color based on latency thresholds (colorblind-safe)
    pub fn status_color(&self) -> RelayHealthColor {
        let status = match self.health_status() {
            RelayHealth::Excellent | RelayHealth::Good => RelayHealthStatus::Healthy,
            RelayHealth::Fair => RelayHealthStatus::Degraded,
            RelayHealth::Poor | RelayHealth::Offline => RelayHealthStatus::Unreachable,
            RelayHealth::Unknown => RelayHealthStatus::Degraded,
        };

        relay_health_color(&status)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayHealth {
    Excellent, // < 50ms
    Good,      // < 100ms
    Fair,      // < 200ms
    Poor,      // >= 200ms
    Unknown,   // No latency data
    Offline,   // Not reachable
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

/// Colorblind-safe colors for relay health status
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelayHealthColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl RelayHealthColor {
    /// Get as RGB array with values in 0.0-1.0 range
    pub fn as_rgb(&self) -> [f32; 3] {
        [self.red, self.green, self.blue]
    }

    /// Get as RGB array with values in 0-255 range
    pub fn as_rgb_u8(&self) -> [u8; 3] {
        [
            (self.red * 255.0) as u8,
            (self.green * 255.0) as u8,
            (self.blue * 255.0) as u8,
        ]
    }

    /// Create a RelayHealthColor from RGB [f32; 3] values
    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        Self {
            red: rgb[0],
            green: rgb[1],
            blue: rgb[2],
        }
    }
}

/// Relay monitoring service
pub struct RelayMonitor {
    // endpoint field removed
    stats: Arc<RwLock<HashMap<String, RelayStats>>>,
    monitoring_active: Arc<std::sync::atomic::AtomicBool>,
}

impl RelayMonitor {
    /// Create a new relay monitor
    pub fn new(relay_addrs: Vec<String>) -> Self {
        let mut stats = HashMap::new();

        for addr in relay_addrs {
            stats.insert(addr.clone(), RelayStats::new(addr));
        }

        Self {
            stats: Arc::new(RwLock::new(stats)),
            monitoring_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Create a stub relay monitor (deprecated - use new() instead)
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
        self.monitoring_active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Spawn background monitoring task
        tokio::spawn(async move {
            tracing::info!("Starting relay monitoring");

            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while self
                .monitoring_active
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                interval.tick().await;

                if let Err(e) = self.check_relays().await {
                    tracing::error!("Relay health check failed: {}", e);
                }
            }

            tracing::info!("Relay monitoring stopped");
        });

        stats_clone
    }

    /// Main monitoring loop
    #[allow(dead_code)]
    async fn monitor_loop(&self) -> Result<()> {
        tracing::info!("Starting relay monitoring");

        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while self
            .monitoring_active
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            if let Err(e) = self.check_relays().await {
                tracing::error!("Relay health check failed: {}", e);
            }
        }

        tracing::info!("Relay monitoring stopped");
        Ok(())
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        self.monitoring_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check all configured relays
    async fn check_relays(&self) -> Result<()> {
        tracing::debug!("Checking relay health");

        // For now, simulate relay health checks
        // In a real implementation, this would:
        // 1. Query libp2p swarm for relay server status
        // 2. Ping relay servers to measure latency
        // 3. Check active relay reservations

        let mut stats = self.stats.write().await;

        for stat in stats.values_mut() {
            // Simulate health check
            stat.is_reachable = true;
            stat.latency_ms = Some(20 + (rand::random::<u64>() % 80)); // 20-100ms
            stat.last_checked = Instant::now();
            stat.successful_connections += 1;
            stat.connection_attempts += 1;

            // Randomly mark one relay as home
            if stat.is_home_relay == false && rand::random::<f32>() < 0.1 {
                stat.is_home_relay = true;
                tracing::info!("🏠 {} is now the home relay", stat.address);
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

/// Relay service status tracking with restart capability
pub struct RelayServiceState {
    pub status: RelayHealthStatus,
    pub restart_attempts: u32,
    pub last_restart: Option<Instant>,
    pub start_time: Instant,
    pub version: String,
    pub listening_port: Option<u16>,
    pub auto_restart_enabled: bool,
    health_checks: u32,
    healthy_checks: u32,
    last_update: Instant,
    restart_callback: Option<Box<dyn Fn() -> Result<()> + Send + Sync>>,
}

impl RelayServiceState {
    /// Create a new relay service state tracker
    pub fn new() -> Self {
        Self {
            status: RelayHealthStatus::Unreachable,
            restart_attempts: 0,
            last_restart: None,
            start_time: Instant::now(),
            version: "0.0.0".to_string(),
            listening_port: None,
            auto_restart_enabled: true,
            health_checks: 0,
            healthy_checks: 0,
            last_update: Instant::now(),
            restart_callback: None,
        }
    }

    /// Register a restart callback
    pub fn with_restart_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        self.restart_callback = Some(Box::new(callback));
        self
    }

    /// Set the relay version
    pub fn set_version(&mut self, version: String) {
        self.version = version;
    }

    /// Set the listening port
    pub fn set_port(&mut self, port: u16) {
        self.listening_port = Some(port);
    }

    /// Mark the relay as healthy
    pub fn set_healthy(&mut self) {
        self.status = RelayHealthStatus::Healthy;
        self.healthy_checks += 1;
        self.health_checks += 1;
        self.last_update = Instant::now();
    }

    /// Mark the relay as degraded
    pub fn set_degraded(&mut self) {
        self.status = RelayHealthStatus::Degraded;
        self.health_checks += 1;
        self.last_update = Instant::now();
    }

    /// Mark the relay as unreachable
    pub fn set_unreachable(&mut self) {
        self.status = RelayHealthStatus::Unreachable;
        self.health_checks += 1;
        self.last_update = Instant::now();
    }

    /// Mark the relay as restarting
    pub fn set_restarting(&mut self) {
        self.status = RelayHealthStatus::Restarting;
        self.last_restart = Some(Instant::now());
        self.health_checks += 1;
        self.last_update = Instant::now();
    }

    /// Mark the relay as failed
    pub fn set_failed(&mut self) {
        self.status = RelayHealthStatus::Failed;
        self.health_checks += 1;
        self.last_update = Instant::now();
    }

    /// Attempt to restart the relay
    pub fn restart(&mut self) -> Result<()> {
        self.set_restarting();
        self.restart_attempts += 1;

        if let Some(restart_callback) = &self.restart_callback {
            restart_callback()
        } else {
            Ok(())
        }
    }

    /// Get health metrics for the relay
    pub fn health_metrics(&self) -> RelayHealthEvent {
        RelayHealthEvent {
            status: self.status.clone(),
            latency_ms: None, // We'll need to add latency tracking
            port: self.listening_port,
            is_self_relay: true,
            last_restart: self.last_restart,
            timestamp: Instant::now(),
        }
    }

    /// Health percentage (0-100)
    pub fn health_percentage(&self) -> u8 {
        if self.health_checks == 0 {
            return 0;
        }

        let percentage = (self.healthy_checks as f64 / self.health_checks as f64) * 100.0;
        percentage.min(100.0) as u8
    }
}

/// Restartable relay manager for embedded relay operation
pub struct RestartableRelay {
    state: Arc<RwLock<RelayServiceState>>,
    shutdown_signal: Option<tokio::sync::oneshot::Sender<()>>,
    handle: Option<tokio::task::JoinHandle<Result<()>>>,
    port_manager: crate::port::PortManager,
    max_restarts: u32,
    restart_counter: Arc<std::sync::atomic::AtomicU32>,
}

impl RestartableRelay {
    /// Create a new restartable relay manager
    pub fn new(port_manager: crate::port::PortManager) -> Self {
        Self {
            state: Arc::new(RwLock::new(RelayServiceState::new())),
            shutdown_signal: None,
            handle: None,
            port_manager,
            max_restarts: 3,
            restart_counter: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    /// Get a reference to the relay state
    pub fn state(&self) -> Arc<RwLock<RelayServiceState>> {
        self.state.clone()
    }

    /// Start the relay with the given startup function
    pub async fn start<F, Fut>(&mut self, start_fn: F) -> Result<u16>
    where
        F: Fn(u16, tokio::sync::oneshot::Receiver<()>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // Clean up any existing relay
        if self.shutdown_signal.is_some() || self.handle.is_some() {
            self.stop().await?;
        }

        // Get a port for the relay (use saved port if available)
        let port = self.port_manager.get_relay_port()?;

        // Create shutdown channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.shutdown_signal = Some(tx);

        // Update state
        {
            let mut state = self.state.write().await;
            state.set_port(port);
            state.status = RelayHealthStatus::Restarting; // Starting up
            state.start_time = Instant::now();
            state.last_restart = Some(Instant::now());
        }

        // Reset restart counter
        self.restart_counter
            .store(0, std::sync::atomic::Ordering::Relaxed);

        // Create a cancellation token for the task
        let state_clone = self.state.clone();
        let restart_counter = self.restart_counter.clone();
        let max_restarts = self.max_restarts;

        // Start the relay task
        self.handle = Some(tokio::spawn(async move {
            // Start the relay
            let relay_result = start_fn(port, rx).await;

            // Update state based on result
            let mut state = state_clone.write().await;

            match &relay_result {
                Ok(_) => {
                    tracing::info!("Relay task completed successfully");
                    state.status = RelayHealthStatus::Healthy;
                }
                Err(e) => {
                    tracing::error!("Relay task failed: {}", e);

                    // Increment restart counter
                    let current_restarts =
                        restart_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    state.restart_attempts = current_restarts + 1;

                    if state.restart_attempts >= max_restarts {
                        state.status = RelayHealthStatus::Failed;
                        state.auto_restart_enabled = false;
                    } else {
                        state.status = RelayHealthStatus::Unreachable;
                    }
                }
            }

            relay_result
        }));

        // Return the port being used
        Ok(port)
    }

    /// Stop the relay
    pub async fn stop(&mut self) -> Result<()> {
        // Send shutdown signal if available
        if let Some(signal) = self.shutdown_signal.take() {
            let _ = signal.send(());
        }

        // Wait for the handle to finish
        if let Some(handle) = self.handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        // Update state
        let mut state = self.state.write().await;
        state.status = RelayHealthStatus::Unreachable;

        Ok(())
    }

    /// Restart the relay with the given startup function
    pub async fn restart<F, Fut>(&mut self, start_fn: F) -> Result<u16>
    where
        F: Fn(u16, tokio::sync::oneshot::Receiver<()>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        // Update state
        {
            let mut state = self.state.write().await;

            // Check restart counter
            let restart_attempts = self
                .restart_counter
                .load(std::sync::atomic::Ordering::Relaxed);
            if restart_attempts >= self.max_restarts {
                state.set_failed();
                return Err(anyhow::anyhow!(
                    "Too many restart attempts ({})",
                    restart_attempts
                ));
            }

            state.set_restarting();
        }

        // Stop the current relay
        self.stop().await?;

        // Small delay for full shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start a new relay
        self.start(start_fn).await
    }

    /// Set the maximum number of restart attempts
    pub fn set_max_restarts(&mut self, max_restarts: u32) {
        self.max_restarts = max_restarts;
    }

    /// Get the current restart counter
    pub fn restart_count(&self) -> u32 {
        self.restart_counter
            .load(std::sync::atomic::Ordering::Relaxed)
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

/// Get color for relay health status (colorblind-safe)
pub fn relay_health_color(status: &RelayHealthStatus) -> RelayHealthColor {
    use p2pgo_core::color_constants::relay_status;

    let rgb = match status {
        RelayHealthStatus::Healthy => relay_status::HEALTHY,
        RelayHealthStatus::Degraded => relay_status::DEGRADED,
        RelayHealthStatus::Unreachable => relay_status::OFFLINE,
        RelayHealthStatus::Restarting => relay_status::RESTARTING,
        RelayHealthStatus::Failed => relay_status::ERROR,
    };

    RelayHealthColor::from_rgb(rgb)
}

/// Relay capacity report for UI metrics
#[derive(Debug, Clone)]
pub struct RelayCapacityReport {
    pub current_connections: usize,
    pub max_connections: usize,
    pub current_bandwidth_mbps: f64,
    pub max_bandwidth_mbps: f64,
    pub timestamp: std::time::Instant,
}

impl Default for RelayCapacityReport {
    fn default() -> Self {
        Self {
            current_connections: 0,
            max_connections: 0,
            current_bandwidth_mbps: 0.0,
            max_bandwidth_mbps: 0.0,
            timestamp: std::time::Instant::now(),
        }
    }
}

impl RelayCapacityReport {
    pub fn new(
        current_connections: usize,
        max_connections: usize,
        current_bandwidth_mbps: f64,
        max_bandwidth_mbps: f64,
    ) -> Self {
        Self {
            current_connections,
            max_connections,
            current_bandwidth_mbps,
            max_bandwidth_mbps,
            timestamp: std::time::Instant::now(),
        }
    }
}
