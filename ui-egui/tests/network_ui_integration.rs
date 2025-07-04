//! Direct test for network panel state machine
//!
//! Tests that the NetworkPanel properly transitions states based on relay health events

use p2pgo_ui_egui::network_panel::{NetworkPanel, NetworkState};

#[cfg(feature = "iroh")]
use p2pgo_network::relay_monitor::RelayHealthStatus;

#[cfg(feature = "iroh")]
#[test]
#[ignore] // Temporarily ignored due to import issues
fn test_panel_relay_health_transitions() {
    // Create a network panel
    let mut panel = NetworkPanel::new();
    
    // Default state should be Offline
    assert_eq!(*panel.network_state(), NetworkState::Offline);
    
    // Simulate receiving a "healthy" relay event
    panel.update_relay_health(RelayHealthStatus::Healthy, Some(9876));
    assert_eq!(*panel.network_state(), NetworkState::Online);
    
    // Simulate receiving a "failed" relay event
    panel.update_relay_health(RelayHealthStatus::Failed, None);
    assert_eq!(*panel.network_state(), NetworkState::Offline);
}

#[cfg(feature = "iroh")]
#[test]
fn test_relay_status_notifications() {
    // Instead of testing through the App, test NetworkPanel directly
    let mut panel = NetworkPanel::new();
    
    // Test initial state
    assert_eq!(*panel.network_state(), NetworkState::Offline);
    
    // Test relay status notifications
    panel.update_relay_health(RelayHealthStatus::Restarting, None);
    assert_eq!(*panel.network_state(), NetworkState::StartingRelay);
    
    // Test relay port update
    panel.update_relay_health(RelayHealthStatus::Healthy, Some(12345));
    assert_eq!(*panel.network_state(), NetworkState::Online);
    assert_eq!(panel.relay_port(), Some(12345));
    
    // Test degraded status
    panel.update_relay_health(RelayHealthStatus::Degraded, Some(12345));
    assert_eq!(*panel.network_state(), NetworkState::Degraded);
    
    // Test failure
    panel.update_relay_health(RelayHealthStatus::Failed, None);
    assert_eq!(*panel.network_state(), NetworkState::Offline);
}
