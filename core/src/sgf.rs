// SPDX-License-Identifier: MIT OR Apache-2.0

//! SGF (Smart Game Format) parsing and generation

use crate::{Color, Coord, GameState, Move};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Represents an SGF property
#[derive(Debug, Clone)]
struct SgfProperty {
    /// Property identifier
    id: String,
    /// Property values
    values: Vec<String>,
}

/// Represents an SGF node
#[derive(Debug, Clone)]
struct SgfNode {
    /// Properties in the node
    properties: Vec<SgfProperty>,
}

/// Represents an SGF game tree
#[derive(Debug, Clone)]
struct SgfTree {
    /// Nodes in the tree (sequence)
    nodes: Vec<SgfNode>,
    /// Variations (branches)
    #[allow(dead_code)]
    variations: Vec<SgfTree>,
}

/// SGF parser and generator
pub struct SgfProcessor {
    /// The game state
    game_state: GameState,
}

impl SgfProcessor {
    /// Create a new SGF processor for the given game state
    pub fn new(game_state: GameState) -> Self {
        Self { game_state }
    }

    /// Parse an SGF string and return a game state
    pub fn parse(&mut self, sgf_text: &str) -> Result<GameState> {
        let tree = self.parse_sgf(sgf_text)?;
        self.convert_tree_to_game_state(tree)
    }

    /// Parse SGF text into an SGF tree
    fn parse_sgf(&self, sgf_text: &str) -> Result<SgfTree> {
        let mut chars = sgf_text.chars().peekable();
        self.parse_game_tree(&mut chars)
    }

