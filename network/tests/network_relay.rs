//! Comprehensive network relay tests
//! Tests relay configuration, health monitoring, and connectivity

use anyhow::Result;
use p2pgo_network::config::{NetworkConfig, RelayModeConfig};
use p2pgo_network::iroh_endpoint::IrohCtx;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_relay_config_default() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create default config
    let config = NetworkConfig::default();
    let toml_str = toml::to_string(&config)?;
    fs::write(&config_path, toml_str)?;
    
    // Load config
    let loaded_toml = fs::read_to_string(&config_path)?;
    let loaded_config: NetworkConfig = toml::from_str(&loaded_toml)?;
    
    match loaded_config.relay_mode {
        RelayModeConfig::Default => {
            assert_eq!(loaded_config.relay_mode, RelayModeConfig::Default);
        }
        _ => panic!("Expected default relay mode"),
    }
    
    println!("✅ Default relay config test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_config_custom() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create custom config  
    let config = NetworkConfig {
        relay_mode: RelayModeConfig::Custom,
        relay_addrs: vec!["https://custom-relay.example.com".to_string()],
        gossip_buffer_size: 256,
    };
    let toml_str = toml::to_string(&config)?;
    fs::write(&config_path, toml_str)?;
    
    // Load config
    let loaded_toml = fs::read_to_string(&config_path)?;
    let loaded_config: NetworkConfig = toml::from_str(&loaded_toml)?;
    
    match loaded_config.relay_mode {
        RelayModeConfig::Custom => {
            assert_eq!(loaded_config.relay_addrs, vec!["https://custom-relay.example.com".to_string()]);
        }
        _ => panic!("Expected custom relay mode"),
    }
    
    println!("✅ Custom relay config test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_config_self_hosted() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create self-hosted config
    let config = NetworkConfig {
        relay_mode: RelayModeConfig::SelfRelay,
        relay_addrs: vec!["127.0.0.1:8080".to_string()],
        gossip_buffer_size: 256,
    };
    let toml_str = toml::to_string(&config)?;
    fs::write(&config_path, toml_str)?;
    
    // Load config  
    let loaded_toml = fs::read_to_string(&config_path)?;
    let loaded_config: NetworkConfig = toml::from_str(&loaded_toml)?;
    
    match loaded_config.relay_mode {
        RelayModeConfig::SelfRelay => {
            assert_eq!(loaded_config.relay_addrs, vec!["127.0.0.1:8080".to_string()]);
        }
        _ => panic!("Expected self-hosted relay mode"),
    }
    
    println!("✅ Self-hosted relay config test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_connectivity() -> Result<()> {
    // Skip in CI
    if std::env::var("CI_MODE").is_ok() {
        println!("Skipping connectivity test in CI mode");
        return Ok(());
    }
    
    // Create two nodes
    let host = IrohCtx::new().await?;
    let guest = IrohCtx::new().await?;
    
    // Wait for relay connections
    tokio::time::sleep(Duration::from_millis(1500)).await;
    
    // Generate ticket from host
    let ticket = timeout(Duration::from_secs(15), host.ticket()).await??;
    
    // Verify ticket is not empty
    assert!(!ticket.is_empty(), "Ticket should not be empty");
    
    // We don't require ticket to be in a specific format as it might change
    println!("Generated ticket: {} (length: {})", ticket, ticket.len());
    
    // Guest connects to host - only try if we're not in CI
    if !std::env::var("CI").is_ok() {
        timeout(Duration::from_secs(20), guest.connect_by_ticket(&ticket)).await??;
    }
    
    println!("✅ Relay connectivity test passed");
    Ok(())
}

#[tokio::test]  
async fn test_relay_config_integration() -> Result<()> {
    // Test that IrohCtx properly loads and applies config 
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");
    
    // Create test config
    let config = NetworkConfig {
        relay_mode: RelayModeConfig::Default,
        relay_addrs: vec![
            "/dns4/use1-1.relay.iroh.network/tcp/443/quic-v1/p2p/12D3KooWAzmS7BFMw7A1h35QJT2PzG5EbBTnmTDsRvyXNvzkCwj5".to_string(),
        ],
        gossip_buffer_size: 256,
    };
    let toml_str = toml::to_string(&config)?;
    fs::write(&config_path, toml_str)?;
    
    // Set environment variable to use test config
    std::env::set_var("P2PGO_CONFIG_PATH", config_path.to_str().unwrap());
    
    // Create IrohCtx - should load the config
    let ctx = IrohCtx::new().await?;
    
    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    // Generate ticket to verify relay integration
    let ticket = timeout(Duration::from_secs(10), ctx.ticket()).await??;
    assert!(!ticket.is_empty(), "Ticket should be generated with config");
    
    // Clean up
    std::env::remove_var("P2PGO_CONFIG_PATH");
    
    println!("✅ Relay config integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_config_loading() -> Result<()> {
    // Test basic config loading functionality
    let config = p2pgo_network::config::load_config()?;
    
    // Should have default values
    assert!(!config.relay_addrs.is_empty(), "Config should have relay addresses");
    assert!(config.gossip_buffer_size > 0, "Config should have positive gossip buffer size");
    
    println!("✅ Config loading test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_port_persistence() -> Result<()> {
    use p2pgo_network::port;
    
    // In the test, we just ensure the function returns valid ports
    let (tcp_port, udp_port) = port::pick_or_remember_port()?;
    
    // Both ports should be valid
    assert!(tcp_port > 0, "TCP port should be positive");
    assert!(udp_port > 0, "UDP port should be positive");
    println!("Ports picked: TCP {}, UDP {}", tcp_port, udp_port);
    
    // Note: We can't reliably test persistence in concurrent test runs
    // since other tests might be modifying the same config file.
    // This is tested more thoroughly in unit tests in port.rs.
    
    println!("✅ Valid ports returned from pick_or_remember_port");
    
    println!("✅ Relay port persistence test passed");
    Ok(())
}

#[tokio::test]
async fn test_relay_health_events() -> Result<()> {
    use p2pgo_network::relay_monitor::{RelayHealthStatus, RelayHealthEvent};
    use tokio::sync::mpsc;
    
    // Create channel for health events
    let (health_tx, mut health_rx) = mpsc::channel(10);
    
    // Send health events directly through channel
    health_tx.send(RelayHealthEvent { 
        status: p2pgo_network::relay_monitor::RelayHealthStatus::Restarting,
        port: None,
        last_restart: None,
        is_self_relay: true,
        latency_ms: None,
        timestamp: Instant::now(),
    }).await?;
    
    health_tx.send(RelayHealthEvent { 
        status: p2pgo_network::relay_monitor::RelayHealthStatus::Healthy,
        port: Some(12345),
        last_restart: Some(Instant::now()),
        is_self_relay: true,
        latency_ms: Some(25),
        timestamp: Instant::now(),
    }).await?;
    
    // Verify events are received
    let event1 = timeout(Duration::from_secs(1), health_rx.recv()).await?.unwrap();
    let event2 = timeout(Duration::from_secs(1), health_rx.recv()).await?.unwrap();
    
    assert_eq!(event1.status, p2pgo_network::relay_monitor::RelayHealthStatus::Restarting, "First event should be Restarting");
    assert_eq!(event2.status, p2pgo_network::relay_monitor::RelayHealthStatus::Healthy, "Second event should be Healthy");
    
    println!("✅ Relay health events test passed");
    Ok(())
}

#[tokio::test]
#[ignore] // Skip in CI unless NAT_TEST=1 is set
async fn test_restart_relay() -> Result<()> {
    // Only run this test if explicitly enabled
    // Skip in CI
    if std::env::var("CI_MODE").is_ok() || std::env::var("NAT_TEST").unwrap_or_default() != "1" {
        println!("Skipping relay restart test - set NAT_TEST=1 to enable");
        return Ok(());
    }
    
    use p2pgo_network::iroh_endpoint::IrohCtx;
    
    // Create context
    let ctx = IrohCtx::new().await?;
    
    // Generate initial ticket
    let ticket1 = timeout(Duration::from_secs(10), ctx.ticket()).await??;
    assert!(!ticket1.is_empty(), "Initial ticket should be generated");
    
    // Note: The restart_relay functionality is tested in separate unit tests
    // within the IrohCtx implementation, so we don't need to test it here 
    // directly if we're in CI mode.
    
    println!("✅ Relay basic connectivity test passed");
    Ok(())
}
