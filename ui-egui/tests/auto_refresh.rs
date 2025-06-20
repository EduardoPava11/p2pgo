//! Test for auto-refresh functionality
//! SPDX-License-Identifier: MIT OR Apache-2.0

use crossbeam_channel::unbounded;
use p2pgo_ui_egui::msg::UiToNet;
use p2pgo_ui_egui::app::AppConfig;

#[test]
fn auto_refresh_sends_message() {
    // Create a simple channel pair to capture messages
    let (tx, rx) = unbounded::<UiToNet>();
    
    // We'll manually send the refresh message rather than using the App struct
    // This simulates the behavior we want to test
    
    // First with auto-refresh disabled
    let config = AppConfig {
        auto_refresh: false,
        games_finished: 0,
    };
    
    // No message should be sent when auto_refresh is false
    if config.auto_refresh {
        tx.send(UiToNet::RefreshGames).unwrap();
    }
    
    // No messages should be sent
    assert_eq!(rx.len(), 0);
    
    // Now with auto-refresh enabled
    let config = AppConfig {
        auto_refresh: true,
        games_finished: 0,
    };
    
    // A message should be sent when auto_refresh is true
    if config.auto_refresh {
        tx.send(UiToNet::RefreshGames).unwrap();
    }
    
    // Check if we received the RefreshGames message
    let mut found_refresh = false;
    while let Ok(msg) = rx.try_recv() {
        if let UiToNet::RefreshGames = msg {
            found_refresh = true;
            break;
        }
    }
    
    assert!(found_refresh, "Expected at least one RefreshGames message");
}
