// Simple P2P networking demo for P2P Go game
// This demonstrates the decentralized peer-to-peer capabilities

use libp2p::{
    gossipsub, identity, kad, mdns, noise, relay, swarm::NetworkBehaviour, tcp, yamux,
    PeerId, SwarmBuilder, futures::StreamExt,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt};

// Custom network behaviour for P2P Go
#[derive(NetworkBehaviour)]
struct P2PGoBehaviour {
    relay: relay::client::Behaviour,
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    mdns: mdns::tokio::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a random peer ID
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {}", local_peer_id);

    // Create a Swarm to manage peers and connections
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|keypair, relay_behavior| {
            // Create a Gossipsub topic for game updates
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .expect("Valid config");

            let mut gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                gossipsub_config,
            )
            .expect("Correct configuration");

            // Create topics for different game channels
            let game_topic = gossipsub::IdentTopic::new("p2pgo/games/v1");
            let lobby_topic = gossipsub::IdentTopic::new("p2pgo/lobby/v1");
            let rna_topic = gossipsub::IdentTopic::new("p2pgo/rna/v1");

            gossipsub.subscribe(&game_topic)?;
            gossipsub.subscribe(&lobby_topic)?;
            gossipsub.subscribe(&rna_topic)?;

            // Create Kademlia for peer discovery
            let mut cfg = kad::Config::default();
            cfg.set_query_timeout(Duration::from_secs(5 * 60));
            let store = kad::store::MemoryStore::new(keypair.public().to_peer_id());
            let kad = kad::Behaviour::with_config(keypair.public().to_peer_id(), store, cfg);

            // mDNS for local discovery
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), keypair.public().to_peer_id())?;

            Ok(P2PGoBehaviour {
                relay: relay_behavior,
                gossipsub,
                kad,
                mdns,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on all interfaces and a random OS-assigned port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Read commands from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    println!("P2P Go Network Demo");
    println!("Commands:");
    println!("  /peers - List connected peers");
    println!("  /game <message> - Broadcast game update");
    println!("  /lobby <message> - Broadcast lobby message");
    println!("  /rna <message> - Share training data");
    println!("  /discover - Start peer discovery");

    loop {
        tokio::select! {
            Ok(Some(line)) = stdin.next_line() => {
                let mut parts = line.split(' ');
                match parts.next() {
                    Some("/peers") => {
                        let peers: Vec<_> = swarm.connected_peers().cloned().collect();
                        if peers.is_empty() {
                            println!("No connected peers");
                        } else {
                            println!("Connected peers:");
                            for peer in peers {
                                println!("  {}", peer);
                            }
                        }
                    }
                    Some("/game") => {
                        let msg = parts.collect::<Vec<_>>().join(" ");
                        let topic = gossipsub::IdentTopic::new("p2pgo/games/v1");
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, msg.as_bytes()) {
                            println!("Failed to publish game message: {:?}", e);
                        } else {
                            println!("Published game update: {}", msg);
                        }
                    }
                    Some("/lobby") => {
                        let msg = parts.collect::<Vec<_>>().join(" ");
                        let topic = gossipsub::IdentTopic::new("p2pgo/lobby/v1");
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, msg.as_bytes()) {
                            println!("Failed to publish lobby message: {:?}", e);
                        } else {
                            println!("Published lobby message: {}", msg);
                        }
                    }
                    Some("/rna") => {
                        let msg = parts.collect::<Vec<_>>().join(" ");
                        let topic = gossipsub::IdentTopic::new("p2pgo/rna/v1");
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, msg.as_bytes()) {
                            println!("Failed to publish RNA message: {:?}", e);
                        } else {
                            println!("Shared training data: {}", msg);
                        }
                    }
                    Some("/discover") => {
                        swarm.behaviour_mut().kad.bootstrap()?;
                        println!("Started DHT bootstrap for peer discovery");
                    }
                    _ => {
                        println!("Unknown command. Type /help for commands.");
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on: {}", address);
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(P2PGoBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                        for (peer_id, _) in peers {
                            println!("Discovered peer via mDNS: {}", peer_id);
                            swarm.behaviour_mut().kad.add_address(&peer_id, "/ip4/127.0.0.1/tcp/0".parse().unwrap());
                        }
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(P2PGoBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        let topic = message.topic.clone();
                        let data = String::from_utf8_lossy(&message.data);
                        println!(
                            "Received message on {}: '{}' from {} (id: {})",
                            topic, data, peer_id, id
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}