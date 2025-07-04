use libp2p::{
    autonat, dcutr, gossipsub, identify, kad, mdns,
    swarm::NetworkBehaviour,
    PeerId,
};
use std::time::Duration;

/// Main network behaviour for P2P Go
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "Event")]
pub struct P2PGoBehaviour {
    // Relay client removed for libp2p 0.53 compatibility
    // TODO: Add relay support when API stabilizes
    
    /// DHT for peer discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    
    /// Gossipsub for game data and RNA propagation
    pub gossipsub: gossipsub::Behaviour,
    
    /// Direct connection upgrade through relay
    pub dcutr: dcutr::Behaviour,
    
    /// Peer identification (required for Kademlia)
    pub identify: identify::Behaviour,
    
    /// AutoNAT for external address detection
    pub autonat: autonat::Behaviour,
    
    /// mDNS for local network discovery
    pub mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum Event {
    Kademlia(kad::Event),
    Gossipsub(gossipsub::Event),
    Dcutr(dcutr::Event),
    Identify(identify::Event),
    Autonat(autonat::Event),
    Mdns(mdns::Event),
}


impl From<kad::Event> for Event {
    fn from(event: kad::Event) -> Self {
        Event::Kademlia(event)
    }
}

impl From<gossipsub::Event> for Event {
    fn from(event: gossipsub::Event) -> Self {
        Event::Gossipsub(event)
    }
}

impl From<dcutr::Event> for Event {
    fn from(event: dcutr::Event) -> Self {
        Event::Dcutr(event)
    }
}

impl From<identify::Event> for Event {
    fn from(event: identify::Event) -> Self {
        Event::Identify(event)
    }
}

impl From<autonat::Event> for Event {
    fn from(event: autonat::Event) -> Self {
        Event::Autonat(event)
    }
}

impl From<mdns::Event> for Event {
    fn from(event: mdns::Event) -> Self {
        Event::Mdns(event)
    }
}

impl P2PGoBehaviour {
    pub fn new(local_peer_id: PeerId, keypair: &libp2p::identity::Keypair) -> anyhow::Result<Self> {
        // Configure Kademlia
        let mut kad_config = kad::Config::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        let store = kad::store::MemoryStore::new(local_peer_id);
        let kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // Configure Gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .mesh_n(6)
            .mesh_n_low(4)
            .mesh_n_high(12)
            .build()
            .map_err(|e| anyhow::anyhow!("Gossipsub config error: {}", e))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).map_err(|e| anyhow::anyhow!("Failed to create gossipsub: {}", e))?;

        // Configure Identify
        let identify = identify::Behaviour::new(identify::Config::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));

        // Configure AutoNAT
        let autonat = autonat::Behaviour::new(
            local_peer_id,
            autonat::Config {
                retry_interval: Duration::from_secs(10),
                refresh_interval: Duration::from_secs(30),
                boot_delay: Duration::from_secs(5),
                throttle_server_period: Duration::from_secs(10),
                only_global_ips: false,
                ..Default::default()
            },
        );

        Ok(Self {
            kademlia,
            gossipsub,
            dcutr: dcutr::Behaviour::new(local_peer_id),
            identify,
            autonat,
            mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
        })
    }
}