use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use p2pgo_network::{run_bootstrap_relay, RelayNode, BootstrapConfig};

/// P2P Go Relay Node
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "4001")]
    port: u16,
    
    /// Run as bootstrap relay (first node in network)
    #[arg(long)]
    bootstrap: bool,
    
    /// Connect to existing relay
    #[arg(long)]
    connect: Option<String>,
    
    /// Data directory
    #[arg(long, default_value = "~/.p2pgo/relay")]
    data_dir: PathBuf,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let filter = if args.verbose {
        EnvFilter::from_default_env()
            .add_directive("p2pgo_network=debug".parse()?)
            .add_directive("libp2p=debug".parse()?)
    } else {
        EnvFilter::from_default_env()
            .add_directive("p2pgo_network=info".parse()?)
            .add_directive("libp2p=info".parse()?)
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    tracing::info!("Starting P2P Go Relay Node");
    
    if args.bootstrap {
        // Run as bootstrap relay
        tracing::info!("Running as bootstrap relay on port {}", args.port);
        run_bootstrap_relay(args.port).await?;
    } else {
        // Run as regular relay
        let bootstrap_config = if let Some(connect_addr) = args.connect {
            tracing::info!("Connecting to relay: {}", connect_addr);
            BootstrapConfig {
                relay_address: Some(connect_addr.parse()?),
                enable_mdns: true,
                enable_relay_discovery: true,
            }
        } else {
            BootstrapConfig::default()
        };
        
        let relay = RelayNode::new(args.port, bootstrap_config).await?;
        tracing::info!("Relay node started with peer_id: {}", relay.peer_id());
        
        // Run the relay
        relay.run().await?;
    }
    
    Ok(())
}