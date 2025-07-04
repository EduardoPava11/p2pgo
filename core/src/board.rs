// SPDX-License-Identifier: MIT OR Apache-2.0

//! Board representation and manipulation

/// Represents the Go board with stones and empty positions
#[derive(Clone)]
pub struct Board {
    /// Size of the board (typically 9, 13, or 19)
    size: u8,
    /// Positions on the board
    positions: Vec<Option<crate::Color>>,
}

impl Board {
    /// Create a new empty board with the specified size
    pub fn new(size: u8) -> Self {
        let cells = (size as usize) * (size as usize);
        Self {
            size,
            positions: vec![None; cells],
        }
    }
    
    /// Get the stone at the specified coordinate
    pub fn get(&self, coord: crate::Coord) -> Option<crate::Color> {
        if !coord.is_valid(self.size) {
            return None;
        }
        
        let idx = self.coord_to_index(coord);
        self.positions[idx]
    }
    
    /// Place a stone at the specified coordinate
    pub fn place(&mut self, coord: crate::Coord, color: crate::Color) -> bool {
        if !coord.is_valid(self.size) {
            return false;
        }
        
        let idx = self.coord_to_index(coord);
        if self.positions[idx].is_some() {
            return false;
        }
        
        self.positions[idx] = Some(color);
        true
    }
    
    /// Convert a coordinate to a vector index
    fn coord_to_index(&self, coord: crate::Coord) -> usize {
        (coord.y as usize) * (self.size as usize) + (coord.x as usize)
    }
    
    /// Get adjacent coordinates (up, down, left, right)
    pub fn adjacent_coords(&self, coord: crate::Coord) -> Vec<crate::Coord> {
        let mut result = Vec::with_capacity(4);
        let x = coord.x;
        let y = coord.y;
        
        // Up
        if y > 0 {
            result.push(crate::Coord::new(x, y - 1));
        }
        
        // Down
        if y < self.size - 1 {
            result.push(crate::Coord::new(x, y + 1));
        }
        
        // Left
        if x > 0 {
            result.push(crate::Coord::new(x - 1, y));
        }
        
        // Right
        if x < self.size - 1 {
            result.push(crate::Coord::new(x + 1, y));
        }
        
        result
    }
    
    /// Get the size of the board
    pub fn size(&self) -> u8 {
        self.size
    }
    
    /// Remove a stone at the specified coordinate
    pub fn remove(&mut self, coord: crate::Coord) -> bool {
        if !coord.is_valid(self.size) {
            return false;
        }
        
        let idx = self.coord_to_index(coord);
        if self.positions[idx].is_none() {
            return false;
        }
        
        self.positions[idx] = None;
        true
    }
    
    /// Calculate a hash of the current board position
    pub fn position_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        self.positions.hash(&mut hasher);
        hasher.finish()
    }
}


