//! Placeholder for neural network functionality

use egui::Context;

pub struct NeuralTrainingUI;

pub struct NeuralOverlay {
    pub enabled: bool,
}

impl NeuralTrainingUI {
    pub fn new() -> Self {
        Self
    }
    pub fn show(&mut self, _ctx: &Context) {}
    pub fn ui(&mut self, ctx: &Context) {
        self.show(ctx)
    }
    pub fn render(&mut self, _ui: &mut egui::Ui) -> Option<Vec<u8>> {
        None
    }
}

impl NeuralOverlay {
    pub fn new() -> Self {
        Self { enabled: false }
    }
    pub fn show(&mut self, _ctx: &Context) {}
    pub fn render(
        &mut self,
        _ctx: &Context,
        _ui: &mut egui::Ui,
        _board_widget: &crate::board_widget::BoardWidget,
    ) {
    }
    pub fn render_overlay(
        &mut self,
        _ui: &mut egui::Ui,
        _board_rect: egui::Rect,
        _cell_size: f32,
        _game_state: &p2pgo_core::GameState,
    ) {
    }
    pub fn update(&mut self, _game_state: &p2pgo_core::GameState, _dt: f32) {}
    pub fn render_controls(&mut self, _ui: &mut egui::Ui) {}
    pub fn render_win_probability(
        &mut self,
        _ui: &mut egui::Ui,
        _game_state: &p2pgo_core::GameState,
    ) {
    }
}
