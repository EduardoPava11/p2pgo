//! Health check and monitoring endpoints for P2P network

use serde::{Serialize, Deserialize};
use std::time::{Instant, SystemTime};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Overall system health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is healthy and operational
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy and may not function properly
    Unhealthy,
}

/// Component health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Human-readable message
    pub message: String,
    /// Last check timestamp
    pub last_check: SystemTime,
    /// Additional metadata
    #[serde(flatten)]
    pub metadata: serde_json::Value,
}

/// Network connectivity health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    /// Number of connected peers
    pub connected_peers: usize,
    /// Number of relay connections
    pub relay_connections: usize,
    /// Average ping latency to peers
    pub avg_latency_ms: Option<f64>,
    /// Packet loss percentage
    pub packet_loss_percent: f32,
    /// NAT status
    pub nat_status: String,
    /// Bootstrap status
    pub bootstrap_complete: bool,
}

/// Game subsystem health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameHealth {
    /// Number of active games
    pub active_games: usize,
    /// Number of pending moves
    pub pending_moves: usize,
    /// Average game sync latency
    pub avg_sync_latency_ms: Option<f64>,
    /// Number of sync errors in last hour
    pub sync_errors_last_hour: usize,
}

/// Resource usage health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceHealth {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Open file descriptors
    pub open_fds: usize,
    /// Thread count
    pub thread_count: usize,
}

/// Complete health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Overall status
    pub status: HealthStatus,
    /// Timestamp of check
    pub timestamp: SystemTime,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Individual component health
    pub components: HashMap<String, ComponentHealth>,
    /// Network health details
    pub network: NetworkHealth,
    /// Game health details
    pub game: GameHealth,
    /// Resource health details
    pub resources: ResourceHealth,
}

/// Health check manager
pub struct HealthManager {
    start_time: Instant,
    components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    network_stats: Arc<RwLock<NetworkHealth>>,
    game_stats: Arc<RwLock<GameHealth>>,
}

impl HealthManager {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            components: Arc::new(RwLock::new(HashMap::new())),
            network_stats: Arc::new(RwLock::new(NetworkHealth {
                connected_peers: 0,
                relay_connections: 0,
                avg_latency_ms: None,
                packet_loss_percent: 0.0,
                nat_status: "Unknown".to_string(),
                bootstrap_complete: false,
            })),
            game_stats: Arc::new(RwLock::new(GameHealth {
                active_games: 0,
                pending_moves: 0,
                avg_sync_latency_ms: None,
                sync_errors_last_hour: 0,
            })),
        }
    }
    
    /// Update component health
    pub fn update_component(&self, name: &str, status: HealthStatus, message: String) {
        let component = ComponentHealth {
            name: name.to_string(),
            status,
            message,
            last_check: SystemTime::now(),
            metadata: serde_json::json!({}),
        };
        
        if let Ok(mut components) = self.components.write() {
            components.insert(name.to_string(), component);
        }
    }
    
    /// Update network statistics
    pub fn update_network_stats<F>(&self, updater: F) 
    where
        F: FnOnce(&mut NetworkHealth)
    {
        if let Ok(mut stats) = self.network_stats.write() {
            updater(&mut stats);
        }
    }
    
    /// Update game statistics
    pub fn update_game_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut GameHealth)
    {
        if let Ok(mut stats) = self.game_stats.write() {
            updater(&mut stats);
        }
    }
    
    /// Perform health check
    pub fn check_health(&self) -> HealthCheckResponse {
        let components = self.components.read().unwrap().clone();
        let network = self.network_stats.read().unwrap().clone();
        let game = self.game_stats.read().unwrap().clone();
        
        // Calculate overall status
        let mut overall_status = HealthStatus::Healthy;
        
        // Check component health
        for (_, component) in &components {
            match component.status {
                HealthStatus::Unhealthy => overall_status = HealthStatus::Unhealthy,
                HealthStatus::Degraded => {
                    if overall_status != HealthStatus::Unhealthy {
                        overall_status = HealthStatus::Degraded;
                    }
                }
                _ => {}
            }
        }
        
        // Check network health
        if network.connected_peers == 0 && !network.bootstrap_complete {
            overall_status = HealthStatus::Unhealthy;
        } else if network.connected_peers < 3 {
            if overall_status != HealthStatus::Unhealthy {
                overall_status = HealthStatus::Degraded;
            }
        }
        
        // Get resource usage
        let resources = self.get_resource_usage();
        
        HealthCheckResponse {
            status: overall_status,
            timestamp: SystemTime::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            components,
            network,
            game,
            resources,
        }
    }
    
    /// Get current resource usage
    fn get_resource_usage(&self) -> ResourceHealth {
        // Get memory usage
        let memory_mb = if let Ok(mem_info) = sys_info::mem_info() {
            (mem_info.total - mem_info.avail) as f64 / 1024.0
        } else {
            0.0
        };
        
        // Get CPU usage (simplified)
        let cpu_percent = if let Ok(loadavg) = sys_info::loadavg() {
            (loadavg.one * 100.0) as f32
        } else {
            0.0
        };
        
        ResourceHealth {
            memory_mb,
            cpu_percent,
            open_fds: 0, // Would need platform-specific code
            thread_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }
    
    /// Check if system is ready for traffic
    pub fn is_ready(&self) -> bool {
        let network = self.network_stats.read().unwrap();
        network.bootstrap_complete && network.connected_peers > 0
    }
    
    /// Get liveness status (is the process alive)
    pub fn is_alive(&self) -> bool {
        // Simple liveness - we're running
        true
    }
}

