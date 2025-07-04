//! Game management abstraction

use p2pgo_core::{GameState, Move};
use std::collections::HashMap;
use anyhow::Result;

pub struct GameManager {
    games: HashMap<String, GameState>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
        }
    }
    
    pub fn create_game(&mut self, game_id: &str, board_size: u8) {
        self.games.insert(game_id.to_string(), GameState::new(board_size));
    }
    
    pub fn join_game(&mut self, game_id: &str) {
        // In a real implementation, this would connect to the network
        // For now, just ensure the game exists
        if !self.games.contains_key(game_id) {
            self.create_game(game_id, 9);
        }
    }
    
    pub fn get_game_state(&self, game_id: &str) -> Option<&GameState> {
        self.games.get(game_id)
    }
    
    pub fn make_move(&mut self, game_id: &str, mv: Move) -> Result<()> {
        if let Some(game) = self.games.get_mut(game_id) {
            game.apply_move(mv)?;
        }
        Ok(())
    }
}