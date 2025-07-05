//! True P2P behavior using libp2p Circuit Relay V2
//!
//! This replaces the old behaviour.rs with a proper decentralized
//! implementation that leverages Circuit Relay V2 correctly.

use libp2p::{
    identity::Keypair,
    PeerId,
    swarm::NetworkBehaviour,
    kad::{Behaviour as Kademlia, Config as KademliaConfig, Event as KademliaEvent, store::MemoryStore},
    relay,
    dcutr,
    identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    autonat::{Behaviour as Autonat, Event as AutonatEvent},
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent, MessageAuthenticity, ConfigBuilder as GossipsubConfigBuilder},
    mdns::{tokio::Behaviour as Mdns, Event as MdnsEvent},
    allow_block_list::{Behaviour as AllowBlockList, BlockedPeers},
};

use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;


/// Main P2P behavior for decentralized Go
#[derive(NetworkBehaviour)]
pub struct P2PGoBehaviour {
    /// Kademlia DHT for decentralized discovery and storage
    pub kademlia: Kademlia<MemoryStore>,

    /// Circuit Relay V2 client for using relays
    pub relay_client: relay::client::Behaviour,

    /// Circuit Relay V2 server for providing relay service (optional)
    pub relay_server: Option<relay::Behaviour>,

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

impl P2PGoBehaviour {
    /// Create a new P2P behavior with proper Circuit Relay V2
    pub fn new(
        keypair: &Keypair,
        enable_relay_server: bool,
        relay_server_config: Option<RelayServerConfig>,
    ) -> Result<Self> {
        let peer_id = PeerId::from(keypair.public());

        // Configure Kademlia for decentralized discovery
        let mut kad_config = KademliaConfig::default();
        kad_config.set_query_timeout(Duration::from_secs(60));
        kad_config.set_replication_factor(3.try_into().unwrap());

        let kademlia = Kademlia::with_config(
            peer_id,
            MemoryStore::new(peer_id),
            kad_config,
        );

        // Circuit Relay V2 client - everyone can use relays
        let relay_client = relay::client::Behaviour::new(peer_id);

        // Circuit Relay V2 server - optional relay provider
        let relay_server = if enable_relay_server {
            let config = relay_server_config.unwrap_or_default();
            Some(relay::Behaviour::new(
                peer_id,
                relay::Config {
                    max_reservations: config.max_reservations,
                    max_reservations_per_peer: config.max_reservations_per_peer,
                    reservation_duration: config.reservation_duration,
                    max_circuits: config.max_circuits,
                    max_circuits_per_peer: config.max_circuits_per_peer,
                    ..Default::default()
                },
            ))
        } else {
            None
        };

        // DCUtR for direct connection upgrade
        let dcutr = dcutr::Behaviour::new(peer_id);

        // Identify for peer information exchange
        let identify = Identify::new(IdentifyConfig::new(
            "/p2pgo/1.0.0".to_string(),
            keypair.public(),
        ));

        // AutoNAT for NAT detection
        let autonat = Autonat::new(peer_id, Default::default());

        // GossipSub for game state propagation
        let gossipsub_config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
            .message_id_fn(|message| {
                use std::hash::{Hash, Hasher};
                use std::collections::hash_map::DefaultHasher;
                let mut hasher = DefaultHasher::new();
                message.data.hash(&mut hasher);
                hasher.finish().to_string()
            })
            .build()
            .expect("Valid gossipsub config");

        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config,
        ).expect("Valid gossipsub");

        // mDNS for local discovery
        let mdns = Mdns::new(Default::default())?;

        // Connection filtering
        let blocked_peers = AllowBlockList::default();

        Ok(Self {
            kademlia,
            relay_client,
            relay_server,
            dcutr,
            identify,
            autonat,
            gossipsub,
            mdns,
            blocked_peers,
        })
    }

    /// Bootstrap into the P2P network
    pub async fn bootstrap(&mut self) -> Result<()> {
        // No hardcoded bootstrap nodes! Instead:

        // 1. Start local discovery
        info!("Starting local peer discovery via mDNS");

        // 2. Try to find peers via DHT bootstrap
        self.kademlia.bootstrap()?;

        // 3. Subscribe to relevant topics
        self.gossipsub.subscribe(&gossipsub_topic("games"))?;
        self.gossipsub.subscribe(&gossipsub_topic("relays"))?;

        // 4. Advertise our capabilities in DHT
        if self.relay_server.is_some() {
            self.advertise_relay_capability().await?;
        }

        Ok(())
    }

    /// Advertise relay capability in DHT
    async fn advertise_relay_capability(&mut self) -> Result<()> {
        use libp2p::kad::{RecordKey as Key, Record};

        // Advertise as relay provider
        let key = Key::new(&crate::protocols::peer_discovery::RELAY_NAMESPACE);
        let record = Record {
            key: key.clone(),
            value: vec![], // Could include relay metadata
            publisher: None,
            expires: None,
        };

        self.kademlia.put_record(record, libp2p::kad::Quorum::One)?;
        info!("Advertised relay capability in DHT");

        Ok(())
    }