/// HTTP health check endpoint handler
pub async fn health_handler(
    health_manager: Arc<HealthManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let health = health_manager.check_health();
    let status_code = match health.status {
        HealthStatus::Healthy => warp::http::StatusCode::OK,
        HealthStatus::Degraded => warp::http::StatusCode::OK,
        HealthStatus::Unhealthy => warp::http::StatusCode::SERVICE_UNAVAILABLE,
    };
    
    Ok(warp::reply::with_status(
        warp::reply::json(&health),
        status_code,
    ))
}

/// HTTP readiness endpoint handler
pub async fn ready_handler(
    health_manager: Arc<HealthManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if health_manager.is_ready() {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "ready": true,
                "message": "Service is ready to accept traffic"
            })),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "ready": false,
                "message": "Service is not ready yet"
            })),
            warp::http::StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

/// HTTP liveness endpoint handler
pub async fn alive_handler(
    health_manager: Arc<HealthManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if health_manager.is_alive() {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "alive": true
            })),
            warp::http::StatusCode::OK,
        ))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "alive": false
            })),
            warp::http::StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_manager_creation() {
        let health = HealthManager::new();
        assert!(health.is_alive());
        assert!(!health.is_ready()); // Not ready until bootstrap
    }
    
    #[test]
    fn test_component_update() {
        let health = HealthManager::new();
        health.update_component("test", HealthStatus::Healthy, "All good".to_string());
        
        let check = health.check_health();
        assert!(check.components.contains_key("test"));
        assert_eq!(check.components["test"].status, HealthStatus::Healthy);
    }
    
    #[test]
    fn test_overall_status_calculation() {
        let health = HealthManager::new();
        
        // All healthy
        health.update_component("comp1", HealthStatus::Healthy, "OK".to_string());
        health.update_component("comp2", HealthStatus::Healthy, "OK".to_string());
        assert_eq!(health.check_health().status, HealthStatus::Healthy);
        
        // One degraded
        health.update_component("comp2", HealthStatus::Degraded, "Slow".to_string());
        assert_eq!(health.check_health().status, HealthStatus::Degraded);
        
        // One unhealthy
        health.update_component("comp1", HealthStatus::Unhealthy, "Failed".to_string());
        assert_eq!(health.check_health().status, HealthStatus::Unhealthy);
    }
}