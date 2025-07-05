//! Dark theme for P2P Go with improved contrast and readability

use egui::{
    Button, Color32, FontFamily, FontId, Response, Rounding, Stroke, Style, Ui, Vec2, Visuals,
};

/// Dark theme colors optimized for readability
pub struct DarkColors {
    // Background colors - darker for better contrast
    pub background: Color32,      // Main app background
    pub surface: Color32,         // Panel/card background
    pub surface_variant: Color32, // Slightly different surface

    // Text colors - brighter for readability
    pub text_primary: Color32,   // Main text
    pub text_secondary: Color32, // Less important text
    pub text_disabled: Color32,  // Disabled text

    // Go board colors
    pub board_bg: Color32,    // Board background
    pub black_stone: Color32, // Black stones
    pub white_stone: Color32, // White stones
    pub grid_line: Color32,   // Grid lines
    pub last_move: Color32,   // Last move indicator

    // UI accent colors
    pub primary: Color32,         // Primary actions (red)
    pub primary_variant: Color32, // Primary hover
    pub secondary: Color32,       // Secondary actions
    pub success: Color32,         // Success/good moves
    pub warning: Color32,         // Warnings
    pub error: Color32,           // Errors/urgent

    // Borders and dividers
    pub border: Color32,        // Normal borders
    pub border_strong: Color32, // Emphasized borders
    pub divider: Color32,       // Divider lines
}

impl Default for DarkColors {
    fn default() -> Self {
        Self {
            // Dark backgrounds
            background: Color32::from_gray(18), // Very dark gray
            surface: Color32::from_gray(26),    // Slightly lighter
            surface_variant: Color32::from_gray(32), // Panel variant

            // Bright text for contrast
            text_primary: Color32::from_gray(240), // Almost white
            text_secondary: Color32::from_gray(180), // Light gray
            text_disabled: Color32::from_gray(100), // Darker gray

            // Go board - traditional colors
            board_bg: Color32::from_rgb(220, 179, 92), // Traditional kaya
            black_stone: Color32::from_gray(15),       // Near black
            white_stone: Color32::from_gray(245),      // Off white
            grid_line: Color32::from_gray(40),         // Dark lines
            last_move: Color32::from_rgb(220, 38, 38), // Red indicator

            // UI accents
            primary: Color32::from_rgb(220, 38, 38), // Bold red
            primary_variant: Color32::from_rgb(185, 28, 28), // Darker red
            secondary: Color32::from_rgb(59, 130, 246), // Blue
            success: Color32::from_rgb(34, 197, 94), // Green
            warning: Color32::from_rgb(251, 146, 60), // Orange
            error: Color32::from_rgb(239, 68, 68),   // Bright red

            // Borders
            border: Color32::from_gray(60),         // Subtle border
            border_strong: Color32::from_gray(100), // Strong border
            divider: Color32::from_gray(50),        // Divider
        }
    }
}

/// Apply dark theme to egui context
pub fn apply_dark_theme(ctx: &egui::Context) {
    let colors = DarkColors::default();
    let mut style = (*ctx.style()).clone();

    // Dark mode visuals
    style.visuals.dark_mode = true;

    // Window and panel styling
    style.visuals.window_fill = colors.surface;
    style.visuals.panel_fill = colors.background;
    style.visuals.faint_bg_color = colors.surface_variant;
    style.visuals.extreme_bg_color = colors.background;

    // Text colors
    style.visuals.override_text_color = Some(colors.text_primary);

    // Widget colors
    style.visuals.widgets.noninteractive.bg_fill = colors.surface;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.text_primary);
    style.visuals.widgets.noninteractive.weak_bg_fill = colors.surface_variant;

    style.visuals.widgets.inactive.bg_fill = colors.surface_variant;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.text_primary);
    style.visuals.widgets.inactive.weak_bg_fill = colors.surface;

    style.visuals.widgets.hovered.bg_fill = colors.surface_variant;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, colors.primary);
    style.visuals.widgets.hovered.weak_bg_fill = colors.surface_variant;

    style.visuals.widgets.active.bg_fill = colors.primary;
    style.visuals.widgets.active.fg_stroke = Stroke::new(2.0, colors.primary);
    style.visuals.widgets.active.weak_bg_fill = colors.primary.linear_multiply(0.2);

    // Selection and hyperlinks
    style.visuals.selection.bg_fill = colors.primary.linear_multiply(0.3);
    style.visuals.hyperlink_color = colors.primary;

    // Window styling
    style.visuals.window_shadow.extrusion = 8.0;
    style.visuals.window_shadow.color = Color32::from_black_alpha(180);
    style.visuals.window_stroke = Stroke::new(1.0, colors.border);
    style.visuals.window_rounding = Rounding::same(4.0);

    // Spacing adjustments for compact layout
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.spacing.menu_margin = 10.0.into();
    style.spacing.indent = 20.0;

    // Interaction feedback
    style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    style.visuals.image_loading_spinners = true;

    ctx.set_style(style);
}

/// Style a button for dark theme
pub fn dark_button(ui: &mut Ui, text: &str, primary: bool) -> Response {
    let colors = DarkColors::default();

    let (fill, text_color) = if primary {
        (colors.primary, colors.text_primary)
    } else {
        (colors.surface_variant, colors.text_primary)
    };

    let button = Button::new(text)
        .fill(fill)
        .stroke(Stroke::new(
            1.0,
            if primary {
                colors.primary
            } else {
                colors.border
            },
        ))
        .rounding(Rounding::same(4.0))
        .min_size(Vec2::new(100.0, 36.0));

    let response = ui.add(button);

    // Custom text color
    if response.hovered() || response.clicked() {
        ui.painter().rect_filled(
            response.rect,
            Rounding::same(4.0),
            if primary {
                colors.primary_variant
            } else {
                colors.surface_variant.linear_multiply(1.2)
            },
        );
    }

    response
}

/// Create a styled panel for dark theme
pub fn dark_panel(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    let colors = DarkColors::default();

    egui::Frame::none()
        .fill(colors.surface)
        .stroke(Stroke::new(1.0, colors.border))
        .inner_margin(16.0)
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            if !title.is_empty() {
                ui.heading(title);
                ui.add_space(4.0);
                ui.add(egui::Separator::default().spacing(8.0));
                ui.add_space(8.0);
            }
            content(ui);
        });
}

/// Apply compact spacing for less whitespace
pub fn apply_compact_spacing(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // Reduce all spacing
    style.spacing.item_spacing = Vec2::new(6.0, 4.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.menu_margin = 8.0.into();
    style.spacing.indent = 16.0;
    style.spacing.interact_size = Vec2::new(36.0, 30.0);
    style.spacing.slider_width = 140.0;
    style.spacing.combo_width = 140.0;
    style.spacing.text_edit_width = 200.0;
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 12.0;
    style.spacing.icon_spacing = 4.0;
    style.spacing.tooltip_width = 400.0;
    style.spacing.indent_ends_with_horizontal_line = false;
    style.spacing.combo_height = 200.0;
    style.spacing.scroll_bar_width = 10.0;
    style.spacing.scroll_handle_min_length = 20.0;
    style.spacing.scroll_bar_inner_margin = 2.0;
    style.spacing.scroll_bar_outer_margin = 2.0;

    ctx.set_style(style);
}
