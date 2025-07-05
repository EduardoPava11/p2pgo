// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests for embedded relay functionality

#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::port::PortManager;
    use p2pgo_network::relay_monitor::{RelayHealthStatus, RestartableRelay};
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn test_embedded_relay_start_stop() -> Result<()> {
        // Skip this test on CI if environment indicates
        if std::env::var("CI").is_ok() {
            println!("Skipping embedded relay test on CI");
            return Ok(());
        }

        // Initialize port manager
        let port_manager = PortManager::new()?;

        // Create restartable relay
        let mut relay = RestartableRelay::new(port_manager);

        // Define a simple relay function
        let relay_fn = |port: u16, mut shutdown: oneshot::Receiver<()>| async move {
            println!("Starting mock relay on port {}", port);

            // Bind to the port to simulate relay operation
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
            println!("Bound to port {}", port);

            // Run until shutdown signal
            tokio::select! {
                _ = shutdown => {
                    println!("Received shutdown signal");
                },
                res = listener.accept() => {
                    if let Ok((_, addr)) = res {
                        println!("Got connection from {}", addr);
                    }
                }
            }

            Ok(())
        };

        // Start the relay
        let port = relay.start(relay_fn).await?;
        println!("Relay started on port {}", port);

        // Get the state
        let state = relay.state();

        // Allow some time for startup
        tokio::time::sleep(Duration::from_millis(500)).await;

        {
            let state = state.read().await;
            assert_eq!(state.listening_port, Some(port));
        }

        // Stop the relay
        relay.stop().await?;
        println!("Relay stopped");

        // Wait a bit to ensure shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Port should be available again after stopping
        assert!(p2pgo_network::port::is_port_available(port));

        Ok(())
    }

    #[tokio::test]
    async fn test_relay_restart() -> Result<()> {
        // Skip this test on CI if environment indicates
        if std::env::var("CI").is_ok() {
            println!("Skipping embedded relay restart test on CI");
            return Ok(());
        }

        // Initialize port manager
        let port_manager = PortManager::new()?;

        // Create restartable relay
        let mut relay = RestartableRelay::new(port_manager);

        // Define a test relay that can be restarted
        let relay_fn = |port: u16, mut shutdown: oneshot::Receiver<()>| async move {
            println!("Starting mock relay on port {}", port);

            // Bind to the port to simulate relay operation
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
            println!("Bound to port {}", port);

            tokio::select! {
                _ = shutdown => {
                    println!("Received shutdown signal");
                },
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    println!("Mock relay timeout (simulated normal operation)");
                }
            }

            Ok(())
        };

        // Start the relay
        let port = relay.start(relay_fn).await?;
        println!("Relay started on port {}", port);

        // Run for a short time
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Now restart the relay
        println!("Restarting relay...");
        let new_port = relay.restart(relay_fn).await?;
        println!("Relay restarted on port {}", new_port);

        // The port should be the same after restart
        assert_eq!(port, new_port);

        // Allow some time for the restart
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Get the state after restart
        let state = relay.state();

        {
            let state = state.read().await;
            assert_eq!(state.listening_port, Some(port));
            assert_eq!(state.restart_attempts, 1);
        }

        // Stop the relay
        relay.stop().await?;

        Ok(())
    }
}

// When iroh feature is not enabled, this test module doesn't exist
#[cfg(not(feature = "iroh"))]
#[test]
fn dummy_test() {
    // Empty test to keep the file valid
    assert!(true);
}
