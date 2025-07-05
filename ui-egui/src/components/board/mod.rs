//! Board rendering components

mod interaction;
mod renderer;

pub use interaction::BoardInteraction;
pub use renderer::BoardRenderer;

use egui::{Pos2, Rect, Vec2};
use p2pgo_core::Coord;

/// Convert board coordinate to screen position
pub fn coord_to_pos(coord: Coord, board_rect: Rect, cell_size: f32) -> Pos2 {
    let x = board_rect.min.x + (coord.x as f32) * cell_size;
    let y = board_rect.min.y + (coord.y as f32) * cell_size;
    Pos2::new(x, y)
}

/// Convert screen position to board coordinate
pub fn pos_to_coord(pos: Pos2, board_rect: Rect, cell_size: f32, board_size: u8) -> Option<Coord> {
    if !board_rect.contains(pos) {
        return None;
    }

    let rel_pos = pos - board_rect.min;
    let x = (rel_pos.x / cell_size).round() as u8;
    let y = (rel_pos.y / cell_size).round() as u8;

    if x < board_size && y < board_size {
        Some(Coord { x, y })
    } else {
        None
    }
}
