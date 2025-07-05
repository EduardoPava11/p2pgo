use anyhow::Result;
use clap::Parser;
use libp2p::identity::Keypair;
use p2pgo_network::{RelayNode, RNAMessage, RNAType};
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Multiaddr to connect to (for quick connect)
    #[arg(short, long)]
    connect: Option<String>,
    
    /// SGF file to upload as training data
    #[arg(short, long)]
    sgf: Option<PathBuf>,
    
    /// Move range for SGF (e.g., "0-50")
    #[arg(short, long, default_value = "0-361")]
    range: String,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let filter = if args.debug {
        EnvFilter::new("debug,libp2p=debug")
    } else {
        EnvFilter::new("info,libp2p=info")
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    info!("Starting P2P Go Relay Node");
    
    // Generate or load keypair
    let keypair = Keypair::generate_ed25519();
    
    // Create relay node
    let mut relay = RelayNode::new(keypair)?;
    info!("Local peer ID: {}", relay.peer_id());
    
    // Bootstrap
    relay.bootstrap().await?;
    
    // Show listening addresses
    for addr in relay.listening_addresses() {
        info!("Listening on: {}", addr);
    }
    
    // Quick connect if specified
    if let Some(addr) = args.connect {
        info!("Connecting to: {}", addr);
        match addr.parse() {
            Ok(multiaddr) => {
                relay.connect_to_peer(multiaddr).await?;
            }
            Err(e) => {
                error!("Invalid multiaddr: {}", e);
            }
        }
    }
    
    // Upload SGF if specified
    if let Some(sgf_path) = args.sgf {
        if let Ok(content) = std::fs::read_to_string(&sgf_path) {
            let range_parts: Vec<&str> = args.range.split('-').collect();
            if range_parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    range_parts[0].parse::<u32>(),
                    range_parts[1].parse::<u32>(),
                ) {
                    info!("Uploading SGF: {} (moves {}-{})", sgf_path.display(), start, end);
                    
                    let rna = relay.create_sgf_rna(content, (start as usize, end as usize));
                    relay.broadcast_rna(rna).await?;
                    
                    info!("SGF data broadcast as RNA");
                }
            }
        }
    }
    
    // Show connection instructions
    info!("\n=== Connection Instructions ===");
    info!("To connect another relay to this one, run:");
    for addr in relay.listening_addresses() {
        info!("  p2pgo-relay --connect {}/p2p/{}", addr, relay.peer_id());
    }
    info!("==============================\n");
    
    // Run event loop
    relay.handle_events().await?;
    
    Ok(())
}