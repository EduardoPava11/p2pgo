//! Smart load balancer with real-time metrics and health integration

use anyhow::{anyhow, Result};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, info};

use crate::health::HealthManager;

/// Smart load balancer with capacity-aware routing
pub struct SmartLoadBalancer {
    /// Real-time relay metrics
    relay_metrics: Arc<RwLock<HashMap<PeerId, RelayMetrics>>>,
    /// Load balancing algorithm
    algorithm: LoadBalancingAlgorithm,
    /// Health check integration
    health_checker: Arc<HealthManager>,
    /// Geographic routing configuration
    geo_config: GeoRoutingConfig,
    /// Session affinity store
    session_affinity: Arc<RwLock<HashMap<String, PeerId>>>,
    /// Performance history for learning
    performance_history: Arc<RwLock<HashMap<PeerId, PerformanceHistory>>>,
}

/// Load balancing algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingAlgorithm {
    /// Round robin with weights based on capacity
    WeightedRoundRobin,
    /// Route to relay with least active connections
    LeastConnections,
    /// Route based on available capacity (CPU, memory, bandwidth)
    CapacityAware,
    /// Route based on geographic latency
    GeographicLatency,
    /// Adaptive algorithm that learns from performance
    Adaptive,
}

/// Real-time metrics for a relay
#[derive(Debug, Clone)]
pub struct RelayMetrics {
    /// Unique identifier for the relay
    pub peer_id: PeerId,
    /// Number of active connections
    pub active_connections: u32,
    /// Maximum connections this relay can handle
    pub max_connections: u32,
    /// Current CPU usage percentage (0.0 - 1.0)
    pub cpu_usage: f32,
    /// Current memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Maximum memory available in bytes
    pub max_memory_bytes: u64,
    /// Current bandwidth utilization in bytes/sec
    pub bandwidth_usage_bps: u64,
    /// Maximum bandwidth capacity in bytes/sec
    pub max_bandwidth_bps: u64,
    /// Average latency to this relay in milliseconds
    pub avg_latency_ms: f32,
    /// Geographic location
    pub location: GeoLocation,
    /// Last update timestamp
    pub last_updated: Instant,
    /// Connection success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Current load score (0.0 - 1.0, lower is better)
    pub load_score: f32,
}

/// Geographic location for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Country code
    pub country: String,
    /// Region/state
    pub region: String,
    /// City
    pub city: String,
}

/// Geographic routing configuration
#[derive(Debug, Clone)]
pub struct GeoRoutingConfig {
    /// Prefer relays within this distance (km)
    pub preferred_distance_km: f64,
    /// Maximum acceptable distance (km)
    pub max_distance_km: f64,
    /// Weight for geographic preference (0.0 - 1.0)
    pub geo_weight: f32,
}

/// Performance history for adaptive learning
#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    /// Recent connection attempts
    pub recent_attempts: Vec<ConnectionAttempt>,
    /// Success rate over time
    pub success_rate_history: Vec<(Instant, f32)>,
    /// Latency history
    pub latency_history: Vec<(Instant, f32)>,
    /// Load history
    pub load_history: Vec<(Instant, f32)>,
}

/// Record of a connection attempt
#[derive(Debug, Clone)]
pub struct ConnectionAttempt {
    /// When the attempt was made
    pub timestamp: Instant,
    /// Whether it succeeded
    pub success: bool,
    /// Latency if successful
    pub latency_ms: Option<f32>,
    /// Error reason if failed
    pub error: Option<String>,
}

/// Selection result from load balancer
#[derive(Debug, Clone)]
pub struct SelectionResult {
    /// Selected relay peer ID
    pub peer_id: PeerId,
    /// Reason for selection
    pub reason: SelectionReason,
    /// Confidence in selection (0.0 - 1.0)
    pub confidence: f32,
    /// Alternative options considered
    pub alternatives: Vec<PeerId>,
}

