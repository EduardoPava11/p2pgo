//! Board rendering logic

use egui::{Ui, Vec2, Rect, Color32, Stroke, Pos2};
use p2pgo_core::{GameState, Coord};
use crate::design_system::get_design_system;
use crate::stone_animation::AnimationManager;

/// Board renderer component
pub struct BoardRenderer {
    /// Board size
    board_size: u8,
    /// Cell size in pixels (dynamically calculated)
    cell_size: f32,
    /// Animation manager
    animation_manager: AnimationManager,
    /// Last move position for highlighting
    last_move: Option<Coord>,
}

impl BoardRenderer {
    pub fn new(board_size: u8) -> Self {
        Self {
            board_size,
            cell_size: 30.0, // Default, will be recalculated
            animation_manager: AnimationManager::new(),
            last_move: None,
        }
    }
    
    /// Calculate optimal cell size based on available space
    pub fn calculate_cell_size(&mut self, available_size: Vec2) -> f32 {
        let margin = 60.0; // Total margin for board edges
        let max_board_size = available_size.min_elem() * 0.85; // Use 85% of available space
        
        // Calculate cell size dynamically
        self.cell_size = (max_board_size - margin) / (self.board_size as f32 - 1.0);
        
        // Ensure minimum and maximum cell sizes for usability
        self.cell_size = self.cell_size.clamp(25.0, 60.0);
        
        self.cell_size
    }
    
    /// Get the desired board size
    pub fn get_desired_size(&self) -> Vec2 {
        let margin = 60.0;
        let board_pixel_size = self.cell_size * (self.board_size as f32 - 1.0);
        Vec2::splat(board_pixel_size + margin)
    }
    
    /// Render the board
    pub fn render(&mut self, ui: &mut Ui, rect: Rect, game_state: &GameState) {
        let painter = ui.painter_at(rect);
        let ds = get_design_system();
        
        // Board background
        painter.rect_filled(rect, 0.0, ds.colors.board_bg);
        
        let margin = 20.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(rect.width() - 2.0 * margin),
        );
        
        // Render grid
        self.render_grid(&painter, board_rect);
        
        // Render star points
        self.render_star_points(&painter, board_rect);
        
        // Render stones
        self.render_stones(&painter, board_rect, game_state);
        
        // Render animations
        self.render_animations(&painter, board_rect);
        
