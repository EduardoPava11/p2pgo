// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game engine interfaces and AI backend

use std::time::Duration;
use crate::{GameState, Move};

/// Player backend trait for both human and AI players
pub trait PlayerBackend {
    /// Get the next move from this player
    fn next_move(&mut self, pos: &GameState, time_left: Duration) -> Move;
}

#[cfg(feature = "bot")]
pub mod bot {
    use super::*;
    use std::path::Path;
    
    /// AI bot that loads from a dynamic library
    pub struct DynamicBot {
        // Library handle would go here
    }
    
    impl DynamicBot {
        /// Create a new dynamic bot from the specified library path
        pub fn new(_path: &Path) -> Option<Self> {
            // TODO: implement dynamic library loading
            todo!("Implement dynamic library loading")
        }
    }
    
    impl PlayerBackend for DynamicBot {
        fn next_move(&mut self, pos: &GameState, time_left: Duration) -> Move {
            // TODO: implement AI move calculation via dynamic library
            todo!("Implement AI move calculation")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct RandomPlayer;
    
    impl PlayerBackend for RandomPlayer {
        fn next_move(&mut self, _pos: &GameState, _time_left: Duration) -> Move {
            Move::Pass // Always pass for now
        }
    }
    
    #[test]
    #[ignore = "Implementation needed"]
    fn test_player_backend() {
        // TODO: Implement test for player backend
        todo!("Implement player backend test");
    }
}
