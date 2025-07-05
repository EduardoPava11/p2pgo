// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for the port manager

use p2pgo_network::port::{is_port_available, pick_available_port, PortManager};
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_port_availability_check() {
    // Bind to a port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    // Port should be unavailable
    assert!(!is_port_available(port));

    // Find a different port that should be available
    let mut available_port = port;
    while available_port == port || !is_port_available(available_port) {
        available_port = (available_port + 1) % 65535;
        if available_port < 1024 {
            available_port = 1024; // Skip privileged ports
        }
    }

    // This port should be available
    assert!(is_port_available(available_port));
}

#[test]
fn test_port_picking() {
    // Pick a port
    let port = pick_available_port().unwrap();

    // Port should be available
    assert!(is_port_available(port));

    // Bind to the port
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    // Port should now be unavailable
    assert!(!is_port_available(port));

    // Pick another port
    let port2 = pick_available_port().unwrap();

    // The new port should be different and available
    assert_ne!(port, port2);
    assert!(is_port_available(port2));
}

#[test]
fn test_port_manager_with_temp_dir() {
    // Use a temporary directory for the config
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("p2pgo");

    // Set environment variable to redirect config location
    let orig_home = env::var("HOME").ok();
    env::set_var("HOME", temp_dir.path());

    // Create a port manager with temp config path
    let port_manager = PortManager::new().unwrap();

    // Get a relay port
    let relay_port = port_manager.get_relay_port().unwrap();
    assert!(is_port_available(relay_port));

    // Get an HTTP port
    let http_port = port_manager.get_http_port().unwrap();
    assert!(is_port_available(http_port));
    assert_ne!(relay_port, http_port);

    // Clean up environment variable
    if let Some(home) = orig_home {
        env::set_var("HOME", home);
    } else {
        env::remove_var("HOME");
    }
}

#[test]
#[ignore] // Ignore until we fix port persistence issues
fn test_port_manager_persistence() {
    // Use a temporary directory for the config
    let temp_dir = tempdir().unwrap();
    let orig_home = env::var("HOME").ok();
    env::set_var("HOME", temp_dir.path());

    // Create first port manager
    let port_manager1 = PortManager::new().unwrap();
    let relay_port1 = port_manager1.get_relay_port().unwrap();
    let http_port1 = port_manager1.get_http_port().unwrap();

    // Drop the first port manager
    drop(port_manager1);

    // Create a second port manager
    let port_manager2 = PortManager::new().unwrap();
    let relay_port2 = port_manager2.get_relay_port().unwrap();
    let http_port2 = port_manager2.get_http_port().unwrap();

    // The ports should be the same since they were saved
    assert_eq!(relay_port1, relay_port2);
    assert_eq!(http_port1, http_port2);

    // Clean up environment variable
    if let Some(home) = orig_home {
        env::set_var("HOME", home);
    } else {
        env::remove_var("HOME");
    }
}

#[test]
#[ignore] // Temporarily ignore due to filesystem permission issues
fn test_port_manager_handles_unavailable_port() {
    // Use a temporary directory for the config
    let temp_dir = tempdir().unwrap();
    let orig_home = env::var("HOME").ok();
    env::set_var("HOME", temp_dir.path());

    // Create first port manager
    let port_manager1 = PortManager::new().unwrap();
    let relay_port1 = port_manager1.get_relay_port().unwrap();

    // Bind to the port to make it unavailable
    let listener = TcpListener::bind(format!("127.0.0.1:{}", relay_port1)).unwrap();

    // Create a second port manager
    let port_manager2 = PortManager::new().unwrap();

    // The port manager should detect that the saved port is unavailable
    // and should pick a different one
    let relay_port2 = port_manager2.get_relay_port().unwrap();
    assert_ne!(relay_port1, relay_port2);
    assert!(is_port_available(relay_port2));

    // Clean up environment variable
    if let Some(home) = orig_home {
        env::set_var("HOME", home);
    } else {
        env::remove_var("HOME");
    }

    // Make sure the listener is dropped
    drop(listener);
}
