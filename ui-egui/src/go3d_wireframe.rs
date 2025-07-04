//! 3D Wireframe Go Game - Three intersecting 9x9 planes
//!
//! Visualizes the game as wireframe boxes that can be rotated in 3D space.
//! Three 9x9 planes intersect at their middle rows/columns.

use egui::{Context, Ui, Pos2, Vec2, Color32, Stroke, Painter, Sense};
use std::collections::HashMap;

/// 3D point in space
#[derive(Debug, Clone, Copy)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

impl Point3D {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    
    /// Rotate around Y axis
    fn rotate_y(&self, angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a - self.z * sin_a,
            y: self.y,
            z: self.x * sin_a + self.z * cos_a,
        }
    }
    
    /// Rotate around X axis
    fn rotate_x(&self, angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x,
            y: self.y * cos_a - self.z * sin_a,
            z: self.y * sin_a + self.z * cos_a,
        }
    }
    
    /// Project to 2D screen coordinates
    fn project(&self, center: Pos2, scale: f32) -> Pos2 {
        // Simple perspective projection
        let perspective = 1.0 / (1.0 + self.z * 0.001);
        Pos2::new(
            center.x + self.x * scale * perspective,
            center.y - self.y * scale * perspective, // Negative because Y goes down in screen coords
        )
    }
}

/// Player color in 3D Go
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color3D {
    Black,
    White,
    Red,
}

impl Color3D {
    pub fn to_color32(&self) -> Color32 {
        match self {
            Color3D::Black => Color32::from_rgb(20, 20, 20),
            Color3D::White => Color32::from_rgb(240, 240, 240),
            Color3D::Red => Color32::from_rgb(200, 50, 50),
        }
    }
    
    pub fn next(&self) -> Self {
        match self {
            Color3D::Black => Color3D::White,
            Color3D::White => Color3D::Red,
            Color3D::Red => Color3D::Black,
        }
    }
}

/// 3D coordinate on the intersecting planes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord3D {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Coord3D {
    pub fn new(x: i8, y: i8, z: i8) -> Self {
        Self { x, y, z }
    }
    
    /// Check if this coordinate is on one of the three planes
    pub fn is_valid(&self) -> bool {
        let on_xy = self.z == 0 && self.x >= -4 && self.x <= 4 && self.y >= -4 && self.y <= 4;
        let on_xz = self.y == 0 && self.x >= -4 && self.x <= 4 && self.z >= -4 && self.z <= 4;
        let on_yz = self.x == 0 && self.y >= -4 && self.y <= 4 && self.z >= -4 && self.z <= 4;
        on_xy || on_xz || on_yz
    }
}

/// 3D Wireframe Go game
pub struct Go3DWireframe {
    /// Stones on the board
    stones: HashMap<Coord3D, Color3D>,
    /// Current player
    current_player: Color3D,
    /// Rotation angles
    rotation_x: f32,
    rotation_y: f32,
    /// Mouse drag state
    dragging: bool,
    last_mouse_pos: Option<Pos2>,
    /// Selected position for stone placement
    selected_pos: Option<Coord3D>,
    /// Grid scale
    grid_scale: f32,
}

impl Go3DWireframe {
    pub fn new() -> Self {
        Self {
            stones: HashMap::new(),
            current_player: Color3D::Black,
            rotation_x: 0.3,
            rotation_y: 0.5,
            dragging: false,
            last_mouse_pos: None,
            selected_pos: None,
            grid_scale: 30.0,
        }
    }
    
    /// Place a stone at the given coordinate
    pub fn place_stone(&mut self, coord: Coord3D) {
        if coord.is_valid() && !self.stones.contains_key(&coord) {
            self.stones.insert(coord, self.current_player);
            self.current_player = self.current_player.next();
        }
    }
    
