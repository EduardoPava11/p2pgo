//! Board interaction handling

use egui::{Ui, Response, Rect, Vec2, Sense};
use p2pgo_core::{GameState, Coord, Color};
use crossbeam_channel::Sender;
use crate::msg::UiToNet;

/// Board interaction handler
pub struct BoardInteraction {
    /// Board size
    board_size: u8,
    /// Current hover position
    hover_pos: Option<Coord>,
    /// Cell size for interaction
    cell_size: f32,
}

impl BoardInteraction {
    pub fn new(board_size: u8) -> Self {
        Self {
            board_size,
            hover_pos: None,
            cell_size: 30.0,
        }
    }
    
    /// Set cell size from renderer
    pub fn set_cell_size(&mut self, cell_size: f32) {
        self.cell_size = cell_size;
    }
    
    /// Handle board interaction
    pub fn handle_interaction(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        game_state: &GameState,
        ui_tx: Option<&Sender<UiToNet>>,
    ) -> Option<Coord> {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        
        // Handle hover
        if let Some(hover_pos) = response.hover_pos() {
            self.hover_pos = self.pos_to_coord(hover_pos, rect);
        } else {
            self.hover_pos = None;
        }
        
        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(coord) = self.pos_to_coord(pos, rect) {
                    if self.is_valid_move(coord, game_state) {
                        // Send debug event for testing
                        if let Some(tx) = ui_tx {
                            let _ = tx.send(UiToNet::DebugMovePlaced(coord));
                        }
                        return Some(coord);
                    }
                }
            }
        }
        
        None
    }
    
    /// Convert screen position to board coordinate
    fn pos_to_coord(&self, pos: egui::Pos2, rect: Rect) -> Option<Coord> {
        let margin = 20.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(rect.width() - 2.0 * margin),
        );
        
        super::pos_to_coord(pos, board_rect, self.cell_size, self.board_size)
    }
    
    /// Check if a move is valid (basic client-side validation)
    pub fn is_valid_move(&self, coord: Coord, game_state: &GameState) -> bool {
        // Check bounds
        if coord.x >= self.board_size || coord.y >= self.board_size {
            return false;
        }
        
        // Check if position is empty
        let idx = (coord.y as usize) * (game_state.board_size as usize) + (coord.x as usize);
        if game_state.board.get(idx).and_then(|c| *c).is_some() {
            return false;
        }
        
        // TODO: Add ko rule validation
        // TODO: Add suicide rule validation
        
        true
    }
    
    /// Get current hover position
    pub fn get_hover_pos(&self) -> Option<Coord> {
        self.hover_pos
    }
}