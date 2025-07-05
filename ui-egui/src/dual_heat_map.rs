use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Ui, Vec2};
use p2pgo_core::{Color as GoColor, GameState};

/// Dual heat map for combining sword and shield neural networks
/// Uses orthogonal colors to create interference patterns
pub struct DualHeatMap {
    /// Sword network heat map (attack/offensive moves)
    sword_enabled: bool,
    /// Shield network heat map (defense/defensive moves)
    shield_enabled: bool,
    /// Combined opacity (0.0 - 1.0)
    opacity: f32,
    /// Blend mode for combining networks
    blend_mode: BlendMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum BlendMode {
    /// Additive blending - colors add together
    Additive,
    /// Multiplicative - colors multiply (darker)
    Multiplicative,
    /// Interference - creates color patterns from orthogonal colors
    Interference,
}

impl DualHeatMap {
    pub fn new() -> Self {
        Self {
            sword_enabled: false,
            shield_enabled: false,
            opacity: 0.6,
            blend_mode: BlendMode::Interference,
        }
    }

    /// Render combined heat map overlay
    pub fn render_overlay(
        &self,
        ui: &mut Ui,
        board_rect: Rect,
        cell_size: f32,
        game_state: &GameState,
        sword_predictions: &[[f32; 19]; 19],
        shield_predictions: &[[f32; 19]; 19],
    ) {
        if !self.sword_enabled && !self.shield_enabled {
            return;
        }

        let painter = ui.painter();

        // Draw combined heat map
        for y in 0..19 {
            for x in 0..19 {
                let sword_prob = if self.sword_enabled {
                    sword_predictions[y][x]
                } else {
                    0.0
                };
                let shield_prob = if self.shield_enabled {
                    shield_predictions[y][x]
                } else {
                    0.0
                };

                // Skip very low probability moves
                if sword_prob < 0.01 && shield_prob < 0.01 {
                    continue;
                }

                // Calculate position
                let pos = Pos2::new(
                    board_rect.min.x + (x as f32 + 0.5) * cell_size,
                    board_rect.min.y + (y as f32 + 0.5) * cell_size,
                );

                // Get blended color using orthogonal color space
                let color = self.blend_colors(sword_prob, shield_prob);
                let max_prob = sword_prob.max(shield_prob);
                let radius = cell_size * 0.4 * max_prob.sqrt();

                // Draw heat indicator
                painter.circle_filled(pos, radius, color);

                // Draw edge indicators for high probability moves
                if max_prob > 0.3 {
                    let stroke_color = if sword_prob > shield_prob {
                        Color32::from_rgba_unmultiplied(255, 0, 0, 100) // Red edge for sword
                    } else {
                        Color32::from_rgba_unmultiplied(0, 0, 255, 100) // Blue edge for shield
                    };
                    painter.circle_stroke(pos, radius + 2.0, Stroke::new(1.0, stroke_color));
                }
            }
        }
    }

    /// Blend colors using orthogonal color channels
    fn blend_colors(&self, sword_prob: f32, shield_prob: f32) -> Color32 {
        let alpha = (self.opacity * 255.0) as u8;

        match self.blend_mode {
            BlendMode::Additive => {
                // Sword uses red channel, Shield uses blue channel
                // Creates purple/magenta where they overlap
                let red = (sword_prob * 255.0) as u8;
                let blue = (shield_prob * 255.0) as u8;
                let green = 0; // Keep green at 0 for pure red-blue mixing
                Color32::from_rgba_unmultiplied(red, green, blue, alpha)
            }
            BlendMode::Multiplicative => {
                // Darker where both networks agree
                let combined = sword_prob * shield_prob;
                let intensity = ((1.0 - combined) * 255.0) as u8;
                Color32::from_rgba_unmultiplied(intensity, intensity, intensity, alpha)
            }
            BlendMode::Interference => {
                // Use orthogonal color vectors to create interference patterns
                // Sword: Red-Yellow (warm colors)
                // Shield: Blue-Cyan (cool colors)
                // Overlap creates various greens and purples

                // Sword contribution (red + half green = orange/yellow)
                let sword_red = (sword_prob * 255.0) as u8;
                let sword_green = (sword_prob * 127.0) as u8;

                // Shield contribution (blue + half green = cyan)
                let shield_blue = (shield_prob * 255.0) as u8;
                let shield_green = (shield_prob * 127.0) as u8;

                // Combine with interference
                let red = sword_red;
                let blue = shield_blue;
                let green = sword_green.saturating_add(shield_green).min(255);

                // Add slight phase shift for more interesting patterns
                let phase = ((sword_prob - shield_prob).abs() * 0.3) as u8;

                Color32::from_rgba_unmultiplied(
                    red.saturating_add(phase),
                    green,
                    blue.saturating_add(phase),
                    alpha,
                )
            }
        }
    }

    /// Render control panel
    pub fn render_controls(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label("Neural Heat Maps");

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.sword_enabled, "⚔️ Sword (Attack)");
                ui.checkbox(&mut self.shield_enabled, "🛡️ Shield (Defense)");
            });

            if self.sword_enabled || self.shield_enabled {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    ui.add(egui::Slider::new(&mut self.opacity, 0.1..=1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Blend:");
                    ui.selectable_value(
                        &mut self.blend_mode,
                        BlendMode::Interference,
                        "Interference",
                    );
                    ui.selectable_value(&mut self.blend_mode, BlendMode::Additive, "Additive");
                    ui.selectable_value(
                        &mut self.blend_mode,
                        BlendMode::Multiplicative,
                        "Multiply",
                    );
                });

                // Color legend
                ui.separator();
                ui.label("Color Legend:");
                ui.horizontal(|ui| {
                    // Sword color sample
                    let sword_color = self.blend_colors(1.0, 0.0);
                    ui.colored_label(sword_color, "█");
                    ui.label("Sword");

                    // Shield color sample
                    let shield_color = self.blend_colors(0.0, 1.0);
                    ui.colored_label(shield_color, "█");
                    ui.label("Shield");

                    // Combined color sample
                    let combined_color = self.blend_colors(0.7, 0.7);
                    ui.colored_label(combined_color, "█");
                    ui.label("Both");
                });
            }
        });
    }

    /// Check if any heat map is enabled
    pub fn is_enabled(&self) -> bool {
        self.sword_enabled || self.shield_enabled
    }

    /// Toggle sword heat map
    pub fn toggle_sword(&mut self) {
        self.sword_enabled = !self.sword_enabled;
    }

    /// Toggle shield heat map
    pub fn toggle_shield(&mut self) {
        self.shield_enabled = !self.shield_enabled;
    }
}
