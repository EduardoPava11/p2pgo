//! Connection management with retry logic and circuit breakers

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use libp2p::PeerId;
use tokio::time::sleep;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

/// Connection state for a peer
#[derive(Debug, Clone)]
pub struct ConnectionState {
    /// Peer ID
    pub peer_id: PeerId,
    /// Connection attempts
    pub attempts: u32,
    /// Last attempt time
    pub last_attempt: Option<Instant>,
    /// Connection established time
    pub connected_at: Option<Instant>,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Circuit breaker state
    pub circuit_breaker: CircuitBreakerState,
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, connections allowed
    Closed,
    /// Circuit is open, connections blocked
    Open {
        /// When the circuit was opened
        opened_at: Instant,
    },
    /// Circuit is half-open, testing connection
    HalfOpen,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Exponential backoff factor
    pub backoff_factor: f64,
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Circuit breaker threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker timeout
    pub circuit_breaker_timeout: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_factor: 2.0,
            max_attempts: 10,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Connection manager with retry logic
pub struct ConnectionManager {
    /// Connection states by peer ID
    states: Arc<Mutex<HashMap<PeerId, ConnectionState>>>,
    /// Retry configuration
    config: RetryConfig,
}

impl ConnectionManager {
    pub fn new(config: RetryConfig) -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
    
    /// Attempt to connect to a peer with retry logic
    pub async fn connect_with_retry<F, Fut>(
        &self,
        peer_id: PeerId,
        connect_fn: F,
    ) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut attempt = 0;
        let mut delay = self.config.initial_delay;
        
        loop {
            // Check circuit breaker
            if !self.should_attempt_connection(&peer_id) {
                return Err(anyhow!("Circuit breaker is open for peer {}", peer_id));
            }
            
            // Update state
            self.update_state(&peer_id, |state| {
                state.attempts += 1;
                state.last_attempt = Some(Instant::now());
                
                // Set to half-open if was open
                if matches!(state.circuit_breaker, CircuitBreakerState::Open { .. }) {
                    state.circuit_breaker = CircuitBreakerState::HalfOpen;
                }
            });
            
            // Attempt connection
            match connect_fn().await {
                Ok(()) => {
                    info!("Successfully connected to peer {} after {} attempts", peer_id, attempt + 1);
                    
                    // Update state on success
                    self.update_state(&peer_id, |state| {
                        state.connected_at = Some(Instant::now());
                        state.consecutive_failures = 0;
                        state.circuit_breaker = CircuitBreakerState::Closed;
                    });
                    
                    return Ok(());
                }
                Err(e) => {
                    warn!("Connection attempt {} to peer {} failed: {}", attempt + 1, peer_id, e);
                    
                    // Update failure count
                    self.update_state(&peer_id, |state| {
                        state.consecutive_failures += 1;
                        
                        // Open circuit breaker if threshold reached
                        if state.consecutive_failures >= self.config.circuit_breaker_threshold {
                            state.circuit_breaker = CircuitBreakerState::Open {
                                opened_at: Instant::now(),
                            };
                            error!(
                                "Circuit breaker opened for peer {} after {} consecutive failures",
                                peer_id, state.consecutive_failures
                            );
                        }
                    });
                    
                    attempt += 1;
                    
                    // Check if we've exceeded max attempts
                    if attempt >= self.config.max_attempts {
                        return Err(anyhow!(
                            "Failed to connect to peer {} after {} attempts",
                            peer_id, attempt
                        ));
                    }
                    
                    // Wait before next attempt
                    info!("Retrying connection to peer {} in {:?}", peer_id, delay);
                    sleep(delay).await;
                    
                    // Calculate next delay with exponential backoff
                    delay = std::cmp::min(
                        Duration::from_secs_f64(delay.as_secs_f64() * self.config.backoff_factor),
                        self.config.max_delay,
                    );
                }
            }
        }
    }
    
    /// Check if connection should be attempted
    fn should_attempt_connection(&self, peer_id: &PeerId) -> bool {
        let states = self.states.lock().unwrap();
        
        if let Some(state) = states.get(peer_id) {
            match state.circuit_breaker {
                CircuitBreakerState::Closed => true,
                CircuitBreakerState::HalfOpen => true,
                CircuitBreakerState::Open { opened_at } => {
                    // Check if timeout has passed
                    opened_at.elapsed() >= self.config.circuit_breaker_timeout
                }
            }
        } else {
            true // New peer, allow connection
        }
    }
    
    /// Update connection state
    fn update_state<F>(&self, peer_id: &PeerId, updater: F)
    where
        F: FnOnce(&mut ConnectionState),
    {
        let mut states = self.states.lock().unwrap();
        
        let state = states.entry(*peer_id).or_insert_with(|| ConnectionState {
            peer_id: *peer_id,
            attempts: 0,
            last_attempt: None,
            connected_at: None,
            consecutive_failures: 0,
            circuit_breaker: CircuitBreakerState::Closed,
        });
        
        updater(state);
    }
    
    /// Get connection state for a peer
    pub fn get_state(&self, peer_id: &PeerId) -> Option<ConnectionState> {
        self.states.lock().unwrap().get(peer_id).cloned()
    }
    