    /// Parse a game tree
    fn parse_game_tree(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<SgfTree> {
        // Skip leading whitespace
        self.skip_whitespace(chars);

        // Expect '('
        if chars.next() != Some('(') {
            return Err(anyhow!("Expected '(' at start of game tree"));
        }

        // Parse sequence of nodes
        let mut nodes = Vec::new();
        let mut variations = Vec::new();

        // Skip whitespace
        self.skip_whitespace(chars);

        // Parse nodes until we hit a '(' or ')'
        while chars.peek() == Some(&';') {
            let node = self.parse_node(chars)?;
            nodes.push(node);
            self.skip_whitespace(chars);
        }

        // Parse variations
        while chars.peek() == Some(&'(') {
            let subtree = self.parse_game_tree(chars)?;
            variations.push(subtree);
            self.skip_whitespace(chars);
        }

        // Expect ')'
        if chars.next() != Some(')') {
            return Err(anyhow!("Expected ')' at end of game tree"));
        }

        Ok(SgfTree { nodes, variations })
    }

    /// Parse a node
    fn parse_node(&self, chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<SgfNode> {
        // Expect ';'
        if chars.next() != Some(';') {
            return Err(anyhow!("Expected ';' at start of node"));
        }

        let mut properties = Vec::new();

        // Skip whitespace
        self.skip_whitespace(chars);

        // Parse properties
        while let Some(&c) = chars.peek() {
            if c.is_ascii_uppercase() {
                let property = self.parse_property(chars)?;
                properties.push(property);
                self.skip_whitespace(chars);
            } else {
                break;
            }
        }

        Ok(SgfNode { properties })
    }

    /// Parse a property
    fn parse_property(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<SgfProperty> {
        // Parse property ID
        let mut id = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_uppercase() {
                id.push(chars.next().unwrap());
            } else {
                break;
            }
        }

        // Skip whitespace
        self.skip_whitespace(chars);

        // Parse property values
        let mut values = Vec::new();
        while chars.peek() == Some(&'[') {
            values.push(self.parse_property_value(chars)?);
            self.skip_whitespace(chars);
        }

        Ok(SgfProperty { id, values })
    }

    /// Parse a property value
    fn parse_property_value(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<String> {
        // Expect '['
        if chars.next() != Some('[') {
            return Err(anyhow!("Expected '[' at start of property value"));
        }

        let mut value = String::new();
        let mut escaped = false;

        while let Some(&c) = chars.peek() {
            if escaped {
                value.push(c);
                escaped = false;
                chars.next();
            } else if c == '\\' {
                escaped = true;
                chars.next();
            } else if c == ']' {
                chars.next();
                break;
            } else {
                value.push(c);
                chars.next();
            }
        }

        Ok(value)
    }

    /// Skip whitespace and SGF comments
    fn skip_whitespace(&self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else if c == 'c' && chars.clone().take(2).collect::<String>() == "c[" {
                // Skip SGF comment
                let mut comment_depth = 0;
                for c in chars.by_ref() {
                    if c == '[' {
                        comment_depth += 1;
                    } else if c == ']' {
                        comment_depth -= 1;
                        if comment_depth == 0 {
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Convert an SGF tree to a game state
    fn convert_tree_to_game_state(&mut self, tree: SgfTree) -> Result<GameState> {
        let mut props = HashMap::new();

        // Extract root properties
        if let Some(root) = tree.nodes.first() {
            for prop in &root.properties {
                props.insert(prop.id.clone(), prop.values.clone());
            }
        }

        // Get board size
        let size = if let Some(sz) = props.get("SZ") {
            sz[0].parse::<u8>().unwrap_or(19)
        } else {
            19 // Default board size
        };

        // Create a new game state
        let mut game_state = GameState::new(size);

        // Process the main line through the tree (including variations)
        self.process_tree_main_line(&mut game_state, &tree)?;

        Ok(game_state)
    }

    /// Process the main line through the tree (first variation)
    fn process_tree_main_line(&self, game_state: &mut GameState, tree: &SgfTree) -> Result<()> {
        // Process moves in current nodes
        for node in tree
            .nodes
            .iter()
            .skip(if game_state.moves.is_empty() { 1 } else { 0 })
        {
            for prop in &node.properties {
                match prop.id.as_str() {
                    "B" => {
                        // Black's move
                        if prop.values.is_empty() || prop.values[0].is_empty() {
                            // Empty value is a pass
                            game_state.apply_move(Move::Pass)?;
                        } else {
                            // Try to parse as a coordinate
                            match self.parse_sgf_coord(&prop.values[0], game_state.board_size) {
                                Ok(coord) => {
                                    game_state.apply_move(Move::Place {
                                        x: coord.x,
                                        y: coord.y,
                                        color: Color::Black,
                                    })?;
                                }
                                Err(_) => {
                                    game_state.apply_move(Move::Pass)?;
                                }
                            }
                        }
                    }
                    "W" => {
                        // White's move
                        if prop.values.is_empty() || prop.values[0].is_empty() {
                            // Empty value is a pass
                            game_state.apply_move(Move::Pass)?;
                        } else {
                            // Try to parse as a coordinate
                            match self.parse_sgf_coord(&prop.values[0], game_state.board_size) {
                                Ok(coord) => {
                                    game_state.apply_move(Move::Place {
                                        x: coord.x,
                                        y: coord.y,
                                        color: Color::White,
                                    })?;
                                }
                                Err(_) => {
                                    game_state.apply_move(Move::Pass)?;
                                }
                            }
                        }
                    }
                    _ => (), // Ignore other properties
                }
            }
        }

        // Follow the first variation (main line)
        if let Some(first_variation) = tree.variations.first() {
            self.process_tree_main_line(game_state, first_variation)?;
        }

        Ok(())
    }

    /// Parse an SGF coordinate like "ab" into a Coord
    fn parse_sgf_coord(&self, sgf_coord: &str, board_size: u8) -> Result<Coord> {
        // Empty coordinate is a pass move, but we should handle it elsewhere
        if sgf_coord.is_empty() {
            // Default to a pass move
            return Err(anyhow!("Empty coordinate is a pass move"));
        }

        if sgf_coord.len() < 2 {
            return Err(anyhow!("Invalid SGF coordinate: too short"));
        }

        let mut chars = sgf_coord.chars();
        let x = chars.next().unwrap() as u8 - b'a';
        let y = chars.next().unwrap() as u8 - b'a';

        if x >= board_size || y >= board_size {
            return Err(anyhow!("SGF coordinate out of board bounds"));
        }

        Ok(Coord::new(x, y))
    }

    /// Generate an SGF string from the current game state
    pub fn generate(&self) -> String {
        let mut sgf = String::new();

        // Start game tree
        sgf.push('(');

        // Root node with game info
        sgf.push(';');
        sgf.push_str(&format!("FF[4]GM[1]SZ[{}]", self.game_state.board_size));

        // Add player info if available
        sgf.push_str("AP[p2pgo]");

        // Add move sequences
        let mut current_color = Color::Black; // Go starts with Black

        for mv in &self.game_state.moves {
            sgf.push(';');

            match mv {
                Move::Place { x, y, color } => {
                    let sgf_x = (b'a' + x) as char;
                    let sgf_y = (b'a' + y) as char;

                    match color {
                        Color::Black => sgf.push_str(&format!("B[{}{}]", sgf_x, sgf_y)),
                        Color::White => sgf.push_str(&format!("W[{}{}]", sgf_x, sgf_y)),
                    }

                    // Update current color to match the move
                    current_color = *color;
                }
                Move::Pass => match current_color {
                    Color::Black => sgf.push_str("B[]"),
                    Color::White => sgf.push_str("W[]"),
                },
                Move::Resign => {
                    // Resignation is typically handled with a RE property
                    match current_color {
                        Color::Black => sgf.push_str("B[]C[Black resigns]RE[W+Resign]"),
                        Color::White => sgf.push_str("W[]C[White resigns]RE[B+Resign]"),
                    }
                }
            }

            // Switch color for next move
            current_color = current_color.opposite();
        }

        // Close game tree
        sgf.push(')');

        sgf
    }
}