/// Reason for relay selection
#[derive(Debug, Clone)]
pub enum SelectionReason {
    LowestLoad,
    BestLatency,
    GeographicProximity,
    SessionAffinity,
    OnlyHealthyOption,
    AdaptiveLearning,
}

impl Default for GeoRoutingConfig {
    fn default() -> Self {
        Self {
            preferred_distance_km: 500.0,
            max_distance_km: 2000.0,
            geo_weight: 0.3,
        }
    }
}

impl Default for PerformanceHistory {
    fn default() -> Self {
        Self {
            recent_attempts: Vec::new(),
            success_rate_history: Vec::new(),
            latency_history: Vec::new(),
            load_history: Vec::new(),
        }
    }
}

impl SmartLoadBalancer {
    /// Create a new smart load balancer
    pub fn new(algorithm: LoadBalancingAlgorithm, health_checker: Arc<HealthManager>) -> Self {
        Self {
            relay_metrics: Arc::new(RwLock::new(HashMap::new())),
            algorithm,
            health_checker,
            geo_config: GeoRoutingConfig::default(),
            session_affinity: Arc::new(RwLock::new(HashMap::new())),
            performance_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Select the best relay for a client
    pub fn select_relay(
        &self,
        client_location: Option<GeoLocation>,
        session_id: Option<String>,
    ) -> Result<SelectionResult> {
        // Get healthy relays from metrics (since HealthCheckService doesn't have get_healthy_relays)
        let relay_metrics = self.relay_metrics.read().unwrap();
        let healthy_relays: Vec<PeerId> = relay_metrics
            .iter()
            .filter(|(_, metrics)| {
                // Consider relay healthy if it's recently updated and has reasonable load
                metrics.last_updated.elapsed() < Duration::from_secs(60) && metrics.load_score < 0.9
            })
            .map(|(peer_id, _)| *peer_id)
            .collect();
        drop(relay_metrics);

        if healthy_relays.is_empty() {
            return Err(anyhow!("No healthy relays available"));
        }

        // Check for session affinity
        if let Some(session_id) = &session_id {
            if let Some(affinity_peer) = self.get_session_affinity(session_id) {
                if healthy_relays.contains(&affinity_peer) {
                    return Ok(SelectionResult {
                        peer_id: affinity_peer,
                        reason: SelectionReason::SessionAffinity,
                        confidence: 0.9,
                        alternatives: healthy_relays
                            .into_iter()
                            .filter(|p| *p != affinity_peer)
                            .collect(),
                    });
                }
            }
        }

        // Get current metrics for healthy relays
        let relay_metrics = self.relay_metrics.read().unwrap();
        let mut candidates: Vec<(PeerId, f32)> = healthy_relays
            .into_iter()
            .filter_map(|peer_id| {
                relay_metrics.get(&peer_id).map(|metrics| {
                    let score = self.calculate_selection_score(metrics, client_location.as_ref());
                    (peer_id, score)
                })
            })
            .collect();

        if candidates.is_empty() {
            return Err(anyhow!("No relay metrics available for healthy relays"));
        }

        // Sort by score (lower is better)
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let selected = candidates[0].0;
        let reason = self.determine_selection_reason(&candidates[0], client_location.as_ref());
        let confidence = self.calculate_confidence(&candidates);

        // Update session affinity if provided
        if let Some(session_id) = session_id {
            self.set_session_affinity(session_id, selected);
        }

        Ok(SelectionResult {
            peer_id: selected,
            reason,
            confidence,
            alternatives: candidates
                .into_iter()
                .skip(1)
                .map(|(peer, _)| peer)
                .collect(),
        })
    }

    /// Calculate selection score for a relay
    fn calculate_selection_score(
        &self,
        metrics: &RelayMetrics,
        client_location: Option<&GeoLocation>,
    ) -> f32 {
        match self.algorithm {
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                // Score based on connection capacity
                let capacity_ratio =
                    metrics.active_connections as f32 / metrics.max_connections as f32;
                capacity_ratio + (metrics.cpu_usage * 0.3) + (metrics.load_score * 0.2)
            }
            LoadBalancingAlgorithm::LeastConnections => metrics.active_connections as f32,
            LoadBalancingAlgorithm::CapacityAware => {
                let cpu_score = metrics.cpu_usage;
                let memory_score =
                    (metrics.memory_usage_bytes as f32) / (metrics.max_memory_bytes as f32);
                let bandwidth_score =
                    (metrics.bandwidth_usage_bps as f32) / (metrics.max_bandwidth_bps as f32);
                let connection_score =
                    (metrics.active_connections as f32) / (metrics.max_connections as f32);

                (cpu_score + memory_score + bandwidth_score + connection_score) / 4.0
            }
            LoadBalancingAlgorithm::GeographicLatency => {
                let mut score = metrics.avg_latency_ms / 100.0; // Normalize latency

                if let Some(client_loc) = client_location {
                    let distance = self.calculate_distance(&metrics.location, client_loc);
                    score += (distance / 1000.0) as f32 * self.geo_config.geo_weight;
                    // Distance weight
                }

                score
            }
            LoadBalancingAlgorithm::Adaptive => {
                self.calculate_adaptive_score(metrics, client_location)
            }
        }
    }

    /// Calculate adaptive score using performance history
    fn calculate_adaptive_score(
        &self,
        metrics: &RelayMetrics,
        client_location: Option<&GeoLocation>,
    ) -> f32 {
        let history = self.performance_history.read().unwrap();

        let base_score = self.calculate_selection_score_for_algorithm(
            metrics,
            client_location,
            LoadBalancingAlgorithm::CapacityAware,
        );

        if let Some(perf_history) = history.get(&metrics.peer_id) {
            // Adjust score based on historical performance
            let success_adjustment = 1.0 - perf_history.recent_success_rate();
            let latency_adjustment = perf_history.recent_avg_latency() / 100.0;

            base_score + (success_adjustment * 0.3) + (latency_adjustment * 0.2)
        } else {
            // No history, use slightly higher score to encourage exploration
            base_score + 0.1
        }
    }

    /// Calculate score for a specific algorithm
    fn calculate_selection_score_for_algorithm(
        &self,
        metrics: &RelayMetrics,
        client_location: Option<&GeoLocation>,
        algorithm: LoadBalancingAlgorithm,
    ) -> f32 {
        let _original_algorithm = self.algorithm;
        let temp_balancer = self.clone_with_algorithm(algorithm);
        temp_balancer.calculate_selection_score(metrics, client_location)
    }

    /// Clone with different algorithm (helper for adaptive scoring)
    fn clone_with_algorithm(&self, algorithm: LoadBalancingAlgorithm) -> SmartLoadBalancer {
        SmartLoadBalancer {
            relay_metrics: self.relay_metrics.clone(),
            algorithm,
            health_checker: self.health_checker.clone(),
            geo_config: self.geo_config.clone(),
            session_affinity: self.session_affinity.clone(),
            performance_history: self.performance_history.clone(),
        }
    }

    /// Update metrics for a relay
    pub fn update_relay_metrics(&self, metrics: RelayMetrics) {
        let mut relay_metrics = self.relay_metrics.write().unwrap();
        relay_metrics.insert(metrics.peer_id, metrics);
    }

    /// Remove metrics for a relay (when it goes offline)
    pub fn remove_relay(&self, peer_id: &PeerId) {
        let mut relay_metrics = self.relay_metrics.write().unwrap();
        relay_metrics.remove(peer_id);
    }

    /// Record connection attempt for adaptive learning
    pub fn record_connection_attempt(&self, peer_id: PeerId, attempt: ConnectionAttempt) {
        let mut history = self.performance_history.write().unwrap();
        let perf_history = history.entry(peer_id).or_default();

        perf_history.recent_attempts.push(attempt.clone());

        // Keep only recent attempts (last 100)
        if perf_history.recent_attempts.len() > 100 {
            perf_history.recent_attempts.remove(0);
        }

        // Update success rate history
        let recent_success_rate = perf_history.recent_success_rate();
        perf_history
            .success_rate_history
            .push((attempt.timestamp, recent_success_rate));

        // Update latency history if successful
        if let Some(latency) = attempt.latency_ms {
            perf_history
                .latency_history
                .push((attempt.timestamp, latency));
        }

        // Cleanup old history (keep last 24 hours)
        let cutoff = Instant::now() - Duration::from_secs(24 * 3600);
        perf_history
            .success_rate_history
            .retain(|(timestamp, _)| *timestamp > cutoff);
        perf_history
            .latency_history
            .retain(|(timestamp, _)| *timestamp > cutoff);
    }

    /// Get session affinity mapping
    fn get_session_affinity(&self, session_id: &str) -> Option<PeerId> {
        self.session_affinity
            .read()
            .unwrap()
            .get(session_id)
            .copied()
    }

    /// Set session affinity mapping
    fn set_session_affinity(&self, session_id: String, peer_id: PeerId) {
        self.session_affinity
            .write()
            .unwrap()
            .insert(session_id, peer_id);
    }

    /// Calculate geographic distance between two locations (Haversine formula)
    fn calculate_distance(&self, loc1: &GeoLocation, loc2: &GeoLocation) -> f64 {
        let lat1_rad = loc1.lat.to_radians();
        let lat2_rad = loc2.lat.to_radians();
        let delta_lat = (loc2.lat - loc1.lat).to_radians();
        let delta_lng = (loc2.lng - loc1.lng).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        6371.0 * c // Earth's radius in kilometers
    }

    /// Determine the reason for selection
    fn determine_selection_reason(
        &self,
        _selected: &(PeerId, f32),
        client_location: Option<&GeoLocation>,
    ) -> SelectionReason {
        match self.algorithm {
            LoadBalancingAlgorithm::WeightedRoundRobin => SelectionReason::LowestLoad,
            LoadBalancingAlgorithm::LeastConnections => SelectionReason::LowestLoad,
            LoadBalancingAlgorithm::CapacityAware => SelectionReason::LowestLoad,
            LoadBalancingAlgorithm::GeographicLatency => {
                if client_location.is_some() {
                    SelectionReason::GeographicProximity
                } else {
                    SelectionReason::BestLatency
                }
            }
            LoadBalancingAlgorithm::Adaptive => SelectionReason::AdaptiveLearning,
        }
    }

    /// Calculate confidence in selection
    fn calculate_confidence(&self, candidates: &[(PeerId, f32)]) -> f32 {
        if candidates.len() < 2 {
            return 1.0;
        }

        let best_score = candidates[0].1;
        let second_best_score = candidates[1].1;

        // Higher confidence when there's a clear winner
        let score_diff = second_best_score - best_score;
        (score_diff * 2.0).min(1.0).max(0.3)
    }

    /// Get current load balancing statistics
    pub fn get_statistics(&self) -> LoadBalancerStatistics {
        let relay_metrics = self.relay_metrics.read().unwrap();
        let total_relays = relay_metrics.len();
        let total_connections: u32 = relay_metrics.values().map(|m| m.active_connections).sum();
        let avg_cpu_usage: f32 = if total_relays > 0 {
            relay_metrics.values().map(|m| m.cpu_usage).sum::<f32>() / total_relays as f32
        } else {
            0.0
        };

        let session_affinity = self.session_affinity.read().unwrap();
        let active_sessions = session_affinity.len();

        LoadBalancerStatistics {
            total_relays,
            total_connections,
            avg_cpu_usage,
            active_sessions,
            algorithm: self.algorithm,
        }
    }

    /// Start metrics collection background task
    pub async fn start_metrics_collection(&self) -> Result<()> {
        let metrics = self.relay_metrics.clone();
        let _health_checker = self.health_checker.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                // Get all relay peer IDs from metrics
                let healthy_relays: Vec<PeerId> = {
                    let metrics_guard = metrics.read().unwrap();
                    metrics_guard.keys().copied().collect()
                };

                let mut metrics_guard = metrics.write().unwrap();
                for peer_id in healthy_relays {
                    if let Some(relay_metrics) = metrics_guard.get_mut(&peer_id) {
                        relay_metrics.load_score = Self::calculate_load_score(relay_metrics);
                        relay_metrics.last_updated = Instant::now();
                    }
                }

                // Remove stale metrics (older than 5 minutes)
                let cutoff = Instant::now() - Duration::from_secs(300);
                metrics_guard.retain(|_, metrics| metrics.last_updated > cutoff);

                debug!("Updated metrics for {} relays", metrics_guard.len());
            }
        });

        info!("Started smart load balancer metrics collection");
        Ok(())
    }

    /// Calculate overall load score for a relay
    fn calculate_load_score(metrics: &RelayMetrics) -> f32 {
        let cpu_weight = 0.3;
        let memory_weight = 0.25;
        let bandwidth_weight = 0.25;
        let connection_weight = 0.2;

        let cpu_score = metrics.cpu_usage;
        let memory_score = (metrics.memory_usage_bytes as f32) / (metrics.max_memory_bytes as f32);
        let bandwidth_score =
            (metrics.bandwidth_usage_bps as f32) / (metrics.max_bandwidth_bps as f32);
        let connection_score =
            (metrics.active_connections as f32) / (metrics.max_connections as f32);

        (cpu_score * cpu_weight)
            + (memory_score * memory_weight)
            + (bandwidth_score * bandwidth_weight)
            + (connection_score * connection_weight)
    }
}

