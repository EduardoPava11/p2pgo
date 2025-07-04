//! P2P Go - Decentralized Go with Neural Networks

fn main() {
    // Run with panic=abort to avoid libunwind dependency
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Application error: {}", info);
    }));
    
    if let Err(e) = p2pgo_ui_v2::run() {
        eprintln!("Failed to run application: {}", e);
        std::process::exit(1);
    }
}