    /// Render the 3D wireframe game
    pub fn ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default()
                .fill(Color32::from_rgb(20, 20, 20)) // Dark background for better contrast
                .inner_margin(egui::Margin::same(20.0)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Title
                    ui.label(egui::RichText::new("3D Go - Wireframe View").size(24.0).color(Color32::WHITE));
                    ui.label("Three intersecting 9×9 planes • Click boxes to place stones");
                    ui.add_space(10.0);
                    
                    // Current player
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Current Player:").color(Color32::WHITE));
                        let (response, painter) = ui.allocate_painter(Vec2::splat(20.0), Sense::hover());
                        painter.circle_filled(
                            response.rect.center(),
                            8.0,
                            self.current_player.to_color32()
                        );
                        
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(format!("Stones placed: {}", self.stones.len())).color(Color32::WHITE));
                    });
                    
                    ui.add_space(20.0);
                    
                    // 3D view
                    self.draw_3d_view(ui);
                    
                    ui.add_space(10.0);
                    
                    // Controls
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Drag to rotate • Click boxes to place stones").color(Color32::GRAY));
                        
                        ui.add_space(20.0);
                        
                        if ui.button("Clear Board").clicked() {
                            self.stones.clear();
                            self.current_player = Color3D::Black;
                        }
                        
                        ui.add_space(10.0);
                        
                        if ui.button("Random Stones").clicked() {
                            self.add_random_stones();
                        }
                    });
                });
            });
    }
    
    /// Draw the 3D wireframe view
    fn draw_3d_view(&mut self, ui: &mut Ui) {
        let desired_size = ui.available_size();
        let size = Vec2::new(
            desired_size.x.min(800.0),
            desired_size.y.min(600.0)
        );
        
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = response.rect;
        let center = rect.center();
        
        // Handle mouse rotation
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(last_pos) = self.last_mouse_pos {
                    let delta = pos - last_pos;
                    self.rotation_y += delta.x * 0.01;
                    self.rotation_x += delta.y * 0.01;
                    self.rotation_x = self.rotation_x.clamp(-1.5, 1.5);
                }
                self.last_mouse_pos = Some(pos);
            }
        } else {
            self.last_mouse_pos = None;
        }
        
        // Draw the three intersecting planes as wireframe grids
        self.draw_wireframe_planes(&painter, center);
        
        // Draw stones
        self.draw_stones(&painter, center);
        
        // Handle clicks for stone placement
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(coord) = self.find_clicked_position(pos, center) {
                    self.place_stone(coord);
                }
            }
        }
    }
    
    /// Draw the three wireframe planes
    fn draw_wireframe_planes(&self, painter: &Painter, center: Pos2) {
        // Define the three planes
        let planes = [
            // XY plane (Z=0)
            (Color32::from_rgba_unmultiplied(255, 100, 100, 100), 0),
            // XZ plane (Y=0)
            (Color32::from_rgba_unmultiplied(100, 255, 100, 100), 1),
            // YZ plane (X=0)
            (Color32::from_rgba_unmultiplied(100, 100, 255, 100), 2),
        ];
        
        for (plane_color, plane_type) in planes {
            // Draw 9x9 grid of boxes
            for i in -4..=4 {
                for j in -4..=4 {
                    let coord = match plane_type {
                        0 => Coord3D::new(i, j, 0), // XY plane
                        1 => Coord3D::new(i, 0, j), // XZ plane
                        2 => Coord3D::new(0, i, j), // YZ plane
                        _ => continue,
                    };
                    
                    self.draw_wireframe_box(painter, center, coord, plane_color);
                }
            }
        }
        
        // Highlight intersection lines with brighter color
        let intersection_color = Color32::from_rgb(255, 255, 100);
        let intersection_stroke = Stroke::new(2.0, intersection_color);
        
        // Draw intersection lines more prominently
        for i in -4..=4 {
            // X-axis (intersection of XY and XZ planes)
            let p1 = self.world_to_screen(Point3D::new(i as f32, 0.0, 0.0), center);
            let p2 = self.world_to_screen(Point3D::new((i + 1) as f32, 0.0, 0.0), center);
            painter.line_segment([p1, p2], intersection_stroke);
            
            // Y-axis (intersection of XY and YZ planes)
            let p1 = self.world_to_screen(Point3D::new(0.0, i as f32, 0.0), center);
            let p2 = self.world_to_screen(Point3D::new(0.0, (i + 1) as f32, 0.0), center);
            painter.line_segment([p1, p2], intersection_stroke);
            
            // Z-axis (intersection of XZ and YZ planes)
            let p1 = self.world_to_screen(Point3D::new(0.0, 0.0, i as f32), center);
            let p2 = self.world_to_screen(Point3D::new(0.0, 0.0, (i + 1) as f32), center);
            painter.line_segment([p1, p2], intersection_stroke);
        }
    }
    
    /// Draw a single wireframe box
    fn draw_wireframe_box(&self, painter: &Painter, center: Pos2, coord: Coord3D, color: Color32) {
        let x = coord.x as f32;
        let y = coord.y as f32;
        let z = coord.z as f32;
        let s = 0.5; // Half size of box
        
        // Define the 8 corners of the box
        let corners = [
            Point3D::new(x - s, y - s, z - s),
            Point3D::new(x + s, y - s, z - s),
            Point3D::new(x + s, y + s, z - s),
            Point3D::new(x - s, y + s, z - s),
            Point3D::new(x - s, y - s, z + s),
            Point3D::new(x + s, y - s, z + s),
            Point3D::new(x + s, y + s, z + s),
            Point3D::new(x - s, y + s, z + s),
        ];
        
        // Convert to screen coordinates
        let screen_corners: Vec<Pos2> = corners.iter()
            .map(|p| self.world_to_screen(*p, center))
            .collect();
        
        let stroke = Stroke::new(1.0, color);
        
        // Draw the 12 edges of the box
        let edges = [
            // Bottom face
            (0, 1), (1, 2), (2, 3), (3, 0),
            // Top face
            (4, 5), (5, 6), (6, 7), (7, 4),
            // Vertical edges
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];
        
        for (i, j) in edges {
            painter.line_segment([screen_corners[i], screen_corners[j]], stroke);
        }
    }
    
    /// Draw stones as spheres
    fn draw_stones(&self, painter: &Painter, center: Pos2) {
        for (coord, color) in &self.stones {
            let world_pos = Point3D::new(coord.x as f32, coord.y as f32, coord.z as f32);
            let screen_pos = self.world_to_screen(world_pos, center);
            
            // Draw sphere with simple shading
            let radius = 12.0;
            
            // Shadow
            painter.circle_filled(
                screen_pos + Vec2::new(2.0, 2.0),
                radius * 1.1,
                Color32::from_rgba_unmultiplied(0, 0, 0, 50)
            );
            
            // Main sphere
            painter.circle_filled(screen_pos, radius, color.to_color32());
            
            // Highlight
            let highlight_offset = Vec2::new(-radius * 0.3, -radius * 0.3);
            painter.circle_filled(
                screen_pos + highlight_offset,
                radius * 0.3,
                Color32::from_rgba_unmultiplied(255, 255, 255, 100)
            );
            
            // Outline
            painter.circle_stroke(
                screen_pos,
                radius,
                Stroke::new(1.0, Color32::from_gray(100))
            );
        }
    }
    
    /// Convert world coordinates to screen coordinates
    fn world_to_screen(&self, point: Point3D, center: Pos2) -> Pos2 {
        let rotated = point
            .rotate_y(self.rotation_y)
            .rotate_x(self.rotation_x);
        rotated.project(center, self.grid_scale)
    }
    
    /// Find which grid position was clicked
    fn find_clicked_position(&self, click_pos: Pos2, center: Pos2) -> Option<Coord3D> {
        let mut best_coord = None;
        let mut best_distance = f32::MAX;
        
        // Check all valid positions
        for x in -4..=4 {
            for y in -4..=4 {
                for z in -4..=4 {
                    let coord = Coord3D::new(x, y, z);
                    if !coord.is_valid() {
                        continue;
                    }
                    
                    let world_pos = Point3D::new(x as f32, y as f32, z as f32);
                    let screen_pos = self.world_to_screen(world_pos, center);
                    let distance = (screen_pos - click_pos).length();
                    
                    if distance < 20.0 && distance < best_distance {
                        best_distance = distance;
                        best_coord = Some(coord);
                    }
                }
            }
        }
        
        best_coord
    }
    
    /// Add random stones for testing
    fn add_random_stones(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for _ in 0..15 {
            let plane = rng.gen_range(0..3);
            let i = rng.gen_range(-4..=4);
            let j = rng.gen_range(-4..=4);
            
            let coord = match plane {
                0 => Coord3D::new(i, j, 0),
                1 => Coord3D::new(i, 0, j),
                _ => Coord3D::new(0, i, j),
            };
            
            if !self.stones.contains_key(&coord) {
                let color = match rng.gen_range(0..3) {
                    0 => Color3D::Black,
                    1 => Color3D::White,
                    _ => Color3D::Red,
                };
                self.stones.insert(coord, color);
            }
        }
    }
}