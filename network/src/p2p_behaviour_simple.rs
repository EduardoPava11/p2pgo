//! Simplified P2P behavior implementation that works with libp2p 0.53
//!
//! This provides the core P2P functionality using existing patterns
//! from the codebase while incorporating Circuit Relay V2 properly.

use libp2p::{
    identity::Keypair,
    PeerId,
    swarm::NetworkBehaviour,
    kad::{Behaviour as Kademlia, Config as KademliaConfig, Event as KademliaEvent, store::MemoryStore, RecordKey, Record, Quorum},
    dcutr,
    identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    autonat::{Behaviour as Autonat, Event as AutonatEvent},
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent, MessageAuthenticity, ConfigBuilder as GossipsubConfigBuilder, ValidationMode, IdentTopic},
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    allow_block_list::{Behaviour as AllowBlockList, BlockedPeers},
};

use std::time::Duration;
use anyhow::Result;
use tracing::info;

/// P2P behavior without relay server (most nodes)
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "P2PEvent")]
pub struct P2PBehaviour {
    /// Kademlia DHT for decentralized discovery and storage
    pub kademlia: Kademlia<MemoryStore>,
    
    /// Direct connection upgrade through relay
    pub dcutr: dcutr::Behaviour,
    
    /// Identify protocol for peer information exchange
    pub identify: Identify,
    
    /// AutoNAT for NAT detection
    pub autonat: Autonat,
    
    /// GossipSub for game state propagation
    pub gossipsub: Gossipsub,
    
    /// mDNS for local peer discovery
    pub mdns: Mdns,
    
    /// Connection allow/block list
    pub blocked_peers: AllowBlockList<BlockedPeers>,
}

/// P2P behavior with relay server capability
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "P2PRelayEvent")]
pub struct P2PRelayBehaviour {
    /// Kademlia DHT for decentralized discovery and storage
    pub kademlia: Kademlia<MemoryStore>,
    
    /// Relay server behavior
    pub relay: libp2p::relay::Behaviour,
    
    /// Direct connection upgrade through relay
    pub dcutr: dcutr::Behaviour,
    
    /// Identify protocol for peer information exchange
    pub identify: Identify,
    
    /// AutoNAT for NAT detection
    pub autonat: Autonat,
    
    /// GossipSub for game state propagation
    pub gossipsub: Gossipsub,
    
    /// mDNS for local peer discovery
    pub mdns: Mdns,
    
    /// Connection allow/block list
    pub blocked_peers: AllowBlockList<BlockedPeers>,
}

/// Events from P2P behavior
#[derive(Debug)]
pub enum P2PEvent {
    Kademlia(KademliaEvent),
    Dcutr(dcutr::Event),
    Identify(IdentifyEvent),
    Autonat(AutonatEvent),
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
    BlockedPeers(void::Void),
}

/// Events from P2P relay behavior
#[derive(Debug)]
pub enum P2PRelayEvent {
    Kademlia(KademliaEvent),
    Relay(libp2p::relay::Event),
    Dcutr(dcutr::Event),
    Identify(IdentifyEvent),
    Autonat(AutonatEvent),
    Gossipsub(GossipsubEvent),
    Mdns(MdnsEvent),
    BlockedPeers(void::Void),
}

// Event conversions for P2PBehaviour
impl From<KademliaEvent> for P2PEvent {
    fn from(event: KademliaEvent) -> Self {
        P2PEvent::Kademlia(event)
    }
}

impl From<dcutr::Event> for P2PEvent {
    fn from(event: dcutr::Event) -> Self {
        P2PEvent::Dcutr(event)
    }
}

impl From<IdentifyEvent> for P2PEvent {
    fn from(event: IdentifyEvent) -> Self {
        P2PEvent::Identify(event)
    }
}

impl From<AutonatEvent> for P2PEvent {
    fn from(event: AutonatEvent) -> Self {
        P2PEvent::Autonat(event)
    }
}

