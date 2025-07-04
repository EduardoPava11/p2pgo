//! Styled button component

use egui::{Color32, Response, RichText, Ui, Vec2, Widget};
use super::theme::{Colors, Spacing, Styles, Typography};

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

pub struct StyledButton {
    text: String,
    style: ButtonStyle,
    size: ButtonSize,
    enabled: bool,
    min_width: Option<f32>,
}

impl StyledButton {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: ButtonStyle::Primary,
            size: ButtonSize::Medium,
            enabled: true,
            min_width: None,
        }
    }
    
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }
    
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }
    
    fn get_colors(&self) -> (Color32, Color32, Color32) {
        match self.style {
            ButtonStyle::Primary => (
                Colors::PRIMARY,
                Color32::from_rgb(
                    (Colors::PRIMARY.r() as f32 * 1.2).min(255.0) as u8,
                    (Colors::PRIMARY.g() as f32 * 1.2).min(255.0) as u8,
                    (Colors::PRIMARY.b() as f32 * 1.2).min(255.0) as u8,
                ),
                Colors::TEXT_PRIMARY,
            ),
            ButtonStyle::Secondary => (
                Colors::SECONDARY,
                Color32::from_rgb(
                    (Colors::SECONDARY.r() as f32 * 1.2).min(255.0) as u8,
                    (Colors::SECONDARY.g() as f32 * 1.2).min(255.0) as u8,
                    (Colors::SECONDARY.b() as f32 * 1.2).min(255.0) as u8,
                ),
                Colors::TEXT_PRIMARY,
            ),
            ButtonStyle::Danger => (
                Colors::ACCENT,
                Color32::from_rgb(
                    (Colors::ACCENT.r() as f32 * 1.2).min(255.0) as u8,
                    (Colors::ACCENT.g() as f32 * 1.2).min(255.0) as u8,
                    (Colors::ACCENT.b() as f32 * 1.2).min(255.0) as u8,
                ),
                Colors::TEXT_PRIMARY,
            ),
            ButtonStyle::Ghost => (
                Color32::TRANSPARENT,
                Color32::from_rgb(
                    (Colors::SURFACE.r() as f32 * 1.5).min(255.0) as u8,
                    (Colors::SURFACE.g() as f32 * 1.5).min(255.0) as u8,
                    (Colors::SURFACE.b() as f32 * 1.5).min(255.0) as u8,
                ),
                Colors::TEXT_PRIMARY,
            ),
        }
    }
    
    fn get_size_params(&self) -> (f32, Vec2) {
        match self.size {
            ButtonSize::Small => (
                Typography::FONT_SMALL,
                Vec2::new(Spacing::SM, Spacing::XS),
            ),
            ButtonSize::Medium => (
                Typography::FONT_BODY,
                Vec2::new(Spacing::MD, Spacing::SM),
            ),
            ButtonSize::Large => (
                Typography::FONT_HEADING,
                Vec2::new(Spacing::LG, Spacing::MD),
            ),
        }
    }
}

impl Widget for StyledButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (base_color, hover_color, text_color) = self.get_colors();
        let (font_size, padding) = self.get_size_params();
        
        let text = RichText::new(&self.text)
            .size(font_size)
            .color(if self.enabled { text_color } else { text_color.linear_multiply(0.5) });
        
        let mut button = egui::Button::new(text)
            .rounding(Styles::rounding())
            .fill(if self.enabled { base_color } else { base_color.linear_multiply(0.5) });
        
        if let Some(width) = self.min_width {
            let height = match self.size {
                ButtonSize::Small => 32.0,
                ButtonSize::Medium => 40.0,
                ButtonSize::Large => 48.0,
            };
            button = button.min_size(Vec2::new(width, height));
        }
        
        let response = ui.add_enabled(self.enabled, button);
        
        // Add hover effect
        if response.hovered() && self.enabled {
            ui.painter().rect_filled(
                response.rect,
                Styles::rounding(),
                hover_color.linear_multiply(0.1),
            );
        }
        
        response
    }
}

// Convenience functions
pub fn primary_button(text: impl Into<String>) -> StyledButton {
    StyledButton::new(text).style(ButtonStyle::Primary)
}

pub fn secondary_button(text: impl Into<String>) -> StyledButton {
    StyledButton::new(text).style(ButtonStyle::Secondary)
}

pub fn danger_button(text: impl Into<String>) -> StyledButton {
    StyledButton::new(text).style(ButtonStyle::Danger)
}

pub fn ghost_button(text: impl Into<String>) -> StyledButton {
    StyledButton::new(text).style(ButtonStyle::Ghost)
}