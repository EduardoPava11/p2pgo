// SPDX-License-Identifier: MIT OR Apache-2.0

//! Game rules and validation logic

use crate::{board::Board, Color, Coord, GameError};
use std::collections::HashSet;

/// Validates game rules for Go
pub struct RuleValidator<'a> {
    /// The board being checked
    board: &'a Board,
    /// Last board state for ko rule checking
    previous_board: &'a Board,
}

impl<'a> RuleValidator<'a> {
    /// Create a new rules validator
    pub fn new(board: &'a Board, previous_board: &'a Board) -> Self {
        Self {
            board,
            previous_board,
        }
    }

    /// Check if a move is valid
    pub fn check_move(&self, coord: Coord, color: Color) -> Result<(), GameError> {
        // Basic validation
        if !coord.is_valid(self.board.size()) {
            return Err(GameError::InvalidCoordinate);
        }

        if self.board.get(coord).is_some() {
            return Err(GameError::OccupiedPosition);
        }

        // Create a temporary board with the move applied
        let mut temp_board = self.board.clone();
        temp_board.place(coord, color);

        // Check for suicide
        let group_coords = find_group(&temp_board, coord);
        if Self::liberties(&temp_board, &group_coords) == 0 {
            // If all neighboring groups of opponent stones have liberties, this is suicide
            let opponent_color = color.opposite();
            let mut will_capture = false;

            for neighbor in self.board.adjacent_coords(coord) {
                if let Some(stone_color) = self.board.get(neighbor) {
                    if stone_color == opponent_color {
                        let neighbor_group = find_group(&temp_board, neighbor);
                        if Self::liberties(&temp_board, &neighbor_group) == 0 {
                            will_capture = true;
                            break;
                        }
                    }
                }
            }

            if !will_capture {
                return Err(GameError::SelfCapture);
            }
        }

        // Check for ko - compare with previous board
        if self.previous_board.size() == temp_board.size() {
            let mut captured_coords = Vec::new();

            // Find stones that would be captured
            for neighbor in temp_board.adjacent_coords(coord) {
                if let Some(stone_color) = temp_board.get(neighbor) {
                    if stone_color == color.opposite() {
                        let neighbor_group = find_group(&temp_board, neighbor);
                        if Self::liberties(&temp_board, &neighbor_group) == 0 {
                            captured_coords.extend(neighbor_group);
                        }
                    }
                }
            }

            // Ko happens when: capturing exactly one stone AND the resulting board equals previous board
            if captured_coords.len() == 1 {
                // Create a board after captures
                let mut after_capture = temp_board.clone();
                for c in &captured_coords {
                    after_capture.remove(*c);
                }

                // Compare with previous board (ko detection)
                let mut same_as_previous = true;

                for y in 0..self.board.size() {
                    for x in 0..self.board.size() {
                        let c = Coord::new(x, y);
                        if after_capture.get(c) != self.previous_board.get(c) {
                            same_as_previous = false;
                            break;
                        }
                    }
                    if !same_as_previous {
                        break;
                    }
                }

                if same_as_previous {
                    tracing::debug!("Ko violation detected at {:?}", coord);
                    return Err(GameError::KoViolation);
                }
            }
        }

        Ok(())
    }

    /// Calculate the number of liberties for a group of stones
    pub fn liberties(board: &Board, group: &[Coord]) -> usize {
        let mut liberties_set = HashSet::new();

        for &coord in group {
            for neighbor in board.adjacent_coords(coord) {
                if board.get(neighbor).is_none() {
                    liberties_set.insert(neighbor);
                }
            }
        }

        liberties_set.len()
    }

    /// Find stones that would be captured after a move
    pub fn find_captures(&self, last_move: Coord) -> Vec<Coord> {
        let mut captures = Vec::new();
        let color = self.board.get(last_move).unwrap();
        let opponent = color.opposite();

        // Check all adjacent groups for captures
        for neighbor in self.board.adjacent_coords(last_move) {
            if let Some(stone_color) = self.board.get(neighbor) {
                if stone_color == opponent {
                    let group = find_group(self.board, neighbor);
                    if Self::liberties(self.board, &group) == 0 {
                        captures.extend(group);
                    }
                }
            }
        }

        captures
    }
}

/// Find all stones in a group connected to the stone at coord
fn find_group(board: &Board, coord: Coord) -> Vec<Coord> {
    let target_color = match board.get(coord) {
        Some(color) => color,
        None => return Vec::new(),
    };

    let mut group = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = vec![coord];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }

        visited.insert(current);
        group.push(current);

        // Add adjacent stones of the same color
        for neighbor in board.adjacent_coords(current) {
            if let Some(color) = board.get(neighbor) {
                if color == target_color && !visited.contains(&neighbor) {
                    queue.push(neighbor);
                }
            }
        }
    }

    group
}
