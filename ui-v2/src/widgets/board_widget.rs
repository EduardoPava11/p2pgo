//! Go board rendering widget

use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use p2pgo_core::{Coord, Color, GameState};
use crate::core::{Colors, Typography};

pub struct BoardWidget<'a> {
    game_state: &'a GameState,
    show_coordinates: bool,
    show_last_move: bool,
    highlight_positions: Vec<(Coord, Color32)>,
    heat_map: Option<&'a [[f32; 9]; 9]>,
    interactive: bool,
}

impl<'a> BoardWidget<'a> {
    pub fn new(game_state: &'a GameState) -> Self {
        Self {
            game_state,
            show_coordinates: true,
            show_last_move: true,
            highlight_positions: Vec::new(),
            heat_map: None,
            interactive: true,
        }
    }
    
    pub fn show_coordinates(mut self, show: bool) -> Self {
        self.show_coordinates = show;
        self
    }
    
    pub fn show_last_move(mut self, show: bool) -> Self {
        self.show_last_move = show;
        self
    }
    
    pub fn highlight(mut self, positions: Vec<(Coord, Color32)>) -> Self {
        self.highlight_positions = positions;
        self
    }
    
    pub fn heat_map(mut self, heat_map: &'a [[f32; 9]; 9]) -> Self {
        self.heat_map = Some(heat_map);
        self
    }
    
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
    
    pub fn show(self, ui: &mut Ui) -> BoardResponse {
        let board_size = self.game_state.board_size as f32;
        let margin = if self.show_coordinates { 30.0 } else { 20.0 };
        let available_size = ui.available_size();
        let max_size = available_size.min_elem() - margin * 2.0;
        let cell_size = max_size / board_size;
        let board_pixel_size = cell_size * board_size;
        
        let (response, painter) = ui.allocate_painter(
            Vec2::splat(board_pixel_size + margin * 2.0),
            if self.interactive { Sense::click() } else { Sense::hover() },
        );
        
        let board_rect = Rect::from_center_size(
            response.rect.center(),
            Vec2::splat(board_pixel_size),
        );
        
        // Draw board background
        painter.rect_filled(
            board_rect.expand(margin / 2.0),
            4.0,
            Colors::BOARD,
        );
        
        // Draw grid lines
        self.draw_grid(&painter, board_rect, board_size as usize);
        
        // Draw coordinates if enabled
        if self.show_coordinates {
            self.draw_coordinates(&painter, board_rect, board_size as usize, margin);
        }
        
        // Draw heat map if provided
        if let Some(heat_map) = self.heat_map {
            self.draw_heat_map(&painter, board_rect, cell_size, heat_map);
        }
        
        // Draw stones
        self.draw_stones(&painter, board_rect, cell_size);
        
        // Draw highlights
        for (pos, color) in &self.highlight_positions {
            let center = self.board_pos_to_screen(*pos, board_rect, cell_size);
            painter.circle_stroke(center, cell_size * 0.5, Stroke::new(3.0, *color));
        }
        
        // Draw last move marker
        if self.show_last_move {
            if let Some(last_move) = self.game_state.moves.last() {
                if let p2pgo_core::Move::Place { x, y, color } = last_move {
                    let pos = Coord::new(*x, *y);
                    let center = self.board_pos_to_screen(pos, board_rect, cell_size);
                    let marker_color = match color {
                        Color::Black => Colors::WHITE_STONE,
                        Color::White => Colors::BLACK_STONE,
                    };
                    painter.circle_filled(center, cell_size * 0.15, marker_color);
                }
            }
        }
        
        // Handle click
        let clicked_position = if response.clicked() && self.interactive {
            self.screen_to_board_pos(response.interact_pointer_pos().unwrap_or_default(), board_rect, cell_size)
        } else {
            None
        };
        
        BoardResponse {
            response,
            clicked_position,
        }
    }
    
    fn draw_grid(&self, painter: &Painter, rect: Rect, size: usize) {
        let cell_size = rect.width() / size as f32;
        let half_cell = cell_size / 2.0;
        
        // Grid lines
        for i in 0..size {
            let offset = i as f32 * cell_size + half_cell;
            
            // Vertical lines
            painter.line_segment(
                [
                    Pos2::new(rect.left() + offset, rect.top() + half_cell),
                    Pos2::new(rect.left() + offset, rect.bottom() - half_cell),
                ],
                Stroke::new(1.0, Color32::from_gray(80)),
            );
            
            // Horizontal lines
            painter.line_segment(
                [
                    Pos2::new(rect.left() + half_cell, rect.top() + offset),
                    Pos2::new(rect.right() - half_cell, rect.top() + offset),
                ],
                Stroke::new(1.0, Color32::from_gray(80)),
            );
        }
        
        // Star points for 9x9 board
        if size == 9 {
            let star_points = [(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)];
            for (x, y) in star_points {
                let pos = Pos2::new(
                    rect.left() + (x as f32 + 0.5) * cell_size,
                    rect.top() + (y as f32 + 0.5) * cell_size,
                );
                painter.circle_filled(pos, 3.0, Color32::from_gray(80));
            }
        }
    }
    