        // Render last move indicator
        if let Some(last_coord) = self.last_move {
            self.render_last_move(&painter, board_rect, last_coord);
        }
    }
    
    /// Render grid lines
    fn render_grid(&self, painter: &egui::Painter, board_rect: Rect) {
        let ds = get_design_system();
        let line_color = ds.colors.grid_line;
        let line_stroke = Stroke::new(1.0, line_color);
        
        for i in 0..self.board_size {
            let offset = (i as f32) * self.cell_size;
            
            // Vertical lines
            let start = Pos2::new(board_rect.min.x + offset, board_rect.min.y);
            let end = Pos2::new(board_rect.min.x + offset, board_rect.max.y);
            painter.line_segment([start, end], line_stroke);
            
            // Horizontal lines
            let start = Pos2::new(board_rect.min.x, board_rect.min.y + offset);
            let end = Pos2::new(board_rect.max.x, board_rect.min.y + offset);
            painter.line_segment([start, end], line_stroke);
        }
    }
    
    /// Render star points (hoshi)
    fn render_star_points(&self, painter: &egui::Painter, board_rect: Rect) {
        let ds = get_design_system();
        let star_points = match self.board_size {
            19 => vec![
                (3, 3), (3, 9), (3, 15),
                (9, 3), (9, 9), (9, 15),
                (15, 3), (15, 9), (15, 15),
            ],
            13 => vec![(3, 3), (3, 9), (6, 6), (9, 3), (9, 9)],
            9 => vec![(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)],
            _ => vec![],
        };
        
        for (x, y) in star_points {
            let pos = super::coord_to_pos(Coord { x, y }, board_rect, self.cell_size);
            painter.circle_filled(pos, 3.0, ds.colors.grid_line);
        }
    }
    
    /// Render stones on the board
    fn render_stones(&self, painter: &egui::Painter, board_rect: Rect, game_state: &GameState) {
        let ds = get_design_system();
        let stone_radius = self.cell_size * 0.4;
        
        for y in 0..self.board_size {
            for x in 0..self.board_size {
                let coord = Coord { x, y };
                let idx = (y as usize) * (game_state.board_size as usize) + (x as usize);
                
                if let Some(color) = game_state.board.get(idx).and_then(|c| *c) {
                    let pos = super::coord_to_pos(coord, board_rect, self.cell_size);
                    let stone_color = match color {
                        p2pgo_core::Color::Black => ds.colors.black_stone,
                        p2pgo_core::Color::White => ds.colors.white_stone,
                    };
                    painter.circle_filled(pos, stone_radius, stone_color);
                    painter.circle_stroke(pos, stone_radius, Stroke::new(1.0, ds.colors.grid_line));
                }
            }
        }
    }
    
    /// Render animations
    fn render_animations(&self, painter: &egui::Painter, board_rect: Rect) {
        let ds = get_design_system();
        let stone_radius = self.cell_size * 0.4;
        
        for animation in self.animation_manager.get_animations() {
            let base_pos = super::coord_to_pos(animation.coord, board_rect, self.cell_size);
            let transform = animation.get_transform(base_pos, stone_radius);
            
            // Draw ripple effect for placement animations
            if let Some(ripple) = animation.get_ripple() {
                let ripple_color = match animation.color {
                    p2pgo_core::Color::Black => Color32::from_rgba_unmultiplied(0, 0, 0, (ripple.opacity * 255.0) as u8),
                    p2pgo_core::Color::White => Color32::from_rgba_unmultiplied(255, 255, 255, (ripple.opacity * 255.0) as u8),
                };
                painter.circle_stroke(
                    base_pos,
                    stone_radius * ripple.radius_factor,
                    Stroke::new(2.0, ripple_color),
                );
            }
            
            // Draw the animated stone
            let stone_color = match animation.color {
                p2pgo_core::Color::Black => {
                    let base = ds.colors.black_stone;
                    Color32::from_rgba_unmultiplied(
                        base.r(),
                        base.g(),
                        base.b(),
                        (base.a() as f32 * transform.opacity) as u8,
                    )
                }
                p2pgo_core::Color::White => {
                    let base = ds.colors.white_stone;
                    Color32::from_rgba_unmultiplied(
                        base.r(),
                        base.g(),
                        base.b(),
                        (base.a() as f32 * transform.opacity) as u8,
                    )
                }
            };
            
            let animated_radius = stone_radius * transform.scale;
            painter.circle_filled(transform.position, animated_radius, stone_color);
            
            if transform.opacity > 0.1 {
                painter.circle_stroke(
                    transform.position,
                    animated_radius,
                    Stroke::new(1.0, ds.colors.grid_line.linear_multiply(transform.opacity)),
                );
            }
        }
    }
    
    /// Render last move indicator
    fn render_last_move(&self, painter: &egui::Painter, board_rect: Rect, coord: Coord) {
        let pos = super::coord_to_pos(coord, board_rect, self.cell_size);
        let stone_radius = self.cell_size * 0.4;
        let indicator_color = Color32::from_rgb(220, 38, 38); // Red
        painter.circle_stroke(pos, stone_radius * 0.6, Stroke::new(2.0, indicator_color));
    }
    
    /// Update animations
    pub fn update_animations(&mut self) -> bool {
        self.animation_manager.update()
    }
    
    /// Get animation manager
    pub fn animation_manager(&mut self) -> &mut AnimationManager {
        &mut self.animation_manager
    }
    
    /// Set last move
    pub fn set_last_move(&mut self, coord: Option<Coord>) {
        self.last_move = coord;
    }
}