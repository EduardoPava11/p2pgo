// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test program for the update checker functionality

use p2pgo_ui_egui::update_checker::{UpdateChecker, Version};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("P2P Go Update Checker Test");
    println!("==========================\n");
    
    // Test version parsing
    println!("Testing version parsing:");
    let versions = vec![
        "0.1.4",
        "1.0.0", 
        "2.3.5-beta.1",
        "1.0.0-rc.2",
    ];
    
    for version_str in versions {
        match Version::parse(version_str) {
            Ok(v) => println!("  ✓ Parsed '{}' -> {}", version_str, v.to_string()),
            Err(e) => println!("  ✗ Failed to parse '{}': {}", version_str, e),
        }
    }
    
    println!("\nTesting version comparison:");
    let v1 = Version::parse("1.0.0")?;
    let v2 = Version::parse("1.0.1")?;
    let v3 = Version::parse("1.0.0-beta.1")?;
    
    println!("  {} < {} : {}", v1.to_string(), v2.to_string(), v1 < v2);
    println!("  {} < {} : {}", v3.to_string(), v1.to_string(), v3 < v1);
    
    // Test update checking
    println!("\nTesting update checker:");
    let current_version = Version::parse("0.1.4")?;
    println!("  Current version: {}", current_version.to_string());
    
    let checker = UpdateChecker::new(current_version, "stable".to_string());
    
    // Check local manifest
    let manifest_path = Path::new("update_manifest.json");
    if manifest_path.exists() {
        println!("  Found local manifest file");
        
        match checker.check_file(manifest_path) {
            Ok(result) => {
                println!("\n  Update Check Result:");
                println!("    - Update available: {}", result.update_available);
                println!("    - Update required: {}", result.update_required);
                
                if let Some(ref latest) = result.latest_version {
                    println!("    - Latest version: {}", latest.to_string());
                }
                
                if let Some(ref announcement) = result.announcement {
                    println!("    - Announcement: {}", announcement);
                }
                
                if let Some(ref info) = result.update_info {
                    println!("\n  Update Info:");
                    println!("    - Download URL: {}", info.download_url);
                    println!("    - Size: {} MB", info.size as f64 / 1_048_576.0);
                    println!("    - Release date: {}", info.release_date);
                    println!("    - Supports in-place update: {}", info.supports_in_place);
                    
                    // Show platform-specific info
                    if let Some(platform_info) = checker.get_platform_info(info) {
                        println!("\n  Platform-specific info:");
                        println!("    - Platform: {} ({})", platform_info.platform, platform_info.arch);
                        if let Some(ref url) = platform_info.download_url {
                            println!("    - Download URL: {}", url);
                        }
                        if let Some(ref notes) = platform_info.notes {
                            println!("    - Notes: {}", notes);
                        }
                    }
                    
                    println!("\n  Release Notes:");
                    println!("{}", info.release_notes);
                }
            }
            Err(e) => {
                println!("  ✗ Failed to check for updates: {}", e);
            }
        }
    } else {
        println!("  No local manifest file found");
        println!("  Create one with: ./scripts/generate_update_manifest.sh");
    }
    
    // Test with different channels
    println!("\n\nTesting beta channel:");
    let beta_checker = UpdateChecker::new(current_version.clone(), "beta".to_string());
    if manifest_path.exists() {
        if let Ok(result) = beta_checker.check_file(manifest_path) {
            if let Some(ref latest) = result.latest_version {
                println!("  Beta channel latest: {}", latest.to_string());
            }
        }
    }
    
    Ok(())
}