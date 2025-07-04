// SPDX-License-Identifier: MIT OR Apache-2.0
//! build.rs script to configure build and generate Info.plist from VERSION file

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Watch VERSION file for changes to trigger rebuild
    println!("cargo:rerun-if-changed=../VERSION");
    
    // Read version from VERSION file in root directory
    let version_path = Path::new("..").join("VERSION");
    let version = fs::read_to_string(&version_path)
        .map_err(|e| format!("Failed to read VERSION file: {}", e))?
        .trim()
        .to_string();
    
    println!("cargo:info=Building P2P Go version: {}", version);
    
    if cfg!(target_os = "macos") {
        // Ensure the main binary will search @executable_path/../Frameworks
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        
        // Set linker to prioritize system libraries over Homebrew
        // This prevents linking against Homebrew's libunwind which causes architecture issues
        println!("cargo:rustc-link-search=native=/usr/lib");
        println!("cargo:rustc-link-search=native=/System/Library/Frameworks");
        
        // Use -search_paths_first to ensure system libraries are found first
        println!("cargo:rustc-link-arg=-Wl,-search_paths_first");
        
        // Define the version for compile-time access
        println!("cargo:rustc-env=P2PGO_VERSION={}", version);
    }
    
    Ok(())
}
