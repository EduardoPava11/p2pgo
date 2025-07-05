//! Styled text input component

use super::theme::{Colors, Spacing, Styles};
use egui::{Response, TextEdit, Ui, Vec2};

pub struct StyledInput<'a> {
    text: &'a mut String,
    hint_text: Option<String>,
    multiline: bool,
    desired_width: Option<f32>,
    desired_rows: Option<usize>,
    password: bool,
}

impl<'a> StyledInput<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            hint_text: None,
            multiline: false,
            desired_width: None,
            desired_rows: None,
            password: false,
        }
    }

    pub fn hint_text(mut self, hint: impl Into<String>) -> Self {
        self.hint_text = Some(hint.into());
        self
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = Some(rows);
        self
    }

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let mut text_edit = if self.multiline {
            TextEdit::multiline(self.text)
        } else {
            TextEdit::singleline(self.text)
        };

        if let Some(hint) = self.hint_text {
            text_edit = text_edit.hint_text(hint);
        }

        if let Some(width) = self.desired_width {
            text_edit = text_edit.desired_width(width);
        } else if !self.multiline {
            text_edit = text_edit.desired_width(f32::INFINITY);
        }

        if let Some(rows) = self.desired_rows {
            text_edit = text_edit.desired_rows(rows);
        }

        if self.password {
            text_edit = text_edit.password(true);
        }

        // Apply consistent styling
        let height = if self.multiline {
            self.desired_rows.unwrap_or(3) as f32 * 20.0
        } else {
            Styles::INPUT_HEIGHT
        };

        ui.spacing_mut().text_edit_width = self.desired_width.unwrap_or(200.0);

        let response = ui.add_sized(
            Vec2::new(self.desired_width.unwrap_or(ui.available_width()), height),
            text_edit,
        );

        // Custom styling for focus state
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect,
                Styles::rounding(),
                egui::Stroke::new(2.0, Colors::PRIMARY),
            );
        }

        response
    }
}

// Labeled input helper
pub struct LabeledInput<'a> {
    label: String,
    input: StyledInput<'a>,
}

impl<'a> LabeledInput<'a> {
    pub fn new(label: impl Into<String>, text: &'a mut String) -> Self {
        Self {
            label: label.into(),
            input: StyledInput::new(text),
        }
    }

    pub fn hint_text(mut self, hint: impl Into<String>) -> Self {
        self.input = self.input.hint_text(hint);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.label(&self.label);
            ui.add_space(Spacing::XS);
            self.input.show(ui)
        })
        .inner
    }
}
