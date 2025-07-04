// SPDX-License-Identifier: MIT OR Apache-2.0

//! Static registry for game channels

use std::sync::Arc;
use super::GameChannel;

#[cfg(feature = "iroh")]
use std::collections::HashMap;

/// Game channel registry for managing active channels
pub struct GameChannelRegistry;

impl GameChannelRegistry {
    /// Get the global registry instance
    #[cfg(feature = "iroh")]
    fn instance() -> &'static tokio::sync::RwLock<HashMap<String, std::sync::Weak<GameChannel>>> {
        use std::sync::OnceLock;
        static GAME_CHANNELS: OnceLock<tokio::sync::RwLock<HashMap<String, std::sync::Weak<GameChannel>>>> = 
            OnceLock::new();
        
        GAME_CHANNELS.get_or_init(|| {
            tokio::sync::RwLock::new(HashMap::new())
        })
    }
    
    /// Register a game channel in the global registry
    #[cfg(feature = "iroh")]
    pub fn register_channel(game_id: &str, channel: &Arc<GameChannel>) {
        let registry = Self::instance();
        let game_id = game_id.to_string();
        let weak_channel = Arc::downgrade(channel);
        
        // Spawn a task to register the channel asynchronously
        tokio::spawn(async move {
            let mut registry = registry.write().await;
            registry.insert(game_id.clone(), weak_channel);
            tracing::debug!("Registered game channel: {}", game_id);
        });
    }
    
    /// Get a game channel by game ID
    #[cfg(feature = "iroh")]
    pub async fn get_channel(game_id: &str) -> Option<Arc<GameChannel>> {
        let registry = Self::instance();
        let registry = registry.read().await;
        
        if let Some(weak_channel) = registry.get(game_id) {
            weak_channel.upgrade()
        } else {
            None
        }
    }
    
    /// Remove a game channel from the registry
    #[cfg(feature = "iroh")]
    pub async fn unregister_channel(game_id: &str) {
        let registry = Self::instance();
        let mut registry = registry.write().await;
        
        if registry.remove(game_id).is_some() {
            tracing::debug!("Unregistered game channel: {}", game_id);
        }
    }
    
    /// List all active game IDs in the registry
    #[cfg(feature = "iroh")]
    pub async fn list_active_games() -> Vec<String> {
        let registry = Self::instance();
        let mut registry = registry.write().await;
        
        // Clean up dead weak references and collect active game IDs
        let mut active_games = Vec::new();
        let mut to_remove = Vec::new();
        
        for (game_id, weak_channel) in registry.iter() {
            if weak_channel.upgrade().is_some() {
                active_games.push(game_id.clone());
            } else {
                to_remove.push(game_id.clone());
            }
        }
        
        // Remove dead references
        for game_id in to_remove {
            registry.remove(&game_id);
        }
        
        tracing::debug!("Found {} active game channels", active_games.len());
        active_games
    }
    
    /// Get statistics about the registry
    #[cfg(feature = "iroh")]
    pub async fn get_stats() -> RegistryStats {
        let registry = Self::instance();
        let registry = registry.read().await;
        
        let total_entries = registry.len();
        let mut active_channels = 0;
        
        for weak_channel in registry.values() {
            if weak_channel.upgrade().is_some() {
                active_channels += 1;
            }
        }
        
        RegistryStats {
            total_entries,
            active_channels,
            dead_references: total_entries - active_channels,
        }
    }
    
    /// Clean up dead weak references from the registry
    #[cfg(feature = "iroh")]
    pub async fn cleanup_dead_references() -> usize {
        let registry = Self::instance();
        let mut registry = registry.write().await;
        
        let mut to_remove = Vec::new();
        
        for (game_id, weak_channel) in registry.iter() {
            if weak_channel.upgrade().is_none() {
                to_remove.push(game_id.clone());
            }
        }
        
        let removed_count = to_remove.len();
        for game_id in to_remove {
            registry.remove(&game_id);
        }
        
        if removed_count > 0 {
            tracing::debug!("Cleaned up {} dead references from registry", removed_count);
        }
        
        removed_count
    }
}

/// Statistics about the game channel registry
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total number of entries in the registry
    pub total_entries: usize,
    /// Number of active (live) channels
    pub active_channels: usize,
    /// Number of dead weak references
    pub dead_references: usize,
}

/// Register a game channel in the global registry
#[cfg(feature = "iroh")]
pub fn register(game_id: &str, channel: &Arc<GameChannel>) {
    GameChannelRegistry::register_channel(game_id, channel);
}

/// Get a game channel by game ID
#[cfg(feature = "iroh")]
pub async fn get_for_game_id(game_id: &str) -> Option<Arc<GameChannel>> {
    GameChannelRegistry::get_channel(game_id).await
}

/// Unregister a game channel
#[cfg(feature = "iroh")]
pub async fn unregister(game_id: &str) {
    GameChannelRegistry::unregister_channel(game_id).await
}

/// List all active game IDs
#[cfg(feature = "iroh")]
pub async fn list_active_games() -> Vec<String> {
    GameChannelRegistry::list_active_games().await
}

/// Get registry statistics
#[cfg(feature = "iroh")]
pub async fn get_stats() -> RegistryStats {
    GameChannelRegistry::get_stats().await
}

/// Clean up dead references
#[cfg(feature = "iroh")]
pub async fn cleanup() -> usize {
    GameChannelRegistry::cleanup_dead_references().await
}

// Stubs for non-iroh builds
#[cfg(not(feature = "iroh"))]
pub fn register(_game_id: &str, _channel: &Arc<GameChannel>) {
    // No-op for non-iroh builds
}

#[cfg(not(feature = "iroh"))]
pub async fn get_for_game_id(_game_id: &str) -> Option<Arc<GameChannel>> {
    None
}

#[cfg(not(feature = "iroh"))]
pub async fn unregister(_game_id: &str) {
    // No-op for non-iroh builds
}

#[cfg(not(feature = "iroh"))]
pub async fn list_active_games() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "iroh")]
    #[tokio::test]
    async fn test_registry_operations() {
        use super::GameChannel;
        
        // Create test channels
        let game_id1 = "test-game-1";
        let game_id2 = "test-game-2";
        
        let state1 = GameState::new(9);
        let state2 = GameState::new(13);
        
        let channel1 = Arc::new(GameChannel::new(game_id1.to_string(), state1));
        let channel2 = Arc::new(GameChannel::new(game_id2.to_string(), state2));
        
        // Register channels
        register(game_id1, &channel1);
        register(game_id2, &channel2);
        
        // Allow async registration to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Test retrieval
        let retrieved1 = get_for_game_id(game_id1).await;
        assert!(retrieved1.is_some());
        assert_eq!(retrieved1.unwrap().get_game_id(), game_id1);
        
        // Test listing
        let active_games = list_active_games().await;
        assert!(active_games.contains(&game_id1.to_string()));
        assert!(active_games.contains(&game_id2.to_string()));
        
        // Test stats
        let stats = get_stats().await;
        assert!(stats.active_channels >= 2);
        
        // Test unregistration
        unregister(game_id1).await;
        let retrieved_after_unregister = get_for_game_id(game_id1).await;
        assert!(retrieved_after_unregister.is_none());
        
        // Test cleanup
        drop(channel2); // Drop the strong reference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let cleaned = cleanup().await;
        assert!(cleaned > 0);
    }
}
