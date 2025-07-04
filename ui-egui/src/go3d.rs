//! 3D 9x9x9 Go Game Implementation
//!
//! A three-player variant of Go played on a 3D board with three intersecting planes.
//! Each plane is a 9x9 grid in the X, Y, or Z dimension.

use egui::{Context, Ui, Pos2, Rect, Vec2, Color32, Stroke, Painter};
use crate::ui_config::UiConfig;
use std::collections::HashMap;

/// Player color in 3D Go (Black, White, or Red)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color3D {
    Black,
    White,
    Red,
}

impl Color3D {
    /// Get the next player in turn order
    pub fn next(&self) -> Self {
        match self {
            Color3D::Black => Color3D::White,
            Color3D::White => Color3D::Red,
            Color3D::Red => Color3D::Black,
        }
    }
    
    /// Convert to egui Color32
    pub fn to_color32(&self) -> Color32 {
        match self {
            Color3D::Black => Color32::from_rgb(10, 10, 10),
            Color3D::White => Color32::from_rgb(250, 250, 250),
            Color3D::Red => Color32::from_rgb(200, 50, 50),
        }
    }
}

/// 3D coordinate in the intersecting planes
/// Only valid positions are on one of the three orthogonal planes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord3D {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl Coord3D {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }
    
    /// Check if this coordinate is on one of the three planes
    pub fn is_valid(&self) -> bool {
        // XY plane at Z=4 (middle)
        let on_xy = self.z == 4 && self.x < 9 && self.y < 9;
        // XZ plane at Y=4 (middle)
        let on_xz = self.y == 4 && self.x < 9 && self.z < 9;
        // YZ plane at X=4 (middle)
        let on_yz = self.x == 4 && self.y < 9 && self.z < 9;
        
        on_xy || on_xz || on_yz
    }
    
    /// Get adjacent positions (only on the planes)
    pub fn adjacent(&self) -> Vec<Coord3D> {
        let mut adj = Vec::new();
        
        // Check all 6 directions
        let candidates = [
            Coord3D::new(self.x.wrapping_sub(1), self.y, self.z),
            Coord3D::new(self.x + 1, self.y, self.z),
            Coord3D::new(self.x, self.y.wrapping_sub(1), self.z),
            Coord3D::new(self.x, self.y + 1, self.z),
            Coord3D::new(self.x, self.y, self.z.wrapping_sub(1)),
            Coord3D::new(self.x, self.y, self.z + 1),
        ];
        
        for coord in candidates {
            if coord.is_valid() {
                adj.push(coord);
            }
        }
        
        adj
    }
}

/// View plane for 2D projection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPlane {
    XY, // XY plane at Z=4
    XZ, // XZ plane at Y=4  
    YZ, // YZ plane at X=4
}

/// 3D Go game state
pub struct Go3DGame {
    /// 3D board state - None for empty, Some(color) for stone
    board: HashMap<Coord3D, Color3D>,
    /// Current player
    current_player: Color3D,
    /// UI configuration
    ui_config: UiConfig,
    /// Current view settings
    current_view: ViewPlane,
    /// Highlighted coordinates for better 3D navigation
    highlighted: Option<Coord3D>,
    /// Last move for highlighting
    last_move: Option<Coord3D>,
    /// Move count
    move_count: u32,
}

impl Go3DGame {
    /// Create a new 3D Go game
    pub fn new() -> Self {
        Self {
            board: HashMap::new(),
            current_player: Color3D::Black,
            ui_config: UiConfig::default(),
            current_view: ViewPlane::XY, // Start viewing XY plane
            highlighted: None,
            last_move: None,
            move_count: 0,
        }
    }
    
    /// Place a stone at the given coordinate
    pub fn place_stone(&mut self, coord: Coord3D) -> Result<(), &'static str> {
        if !coord.is_valid() {
            return Err("Position not on any plane");
        }
        
        if self.board.contains_key(&coord) {
            return Err("Position already occupied");
        }
        
        // Place the stone
        self.board.insert(coord, self.current_player);
        self.last_move = Some(coord);
        self.move_count += 1;
        
        // TODO: Check for captures on intersecting planes
        
        // Next player's turn
        self.current_player = self.current_player.next();
        
