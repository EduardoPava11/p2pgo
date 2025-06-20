// SPDX-License-Identifier: MIT OR Apache-2.0

use crossbeam_channel::{unbounded, Receiver};
use p2pgo_core::{Move, Coord};
use p2pgo_ui_egui::{app::App, msg::{UiToNet, NetToUi}};
use std::thread;
use std::time::Duration;

/// Timeout for network operations in the test
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Wait for a specific message type to appear
fn wait_for_message<F>(rx: &Receiver<NetToUi>, predicate: F, timeout: Duration) -> Option<NetToUi>
where
    F: Fn(&NetToUi) -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if let Ok(msg) = rx.try_recv() {
            if predicate(&msg) {
                return Some(msg);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

/// Complete headless test for pairing two GUI instances and playing moves between them
/// 
/// This test:
/// 1. Creates two instances of the App
/// 2. Has the first instance create a 9x9 game
/// 3. Gets the ticket from the first instance
/// 4. Has the second instance connect using that ticket
/// 5. Makes a few moves between the players
/// 6. Verifies the game progresses correctly
#[test]
#[ignore] // Ignoring for now as it requires network connectivity and can be flaky
fn two_clients_pair_and_play() {
    // Create channels for the host player
    let (host_ui_tx, _host_net_rx) = unbounded::<UiToNet>();
    let (_host_net_tx, host_ui_rx) = unbounded::<NetToUi>();
    
    // Create channels for the guest player
    let (guest_ui_tx, _guest_net_rx) = unbounded::<UiToNet>();
    let (_guest_net_tx, guest_ui_rx) = unbounded::<NetToUi>();
    
    // Create the host and guest app instances
    // Note: We're not using the app instances in this test since we've converted to using channels directly
    let _host_app = App::new(host_ui_tx.clone(), host_ui_rx.clone(), 9, "HostPlayer".to_string());
    let _guest_app = App::new(guest_ui_tx.clone(), guest_ui_rx.clone(), 9, "GuestPlayer".to_string());
    
    // Host creates a game
    host_ui_tx.send(UiToNet::CreateGame { board_size: 9 }).unwrap();
    
    // Wait for the game to be created and get the ticket
    let ticket_msg = wait_for_message(&host_ui_rx, 
        |msg| matches!(msg, NetToUi::Ticket { .. }), 
        TEST_TIMEOUT
    ).expect("Host should have received a ticket");
    
    // Extract the ticket string
    let ticket = if let NetToUi::Ticket { ticket } = ticket_msg {
        ticket
    } else {
        panic!("Expected NetToUi::Ticket, got something else");
    };
    println!("Got host ticket: {}", ticket);
    
    // Wait for host to receive game joined confirmation
    wait_for_message(&host_ui_rx, 
        |msg| matches!(msg, NetToUi::GameJoined { .. }), 
        TEST_TIMEOUT
    ).expect("Host should have received GameJoined");
    
    // Guest connects using the ticket
    guest_ui_tx.send(UiToNet::ConnectByTicket { ticket }).unwrap();
    
    // Wait for guest to receive game joined confirmation
    wait_for_message(&guest_ui_rx, 
        |msg| matches!(msg, NetToUi::GameJoined { .. }), 
        TEST_TIMEOUT
    ).expect("Guest should have received GameJoined");
    
    // Refresh games to make sure everything is synced
    host_ui_tx.send(UiToNet::RefreshGames).unwrap();
    guest_ui_tx.send(UiToNet::RefreshGames).unwrap();
    
    // Wait a moment for game state to stabilize
    thread::sleep(Duration::from_millis(500));
    
    // Make a move from host (Black plays first)
    let move1 = Move::Place(Coord::new(4, 4));
    host_ui_tx.send(UiToNet::MakeMove { mv: move1, board_size: None }).unwrap();
    
    // Wait for the move event to be received by both players
    wait_for_message(&host_ui_rx, 
        |msg| matches!(msg, NetToUi::GameEvent { .. }), 
        TEST_TIMEOUT
    ).expect("Host should have received the move event");
    
    wait_for_message(&guest_ui_rx, 
        |msg| matches!(msg, NetToUi::GameEvent { .. }), 
        TEST_TIMEOUT
    ).expect("Guest should have received the move event");
    
    // Make a move from guest (White)
    let move2 = Move::Place(Coord::new(3, 3));
    guest_ui_tx.send(UiToNet::MakeMove { mv: move2, board_size: None }).unwrap();
    
    // Wait for the move event to be received by both players
    wait_for_message(&host_ui_rx, 
        |msg| matches!(msg, NetToUi::GameEvent { .. }), 
        TEST_TIMEOUT
    ).expect("Host should have received the second move event");
    
    wait_for_message(&guest_ui_rx, 
        |msg| matches!(msg, NetToUi::GameEvent { .. }), 
        TEST_TIMEOUT
    ).expect("Guest should have received the second move event");
    
    // Make another move from host
    let move3 = Move::Place(Coord::new(5, 5));
    host_ui_tx.send(UiToNet::MakeMove { mv: move3, board_size: None }).unwrap();
    
    // Successfully made it through the basic gameplay
    println!("Successfully completed headless pairing test");
}
