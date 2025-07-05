//! Simple test to see Event structure
#[cfg(feature = "iroh")]
#[tokio::test]
async fn check_event_variants() {
    use iroh_gossip::net::Event;

    // Use match to see all variants the compiler knows about
    let dummy_event: Option<Event> = None;
    if let Some(event) = dummy_event {
        match event {
            Event::Gossip(gossip_event) => {
                // Check what GossipEvent contains
                println!("Gossip event: {:?}", gossip_event);
            }
            Event::Lagged => {
                println!("Lagged event");
            } // This will fail if we miss a variant
        }
    }
}