        Ok(())
    }
    
    /// Toggle stone at position (for testing)
    pub fn toggle_stone(&mut self, coord: Coord3D) {
        if let Some(color) = self.board.get(&coord) {
            // Cycle through colors
            let next_color = match color {
                Color3D::Black => Color3D::White,
                Color3D::White => Color3D::Red,
                Color3D::Red => {
                    self.board.remove(&coord);
                    return;
                }
            };
            self.board.insert(coord, next_color);
        } else {
            // Place new stone
            self.board.insert(coord, self.current_player);
            self.current_player = self.current_player.next();
        }
    }
    
    /// Render the 3D game UI
    pub fn ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(245, 245, 245))
                .inner_margin(egui::Margin::same(20.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Title and controls
                    ui.label(egui::RichText::new("3D Go - Three Intersecting Planes").size(24.0).color(Color32::from_gray(51)));
                    ui.label("243 positions on three orthogonal 9×9 planes");
                    ui.add_space(10.0);
                    
                    // Current player and move count
                    ui.horizontal(|ui| {
                        ui.label("Current Player:");
                        let (response, painter) = ui.allocate_painter(Vec2::splat(20.0), egui::Sense::hover());
                        painter.circle_filled(
                            response.rect.center(),
                            8.0,
                            self.current_player.to_color32()
                        );
                        ui.add_space(20.0);
                        ui.label(format!("Move: {}", self.move_count));
                    });
                    
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                    
                    // View controls
                    ui.horizontal(|ui| {
                        ui.label("View:");
                        
                        // Plane selection for the three intersecting planes
                        if ui.selectable_label(matches!(self.current_view, ViewPlane::XY), "XY Plane (Z=4)").clicked() {
                            self.current_view = ViewPlane::XY;
                        }
                        if ui.selectable_label(matches!(self.current_view, ViewPlane::XZ), "XZ Plane (Y=4)").clicked() {
                            self.current_view = ViewPlane::XZ;
                        }
                        if ui.selectable_label(matches!(self.current_view, ViewPlane::YZ), "YZ Plane (X=4)").clicked() {
                            self.current_view = ViewPlane::YZ;
                        }
                    });
                    
                    ui.add_space(20.0);
                    
                    // Main 3D board visualization
                    self.draw_3d_board(ui);
                    
                    ui.add_space(20.0);
                    
                    // Controls
                    ui.horizontal(|ui| {
                        if ui.button("Clear Board").clicked() {
                            self.board.clear();
                            self.current_player = Color3D::Black;
                            self.move_count = 0;
                            self.last_move = None;
                        }
                        
                        ui.add_space(10.0);
                        
                        if ui.button("Random Stones").clicked() {
                            // Add random stones on the three planes for testing
                            use rand::Rng;
                            let mut rng = rand::thread_rng();
                            
                            // Add stones on each plane
                            for _ in 0..10 {
                                // XY plane
                                let coord1 = Coord3D::new(rng.gen_range(0..9), rng.gen_range(0..9), 4);
                                if !self.board.contains_key(&coord1) {
                                    self.board.insert(coord1, Color3D::Black);
                                }
                                
                                // XZ plane  
                                let coord2 = Coord3D::new(rng.gen_range(0..9), 4, rng.gen_range(0..9));
                                if !self.board.contains_key(&coord2) {
                                    self.board.insert(coord2, Color3D::White);
                                }
                                
                                // YZ plane
                                let coord3 = Coord3D::new(4, rng.gen_range(0..9), rng.gen_range(0..9));
                                if !self.board.contains_key(&coord3) {
                                    self.board.insert(coord3, Color3D::Red);
                                }
                            }
                        }
                    });
                });
            });
    }
    
    /// Draw the 3D board visualization
    fn draw_3d_board(&mut self, ui: &mut Ui) {
        let available_size = ui.available_size();
        let board_size = available_size.x.min(available_size.y - 100.0).min(600.0);
        
        ui.horizontal(|ui| {
            // Main board view
            self.draw_main_view(ui, board_size);
            
            ui.add_space(20.0);
            
            // 3D overview
            self.draw_3d_overview(ui, board_size * 0.5);
        });
    }
    
    /// Draw the main 2D view of current plane
    fn draw_main_view(&mut self, ui: &mut Ui, size: f32) {
        let (response, painter) = ui.allocate_painter(Vec2::splat(size), egui::Sense::click());
        let rect = response.rect;
        
        // White background
        painter.rect_filled(rect, 0.0, Color32::WHITE);
        
        // Draw grid
        let cell_size = size / 9.0;
        let grid_stroke = Stroke::new(1.0, Color32::BLACK);
        
        for i in 0..10 {
            let offset = i as f32 * cell_size;
            // Vertical lines
            painter.line_segment(
                [
                    Pos2::new(rect.min.x + offset, rect.min.y),
                    Pos2::new(rect.min.x + offset, rect.max.y)
                ],
                grid_stroke
            );
            // Horizontal lines
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, rect.min.y + offset),
                    Pos2::new(rect.max.x, rect.min.y + offset)
                ],
                grid_stroke
            );
        }
        
        // Draw intersection lines from other planes
        self.draw_plane_intersections(&painter, rect, cell_size);
        
        // Draw stones in current plane
        for (coord, color) in &self.board {
            if self.coord_in_view(coord) {
                let (x, y) = self.project_to_2d(coord);
                let pos = Pos2::new(
                    rect.min.x + (x as f32 + 0.5) * cell_size,
                    rect.min.y + (y as f32 + 0.5) * cell_size
                );
                
                // Draw sphere as circle with gradient
                self.draw_sphere(&painter, pos, cell_size * 0.4, *color);
                
                // Highlight last move
                if Some(*coord) == self.last_move {
                    painter.circle(
                        pos,
                        cell_size * 0.45,
                        Color32::TRANSPARENT,
                        Stroke::new(2.0, Color32::from_rgb(100, 200, 100))
                    );
                }
            }
        }
        
        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let relative = pos - rect.min;
                let x = (relative.x / cell_size) as u8;
                let y = (relative.y / cell_size) as u8;
                
                if x < 9 && y < 9 {
                    let coord = self.unproject_from_2d(x, y);
                    self.toggle_stone(coord);
                }
            }
        }
    }
    
    /// Draw the 3D overview showing all planes
    fn draw_3d_overview(&self, ui: &mut Ui, size: f32) {
        ui.group(|ui| {
            ui.label("3D Overview");
            
            let (response, painter) = ui.allocate_painter(Vec2::splat(size), egui::Sense::hover());
            let rect = response.rect;
            
            // Light gray background
            painter.rect_filled(rect, 0.0, Color32::from_gray(250));
            
            // Draw three orthogonal planes
            let center = rect.center();
            let plane_size = size * 0.6;
            let offset = size * 0.15;
            
            // Draw the three orthogonal planes
            
            // XY plane (at Z=4) - horizontal
            let xy_alpha = if matches!(self.current_view, ViewPlane::XY) { 80 } else { 40 };
            painter.rect_filled(
                Rect::from_center_size(
                    center,
                    Vec2::new(plane_size * 0.7, plane_size * 0.7)
                ),
                0.0,
                Color32::from_rgba_unmultiplied(255, 100, 100, xy_alpha)
            );
            
            // XZ plane (at Y=4) - vertical front-back
            let xz_alpha = if matches!(self.current_view, ViewPlane::XZ) { 80 } else { 40 };
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x, center.y - offset),
                    Vec2::new(plane_size * 0.7, plane_size * 0.3)
                ),
                0.0,
                Color32::from_rgba_unmultiplied(100, 100, 255, xz_alpha)
            );
            
            // YZ plane (at X=4) - vertical left-right
            let yz_alpha = if matches!(self.current_view, ViewPlane::YZ) { 80 } else { 40 };
            painter.rect_filled(
                Rect::from_center_size(
                    Pos2::new(center.x - offset * 0.5, center.y - offset * 0.5),
                    Vec2::new(plane_size * 0.3, plane_size * 0.7)
                ),
                0.0,
                Color32::from_rgba_unmultiplied(100, 255, 100, yz_alpha)
            );
            
            // Draw stones as small dots
            for (coord, color) in &self.board {
                let pos = self.project_to_3d_overview(coord, center, plane_size * 0.7, offset * 0.3);
                painter.circle_filled(pos, 3.0, color.to_color32());
            }
        });
    }
    
    /// Draw intersection lines from other planes
    fn draw_plane_intersections(&self, painter: &Painter, rect: Rect, cell_size: f32) {
        
        match self.current_view {
            ViewPlane::XY => {
                // Draw intersection with XZ plane (at Y=4)
                let y_pos = rect.min.y + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(rect.min.x, y_pos),
                        Pos2::new(rect.max.x, y_pos)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(100, 100, 255))
                );
                
                // Draw intersection with YZ plane (at X=4)
                let x_pos = rect.min.x + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(x_pos, rect.min.y),
                        Pos2::new(x_pos, rect.max.y)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(100, 255, 100))
                );
            }
            ViewPlane::XZ => {
                // Draw intersection with XY plane (at Z=4)
                let z_pos = rect.min.y + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(rect.min.x, z_pos),
                        Pos2::new(rect.max.x, z_pos)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(255, 100, 100))
                );
                
                // Draw intersection with YZ plane (at X=4)
                let x_pos = rect.min.x + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(x_pos, rect.min.y),
                        Pos2::new(x_pos, rect.max.y)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(100, 255, 100))
                );
            }
            ViewPlane::YZ => {
                // Draw intersection with XY plane (at Z=4)
                let z_pos = rect.min.y + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(rect.min.x, z_pos),
                        Pos2::new(rect.max.x, z_pos)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(255, 100, 100))
                );
                
                // Draw intersection with XZ plane (at Y=4)
                let y_pos = rect.min.x + 4.5 * cell_size;
                painter.line_segment(
                    [
                        Pos2::new(y_pos, rect.min.y),
                        Pos2::new(y_pos, rect.max.y)
                    ],
                    Stroke::new(2.0, Color32::from_rgb(100, 100, 255))
                );
            }
        }
    }
    
    /// Draw a sphere (stone) with simple shading
    fn draw_sphere(&self, painter: &Painter, pos: Pos2, radius: f32, color: Color3D) {
        // Simple 3-layer gradient for 3D effect
        for i in 0..3 {
            let t = i as f32 / 2.0;
            let layer_radius = radius * (1.0 - t * 0.15);
            
            let layer_color = match color {
                Color3D::Black => Color32::from_gray((10.0 + t * 20.0) as u8),
                Color3D::White => Color32::from_gray((250.0 - t * 20.0) as u8),
                Color3D::Red => Color32::from_rgb(
                    (200.0 - t * 30.0) as u8,
                    (50.0 - t * 20.0) as u8,
                    (50.0 - t * 20.0) as u8
                ),
            };
            
            painter.circle_filled(pos, layer_radius, layer_color);
        }
        
        // Outline
        painter.circle(
            pos,
            radius,
            Color32::TRANSPARENT,
            Stroke::new(0.8, Color32::from_gray(100))
        );
    }
    
    /// Check if coordinate is in current view plane
    fn coord_in_view(&self, coord: &Coord3D) -> bool {
        match self.current_view {
            ViewPlane::XY => coord.z == 4,
            ViewPlane::XZ => coord.y == 4,
            ViewPlane::YZ => coord.x == 4,
        }
    }
    
    /// Project 3D coordinate to 2D for current view
    fn project_to_2d(&self, coord: &Coord3D) -> (u8, u8) {
        match self.current_view {
            ViewPlane::XY => (coord.x, coord.y),
            ViewPlane::XZ => (coord.x, coord.z),
            ViewPlane::YZ => (coord.y, coord.z),
        }
    }
    
    /// Unproject 2D coordinate to 3D for current view
    fn unproject_from_2d(&self, x: u8, y: u8) -> Coord3D {
        match self.current_view {
            ViewPlane::XY => Coord3D::new(x, y, 4),
            ViewPlane::XZ => Coord3D::new(x, 4, y),
            ViewPlane::YZ => Coord3D::new(4, x, y),
        }
    }
    
    /// Project to 3D overview position
    fn project_to_3d_overview(&self, coord: &Coord3D, center: Pos2, size: f32, spacing: f32) -> Pos2 {
        let x = center.x + (coord.x as f32 - 4.0) * size / 9.0 * 0.7;
        let y = center.y + (coord.y as f32 - 4.0) * size / 9.0 * 0.7 - coord.z as f32 * spacing;
        Pos2::new(x, y)
    }
}