// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(warnings)]
#![deny(clippy::all)]
#![allow(dead_code)] // Allow for development

//! P2P Go UI library

pub mod app;
pub mod view;
pub mod msg;
pub mod board_widget;
pub mod worker;
pub mod network_panel;
pub mod clipboard_helper;
pub mod toast_manager;
pub mod ui_config;
pub mod offline_game;
pub mod go3d;
pub mod go3d_wireframe;
pub mod design_system;
// pub mod sgf_upload;
// pub mod heat_map;
pub mod lobby;
pub mod bootstrap_status;
// pub mod network_visualization;
// pub mod neural_visualization;
pub mod enhanced_lobby;
// pub mod neural_animation;
// pub mod training_visualization;
// pub mod neural_training_ui;
// pub mod neural_overlay;
pub mod error_logger;
// pub mod neural_config_ui;
// pub mod sgf_training_ui;
// pub mod neural_game_ui;
// pub mod heat_map_integration;
pub mod dark_theme;
pub mod neural_placeholder;
pub mod connection_status;
pub mod labeled_input;
pub mod stone_animation;
pub mod sound_manager;
// pub mod update_checker;
// pub mod update_ui;
pub mod components;
pub mod game_activity_logger;
pub mod training;
pub mod dual_heat_map;
pub mod heat_map;

// No re-exports for now

// Headless function for testing
#[cfg(feature = "headless")]
pub fn headless() -> anyhow::Result<()> {
    use crossbeam_channel::unbounded;
    use crate::msg::UiToNet;
    use p2pgo_core::{Move, Coord, Color};
    
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
        Move::Place { x: 3, y: 3, color: Color::Black }, // D4 - Black
        Move::Place { x: 5, y: 3, color: Color::White }, // F4 - White  
        Move::Place { x: 4, y: 4, color: Color::Black }, // E5 - Black
        Move::Pass,                                      // White pass
        Move::Pass,                                      // Black pass
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
