//! UI Configuration System for P2P Go
//!
//! This module provides comprehensive UI customization including:
//! - Board appearance (grid color, line width, stone rendering)
//! - Button styling (font, size, colors, padding)
//! - Territory marking visualization
//! - WASM tensor-based parameter adjustment

use serde::{Serialize, Deserialize};
use egui::{Color32, FontId, FontFamily, Vec2, Rounding};

/// Complete UI configuration for the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Window configuration
    pub window: WindowConfig,
    /// Board visual configuration
    pub board: BoardConfig,
    /// Button styling configuration
    pub button: ButtonConfig,
    /// Territory marking configuration
    pub territory: TerritoryConfig,
    /// Font configuration
    pub fonts: FontConfig,
    /// Color scheme
    pub colors: ColorScheme,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Initial window size (width, height)
    pub initial_size: (f32, f32),
    /// Minimum window size
    pub min_size: (f32, f32),
    /// Maximum window size (None for unlimited)
    pub max_size: Option<(f32, f32)>,
    /// Window padding
    pub padding: f32,
    /// Background color
    pub background_color: SerializableColor,
}

/// Board visual configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardConfig {
    /// Board size in pixels (will be square)
    pub size: f32,
    /// Board margin from window edges
    pub margin: f32,
    /// Grid line color
    pub grid_color: SerializableColor,
    /// Grid line width
    pub grid_line_width: f32,
    /// Star point radius (for 9x9: typically at 3,3 5,5 7,7 etc)
    pub star_point_radius: f32,
    /// Board background color
    pub background_color: SerializableColor,
    /// Stone radius as fraction of cell size
    pub stone_radius_ratio: f32,
    /// Stone outline width
    pub stone_outline_width: f32,
    /// Black stone color
    pub black_stone_color: SerializableColor,
    /// White stone color
    pub white_stone_color: SerializableColor,
    /// Last move marker size ratio
    pub last_move_marker_ratio: f32,
    /// Coordinate labels (A-J, 1-9)
    pub show_coordinates: bool,
    /// Coordinate font size
    pub coordinate_font_size: f32,
}

/// Button styling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Text color
    pub text_color: SerializableColor,
    /// Background color (normal state)
    pub background_color: SerializableColor,
    /// Background color (hovered)
    pub hover_color: SerializableColor,
    /// Background color (clicked)
    pub click_color: SerializableColor,
    /// Border color
    pub border_color: SerializableColor,
    /// Border width
    pub border_width: f32,
    /// Corner rounding
    pub corner_radius: f32,
    /// Padding (x, y)
    pub padding: (f32, f32),
    /// Minimum button size
    pub min_size: (f32, f32),
    /// Shadow offset (x, y) - None for no shadow
    pub shadow_offset: Option<(f32, f32)>,
    /// Shadow color
    pub shadow_color: SerializableColor,
}

/// Territory marking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerritoryConfig {
    /// Territory marker type
    pub marker_type: TerritoryMarkerType,
    /// Marker size ratio relative to cell
    pub marker_size_ratio: f32,
    /// Black territory color
    pub black_territory_color: SerializableColor,
    /// White territory color  
    pub white_territory_color: SerializableColor,
    /// Neutral/dame color
    pub neutral_color: SerializableColor,
    /// Territory outline width
    pub outline_width: f32,
    /// Animation duration for territory changes (ms)
    pub animation_duration: u32,
    /// Show territory count
    pub show_count: bool,
}

/// Territory marker visualization type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerritoryMarkerType {
    /// Small square markers
    Square,
    /// Small circle markers
    Circle,
    /// Cross markers
    Cross,
    /// Fill entire intersection
    Fill,
    /// Transparent overlay
    Overlay,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// UI font family
    pub ui_font: String,
    /// Monospace font for coordinates
    pub mono_font: String,
    /// Bold font weight
    pub bold_weight: f32,
    /// Line height multiplier
    pub line_height: f32,
}

/// Color scheme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Primary color (for important UI elements)
    pub primary: SerializableColor,
    /// Secondary color
    pub secondary: SerializableColor,
    /// Success color (winning, good moves)
    pub success: SerializableColor,
    /// Warning color
    pub warning: SerializableColor,
    /// Error color
    pub error: SerializableColor,
    /// Info color
    pub info: SerializableColor,
    /// Text color on dark backgrounds
    pub text_light: SerializableColor,
    /// Text color on light backgrounds
    pub text_dark: SerializableColor,
}

/// Serializable color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SerializableColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color32> for SerializableColor {
    fn from(color: Color32) -> Self {
        let [r, g, b, a] = color.to_array();
        Self { r, g, b, a }
    }
}

impl From<SerializableColor> for Color32 {
    fn from(color: SerializableColor) -> Self {
        Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                title: "P2P Go - Offline Mode".to_string(),
                initial_size: (900.0, 900.0), // Square window for 9x9
                min_size: (600.0, 600.0),
                max_size: None,
                padding: 20.0,
                background_color: Color32::from_rgb(245, 245, 245).into(), // Clean light gray background
            },
            
            board: BoardConfig {
                size: 800.0, // Generous board size
                margin: 50.0,
                grid_color: Color32::from_rgb(0, 0, 0).into(), // Pure black grid lines like OGS
                grid_line_width: 1.0, // Thinner lines for cleaner look
                star_point_radius: 3.5,
                background_color: Color32::from_rgb(255, 255, 255).into(), // Pure white board like OGS
                stone_radius_ratio: 0.46, // Slightly smaller for cleaner look
                stone_outline_width: 0.8,
                black_stone_color: Color32::from_rgb(10, 10, 10).into(),
                white_stone_color: Color32::from_rgb(250, 250, 250).into(),
                last_move_marker_ratio: 0.25,
                show_coordinates: true,
                coordinate_font_size: 12.0,
            },
            
