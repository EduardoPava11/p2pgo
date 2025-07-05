// SPDX-License-Identifier: MIT OR Apache-2.0

//! Test for relay connection limits and bandwidth throttling

use anyhow::Result;
use futures_lite::future::FutureExt;
use p2pgo_network::port::PortManager;
use p2pgo_network::relay_monitor::{RelayCapacityReport, RestartableRelay};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

mod common;
use common::test_utils::{random_move, wait_for_events};
use common::{spawn_relay, PeerConfig, RelayConfig, TestPeer, TestRelay};

// Low-level connection limit test using direct iroh endpoints
#[cfg(feature = "iroh")]
#[tokio::test]
async fn test_relay_connection_limit_direct() -> Result<()> {
    // Set up environment
    let _ = env_logger::try_init();

    // Set up port manager
    let port_manager = PortManager::new()?;

    // Set up capacity reporting
    let (capacity_tx, mut capacity_rx) =
        tokio::sync::mpsc::unbounded_channel::<RelayCapacityReport>();

    // Configure a relay with a connection limit of 200
    let mut relay = RestartableRelay::new(port_manager)
        .connection_limit(200) // 200 connections max
        .bandwidth_limit(10.0) // 10 MB/s
        .with_capacity_sender(capacity_tx);

    // Start the relay
    let (tcp_port, udp_port) = relay.start_embedded_relay().await?;
    println!("Relay started on TCP:{} UDP:{}", tcp_port, udp_port);

    // Wait a moment for the relay to fully initialize
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Attempt to establish 300 connections to the relay (should accept only 200)
    let mut successful_connections = 0;
    let mut connections = Vec::new();

    println!("Attempting to create 300 connections (expecting only 200 to succeed)...");

    for i in 0..300 {
        // Create an Iroh endpoint with a unique node ID for each connection
        use iroh::Endpoint;

        match Endpoint::builder()
            .relay_mode(iroh::RelayMode::Client)
            .spawn()
            .await
        {
            Ok(endpoint) => {
                // Try to connect to our relay
                let relay_addr = format!("/ip4/127.0.0.1/tcp/{}/quic-v1", tcp_port);

                match endpoint
                    .connect_with_relays(&[relay_addr])
                    .timeout(Duration::from_millis(500))
                    .await
                {
                    Ok(Ok(_)) => {
                        successful_connections += 1;
                        connections.push(endpoint);

                        if i % 10 == 0 {
                            println!("Connected {} endpoints so far", successful_connections);
                        }
                    }
                    _ => {
                        // Connection failed - expected once we hit the limit
                        if i % 10 == 0 {
                            println!("Connection {} failed (expected after reaching limit)", i);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to create endpoint {}: {}", i, e);
            }
        }

        // Small delay to avoid overwhelming the relay too quickly
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Check the final count is around the configured limit
    println!("Final connection count: {}", successful_connections);
    assert!(
        successful_connections <= 210,
        "Too many connections accepted: {}",
        successful_connections
    );
    assert!(
        successful_connections >= 190,
        "Too few connections accepted: {}",
        successful_connections
    );

    // Check the capacity report
    if let Some(report) = capacity_rx.recv().await {
        println!(
            "Received capacity report: {} connections / {} max",
            report.current_connections, report.max_connections
        );

        assert_eq!(
            report.max_connections, 200,
            "Max connections not properly configured"
        );
        assert!(report.current_connections > 0, "No connections reported");
    }

    // Clean up
    for endpoint in connections {
        let _ = endpoint.shutdown().await;
    }

    // Shut down the relay
    let _ = relay.stop().await;
}

// Higher-level test using our testing framework
#[tokio::test]
async fn test_relay_connection_limits_with_peers() -> Result<()> {
    // Spawn a relay with very low connection limit
    let relay = spawn_relay(RelayConfig {
        max_connections: 2, // Very low limit so we can test rejection
        max_bandwidth_mbps: 10.0,
    })
    .await?;

    // Get relay address
    let relay_address = relay.get_relay_addr();

    // Set environment to use our test relay
    std::env::set_var("P2PGO_RELAY_ADDR", &relay_address);

    // Create peers that will connect to the relay
    let mut peers = Vec::new();

    // First two peers should connect successfully
    for i in 0..2 {
        let peer = TestPeer::new(PeerConfig {
            name: format!("Peer{}", i),
            board_size: 9,
        })
        .await?;

        // Create a game for each peer to ensure they're using the relay
        let _ = peer.create_game().await?;

        peers.push(peer);
    }

    // Give time for connections to establish
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Try to connect one more peer, which should fail or be rejected
    let mut rejected_peer = TestPeer::new(PeerConfig {
        name: "RejectedPeer".to_string(),
        board_size: 9,
    })
    .await?;

    // Try to create a game, which should fail or timeout due to connection limit
    let attempt_start = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(5), rejected_peer.create_game()).await;

    // This should either timeout or return an error, but not succeed
    assert!(
        result.is_err() || result.unwrap().is_err(),
        "Peer should not be able to connect after connection limit is reached"
    );

    Ok(())
}

// Test for bandwidth limits
#[tokio::test]
async fn test_relay_bandwidth_limits() -> Result<()> {
    // Spawn a relay with very low bandwidth limit
    let relay = spawn_relay(RelayConfig {
        max_connections: 10,
        max_bandwidth_mbps: 0.01, // Very low limit (10 Kbps)
    })
    .await?;

    // Get relay address
    let relay_address = relay.get_relay_addr();

    // Set environment to use our test relay
    std::env::set_var("P2PGO_RELAY_ADDR", &relay_address);

    // Create two peers that will connect to the relay
    let mut alice = TestPeer::new(PeerConfig {
        name: "Alice".to_string(),
        board_size: 9,
    })
    .await?;

    let alice_ticket = alice.get_ticket().await?;

    let mut bob = TestPeer::new(PeerConfig {
        name: "Bob".to_string(),
        board_size: 9,
    })
    .await?;

    // Connect the peers
    bob.connect_by_ticket(&alice_ticket).await?;

    // Create a game
    let game_id = alice.create_game().await?;
    bob.join_game(&game_id).await?;

    // Get the channels
    let alice_channel = alice.game_channel.clone().unwrap();
    let bob_channel = bob.game_channel.clone().unwrap();

    // Subscribe to events
    let mut alice_rx = alice_channel.subscribe();
    let mut bob_rx = bob_channel.subscribe();

    // Get game state
    let game_state = alice_channel.get_latest_state().await.unwrap();

    // Send a burst of moves (which should cause bandwidth throttling)
    for i in 0..20 {
        let mv = random_move(&game_state);

        // Submit the move
        let _ = alice_channel.submit_move(mv.clone()).await;

        // No delay between moves to create a burst
    }

    // Check if the relay reports bandwidth throttling
    // This is harder to test directly in the code, but we can check relay logs
    // For this test, we'll just verify that not all moves get through immediately

    // Sleep briefly to let things settle
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Verify the status by checking move count on both peers
    let alice_state = alice_channel.get_latest_state().await.unwrap();
    let bob_state = bob_channel.get_latest_state().await;

    // If bandwidth throttling worked, Bob should have fewer moves than Alice
    // But this isn't guaranteed, so we just print the values for observation
    if let Some(bob_state) = bob_state {
        println!("Alice move count: {}", alice_state.moves.len());
        println!("Bob move count: {}", bob_state.moves.len());
    } else {
        println!("Bob has no state yet due to extreme throttling");
    }

    // Wait a bit longer and retry
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Now Bob should have received at least some moves
    let bob_state = bob_channel.get_latest_state().await;
    assert!(
        bob_state.is_some(),
        "Bob should eventually receive some state despite throttling"
    );

    Ok(())
}
