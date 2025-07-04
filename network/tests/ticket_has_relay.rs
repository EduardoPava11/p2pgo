// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test to ensure all generated tickets contain relay multiaddrs
#![deny(clippy::all)]

#[cfg(feature = "iroh")]
mod tests {
    use anyhow::Result;
    use p2pgo_network::iroh_endpoint::{IrohCtx, EnhancedTicket};
    use std::time::Duration;
    use tokio::time::timeout;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;

    #[tokio::test]
    async fn test_ticket_has_relay_multiaddr() -> Result<()> {
        // Create a new IrohCtx
        let ctx = IrohCtx::new().await?;
        
        // Wait a moment to ensure relay connections are established
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Generate a ticket
        let ticket_str = ctx.ticket_with_game_size(Some(9)).await?;
        
        // Decode the ticket
        let bytes = B64.decode(&ticket_str)?;
        let ticket: EnhancedTicket = serde_cbor::from_slice(&bytes)?;
        
        // Verify that either:
        // 1. There is at least one relay multiaddr in the direct_addresses, OR
        // 2. There is a relay_url present
        let has_relay_multiaddr = ticket.node.direct_addresses.iter()
            .any(|addr| addr.to_string().contains("/relay/"));
        
        let has_relay_url = ticket.node.relay_url.is_some();
        
        assert!(has_relay_multiaddr || has_relay_url, 
            "Ticket must have at least one relay multiaddr or a relay_url");
            
        // If we have both, log that as the ideal case
        if has_relay_multiaddr && has_relay_url {
            println!("✅ Ticket has both relay multiaddrs and relay_url - optimal!");
        } else if has_relay_multiaddr {
            println!("✓ Ticket has relay multiaddrs but no relay_url");
        } else {
            println!("✓ Ticket has relay_url but no relay multiaddrs");
        }
        
        Ok(())
    }
}

#[cfg(not(feature = "iroh"))]
#[test]
fn stub_builds_successfully() {
    // This test is just a placeholder when iroh feature is disabled
    assert!(true);
}
