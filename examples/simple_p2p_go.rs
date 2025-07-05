// Simple P2P Go game networking demonstration
// Shows how peers can discover each other and share game moves

use libp2p::{
    futures::StreamExt, gossipsub, identity, mdns, noise, swarm::NetworkBehaviour, tcp, yamux,
    PeerId, SwarmBuilder,
};
use p2pgo_core::{Color, Coord, GameState, Move as GoMove};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::io::{self, AsyncBufReadExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameMessage {
    game_id: String,
    player: String,
    action: GameAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum GameAction {
    CreateGame { board_size: usize },
    JoinGame,
    PlaceStone { x: usize, y: usize },
    Pass,
    Resign,
}

#[derive(NetworkBehaviour)]
struct GameBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

struct P2PGoGame {
    game_state: GameState,
    peer_id: PeerId,
    opponent: Option<PeerId>,
}

impl P2PGoGame {
    fn new(peer_id: PeerId) -> Self {
        Self {
            game_state: GameState::new(19), // Standard 19x19 board
            peer_id,
            opponent: None,
        }
    }

    fn handle_move(&mut self, x: usize, y: usize) -> Result<(), String> {
        // Create a move with the current player's color
        let color = self.game_state.current_player;
        let coord = Coord::new(x, y);

        // Check if move is valid
        let idx = y * self.game_state.board_size as usize + x;
        if self.game_state.board[idx].is_some() {
            return Err("Position already occupied".to_string());
        }

        // Make the move
        self.game_state
            .apply_move(GoMove::Place {
                x: x as u8,
                y: y as u8,
                color,
            })
            .map_err(|e| format!("Invalid move: {:?}", e))?;

        println!(
            "Placed {} stone at ({}, {})",
            if color == Color::Black {
                "black"
            } else {
                "white"
            },
            x,
            y
        );

        Ok(())
    }

    fn display_board(&self) {
        println!("\nCurrent board state:");
        println!("   A B C D E F G H J K L M N O P Q R S T");

        for y in 0..self.game_state.board_size {
            print!("{:2} ", 19 - y);
            for x in 0..self.game_state.board_size {
                let idx = y as usize * self.game_state.board_size as usize + x as usize;
                match self.game_state.board[idx] {
                    None => print!(". "),
                    Some(Color::Black) => print!("● "),
                    Some(Color::White) => print!("○ "),
                }
            }
            println!("{:2}", 19 - y);
        }
        println!("   A B C D E F G H J K L M N O P Q R S T");
        println!(
            "Current turn: {}",
            if self.game_state.current_player == Color::Black {
                "Black"
            } else {
                "White"
            }
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create identity
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Starting P2P Go node with ID: {}", local_peer_id);

    // Create the game instance
    let mut game = P2PGoGame::new(local_peer_id);

    // Build the swarm
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|keypair| {
            // Configure Gossipsub
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .message_id_fn(message_id_fn)
                .build()
                .expect("Valid config");

            let mut gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(keypair.clone()),
                gossipsub_config,
            )?;

            // Subscribe to game topic
            gossipsub.subscribe(&gossipsub::IdentTopic::new("p2pgo/games"))?;

            // mDNS for local peer discovery
            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                keypair.public().to_peer_id(),
            )?;

            Ok(GameBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Listen on all interfaces
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Display instructions
    println!("\nP2P Go Game - Decentralized Go Playing");
    println!("=======================================");
    println!("Commands:");
    println!("  /new - Create a new game");
    println!("  /join <peer_id> - Join a game");
    println!("  /move <x> <y> - Place a stone (e.g., /move 3 3)");
    println!("  /pass - Pass your turn");
    println!("  /board - Display the current board");
    println!("  /peers - List discovered peers");
    println!("  /quit - Exit the game");

    game.display_board();

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            Ok(Some(line)) = stdin.next_line() => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                match parts.get(0).map(|s| s.as_ref()) {
                    Some("/new") => {
                        game = P2PGoGame::new(local_peer_id);
                        let msg = GameMessage {
                            game_id: uuid::Uuid::new_v4().to_string(),
                            player: local_peer_id.to_string(),
                            action: GameAction::CreateGame { board_size: 19 },
                        };
                        let data = serde_json::to_vec(&msg)?;
                        swarm.behaviour_mut().gossipsub.publish(
                            gossipsub::IdentTopic::new("p2pgo/games"),
                            data,
                        )?;
                        println!("Created new game!");
                        game.display_board();
                    }
                    Some("/move") => {
                        if let (Some(x_str), Some(y_str)) = (parts.get(1), parts.get(2)) {
                            if let (Ok(x), Ok(y)) = (x_str.parse::<usize>(), y_str.parse::<usize>()) {
                                match game.handle_move(x, y) {
                                    Ok(_) => {
                                        let msg = GameMessage {
                                            game_id: "current".to_string(),
                                            player: local_peer_id.to_string(),
                                            action: GameAction::PlaceStone { x, y },
                                        };
                                        let data = serde_json::to_vec(&msg)?;
                                        swarm.behaviour_mut().gossipsub.publish(
                                            gossipsub::IdentTopic::new("p2pgo/games"),
                                            data,
                                        )?;
                                        game.display_board();
                                    }
                                    Err(e) => println!("Move failed: {}", e),
                                }
                            } else {
                                println!("Invalid coordinates. Use numbers, e.g., /move 3 3");
                            }
                        } else {
                            println!("Usage: /move <x> <y>");
                        }
                    }
                    Some("/board") => {
                        game.display_board();
                    }
                    Some("/peers") => {
                        let peers: Vec<_> = swarm.connected_peers().cloned().collect();
                        if peers.is_empty() {
                            println!("No peers discovered yet");
                        } else {
                            println!("Discovered peers:");
                            for peer in peers {
                                println!("  - {}", peer);
                            }
                        }
                    }
                    Some("/quit") => {
                        println!("Thanks for playing P2P Go!");
                        break;
                    }
                    _ => {
                        println!("Unknown command. Available commands: /new, /move, /board, /peers, /quit");
                    }
                }
            }
            event = swarm.select_next_some() => {
                match event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on: {}", address);
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(GameBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                        for (peer_id, _) in peers {
                            println!("Discovered peer: {}", peer_id);
                        }
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(GameBehaviourEvent::Gossipsub(
                        gossipsub::Event::Message { message, .. }
                    )) => {
                        if let Ok(game_msg) = serde_json::from_slice::<GameMessage>(&message.data) {
                            println!("Received game message: {:?}", game_msg);
                            match game_msg.action {
                                GameAction::PlaceStone { x, y } => {
                                    println!("Opponent played at ({}, {})", x, y);
                                    // In a real implementation, we'd update our game state
                                }
                                GameAction::CreateGame { board_size } => {
                                    println!("New game created with board size {}", board_size);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
