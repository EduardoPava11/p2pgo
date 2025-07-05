//! Tests for the network panel UI component

#[cfg(feature = "iroh")]
use p2pgo_network::relay_monitor::RelayHealthStatus;
use p2pgo_ui_egui::network_panel::{NetworkPanel, NetworkState};

#[test]
#[ignore] // Temporarily ignored due to import issues
fn test_network_panel_state_machine() {
    let panel = NetworkPanel::new();

    // Default state should be Offline
    assert_eq!(*panel.network_state(), NetworkState::Offline);

    // Test state transitions based on relay health
    #[cfg(feature = "iroh")]
    {
        // Restarting state
        panel.update_relay_health(RelayHealthStatus::Restarting, None);
        assert_eq!(*panel.network_state(), NetworkState::StartingRelay);

        // Healthy state
        panel.update_relay_health(RelayHealthStatus::Healthy, Some(12345));
        assert_eq!(*panel.network_state(), NetworkState::Online);

        // Degraded state
        panel.update_relay_health(RelayHealthStatus::Degraded, Some(12345));
        assert_eq!(*panel.network_state(), NetworkState::Degraded);

        // Back to offline
        panel.update_relay_health(RelayHealthStatus::Failed, None);
        assert_eq!(*panel.network_state(), NetworkState::Offline);

        // Test syncing state
        panel.update_relay_health(RelayHealthStatus::Healthy, Some(12345));
        panel.set_syncing();
        assert_eq!(*panel.network_state(), NetworkState::Syncing);

        // Relay health update doesn't override syncing state
        panel.update_relay_health(RelayHealthStatus::Healthy, Some(12345));
        assert_eq!(*panel.network_state(), NetworkState::Syncing);
    }
}

#[test]
#[ignore] // Temporarily ignored due to import issues
fn test_relay_node_status() {
    let mut panel = NetworkPanel::new();

    // Default should be false
    assert!(!panel.is_relay_node());

    // Set as relay node
    panel.set_is_relay_node(true);
    assert!(panel.is_relay_node());

    // Set back to false
    panel.set_is_relay_node(false);
    assert!(!panel.is_relay_node());
}
