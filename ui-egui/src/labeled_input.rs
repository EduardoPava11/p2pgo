//! Labeled input components with clear visual distinction
//! for game codes vs connection tickets

use crate::dark_theme::DarkColors;
use egui::{Color32, FontId, Response, Rounding, Ui, Vec2, Widget};

/// Type of identifier being displayed/entered
#[derive(Debug, Clone, PartialEq)]
pub enum IdentifierType {
    GameCode,
    ConnectionTicket,
}

impl IdentifierType {
    fn icon(&self) -> &'static str {
        match self {
            IdentifierType::GameCode => "🎮",
            IdentifierType::ConnectionTicket => "🔑",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            IdentifierType::GameCode => "Game Code",
            IdentifierType::ConnectionTicket => "Connection Ticket",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            IdentifierType::GameCode => "Share this code with others to join your game",
            IdentifierType::ConnectionTicket => "Use this for direct peer-to-peer connection",
        }
    }

    fn help_text(&self) -> &'static str {
        match self {
            IdentifierType::GameCode => {
                "Game codes are short identifiers for joining games through the relay network. \
                 They are easier to share and work when both players are online."
            }
            IdentifierType::ConnectionTicket => {
                "Connection tickets establish direct peer-to-peer connections. \
                 They are longer but provide better performance and work even without relays."
            }
        }
    }

    fn border_style(&self) -> (Color32, f32, bool) {
        let colors = DarkColors::default();
        match self {
            IdentifierType::GameCode => (colors.primary, 2.0, false), // Solid border
            IdentifierType::ConnectionTicket => (colors.secondary, 2.0, true), // Dashed border
        }
    }
}

/// Display a labeled identifier with appropriate styling
pub fn show_labeled_identifier(ui: &mut Ui, id_type: IdentifierType, value: &str) -> Response {
    let colors = DarkColors::default();
    let (border_color, border_width, _is_dashed) = id_type.border_style();

    // Container with border
    let response = egui::Frame::none()
        .fill(colors.surface_variant)
        .stroke(egui::Stroke::new(border_width, border_color))
        .inner_margin(12.0)
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Header with icon and label
                ui.horizontal(|ui| {
                    ui.label(id_type.icon());
                    ui.label(
                        egui::RichText::new(id_type.label())
                            .color(colors.text_primary)
                            .strong(),
                    );

                    // Info button
                    if ui
                        .small_button("ⓘ")
                        .on_hover_text(id_type.help_text())
                        .clicked()
                    {
                        // Could show more detailed help
                    }
                });

                ui.add_space(4.0);

                // Value display with copy button
                ui.horizontal(|ui| {
                    // Monospace font for the value
                    let text = egui::RichText::new(value)
                        .font(FontId::monospace(14.0))
                        .color(colors.text_primary);

                    ui.label(text);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📋 Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = value.to_string());
                        }
                    });
                });

                // Description
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(id_type.description())
                        .color(colors.text_secondary)
                        .small(),
                );
            });
        })
        .response;

    response
}

/// Input field for entering identifiers with clear labeling
pub fn show_labeled_input(
    ui: &mut Ui,
    id_type: IdentifierType,
    value: &mut String,
    placeholder: &str,
) -> Response {
    let colors = DarkColors::default();
    let (border_color, border_width, _is_dashed) = id_type.border_style();

    egui::Frame::none()
        .fill(colors.surface_variant)
        .stroke(egui::Stroke::new(border_width, border_color))
        .inner_margin(12.0)
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(id_type.icon());
                    ui.label(
                        egui::RichText::new(id_type.label())
                            .color(colors.text_primary)
                            .strong(),
                    );
                });

                ui.add_space(4.0);

                // Input field
                let response = ui.add(
                    egui::TextEdit::singleline(value)
                        .font(FontId::monospace(14.0))
                        .hint_text(placeholder)
                        .desired_width(f32::INFINITY),
                );

                // Help text
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(id_type.description())
                        .color(colors.text_secondary)
                        .small(),
                );

                response
            })
            .inner
        })
        .inner
}

/// Compact identifier display for lists
pub fn show_identifier_badge(ui: &mut Ui, id_type: IdentifierType, value: &str) -> Response {
    let colors = DarkColors::default();
    let (border_color, _, _) = id_type.border_style();

    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // Background frame
            egui::Frame::none()
                .fill(border_color.linear_multiply(0.2))
                .inner_margin(Vec2::new(8.0, 4.0))
                .rounding(Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(id_type.icon());
                        ui.label(
                            egui::RichText::new(value)
                                .font(FontId::monospace(12.0))
                                .color(colors.text_primary),
                        );
                    });
                });
        })
        .response;

    response.on_hover_text(format!("{}: {}", id_type.label(), value))
}
