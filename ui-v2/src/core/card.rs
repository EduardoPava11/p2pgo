//! Card container component

use egui::{Frame, Response, Ui};
use super::theme::{Colors, Spacing, Styles, elevation_1, elevation_2};

#[derive(Clone, Copy, PartialEq)]
pub enum CardElevation {
    None,
    Low,
    High,
}

pub struct Card {
    elevation: CardElevation,
    padding: f32,
}

impl Card {
    pub fn new() -> Self {
        Self {
            elevation: CardElevation::Low,
            padding: Spacing::MD,
        }
    }
    
    pub fn elevation(mut self, elevation: CardElevation) -> Self {
        self.elevation = elevation;
        self
    }
    
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
    
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> (Response, R) {
        let shadow = match self.elevation {
            CardElevation::None => Default::default(),
            CardElevation::Low => elevation_1(),
            CardElevation::High => elevation_2(),
        };
        
        let frame = Frame::none()
            .fill(Colors::SURFACE)
            .inner_margin(self.padding)
            .rounding(Styles::rounding())
            .shadow(shadow);
        
        let response = frame.show(ui, content);
        (response.response, response.inner)
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

// Section card with heading
pub struct SectionCard {
    title: String,
    collapsible: bool,
    default_open: bool,
}

impl SectionCard {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            collapsible: false,
            default_open: true,
        }
    }
    
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }
    
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    
    pub fn show<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        Card::new().show(ui, |ui| {
            if self.collapsible {
                let mut state = ui.data_mut(|d| {
                    d.get_temp::<bool>(ui.id().with(&self.title))
                        .unwrap_or(self.default_open)
                });
                
                let header = egui::CollapsingHeader::new(&self.title)
                    .default_open(state)
                    .show(ui, |ui| {
                        ui.add_space(Spacing::SM);
                        content(ui)
                    });
                
                ui.data_mut(|d| d.insert_temp(ui.id().with(&self.title), header.header_response.clicked()));
                
                header.body_returned
            } else {
                ui.heading(&self.title);
                ui.add_space(Spacing::SM);
                Some(content(ui))
            }
        }).1
    }
}