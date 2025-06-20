// SPDX-License-Identifier: MIT OR Apache-2.0

#![deny(warnings)]
#![deny(clippy::all)]

//! P2P Go UI library

pub mod app;
pub mod view;
pub mod msg;
pub mod board_widget;
pub mod worker;

// Headless function for testing
#[cfg(feature = "headless")]
pub fn headless() -> anyhow::Result<()> {
    use crossbeam_channel::unbounded;
    use crate::msg::UiToNet;
    use p2pgo_core::{Move, Coord};
    
    let (ui_tx, net_rx) = unbounded();
    let (net_tx, ui_rx) = unbounded();
    
    // Spawn background worker with a shorter timeout
    std::thread::spawn(move || {
        let _ = worker::start(net_rx, net_tx);
    });
    
    let mut app = app::App::new_headless_with_channels(ui_tx.clone(), ui_rx);
    
    // Create a game
    let _ = ui_tx.send(UiToNet::CreateGame { board_size: 9 });
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    // Process initial messages
    for _ in 0..10 {
        app.tick_headless();
    }
    
    // Simulate some game moves with reduced timing
    let moves = vec![
        Move::Place(Coord::new(3, 3)), // D4 - Black
        Move::Place(Coord::new(5, 3)), // F4 - White  
        Move::Place(Coord::new(4, 4)), // E5 - Black
        Move::Pass,                    // White pass
        Move::Pass,                    // Black pass
    ];
    
    for mv in moves.iter() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = ui_tx.send(UiToNet::MakeMove { mv: mv.clone(), board_size: None });
        
        // Process messages after each move
        for _ in 0..5 {
            app.tick_headless();
        }
    }
    
    // Final processing
    for _ in 0..10 {
        app.tick_headless();
    }
    
    println!("Headless simulation completed");
    Ok(())
}

#[cfg(test)]
mod test_util;

#[cfg(test)]
pub use test_util::*;