impl From<GossipsubEvent> for P2PEvent {
    fn from(event: GossipsubEvent) -> Self {
        P2PEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for P2PEvent {
    fn from(event: MdnsEvent) -> Self {
        P2PEvent::Mdns(event)
    }
}

impl From<void::Void> for P2PEvent {
    fn from(event: void::Void) -> Self {
        P2PEvent::BlockedPeers(event)
    }
}

// Event conversions for P2PRelayBehaviour
impl From<KademliaEvent> for P2PRelayEvent {
    fn from(event: KademliaEvent) -> Self {
        P2PRelayEvent::Kademlia(event)
    }
}

impl From<libp2p::relay::Event> for P2PRelayEvent {
    fn from(event: libp2p::relay::Event) -> Self {
        P2PRelayEvent::Relay(event)
    }
}

impl From<dcutr::Event> for P2PRelayEvent {
    fn from(event: dcutr::Event) -> Self {
        P2PRelayEvent::Dcutr(event)
    }
}

impl From<IdentifyEvent> for P2PRelayEvent {
    fn from(event: IdentifyEvent) -> Self {
        P2PRelayEvent::Identify(event)
    }
}

impl From<AutonatEvent> for P2PRelayEvent {
    fn from(event: AutonatEvent) -> Self {
        P2PRelayEvent::Autonat(event)
    }
}

impl From<GossipsubEvent> for P2PRelayEvent {
    fn from(event: GossipsubEvent) -> Self {
        P2PRelayEvent::Gossipsub(event)
    }
}

impl From<MdnsEvent> for P2PRelayEvent {
    fn from(event: MdnsEvent) -> Self {
        P2PRelayEvent::Mdns(event)
    }
}

impl From<void::Void> for P2PRelayEvent {
    fn from(event: void::Void) -> Self {
        P2PRelayEvent::BlockedPeers(event)
    }
}

/// Helper functions for creating behaviors
impl P2PBehaviour {
    /// Create a new P2P behavior without relay server
    pub fn new(keypair: &Keypair) -> Result<Self> {
        let peer_id = PeerId::from(keypair.public());
        
        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        
        let kademlia = Kademlia::with_config(
            peer_id,
            MemoryStore::new(peer_id),
            kad_config,
        );
        
        // DCUtR for connection upgrade
        let dcutr = dcutr::Behaviour::new(peer_id);
        
        // Identify
        let identify = Identify::new(IdentifyConfig::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));
        
        // AutoNAT
        let autonat = Autonat::new(peer_id, Default::default());
        
        // GossipSub
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .build()
            .expect("Valid config");
            
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).expect("Valid gossipsub");
        
        // mDNS
        let mdns = Mdns::new(Default::default(), peer_id)?;
        
        // Blocked peers
        let blocked_peers = AllowBlockList::default();
        
        Ok(Self {
            kademlia,
            dcutr,
            identify,
            autonat,
            gossipsub,
            mdns,
            blocked_peers,
        })
    }
    
    /// Bootstrap the node
    pub async fn bootstrap(&mut self) -> Result<()> {
        // Bootstrap Kademlia
        self.kademlia.bootstrap()?;
        
        // Subscribe to topics
        self.gossipsub.subscribe(&gossipsub_topic("games"))?;
        self.gossipsub.subscribe(&gossipsub_topic("relays"))?;
        
        info!("P2P node bootstrapped");
        Ok(())
    }
    
    /// Advertise relay capability
    pub async fn advertise_relay(&mut self) -> Result<()> {
        let key = RecordKey::new(&"/p2pgo/relays/1.0.0");
        let record = Record {
            key: key.clone(),
            value: vec![],
            publisher: None,
            expires: None,
        };
        
        self.kademlia.put_record(record, Quorum::One)?;
        info!("Advertised relay capability");
        Ok(())
    }
}

impl P2PRelayBehaviour {
    /// Create a new P2P behavior with relay server
    pub fn new(keypair: &Keypair) -> Result<Self> {
        let peer_id = PeerId::from(keypair.public());
        
        // Configure Kademlia
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        
        let kademlia = Kademlia::with_config(
            peer_id,
            MemoryStore::new(peer_id),
            kad_config,
        );
        
        // Relay server
        let relay = libp2p::relay::Behaviour::new(
            peer_id,
            libp2p::relay::Config::default(),
        );
        
        // DCUtR for connection upgrade
        let dcutr = dcutr::Behaviour::new(peer_id);
        
        // Identify
        let identify = Identify::new(IdentifyConfig::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));
        
        // AutoNAT
        let autonat = Autonat::new(peer_id, Default::default());
        
        // GossipSub
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(ValidationMode::Strict)
            .build()
            .expect("Valid config");
            
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).expect("Valid gossipsub");
        
        // mDNS
        let mdns = Mdns::new(Default::default(), peer_id)?;
        
        // Blocked peers
        let blocked_peers = AllowBlockList::default();
        
        Ok(Self {
            kademlia,
            relay,
            dcutr,
            identify,
            autonat,
            gossipsub,
            mdns,
            blocked_peers,
        })
    }
}

/// Create a gossipsub topic
fn gossipsub_topic(name: &str) -> IdentTopic {
    IdentTopic::new(format!("/p2pgo/{}/1.0.0", name))
}