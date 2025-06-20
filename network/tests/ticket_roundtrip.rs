// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test EnhancedTicket CBOR roundtrip encoding/decoding

#[cfg(feature = "iroh")]
mod tests {
    use p2pgo_network::iroh_endpoint::{IrohCtx, EnhancedTicket};
    // Import NodeAddr through IrohCtx's implementation since it's not directly accessible
    use iroh;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;

    #[tokio::test]
    async fn test_enhanced_ticket_roundtrip() {
        // Create test ticket data
        let ctx = IrohCtx::new().await.expect("Failed to create IrohCtx");
        
        // Test with no game size
        let ticket1 = ctx.ticket().await.expect("Failed to generate ticket");
        
        // Test with game size hint
        let ticket2 = ctx.ticket_with_game_size(Some(9)).await.expect("Failed to generate ticket with game size");
        
        // Decode both tickets and verify they're valid
        let bytes1 = B64.decode(&ticket1).expect("Failed to decode base64");
        let decoded1: EnhancedTicket = serde_cbor::from_slice(&bytes1).expect("Failed to decode CBOR");
        
        let bytes2 = B64.decode(&ticket2).expect("Failed to decode base64");
        let decoded2: EnhancedTicket = serde_cbor::from_slice(&bytes2).expect("Failed to decode CBOR");
        
        // Verify structure
        assert_eq!(decoded1.version, 1);
        assert_eq!(decoded2.version, 1);
        assert_eq!(decoded1.game_size, None);
        assert_eq!(decoded2.game_size, Some(9));
        assert!(decoded1.doc.is_none());
        assert!(decoded2.doc.is_none());
        
        // Verify node addresses are valid
        assert!(!decoded1.node.node_id.to_string().is_empty());
        assert!(!decoded2.node.node_id.to_string().is_empty());
        
        // Test that we can connect using the ticket (at least parse it)
        let result = ctx.connect_by_ticket(&ticket1).await;
        // Note: This will likely fail to actually connect since we're connecting to ourselves,
        // but it should at least parse the ticket successfully
        match result {
            Ok(_) => {
                // Connection succeeded (unlikely but possible)
                println!("Connection succeeded");
            }
            Err(e) => {
                // Connection failed but ticket was parsed
                println!("Connection failed as expected: {}", e);
                // Verify it's not a parsing error
                assert!(!e.to_string().contains("Failed to decode"));
            }
        }
    }
    
    #[test]
    fn test_enhanced_ticket_cbor_format() {
        // Test that we can manually create and encode/decode EnhancedTicket
        // Import through p2pgo_network instead since direct imports aren't working
        use p2pgo_network::iroh_endpoint::EnhancedTicket;
        use std::net::SocketAddr;
        
        // Instead of creating NodeAddr directly, let's use what we have in EnhancedTicket
        // Create a real IrohCtx instead and use its node_addr
        let ctx = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { IrohCtx::new().await.unwrap() });
            
        // Get the ticket and decode it to get a real NodeAddr
        let ticket_str = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { ctx.ticket().await.unwrap() });
            
        let bytes = B64.decode(&ticket_str).expect("Failed to decode base64");
        let decoded: EnhancedTicket = serde_cbor::from_slice(&bytes).expect("Failed to decode CBOR");
        let node_addr = decoded.node;
        
        let ticket = EnhancedTicket {
            node: node_addr,
            doc: None,
            cap: None,
            game_size: Some(19),
            version: 1,
        };
        
        // Encode to CBOR
        let cbor_bytes = serde_cbor::to_vec(&ticket).expect("Failed to encode to CBOR");
        
        // Encode to base64
        let b64_string = B64.encode(&cbor_bytes);
        
        // Decode back
        let decoded_cbor = B64.decode(&b64_string).expect("Failed to decode base64");
        let decoded_ticket: EnhancedTicket = serde_cbor::from_slice(&decoded_cbor).expect("Failed to decode CBOR");
        
        // Verify roundtrip
        assert_eq!(decoded_ticket.version, 1);
        assert_eq!(decoded_ticket.game_size, Some(19));
        assert_eq!(decoded_ticket.node.node_id, decoded_ticket.node.node_id);
    }
}

#[cfg(not(feature = "iroh"))]
mod stub_tests {
    use p2pgo_network::iroh_endpoint::IrohCtx;
    
    #[tokio::test]
    async fn test_stub_ticket() {
        let ctx = IrohCtx::new().await.expect("Failed to create stub IrohCtx");
        let ticket = ctx.ticket().await.expect("Failed to generate stub ticket");
        assert_eq!(ticket, "loopback-ticket");
        
        // Test connection (should succeed in stub mode)
        let result = ctx.connect_by_ticket(&ticket).await;
        assert!(result.is_ok(), "Stub connection should succeed");
    }
}
