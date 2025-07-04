//! Domain-specific UI widgets

pub mod board_widget;
pub mod neural_panel;

pub use board_widget::{BoardWidget, BoardResponse};
pub use neural_panel::{NeuralPanel, PanelPosition, SuggestedMove};