//! Connection status widget

use crate::connection_status::{ConnectionState, ConnectionStatusWidget};
use egui::{Color32, Ui, Widget};

/// Connection status display
pub struct ConnectionStatus<'a> {
    widget: &'a ConnectionStatusWidget,
}

impl<'a> ConnectionStatus<'a> {
    pub fn new(widget: &'a ConnectionStatusWidget) -> Self {
        Self { widget }
    }
}

impl<'a> Widget for ConnectionStatus<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        // For now, just create a simple label
        // The actual ConnectionStatusWidget would need to be refactored to support this
        ui.label("Connection Status")
    }
}
