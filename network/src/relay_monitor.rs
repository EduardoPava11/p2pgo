// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relay health monitoring and metrics collection

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
// Using core color constants directly

#[cfg(feature = "iroh")]
use iroh::Endpoint;

/// Health status of a relay
#[derive(Debug, Clone, PartialEq)]
pub enum RelayHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
    Restarting,
    Failed,
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
            (self.blue * 255.0) as u8
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
    
    /// Main monitoring loop
    #[allow(dead_code)]
    async fn monitor_loop(&self) -> Result<()> {
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
        Ok(())
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
        self.restart_counter.store(0, std::sync::atomic::Ordering::Relaxed);
        
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
                },
                Err(e) => {
                    tracing::error!("Relay task failed: {}", e);
                    
                    // Increment restart counter
                    let current_restarts = restart_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
            let restart_attempts = self.restart_counter.load(std::sync::atomic::Ordering::Relaxed);
            if restart_attempts >= self.max_restarts {
                state.set_failed();
                return Err(anyhow::anyhow!("Too many restart attempts ({})", restart_attempts));
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
        self.restart_counter.load(std::sync::atomic::Ordering::Relaxed)
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
        max_bandwidth_mbps: f64
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

/// Embedded relay server with restart capabilities
#[cfg(feature = "iroh")]
pub struct RestartableRelay {
    state: Arc<RwLock<RelayServiceState>>,
    port_manager: crate::port::PortManager,
    handle: Option<crate::net_util::CancelableTask>,
    health_sender: Option<tokio::sync::mpsc::UnboundedSender<RelayHealthEvent>>,
    capacity_sender: Option<tokio::sync::mpsc::UnboundedSender<RelayCapacityReport>>,
    capacity_interval: std::time::Duration,
    max_connections: usize,
    max_bandwidth_mbps: f64,
}

#[cfg(feature = "iroh")]
impl RestartableRelay {
    /// Create a new restartable relay
    pub fn new(port_manager: crate::port::PortManager) -> Self {
        Self {
            state: Arc::new(RwLock::new(RelayServiceState::new())),
            port_manager,
            handle: None,
            health_sender: None,
            capacity_sender: None,
            capacity_interval: std::time::Duration::from_secs(5),
            max_connections: 200, // Default value
            max_bandwidth_mbps: 10.0, // Default: 10 MB/s
        }
    }
    
    /// Set maximum connection limit
    pub fn connection_limit(mut self, limit: usize) -> Self {
        self.max_connections = limit;
        self
    }
    
    /// Set maximum bandwidth limit in Mbps
    pub fn bandwidth_limit(mut self, mbps: f64) -> Self {
        self.max_bandwidth_mbps = mbps;
        self
    }
    
    /// Set health event sender
    pub fn with_health_sender(mut self, sender: tokio::sync::mpsc::UnboundedSender<RelayHealthEvent>) -> Self {
        self.health_sender = Some(sender);
        self
    }
    
    /// Set capacity report sender
    pub fn with_capacity_sender(mut self, sender: tokio::sync::mpsc::UnboundedSender<RelayCapacityReport>) -> Self {
        self.capacity_sender = Some(sender);
        self
    }
    
    /// Send a health event
    async fn send_health_event(&self, status: RelayHealthStatus, latency_ms: Option<u64>) {
        if let Some(sender) = &self.health_sender {
            let port = {
                let state = self.state.read().await;
                state.listening_port
            };
            
            let last_restart = {
                let state = self.state.read().await;
                state.last_restart
            };

            let event = RelayHealthEvent {
                status,
                latency_ms,
                port,
                is_self_relay: true,
                last_restart,
                timestamp: Instant::now(),
            };
            
            let _ = sender.send(event);
        }
    }

    /// Send a capacity report
    async fn send_capacity_report(&self, current_connections: usize, current_bandwidth_mbps: f64) {
        if let Some(sender) = &self.capacity_sender {
            let report = RelayCapacityReport::new(
                current_connections,
                self.max_connections,
                current_bandwidth_mbps,
                self.max_bandwidth_mbps
            );
            
            let _ = sender.send(report);
        }
    }

    /// Start the embedded relay
    pub async fn start_embedded_relay(&mut self) -> Result<(u16, u16)> {
        tracing::info!("Starting embedded relay server...");
        
        // Get ports from port manager using the reusable socket option
        let (tcp_port, udp_port) = self.port_manager.get_relay_ports()?;
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.set_port(tcp_port);
            state.set_restarting();
        }
        
        // Send initial health event
        self.send_health_event(RelayHealthStatus::Restarting, None).await;
        
        // Send initial capacity report
        self.send_capacity_report(0, 0.0).await;
        
        // Create a new relay task with automatic restart
        let state_clone = self.state.clone();
        let health_sender = self.health_sender.clone();
        let capacity_sender = self.capacity_sender.clone();
        let max_connections = self.max_connections;
        let max_bandwidth_mbps = self.max_bandwidth_mbps;
        let capacity_interval = self.capacity_interval;
        
        let relay_task = spawn_cancelable!(
            name: "embedded_relay",
            max_restarts: 3,
            restart_delay_ms: 2000,
            window_secs: 30,
            |shutdown| async move {
                use iroh::relay;
                
                // Initialize TCP listener on provided port
                // Override environment values with instance values
                let max_connections = std::env::var("RELAY_MAX_CONNS")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(max_connections);
                
                let max_bandwidth = std::env::var("RELAY_MAX_MBPS")
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(max_bandwidth_mbps);
                    
                // Convert Mbps to bytes per second (for iroh API)
                let max_bandwidth_bps = (max_bandwidth * 1024.0 * 1024.0 / 8.0) as usize;
                
                tracing::info!("Configuring relay with limits: {} connections, {:.2} Mbps", 
                    max_connections, max_bandwidth);
                
                // Use socket2 to create a socket with SO_REUSEADDR and SO_REUSEPORT
                let tcp_addr = format!("0.0.0.0:{}", tcp_port).parse().unwrap();
                
                let mut relay_builder = relay::Server::builder()
                    .listen_addr(tcp_addr)
                    .advertise_addr(tcp_addr)
                    .connection_limit(max_connections)
                    .bandwidth_limit(max_bandwidth_bps);
                
                // If UDP port is different, configure it
                if tcp_port != udp_port {
                    relay_builder = relay_builder
                        .udp_listen_addr(format!("0.0.0.0:{}", udp_port).parse().unwrap())
                        .udp_advertise_addr(format!("0.0.0.0:{}", udp_port).parse().unwrap());
                };
                
                // Build and start the relay
                let relay = relay_builder.spawn().await?;
                
                // Update state to healthy now that relay is running
                {
                    let mut state = state_clone.write().await;
                    state.set_healthy();
                }
                
                // Send health event for UI
                if let Some(sender) = &health_sender {
                    let event = RelayHealthEvent {
                        status: RelayHealthStatus::Healthy,
                        latency_ms: Some(0), // We're the relay, so latency is 0
                        port: Some(tcp_port),
                        is_self_relay: true,
                        last_restart: None,
                        timestamp: Instant::now(),
                    };
                    let _ = sender.send(event);
                }
                
                tracing::info!("Embedded relay running on TCP:{} UDP:{}", tcp_port, udp_port);
                
                // Start a capacity monitoring task
                let relay_clone = relay.clone();
                let capacity_task = if let Some(capacity_sender) = &capacity_sender {
                    let sender = capacity_sender.clone();
                    let interval_duration = capacity_interval;
                    let local_max_connections = max_connections;
                    let local_max_bandwidth = max_bandwidth;
                    
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(interval_duration);
                        
                        loop {
                            interval.tick().await;
                            
                            // Get current connection count and bandwidth
                            let metrics = relay_clone.metrics();
                            let current_connections = metrics.current_connections();
                            
                            // Convert bandwidth from bytes/sec to Mbps
                            let bytes_per_sec = metrics.current_bandwidth().unwrap_or(0);
                            let current_mbps = (bytes_per_sec as f64) * 8.0 / 1024.0 / 1024.0;
                            
                            // Send capacity report
                            let report = RelayCapacityReport::new(
                                current_connections, 
                                local_max_connections,
                                current_mbps,
                                local_max_bandwidth
                            );
                            let _ = sender.send(report);
                        }
                    })
                } else {
                    tokio::spawn(async {})
                };
                
                // Wait for cancellation
                shutdown.cancelled().await;
                
                // Cancel capacity monitoring
                capacity_task.abort();
                
                // Shutdown relay gracefully
                relay.graceful_shutdown(Some(Duration::from_secs(5))).await?;
                
                Ok::<(), anyhow::Error>(())
            }
        );
        
        self.handle = Some(relay_task);
        
        Ok((tcp_port, udp_port))
    }
    
    /// Stop the embedded relay
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut task) = self.handle.take() {
            tracing::info!("Stopping embedded relay...");
            task.cancel().await?;
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.set_unreachable();
        }
        
        // Send health event
        self.send_health_event(RelayHealthStatus::Unreachable, None).await;
        
        Ok(())
    }
    
    /// Restart the embedded relay
    pub async fn restart(&mut self) -> Result<(u16, u16)> {
        // Update state
        {
            let mut state = self.state.write().await;
            state.set_restarting();
        }
        
        // Send health event
        self.send_health_event(RelayHealthStatus::Restarting, None).await;
        
        // Stop current relay
        if let Some(mut task) = self.handle.take() {
            tracing::info!("Stopping embedded relay for restart...");
            task.cancel().await?;
        }
        
        // Start a new relay
        self.start_embedded_relay().await
    }
}