/// Performance history helper methods
impl PerformanceHistory {
    /// Calculate recent success rate
    pub fn recent_success_rate(&self) -> f32 {
        if self.recent_attempts.is_empty() {
            return 1.0; // Assume good until proven otherwise
        }

        let successes = self.recent_attempts.iter().filter(|a| a.success).count();
        successes as f32 / self.recent_attempts.len() as f32
    }

    /// Calculate recent average latency
    pub fn recent_avg_latency(&self) -> f32 {
        let latencies: Vec<f32> = self
            .recent_attempts
            .iter()
            .filter_map(|a| a.latency_ms)
            .collect();

        if latencies.is_empty() {
            return 100.0; // Default assumption
        }

        latencies.iter().sum::<f32>() / latencies.len() as f32
    }
}

/// Load balancer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStatistics {
    pub total_relays: usize,
    pub total_connections: u32,
    pub avg_cpu_usage: f32,
    pub active_sessions: usize,
    pub algorithm: LoadBalancingAlgorithm,
}

// Make LoadBalancingAlgorithm serializable
impl Serialize for LoadBalancingAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            LoadBalancingAlgorithm::WeightedRoundRobin => "weighted_round_robin",
            LoadBalancingAlgorithm::LeastConnections => "least_connections",
            LoadBalancingAlgorithm::CapacityAware => "capacity_aware",
            LoadBalancingAlgorithm::GeographicLatency => "geographic_latency",
            LoadBalancingAlgorithm::Adaptive => "adaptive",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for LoadBalancingAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "weighted_round_robin" => Ok(LoadBalancingAlgorithm::WeightedRoundRobin),
            "least_connections" => Ok(LoadBalancingAlgorithm::LeastConnections),
            "capacity_aware" => Ok(LoadBalancingAlgorithm::CapacityAware),
            "geographic_latency" => Ok(LoadBalancingAlgorithm::GeographicLatency),
            "adaptive" => Ok(LoadBalancingAlgorithm::Adaptive),
            _ => Err(serde::de::Error::custom("Invalid load balancing algorithm")),
        }
    }
}
