#[cfg(feature = "iroh")]
mod live {
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use std::time::Duration;

    #[tokio::test]
    async fn endpoint_binds_and_generates_tickets() {
        // Create first endpoint with a slight delay to ensure clean socket binding
        let ctx_a = IrohCtx::new().await.unwrap();
        // Give the endpoint a moment to fully initialize
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let ticket = ctx_a.ticket().await.unwrap();
        assert!(ticket.len() > 40, "Ticket should be a non-trivial string");
        println!("Generated ticket: {}", ticket);

        // Create second endpoint
        let ctx_b = IrohCtx::new().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Generate a ticket from second endpoint too
        let ticket_b = ctx_b.ticket().await.unwrap();
        assert!(ticket_b.len() > 40, "Ticket B should be a non-trivial string");
        
        // Verify node IDs are different
        assert_ne!(ctx_a.node_id(), ctx_b.node_id(), "Node IDs should be different");
        println!("Node A: {}, Node B: {}", ctx_a.node_id(), ctx_b.node_id());
        println!("Ticket A: {}", ticket);
        println!("Ticket B: {}", ticket_b);
        
        // Note: Full connection testing will be done separately with a mock discovery service
        // For now, we're verifying that we can generate valid tickets
    }
    
    #[tokio::test]
    async fn endpoint_parses_tickets() {
        // Generate a test ticket
        let ctx = IrohCtx::new().await.unwrap();
        let ticket = ctx.ticket().await.unwrap();
        
        // Verify we can parse it (should not throw an error)
        let result = ctx.connect_by_ticket(&ticket).await;
        
        // The connection may fail without a discovery service or if connecting to self, but the parsing should work
        if let Err(e) = &result {
            // It should fail with a specific error about addressing or connection
            let error_str = e.to_string().to_lowercase();
            assert!(
                error_str.contains("addressing") || 
                error_str.contains("discovery") ||
                error_str.contains("connect") ||
                error_str.contains("connection"),
                "Expected error about addressing, discovery, or connection, got: {}", e
            );
        }
    }
}

// When iroh feature is not enabled, this test module doesn't exist
#[cfg(not(feature = "iroh"))]
#[test]
fn stub_builds_successfully() {
    // This test exists just to ensure the file compiles without the iroh feature
    println!("Stub build works correctly");
}
