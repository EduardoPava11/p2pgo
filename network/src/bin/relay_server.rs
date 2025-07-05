//! P2P Go Relay Server Binary
//!
//! Run with: cargo run --bin relay_server --features iroh

use anyhow::Result;
use clap::Parser;
use p2pgo_network::relay_server::RelayServerBuilder;
use p2pgo_network::relay_mesh::GossipConfig;
use std::time::Duration;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "P2P Go Relay Server")]
struct Args {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,
    
    /// Port to listen on (0 for auto)
    #[arg(long, default_value = "0")]
    port: u16,
    
    /// Maximum connections
    #[arg(long, default_value = "1000")]
    max_connections: usize,
    
    /// Maximum bandwidth in Mbps
    #[arg(long, default_value = "100.0")]
    max_bandwidth: f64,
    
    /// Bootstrap relay addresses (comma-separated)
    #[arg(long)]
    bootstrap: Option<String>,
    
    /// Disable relay mesh networking
    #[arg(long)]
    no_mesh: bool,
    
    /// Gossip announce interval in seconds
    #[arg(long, default_value = "30")]
    announce_interval: u64,
    
    /// Gossip fanout (peers per round)
    #[arg(long, default_value = "5")]
    gossip_fanout: usize,
    
    /// Store game history
    #[arg(long)]
    store_history: bool,
    
    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(args.log_level.parse()?)
        )
        .init();
    
    info!("Starting P2P Go Relay Server");
    info!("Configuration:");
    info!("  Bind: {}:{}", args.bind, args.port);
    info!("  Max connections: {}", args.max_connections);
    info!("  Max bandwidth: {} Mbps", args.max_bandwidth);
    info!("  Mesh networking: {}", !args.no_mesh);
    info!("  Store history: {}", args.store_history);
    
    // Parse bootstrap relays
    let bootstrap_relays: Vec<String> = args.bootstrap
        .map(|s| s.split(',').map(String::from).collect())
        .unwrap_or_default();
    
    if !bootstrap_relays.is_empty() {
        info!("Bootstrap relays:");
        for relay in &bootstrap_relays {
            info!("  - {}", relay);
        }
    }
    
    // Configure gossip protocol
    let mut gossip_config = GossipConfig::default();
    gossip_config.announce_interval = Duration::from_secs(args.announce_interval);
    gossip_config.gossip_fanout = args.gossip_fanout;
    
    // Build and start server
    let server = RelayServerBuilder::new()
        .bind_address(&args.bind)
        .port(args.port)
        .max_connections(args.max_connections)
        .max_bandwidth_mbps(args.max_bandwidth)
        .bootstrap_relays(bootstrap_relays)
        .enable_mesh(!args.no_mesh)
        .gossip_config(gossip_config)
        .build()
        .await?;
    
    server.start().await?;
    
    info!("Relay server running");
    info!("Press Ctrl+C to stop");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down...");
    server.shutdown().await;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}