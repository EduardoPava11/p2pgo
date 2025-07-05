#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::iroh_endpoint::IrohCtx;
    use std::time::Duration;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn document_sync_between_nodes() -> Result<()> {
        // Create two nodes
        let node_a = IrohCtx::new().await?;
        let node_b = IrohCtx::new().await?;

        // Wait for nodes to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Generate a unique game ID that both nodes will use
        let game_id = Uuid::new_v4().to_string();
        println!("Testing with game_id: {}", game_id);

        // Create some test move data
        let test_move_data = b"Test move data for synchronization";

        // Connect the nodes first to ensure they can see each other
        let ticket_a = node_a.ticket().await?;
        node_b.connect_by_ticket(&ticket_a).await?;

        // Wait for connection to be established
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Node A stores a move
        println!("Node A storing training move...");
        node_a
            .store_training_move(&game_id, 0, test_move_data)
            .await?;

        // Get node B's address and explicitly sync with it
        let node_b_addr = node_b.node_addr().await?;
        println!(
            "Explicitly syncing document with node B: {}",
            node_b_addr.node_id
        );
        node_a
            .sync_document_with_node(&game_id, &node_b_addr)
            .await?;

        // Give more time for the change to propagate (iroh-docs v0.35 might need more time)
        println!("Waiting for document synchronization...");
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Node B tries to fetch the document
        println!("Node B fetching training document...");
        let fetched_moves = timeout(
            Duration::from_secs(10), // Increase the timeout
            node_b.fetch_training_doc(&game_id),
        )
        .await??;

        // Verify that Node B received the move data
        assert!(
            !fetched_moves.is_empty(),
            "Node B should have received the document"
        );
        println!(
            "Document successfully synchronized, found {} moves",
            fetched_moves.len()
        );

        // Verify the content matches
        let first_move = &fetched_moves[0];
        assert_eq!(
            first_move, test_move_data,
            "Move data should match what was stored"
        );

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
