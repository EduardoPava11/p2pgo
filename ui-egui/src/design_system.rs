//! Unified Design System for P2P Go
//! 
//! All UI components should use this design system to maintain consistency
//! The design is centered around the 9x9 Go board aesthetic

use egui::{Color32, FontId, FontFamily, Rounding, Stroke, Vec2, Button, Response, Ui};

/// Core colors - Clean black/white/red design
pub struct GoColors {
    /// Pure black for maximum contrast
    pub black: Color32,
    /// Pure white for clean look
    pub white: Color32,
    /// Bold red for accents and important actions
    pub red: Color32,
    /// Light gray for subtle elements
    pub gray_light: Color32,
    /// Medium gray for borders
    pub gray_medium: Color32,
    /// Dark gray for less important text
    pub gray_dark: Color32,
    /// Board background - light gray
    pub board_bg: Color32,
    /// Black stone color
    pub black_stone: Color32,
    /// White stone color  
    pub white_stone: Color32,
    /// Grid line color
    pub grid_line: Color32,
    /// Main background
    pub background: Color32,
    /// Primary text
    pub text_primary: Color32,
    /// Secondary text
    pub text_secondary: Color32,
    /// Success state
    pub success: Color32,
    /// Error/important state
    pub error: Color32,
}

impl Default for GoColors {
    fn default() -> Self {
        Self {
            // Core palette
            black: Color32::BLACK,
            white: Color32::WHITE,
            red: Color32::from_rgb(220, 38, 38), // Bold red
            
            // Grays
            gray_light: Color32::from_gray(245),
            gray_medium: Color32::from_gray(200),
            gray_dark: Color32::from_gray(100),
            
            // Board specific
            board_bg: Color32::from_gray(240),
            black_stone: Color32::BLACK,
            white_stone: Color32::WHITE,
            grid_line: Color32::BLACK,
            
            // UI colors
            background: Color32::WHITE,
            text_primary: Color32::BLACK,
            text_secondary: Color32::from_gray(100),
            success: Color32::from_rgb(34, 197, 94), // Green
            error: Color32::from_rgb(220, 38, 38), // Red
        }
    }
}

/// Typography system - Bold and clean
pub struct GoTypography {
    pub font_family: String,
    pub font_size_small: f32,
    pub font_size_body: f32,
    pub font_size_heading: f32,
    pub font_size_title: f32,
    pub font_size_large: f32,
    pub line_height: f32,
    pub font_weight_normal: f32,
    pub font_weight_bold: f32,
}

impl Default for GoTypography {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(), // Modern, clean font
            font_size_small: 13.0,
            font_size_body: 16.0,
            font_size_heading: 20.0,
            font_size_title: 28.0,
            font_size_large: 36.0,
            line_height: 1.5,
            font_weight_normal: 400.0,
            font_weight_bold: 700.0,
        }
    }
}

/// Spacing system based on board grid
pub struct GoSpacing {
    pub grid_unit: f32,  // Base unit (30px like board cells)
    pub xs: f32,         // 0.25 * grid
    pub sm: f32,         // 0.5 * grid
    pub md: f32,         // 1.0 * grid
    pub lg: f32,         // 1.5 * grid
    pub xl: f32,         // 2.0 * grid
}

impl Default for GoSpacing {
    fn default() -> Self {
        let grid = 30.0;
        Self {
            grid_unit: grid,
            xs: grid * 0.25,
            sm: grid * 0.5,
            md: grid,
            lg: grid * 1.5,
            xl: grid * 2.0,
        }
    }
}

/// The main design system
pub struct GoDesignSystem {
    pub colors: GoColors,
    pub typography: GoTypography,
    pub spacing: GoSpacing,
}

impl Default for GoDesignSystem {
    fn default() -> Self {
        Self {
            colors: GoColors::default(),
            typography: GoTypography::default(),
            spacing: GoSpacing::default(),
        }
    }
}

