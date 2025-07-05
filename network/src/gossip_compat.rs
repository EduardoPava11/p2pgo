//! Helper module for GossipEvent handling in iroh-gossip v0.35
//!
//! Note: This is a compatibility layer for working with iroh-gossip v0.35, which
//! has a different GossipEvent structure than previous versions.

#[cfg(feature = "iroh")]
use iroh_gossip::net::{Event, GossipEvent};

/// Helper function to get message content from GossipEvent
///
/// In iroh-gossip v0.35, we need to extract content from the GossipEvent::Received variant
#[cfg(feature = "iroh")]
pub fn extract_bytes(event: &Event) -> Option<Vec<u8>> {
    match event {
        Event::Gossip(GossipEvent::Received(message)) => {
            // Access the message content directly and convert Bytes to Vec<u8>
            Some(message.content.to_vec())
        }
        // For other variants (Joined, NeighborUp, NeighborDown), return None
        _ => None,
    }
}

/// Check if this is a received message event
#[cfg(feature = "iroh")]
pub fn is_received_message(event: &Event) -> bool {
    matches!(event, Event::Gossip(GossipEvent::Received(_)))
}
