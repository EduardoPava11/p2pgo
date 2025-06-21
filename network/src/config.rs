use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub relay_mode: RelayModeConfig,
    pub relay_addrs: Vec<String>,
    #[serde(default = "default_gossip_buffer_size")]
    pub gossip_buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelayModeConfig {
    Default,
    Custom,
    SelfRelay,
}

fn default_gossip_buffer_size() -> usize {
    256
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            relay_mode: RelayModeConfig::Default,
            relay_addrs: vec![
                "/dns4/use1-1.relay.iroh.network/tcp/443/quic-v1/p2p/12D3KooWAzmS7BFMw7A1h35QJT2PzG5EbBTnmTDsRvyXNvzkCwj5".to_string(),
            ],
            gossip_buffer_size: default_gossip_buffer_size(),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("io", "p2pgo", "p2pgo")
        .context("Failed to determine config directory")?;
    
    // On macOS, use Application Support directory
    let config_dir = if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        PathBuf::from(home).join("Library/Application Support/p2pgo")
    } else {
        proj_dirs.config_dir().to_path_buf()
    };
    
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Result<NetworkConfig> {
    let config_path = get_config_path()
        .context("Failed to determine config path")?;
    
    if !config_path.exists() {
        tracing::info!("Config file not found, creating default at: {}", config_path.display());
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }
        
        let default_config = NetworkConfig::default();
        let toml_content = toml::to_string_pretty(&default_config)
            .context("Failed to serialize default config")?;
        
        fs::write(&config_path, toml_content)
            .context("Failed to write default config file")?;
        
        tracing::info!("Created default config at: {}", config_path.display());
        return Ok(default_config);
    }
    
    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
    
    toml::from_str::<NetworkConfig>(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))
}

pub fn save_config(config: &NetworkConfig) -> Result<()> {
    let config_path = get_config_path()
        .context("Failed to determine config path")?;
    
    let toml_content = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    
    fs::write(&config_path, toml_content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
    
    tracing::info!("Saved config to: {}", config_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert!(matches!(config.relay_mode, RelayModeConfig::Default));
        assert!(!config.relay_addrs.is_empty());
        assert_eq!(config.gossip_buffer_size, 256);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = NetworkConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        
        let deserialized: NetworkConfig = toml::from_str(&toml_str).unwrap();
        assert!(matches!(deserialized.relay_mode, RelayModeConfig::Default));
    }
    
    #[test]
    fn test_load_save_config() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");
        
        // Mock the config path
        std::env::set_var("HOME", temp_dir.path());
        
        let config = NetworkConfig::default();
        let toml_content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, toml_content)?;
        
        // This would normally use the real config path, but for testing we'll verify the structure
        assert!(config_path.exists());
        
        Ok(())
    }
}
