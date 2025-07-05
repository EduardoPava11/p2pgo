//! Design System Theme
//! Muted colors for better readability, consistent spacing and typography

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};
use std::collections::BTreeMap;

// Color Palette (Muted for readability)
pub struct Colors;

impl Colors {
    // Primary Colors (Traditional Go)
    pub const BOARD: Color32 = Color32::from_rgb(220, 179, 92); // Traditional Kaya wood
    pub const BLACK_STONE: Color32 = Color32::from_gray(15); // Near black
    pub const WHITE_STONE: Color32 = Color32::from_gray(245); // Off white

    // UI Colors (Muted for better readability)
    pub const BACKGROUND: Color32 = Color32::from_gray(28); // Dark gray
    pub const SURFACE: Color32 = Color32::from_gray(38); // Slightly lighter
    pub const PRIMARY: Color32 = Color32::from_rgb(67, 160, 71); // Muted green
    pub const SECONDARY: Color32 = Color32::from_rgb(66, 115, 179); // Muted blue
    pub const ACCENT: Color32 = Color32::from_rgb(179, 67, 67); // Muted red
    pub const TEXT_PRIMARY: Color32 = Color32::from_gray(230); // Light gray
    pub const TEXT_SECONDARY: Color32 = Color32::from_gray(180); // Medium gray

    // Neural Network Colors
    pub const NEURAL_POLICY: Color32 = Color32::from_rgb(100, 150, 255); // Soft blue
    pub const NEURAL_VALUE: Color32 = Color32::from_rgb(100, 200, 100); // Soft green

    // Status Colors
    pub const SUCCESS: Color32 = Color32::from_rgb(46, 125, 50);
    pub const WARNING: Color32 = Color32::from_rgb(245, 124, 0);
    pub const ERROR: Color32 = Color32::from_rgb(211, 47, 47);
    pub const INFO: Color32 = Color32::from_rgb(25, 118, 210);
}

// Typography
pub struct Typography;

impl Typography {
    pub const FONT_TITLE: f32 = 24.0;
    pub const FONT_HEADING: f32 = 18.0;
    pub const FONT_BODY: f32 = 14.0;
    pub const FONT_SMALL: f32 = 12.0;
    pub const FONT_MONO: f32 = 13.0;

    pub fn title() -> FontId {
        FontId::new(Self::FONT_TITLE, FontFamily::Proportional)
    }

    pub fn heading() -> FontId {
        FontId::new(Self::FONT_HEADING, FontFamily::Proportional)
    }

    pub fn body() -> FontId {
        FontId::new(Self::FONT_BODY, FontFamily::Proportional)
    }

    pub fn small() -> FontId {
        FontId::new(Self::FONT_SMALL, FontFamily::Proportional)
    }

    pub fn mono() -> FontId {
        FontId::new(Self::FONT_MONO, FontFamily::Monospace)
    }
}

// Spacing
pub struct Spacing;

impl Spacing {
    pub const UNIT: f32 = 8.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 32.0;
}

// Common Styles
pub struct Styles;

impl Styles {
    pub const BORDER_RADIUS: f32 = 4.0;
    pub const BUTTON_HEIGHT: f32 = 40.0;
    pub const INPUT_HEIGHT: f32 = 36.0;

    pub fn rounding() -> Rounding {
        Rounding::same(Self::BORDER_RADIUS)
    }

    pub fn border_stroke() -> Stroke {
        Stroke::new(1.0, Colors::SURFACE.gamma_multiply(1.5))
    }
}

// Apply theme to egui context
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = Style::default();

    // Configure visuals
    let mut visuals = Visuals::dark();

    // Window styling
    visuals.window_fill = Colors::BACKGROUND;
    visuals.panel_fill = Colors::SURFACE;
    visuals.window_stroke = Styles::border_stroke();
    visuals.window_rounding = Styles::rounding();

    // Widget styling
    visuals.widgets.noninteractive.bg_fill = Colors::SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Colors::TEXT_SECONDARY);
    visuals.widgets.noninteractive.rounding = Styles::rounding();

    visuals.widgets.inactive.bg_fill = Colors::SURFACE.gamma_multiply(1.2);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Colors::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Styles::rounding();

    visuals.widgets.hovered.bg_fill = Colors::PRIMARY.linear_multiply(0.3);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Colors::PRIMARY);
    visuals.widgets.hovered.rounding = Styles::rounding();

    visuals.widgets.active.bg_fill = Colors::PRIMARY.linear_multiply(0.5);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Colors::PRIMARY);
    visuals.widgets.active.rounding = Styles::rounding();

    // Selection colors
    visuals.selection.bg_fill = Colors::PRIMARY.linear_multiply(0.3);
    visuals.selection.stroke = Stroke::new(1.0, Colors::PRIMARY);

    // Hyperlink color
    visuals.hyperlink_color = Colors::SECONDARY;

    // Code colors
    visuals.code_bg_color = Colors::SURFACE.gamma_multiply(1.3);

    // Apply visuals
    style.visuals = visuals;

    // Configure text styles
    let mut text_styles = BTreeMap::new();
    text_styles.insert(TextStyle::Heading, Typography::heading());
    text_styles.insert(TextStyle::Body, Typography::body());
    text_styles.insert(TextStyle::Small, Typography::small());
    text_styles.insert(TextStyle::Button, Typography::body());
    text_styles.insert(TextStyle::Monospace, Typography::mono());

    style.text_styles = text_styles;

    // Configure spacing
    style.spacing.item_spacing = egui::vec2(Spacing::SM, Spacing::SM);
    style.spacing.button_padding = egui::vec2(Spacing::MD, Spacing::SM);
    style.spacing.indent = Spacing::MD;

    // Apply style
    ctx.set_style(style);
}

// Shadow utilities
pub fn shadow_color(opacity: f32) -> Color32 {
    Color32::from_black_alpha((opacity * 255.0) as u8)
}

pub fn elevation_1() -> egui::Shadow {
    egui::epaint::Shadow {
        blur: 4.0,
        spread: 0.0,
        color: shadow_color(0.2),
        offset: egui::vec2(0.0, 2.0),
    }
}

pub fn elevation_2() -> egui::Shadow {
    egui::epaint::Shadow {
        blur: 8.0,
        spread: 0.0,
        color: shadow_color(0.3),
        offset: egui::vec2(0.0, 4.0),
    }
}
