//! Wrapper to avoid libunwind dependency

use std::panic;

fn main() {
    // Set up panic handler that doesn't require unwinding
    panic::set_hook(Box::new(|info| {
        eprintln!("Application error: {}", info);
    }));

    // Run the actual app
    if let Err(e) = p2pgo_ui_v2::run() {
        eprintln!("Failed to run application: {}", e);
        std::process::exit(1);
    }
}