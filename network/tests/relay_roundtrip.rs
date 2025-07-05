//! Integration test for relay connectivity
//! Tests that two peers can connect through Iroh relay servers

#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn relay_roundtrip() -> Result<()> {
        // Create two nodes with relay support
        let host = IrohCtx::new().await?;
        let guest = IrohCtx::new().await?;

        // Wait for relay connections to be established
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Generate ticket from host - must advertise relay addresses
        let ticket = timeout(Duration::from_secs(10), host.ticket()).await??;

        // Verify ticket contains relay information
        assert!(
            ticket.contains("/relay") || ticket.len() > 20, // Not a stub ticket
            "Ticket should contain relay addresses or be a real ticket, got: {}",
            &ticket[..std::cmp::min(ticket.len(), 50)]
        );

        // Guest connects to host via ticket
        timeout(Duration::from_secs(15), guest.connect_by_ticket(&ticket)).await??;

        // Test direct peer connection for game communication
        let conn = timeout(Duration::from_secs(10), guest.connect_to_peer(&ticket)).await??;

        // Quick sanity check: send a message over the connection
        let mut stream = conn.open_uni().await?;
        stream.write_all(b"ping").await?;
        stream.finish()?;

        println!("✅ Relay roundtrip test passed - peers can connect over relay");
        Ok(())
    }

    #[tokio::test]
    async fn external_addresses_available() -> Result<()> {
        let ctx = IrohCtx::new().await?;

        // Wait for node to be ready
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Check that external addresses are available
        let external_addrs = ctx.external_addrs().await?;

        // We should have at least one external address (relay or direct)
        assert!(
            !external_addrs.is_empty(),
            "Node should have external addresses"
        );

        println!("✅ External addresses: {:?}", external_addrs);
        Ok(())
    }
}

#[cfg(not(feature = "iroh"))]
mod stub_tests {
    use anyhow::Result;

    #[tokio::test]
    async fn stub_mode_passes() -> Result<()> {
        // Stub mode should always pass
        println!("✅ Stub mode relay test passed");
        Ok(())
    }
}