    /// Reset connection state for a peer
    pub fn reset_peer(&self, peer_id: &PeerId) {
        let mut states = self.states.lock().unwrap();
        states.remove(peer_id);
    }
    
    /// Get all peer states
    pub fn get_all_states(&self) -> Vec<ConnectionState> {
        self.states.lock().unwrap().values().cloned().collect()
    }
    
    /// Handle disconnection
    pub fn handle_disconnection(&self, peer_id: &PeerId) {
        self.update_state(peer_id, |state| {
            state.connected_at = None;
        });
    }
}

/// Reconnection strategy
pub enum ReconnectionStrategy {
    /// Always attempt to reconnect
    Always,
    /// Only reconnect if was previously connected
    OnlyPreviouslyConnected,
    /// Never automatically reconnect
    Never,
    /// Custom strategy
    Custom(Box<dyn Fn(&ConnectionState) -> bool + Send + Sync>),
}

/// Automatic reconnection manager
pub struct ReconnectionManager {
    connection_manager: Arc<ConnectionManager>,
    strategy: ReconnectionStrategy,
    /// Active reconnection tasks
    active_tasks: Arc<Mutex<HashMap<PeerId, tokio::task::JoinHandle<()>>>>,
}

impl ReconnectionManager {
    pub fn new(connection_manager: Arc<ConnectionManager>, strategy: ReconnectionStrategy) -> Self {
        Self {
            connection_manager,
            strategy,
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Handle peer disconnection and potentially trigger reconnection
    pub fn handle_disconnection<F, Fut>(&self, peer_id: PeerId, connect_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        // Update connection state
        self.connection_manager.handle_disconnection(&peer_id);
        
        // Check if we should reconnect
        let should_reconnect = match &self.strategy {
            ReconnectionStrategy::Always => true,
            ReconnectionStrategy::Never => false,
            ReconnectionStrategy::OnlyPreviouslyConnected => {
                if let Some(state) = self.connection_manager.get_state(&peer_id) {
                    state.connected_at.is_some()
                } else {
                    false
                }
            }
            ReconnectionStrategy::Custom(strategy) => {
                if let Some(state) = self.connection_manager.get_state(&peer_id) {
                    strategy(&state)
                } else {
                    false
                }
            }
        };
        
        if should_reconnect {
            self.trigger_reconnection(peer_id, connect_fn);
        }
    }
    
    /// Trigger reconnection for a peer
    fn trigger_reconnection<F, Fut>(&self, peer_id: PeerId, connect_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let mut tasks = self.active_tasks.lock().unwrap();
        
        // Check if already reconnecting
        if tasks.contains_key(&peer_id) {
            return;
        }
        
        let connection_manager = self.connection_manager.clone();
        let active_tasks = self.active_tasks.clone();
        
        // Spawn reconnection task
        let task = tokio::spawn(async move {
            info!("Starting automatic reconnection for peer {}", peer_id);
            
            // Wait a bit before reconnecting
            sleep(Duration::from_secs(5)).await;
            
            // Attempt reconnection
            match connection_manager.connect_with_retry(peer_id, connect_fn).await {
                Ok(()) => info!("Successfully reconnected to peer {}", peer_id),
                Err(e) => error!("Failed to reconnect to peer {}: {}", peer_id, e),
            }
            
            // Remove from active tasks
            active_tasks.lock().unwrap().remove(&peer_id);
        });
        
        tasks.insert(peer_id, task);
    }
    
    /// Cancel reconnection for a peer
    pub fn cancel_reconnection(&self, peer_id: &PeerId) {
        let mut tasks = self.active_tasks.lock().unwrap();
        if let Some(task) = tasks.remove(peer_id) {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_successful_connection() {
        let config = RetryConfig::default();
        let manager = ConnectionManager::new(config);
        let peer_id = PeerId::random();
        
        let result = manager.connect_with_retry(peer_id, || async {
            Ok(())
        }).await;
        
        assert!(result.is_ok());
        
        let state = manager.get_state(&peer_id).unwrap();
        assert_eq!(state.attempts, 1);
        assert_eq!(state.consecutive_failures, 0);
        assert!(matches!(state.circuit_breaker, CircuitBreakerState::Closed));
    }
    
    #[tokio::test]
    async fn test_retry_with_eventual_success() {
        let mut config = RetryConfig::default();
        config.initial_delay = Duration::from_millis(10);
        
        let manager = ConnectionManager::new(config);
        let peer_id = PeerId::random();
        
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = manager.connect_with_retry(peer_id, || {
            let count = attempt_count_clone.clone();
            async move {
                let mut c = count.lock().unwrap();
                *c += 1;
                if *c < 3 {
                    Err(anyhow!("Connection failed"))
                } else {
                    Ok(())
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let mut config = RetryConfig::default();
        config.initial_delay = Duration::from_millis(10);
        config.circuit_breaker_threshold = 2;
        config.max_attempts = 5;
        
        let manager = ConnectionManager::new(config);
        let peer_id = PeerId::random();
        
        let result = manager.connect_with_retry(peer_id, || async {
            Err(anyhow!("Always fails"))
        }).await;
        
        assert!(result.is_err());
        
        let state = manager.get_state(&peer_id).unwrap();
        assert!(state.consecutive_failures >= 2);
        assert!(matches!(state.circuit_breaker, CircuitBreakerState::Open { .. }));
    }
}