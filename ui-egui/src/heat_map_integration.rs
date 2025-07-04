//! Integration code for heat map functionality

use eframe::egui;
use crate::heat_map::HeatMapOverlay;

/// Example integration for heat map toggle
/// 
/// Add this to your game view rendering:
/// ```
/// // In your game view update method:
/// if ctx.input(|i| i.key_pressed(egui::Key::H)) {
///     self.heat_map.toggle();
/// }
/// 
/// // In your board rendering:
/// if self.heat_map.is_enabled() {
///     self.heat_map.render(ui, &game_state, board_rect);
/// }
/// ```
pub struct HeatMapIntegration;

impl HeatMapIntegration {
    /// Example of how to add heat map controls to UI
    pub fn render_controls(ui: &mut egui::Ui, heat_map: &mut HeatMapOverlay) {
        ui.horizontal(|ui| {
            // Toggle button
            let toggle_text = if heat_map.is_enabled() {
                "🔥 Heat Map ON"
            } else {
                "❄️ Heat Map OFF"
            };
            
            if ui.button(toggle_text).clicked() {
                heat_map.toggle();
            }
            
            ui.label("(Press H to toggle)");
            
            // Only show controls when enabled
            if heat_map.is_enabled() {
                ui.separator();
                
                // Opacity slider
                ui.label("Opacity:");
                let mut opacity = heat_map.get_opacity();
                if ui.add(egui::Slider::new(&mut opacity, 0.1..=1.0)
                    .clamp_to_range(true))
                    .changed() 
                {
                    heat_map.set_opacity(opacity);
                }
            }
        });
    }
    
    /// Example of status indicator
    pub fn render_status(ui: &mut egui::Ui, heat_map: &HeatMapOverlay) {
        if heat_map.is_enabled() {
            ui.horizontal(|ui| {
                ui.label("🧠");
                ui.label("Neural assistant active");
            });
        }
    }
}