impl GoDesignSystem {
    /// Get font ID for different text styles
    pub fn font_id(&self, style: TextStyle) -> FontId {
        let size = match style {
            TextStyle::Small => self.typography.font_size_small,
            TextStyle::Body => self.typography.font_size_body,
            TextStyle::Heading => self.typography.font_size_heading,
            TextStyle::Title => self.typography.font_size_title,
        };
        FontId::new(size, FontFamily::Proportional)
    }
    
    /// Style a button with clean black/white design
    pub fn style_button(&self, ui: &mut Ui, text: &str) -> Response {
        let button = Button::new(text)
            .fill(self.colors.white)
            .stroke(Stroke::new(2.0, self.colors.black))
            .rounding(Rounding::same(0.0)) // Sharp corners for modern look
            .min_size(Vec2::new(100.0, 40.0));
            
        let response = ui.add(button);
        
        if response.hovered() {
            ui.painter().rect_filled(
                response.rect,
                Rounding::same(0.0),
                self.colors.gray_light,
            );
        }
        
        response
    }
    
    /// Style a primary action button (bold red)
    pub fn style_primary_button(&self, ui: &mut Ui, text: &str) -> Response {
        let button = Button::new(text)
            .fill(self.colors.red)
            .stroke(Stroke::new(0.0, self.colors.red))
            .rounding(Rounding::same(0.0))
            .min_size(Vec2::new(140.0, 48.0));
            
        let response = ui.add(button);
        
        // Make text white on red button
        let galley = response.ctx.fonts(|f| {
            f.layout_no_wrap(
                text.to_string(),
                self.font_id(TextStyle::Body),
                self.colors.white,
            )
        });
        ui.painter().galley(
            response.rect.center() - galley.size() / 2.0,
            galley,
        );
        
        response
    }
    
    /// Apply the design system to egui context
    pub fn apply_to_context(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        // Apply colors
        style.visuals.window_fill = self.colors.background;
        style.visuals.panel_fill = self.colors.background;
        // Button styling through widgets
        style.visuals.widgets.inactive.bg_fill = self.colors.white;
        style.visuals.hyperlink_color = self.colors.red;
        style.visuals.selection.bg_fill = self.colors.red.linear_multiply(0.2);
        
        // Make everything sharp and clean
        style.visuals.window_rounding = Rounding::ZERO;
        style.visuals.widgets.inactive.rounding = Rounding::ZERO;
        style.visuals.menu_rounding = Rounding::ZERO;
        
        // Bold strokes
        style.visuals.window_stroke = Stroke::new(2.0, self.colors.black);
        
        // Apply spacing
        style.spacing.item_spacing = Vec2::new(self.spacing.sm, self.spacing.sm);
        style.spacing.button_padding = Vec2::new(self.spacing.md, self.spacing.sm);
        style.spacing.indent = self.spacing.md;
        
        ctx.set_style(style);
    }
}

/// Text style variants
pub enum TextStyle {
    Small,
    Body,
    Heading,
    Title,
}

/// Global design system instance
pub fn get_design_system() -> &'static GoDesignSystem {
    static DESIGN_SYSTEM: std::sync::OnceLock<GoDesignSystem> = std::sync::OnceLock::new();
    DESIGN_SYSTEM.get_or_init(GoDesignSystem::default)
}

/// Clean panel with black border
pub fn board_panel(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    let ds = get_design_system();
    
    egui::Frame::none()
        .fill(ds.colors.white)
        .stroke(Stroke::new(2.0, ds.colors.black))
        .inner_margin(ds.spacing.md)
        .rounding(Rounding::ZERO)
        .show(ui, |ui| {
            // Bold title
            ui.label(egui::RichText::new(title)
                .font(ds.font_id(TextStyle::Heading))
                .strong()
                .color(ds.colors.black));
            ui.add_space(ds.spacing.sm);
            ui.add(egui::Separator::default().horizontal());
            ui.add_space(ds.spacing.sm);
            content(ui);
        });
}