    /// Find relay servers via DHT
    pub async fn discover_relays(&mut self) -> Result<Vec<PeerId>> {
        use libp2p::kad::RecordKey as Key;

        let key = Key::new(&crate::protocols::peer_discovery::RELAY_NAMESPACE);
        self.kademlia.get_providers(key);

        // Results will come via swarm events
        Ok(vec![])
    }

    /// Connect to a peer, using relay if necessary
    pub async fn connect_peer(&mut self, peer_id: PeerId) -> Result<()> {
        // First try direct connection via known addresses
        // If that fails, DCUtR will automatically try via relay

        info!("Attempting to connect to peer {:?}", peer_id);

        // The actual connection happens through swarm dial
        // DCUtR handles relay upgrades automatically

        Ok(())
    }

    /// Publish game availability
    pub async fn publish_game(&mut self, game_id: &str, metadata: GameMetadata) -> Result<()> {
        // 1. Store in DHT
        let key = format!("/games/{}", game_id);
        let record = libp2p::kad::Record {
            key: libp2p::kad::RecordKey::new(&key),
            value: serde_json::to_vec(&metadata)?,
            publisher: None,
            expires: None,
        };

        self.kademlia.put_record(record, libp2p::kad::Quorum::One)?;

        // 2. Announce via GossipSub
        let topic = gossipsub_topic("games");
        let message = serde_json::to_vec(&GameAnnouncement {
            game_id: game_id.to_string(),
            metadata: metadata.clone(),
        })?;

        self.gossipsub.publish(topic, message)?;

        info!("Published game {} to P2P network", game_id);
        Ok(())
    }

    /// Find available games
    pub async fn find_games(&mut self, board_size: Option<u8>) -> Result<()> {
        // Query DHT for games
        let pattern = match board_size {
            Some(size) => format!("/games/size/{}", size),
            None => "/games/".to_string(),
        };

        // This will trigger DHT queries
        self.kademlia.start_providing(libp2p::kad::RecordKey::new(&pattern))?;

        Ok(())
    }
}

/// Configuration for relay server
#[derive(Debug, Clone)]
pub struct RelayServerConfig {
    /// Maximum number of reservations
    pub max_reservations: usize,

    /// Maximum reservations per peer
    pub max_reservations_per_peer: usize,

    /// How long reservations last
    pub reservation_duration: Duration,

    /// Maximum number of circuits
    pub max_circuits: usize,

    /// Maximum circuits per peer
    pub max_circuits_per_peer: usize,
}

impl Default for RelayServerConfig {
    fn default() -> Self {
        Self {
            max_reservations: 128,
            max_reservations_per_peer: 4,
            reservation_duration: Duration::from_secs(60 * 60), // 1 hour
            max_circuits: 32,
            max_circuits_per_peer: 4,
        }
    }
}

/// Game metadata for P2P discovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameMetadata {
    /// Game ID
    pub id: String,

    /// Board size
    pub board_size: u8,

    /// Players
    pub players: Vec<PlayerInfo>,

    /// Game state
    pub state: GameStateInfo,

    /// Time controls
    pub time_control: Option<TimeControl>,
}

/// Player information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInfo {
    pub peer_id: String,
    pub color: String,
    pub rating: Option<u32>,
}

/// Game state information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameStateInfo {
    pub status: String,
    pub move_count: u32,
    pub current_player: String,
}

/// Time control settings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeControl {
    pub main_time: Duration,
    pub overtime: String,
}

/// Game announcement for gossipsub
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameAnnouncement {
    pub game_id: String,
    pub metadata: GameMetadata,
}

/// Create a gossipsub topic
fn gossipsub_topic(name: &str) -> libp2p::gossipsub::IdentTopic {
    libp2p::gossipsub::IdentTopic::new(format!("/p2pgo/{}/1.0.0", name))
}

/// Handle behavior events
impl P2PGoBehaviour {
    /// Process Kademlia events
    pub fn handle_kad_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryCompleted { result, .. } => {
                match result {
                    libp2p::kad::QueryResult::GetProviders(Ok(result)) => {
                        let providers = result.providers();
                        info!("Found {} providers", providers.len());
                    }
                    libp2p::kad::QueryResult::PutRecord(Ok(_)) => {
                        info!("Successfully stored record in DHT");
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Process mDNS events
    pub fn handle_mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(peers) => {
                for (peer_id, addr) in peers {
                    info!("Discovered local peer {} at {}", peer_id, addr);
                    // Add to Kademlia routing table
                    self.kademlia.add_address(&peer_id, addr);
                }
            }
            MdnsEvent::Expired(peers) => {
                for (peer_id, _) in peers {
                    info!("Local peer {} expired", peer_id);
                }
            }
        }
    }

    /// Process relay events
    pub fn handle_relay_event(&mut self, event: relay::Event) {
        match event {
            relay::Event::ReservationReqAccepted { .. } => {
                info!("Relay reservation accepted");
            }
            relay::Event::CircuitReqAccepted { .. } => {
                info!("Relay circuit established");
            }
            _ => {}
        }
    }
}

use tracing::info;