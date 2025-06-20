// SPDX-License-Identifier: MIT OR Apache-2.0

//! Go board widget for rendering the game board.

use eframe::egui::{self, Color32, Stroke, Pos2, Vec2, Rect};
use p2pgo_core::{GameState, Color, Coord, Tag};
use crossbeam_channel::Sender;
use crate::msg::UiToNet;

/// Widget for rendering and interacting with a Go board
pub struct BoardWidget {
    /// Board size
    board_size: u8,
    /// Cell size in pixels
    cell_size: f32,
    /// Current tag palette selection
    tag_palette: Option<Tag>,
    /// Ghost stones (AI suggestions) to display
    ghost_stones: Vec<Coord>,
}

impl BoardWidget {
    pub fn new(board_size: u8) -> Self {
        Self {
            board_size,
            cell_size: 30.0,
            tag_palette: None,
            ghost_stones: Vec::new(),
        }
    }

    /// Get the board size
    pub fn get_board_size(&self) -> u8 {
        self.board_size
    }

    /// Render the board and return clicked coordinate if any
    pub fn render(&mut self, ui: &mut egui::Ui, game_state: &GameState, ui_tx: Option<&Sender<UiToNet>>) -> Option<Coord> {
        let board_pixel_size = self.cell_size * (self.board_size as f32 - 1.0);
        let desired_size = Vec2::splat(board_pixel_size + 40.0); // Extra space for margins
        
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        
        if ui.is_rect_visible(rect) {
            self.paint_board(ui, rect, game_state);
        }
        
        // Handle key bindings for tag palette
        if ui.input(|i| i.key_pressed(egui::Key::A)) {
            self.tag_palette = Some(Tag::Activity);
        }
        if ui.input(|i| i.key_pressed(egui::Key::B)) {
            self.tag_palette = Some(Tag::Avoidance);
        }
        if ui.input(|i| i.key_pressed(egui::Key::R)) {
            self.tag_palette = Some(Tag::Reactivity);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.tag_palette = None;
        }
        
        // Handle clicks
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some(coord) = self.pos_to_coord(pos, rect) {
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    
                    tracing::debug!(
                        x = coord.x,
                        y = coord.y,
                        pos_x = pos.x,
                        pos_y = pos.y,
                        shift_held = shift_held,
                        tag_palette = ?self.tag_palette,
                        "Board click detected"
                    );
                    
                    // Handle Shift+click for tag palette popup
                    if shift_held {
                        let popup_id = ui.id().with("tag_palette_popup");
                        ui.memory_mut(|mem| mem.open_popup(popup_id));
                        
                        // Get response from last widget to position the popup correctly
                        let last_response = ui.label(""); // Temporary widget to get a response
                        egui::popup::popup_below_widget(ui, popup_id, &last_response, |ui| {
                            ui.set_min_width(120.0);
                            ui.vertical(|ui| {
                                if ui.button("Activity (A)").clicked() {
                                    self.tag_palette = Some(Tag::Activity);
                                    // Send tag to network if UI channel exists
                                    if let Some(tx) = ui_tx {
                                        if let Some(gid) = self.extract_game_id_from_ui(ui) {
                                            let _ = tx.send(UiToNet::SetTag {
                                                gid,
                                                seq: 0, // Use current move sequence number
                                                tag: Tag::Activity,
                                            });
                                        }
                                    }
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                                if ui.button("Avoidance (B)").clicked() {
                                    self.tag_palette = Some(Tag::Avoidance);
                                    if let Some(tx) = ui_tx {
                                        if let Some(gid) = self.extract_game_id_from_ui(ui) {
                                            let _ = tx.send(UiToNet::SetTag {
                                                gid,
                                                seq: 0,
                                                tag: Tag::Avoidance,
                                            });
                                        }
                                    }
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                                if ui.button("Reactivity (R)").clicked() {
                                    self.tag_palette = Some(Tag::Reactivity);
                                    if let Some(tx) = ui_tx {
                                        if let Some(gid) = self.extract_game_id_from_ui(ui) {
                                            let _ = tx.send(UiToNet::SetTag {
                                                gid,
                                                seq: 0,
                                                tag: Tag::Reactivity,
                                            });
                                        }
                                    }
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                                if ui.button("Clear").clicked() {
                                    self.tag_palette = None;
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                            });
                        });
                        return None; // Don't return coordinate for tag popup
                    }
                    
                    // Send debug event for testing
                    if let Some(tx) = ui_tx {
                        let _ = tx.send(UiToNet::DebugMovePlaced(coord));
                    }
                    return Some(coord);
                }
            }
        }
        
        None
    }

    fn paint_board(&self, ui: &mut egui::Ui, rect: Rect, game_state: &GameState) {
        let painter = ui.painter_at(rect);
        
        // Board background
        painter.rect_filled(rect, 5.0, Color32::from_rgb(220, 179, 92));
        
        let margin = 20.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(rect.width() - 2.0 * margin),
        );
        
        // Draw grid lines
        let line_color = Color32::BLACK;
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
        
        // Draw star points for standard board sizes
        if self.board_size == 19 {
            let star_points = vec![
                (3, 3), (3, 9), (3, 15),
                (9, 3), (9, 9), (9, 15),
                (15, 3), (15, 9), (15, 15),
            ];
            for (x, y) in star_points {
                let pos = self.coord_to_pos(Coord { x, y }, board_rect);
                painter.circle_filled(pos, 3.0, line_color);
            }
        } else if self.board_size == 13 {
            let star_points = vec![(3, 3), (3, 9), (6, 6), (9, 3), (9, 9)];
            for (x, y) in star_points {
                let pos = self.coord_to_pos(Coord { x, y }, board_rect);
                painter.circle_filled(pos, 3.0, line_color);
            }
        } else if self.board_size == 9 {
            let star_points = vec![(2, 2), (2, 6), (4, 4), (6, 2), (6, 6)];
            for (x, y) in star_points {
                let pos = self.coord_to_pos(Coord { x, y }, board_rect);
                painter.circle_filled(pos, 3.0, line_color);
            }
        }
        
        // Draw stones
        let stone_radius = self.cell_size * 0.4;
        for x in 0..self.board_size {
            for y in 0..self.board_size {
                let coord = Coord { x, y };
                // Convert coordinate to index in board vector
                let idx = (coord.y as usize) * (game_state.board_size as usize) + (coord.x as usize);
                if let Some(color) = game_state.board.get(idx).and_then(|c| *c) {
                    let pos = self.coord_to_pos(coord, board_rect);
                    let stone_color = match color {
                        Color::Black => Color32::BLACK,
                        Color::White => Color32::WHITE,
                    };
                    painter.circle_filled(pos, stone_radius, stone_color);
                    painter.circle_stroke(pos, stone_radius, Stroke::new(1.0, Color32::BLACK));
                }
            }
        }
        
        // Draw ghost stones (AI suggestions) with 50% alpha
        for coord in &self.ghost_stones {
            let pos = self.coord_to_pos(*coord, board_rect);
            let ghost_color = match game_state.current_player {
                Color::Black => Color32::from_rgba_unmultiplied(0, 0, 0, 80),
                Color::White => Color32::from_rgba_unmultiplied(255, 255, 255, 80),
            };
            painter.circle_filled(pos, stone_radius * 0.8, ghost_color);
        }
    }

    fn coord_to_pos(&self, coord: Coord, board_rect: Rect) -> Pos2 {
        let x = board_rect.min.x + (coord.x as f32) * self.cell_size;
        let y = board_rect.min.y + (coord.y as f32) * self.cell_size;
        Pos2::new(x, y)
    }

    fn pos_to_coord(&self, pos: Pos2, rect: Rect) -> Option<Coord> {
        let margin = 20.0;
        let board_rect = Rect::from_min_size(
            rect.min + Vec2::splat(margin),
            Vec2::splat(rect.width() - 2.0 * margin),
        );
        
        if !board_rect.contains(pos) {
            return None;
        }
        
        let rel_pos = pos - board_rect.min;
        let x = (rel_pos.x / self.cell_size).round() as u8;
        let y = (rel_pos.y / self.cell_size).round() as u8;
        
        if x < self.board_size && y < self.board_size {
            Some(Coord { x, y })
        } else {
            None
        }
    }
    
    /// Set ghost stones for AI suggestions
    #[allow(dead_code)]
    pub fn set_ghost_stones(&mut self, stones: Vec<Coord>) {
        self.ghost_stones = stones;
    }
    
    /// Clear all ghost stones
    #[allow(dead_code)]
    pub fn clear_ghost_stones(&mut self) {
        self.ghost_stones.clear();
    }
    
    /// Set the current tag palette selection
    #[allow(dead_code)]
    pub fn set_tag_palette(&mut self, tag: Option<Tag>) {
        self.tag_palette = tag;
    }
    
    /// Get the current tag palette selection
    #[allow(dead_code)]
    pub fn get_tag_palette(&self) -> Option<Tag> {
        self.tag_palette
    }
    
    /// Helper method to extract game ID from the UI context
    fn extract_game_id_from_ui(&self, ui: &egui::Ui) -> Option<String> {
        // This is a bit of a hack - ideally we would get this from the app directly
        // This pattern assumes the game ID is stored in the parent context
        let ctx = ui.ctx();
        let mut memory = ctx.data_mut(|data| {
            data.get_temp::<String>(egui::Id::new("current_game_id"))
        });
        
        // If not found in memory, use fallback "current-game" for testing
        if memory.is_none() {
            memory = Some("current-game".to_string());
        }
        
        memory
    }
    
    /// Handle shift-click at a specific coordinate (for testing)
    #[allow(dead_code)]
    pub fn handle_shift_click(&mut self, _coord: Coord) {
        // Simulate tag palette behavior from shift+click
        // Open tag palette and set to Activity by default
        self.tag_palette = Some(Tag::Activity);
    }
    
    /// Set AI move suggestions as ghost stones
    #[allow(dead_code)]
    pub fn set_ai_suggestions(&mut self, suggestions: Vec<(Coord, f32)>) {
        // Take top 3 suggestions and set as ghost stones
        let mut sorted_suggestions = suggestions;
        sorted_suggestions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        self.ghost_stones = sorted_suggestions.into_iter()
            .take(3)
            .map(|(coord, _)| coord)
            .collect();
    }
}