            button: ButtonConfig {
                font_family: "Open Sans".to_string(),
                font_size: 14.0,
                text_color: Color32::from_rgb(51, 51, 51).into(), // Dark gray text
                background_color: Color32::from_rgb(255, 255, 255).into(), // White buttons
                hover_color: Color32::from_rgb(240, 240, 240).into(),
                click_color: Color32::from_rgb(220, 220, 220).into(),
                border_color: Color32::from_rgb(200, 200, 200).into(),
                border_width: 1.0,
                corner_radius: 4.0,
                padding: (12.0, 8.0),
                min_size: (80.0, 32.0),
                shadow_offset: None, // No shadow for cleaner look
                shadow_color: Color32::from_rgba_unmultiplied(0, 0, 0, 0).into(),
            },
            
            territory: TerritoryConfig {
                marker_type: TerritoryMarkerType::Square,
                marker_size_ratio: 0.25, // Smaller markers
                black_territory_color: Color32::from_rgba_unmultiplied(0, 0, 0, 120).into(),
                white_territory_color: Color32::from_rgba_unmultiplied(200, 200, 200, 120).into(),
                neutral_color: Color32::from_rgba_unmultiplied(128, 128, 128, 80).into(),
                outline_width: 0.5,
                animation_duration: 150,
                show_count: true,
            },
            
            fonts: FontConfig {
                ui_font: "Open Sans".to_string(),
                mono_font: "Fira Code".to_string(),
                bold_weight: 700.0,
                line_height: 1.4,
            },
            
            colors: ColorScheme {
                primary: Color32::from_rgb(64, 128, 255).into(),
                secondary: Color32::from_rgb(128, 64, 255).into(),
                success: Color32::from_rgb(64, 192, 64).into(),
                warning: Color32::from_rgb(255, 192, 64).into(),
                error: Color32::from_rgb(255, 64, 64).into(),
                info: Color32::from_rgb(64, 192, 255).into(),
                text_light: Color32::WHITE.into(),
                text_dark: Color32::from_rgb(32, 32, 32).into(),
            },
        }
    }
}

impl UiConfig {
    /// Load config from file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(config)
    }
    
    /// Save config to file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
    
    /// Apply WASM tensor parameters to modify UI
    pub fn apply_tensor_params(&mut self, tensor_data: &[f32]) {
        // Example: Use tensor values to adjust UI parameters
        // This could be from a trained model that learns user preferences
        
        if tensor_data.len() >= 3 {
            // Adjust grid line width based on first tensor value
            self.board.grid_line_width = 0.5 + (tensor_data[0] * 2.0).clamp(0.0, 3.0);
            
            // Adjust stone size based on second tensor value
            self.board.stone_radius_ratio = 0.4 + (tensor_data[1] * 0.2).clamp(0.0, 0.1);
            
            // Adjust territory marker size based on third tensor value
            self.territory.marker_size_ratio = 0.2 + (tensor_data[2] * 0.3).clamp(0.0, 0.2);
        }
        
        // More sophisticated mappings could be added here
        // For example, using tensor values to interpolate between color schemes
        // or to adjust animation speeds based on learned user preferences
    }
}

/// Helper to create egui FontId from config
pub fn create_font_id(_config: &UiConfig, size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Helper to create button from config
pub fn styled_button(ui: &mut egui::Ui, config: &ButtonConfig, text: &str) -> egui::Response {
    let font_id = FontId::new(config.font_size, FontFamily::Proportional);
    
    let text_color: Color32 = config.text_color.into();
    let button_size = Vec2::new(
        config.min_size.0.max(config.padding.0 * 2.0 + 100.0),
        config.min_size.1.max(config.padding.1 * 2.0 + config.font_size)
    );
    
    // Create custom button with full styling
    let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
    
    if ui.is_rect_visible(rect) {
        let _visuals = ui.style().interact(&response);
        
        // Determine background color based on state
        let bg_color: Color32 = if response.clicked() {
            config.click_color.into()
        } else if response.hovered() {
            config.hover_color.into()
        } else {
            config.background_color.into()
        };
        
        // Draw shadow if configured
        if let Some((x_offset, y_offset)) = config.shadow_offset {
            let shadow_rect = rect.translate(Vec2::new(x_offset, y_offset));
            ui.painter().rect_filled(
                shadow_rect,
                Rounding::same(config.corner_radius),
                Color32::from(config.shadow_color)
            );
        }
        
        // Draw button background
        ui.painter().rect(
            rect,
            Rounding::same(config.corner_radius),
            bg_color,
            egui::Stroke::new(config.border_width, Color32::from(config.border_color))
        );
        
        // Draw text
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            font_id,
            text_color,
        );
    }
    
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = UiConfig::default();
        assert_eq!(config.window.initial_size, (900.0, 900.0));
        assert_eq!(config.board.size, 800.0);
        assert_eq!(config.button.font_size, 16.0);
    }
    
    #[test]
    fn test_color_conversion() {
        let egui_color = Color32::from_rgb(100, 150, 200);
        let ser_color: SerializableColor = egui_color.into();
        let back_color: Color32 = ser_color.into();
        assert_eq!(egui_color, back_color);
    }
    
    #[test]
    fn test_tensor_params() {
        let mut config = UiConfig::default();
        let original_grid_width = config.board.grid_line_width;
        
        let tensor_data = vec![0.5, 0.5, 0.5];
        config.apply_tensor_params(&tensor_data);
        
        assert_ne!(config.board.grid_line_width, original_grid_width);
    }
}