    fn draw_coordinates(&self, painter: &Painter, rect: Rect, size: usize, margin: f32) {
        let cell_size = rect.width() / size as f32;
        let font = Typography::small();
        
        for i in 0..size {
            // Letters (A-J, skipping I)
            let letter = if i < 8 { (b'A' + i as u8) as char } else { 'J' };
            painter.text(
                Pos2::new(
                    rect.left() + (i as f32 + 0.5) * cell_size,
                    rect.bottom() + margin * 0.5,
                ),
                egui::Align2::CENTER_CENTER,
                letter,
                font.clone(),
                Colors::TEXT_SECONDARY,
            );
            
            // Numbers (1-9, bottom to top)
            let number = (size - i).to_string();
            painter.text(
                Pos2::new(
                    rect.left() - margin * 0.5,
                    rect.top() + (i as f32 + 0.5) * cell_size,
                ),
                egui::Align2::CENTER_CENTER,
                &number,
                font.clone(),
                Colors::TEXT_SECONDARY,
            );
        }
    }
    
    fn draw_heat_map(&self, painter: &Painter, rect: Rect, cell_size: f32, heat_map: &[[f32; 9]; 9]) {
        for x in 0..9 {
            for y in 0..9 {
                let value = heat_map[y][x];
                if value > 0.01 {
                    let pos = self.board_pos_to_screen(Coord::new(x as u8, y as u8), rect, cell_size);
                    let alpha = (value * 150.0) as u8;
                    let color = Color32::from_rgba_unmultiplied(255, 100, 100, alpha);
                    painter.circle_filled(pos, cell_size * 0.3, color);
                }
            }
        }
    }
    
    fn draw_stones(&self, painter: &Painter, rect: Rect, cell_size: f32) {
        for y in 0..self.game_state.board_size {
            for x in 0..self.game_state.board_size {
                let idx = (y as usize) * (self.game_state.board_size as usize) + (x as usize);
                if let Some(color) = self.game_state.board[idx] {
                    let pos = Coord::new(x, y);
                    let center = self.board_pos_to_screen(pos, rect, cell_size);
                    self.draw_stone(painter, center, cell_size * 0.45, color);
                }
            }
        }
    }
    
    fn draw_stone(&self, painter: &Painter, center: Pos2, radius: f32, color: Color) {
        let (base_color, highlight_color, shadow_color) = match color {
            Color::Black => (
                Colors::BLACK_STONE,
                Color32::from_gray(40),
                Color32::from_black_alpha(180),
            ),
            Color::White => (
                Colors::WHITE_STONE,
                Color32::WHITE,
                Color32::from_black_alpha(80),
            ),
        };
        
        // Shadow
        painter.circle_filled(
            center + Vec2::new(1.0, 1.0),
            radius,
            shadow_color,
        );
        
        // Stone
        painter.circle_filled(center, radius, base_color);
        
        // Highlight for 3D effect
        painter.circle_filled(
            center - Vec2::new(radius * 0.3, radius * 0.3),
            radius * 0.2,
            highlight_color.linear_multiply(0.3),
        );
    }
    
    fn board_pos_to_screen(&self, pos: Coord, rect: Rect, cell_size: f32) -> Pos2 {
        Pos2::new(
            rect.left() + (pos.x as f32 + 0.5) * cell_size,
            rect.top() + (pos.y as f32 + 0.5) * cell_size,
        )
    }
    
    fn screen_to_board_pos(&self, screen_pos: Pos2, rect: Rect, cell_size: f32) -> Option<Coord> {
        if !rect.contains(screen_pos) {
            return None;
        }
        
        let x = ((screen_pos.x - rect.left()) / cell_size) as u8;
        let y = ((screen_pos.y - rect.top()) / cell_size) as u8;
        
        if x < self.game_state.board_size && y < self.game_state.board_size {
            Some(Coord::new(x, y))
        } else {
            None
        }
    }
}

pub struct BoardResponse {
    pub response: Response,
    pub clicked_position: Option<Coord>,
}