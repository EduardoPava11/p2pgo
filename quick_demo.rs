//! Quick P2P Go Demo - Standalone binary for DMG
//! This creates a simple playable Go game without networking dependencies

use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Vec2};
use p2pgo_core::{GameState, Move, Color, Coord};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("P2P Go - Demo")
            .with_inner_size([800.0, 850.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "P2P Go Demo",
        options,
        Box::new(|_cc| Box::new(P2PGoDemo::default())),
    )
}

struct P2PGoDemo {
    game_state: GameState,
    board_size: usize,
    last_move: Option<Coord>,
}

impl Default for P2PGoDemo {
    fn default() -> Self {
        let board_size = 9;
        Self {
            game_state: GameState::new(board_size as u8),
            board_size,
            last_move: None,
        }
    }
}

impl eframe::App for P2PGoDemo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("P2P Go - Demo Game");
            
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Current Player: {}",
                    match self.game_state.current_player {
                        Color::Black => "Black ⚫",
                        Color::White => "White ⚪",
                    }
                ));
                
                if ui.button("Pass").clicked() {
                    let _ = self.game_state.apply_move(Move::Pass);
                }
                
                if ui.button("New Game").clicked() {
                    self.game_state = GameState::new(self.board_size as u8);
                    self.last_move = None;
                }
            });
            
            ui.separator();
            
            // Draw the board
            let available_size = ui.available_size();
            let board_size_px = available_size.x.min(available_size.y - 50.0);
            let cell_size = board_size_px / (self.board_size + 1) as f32;
            let board_rect = Rect::from_min_size(
                ui.cursor().min + Vec2::new(cell_size, cell_size),
                Vec2::splat(board_size_px - 2.0 * cell_size),
            );
            
            let response = ui.allocate_rect(
                Rect::from_min_size(ui.cursor().min, Vec2::splat(board_size_px)),
                Sense::click(),
            );
            
            let painter = ui.painter();
            
            // Board background
            painter.rect_filled(board_rect, 0.0, Color32::from_rgb(220, 179, 92));
            
            // Grid lines
            for i in 0..self.board_size {
                let x = board_rect.min.x + i as f32 * cell_size;
                let y = board_rect.min.y + i as f32 * cell_size;
                
                // Vertical lines
                painter.line_segment(
                    [Pos2::new(x, board_rect.min.y), Pos2::new(x, board_rect.max.y)],
                    Stroke::new(1.0, Color32::BLACK),
                );
                
                // Horizontal lines
                painter.line_segment(
                    [Pos2::new(board_rect.min.x, y), Pos2::new(board_rect.max.x, y)],
                    Stroke::new(1.0, Color32::BLACK),
                );
            }
            
            // Star points for 9x9 board
            if self.board_size == 9 {
                let star_points = [(2, 2), (6, 2), (4, 4), (2, 6), (6, 6)];
                for (x, y) in star_points.iter() {
                    let pos = Pos2::new(
                        board_rect.min.x + *x as f32 * cell_size,
                        board_rect.min.y + *y as f32 * cell_size,
                    );
                    painter.circle_filled(pos, 3.0, Color32::BLACK);
                }
            }
            
            // Draw stones
            for y in 0..self.board_size {
                for x in 0..self.board_size {
                    let idx = y * self.board_size + x;
                    if let Some(color) = self.game_state.board[idx] {
                        let pos = Pos2::new(
                            board_rect.min.x + x as f32 * cell_size,
                            board_rect.min.y + y as f32 * cell_size,
                        );
                        
                        let stone_color = match color {
                            Color::Black => Color32::BLACK,
                            Color::White => Color32::WHITE,
                        };
                        
                        // Stone with slight 3D effect
                        painter.circle_filled(pos, cell_size * 0.45, stone_color);
                        painter.circle_stroke(
                            pos, 
                            cell_size * 0.45, 
                            Stroke::new(1.0, Color32::from_gray(64))
                        );
                        
                        // Highlight last move
                        if self.last_move == Some(Coord::new(x as u8, y as u8)) {
                            painter.circle_stroke(
                                pos,
                                cell_size * 0.25,
                                Stroke::new(2.0, Color32::RED),
                            );
                        }
                    }
                }
            }
            
            // Handle clicks
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let x = ((pos.x - board_rect.min.x + cell_size * 0.5) / cell_size) as usize;
                    let y = ((pos.y - board_rect.min.y + cell_size * 0.5) / cell_size) as usize;
                    
                    if x < self.board_size && y < self.board_size {
                        let coord = Coord::new(x as u8, y as u8);
                        let color = self.game_state.current_player;
                        if let Ok(_) = self.game_state.apply_move(Move::Place { x: x as u8, y: y as u8, color }) {
                            self.last_move = Some(coord);
                        }
                    }
                }
            }
            
            // Game info
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Move: {}", self.game_state.moves.len()));
                ui.label(format!("Captures - Black: {}, White: {}", 
                    self.game_state.captures.0, 
                    self.game_state.captures.1
                ));
            });
            
            if self.game_state.result.is_some() {
                ui.separator();
                ui.heading("Game Over!");
                if let Some(result) = &self.game_state.result {
                    ui.label(format!("Result: {:?}", result));
                }
            }
        });
    }
}