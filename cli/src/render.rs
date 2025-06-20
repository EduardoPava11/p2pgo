// SPDX-License-Identifier: MIT OR Apache-2.0

//! ASCII board rendering for the CLI.

use p2pgo_core::{GameState, Color, Coord};

/// Render the game board as ASCII art
pub fn render_board(game_state: &GameState) -> String {
    let size = game_state.board_size;
    let mut output = String::new();
    
    // Add column labels
    output.push_str("   ");
    for col in 0..size {
        let col_char = coord_to_column_char(col);
        output.push_str(&format!(" {}", col_char));
    }
    output.push('\n');
    
    // Add rows with row numbers and board content
    for row in 0..size {
        // Row number (1-indexed)
        output.push_str(&format!("{:2} ", row + 1));
        
        for col in 0..size {
            let coord = Coord::new(col, row);
            let idx = (row as usize) * (size as usize) + (col as usize);
            
            let symbol = match game_state.board.get(idx).unwrap_or(&None) {
                Some(Color::Black) => "●",
                Some(Color::White) => "○",
                None => {
                    // Check if this is a star point
                    if is_star_point(coord, size) {
                        "+"
                    } else if (row == 0 || row == size - 1) && (col == 0 || col == size - 1) {
                        // Corner
                        "+"
                    } else if row == 0 || row == size - 1 || col == 0 || col == size - 1 {
                        // Edge
                        "+"
                    } else {
                        // Normal intersection
                        "+"
                    }
                }
            };
            
            output.push_str(&format!(" {}", symbol));
        }
        
        // Add row number again on the right
        output.push_str(&format!(" {}", row + 1));
        output.push('\n');
    }
    
    // Add column labels again at bottom
    output.push_str("   ");
    for col in 0..size {
        let col_char = coord_to_column_char(col);
        output.push_str(&format!(" {}", col_char));
    }
    output.push('\n');
    
    output
}

/// Convert a column index to a column character (A-T, skipping I)
fn coord_to_column_char(col: u8) -> char {
    if col < 8 {
        (b'A' + col) as char
    } else {
        (b'A' + col + 1) as char // Skip 'I'
    }
}

/// Check if a coordinate is a star point on the board
fn is_star_point(coord: Coord, board_size: u8) -> bool {
    let (x, y) = (coord.x, coord.y);
    
    match board_size {
        9 => {
            // 9x9 has star points at (2,2), (2,6), (4,4), (6,2), (6,6)
            matches!((x, y), (2, 2) | (2, 6) | (4, 4) | (6, 2) | (6, 6))
        }
        13 => {
            // 13x13 has star points at (3,3), (3,9), (6,6), (9,3), (9,9)
            matches!((x, y), (3, 3) | (3, 9) | (6, 6) | (9, 3) | (9, 9))
        }
        19 => {
            // 19x19 has star points at corners, sides, and center
            matches!(
                (x, y),
                (3, 3) | (3, 9) | (3, 15) |
                (9, 3) | (9, 9) | (9, 15) |
                (15, 3) | (15, 9) | (15, 15)
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2pgo_core::{Move};
    
    #[test]
    fn test_render_empty_9x9_board() {
        let game_state = GameState::new(9);
        let output = render_board(&game_state);
        
        // Should contain column labels A-J (skipping I)
        assert!(output.contains("A B C D E F G H J"));
        
        // Should contain row numbers 1-9
        assert!(output.contains(" 1 "));
        assert!(output.contains(" 9 "));
        
        // Should have proper dimensions
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 11); // 2 label rows + 9 board rows
    }
    
    #[test]
    fn test_render_board_with_stones() {
        let mut game_state = GameState::new(9);
        
        // Place some stones
        game_state.apply_move(Move::Place(Coord::new(4, 4))).unwrap(); // Black at center
        game_state.apply_move(Move::Place(Coord::new(3, 3))).unwrap(); // White at corner
        
        let output = render_board(&game_state);
        
        // Should contain black and white stones
        assert!(output.contains("●"));
        assert!(output.contains("○"));
    }
    
    #[test]
    fn test_coord_to_column_char() {
        assert_eq!(coord_to_column_char(0), 'A');
        assert_eq!(coord_to_column_char(7), 'H');
        assert_eq!(coord_to_column_char(8), 'J'); // Skip 'I'
        assert_eq!(coord_to_column_char(17), 'S'); // Column 17 maps to 'S'
        assert_eq!(coord_to_column_char(18), 'T'); // Column 18 maps to 'T'
    }
    
    #[test]
    fn test_star_points() {
        // Test 9x9 star points
        assert!(is_star_point(Coord::new(4, 4), 9)); // Center
        assert!(is_star_point(Coord::new(2, 2), 9)); // Corner
        assert!(!is_star_point(Coord::new(0, 0), 9)); // Not a star point
        
        // Test 19x19 star points
        assert!(is_star_point(Coord::new(9, 9), 19)); // Center
        assert!(is_star_point(Coord::new(3, 3), 19)); // Corner
        assert!(!is_star_point(Coord::new(0, 0), 19)); // Not a star point
    }
}
