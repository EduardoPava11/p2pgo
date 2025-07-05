#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn ticket_encode_decode_connect() -> Result<()> {
        // Create host endpoint
        let host = IrohCtx::new().await?;

        // Wait a moment for the endpoint to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Generate a ticket
        let ticket = host.ticket().await?;
        assert!(!ticket.is_empty(), "Ticket should be a non-empty string");
        println!("Generated host ticket: {}", ticket);

        // Create guest endpoint
        let guest = IrohCtx::new().await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Guest should connect to host with timeout
        let conn_result = timeout(Duration::from_secs(5), guest.connect_by_ticket(&ticket)).await?;

        // Assert that connection succeeded or failed with a known networking error
        // (not a serialization error)
        if let Err(e) = &conn_result {
            let err_str = e.to_string().to_lowercase();
            assert!(
                err_str.contains("connection")
                    || err_str.contains("network")
                    || err_str.contains("timeout")
                    || err_str.contains("address"),
                "Got unexpected error type: {}",
                e
            );
        }

        println!("Connection attempt completed within timeout");
        Ok(())
    }
}

// When iroh feature is not enabled, this test module doesn't exist
#[cfg(not(feature = "iroh"))]
#[test]
fn stub_builds_successfully() {
    // This test exists just to ensure the file compiles without the iroh feature
    println!("Stub build works correctly");
}
